//! End-to-end command runner with strict stdout/stderr separation.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{self, BufReader, BufWriter, IsTerminal, Read, Write},
    path::Path,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
        mpsc::{SyncSender, sync_channel},
    },
    thread,
};

use thiserror::Error;
use tq_core::{
    Analysis, AnalysisContext, Analyzed, AutomaticPlan, Compiled, Diagnostic, Events,
    HybridBlocking, HybridPreparation, InputCursor, InputValue, Number, PathComponent, Plan,
    PlanKind, Query, ResolveOptions, Resolved, SourceId, StableSortPipeline,
    StableSortPipelineObservations, Transcode, TranscodeCommitment, TranscodeDuplicatePolicy,
    TranscodeInput, TranscodeLimits, TranscodeProof, Value, Vm, VmError, VmLimits, VmObservations,
    analyze_with_context, parallel_worker_count, parse_bytes, resolve,
};
use tq_formats::{
    DecodeOptions, DocumentSource, FormatError, InputFormat, JsonDocumentSource, JsonEventOptions,
    JsonLinesDocumentSource, OutputError, OutputFormat, OutputOptions, ParallelJsonObservations,
    ParallelJsonOptions, ProbeReport, SelectedStreamObservations, StreamOptions, StreamRecord,
    StreamSelection, ToonFraming, VecDocumentSource, decode_bytes, decode_json,
    decode_json_event_stream, decode_toon, probe_format, probe_reader, stream_json,
    stream_json_selected_records_parallel, stream_json_selected_records_with_control, stream_toon,
    stream_toon_selected_records_with_control, write_results,
};
use tq_toon::{
    ArrayPreparationConfig, DecodeIntoError, Decoder, DuplicateKeyPolicy, KeyFolding,
    PreparationArena, PreparationLimits, PreparationObservations, PublicationBuffer,
    PublicationError, SpoolError, TranscodeConsumer, TranscodeError, WriterError,
};

use crate::{
    CliError, ColorMode, Command, ExecutionOverride, ExitStatus, ExplainFormat,
    ExternalArgumentKind, FilterSource, PositionalArgumentKind, RunOptions, generated_help,
};

static CANCELLATION: OnceLock<Arc<AtomicBool>> = OnceLock::new();
const INPUT_BUFFER_BYTES: usize = 64 * 1024;
// A rendezvous channel forces one kernel wakeup per document. This small bound
// keeps source read-ahead and retained values fixed while allowing the decoder
// and evaluator to run in batches.
const REMAINING_INPUT_BUFFER_DOCUMENTS: usize = 16;

const AMBIENT_ENVIRONMENT: &str = "__tq_ambient_environment";
const AMBIENT_PLATFORM: &str = "__tq_ambient_platform";
const INPUT_FILENAME: &str = "__tq_input_filename";
const INPUT_LINE_NUMBER: &str = "__tq_input_line_number";

/// Command execution failure with a stable exit category.
#[derive(Debug, Error)]
pub enum RunError {
    /// CLI parsing or compatibility validation.
    #[error(transparent)]
    Cli(#[from] CliError),
    /// Query compilation pipeline.
    #[error("query compilation failed: {0}")]
    Compile(Box<Diagnostic>),
    /// Input decoding/profile failure.
    #[error(transparent)]
    Input(#[from] FormatError),
    /// Query runtime failure.
    #[error(transparent)]
    Runtime(#[from] VmError),
    /// Result formatting or output failure.
    #[error(transparent)]
    Output(#[from] OutputError),
    /// Internal JSON report/raw serialization failure.
    #[error("JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),
    /// File/stdin/query I/O.
    #[error("system I/O failed: {0}")]
    Io(#[from] io::Error),
    /// Path-bearing file I/O without file-content disclosure.
    #[error("system I/O failed for '{path}': {source}")]
    IoPath {
        /// User-visible path identity.
        path: String,
        /// Underlying operating-system failure.
        source: io::Error,
    },
    /// Mode recognized but not admitted by the current plan.
    #[error("unsupported mode: {0}")]
    Unsupported(String),
    /// Raw-output encoding failure.
    #[error("raw output failed: {0}")]
    RawOutput(&'static str),
    /// Unframed structured output did not produce exactly one result.
    #[error("output cardinality failed: {0}")]
    Cardinality(&'static str),
    /// A CLI-level input, result, or output envelope was exceeded.
    #[error("resource limit exceeded: {0}")]
    Resource(&'static str),
    /// A source-specific input envelope was exceeded.
    #[error("resource limit exceeded for '{identity}': {resource}")]
    ResourceSource {
        /// Stable source or path identity without source contents.
        identity: String,
        /// Stable resource classification.
        resource: &'static str,
    },
    /// Execution was interrupted cooperatively.
    #[error("execution interrupted")]
    Interrupted,
}

impl RunError {
    /// Process status category for this failure.
    #[must_use]
    pub fn status(&self) -> ExitStatus {
        match self {
            Self::Cli(CliError::Unsupported(_))
            | Self::Unsupported(_)
            | Self::Runtime(VmError::Unsupported { .. }) => ExitStatus::Unsupported,
            Self::Cli(_) | Self::Io(_) | Self::IoPath { .. } => ExitStatus::Usage,
            Self::Compile(_) => ExitStatus::Compile,
            Self::Resource(_)
            | Self::ResourceSource { .. }
            | Self::Input(FormatError::Resource(_))
            | Self::Runtime(VmError::Resource { .. }) => ExitStatus::Resource,
            Self::Interrupted | Self::Runtime(VmError::Interrupted) => ExitStatus::Interrupted,
            Self::Input(error) if error.to_string().contains("resource limit exceeded") => {
                ExitStatus::Resource
            }
            Self::Output(error) if error.to_string().contains("resource limit exceeded") => {
                ExitStatus::Resource
            }
            Self::Input(_) => ExitStatus::Input,
            Self::Runtime(_)
            | Self::Output(_)
            | Self::Json(_)
            | Self::RawOutput(_)
            | Self::Cardinality(_) => ExitStatus::Runtime,
        }
    }

    fn is_broken_pipe(&self) -> bool {
        match self {
            Self::Io(error) | Self::IoPath { source: error, .. } => {
                error.kind() == io::ErrorKind::BrokenPipe
            }
            Self::Output(error) => error.is_broken_pipe(),
            _ => false,
        }
    }
}

/// Runs a parsed command against process stdio and writes diagnostics only to
/// stderr.
#[must_use]
pub fn run(mut command: Command) -> ExitStatus {
    if let Command::Run(options) = &mut command
        && options.color == ColorMode::Auto
    {
        let terminal = options.capability_policy.terminal && io::stdout().is_terminal();
        let no_color = options.capability_policy.environment
            && std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty());
        options.color = if terminal && !no_color {
            ColorMode::Always
        } else {
            ColorMode::Never
        };
    }
    let mut stdin = io::stdin();
    let stdout = io::stdout().lock();
    let mut stdout = io::BufWriter::with_capacity(64 * 1024, stdout);
    let mut stderr = io::stderr().lock();
    if let Err(error) = install_interrupt_handler() {
        let _ = writeln!(stderr, "tq: could not install interrupt handler: {error}");
        return ExitStatus::Usage;
    }
    let result = run_with_io(command, &mut stdin, &mut stdout, &mut stderr);
    if let Err(error) = stdout.flush() {
        if error.kind() == io::ErrorKind::BrokenPipe {
            return ExitStatus::Success;
        }
        let _ = writeln!(stderr, "tq: system I/O failed: {error}");
        return ExitStatus::Runtime;
    }
    match result {
        Ok(status) => status,
        Err(error) => {
            if error.is_broken_pipe() {
                return ExitStatus::Success;
            }
            let _ = writeln!(stderr, "tq: {error}");
            error.status()
        }
    }
}

fn install_interrupt_handler() -> io::Result<()> {
    if CANCELLATION.get().is_some() {
        return Ok(());
    }
    let flag = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&flag))?;
    let _ = CANCELLATION.set(flag);
    Ok(())
}

fn cancellation() -> Option<Arc<AtomicBool>> {
    CANCELLATION.get().cloned()
}

/// Runs a parsed command with injectable stdio for compatibility tests.
///
/// # Errors
///
/// Returns stable CLI, compile, input, runtime, resource, or output failures.
pub fn run_with_io<R: Read + Send, W: Write, E: Write>(
    command: Command,
    stdin: &mut R,
    stdout: &mut W,
    stderr: &mut E,
) -> Result<ExitStatus, RunError> {
    match command {
        Command::Help => {
            stdout.write_all(generated_help().as_bytes())?;
            Ok(ExitStatus::Success)
        }
        Command::Version => {
            writeln!(
                stdout,
                "tq {} (TOON v3; jq target 1.8.x; revision {})",
                env!("CARGO_PKG_VERSION"),
                option_env!("TQ_BUILD_REVISION").unwrap_or("unknown")
            )?;
            Ok(ExitStatus::Success)
        }
        Command::Compatibility => {
            stdout.write_all(include_bytes!(
                "../tests/compatibility/reviews/coverage-v1.json"
            ))?;
            stdout.write_all(b"\n")?;
            Ok(ExitStatus::Success)
        }
        Command::BuildConfiguration => {
            writeln!(
                stdout,
                "target={} binary-stdio={} formats=toon,yaml,json,jsonl jq-target=1.8.x",
                std::env::consts::OS,
                if cfg!(windows) {
                    "requested-with--binary"
                } else {
                    "native"
                },
            )?;
            Ok(ExitStatus::Success)
        }
        Command::Run(options) => run_filter(&options, stdin, stdout, stderr),
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "bounded format detection and planning stay visibly ahead of decoder execution"
)]
fn run_filter<R: Read + Send, W: Write, E: Write>(
    options: &RunOptions,
    stdin: &mut R,
    stdout: &mut W,
    stderr: &mut E,
) -> Result<ExitStatus, RunError> {
    validate_capability_policy(options)?;
    let (query_name, query) = load_filter(options)?;
    let variables = parse_external_arguments(options)?;
    let resolve_options = ResolveOptions {
        variables: variables
            .keys()
            .filter(|name| !name.starts_with("__tq_"))
            .cloned()
            .collect::<BTreeSet<_>>(),
        module_roots: options.module_paths.clone(),
        module_limit: options.limits.depth,
        module_bytes: usize::try_from(options.limits.input_bytes)
            .unwrap_or(usize::MAX)
            .min(ResolveOptions::default().module_bytes),
    };
    let parsed = parse_bytes(&query_name, &query).map_err(RunError::Compile)?;
    let resolved = resolve(parsed, &resolve_options).map_err(RunError::Compile)?;
    let automatic_mode =
        !options.stream && !options.slurp && !options.raw_input && !options.null_input;
    if automatic_mode && options.input_format == InputFormat::Auto {
        if options.proxy_on_error {
            let file_events = auto_file_events_available(options)?;
            if options.files.is_empty() || options.files.iter().any(|path| path == Path::new("-")) {
                let bytes = read_limited(&mut *stdin, options.limits.input_bytes, "<stdin>")?;
                let stdin_events = match probe_format(&bytes, options.limits.lookahead_bytes) {
                    Ok(probe) => decoder_events_available(probe.selected),
                    Err(error) if proxyable_format_error(&error) => false,
                    Err(error) => return Err(error.into()),
                };
                return run_resolved_filter(
                    options,
                    resolved,
                    &variables,
                    file_events && stdin_events,
                    None,
                    &[],
                    &mut bytes.as_slice(),
                    stdout,
                    stderr,
                );
            }
            return run_resolved_filter(
                options,
                resolved,
                &variables,
                file_events,
                None,
                &[],
                stdin,
                stdout,
                stderr,
            );
        }
        let file_events = auto_file_events_available(options)?;
        let common = auto_file_common_format(options)?;
        if options.files.is_empty() || options.files.iter().any(|path| path == Path::new("-")) {
            let reader = LimitedReader::new(&mut *stdin, options.limits.input_bytes, "<stdin>");
            let (probe, mut replay) = probe_reader(reader, options.limits.lookahead_bytes)?;
            let transcode_input = common
                .selected
                .filter(|format| *format == probe.selected)
                .or_else(|| options.files.is_empty().then_some(probe.selected));
            let mut detections = common.detections;
            detections.push(DetectionObservation::new("<stdin>", &probe));
            return run_resolved_filter(
                options,
                resolved,
                &variables,
                file_events && decoder_events_available(probe.selected),
                transcode_input,
                &detections,
                &mut replay,
                stdout,
                stderr,
            );
        }
        return run_resolved_filter(
            options,
            resolved,
            &variables,
            file_events,
            common.selected,
            &common.detections,
            stdin,
            stdout,
            stderr,
        );
    }
    run_resolved_filter(
        options,
        resolved,
        &variables,
        automatic_mode
            && matches!(
                options.input_format,
                InputFormat::Json | InputFormat::JsonLines | InputFormat::Toon
            ),
        match options.input_format {
            InputFormat::Json | InputFormat::Toon => Some(options.input_format),
            _ => None,
        },
        &[],
        stdin,
        stdout,
        stderr,
    )
}

struct CommonInputFormat {
    selected: Option<InputFormat>,
    detections: Vec<DetectionObservation>,
}

#[derive(Clone, Debug)]
struct DetectionObservation {
    identity: String,
    selected: InputFormat,
    lookahead_bytes: usize,
    commitment_bytes: usize,
    rejections: Vec<(InputFormat, String)>,
}

impl DetectionObservation {
    fn new(identity: impl Into<String>, report: &ProbeReport) -> Self {
        Self {
            identity: identity.into(),
            selected: report.selected,
            lookahead_bytes: report.lookahead_bytes,
            commitment_bytes: report.commitment_bytes,
            rejections: report.rejections.clone(),
        }
    }
}

fn auto_file_common_format(options: &RunOptions) -> Result<CommonInputFormat, RunError> {
    let mut common = None;
    let mut detections = Vec::new();
    for path in options.files.iter().filter(|path| *path != Path::new("-")) {
        let identity = path.display().to_string();
        let selected = if let Some(format) = format_from_path(path) {
            format
        } else {
            let reader =
                LimitedReader::new(open_path(path)?, options.limits.input_bytes, &identity);
            let (report, _) = probe_reader(reader, options.limits.lookahead_bytes)?;
            detections.push(DetectionObservation::new(identity, &report));
            report.selected
        };
        if common.is_some_and(|format| format != selected) {
            return Ok(CommonInputFormat {
                selected: None,
                detections,
            });
        }
        common = Some(selected);
    }
    Ok(CommonInputFormat {
        selected: common,
        detections,
    })
}

const fn decoder_events_available(format: InputFormat) -> bool {
    matches!(
        format,
        InputFormat::Json | InputFormat::JsonLines | InputFormat::Toon
    )
}

fn auto_file_events_available(options: &RunOptions) -> Result<bool, RunError> {
    let mut available = true;
    for path in options.files.iter().filter(|path| *path != Path::new("-")) {
        let identity = path.display().to_string();
        if let Some(format) = format_from_path(path) {
            available &= decoder_events_available(format);
        } else {
            let reader =
                LimitedReader::new(open_path(path)?, options.limits.input_bytes, &identity);
            match probe_reader(reader, options.limits.lookahead_bytes) {
                Ok((probe, _)) => available &= decoder_events_available(probe.selected),
                Err(error) if options.proxy_on_error && proxyable_format_error(&error) => {
                    available = false;
                }
                Err(error) => return Err(error.into()),
            }
        }
    }
    Ok(available)
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "the command lifecycle keeps pre-input planning visibly ahead of decoder execution"
)]
fn run_resolved_filter<R: Read + Send, W: Write, E: Write>(
    options: &RunOptions,
    resolved: Query<Resolved>,
    variables: &BTreeMap<Arc<str>, Value>,
    automatic_streaming: bool,
    transcode_input: Option<InputFormat>,
    detections: &[DetectionObservation],
    stdin: &mut R,
    stdout: &mut W,
    stderr: &mut E,
) -> Result<ExitStatus, RunError> {
    let analyzed = analyze_with_context(
        resolved,
        AnalysisContext {
            event_input: options.stream,
            whole_input: options.slurp,
            automatic_streaming,
        },
    );
    let mut analysis = analyzed.analysis().clone();
    if options.execution_override == ExecutionOverride::Document && !options.stream {
        analysis.selected_plan = PlanKind::Document;
        analysis.stream_rejection =
            Some("forced document override for differential run".to_owned());
    }
    if options.execution_override == ExecutionOverride::Document {
        analysis.transcode_rejection =
            Some("forced document override for differential run".to_owned());
    } else {
        match transcode_proof(options, analyzed.capabilities(), transcode_input) {
            Ok(proof) => {
                analysis.selected_plan = PlanKind::Transcode;
                analysis.transcode_proof = Some(proof);
                analysis.transcode_rejection = None;
            }
            Err(reason) => analysis.transcode_rejection = Some(reason.to_owned()),
        }
    }
    if let Some(explain) = options.explain {
        write_explain(explain, options, &analyzed, &analysis, stderr)?;
    }
    let program = analyzed.compile().map_err(RunError::Compile)?;

    if analysis.selected_plan == PlanKind::Transcode {
        let proof = analysis
            .transcode_proof
            .expect("selected transcode analysis carries a proof");
        let plan = program.transcode_plan(proof).map_err(RunError::Compile)?;
        return run_transcode_filter(options, &plan, &analysis, detections, stdin, stdout);
    }

    if options.stream {
        let plan = program.event_plan().map_err(RunError::Compile)?;
        return run_event_filter(options, &plan, variables, &analysis, stdin, stdout, stderr);
    }
    if matches!(
        analysis.selected_plan,
        PlanKind::Events | PlanKind::Subtree | PlanKind::HybridBlocking
    ) {
        return match program.automatic_plan().map_err(RunError::Compile)? {
            AutomaticPlan::Events(plan) => {
                run_automatic_filter(options, &plan, variables, &analysis, stdin, stdout, stderr)
            }
            AutomaticPlan::Subtree(plan) => {
                run_automatic_filter(options, &plan, variables, &analysis, stdin, stdout, stderr)
            }
            AutomaticPlan::HybridBlocking(plan) => {
                run_hybrid_filter(options, &plan, variables, &analysis, stdin, stdout, stderr)
            }
            _ => unreachable!("automatic selection returns an automatic typed plan"),
        };
    }
    let plan = program.document_plan();

    let mut result_output = ResultOutput::new(stdout, options);
    let mut result_count = 0_usize;
    let mut last = None;
    let mut observations = Vec::new();
    let mut runtime_error = None;
    {
        let mut evaluate = |input, input_cursor: Option<InputCursor>| -> Result<bool, RunError> {
            let input = match input {
                StructuredInput::Value(input) => input,
                StructuredInput::Proxy(bytes) => {
                    result_output.proxy(&bytes)?;
                    return Ok(true);
                }
            };
            let mut vm =
                Vm::new_with_variables(&plan, input, vm_limits(options), variables.clone())
                    .with_trace_limit(options.trace_limit);
            if let Some(cursor) = input_cursor {
                vm = vm.with_input_cursor(cursor);
            }
            if let Some(flag) = cancellation() {
                vm = vm.with_cancellation(flag);
            }
            loop {
                match vm.next_result() {
                    Ok(Some(value)) => {
                        last = Some(value.clone());
                        result_output.emit(&value)?;
                        result_count = result_count.saturating_add(1);
                    }
                    Ok(None) => break,
                    Err(error) => {
                        runtime_error = Some(error);
                        break;
                    }
                }
            }
            if options.trace_limit != 0 {
                for entry in vm.trace() {
                    writeln!(stderr, "trace: {entry}")?;
                }
            }
            observations.push(vm.observations());
            Ok(runtime_error.is_none())
        };

        if analysis.capabilities.whole_input
            && !options.slurp
            && !options.raw_input
            && !options.proxy_on_error
        {
            thread::scope(|scope| {
                let (sender, receiver) = sync_channel(REMAINING_INPUT_BUFFER_DOCUMENTS);
                scope.spawn(move || produce_remaining_inputs(options, stdin, &sender));
                let cursor = InputCursor::from_provider(move || match receiver.recv() {
                    Ok(RemainingInputMessage::Value(value)) => Ok(Some(value)),
                    Ok(RemainingInputMessage::Error(error)) => Err(error),
                    Ok(RemainingInputMessage::Done) | Err(_) => Ok(None),
                });
                let result: Result<(), RunError> = (|| {
                    while let Some(input) = pull_remaining_input(&cursor)? {
                        if !evaluate(StructuredInput::Value(input), Some(cursor.clone()))? {
                            break;
                        }
                    }
                    Ok(())
                })();
                drop(cursor);
                result
            })?;
        } else if analysis.capabilities.whole_input && !options.slurp && !options.raw_input {
            match load_inputs(options, stdin)? {
                LoadedInputs::Proxy(bytes) => {
                    let _ = evaluate(StructuredInput::Proxy(bytes), None)?;
                }
                LoadedInputs::Documents(inputs) => {
                    let cursor = InputCursor::from_input_values(
                        inputs
                            .into_iter()
                            .map(|document| InputValue {
                                value: document.value,
                                identity: Arc::from(document.identity),
                                line_number: document.index.saturating_add(1),
                            })
                            .collect(),
                    );
                    while let Some(input) = cursor.next_value()? {
                        if !evaluate(StructuredInput::Value(input), Some(cursor.clone()))? {
                            break;
                        }
                    }
                }
            }
        } else if options.slurp || options.raw_input {
            match load_inputs(options, stdin)? {
                LoadedInputs::Proxy(bytes) => {
                    let _ = evaluate(StructuredInput::Proxy(bytes), None)?;
                }
                LoadedInputs::Documents(inputs) => {
                    let values = if options.slurp && !options.raw_input {
                        vec![Value::array(
                            inputs
                                .into_iter()
                                .map(|document| document.value)
                                .collect::<Vec<_>>(),
                        )]
                    } else {
                        inputs.into_iter().map(|document| document.value).collect()
                    };
                    for input in values {
                        if !evaluate(StructuredInput::Value(input), None)? {
                            break;
                        }
                    }
                }
            }
        } else {
            for_each_structured_input(options, stdin, &mut |input| evaluate(input, None))?;
        }
    }

    if runtime_error.is_none() {
        result_output.finish()?;
    }
    if let Some(path) = &options.report_file {
        write_report(
            path,
            &observations,
            result_count,
            result_output.written(),
            options,
            analysis.selected_plan,
            ReportExecution {
                analysis: &analysis,
                retention: RetentionObservations::default(),
                resource_outcome: "success",
            },
        )?;
    }
    if let Some(error) = runtime_error {
        if let VmError::Input { message } = error {
            return Err(RunError::Input(FormatError::Parse {
                format: InputFormat::Auto,
                message: message.to_string(),
            }));
        }
        return Err(RunError::Runtime(error));
    }
    Ok(result_output.exit_status(options.exit_status, last.as_ref()))
}

enum StructuredInput {
    Value(Value),
    Proxy(Vec<u8>),
}

fn transcode_proof(
    options: &RunOptions,
    capabilities: tq_core::Capabilities,
    format: Option<InputFormat>,
) -> Result<TranscodeProof, &'static str> {
    if !capabilities.semantic_identity {
        return Err("query is not proven semantic identity");
    }
    if options.output_format != OutputFormat::Toon {
        return Err("selected output is not TOON");
    }
    if options.stream
        || options.slurp
        || options.raw_input
        || options.null_input
        || options.raw_output
        || options.join_output
        || options.raw_output0
        || options.proxy_on_error
    {
        return Err("selected CLI mode changes structured identity semantics");
    }
    if options.sort_keys {
        return Err("sorted-key output requires document execution");
    }
    if options.toon_writer.key_folding != KeyFolding::Off {
        return Err("safe key folding requires sibling collision analysis");
    }
    let (input, duplicate_policy) = match format {
        Some(InputFormat::Json) => (TranscodeInput::Json, TranscodeDuplicatePolicy::Reject),
        Some(InputFormat::Toon) if options.strict => {
            (TranscodeInput::Toon, TranscodeDuplicatePolicy::Reject)
        }
        Some(InputFormat::Toon) => return Err("non-strict TOON requires document execution"),
        _ => return Err("selected input is not one common JSON or strict TOON syntax"),
    };
    Ok(TranscodeProof {
        input,
        duplicate_policy,
        late_errors: true,
        canonical_toon_writer: true,
        key_folding_disabled: true,
        commitment: match options.framing {
            ToonFraming::Sequence => TranscodeCommitment::DirectSequence,
            ToonFraming::Unframed => TranscodeCommitment::AtomicUnframed,
        },
        limits: TranscodeLimits {
            maximum_memory_bytes: options.limits.preparation_memory_bytes as u64,
            maximum_spool_bytes: options.limits.spool_bytes,
            maximum_output_bytes: options.limits.output_bytes,
            maximum_depth: options.limits.depth,
            maximum_token_bytes: options.limits.token_bytes,
        },
    })
}

#[allow(
    clippy::too_many_lines,
    reason = "typed proof construction, commitment dispatch, and reporting stay in one lifecycle"
)]
fn run_transcode_filter<R: Read, W: Write>(
    options: &RunOptions,
    plan: &Plan<Compiled, Transcode>,
    analysis: &Analysis,
    detections: &[DetectionObservation],
    stdin: &mut R,
    stdout: &mut W,
) -> Result<ExitStatus, RunError> {
    let proof = *plan
        .transcode_proof()
        .expect("typed transcode plan carries its proof");
    let format = match proof.input {
        TranscodeInput::Json => InputFormat::Json,
        TranscodeInput::Toon => InputFormat::Toon,
    };
    let duplicate_keys = match proof.duplicate_policy {
        TranscodeDuplicatePolicy::Reject => DuplicateKeyPolicy::Reject,
    };
    let commitment = match proof.commitment {
        TranscodeCommitment::DirectSequence => tq_toon::TranscodeCommitment::DirectSequence,
        TranscodeCommitment::AtomicUnframed => tq_toon::TranscodeCommitment::AtomicUnframed,
    };
    let preparation = ArrayPreparationConfig {
        memory_threshold_bytes: options.limits.preparation_memory_bytes,
        maximum_spool_bytes: options.limits.spool_bytes,
        ..ArrayPreparationConfig::default()
    };
    let arena = PreparationArena::new(PreparationLimits {
        memory_bytes: options.limits.preparation_memory_bytes,
        spool_bytes: options.limits.spool_bytes,
        output_bytes: options.limits.output_bytes,
        nesting: options.limits.depth,
    });
    let mut output_bytes = 0_u64;

    let (execution, documents, last_truthy) = match proof.commitment {
        TranscodeCommitment::DirectSequence => {
            let writer = LimitedWriter::new(stdout, &mut output_bytes, options.limits.output_bytes);
            let mut consumer = TranscodeConsumer::new(
                writer,
                options.toon_writer,
                preparation,
                arena.clone(),
                duplicate_keys,
                commitment,
            )
            .with_document_limit(options.limits.results);
            if let Some(flag) = cancellation() {
                consumer = consumer.with_cancellation(flag);
            }
            let result = transcode_sources(options, format, stdin, &mut consumer);
            (result, consumer.documents(), consumer.last_truthy())
        }
        TranscodeCommitment::AtomicUnframed => {
            let publication = BufWriter::with_capacity(
                64 * 1024,
                PublicationBuffer::new(preparation.clone(), arena.clone()),
            );
            let mut consumer = TranscodeConsumer::new(
                publication,
                options.toon_writer,
                preparation,
                arena.clone(),
                duplicate_keys,
                commitment,
            )
            .with_document_limit(options.limits.results);
            if let Some(flag) = cancellation() {
                consumer = consumer.with_cancellation(flag);
            }
            let decode = transcode_sources(options, format, stdin, &mut consumer);
            let documents = consumer.documents();
            let last_truthy = consumer.last_truthy();
            let publication = consumer.into_inner();
            let result = decode.and_then(|()| {
                let mut publication = publication
                    .into_inner()
                    .map_err(|error| map_publication_buffer_error(error.into_error()))?;
                let mut writer =
                    LimitedWriter::new(stdout, &mut output_bytes, options.limits.output_bytes);
                publication
                    .publish_single(&mut writer, documents)
                    .map_err(map_publication_error)
            });
            (result, documents, last_truthy)
        }
    };

    if let Some(path) = &options.report_file {
        write_transcode_report(
            path,
            options,
            analysis,
            detections,
            TranscodeReportExecution {
                documents,
                output_bytes,
                observations: arena.observations(),
                resource_outcome: execution
                    .as_ref()
                    .map_or_else(|error| resource_outcome(error), |()| "success"),
            },
        )?;
    }
    execution?;
    Ok(if options.exit_status {
        match last_truthy {
            None => ExitStatus::NoResult,
            Some(false) => ExitStatus::FalseOrNull,
            Some(true) => ExitStatus::Success,
        }
    } else {
        ExitStatus::Success
    })
}

fn resource_outcome(error: &RunError) -> &'static str {
    match error.status() {
        ExitStatus::Resource => "resource-limit",
        ExitStatus::Interrupted => "interrupted",
        ExitStatus::Input => "input-error",
        ExitStatus::Runtime | ExitStatus::Unsupported => "output-error",
        ExitStatus::Usage
        | ExitStatus::Compile
        | ExitStatus::NoResult
        | ExitStatus::FalseOrNull => "error",
        ExitStatus::Success => "success",
    }
}

fn transcode_sources<R: Read, W: Write>(
    options: &RunOptions,
    format: InputFormat,
    stdin: &mut R,
    consumer: &mut TranscodeConsumer<W>,
) -> Result<(), RunError> {
    let files = if options.files.is_empty() {
        vec![Path::new("-").to_owned()]
    } else {
        options.files.clone()
    };
    for (index, path) in files.into_iter().enumerate() {
        let source = SourceId::new(u32::try_from(index + 1).unwrap_or(u32::MAX));
        if path == Path::new("-") {
            transcode_reader(options, format, &mut *stdin, "<stdin>", source, consumer)?;
        } else {
            let identity = path.display().to_string();
            transcode_reader(
                options,
                format,
                open_path(&path)?,
                &identity,
                source,
                consumer,
            )?;
        }
    }
    Ok(())
}

fn transcode_reader<R: Read, W: Write>(
    options: &RunOptions,
    format: InputFormat,
    reader: R,
    identity: &str,
    source: SourceId,
    consumer: &mut TranscodeConsumer<W>,
) -> Result<(), RunError> {
    let reader = LimitedReader::new(reader, options.limits.input_bytes, identity);
    match format {
        InputFormat::Json => decode_json_event_stream(
            buffered_input(reader),
            source,
            consumer,
            JsonEventOptions {
                maximum_depth: options.limits.depth,
                maximum_token_bytes: options.limits.token_bytes,
            },
        )
        .map(|_| ())
        .map_err(|message| map_transcode_message(InputFormat::Json, message)),
        InputFormat::Toon => {
            let mut decoder = Decoder::new(
                BufReader::new(reader),
                source,
                decode_options(options, InputFormat::Toon).toon,
            );
            decoder.decode_into(consumer).map_err(|error| match error {
                DecodeIntoError::Decode(error) => RunError::Input(FormatError::Parse {
                    format: InputFormat::Toon,
                    message: error.to_string(),
                }),
                DecodeIntoError::Consumer(error) => map_transcode_error(error),
            })
        }
        _ => unreachable!("typed transcode proof admits only JSON or TOON"),
    }
}

fn map_transcode_message(format: InputFormat, message: String) -> RunError {
    if message.contains("resource limit")
        || message.contains("limit exceeded")
        || message.contains("preparation")
        || message.contains("spool")
    {
        RunError::Resource("transcode-preparation")
    } else if message.contains("interrupted") || message.contains("cancelled") {
        RunError::Interrupted
    } else {
        RunError::Input(FormatError::Parse { format, message })
    }
}

fn map_transcode_error(error: TranscodeError) -> RunError {
    match error {
        TranscodeError::ResultLimit => RunError::Resource("result-count"),
        TranscodeError::Cancelled | TranscodeError::Spool(SpoolError::Cancelled) => {
            RunError::Interrupted
        }
        TranscodeError::Spool(SpoolError::Io(error))
        | TranscodeError::Writer(WriterError::Io(error))
        | TranscodeError::Io(error) => {
            if error.to_string().contains("output resource limit exceeded") {
                RunError::Resource("output-bytes")
            } else {
                RunError::Io(error)
            }
        }
        TranscodeError::Spool(_) => RunError::Resource("transcode-preparation"),
        TranscodeError::Duplicate(key) => RunError::Input(FormatError::Parse {
            format: InputFormat::Toon,
            message: format!("duplicate object key '{key}'"),
        }),
        TranscodeError::Structure(message) => RunError::Input(FormatError::Parse {
            format: InputFormat::Auto,
            message: message.to_owned(),
        }),
    }
}

fn map_publication_error(error: PublicationError) -> RunError {
    match error {
        PublicationError::Cardinality(_) => {
            RunError::Cardinality("unframed TOON requires exactly one result")
        }
        PublicationError::Spool(error) => map_transcode_error(TranscodeError::Spool(error)),
        PublicationError::Io(error) => RunError::Io(error),
    }
}

fn map_publication_buffer_error(error: io::Error) -> RunError {
    if error.to_string().contains("spool") || error.to_string().contains("resource limit") {
        RunError::Resource("transcode-preparation")
    } else {
        RunError::Io(error)
    }
}

fn for_each_structured_input<R: Read, F>(
    options: &RunOptions,
    stdin: &mut R,
    emit: &mut F,
) -> Result<(), RunError>
where
    F: FnMut(StructuredInput) -> Result<bool, RunError>,
{
    if options.null_input {
        let _ = emit(StructuredInput::Value(Value::Null))?;
        return Ok(());
    }
    let files = if options.files.is_empty() {
        vec![Path::new("-").to_owned()]
    } else {
        options.files.clone()
    };
    for path in files {
        let format = selected_input_format(options, &path);
        let keep_going = if path == Path::new("-") {
            for_each_structured_reader(options, format, &mut *stdin, "<stdin>", emit)?
        } else {
            let identity = path.display().to_string();
            for_each_structured_reader(options, format, open_path(&path)?, &identity, emit)?
        };
        if !keep_going {
            break;
        }
    }
    Ok(())
}

fn for_each_structured_reader<R: Read, F>(
    options: &RunOptions,
    format: InputFormat,
    reader: R,
    identity: &str,
    emit: &mut F,
) -> Result<bool, RunError>
where
    F: FnMut(StructuredInput) -> Result<bool, RunError>,
{
    if options.proxy_on_error {
        let bytes = read_limited(reader, options.limits.input_bytes, identity)?;
        let documents = match decode_bytes(&bytes, identity, decode_options(options, format)) {
            Ok(documents) => documents,
            Err(error) if proxyable_format_error(&error) => {
                return emit(StructuredInput::Proxy(bytes));
            }
            Err(error) => return Err(error.into()),
        };
        for document in documents {
            if !emit(StructuredInput::Value(document.value))? {
                return Ok(false);
            }
        }
        return Ok(true);
    }
    if format == InputFormat::JsonLines {
        let reader = LimitedReader::new(reader, options.limits.input_bytes, identity);
        let mut source = JsonLinesDocumentSource::new(
            BufReader::new(reader),
            identity,
            decode_options(options, format),
        );
        while let Some(document) = source.next_document()? {
            if !emit(StructuredInput::Value(document.value))? {
                return Ok(false);
            }
        }
        return Ok(true);
    }

    let bytes = read_limited(reader, options.limits.input_bytes, identity)?;
    for document in decode_bytes(&bytes, identity, decode_options(options, format))? {
        if !emit(StructuredInput::Value(document.value))? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn validate_capability_policy(options: &RunOptions) -> Result<(), RunError> {
    let uses_filesystem = matches!(&options.filter, FilterSource::File(_))
        || options.files.iter().any(|path| path != Path::new("-"))
        || !options.module_paths.is_empty()
        || options.report_file.is_some()
        || options.arguments.iter().any(|argument| {
            matches!(
                argument.kind,
                ExternalArgumentKind::RawFile | ExternalArgumentKind::SlurpFile
            )
        });
    if uses_filesystem && !options.capability_policy.filesystem {
        return Err(CliError::Incompatible(
            "filesystem access is disabled by capability policy".to_owned(),
        )
        .into());
    }
    if options.color == ColorMode::Always && !options.capability_policy.terminal {
        return Err(CliError::Incompatible(
            "--color-output is disabled by terminal capability policy".to_owned(),
        )
        .into());
    }
    if options.allow_environment && !options.capability_policy.environment {
        return Err(CliError::Incompatible(
            "environment access is disabled by capability policy".to_owned(),
        )
        .into());
    }
    if options.allow_platform && !options.capability_policy.platform {
        return Err(CliError::Incompatible(
            "platform access is disabled by capability policy".to_owned(),
        )
        .into());
    }
    Ok(())
}

const JSON_DUPLICATE_LIMITATION: &str = "JSON duplicates reject the current streamed record; use a document plan for last-value normalization";

fn duplicate_key_limitation(proof: Option<TranscodeProof>) -> Option<&'static str> {
    proof
        .is_some_and(|proof| proof.input == TranscodeInput::Json)
        .then_some(JSON_DUPLICATE_LIMITATION)
}

#[allow(
    clippy::too_many_lines,
    reason = "human and JSON plan explanations stay aligned in one function"
)]
fn write_explain(
    format: ExplainFormat,
    options: &RunOptions,
    analyzed: &Query<Analyzed>,
    analysis: &Analysis,
    stderr: &mut impl Write,
) -> Result<(), RunError> {
    let capabilities = analyzed.capabilities();
    let plan = analysis.selected_plan;
    let retained = match plan {
        PlanKind::Transcode => "decoder frames plus one shared bounded preparation arena",
        PlanKind::Events if capabilities.fold_state => {
            "decoder frames, current path, one event value, and one fold accumulator"
        }
        PlanKind::Events => "decoder frames, current path, and one scalar event value",
        PlanKind::WholeInput => "all input documents",
        PlanKind::Blocking => "one document plus blocking operator state",
        PlanKind::Subtree => "one selected complete subtree",
        PlanKind::HybridBlocking => {
            "bounded decoder state plus cardinality-proportional projected collection and blocking state"
        }
        PlanKind::Document => "one complete document",
    };
    let detection = input_format_name(options.input_format);
    match format {
        ExplainFormat::Human => {
            stderr.write_all(analyzed.explain().as_bytes())?;
            writeln!(stderr, "plan: {plan}")?;
            writeln!(stderr, "input-detection: {detection}")?;
            writeln!(stderr, "retained-working-set: {retained}")?;
            writeln!(stderr, "blocking: {}", capabilities.blocking)?;
            writeln!(stderr, "spool-required: {}", plan == PlanKind::Transcode)?;
            if let Some(proof) = &analysis.transcode_proof {
                writeln!(stderr, "identity-proof: semantic-identity")?;
                writeln!(stderr, "duplicate-policy: {:?}", proof.duplicate_policy)?;
                if let Some(limitation) = duplicate_key_limitation(Some(*proof)) {
                    writeln!(stderr, "duplicate-key-limitation: {limitation}")?;
                }
                writeln!(stderr, "commitment-mode: {:?}", proof.commitment)?;
            }
            if let Some(proof) = &analysis.stream_proof {
                writeln!(
                    stderr,
                    "required-path-prefix: {:?}",
                    proof.required_path_prefix
                )?;
                writeln!(stderr, "subtree-complete: {}", proof.subtree_complete)?;
                writeln!(stderr, "value-escapes: {}", proof.value_escapes)?;
                writeln!(stderr, "retention-high-water: available in --report-file")?;
            }
            if let Some(proof) = &analysis.hybrid_proof {
                writeln!(
                    stderr,
                    "collection-boundary: {}..{}",
                    proof.collection.start, proof.collection.end
                )?;
                writeln!(
                    stderr,
                    "blocking-cause: {}..{}",
                    proof.blocking_cause.start, proof.blocking_cause.end
                )?;
                writeln!(stderr, "hybrid-preparation: {:?}", proof.preparation)?;
                let (eligible, reason) = parallel_selected_decode_explain(options, analysis);
                writeln!(
                    stderr,
                    "parallel-selected-decode: {} ({reason})",
                    if eligible {
                        "eligible"
                    } else {
                        "serial-fallback"
                    }
                )?;
            }
            for rewrite in &analysis.optimizer_rewrites {
                writeln!(
                    stderr,
                    "optimizer-rewrite: {} at {}..{}",
                    rewrite.name, rewrite.span.start, rewrite.span.end
                )?;
            }
            if let Some(rejection) = &analysis.stream_rejection {
                writeln!(stderr, "stream-rejection: {rejection}")?;
            }
            if let Some(rejection) = &analysis.transcode_rejection {
                writeln!(stderr, "transcode-rejection: {rejection}")?;
            }
            writeln!(
                stderr,
                "limits: input={} depth={} token={} line={} lookahead={} vm-steps={} results={} output={} prepare-memory={} hybrid-batch-values={} hybrid-in-flight-batches={} hybrid-in-flight-bytes={} decode-batch-values={} decode-batch-bytes={} decode-in-flight-batches={} decode-in-flight-bytes={} spool={}",
                options.limits.input_bytes,
                options.limits.depth,
                options.limits.token_bytes,
                options.limits.line_bytes,
                options.limits.lookahead_bytes,
                options.limits.vm_steps,
                options.limits.results,
                options.limits.output_bytes,
                options.limits.preparation_memory_bytes,
                options.limits.hybrid_batch_values,
                options.limits.hybrid_in_flight_batches,
                options.limits.hybrid_in_flight_bytes,
                options.limits.decode_batch_values,
                options.limits.decode_batch_bytes,
                options.limits.decode_in_flight_batches,
                options.limits.decode_in_flight_bytes,
                options.limits.spool_bytes,
            )?;
        }
        ExplainFormat::Json => {
            let mut report = analyzed.explain_json();
            report["execution"] = serde_json::json!({
                "plan": plan.to_string(),
                "input_detection": detection,
                "retained_working_set": retained,
                "blocking": capabilities.blocking,
                "spool_required": plan == PlanKind::Transcode,
                "proof": if plan == PlanKind::Transcode {
                    serde_json::to_value(analysis.transcode_proof)?
                } else {
                    serde_json::to_value(&analysis.stream_proof)?
                },
                "stream_rejection": analysis.stream_rejection,
                "hybrid_proof": analysis.hybrid_proof,
                "parallel_selected_decode": {
                    "eligible": parallel_selected_decode_explain(options, analysis).0,
                    "reason": parallel_selected_decode_explain(options, analysis).1,
                    "worker_count": parallel_worker_count(),
                },
                "optimizer_rewrites": analysis.optimizer_rewrites,
                "transcode_rejection": analysis.transcode_rejection,
                "identity_proof": analysis.transcode_proof.map(|_| "semantic-identity"),
                "duplicate_policy": analysis.transcode_proof.map(|proof| proof.duplicate_policy),
                "duplicate_key_limitation": duplicate_key_limitation(analysis.transcode_proof),
                "commitment_mode": analysis.transcode_proof.map(|proof| proof.commitment),
                "high_water": {
                    "available_in_report": true
                },
                "limits": {
                    "input_bytes": options.limits.input_bytes,
                    "depth": options.limits.depth,
                    "token_bytes": options.limits.token_bytes,
                    "line_bytes": options.limits.line_bytes,
                    "lookahead_bytes": options.limits.lookahead_bytes,
                    "vm_steps": options.limits.vm_steps,
                    "results": options.limits.results,
                    "output_bytes": options.limits.output_bytes,
                    "preparation_memory_bytes": options.limits.preparation_memory_bytes,
                    "hybrid_batch_values": options.limits.hybrid_batch_values,
                    "hybrid_in_flight_batches": options.limits.hybrid_in_flight_batches,
                    "hybrid_in_flight_bytes": options.limits.hybrid_in_flight_bytes,
                    "decode_batch_values": options.limits.decode_batch_values,
                    "decode_batch_bytes": options.limits.decode_batch_bytes,
                    "decode_in_flight_batches": options.limits.decode_in_flight_batches,
                    "decode_in_flight_bytes": options.limits.decode_in_flight_bytes,
                    "spool_bytes": options.limits.spool_bytes,
                }
            });
            serde_json::to_writer_pretty(&mut *stderr, &report)?;
            stderr.write_all(b"\n")?;
        }
    }
    Ok(())
}

fn parallel_selected_decode_explain<'a>(
    options: &RunOptions,
    analysis: &'a Analysis,
) -> (bool, &'a str) {
    if analysis.selected_plan != PlanKind::HybridBlocking {
        return (
            false,
            analysis
                .stream_rejection
                .as_deref()
                .unwrap_or("plan-is-not-hybrid-blocking"),
        );
    }
    if parallel_worker_count() <= 1 {
        return (false, "one-worker-configured");
    }
    if !matches!(options.input_format, InputFormat::Auto | InputFormat::Json) {
        return (false, "input-format-is-not-json-document");
    }
    if analysis.hybrid_proof.is_none() {
        return (false, "hybrid-plan-has-no-static-prefix-proof");
    }
    (true, "static-array-prefix")
}

const fn input_format_name(format: InputFormat) -> &'static str {
    match format {
        InputFormat::Auto => "auto:toon/json/yaml-bounded-probe",
        InputFormat::Toon => "override:toon",
        InputFormat::Yaml => "override:yaml",
        InputFormat::Json => "override:json",
        InputFormat::JsonLines => "override:jsonl",
        InputFormat::ToonSequence => "override:toon-sequence",
    }
}

const fn concrete_input_format_name(format: InputFormat) -> &'static str {
    match format {
        InputFormat::Auto => "auto",
        InputFormat::Toon => "toon",
        InputFormat::Yaml => "yaml",
        InputFormat::Json => "json",
        InputFormat::JsonLines => "jsonl",
        InputFormat::ToonSequence => "toon-sequence",
    }
}

fn run_event_filter<R: Read, W: Write, E: Write>(
    options: &RunOptions,
    plan: &Plan<Compiled, Events>,
    variables: &BTreeMap<Arc<str>, Value>,
    analysis: &Analysis,
    stdin: &mut R,
    stdout: &mut W,
    stderr: &mut E,
) -> Result<ExitStatus, RunError> {
    let mut executor = StreamExecutor {
        plan,
        variables,
        output: ResultOutput::new(stdout, options),
        stderr,
        trace_remaining: options.trace_limit,
        observations: VmObservations::default(),
        last: None,
        results: 0,
    };
    let files = if options.files.is_empty() {
        vec![Path::new("-").to_owned()]
    } else {
        options.files.clone()
    };
    for path in files {
        if cancellation().is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err(RunError::Interrupted);
        }
        if path == Path::new("-") {
            stream_reader(
                options,
                selected_input_format(options, &path),
                &mut *stdin,
                "<stdin>",
                &mut executor,
            )?;
        } else {
            let identity = path.display().to_string();
            stream_reader(
                options,
                selected_input_format(options, &path),
                open_path(&path)?,
                &identity,
                &mut executor,
            )?;
        }
    }
    executor.output.finish()?;
    if let Some(path) = &options.report_file {
        write_report(
            path,
            &[executor.observations],
            executor.results,
            executor.output.written(),
            options,
            PlanKind::Events,
            ReportExecution {
                analysis,
                retention: RetentionObservations::default(),
                resource_outcome: "success",
            },
        )?;
    }
    Ok(executor
        .output
        .exit_status(options.exit_status, executor.last.as_ref()))
}

#[derive(Clone, Copy, Debug, Default)]
struct RetentionObservations {
    bytes_high_water: usize,
    depth_high_water: usize,
    decoder_depth_high_water: usize,
    completed_subtrees: u64,
    retained_results_high_water: usize,
    retained_bytes_high_water: usize,
    sort_runs: usize,
    in_flight_batches_high_water: usize,
    in_flight_bytes_high_water: usize,
    decode_batches: usize,
    decode_in_flight_batches_high_water: usize,
    decode_in_flight_bytes_high_water: usize,
    decode_reordered_batches_high_water: usize,
}

fn run_automatic_filter<R: Read, W: Write, E: Write, M>(
    options: &RunOptions,
    plan: &Plan<Compiled, M>,
    variables: &BTreeMap<Arc<str>, Value>,
    analysis: &Analysis,
    stdin: &mut R,
    stdout: &mut W,
    stderr: &mut E,
) -> Result<ExitStatus, RunError> {
    let mut executor = AutomaticExecutor {
        plan,
        prefix: plan
            .automatic_prefix()
            .expect("automatic plan has a proven prefix")
            .to_vec(),
        projection: plan.automatic_projection().map(<[PathComponent]>::to_vec),
        variables,
        output: ResultOutput::new(stdout, options),
        stderr,
        trace_remaining: options.trace_limit,
        observations: VmObservations::default(),
        retention: RetentionObservations::default(),
        current: None,
        current_item: None,
        deferred_item: None,
        collected: None,
        hybrid_suffix: None,
        last: None,
        results: 0,
        current_retained_results: 0,
        current_retained_bytes: 0,
    };
    let files = if options.files.is_empty() {
        vec![Path::new("-").to_owned()]
    } else {
        options.files.clone()
    };
    for path in files {
        if cancellation().is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err(RunError::Interrupted);
        }
        if path == Path::new("-") {
            automatic_reader(
                options,
                selected_input_format(options, &path),
                &mut *stdin,
                "<stdin>",
                &mut executor,
            )?;
        } else {
            let identity = path.display().to_string();
            automatic_reader(
                options,
                selected_input_format(options, &path),
                open_path(&path)?,
                &identity,
                &mut executor,
            )?;
        }
    }
    executor.output.finish()?;
    if let Some(path) = &options.report_file {
        write_report(
            path,
            &[executor.observations],
            executor.results,
            executor.output.written(),
            options,
            analysis.selected_plan,
            ReportExecution {
                analysis,
                retention: executor.retention,
                resource_outcome: "success",
            },
        )?;
    }
    Ok(executor
        .output
        .exit_status(options.exit_status, executor.last.as_ref()))
}

fn run_hybrid_filter<R: Read, W: Write, E: Write>(
    options: &RunOptions,
    plan: &Plan<Compiled, HybridBlocking>,
    variables: &BTreeMap<Arc<str>, Value>,
    analysis: &Analysis,
    stdin: &mut R,
    stdout: &mut W,
    stderr: &mut E,
) -> Result<ExitStatus, RunError> {
    let mut executor = AutomaticExecutor {
        plan,
        prefix: plan
            .automatic_prefix()
            .expect("hybrid plan has a proven producer prefix")
            .to_vec(),
        projection: plan.automatic_projection().map(<[PathComponent]>::to_vec),
        variables,
        output: ResultOutput::new(stdout, options),
        stderr,
        trace_remaining: options.trace_limit,
        observations: VmObservations::default(),
        retention: RetentionObservations::default(),
        current: None,
        current_item: None,
        deferred_item: None,
        collected: Some(HybridCollection::new(plan, options)),
        hybrid_suffix: Some(plan),
        last: None,
        results: 0,
        current_retained_results: 0,
        current_retained_bytes: 0,
    };
    let files = if options.files.is_empty() {
        vec![Path::new("-").to_owned()]
    } else {
        options.files.clone()
    };
    let execution = (|| {
        for path in files {
            if cancellation().is_some_and(|flag| flag.load(Ordering::Relaxed)) {
                return Err(RunError::Interrupted);
            }
            if path == Path::new("-") {
                automatic_reader(
                    options,
                    selected_input_format(options, &path),
                    &mut *stdin,
                    "<stdin>",
                    &mut executor,
                )?;
            } else {
                let identity = path.display().to_string();
                automatic_reader(
                    options,
                    selected_input_format(options, &path),
                    open_path(&path)?,
                    &identity,
                    &mut executor,
                )?;
            }
        }
        executor.output.finish()?;
        Ok(executor
            .output
            .exit_status(options.exit_status, executor.last.as_ref()))
    })();
    if let Some(path) = &options.report_file {
        write_report(
            path,
            &[executor.observations],
            executor.results,
            executor.output.written(),
            options,
            PlanKind::HybridBlocking,
            ReportExecution {
                analysis,
                retention: executor.retention,
                resource_outcome: execution
                    .as_ref()
                    .map_or_else(|error| resource_outcome(error), |_| "success"),
            },
        )?;
    }
    execution
}

fn automatic_reader<R: Read, W: Write, E: Write, M>(
    options: &RunOptions,
    format: InputFormat,
    reader: R,
    identity: &str,
    executor: &mut AutomaticExecutor<'_, W, E, M>,
) -> Result<(), RunError> {
    if options.proxy_on_error {
        let bytes = read_limited(reader, options.limits.input_bytes, identity)?;
        match validate_proxy_event_source(&bytes, identity, options, format) {
            Ok(()) => {
                return automatic_reader_inner(
                    options,
                    format,
                    bytes.as_slice(),
                    identity,
                    executor,
                );
            }
            Err(error) if proxyable_format_error(&error) => {
                return executor.output.proxy(&bytes);
            }
            Err(error) => return Err(error.into()),
        }
    }
    automatic_reader_inner(options, format, reader, identity, executor)
}

fn automatic_reader_inner<R: Read, W: Write, E: Write, M>(
    options: &RunOptions,
    format: InputFormat,
    reader: R,
    identity: &str,
    executor: &mut AutomaticExecutor<'_, W, E, M>,
) -> Result<(), RunError> {
    let stream_options = StreamOptions {
        maximum_depth: options.limits.depth,
        maximum_token_bytes: options.limits.token_bytes,
        errors_as_values: false,
    };
    let reader = LimitedReader::new(reader, options.limits.input_bytes, identity);
    match format {
        InputFormat::Json => {
            automatic_json_into(buffered_input(reader), stream_options, true, executor)?;
            executor.finish_source()
        }
        InputFormat::JsonLines => {
            automatic_json_lines_into(reader, identity, options, stream_options, executor)
        }
        InputFormat::Toon => {
            automatic_toon_into(reader, options, stream_options, executor)?;
            executor.finish_source()
        }
        InputFormat::Auto => {
            let (report, replay) = probe_reader(reader, options.limits.lookahead_bytes)?;
            match report.selected {
                InputFormat::Json => {
                    automatic_json_into(buffered_input(replay), stream_options, true, executor)?;
                    executor.finish_source()
                }
                InputFormat::Toon => {
                    automatic_toon_into(replay, options, stream_options, executor)?;
                    executor.finish_source()
                }
                InputFormat::Yaml => Err(RunError::Unsupported(
                    "auto-detected YAML cannot execute a decoder-event plan".to_owned(),
                )),
                InputFormat::Auto | InputFormat::JsonLines | InputFormat::ToonSequence => {
                    unreachable!("probe candidate")
                }
            }
        }
        InputFormat::Yaml | InputFormat::ToonSequence => Err(RunError::Unsupported(
            "automatic bounded plans require JSON or TOON decoder events".to_owned(),
        )),
    }
}

fn automatic_json_lines_into<R: Read, W: Write, E: Write, M>(
    reader: R,
    identity: &str,
    options: &RunOptions,
    stream_options: StreamOptions,
    executor: &mut AutomaticExecutor<'_, W, E, M>,
) -> Result<(), RunError> {
    let mut source = JsonLinesDocumentSource::new(
        BufReader::new(reader),
        identity,
        decode_options(options, InputFormat::JsonLines),
    );
    while let Some((record, line)) = source.next_record()? {
        automatic_json_into(record.as_slice(), stream_options, false, executor)
            .map_err(|error| json_lines_record_error(error, identity, line))?;
        executor.finish_source()?;
    }
    Ok(())
}

fn automatic_json_into<R: io::BufRead, W: Write, E: Write, M>(
    reader: R,
    options: StreamOptions,
    parallel_allowed: bool,
    executor: &mut AutomaticExecutor<'_, W, E, M>,
) -> Result<(), RunError> {
    let mut execution_error = None;
    let selection = executor.stream_selection();
    let parallel =
        parallel_allowed && executor.hybrid_suffix.is_some() && parallel_worker_count() > 1;
    let parallel_options = ParallelJsonOptions {
        batch_values: executor.output.options.limits.decode_batch_values,
        batch_bytes: executor.output.options.limits.decode_batch_bytes,
        in_flight_batches: executor.output.options.limits.decode_in_flight_batches,
        in_flight_bytes: executor.output.options.limits.decode_in_flight_bytes,
    };
    let decoded = {
        let mut accept = |record| match executor.accept(record) {
            Ok(()) => Ok(()),
            Err(error) => {
                execution_error = Some(error);
                Err("automatic stream consumer stopped".to_owned())
            }
        };
        if parallel {
            stream_json_selected_records_parallel(
                reader,
                options,
                selection,
                parallel_options,
                cancellation(),
                &mut accept,
            )
        } else {
            let mut selected = SelectedStreamObservations::default();
            let result = stream_json_selected_records_with_control(
                reader,
                options,
                selection,
                cancellation(),
                &mut selected,
                &mut accept,
            );
            executor.retention.decoder_depth_high_water = executor
                .retention
                .decoder_depth_high_water
                .max(selected.depth_high_water);
            result.map(|()| ParallelJsonObservations::default())
        }
    };
    if let Ok(observations) = decoded.as_ref() {
        executor.retention.decode_batches = executor
            .retention
            .decode_batches
            .saturating_add(observations.batches);
        executor.retention.decoder_depth_high_water = executor
            .retention
            .decoder_depth_high_water
            .max(observations.depth_high_water);
        executor.retention.decode_in_flight_batches_high_water = executor
            .retention
            .decode_in_flight_batches_high_water
            .max(observations.in_flight_batches_high_water);
        executor.retention.decode_in_flight_bytes_high_water = executor
            .retention
            .decode_in_flight_bytes_high_water
            .max(observations.in_flight_bytes_high_water);
        executor.retention.decode_reordered_batches_high_water = executor
            .retention
            .decode_reordered_batches_high_water
            .max(observations.reordered_batches_high_water);
    }
    if let Some(error) = execution_error {
        return Err(error);
    }
    if decoded.is_err() && cancellation().is_some_and(|flag| flag.load(Ordering::Relaxed)) {
        return Err(RunError::Interrupted);
    }
    decoded.map(|_| ()).map_err(RunError::Input)
}

fn automatic_toon_into<R: Read, W: Write, E: Write, M>(
    reader: R,
    options: &RunOptions,
    stream_options: StreamOptions,
    executor: &mut AutomaticExecutor<'_, W, E, M>,
) -> Result<(), RunError> {
    let mut execution_error = None;
    let selection = executor.stream_selection();
    let mut selected = SelectedStreamObservations::default();
    let decoded = stream_toon_selected_records_with_control(
        BufReader::new(reader),
        tq_toon::DecoderConfig {
            strict: options.strict,
            maximum_depth: options.limits.depth,
            maximum_token_bytes: options.limits.token_bytes,
            maximum_line_bytes: options.limits.line_bytes,
            maximum_lookahead_bytes: options.limits.lookahead_bytes,
            ..tq_toon::DecoderConfig::default()
        },
        stream_options,
        selection,
        cancellation(),
        &mut selected,
        |record| match executor.accept(record) {
            Ok(()) => Ok(()),
            Err(error) => {
                execution_error = Some(error);
                Err("automatic stream consumer stopped".to_owned())
            }
        },
    );
    executor.retention.decoder_depth_high_water = executor
        .retention
        .decoder_depth_high_water
        .max(selected.depth_high_water);
    if let Some(error) = execution_error {
        return Err(error);
    }
    if decoded.is_err() && cancellation().is_some_and(|flag| flag.load(Ordering::Relaxed)) {
        return Err(RunError::Interrupted);
    }
    decoded.map_err(RunError::Input)
}

enum HybridCollection {
    Collect(Vec<Value>),
    StableSort {
        pipeline: Option<StableSortPipeline>,
        config: (usize, usize, usize, usize),
    },
}

impl HybridCollection {
    fn new(plan: &Plan<Compiled, HybridBlocking>, options: &RunOptions) -> Self {
        if plan.hybrid_preparation() == HybridPreparation::StableSortRuns {
            let in_flight_bytes = options.limits.hybrid_in_flight_bytes.max(1);
            let in_flight_batches = options.limits.hybrid_in_flight_batches.max(1);
            let config = (
                options.limits.hybrid_batch_values.max(1),
                (in_flight_bytes / in_flight_batches).max(1),
                in_flight_batches,
                in_flight_bytes,
            );
            Self::StableSort {
                pipeline: Some(hybrid_sort_pipeline(config)),
                config,
            }
        } else {
            Self::Collect(Vec::new())
        }
    }

    fn push(&mut self, value: Value) -> Result<(), RunError> {
        match self {
            Self::Collect(values) => {
                values.push(value);
                Ok(())
            }
            Self::StableSort { pipeline, .. } => {
                let bytes = estimate_value_bytes(&value).saturating_add(32);
                pipeline
                    .as_mut()
                    .expect("active hybrid sort pipeline")
                    .push(value, bytes)
                    .map_err(hybrid_pipeline_error)
            }
        }
    }

    fn finish_document(
        &mut self,
    ) -> Result<(Vec<Value>, StableSortPipelineObservations), RunError> {
        match self {
            Self::Collect(values) => Ok((
                std::mem::take(values),
                StableSortPipelineObservations::default(),
            )),
            Self::StableSort { pipeline, config } => {
                let active = pipeline.take().expect("active hybrid sort pipeline");
                let (values, observations) = active.finish().map_err(hybrid_pipeline_error)?;
                *pipeline = Some(hybrid_sort_pipeline(*config));
                Ok((values, observations))
            }
        }
    }
}

fn hybrid_sort_pipeline(config: (usize, usize, usize, usize)) -> StableSortPipeline {
    let pipeline = StableSortPipeline::new(config.0, config.1, config.2, config.3);
    if let Some(flag) = cancellation() {
        pipeline.with_cancellation(flag)
    } else {
        pipeline
    }
}

fn hybrid_pipeline_error(resource: &'static str) -> RunError {
    if resource == "interrupted" {
        RunError::Interrupted
    } else {
        RunError::Resource(resource)
    }
}

struct AutomaticExecutor<'a, W, E, M> {
    plan: &'a Plan<Compiled, M>,
    prefix: Vec<PathComponent>,
    projection: Option<Vec<PathComponent>>,
    variables: &'a BTreeMap<Arc<str>, Value>,
    output: ResultOutput<'a, W>,
    stderr: &'a mut E,
    trace_remaining: usize,
    observations: VmObservations,
    retention: RetentionObservations,
    current: Option<Capture>,
    current_item: Option<Vec<PathComponent>>,
    deferred_item: Option<Value>,
    collected: Option<HybridCollection>,
    hybrid_suffix: Option<&'a Plan<Compiled, HybridBlocking>>,
    last: Option<Value>,
    results: usize,
    current_retained_results: usize,
    current_retained_bytes: usize,
}

impl<W: Write, E: Write, M> AutomaticExecutor<'_, W, E, M> {
    fn stream_selection(&self) -> StreamSelection {
        StreamSelection::new(self.prefix.clone(), self.projection.clone())
    }

    fn accept(&mut self, record: StreamRecord) -> Result<(), RunError> {
        if cancellation().is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err(RunError::Interrupted);
        }
        let (path, value) = record.into_parts();
        let record_bytes = estimate_event_bytes(&path, value.as_ref());
        if path == self.prefix {
            if let Some(value) = value
                && !matches!(&value, Value::Array(values) if values.is_empty())
                && !matches!(&value, Value::Object(values) if values.is_empty())
            {
                self.observe_complete_value(record_bytes, 0)?;
                self.evaluate(value, true)?;
            }
            return Ok(());
        }
        if path.len() <= self.prefix.len() || !path.starts_with(&self.prefix) {
            return Ok(());
        }
        if self.projection.is_some() {
            return self.accept_projection(&path, value, record_bytes);
        }
        let target_length = self.prefix.len().saturating_add(1);
        let target = path[..target_length].to_vec();
        let relative = &path[target_length..];

        if self
            .current
            .as_ref()
            .is_some_and(|capture| capture.path != target)
        {
            self.complete_capture()?;
        }

        if self.plan.scalar_events_only() {
            if relative.is_empty()
                && let Some(value) = value
            {
                self.evaluate(value, false)?;
            }
            return Ok(());
        }

        if relative.is_empty() {
            if let Some(value) = value {
                self.observe_complete_value(record_bytes, 0)?;
                self.retention.completed_subtrees =
                    self.retention.completed_subtrees.saturating_add(1);
                self.evaluate(value, false)?;
            } else if self
                .current
                .as_ref()
                .is_some_and(|capture| capture.path == target)
            {
                let capture = self.current.take().expect("capture was checked");
                self.retention.completed_subtrees =
                    self.retention.completed_subtrees.saturating_add(1);
                self.evaluate(capture.root.into_value()?, false)?;
            }
            return Ok(());
        }
        let Some(value) = value else {
            return Ok(());
        };
        let capture = self.current.get_or_insert_with(|| Capture {
            path: target,
            root: BuildNode::empty_for(&relative[0]),
            bytes: 0,
        });
        capture.bytes = capture
            .bytes
            .saturating_add(record_bytes.saturating_add(64));
        if capture.bytes > self.output.options.limits.preparation_memory_bytes {
            return Err(RunError::Resource("subtree-bytes"));
        }
        let relative_depth = relative.len();
        if relative_depth > self.output.options.limits.depth {
            return Err(RunError::Resource("subtree-depth"));
        }
        capture.root.insert(relative, value)?;
        self.retention.bytes_high_water = self.retention.bytes_high_water.max(capture.bytes);
        self.retention.depth_high_water = self.retention.depth_high_water.max(relative_depth);
        Ok(())
    }

    fn finish_source(&mut self) -> Result<(), RunError> {
        if self.projection.is_some() {
            self.complete_projection_item()?;
        } else {
            self.complete_capture()?;
        }
        if let Some(plan) = self.hybrid_suffix {
            self.finish_hybrid_document(plan)?;
        }
        Ok(())
    }

    fn accept_projection(
        &mut self,
        path: &[PathComponent],
        value: Option<Value>,
        record_bytes: usize,
    ) -> Result<(), RunError> {
        let item_length = self.prefix.len().saturating_add(1);
        let item = path[..item_length].to_vec();
        if self
            .current_item
            .as_ref()
            .is_some_and(|current| *current != item)
        {
            self.complete_projection_item()?;
        }
        if self.current_item.is_none() {
            self.current_item = Some(item);
        }
        let relative = &path[item_length..];
        if relative.is_empty() {
            if let Some(value) = value {
                self.observe_complete_value(record_bytes, 0)?;
                self.deferred_item = Some(value);
            }
            return Ok(());
        }
        let projection = self
            .projection
            .as_deref()
            .expect("projection branch requires a projected path");
        if path_kind_mismatch(relative, projection) {
            if let Some(value) = value {
                self.deferred_item = Some(synthetic_item(relative, value)?);
            }
            return Ok(());
        }
        if relative == projection {
            if let Some(value) = value {
                self.observe_complete_value(record_bytes, 0)?;
                self.current = Some(Capture {
                    path: path.to_vec(),
                    root: BuildNode::Value(value),
                    bytes: record_bytes,
                });
            }
            return Ok(());
        }
        if projection.starts_with(relative) {
            if let Some(value) = value {
                self.deferred_item = Some(synthetic_item(relative, value)?);
            }
            return Ok(());
        }
        if !relative.starts_with(projection) {
            return Ok(());
        }
        let captured_path = &relative[projection.len()..];
        let Some(value) = value else {
            return Ok(());
        };
        let capture = self.current.get_or_insert_with(|| Capture {
            path: path[..item_length + projection.len()].to_vec(),
            root: BuildNode::empty_for(&captured_path[0]),
            bytes: 0,
        });
        capture.bytes = capture
            .bytes
            .saturating_add(record_bytes.saturating_add(64));
        if capture.bytes > self.output.options.limits.preparation_memory_bytes {
            return Err(RunError::Resource("subtree-bytes"));
        }
        capture.root.insert(captured_path, value)?;
        self.retention.bytes_high_water = self.retention.bytes_high_water.max(capture.bytes);
        self.retention.depth_high_water = self.retention.depth_high_water.max(captured_path.len());
        Ok(())
    }

    fn complete_projection_item(&mut self) -> Result<(), RunError> {
        if self.current_item.take().is_none() {
            return Ok(());
        }
        self.retention.completed_subtrees = self.retention.completed_subtrees.saturating_add(1);
        if let Some(capture) = self.current.take() {
            self.emit_direct(capture.root.into_value()?)?;
        } else if let Some(item) = self.deferred_item.take() {
            self.evaluate(item, false)?;
        } else {
            self.emit_direct(Value::Null)?;
        }
        Ok(())
    }

    fn emit_direct(&mut self, value: Value) -> Result<(), RunError> {
        if self.collected.is_some() {
            self.observe_retained_value(&value);
        }
        if let Some(collected) = self.collected.as_mut() {
            return collected.push(value);
        }
        self.output.emit(&value)?;
        self.last = Some(value);
        self.results = self.results.saturating_add(1);
        Ok(())
    }

    fn complete_capture(&mut self) -> Result<(), RunError> {
        let Some(capture) = self.current.take() else {
            return Ok(());
        };
        self.retention.completed_subtrees = self.retention.completed_subtrees.saturating_add(1);
        self.evaluate(capture.root.into_value()?, false)
    }

    fn observe_complete_value(&mut self, bytes: usize, depth: usize) -> Result<(), RunError> {
        if !self.plan.scalar_events_only()
            && bytes > self.output.options.limits.preparation_memory_bytes
        {
            return Err(RunError::Resource("subtree-bytes"));
        }
        self.retention.bytes_high_water = self.retention.bytes_high_water.max(bytes);
        self.retention.depth_high_water = self.retention.depth_high_water.max(depth);
        Ok(())
    }

    fn evaluate(&mut self, input: Value, base: bool) -> Result<(), RunError> {
        let limits = self.remaining_vm_limits();
        let mut vm = if base {
            Vm::new_automatic_base(self.plan, input, limits, self.variables.clone())
        } else {
            Vm::new_automatic_item(self.plan, input, limits, self.variables.clone())
        }
        .with_trace_limit(self.trace_remaining);
        if let Some(flag) = cancellation() {
            vm = vm.with_cancellation(flag);
        }
        let mut output_error = None;
        let evaluated = vm.for_each_result(|value| {
            if self.collected.is_some() {
                self.observe_retained_value(&value);
            }
            if let Some(collected) = self.collected.as_mut() {
                if let Err(error) = collected.push(value) {
                    output_error = Some(error);
                    return false;
                }
                return true;
            }
            if let Err(error) = self.output.emit(&value) {
                output_error = Some(error);
                return false;
            }
            self.last = Some(value);
            self.results = self.results.saturating_add(1);
            true
        });
        merge_observations(&mut self.observations, vm.observations());
        if let Some(error) = output_error {
            return Err(error);
        }
        evaluated.map_err(RunError::Runtime)?;
        if self.trace_remaining != 0 {
            for entry in vm.trace() {
                writeln!(self.stderr, "trace: {entry}")?;
            }
            self.trace_remaining = self.trace_remaining.saturating_sub(vm.trace().len());
        }
        Ok(())
    }

    fn finish_hybrid_document(
        &mut self,
        plan: &Plan<Compiled, HybridBlocking>,
    ) -> Result<(), RunError> {
        let (values, preparation) = self
            .collected
            .as_mut()
            .expect("hybrid execution collects producer results")
            .finish_document()?;
        self.retention.sort_runs = self.retention.sort_runs.saturating_add(preparation.batches);
        self.retention.in_flight_batches_high_water = self
            .retention
            .in_flight_batches_high_water
            .max(preparation.in_flight_batches);
        self.retention.in_flight_bytes_high_water = self
            .retention
            .in_flight_bytes_high_water
            .max(preparation.in_flight_bytes);
        let mut vm = Vm::new_hybrid_suffix(
            plan,
            Value::array(values),
            self.remaining_vm_limits(),
            self.variables.clone(),
        )
        .with_trace_limit(self.trace_remaining);
        if let Some(flag) = cancellation() {
            vm = vm.with_cancellation(flag);
        }
        let mut output_error = None;
        let evaluated = vm.for_each_result(|value| {
            if let Err(error) = self.output.emit(&value) {
                output_error = Some(error);
                return false;
            }
            self.last = Some(value);
            self.results = self.results.saturating_add(1);
            true
        });
        merge_observations(&mut self.observations, vm.observations());
        self.current_retained_results = 0;
        self.current_retained_bytes = 0;
        if let Some(error) = output_error {
            return Err(error);
        }
        evaluated.map_err(RunError::Runtime)?;
        if self.trace_remaining != 0 {
            for entry in vm.trace() {
                writeln!(self.stderr, "trace: {entry}")?;
            }
            self.trace_remaining = self.trace_remaining.saturating_sub(vm.trace().len());
        }
        Ok(())
    }

    fn remaining_vm_limits(&self) -> VmLimits {
        let mut limits = vm_limits(self.output.options);
        limits.steps = limits.steps.saturating_sub(self.observations.steps);
        limits
    }

    fn observe_retained_value(&mut self, value: &Value) {
        self.current_retained_results = self.current_retained_results.saturating_add(1);
        self.current_retained_bytes = self
            .current_retained_bytes
            .saturating_add(estimate_value_bytes(value).saturating_add(32));
        self.retention.retained_results_high_water = self
            .retention
            .retained_results_high_water
            .max(self.current_retained_results);
        self.retention.retained_bytes_high_water = self
            .retention
            .retained_bytes_high_water
            .max(self.current_retained_bytes);
    }
}

struct Capture {
    path: Vec<PathComponent>,
    root: BuildNode,
    bytes: usize,
}

enum BuildNode {
    Missing,
    Value(Value),
    Array(Vec<BuildNode>),
    Object(Vec<(Arc<str>, BuildNode)>),
}

impl BuildNode {
    fn empty_for(component: &PathComponent) -> Self {
        match component {
            PathComponent::Index(_) => Self::Array(Vec::new()),
            PathComponent::Key(_) => Self::Object(Vec::new()),
        }
    }

    fn insert(&mut self, path: &[PathComponent], value: Value) -> Result<(), RunError> {
        let Some((component, tail)) = path.split_first() else {
            *self = Self::Value(value);
            return Ok(());
        };
        match component {
            PathComponent::Index(index) => {
                let Self::Array(values) = self else {
                    return Err(invalid_automatic_event());
                };
                if values.len() <= *index {
                    values.resize_with(index.saturating_add(1), || Self::Missing);
                }
                if !tail.is_empty() && matches!(values[*index], Self::Missing) {
                    values[*index] = Self::empty_for(&tail[0]);
                }
                values[*index].insert(tail, value)
            }
            PathComponent::Key(key) => {
                let Self::Object(values) = self else {
                    return Err(invalid_automatic_event());
                };
                let index = values.iter().position(|(candidate, _)| candidate == key);
                let child = if let Some(index) = index {
                    &mut values[index].1
                } else {
                    values.push((Arc::clone(key), Self::Missing));
                    &mut values.last_mut().expect("entry was pushed").1
                };
                if !tail.is_empty() && matches!(child, Self::Missing) {
                    *child = Self::empty_for(&tail[0]);
                }
                child.insert(tail, value)
            }
        }
    }

    fn into_value(self) -> Result<Value, RunError> {
        match self {
            Self::Missing => Err(invalid_automatic_event()),
            Self::Value(value) => Ok(value),
            Self::Array(values) => values
                .into_iter()
                .map(Self::into_value)
                .collect::<Result<Vec<_>, _>>()
                .map(Value::array),
            Self::Object(values) => values
                .into_iter()
                .map(|(key, value)| Ok((key, value.into_value()?)))
                .collect::<Result<tq_core::Object, RunError>>()
                .map(Value::object),
        }
    }
}

fn path_kind_mismatch(actual: &[PathComponent], expected: &[PathComponent]) -> bool {
    for (actual, expected) in actual.iter().zip(expected) {
        if actual == expected {
            continue;
        }
        return matches!(actual, PathComponent::Key(_))
            != matches!(expected, PathComponent::Key(_));
    }
    false
}

fn synthetic_item(relative: &[PathComponent], value: Value) -> Result<Value, RunError> {
    let Some(first) = relative.first() else {
        return Ok(value);
    };
    let mut root = BuildNode::empty_for(first);
    root.insert(relative, value)?;
    root.into_value()
}

fn estimate_event_bytes(path: &[PathComponent], value: Option<&Value>) -> usize {
    let path_bytes = path.iter().fold(16_usize, |total, component| {
        total.saturating_add(match component {
            PathComponent::Key(key) => key.len().saturating_add(16),
            PathComponent::Index(_) => 16,
        })
    });
    path_bytes.saturating_add(value.map_or(0, estimate_value_bytes))
}

fn estimate_value_bytes(value: &Value) -> usize {
    match value {
        Value::Null => 4,
        Value::Bool(_) => 5,
        Value::Number(number) => number.to_string().len(),
        Value::String(value) => value.len().saturating_add(16),
        Value::Array(values) => values.iter().fold(24_usize, |total, value| {
            total.saturating_add(estimate_value_bytes(value))
        }),
        Value::Object(values) => values.iter().fold(32_usize, |total, (key, value)| {
            total
                .saturating_add(key.len())
                .saturating_add(estimate_value_bytes(value))
        }),
    }
}

fn invalid_automatic_event() -> RunError {
    RunError::Unsupported("automatic decoder produced an invalid path/value event".to_owned())
}

fn stream_reader<R: Read, W: Write, E: Write>(
    options: &RunOptions,
    format: InputFormat,
    reader: R,
    identity: &str,
    executor: &mut StreamExecutor<'_, W, E>,
) -> Result<(), RunError> {
    if options.proxy_on_error {
        let bytes = read_limited(reader, options.limits.input_bytes, identity)?;
        match validate_proxy_event_source(&bytes, identity, options, format) {
            Ok(()) => {
                return stream_reader_inner(options, format, bytes.as_slice(), identity, executor);
            }
            Err(error) if proxyable_format_error(&error) => {
                return executor.output.proxy(&bytes);
            }
            Err(error) => return Err(error.into()),
        }
    }
    stream_reader_inner(options, format, reader, identity, executor)
}

fn stream_reader_inner<R: Read, W: Write, E: Write>(
    options: &RunOptions,
    format: InputFormat,
    reader: R,
    identity: &str,
    executor: &mut StreamExecutor<'_, W, E>,
) -> Result<(), RunError> {
    let stream_options = StreamOptions {
        maximum_depth: options.limits.depth,
        maximum_token_bytes: options.limits.token_bytes,
        errors_as_values: options.stream_errors,
    };
    let reader = LimitedReader::new(reader, options.limits.input_bytes, identity);
    match format {
        InputFormat::Json => stream_json_into(buffered_input(reader), stream_options, executor),
        InputFormat::JsonLines => {
            stream_json_lines_into(reader, identity, options, stream_options, executor)
        }
        InputFormat::Toon => stream_toon_into(reader, options, stream_options, executor),
        InputFormat::Auto => {
            let (report, replay) = probe_reader(reader, options.limits.lookahead_bytes)?;
            match report.selected {
                InputFormat::Json => {
                    stream_json_into(buffered_input(replay), stream_options, executor)
                }
                InputFormat::Toon => stream_toon_into(replay, options, stream_options, executor),
                InputFormat::Yaml => Err(RunError::Unsupported(
                    "auto-detection selected YAML, which is document-at-a-time and cannot satisfy --stream; use --input-format json for JSON syntax".to_owned(),
                )),
                InputFormat::Auto | InputFormat::JsonLines | InputFormat::ToonSequence => {
                    unreachable!("probe candidate")
                }
            }
        }
        InputFormat::Yaml => Err(RunError::Unsupported(
            "YAML input is document-at-a-time and cannot satisfy --stream".to_owned(),
        )),
        InputFormat::ToonSequence => Err(RunError::Unsupported(
            "TOON sequence input cannot currently be nested inside --stream".to_owned(),
        )),
    }
}

fn stream_json_lines_into<R: Read, W: Write, E: Write>(
    reader: R,
    identity: &str,
    options: &RunOptions,
    stream_options: StreamOptions,
    executor: &mut StreamExecutor<'_, W, E>,
) -> Result<(), RunError> {
    let mut source = JsonLinesDocumentSource::new(
        BufReader::new(reader),
        identity,
        decode_options(options, InputFormat::JsonLines),
    );
    while let Some((record, line)) = source.next_record()? {
        stream_json_into(record.as_slice(), stream_options, executor)
            .map_err(|error| json_lines_record_error(error, identity, line))?;
    }
    Ok(())
}

fn json_lines_record_error(error: RunError, identity: &str, line: u64) -> RunError {
    match error {
        RunError::Input(FormatError::Parse { message, .. }) => {
            RunError::Input(FormatError::Parse {
                format: InputFormat::JsonLines,
                message: format!("{identity}:{line}: {message}"),
            })
        }
        error => error,
    }
}

fn proxyable_format_error(error: &FormatError) -> bool {
    matches!(
        error,
        FormatError::Diagnostic(_)
            | FormatError::Parse { .. }
            | FormatError::Probe { .. }
            | FormatError::UnsupportedYaml(_)
    )
}

fn validate_proxy_event_source(
    bytes: &[u8],
    identity: &str,
    options: &RunOptions,
    format: InputFormat,
) -> Result<(), FormatError> {
    let documents = decode_bytes(bytes, identity, decode_options(options, format))?;
    let selected = documents.first().map_or(format, |document| document.format);
    if selected == InputFormat::Json && documents.len() != 1 {
        return Err(FormatError::Parse {
            format: InputFormat::Json,
            message: format!("{identity} requires exactly one JSON value for event input"),
        });
    }
    Ok(())
}

fn stream_json_into<R: Read, W: Write, E: Write>(
    reader: R,
    options: StreamOptions,
    executor: &mut StreamExecutor<'_, W, E>,
) -> Result<(), RunError> {
    let mut execution_error = None;
    let decoded = stream_json(reader, options, |record| match executor.accept(record) {
        Ok(()) => Ok(()),
        Err(error) => {
            execution_error = Some(error);
            Err("stream consumer stopped".to_owned())
        }
    });
    if let Some(error) = execution_error {
        return Err(error);
    }
    decoded.map_err(RunError::Input)
}

fn stream_toon_into<R: Read, W: Write, E: Write>(
    reader: R,
    options: &RunOptions,
    stream_options: StreamOptions,
    executor: &mut StreamExecutor<'_, W, E>,
) -> Result<(), RunError> {
    let mut execution_error = None;
    let decoded = stream_toon(
        BufReader::new(reader),
        tq_toon::DecoderConfig {
            strict: options.strict,
            maximum_depth: options.limits.depth,
            maximum_token_bytes: options.limits.token_bytes,
            maximum_line_bytes: options.limits.line_bytes,
            maximum_lookahead_bytes: options.limits.lookahead_bytes,
            ..tq_toon::DecoderConfig::default()
        },
        stream_options,
        |record| match executor.accept(record) {
            Ok(()) => Ok(()),
            Err(error) => {
                execution_error = Some(error);
                Err("stream consumer stopped".to_owned())
            }
        },
    );
    if let Some(error) = execution_error {
        return Err(error);
    }
    decoded.map_err(RunError::Input)
}

struct LimitedReader<R> {
    reader: R,
    remaining: u64,
    exhausted: bool,
    identity: String,
}

impl<R> LimitedReader<R> {
    fn new(reader: R, limit: u64, identity: &str) -> Self {
        Self {
            reader,
            remaining: limit,
            exhausted: false,
            identity: identity.to_owned(),
        }
    }
}

impl<R: Read> Read for LimitedReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.exhausted {
            return Ok(0);
        }
        if self.remaining == 0 {
            let mut probe = [0_u8; 1];
            if self.reader.read(&mut probe)? == 0 {
                self.exhausted = true;
                return Ok(0);
            }
            return Err(io::Error::other(format!(
                "input resource limit exceeded for '{}': input-bytes",
                self.identity
            )));
        }
        let allowed = usize::try_from(self.remaining)
            .unwrap_or(usize::MAX)
            .min(buffer.len());
        let count = self.reader.read(&mut buffer[..allowed])?;
        self.remaining = self
            .remaining
            .saturating_sub(u64::try_from(count).unwrap_or(u64::MAX));
        Ok(count)
    }
}

struct StreamExecutor<'a, W, E> {
    plan: &'a Plan<Compiled, Events>,
    variables: &'a BTreeMap<Arc<str>, Value>,
    output: ResultOutput<'a, W>,
    stderr: &'a mut E,
    trace_remaining: usize,
    observations: VmObservations,
    last: Option<Value>,
    results: usize,
}

impl<W: Write, E: Write> StreamExecutor<'_, W, E> {
    fn accept(&mut self, input: Value) -> Result<(), RunError> {
        let mut vm = Vm::new_events_with_variables(
            self.plan,
            input,
            vm_limits(self.output.options),
            self.variables.clone(),
        )
        .with_trace_limit(self.trace_remaining);
        if let Some(flag) = cancellation() {
            vm = vm.with_cancellation(flag);
        }
        let mut output_error = None;
        let evaluated = vm.for_each_result(|value| {
            if let Err(error) = self.output.emit(&value) {
                output_error = Some(error);
                return false;
            }
            self.last = Some(value);
            self.results = self.results.saturating_add(1);
            true
        });
        if let Some(error) = output_error {
            return Err(error);
        }
        evaluated.map_err(RunError::Runtime)?;
        if self.trace_remaining != 0 {
            for entry in vm.trace() {
                writeln!(self.stderr, "trace: {entry}")?;
            }
            self.trace_remaining = self.trace_remaining.saturating_sub(vm.trace().len());
        }
        merge_observations(&mut self.observations, vm.observations());
        Ok(())
    }
}

fn vm_limits(options: &RunOptions) -> VmLimits {
    VmLimits {
        steps: options.limits.vm_steps,
        path_stack: options.limits.depth,
        call_stack: options.limits.depth.saturating_mul(4),
        output_bytes: usize::try_from(options.limits.output_bytes).unwrap_or(usize::MAX),
        json_depth: options.limits.depth,
        json_token_bytes: options.limits.token_bytes,
        regex_pattern_bytes: options.limits.token_bytes,
        regex_input_bytes: usize::try_from(options.limits.input_bytes).unwrap_or(usize::MAX),
        ..VmLimits::default()
    }
}

fn merge_observations(total: &mut VmObservations, item: VmObservations) {
    total.value_stack_high_water = total
        .value_stack_high_water
        .max(item.value_stack_high_water);
    total.call_stack_high_water = total.call_stack_high_water.max(item.call_stack_high_water);
    total.path_stack_high_water = total.path_stack_high_water.max(item.path_stack_high_water);
    total.fork_stack_high_water = total.fork_stack_high_water.max(item.fork_stack_high_water);
    total.steps = total.steps.saturating_add(item.steps);
    total.results = total.results.saturating_add(item.results);
}

struct ResultOutput<'a, W> {
    writer: &'a mut W,
    options: &'a RunOptions,
    unframed: Option<Value>,
    written: u64,
    emitted: u64,
    last_was_proxy: bool,
}

impl<'a, W: Write> ResultOutput<'a, W> {
    fn new(writer: &'a mut W, options: &'a RunOptions) -> Self {
        Self {
            writer,
            options,
            unframed: None,
            written: 0,
            emitted: 0,
            last_was_proxy: false,
        }
    }

    fn emit(&mut self, value: &Value) -> Result<(), RunError> {
        if self.emitted >= self.options.limits.results {
            return Err(RunError::Resource("result-count"));
        }
        if self.options.output_format == tq_formats::OutputFormat::Toon
            && self.options.framing == ToonFraming::Unframed
            && self.last_was_proxy
        {
            return Err(OutputError::Toon(tq_toon::SequenceError::Cardinality(
                tq_toon::CardinalityError::Multiple,
            ))
            .into());
        }
        self.emitted = self.emitted.saturating_add(1);
        self.last_was_proxy = false;
        let sorted;
        let value = if self.options.sort_keys {
            sorted = sort_value_keys(value);
            &sorted
        } else {
            value
        };
        if self.options.raw_output {
            let mut writer = LimitedWriter::new(
                &mut *self.writer,
                &mut self.written,
                self.options.limits.output_bytes,
            );
            write_raw(
                &mut writer,
                std::slice::from_ref(value),
                self.options.join_output,
                self.options.raw_output0,
            )?;
            if self.options.unbuffered {
                writer.flush()?;
            }
            return Ok(());
        }
        if self.options.output_format == tq_formats::OutputFormat::Toon
            && self.options.framing == ToonFraming::Unframed
        {
            if self.emitted > 1 {
                return Err(OutputError::Toon(tq_toon::SequenceError::Cardinality(
                    tq_toon::CardinalityError::Multiple,
                ))
                .into());
            }
            if self.unframed.replace(value.clone()).is_some() {
                return Err(OutputError::Toon(tq_toon::SequenceError::Cardinality(
                    tq_toon::CardinalityError::Multiple,
                ))
                .into());
            }
            return Ok(());
        }
        let mut writer = LimitedWriter::new(
            &mut *self.writer,
            &mut self.written,
            self.options.limits.output_bytes,
        );
        if matches!(
            self.options.output_format,
            tq_formats::OutputFormat::Json | tq_formats::OutputFormat::JsonLines
        ) && !self.options.pretty_json
            && !self.options.ascii_output
            && self.options.color != ColorMode::Always
        {
            serde_json::to_writer(&mut writer, value)?;
            writer.write_all(b"\n")?;
            if self.options.unbuffered {
                writer.flush()?;
            }
            return Ok(());
        }
        write_results(
            &mut writer,
            [value],
            OutputOptions {
                format: self.options.output_format,
                pretty_json: self.options.pretty_json,
                json_indent: self.options.json_indent,
                ascii_json: self.options.ascii_output,
                color_json: self.options.color == ColorMode::Always,
                yaml_document_start: self.emitted > 1,
                toon_framing: self.options.framing,
                toon: self.options.toon_writer,
            },
        )?;
        if self.options.unbuffered {
            self.writer.flush()?;
        }
        Ok(())
    }

    fn proxy(&mut self, bytes: &[u8]) -> Result<(), RunError> {
        if self.options.output_format == tq_formats::OutputFormat::Toon
            && self.options.framing == ToonFraming::Unframed
            && (self.emitted != 0 || self.last_was_proxy)
        {
            return Err(OutputError::Toon(tq_toon::SequenceError::Cardinality(
                tq_toon::CardinalityError::Multiple,
            ))
            .into());
        }
        let mut writer = LimitedWriter::new(
            &mut *self.writer,
            &mut self.written,
            self.options.limits.output_bytes,
        );
        writer.write_all(bytes).map_err(OutputError::Io)?;
        if self.options.unbuffered {
            writer.flush().map_err(OutputError::Io)?;
        }
        self.last_was_proxy = true;
        Ok(())
    }

    fn finish(&mut self) -> Result<(), RunError> {
        if self.options.output_format == tq_formats::OutputFormat::Toon
            && self.options.framing == ToonFraming::Unframed
        {
            if self.unframed.is_some() {
                self.flush_unframed()?;
            } else if !self.last_was_proxy {
                return Err(OutputError::Toon(tq_toon::SequenceError::Cardinality(
                    tq_toon::CardinalityError::Zero,
                ))
                .into());
            }
        }
        if self.options.unbuffered {
            self.writer.flush()?;
        }
        Ok(())
    }

    const fn written(&self) -> u64 {
        self.written
    }

    fn exit_status(&self, requested: bool, last: Option<&Value>) -> ExitStatus {
        if self.last_was_proxy {
            ExitStatus::Success
        } else {
            exit_status(requested, last)
        }
    }

    fn flush_unframed(&mut self) -> Result<(), RunError> {
        let value = self.unframed.take().expect("checked unframed output");
        let mut writer = LimitedWriter::new(
            &mut *self.writer,
            &mut self.written,
            self.options.limits.output_bytes,
        );
        write_results(
            &mut writer,
            [&value],
            OutputOptions {
                format: self.options.output_format,
                pretty_json: self.options.pretty_json,
                json_indent: self.options.json_indent,
                ascii_json: self.options.ascii_output,
                color_json: self.options.color == ColorMode::Always,
                yaml_document_start: false,
                toon_framing: self.options.framing,
                toon: self.options.toon_writer,
            },
        )?;
        Ok(())
    }
}

struct LimitedWriter<'a, W> {
    writer: &'a mut W,
    written: &'a mut u64,
    limit: u64,
}

impl<'a, W> LimitedWriter<'a, W> {
    fn new(writer: &'a mut W, written: &'a mut u64, limit: u64) -> Self {
        Self {
            writer,
            written,
            limit,
        }
    }
}

impl<W: Write> Write for LimitedWriter<'_, W> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let length = u64::try_from(buffer.len()).unwrap_or(u64::MAX);
        if (*self.written).saturating_add(length) > self.limit {
            return Err(io::Error::other(
                "output resource limit exceeded: output-bytes",
            ));
        }
        let count = self.writer.write(buffer)?;
        *self.written = (*self.written).saturating_add(u64::try_from(count).unwrap_or(u64::MAX));
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

fn load_filter(options: &RunOptions) -> Result<(String, Vec<u8>), RunError> {
    match &options.filter {
        FilterSource::Inline(query) => Ok(("<command-line>".to_owned(), query.as_bytes().to_vec())),
        FilterSource::File(path) => {
            let identity = path.display().to_string();
            Ok((
                identity.clone(),
                read_limited(open_path(path)?, options.limits.input_bytes, &identity)?,
            ))
        }
    }
}

fn parse_external_arguments(options: &RunOptions) -> Result<BTreeMap<Arc<str>, Value>, RunError> {
    let mut values = BTreeMap::new();
    let mut named = tq_core::Object::new();
    for argument in &options.arguments {
        let value = match argument.kind {
            ExternalArgumentKind::String => Value::string(argument.value.as_str()),
            ExternalArgumentKind::Json => {
                decode_single_json(argument.value.as_bytes(), "--argjson")?
            }
            ExternalArgumentKind::Toon => {
                let config = tq_toon::DecoderConfig {
                    strict: options.strict,
                    ..tq_toon::DecoderConfig::default()
                };
                decode_toon(argument.value.as_bytes(), "--argtoon", config)?
                    .pop()
                    .ok_or_else(|| RunError::Unsupported("--argtoon produced no value".to_owned()))?
                    .value
            }
            ExternalArgumentKind::RawFile => {
                let bytes = read_limited(
                    open_path(Path::new(&argument.value))?,
                    options.limits.input_bytes,
                    &argument.value,
                )?;
                let text = String::from_utf8(bytes).map_err(|error| {
                    RunError::Input(FormatError::Parse {
                        format: InputFormat::Auto,
                        message: format!(
                            "--rawfile '{}' input is not UTF-8: {error}",
                            argument.value
                        ),
                    })
                })?;
                Value::string(text)
            }
            ExternalArgumentKind::SlurpFile => {
                let bytes = read_limited(
                    open_path(Path::new(&argument.value))?,
                    options.limits.input_bytes,
                    &argument.value,
                )?;
                let documents = decode_json(&bytes, &argument.value)?;
                Value::array(
                    documents
                        .into_iter()
                        .map(|document| document.value)
                        .collect::<Vec<_>>(),
                )
            }
        };
        named.insert(Arc::from(argument.name.as_str()), value.clone());
        values.insert(Arc::from(argument.name.as_str()), value);
    }
    let positional = match options.positional_argument_kind {
        None | Some(PositionalArgumentKind::String) => options
            .positional_arguments
            .iter()
            .map(|value| Ok(Value::string(value.as_str())))
            .collect::<Result<Vec<_>, RunError>>()?,
        Some(PositionalArgumentKind::Json) => options
            .positional_arguments
            .iter()
            .map(|value| decode_single_json(value.as_bytes(), "--jsonargs"))
            .collect::<Result<Vec<_>, RunError>>()?,
    };
    let arguments = tq_core::Object::from_iter([
        (Arc::from("named"), Value::object(named)),
        (Arc::from("positional"), Value::array(positional)),
    ]);
    values.insert(Arc::from("ARGS"), Value::object(arguments));
    if options.allow_environment && options.capability_policy.environment {
        let environment = std::env::vars_os()
            .filter_map(|(name, value)| Some((name.into_string().ok()?, value.into_string().ok()?)))
            .collect::<BTreeMap<_, _>>()
            .into_iter()
            .map(|(name, value)| (Arc::from(name), Value::string(value)))
            .collect::<tq_core::Object>();
        values.insert(Arc::from(AMBIENT_ENVIRONMENT), Value::object(environment));
    }
    values.insert(
        Arc::from(AMBIENT_PLATFORM),
        Value::Bool(options.allow_platform && options.capability_policy.platform),
    );
    let input_filename = if options.null_input {
        "<null-input>".to_owned()
    } else {
        options
            .files
            .first()
            .filter(|path| *path != Path::new("-"))
            .map_or_else(|| "<stdin>".to_owned(), |path| path.display().to_string())
    };
    values.insert(Arc::from(INPUT_FILENAME), Value::string(input_filename));
    values.insert(
        Arc::from(INPUT_LINE_NUMBER),
        Value::Number(Number::parse("1").expect("one is an admitted number")),
    );
    Ok(values)
}

fn decode_single_json(bytes: &[u8], identity: &str) -> Result<Value, RunError> {
    let mut documents = decode_json(bytes, identity)?;
    if documents.len() != 1 {
        return Err(RunError::Input(FormatError::Parse {
            format: InputFormat::Json,
            message: format!("{identity} requires exactly one JSON value"),
        }));
    }
    Ok(documents.pop().expect("one document").value)
}

enum RemainingInputMessage {
    Value(InputValue),
    Error(VmError),
    Done,
}

fn produce_remaining_inputs<R: Read>(
    options: &RunOptions,
    stdin: &mut R,
    sender: &SyncSender<RemainingInputMessage>,
) {
    let result = produce_remaining_inputs_inner(options, stdin, sender);
    let message = result.map_or_else(
        |error| RemainingInputMessage::Error(deferred_run_error(error)),
        |()| RemainingInputMessage::Done,
    );
    let _ = sender.send(message);
}

fn produce_remaining_inputs_inner<R: Read>(
    options: &RunOptions,
    stdin: &mut R,
    sender: &SyncSender<RemainingInputMessage>,
) -> Result<(), RunError> {
    if options.null_input {
        send_remaining(
            sender,
            tq_formats::Document {
                value: Value::Null,
                identity: "<null-input>".to_owned(),
                format: options.input_format,
                index: 0,
            },
        );
        return Ok(());
    }
    let files = if options.files.is_empty() {
        vec![Path::new("-").to_owned()]
    } else {
        options.files.clone()
    };
    for path in files {
        if path == Path::new("-") {
            produce_remaining_reader(
                options,
                options.input_format,
                &mut *stdin,
                "<stdin>",
                sender,
            )?;
        } else {
            let identity = path.display().to_string();
            produce_remaining_reader(
                options,
                selected_input_format(options, &path),
                open_path(&path)?,
                &identity,
                sender,
            )?;
        }
    }
    Ok(())
}

fn produce_remaining_reader<R: Read>(
    options: &RunOptions,
    requested: InputFormat,
    reader: R,
    identity: &str,
    sender: &SyncSender<RemainingInputMessage>,
) -> Result<(), RunError> {
    let reader = LimitedReader::new(reader, options.limits.input_bytes, identity);
    if requested == InputFormat::Auto {
        let (report, replay) = probe_reader(reader, options.limits.lookahead_bytes)?;
        return produce_committed_remaining_reader(
            options,
            report.selected,
            replay,
            identity,
            sender,
        );
    }
    produce_committed_remaining_reader(options, requested, reader, identity, sender)
}

fn produce_committed_remaining_reader<R: Read>(
    options: &RunOptions,
    format: InputFormat,
    reader: R,
    identity: &str,
    sender: &SyncSender<RemainingInputMessage>,
) -> Result<(), RunError> {
    let mut source: Box<dyn DocumentSource> = match format {
        InputFormat::Json => Box::new(JsonDocumentSource::new(
            BufReader::with_capacity(INPUT_BUFFER_BYTES, reader),
            identity,
        )),
        InputFormat::JsonLines => Box::new(JsonLinesDocumentSource::new(
            BufReader::new(reader),
            identity,
            decode_options(options, format),
        )),
        InputFormat::Yaml | InputFormat::Toon | InputFormat::ToonSequence => {
            let bytes = read_limited(reader, options.limits.input_bytes, identity)?;
            Box::new(VecDocumentSource::new(decode_bytes(
                &bytes,
                identity,
                decode_options(options, format),
            )?))
        }
        InputFormat::Auto => unreachable!("auto input is committed before decoding"),
    };
    while let Some(document) = source.next_document()? {
        if !send_remaining(sender, document) {
            break;
        }
    }
    Ok(())
}

fn send_remaining(
    sender: &SyncSender<RemainingInputMessage>,
    document: tq_formats::Document,
) -> bool {
    sender
        .send(RemainingInputMessage::Value(InputValue {
            value: document.value,
            identity: Arc::from(document.identity),
            line_number: document.index.saturating_add(1),
        }))
        .is_ok()
}

fn pull_remaining_input(cursor: &InputCursor) -> Result<Option<Value>, RunError> {
    cursor.next_value().map_err(|error| match error {
        VmError::Input { message } => RunError::Input(FormatError::Parse {
            format: InputFormat::Auto,
            message: message.to_string(),
        }),
        error => RunError::Runtime(error),
    })
}

fn deferred_run_error(error: RunError) -> VmError {
    match error {
        RunError::Runtime(error) => error,
        RunError::Resource(resource) => VmError::Resource { resource },
        RunError::Interrupted => VmError::Interrupted,
        error => VmError::Input {
            message: error.to_string().into(),
        },
    }
}

enum LoadedInputs {
    Documents(Vec<tq_formats::Document>),
    Proxy(Vec<u8>),
}

fn load_inputs<R: Read>(options: &RunOptions, stdin: &mut R) -> Result<LoadedInputs, RunError> {
    if options.null_input {
        return Ok(LoadedInputs::Documents(vec![tq_formats::Document {
            value: Value::Null,
            identity: "<null-input>".to_owned(),
            format: InputFormat::Auto,
            index: 0,
        }]));
    }
    let files = if options.files.is_empty() {
        vec![Path::new("-").to_owned()]
    } else {
        options.files.clone()
    };
    let mut documents = Vec::new();
    let mut raw_sources = Vec::new();
    let mut proxy = false;
    for path in files {
        let (identity, bytes) = if path == Path::new("-") {
            (
                "<stdin>".to_owned(),
                read_limited(&mut *stdin, options.limits.input_bytes, "<stdin>")?,
            )
        } else {
            let identity = path.display().to_string();
            let bytes = read_limited(open_path(&path)?, options.limits.input_bytes, &identity)?;
            (identity, bytes)
        };
        if options.raw_input {
            raw_documents(&mut documents, identity, bytes, options.slurp)?;
        } else {
            let decode = decode_options(options, selected_input_format(options, &path));
            match decode_bytes(&bytes, identity, decode) {
                Ok(decoded) => documents.extend(decoded),
                Err(error) if options.proxy_on_error && proxyable_format_error(&error) => {
                    proxy = true;
                }
                Err(error) => return Err(error.into()),
            }
            if options.proxy_on_error {
                raw_sources.push(bytes);
            }
        }
    }
    if proxy {
        Ok(LoadedInputs::Proxy(
            raw_sources.into_iter().flatten().collect(),
        ))
    } else {
        Ok(LoadedInputs::Documents(documents))
    }
}

fn read_limited(mut reader: impl Read, limit: u64, identity: &str) -> Result<Vec<u8>, RunError> {
    let mut bytes = Vec::new();
    reader
        .by_ref()
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limit {
        return Err(RunError::ResourceSource {
            identity: identity.to_owned(),
            resource: "input-bytes",
        });
    }
    Ok(bytes)
}

fn open_path(path: &Path) -> Result<File, RunError> {
    File::open(path).map_err(|source| RunError::IoPath {
        path: path.display().to_string(),
        source,
    })
}

fn buffered_input<R: Read>(reader: R) -> BufReader<R> {
    BufReader::with_capacity(INPUT_BUFFER_BYTES, reader)
}

fn format_from_path(path: &Path) -> Option<InputFormat> {
    let extension = path.extension()?.to_str()?;
    if extension.eq_ignore_ascii_case("yaml") || extension.eq_ignore_ascii_case("yml") {
        Some(InputFormat::Yaml)
    } else if extension.eq_ignore_ascii_case("jsonl") || extension.eq_ignore_ascii_case("ndjson") {
        Some(InputFormat::JsonLines)
    } else if extension.eq_ignore_ascii_case("json") {
        Some(InputFormat::Json)
    } else if extension.eq_ignore_ascii_case("toon") {
        Some(InputFormat::Toon)
    } else {
        None
    }
}

fn selected_input_format(options: &RunOptions, path: &Path) -> InputFormat {
    if options.input_format == InputFormat::Auto && path != Path::new("-") {
        format_from_path(path).unwrap_or(InputFormat::Auto)
    } else {
        options.input_format
    }
}

fn decode_options(options: &RunOptions, format: InputFormat) -> DecodeOptions {
    DecodeOptions {
        format,
        maximum_source_bytes: usize::try_from(options.limits.input_bytes).unwrap_or(usize::MAX),
        maximum_depth: options.limits.depth,
        maximum_token_bytes: options.limits.token_bytes,
        maximum_line_bytes: options.limits.line_bytes,
        toon: tq_toon::DecoderConfig {
            strict: options.strict,
            maximum_depth: options.limits.depth,
            maximum_token_bytes: options.limits.token_bytes,
            maximum_line_bytes: options.limits.line_bytes,
            maximum_lookahead_bytes: options.limits.lookahead_bytes,
            ..tq_toon::DecoderConfig::default()
        },
    }
}

fn raw_documents(
    documents: &mut Vec<tq_formats::Document>,
    identity: String,
    bytes: Vec<u8>,
    slurp: bool,
) -> Result<(), RunError> {
    let text = String::from_utf8(bytes).map_err(|error| {
        RunError::Input(FormatError::Parse {
            format: InputFormat::Auto,
            message: format!("raw input is not UTF-8: {error}"),
        })
    })?;
    if slurp {
        documents.push(tq_formats::Document {
            value: Value::string(text),
            identity,
            format: InputFormat::Auto,
            index: 0,
        });
        return Ok(());
    }
    for (index, line) in text.lines().enumerate() {
        documents.push(tq_formats::Document {
            value: Value::string(line.trim_end_matches('\r')),
            identity: identity.clone(),
            format: InputFormat::Auto,
            index: index as u64,
        });
    }
    Ok(())
}

fn write_raw(
    mut output: impl Write,
    values: &[Value],
    join: bool,
    nul_separator: bool,
) -> Result<(), RunError> {
    for value in values {
        match value {
            Value::String(value) => {
                if nul_separator && value.contains('\0') {
                    return Err(RunError::RawOutput(
                        "cannot emit a string containing NUL with --raw-output0",
                    ));
                }
                output.write_all(value.as_bytes())?;
            }
            _ => serde_json::to_writer(&mut output, value)?,
        }
        if nul_separator {
            output.write_all(b"\0")?;
        } else if !join {
            output.write_all(b"\n")?;
        }
    }
    Ok(())
}

fn sort_value_keys(value: &Value) -> Value {
    match value {
        Value::Array(values) => {
            Value::array(values.iter().map(sort_value_keys).collect::<Vec<_>>())
        }
        Value::Object(values) => {
            let mut entries = values.iter().collect::<Vec<_>>();
            entries.sort_unstable_by_key(|(key, _)| *key);
            Value::object(
                entries
                    .into_iter()
                    .map(|(key, value)| (Arc::clone(key), sort_value_keys(value)))
                    .collect::<tq_core::Object>(),
            )
        }
        _ => value.clone(),
    }
}

fn write_report(
    path: &Path,
    observations: &[tq_core::VmObservations],
    results: usize,
    output_bytes: u64,
    options: &RunOptions,
    plan: PlanKind,
    execution: ReportExecution<'_>,
) -> Result<(), RunError> {
    let ReportExecution {
        analysis,
        retention,
        resource_outcome,
    } = execution;
    let report = serde_json::json!({
        "schema_version": 1,
        "documents": observations.len(),
        "results": results,
        "output_bytes": output_bytes,
        "execution": {
            "plan": plan.to_string(),
            "proof": analysis.stream_proof,
            "hybrid_proof": analysis.hybrid_proof,
            "optimizer_rewrites": analysis.optimizer_rewrites,
            "stream_rejection": analysis.stream_rejection,
            "retained_working_set": match plan {
                PlanKind::Transcode => "bounded-structural-preparation",
                PlanKind::Events => "decoder-events",
                PlanKind::Subtree => "selected-subtree",
                PlanKind::HybridBlocking => "projected-collection-and-blocking-state",
                PlanKind::Document => "document",
                PlanKind::WholeInput => "whole-input",
                PlanKind::Blocking => "document-and-blocking-state",
            },
            "retention_high_water": {
                "bytes": retention.bytes_high_water,
                "depth": retention.depth_high_water,
                "decoder_depth": retention.decoder_depth_high_water,
                "completed_subtrees": retention.completed_subtrees,
                "retained_result_count": retention.retained_results_high_water,
                "retained_estimated_bytes": retention.retained_bytes_high_water,
                "blocking_state": if plan == PlanKind::HybridBlocking { "projected-collection-and-blocking-suffix" } else { "none" },
                "sort_runs": retention.sort_runs,
                "in_flight_batches": retention.in_flight_batches_high_water,
                "in_flight_bytes": retention.in_flight_bytes_high_water,
                "worker_count": parallel_worker_count(),
                "parallel_decode_batches": retention.decode_batches,
                "parallel_decode_in_flight_batches": retention.decode_in_flight_batches_high_water,
                "parallel_decode_in_flight_bytes": retention.decode_in_flight_bytes_high_water,
                "parallel_decode_reordered_batches": retention.decode_reordered_batches_high_water,
                "parallel_decode_active": retention.decode_batches > 0,
                "root_materialized": !matches!(plan, PlanKind::Events | PlanKind::Subtree | PlanKind::HybridBlocking | PlanKind::Transcode),
            },
            "resource_outcome": resource_outcome,
        },
        "limits": {
            "input_bytes": options.limits.input_bytes,
            "depth": options.limits.depth,
            "token_bytes": options.limits.token_bytes,
            "line_bytes": options.limits.line_bytes,
            "lookahead_bytes": options.limits.lookahead_bytes,
            "vm_steps": options.limits.vm_steps,
            "results": options.limits.results,
            "output_bytes": options.limits.output_bytes,
            "preparation_memory_bytes": options.limits.preparation_memory_bytes,
            "hybrid_batch_values": options.limits.hybrid_batch_values,
            "hybrid_in_flight_batches": options.limits.hybrid_in_flight_batches,
            "hybrid_in_flight_bytes": options.limits.hybrid_in_flight_bytes,
            "decode_batch_values": options.limits.decode_batch_values,
            "decode_batch_bytes": options.limits.decode_batch_bytes,
            "decode_in_flight_batches": options.limits.decode_in_flight_batches,
            "decode_in_flight_bytes": options.limits.decode_in_flight_bytes,
            "spool_bytes": options.limits.spool_bytes,
        },
        "observations": observations.iter().map(|item| serde_json::json!({
            "value_stack_high_water": item.value_stack_high_water,
            "call_stack_high_water": item.call_stack_high_water,
            "path_stack_high_water": item.path_stack_high_water,
            "fork_stack_high_water": item.fork_stack_high_water,
            "steps": item.steps,
            "results": item.results,
        })).collect::<Vec<_>>()
    });
    fs::write(path, serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}

#[derive(Clone, Copy)]
struct TranscodeReportExecution<'a> {
    documents: u64,
    output_bytes: u64,
    observations: PreparationObservations,
    resource_outcome: &'a str,
}

fn write_transcode_report(
    path: &Path,
    options: &RunOptions,
    analysis: &Analysis,
    detections: &[DetectionObservation],
    execution: TranscodeReportExecution<'_>,
) -> Result<(), RunError> {
    let TranscodeReportExecution {
        documents,
        output_bytes,
        observations,
        resource_outcome,
    } = execution;
    let proof = analysis
        .transcode_proof
        .expect("transcode report carries a selected proof");
    let detection = detections
        .iter()
        .map(|item| {
            serde_json::json!({
                "identity": item.identity,
                "selected_input_format": concrete_input_format_name(item.selected),
                "lookahead_bytes": item.lookahead_bytes,
                "commitment_bytes": item.commitment_bytes,
                "rejections": item.rejections.iter().map(|(format, reason)| serde_json::json!({
                    "format": concrete_input_format_name(*format),
                    "reason": reason,
                })).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    let report = serde_json::json!({
        "schema_version": 1,
        "documents": documents,
        "results": documents,
        "output_bytes": output_bytes,
        "execution": {
            "plan": "transcode",
            "selected_input_format": concrete_input_format_name(match proof.input {
                TranscodeInput::Json => InputFormat::Json,
                TranscodeInput::Toon => InputFormat::Toon,
            }),
            "input_format_source": if options.input_format == InputFormat::Auto { "detected" } else { "override" },
            "detection": detection,
            "proof": analysis.transcode_proof,
            "transcode_rejection": analysis.transcode_rejection,
            "duplicate_key_limitation": duplicate_key_limitation(analysis.transcode_proof),
            "retained_working_set": "bounded-structural-preparation",
            "materialized_root": false,
            "spooled": observations.spool_bytes_written > 0,
            "commitment_mode": proof.commitment,
            "input_stage_bytes_written": 0,
            "input_stage_bytes_replayed": 0,
            "preparation_high_water_bytes": observations.memory_high_water_bytes,
            "preparation_nesting_high_water": observations.nesting_high_water,
            "object_index_spills": observations.object_index_spills,
            "array_preparations": observations.array_preparations,
            "spool_bytes_written": observations.spool_bytes_written,
            "spool_bytes_replayed": observations.spool_bytes_replayed,
            "prepared_output_bytes": observations.output_bytes,
            "resource_outcome": resource_outcome,
        },
        "limits": {
            "input_bytes": options.limits.input_bytes,
            "depth": options.limits.depth,
            "token_bytes": options.limits.token_bytes,
            "line_bytes": options.limits.line_bytes,
            "lookahead_bytes": options.limits.lookahead_bytes,
            "results": options.limits.results,
            "output_bytes": options.limits.output_bytes,
            "preparation_memory_bytes": options.limits.preparation_memory_bytes,
            "hybrid_batch_values": options.limits.hybrid_batch_values,
            "hybrid_in_flight_batches": options.limits.hybrid_in_flight_batches,
            "hybrid_in_flight_bytes": options.limits.hybrid_in_flight_bytes,
            "decode_batch_values": options.limits.decode_batch_values,
            "decode_batch_bytes": options.limits.decode_batch_bytes,
            "decode_in_flight_batches": options.limits.decode_in_flight_batches,
            "decode_in_flight_bytes": options.limits.decode_in_flight_bytes,
            "spool_bytes": options.limits.spool_bytes,
        },
        "observations": [],
    });
    fs::write(path, serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}

#[derive(Clone, Copy)]
struct ReportExecution<'a> {
    analysis: &'a Analysis,
    retention: RetentionObservations,
    resource_outcome: &'a str,
}

fn exit_status(enabled: bool, last: Option<&Value>) -> ExitStatus {
    if enabled {
        match last {
            None => ExitStatus::NoResult,
            Some(Value::Null | Value::Bool(false)) => ExitStatus::FalseOrNull,
            Some(_) => ExitStatus::Success,
        }
    } else {
        ExitStatus::Success
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::{self, Cursor, Read, Write},
    };

    use super::run_with_io;
    use crate::{Command, ExecutionOverride, ExitStatus, parse_args};
    use tq_core::parallel_worker_count;

    fn execute(
        arguments: &[&str],
        input: &[u8],
    ) -> (Result<ExitStatus, super::RunError>, Vec<u8>, Vec<u8>) {
        let command = parse_args(arguments.iter().copied()).unwrap();
        let mut stdin = input;
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run_with_io(command, &mut stdin, &mut stdout, &mut stderr);
        (status, stdout, stderr)
    }

    fn execute_with_override(
        arguments: &[&str],
        input: &[u8],
        execution_override: ExecutionOverride,
    ) -> (Result<ExitStatus, super::RunError>, Vec<u8>, Vec<u8>) {
        let mut command = parse_args(arguments.iter().copied()).unwrap();
        let Command::Run(options) = &mut command else {
            panic!("expected run command")
        };
        options.execution_override = execution_override;
        let mut stdin = input;
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run_with_io(command, &mut stdin, &mut stdout, &mut stderr);
        (status, stdout, stderr)
    }

    #[test]
    fn internal_override_forces_document_plan_without_changing_output() {
        let arguments = [
            "--input-format",
            "json",
            "--output-format",
            "toon",
            "--explain-json",
            ".[]",
        ];
        let (_, automatic_output, automatic_explain) = execute(&arguments, b"[1,2]");

        let mut command = parse_args(arguments).unwrap();
        let Command::Run(options) = &mut command else {
            panic!("expected run command")
        };
        options.execution_override = ExecutionOverride::Document;
        let mut input = &b"[1,2]"[..];
        let mut forced_output = Vec::new();
        let mut forced_explain = Vec::new();
        assert_eq!(
            run_with_io(command, &mut input, &mut forced_output, &mut forced_explain).unwrap(),
            ExitStatus::Success
        );

        assert_eq!(forced_output, automatic_output);
        let automatic: serde_json::Value = serde_json::from_slice(&automatic_explain).unwrap();
        let forced: serde_json::Value = serde_json::from_slice(&forced_explain).unwrap();
        assert_eq!(automatic["execution"]["plan"], "subtree");
        assert_eq!(forced["execution"]["plan"], "document");
        assert_eq!(
            forced["execution"]["stream_rejection"],
            "forced document override for differential run"
        );
    }

    #[test]
    fn identity_json_transcode_rejects_a_late_duplicate_after_partial_output() {
        let (status, output, explain) = execute(
            &["--input-format", "json", "--explain-json", "."],
            br#"{"b":1,"a":2,"b":3}"#,
        );
        assert_eq!(status.unwrap_err().status(), ExitStatus::Input);
        assert_eq!(output, b"\x1eb: 1\na: 2");
        let explain: serde_json::Value = serde_json::from_slice(&explain).unwrap();
        assert_eq!(explain["execution"]["plan"], "transcode");
        assert_eq!(explain["execution"]["duplicate_policy"], "reject");
        assert_eq!(explain["execution"]["commitment_mode"], "direct-sequence");
    }

    #[test]
    fn transcode_streams_ordered_files_and_keeps_unframed_output_atomic() {
        let directory = tempfile::tempdir().unwrap();
        let first = directory.path().join("first.json");
        let second = directory.path().join("second.json");
        fs::write(&first, b"1").unwrap();
        fs::write(&second, b"2").unwrap();
        let command = parse_args([
            "--input-format",
            "json",
            ".",
            first.to_str().unwrap(),
            second.to_str().unwrap(),
        ])
        .unwrap();
        let mut input = &[][..];
        let mut output = Vec::new();
        let mut error = Vec::new();
        assert_eq!(
            run_with_io(command, &mut input, &mut output, &mut error).unwrap(),
            ExitStatus::Success
        );
        assert_eq!(output, b"\x1e1\n\x1e2\n");
        assert_eq!(error, [] as [u8; 0]);

        let (status, output, error) =
            execute(&["--input-format", "json", "--unframed", "."], b"1 2");
        assert!(matches!(status, Err(super::RunError::Cardinality(_))));
        assert_eq!(output, [] as [u8; 0]);
        assert_eq!(error, [] as [u8; 0]);
    }

    #[test]
    fn auto_detected_identity_json_uses_transcode_and_reports_observations() {
        let directory = tempfile::tempdir().unwrap();
        let report = directory.path().join("transcode.json");
        let command = parse_args([
            "--explain-json",
            "--report-file",
            report.to_str().unwrap(),
            ".",
        ])
        .unwrap();
        let mut input = br"[1,2,3]".as_slice();
        let mut output = Vec::new();
        let mut error = Vec::new();
        assert_eq!(
            run_with_io(command, &mut input, &mut output, &mut error).unwrap(),
            ExitStatus::Success
        );
        let explain: serde_json::Value = serde_json::from_slice(&error).unwrap();
        assert_eq!(explain["execution"]["plan"], "transcode");
        let report: serde_json::Value = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
        assert_eq!(report["execution"]["plan"], "transcode");
        assert_eq!(report["execution"]["array_preparations"], 1);
        assert_eq!(report["execution"]["resource_outcome"], "success");
        assert!(
            report["execution"]["preparation_high_water_bytes"]
                .as_u64()
                .unwrap()
                > 0
        );
        assert_eq!(report["execution"]["spool_bytes_written"], 0);
        assert_eq!(report["execution"]["input_stage_bytes_written"], 0);
        assert_eq!(report["execution"]["input_stage_bytes_replayed"], 0);
        assert_eq!(report["execution"]["selected_input_format"], "json");
        assert_eq!(report["execution"]["input_format_source"], "detected");
        assert_eq!(report["execution"]["detection"][0]["identity"], "<stdin>");
        assert!(report["execution"]["detection"][0]["lookahead_bytes"].is_u64());
        assert!(report["execution"]["detection"][0]["commitment_bytes"].is_u64());
        assert_eq!(report["execution"]["materialized_root"], false);
    }

    #[test]
    fn transcode_reports_resource_failure_and_output_aware_fallbacks() {
        let directory = tempfile::tempdir().unwrap();
        let report = directory.path().join("limited.json");
        let command = parse_args([
            "--input-format",
            "json",
            "--prepare-memory-bytes",
            "0",
            "--max-spool-bytes",
            "1",
            "--report-file",
            report.to_str().unwrap(),
            ".",
        ])
        .unwrap();
        let mut input = br#"["too large"]"#.as_slice();
        let mut output = Vec::new();
        let mut error = Vec::new();
        assert_eq!(
            run_with_io(command, &mut input, &mut output, &mut error)
                .unwrap_err()
                .status(),
            ExitStatus::Resource
        );
        let report: serde_json::Value = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
        assert_eq!(report["execution"]["resource_outcome"], "resource-limit");

        for arguments in [
            ["--input-format", "json", "--output-format", "json", "."],
            [
                "--input-format",
                "json",
                "--sort-keys",
                "--explain-json",
                ".",
            ],
            [
                "--input-format",
                "json",
                "--fold-keys",
                "--explain-json",
                ".",
            ],
        ] {
            let (status, _, explain) = execute(&arguments, br#"{"x":1}"#);
            assert_eq!(status.unwrap(), ExitStatus::Success);
            if arguments.contains(&"--explain-json") {
                let explain: serde_json::Value = serde_json::from_slice(&explain).unwrap();
                assert_eq!(explain["execution"]["plan"], "document");
                assert!(explain["execution"]["transcode_rejection"].is_string());
            }
        }
    }

    #[test]
    fn ambient_builtins_are_denied_by_default_and_admitted_explicitly() {
        let (denied, output, error) = execute(&["--output-format", "json", "-c", "env"], b"null\n");
        assert!(denied.is_err());
        assert_eq!(output, [] as [u8; 0]);
        let error = String::from_utf8(error).expect("UTF-8 stderr");
        assert_eq!(error, "", "run_with_io returns errors to its caller");

        for query in ["now", "input_filename"] {
            let (denied, output, error) =
                execute(&["--output-format", "json", "-c", query], b"null\n");
            let denied = denied.expect_err("platform access should be denied by default");
            assert_eq!(denied.status(), ExitStatus::Runtime);
            assert!(denied.to_string().contains("capability policy"));
            assert_eq!(output, [] as [u8; 0]);
            assert_eq!(error, [] as [u8; 0]);
        }

        let (allowed, output, error) = execute(
            &[
                "--allow-environment",
                "--output-format",
                "json",
                "-c",
                "env | type",
            ],
            b"null\n",
        );
        assert_eq!(allowed.unwrap(), ExitStatus::Success);
        assert_eq!(output, b"\"object\"\n");
        assert_eq!(error, [] as [u8; 0]);

        let (platform, output, error) = execute(
            &[
                "--allow-platform",
                "--output-format",
                "json",
                "-c",
                "[input_filename, input_line_number, (now | type)]",
            ],
            b"null\n",
        );
        assert_eq!(platform.unwrap(), ExitStatus::Success);
        assert_eq!(output, b"[\"<stdin>\",1,\"number\"]\n");
        assert_eq!(error, [] as [u8; 0]);
    }

    #[derive(Default)]
    struct FlushWriter {
        bytes: Vec<u8>,
        flush_points: Vec<usize>,
    }

    impl Write for FlushWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            self.bytes.extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            self.flush_points.push(self.bytes.len());
            Ok(())
        }
    }

    struct CountingReader {
        input: Cursor<Vec<u8>>,
        reads: usize,
    }

    impl Read for CountingReader {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            self.reads = self.reads.saturating_add(1);
            self.input.read(buffer)
        }
    }

    #[test]
    fn identity_uses_toon_sequence_and_keeps_stderr_clean() {
        let (status, stdout, stderr) = execute(&["."], b"name: Ada");
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(stdout, b"\x1ename: Ada\n");
        assert_eq!(stderr, [] as [u8; 0]);
    }

    #[test]
    fn transcode_flushes_the_sequence_prefix_before_root_payload() {
        let command = parse_args(["--input-format", "json", "."]).unwrap();
        let mut input = br#"{"name":"Ada"}"#.as_slice();
        let mut output = FlushWriter::default();
        let mut error = Vec::new();
        assert_eq!(
            run_with_io(command, &mut input, &mut output, &mut error).unwrap(),
            ExitStatus::Success
        );
        assert_eq!(output.bytes, b"\x1ename: Ada\n");
        assert_eq!(output.flush_points.first(), Some(&1));
        assert_eq!(error, [] as [u8; 0]);
    }

    #[test]
    fn lightweight_json_transcode_matches_document_scalar_rendering() {
        for input in [
            br"1e2".as_slice(),
            br#"[1e2,"true",false,null,9007199254740993]"#.as_slice(),
            br#"{"plain":"null","number":1e2,"bool":true,"nil":null,"array":[1e2,"true",false,null]}"#.as_slice(),
        ] {
            let arguments = ["-ijson", "-otoon", "."];
            let automatic =
                execute_with_override(&arguments, input, ExecutionOverride::Automatic);
            let document = execute_with_override(&arguments, input, ExecutionOverride::Document);
            assert_eq!(automatic.0.unwrap(), ExitStatus::Success);
            assert_eq!(document.0.unwrap(), ExitStatus::Success);
            assert_eq!(automatic.1, document.1);
            assert_eq!(automatic.2, [] as [u8; 0]);
            assert_eq!(document.2, [] as [u8; 0]);
        }
    }

    #[test]
    fn json_transcode_buffers_source_reads() {
        let mut json = Vec::with_capacity(256 * 1024 + 2);
        json.push(b'"');
        json.extend(std::iter::repeat_n(b'a', 256 * 1024));
        json.push(b'"');
        let mut input = CountingReader {
            input: Cursor::new(json),
            reads: 0,
        };
        let command = parse_args(["--input-format", "json", "."]).unwrap();
        let mut output = Vec::new();
        let mut error = Vec::new();
        assert_eq!(
            run_with_io(command, &mut input, &mut output, &mut error).unwrap(),
            ExitStatus::Success
        );
        assert!(input.reads < 16, "source was read {} times", input.reads);
        assert_eq!(error, [] as [u8; 0]);
    }

    #[test]
    fn json_event_stream_buffers_source_reads() {
        let mut json = Vec::with_capacity(256 * 1024 + 2);
        json.push(b'"');
        json.extend(std::iter::repeat_n(b'a', 256 * 1024));
        json.push(b'"');
        let mut input = CountingReader {
            input: Cursor::new(json),
            reads: 0,
        };
        let command = parse_args([
            "--stream",
            "--input-format",
            "json",
            "--output-format",
            "json",
            ".",
        ])
        .unwrap();
        let mut output = Vec::new();
        let mut error = Vec::new();
        assert_eq!(
            run_with_io(command, &mut input, &mut output, &mut error).unwrap(),
            ExitStatus::Success
        );
        assert!(input.reads < 16, "source was read {} times", input.reads);
        assert_eq!(error, [] as [u8; 0]);
    }

    #[test]
    fn automatic_json_buffers_source_reads() {
        let item = br#"{"value":1,"discarded":[1,2,3]}"#;
        let mut json = Vec::with_capacity(item.len() * 4_000 + 16);
        json.extend_from_slice(br#"{"items":["#);
        for index in 0..4_000 {
            if index != 0 {
                json.push(b',');
            }
            json.extend_from_slice(item);
        }
        json.extend_from_slice(b"]}");
        let mut input = CountingReader {
            input: Cursor::new(json),
            reads: 0,
        };
        let command = parse_args([
            "--input-format",
            "json",
            "--output-format",
            "json",
            ".items[].value",
        ])
        .unwrap();
        let mut output = Vec::new();
        let mut error = Vec::new();
        assert_eq!(
            run_with_io(command, &mut input, &mut output, &mut error).unwrap(),
            ExitStatus::Success
        );
        assert!(input.reads < 16, "source was read {} times", input.reads);
        assert_eq!(error, [] as [u8; 0]);
    }

    #[test]
    fn inputs_json_buffers_source_reads() {
        let mut json = Vec::with_capacity(16 * 1024);
        for _ in 0..4_000 {
            json.extend_from_slice(b"1\n");
        }
        let mut input = CountingReader {
            input: Cursor::new(json),
            reads: 0,
        };
        let command = parse_args([
            "--input-format",
            "json",
            "--output-format",
            "json",
            "[., inputs] | length",
        ])
        .unwrap();
        let mut output = Vec::new();
        let mut error = Vec::new();
        assert_eq!(
            run_with_io(command, &mut input, &mut output, &mut error).unwrap(),
            ExitStatus::Success
        );
        assert_eq!(output, b"4000\n");
        assert!(input.reads < 16, "source was read {} times", input.reads);
        assert!(error.is_empty());
    }

    #[test]
    fn explicit_module_roots_import_include_metadata_and_reject_cycles() {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("sample.jq"),
            "module {kind:\"test\"}; def twice($x): $x * 2;",
        )
        .unwrap();
        fs::write(root.path().join("included.jq"), "def answer: 42;").unwrap();
        fs::write(root.path().join("a.jq"), "include \"b\"; def a: 1;").unwrap();
        fs::write(root.path().join("b.jq"), "include \"a\"; def b: 2;").unwrap();
        fs::write(root.path().join("c.jq"), "include \"d\"; def c: 3;").unwrap();
        fs::write(root.path().join("d.jq"), "def d: 4;").unwrap();
        fs::write(root.path().join("large.jq"), "def large: 123456;").unwrap();
        let root = root.path().to_string_lossy().into_owned();

        let arguments = [
            "-L",
            root.as_str(),
            "-n",
            "--output-format",
            "json",
            "-c",
            "import \"sample\" as s; include \"included\"; s::twice(3), answer, (\"sample\" | modulemeta)",
        ];
        let (status, stdout, stderr) = execute(&arguments, b"");
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(
            stdout,
            b"6\n42\n{\"kind\":\"test\",\"deps\":[],\"defs\":[\"twice/1\"]}\n"
        );
        assert_eq!(stderr, [] as [u8; 0]);

        let cycle = ["-L", root.as_str(), "-n", "include \"a\"; ."];
        let (status, stdout, _) = execute(&cycle, b"must not be consumed");
        let error = status.unwrap_err().to_string();
        assert!(error.contains("cyclic module import"));
        assert!(error.contains("a.jq") && error.contains("b.jq"));
        assert_eq!(stdout, [] as [u8; 0]);

        let escape = ["-L", root.as_str(), "-n", "include \"../outside\"; ."];
        let (status, _, _) = execute(&escape, b"");
        assert!(
            status
                .unwrap_err()
                .to_string()
                .contains("escapes configured roots")
        );

        #[cfg(unix)]
        {
            let outside = tempfile::tempdir().unwrap();
            let target = outside.path().join("outside.jq");
            fs::write(&target, "def escaped: 1;").unwrap();
            std::os::unix::fs::symlink(target, root.as_str().to_owned() + "/linked.jq").unwrap();
            let linked = ["-L", root.as_str(), "-n", "include \"linked\"; ."];
            let (status, _, _) = execute(&linked, b"");
            assert!(
                status
                    .unwrap_err()
                    .to_string()
                    .contains("resolves outside root")
            );
        }

        let count_limited = [
            "-L",
            root.as_str(),
            "--max-depth",
            "1",
            "-n",
            "include \"c\"; .",
        ];
        let (status, _, _) = execute(&count_limited, b"");
        assert!(
            status
                .unwrap_err()
                .to_string()
                .contains("module count limit")
        );

        let bytes_limited = [
            "-L",
            root.as_str(),
            "--max-input-bytes",
            "8",
            "-n",
            "include \"large\"; .",
        ];
        let (status, _, _) = execute(&bytes_limited, b"");
        assert!(status.unwrap_err().to_string().contains("byte limit"));

        let explain = [
            "-L",
            root.as_str(),
            "-n",
            "--explain-json",
            "import \"sample\" as s; s::twice(1)",
        ];
        let (status, _, stderr) = execute(&explain, b"");
        assert_eq!(status.unwrap(), ExitStatus::Success);
        let report: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
        assert_eq!(report["modules"].as_array().unwrap().len(), 1);
        assert_eq!(report["modules"][0]["sha256"].as_str().unwrap().len(), 64);
    }

    #[test]
    fn json_override_raw_join_null_and_exit_status_are_wired() {
        let (status, stdout, _) = execute(
            &[
                "--input-format",
                "json",
                "--output-format",
                "json",
                "-c",
                ".",
            ],
            br#"{"n":9007199254740993}"#,
        );
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(
            stdout,
            br#"{"n":9007199254740993}
"#
        );

        let (status, stdout, _) = execute(&["-n", "-j", "\"x\""], b"ignored");
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(stdout, b"x");

        let (status, _, _) = execute(&["-n", "-e", "empty"], b"");
        assert_eq!(status.unwrap(), ExitStatus::NoResult);
    }

    #[test]
    fn external_values_compile_and_execute() {
        let (status, stdout, _) = execute(&["-n", "--argjson", "n", "42", "$n"], b"");
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(stdout, b"\x1e42\n");
    }

    #[test]
    fn unbuffered_flushes_after_each_complete_result() {
        let command =
            parse_args(["--output-format", "json", "-nc", "--unbuffered", "1, 2"]).unwrap();
        let mut input = &b""[..];
        let mut output = FlushWriter::default();
        let mut error = Vec::new();
        assert_eq!(
            run_with_io(command, &mut input, &mut output, &mut error).unwrap(),
            ExitStatus::Success
        );
        assert_eq!(output.bytes, b"1\n2\n");
        assert_eq!(output.flush_points, [2, 4, 4]);
        assert_eq!(error, [] as [u8; 0]);
    }

    #[test]
    fn help_version_and_compatibility_write_only_stdout() {
        for command in [Command::Help, Command::Version, Command::Compatibility] {
            let mut input = &b""[..];
            let mut output = Vec::new();
            let mut error = Vec::new();
            assert_eq!(
                run_with_io(command, &mut input, &mut output, &mut error).unwrap(),
                ExitStatus::Success
            );
            assert_ne!(output, [] as [u8; 0]);
            assert_eq!(error, [] as [u8; 0]);
        }
    }

    #[test]
    fn prior_framed_results_survive_a_later_runtime_error() {
        let (status, stdout, stderr) = execute(&["-n", "1, error(\"later\")"], b"");
        assert!(matches!(status, Err(super::RunError::Runtime(_))));
        assert_eq!(stdout, b"\x1e1\n");
        assert_eq!(stderr, [] as [u8; 0]);
    }

    #[test]
    fn result_and_output_limits_preserve_complete_prior_frames() {
        let (status, stdout, _) = execute(&["-n", "--max-results", "1", "1, 2"], b"");
        assert!(matches!(
            status,
            Err(super::RunError::Resource("result-count"))
        ));
        assert_eq!(stdout, b"\x1e1\n");

        let (status, stdout, _) = execute(&["-n", "--max-output-bytes", "3", "1, 2"], b"");
        assert_eq!(status.unwrap_err().status(), ExitStatus::Resource);
        assert_eq!(stdout, b"\x1e1\n");

        let (status, stdout, _) = execute(
            &[
                "-n",
                "--max-results",
                "1",
                "foreach (1,2) as $x (0; . + $x; .)",
            ],
            b"",
        );
        assert!(matches!(
            status,
            Err(super::RunError::Resource("result-count"))
        ));
        assert_eq!(stdout, b"\x1e1\n");

        let (status, stdout, _) = execute(
            &[
                "-n",
                "--max-output-bytes",
                "3",
                "foreach (1,2) as $x (0; . + $x; .)",
            ],
            b"",
        );
        assert_eq!(status.unwrap_err().status(), ExitStatus::Resource);
        assert_eq!(stdout, b"\x1e1\n");

        let (status, stdout, _) = execute(
            &["--input-format", "json", "--max-results", "2", ".."],
            b"[1,2]",
        );
        assert!(matches!(
            status,
            Err(super::RunError::Resource("result-count"))
        ));
        assert_eq!(stdout, b"\x1e[2]: 1,2\n\x1e1\n");

        let (status, stdout, _) = execute(
            &["-n", "--max-output-bytes", "5", r#"1, "abcdef=\(.)""#],
            b"",
        );
        assert_eq!(status.unwrap_err().status(), ExitStatus::Resource);
        assert_eq!(stdout, b"\x1e1\n");
    }

    #[test]
    fn input_limits_fail_with_the_resource_exit_category() {
        let (status, stdout, _) = execute(
            &["--input-format", "json", "--max-input-bytes", "3", "."],
            b"null",
        );
        assert_eq!(status.unwrap_err().status(), ExitStatus::Resource);
        assert_eq!(stdout, [] as [u8; 0]);
    }

    #[test]
    fn explain_json_publishes_plan_detection_and_limits() {
        let (status, stdout, stderr) = execute(
            &["--input-format", "json", "--stream", "--explain-json", "."],
            b"[1]",
        );
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_ne!(stdout, [] as [u8; 0]);
        let report: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
        assert_eq!(report["execution"]["plan"], "events");
        assert_eq!(report["execution"]["input_detection"], "override:json");
        assert!(report["execution"]["limits"]["input_bytes"].is_u64());
    }

    #[test]
    fn automatic_subtree_projection_and_selection_preserve_order_and_missing_values() {
        let input = br#"{"features":[{"id":"a","properties":{"mag":1}},{"id":"b","properties":{"mag":3}},{"id":"c","properties":{}}]}"#;
        let (status, stdout, stderr) = execute(
            &[
                "--input-format",
                "json",
                "--output-format",
                "json",
                "-c",
                ".features[].properties.mag",
            ],
            input,
        );
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(stdout, b"1\n3\nnull\n");
        assert_eq!(stderr, [] as [u8; 0]);

        let (status, stdout, stderr) = execute(
            &[
                "--input-format",
                "json",
                "--output-format",
                "json",
                "-c",
                ".features[] | select(.properties.mag >= 2) | .id",
            ],
            input,
        );
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(stdout, b"\"b\"\n");
        assert_eq!(stderr, [] as [u8; 0]);
    }

    #[test]
    fn automatic_event_plan_filters_scalars_without_retaining_containers() {
        let (status, stdout, stderr) = execute(
            &[
                "--input-format",
                "json",
                "--output-format",
                "json",
                "-c",
                ".items[] | numbers",
            ],
            br#"{"items":[1,{"x":2},null,3]}"#,
        );
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(stdout, b"1\n3\n");
        assert_eq!(stderr, [] as [u8; 0]);
    }

    #[test]
    fn automatic_event_plan_uses_auto_detected_json() {
        let (status, stdout, stderr) = execute(
            &[
                "--output-format",
                "json",
                "-c",
                "--explain-json",
                ".items[] | numbers",
            ],
            br#"{"items":[1,{"x":2},null,3]}"#,
        );
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(stdout, b"1\n3\n");
        let explain: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
        assert_eq!(explain["execution"]["plan"], "events");
        assert_eq!(
            explain["execution"]["retained_working_set"],
            "decoder frames, current path, and one scalar event value"
        );
    }

    #[test]
    fn automatic_event_plan_uses_auto_detected_json_file() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.json");
        std::fs::write(&source, br#"{"items":[1,{"x":2},null,3]}"#).unwrap();
        let command = parse_args([
            "--output-format",
            "json",
            "-c",
            "--explain-json",
            ".items[] | numbers",
            source.to_str().unwrap(),
        ])
        .unwrap();
        let mut input = &[][..];
        let mut output = Vec::new();
        let mut error = Vec::new();
        assert_eq!(
            run_with_io(command, &mut input, &mut output, &mut error).unwrap(),
            ExitStatus::Success
        );
        assert_eq!(output, b"1\n3\n");
        let explain: serde_json::Value = serde_json::from_slice(&error).unwrap();
        assert_eq!(explain["execution"]["plan"], "events");
    }

    #[test]
    fn auto_detected_yaml_keeps_document_fallback() {
        let (status, stdout, stderr) = execute(
            &[
                "--output-format",
                "json",
                "-c",
                "--explain-json",
                ".[] | numbers",
            ],
            b"- 1\n- value: 2\n- 3\n",
        );
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(stdout, b"1\n3\n");
        let explain: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
        assert_eq!(explain["execution"]["plan"], "document");
    }

    #[test]
    fn automatic_projection_uses_auto_detected_toon_decoder_events() {
        let (status, stdout, stderr) = execute(
            &["--output-format", "json", "-c", ".items[].x"],
            b"items[2]:\n  - x: 1\n  - x: 2\n",
        );
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(stdout, b"1\n2\n");
        assert_eq!(stderr, [] as [u8; 0]);
    }

    #[test]
    fn automatic_plans_preserve_prior_frames_before_hostile_type_errors() {
        let (status, stdout, stderr) = execute(
            &[
                "--input-format",
                "json",
                "--output-format",
                "json",
                "-c",
                ".items[] | .x",
            ],
            br#"{"items":[{"x":1},2,{"x":3}]}"#,
        );
        assert!(matches!(status, Err(super::RunError::Runtime(_))));
        assert_eq!(stdout, b"1\n");
        assert_eq!(stderr, [] as [u8; 0]);
    }

    #[test]
    fn automatic_subtree_limit_fails_before_the_first_partial_result() {
        let (status, stdout, stderr) = execute(
            &[
                "--input-format",
                "json",
                "--output-format",
                "json",
                "-c",
                "--prepare-memory-bytes",
                "32",
                ".items[] | .x",
            ],
            br#"{"items":[{"x":"a value deliberately larger than the capture limit"}]}"#,
        );
        assert!(matches!(
            status,
            Err(super::RunError::Resource("subtree-bytes"))
        ));
        assert_eq!(stdout, [] as [u8; 0]);
        assert_eq!(stderr, [] as [u8; 0]);
    }

    #[test]
    fn automatic_explain_and_report_publish_proof_and_retention_high_water() {
        let directory = tempfile::tempdir().unwrap();
        let report_path = directory.path().join("automatic.json");
        let command = parse_args([
            "--input-format",
            "json",
            "--output-format",
            "json",
            "-c",
            "--explain-json",
            "--report-file",
            report_path.to_str().unwrap(),
            ".items[] | .x",
        ])
        .unwrap();
        let mut input = br#"{"items":[{"x":1},{"x":2}]}"#.as_slice();
        let mut output = Vec::new();
        let mut error = Vec::new();
        assert_eq!(
            run_with_io(command, &mut input, &mut output, &mut error).unwrap(),
            ExitStatus::Success
        );
        let explain: serde_json::Value = serde_json::from_slice(&error).unwrap();
        assert_eq!(explain["execution"]["plan"], "subtree");
        assert_eq!(
            explain["execution"]["proof"]["required_path_prefix"][0]["value"],
            "items"
        );
        let report: serde_json::Value =
            serde_json::from_slice(&std::fs::read(report_path).unwrap()).unwrap();
        assert_eq!(report["execution"]["plan"], "subtree");
        assert!(
            report["execution"]["retention_high_water"]["bytes"]
                .as_u64()
                .unwrap()
                > 0
        );
        assert_eq!(
            report["execution"]["retention_high_water"]["completed_subtrees"],
            2
        );
    }

    #[test]
    fn automatic_and_document_plans_agree_across_nested_boundary_shapes() {
        let inputs = [
            br#"{"items":[]}"#.as_slice(),
            br#"{"items":[null,false,0,"",[],{}]}"#.as_slice(),
            br#"{"items":[{"x":1},{"x":[2,3]},{"nested":{"x":4}}]}"#.as_slice(),
            br#"{"items":{"first":{"x":1},"second":{"x":2}}}"#.as_slice(),
        ];
        for input in inputs {
            let automatic = execute(
                &[
                    "--input-format",
                    "json",
                    "--output-format",
                    "json",
                    "-c",
                    ".items[] | .x?",
                ],
                input,
            );
            let document = execute(
                &[
                    "--input-format",
                    "yaml",
                    "--output-format",
                    "json",
                    "-c",
                    ".items[] | .x?",
                ],
                input,
            );
            assert_eq!(automatic.0.unwrap(), document.0.unwrap());
            assert_eq!(automatic.1, document.1);
            assert_eq!(automatic.2, [] as [u8; 0]);
            assert_eq!(document.2, [] as [u8; 0]);
        }
    }

    #[test]
    fn non_event_input_reports_deterministic_pre_input_fallback() {
        let (status, stdout, stderr) = execute(
            &["--input-format", "yaml", "--explain-json", ".items[] | .x"],
            br#"{"items":[{"x":1}]}"#,
        );
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_ne!(stdout, [] as [u8; 0]);
        let explain: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
        assert_eq!(explain["execution"]["plan"], "document");
        assert_eq!(
            explain["execution"]["stream_rejection"],
            "the selected input or CLI mode does not expose automatic decoder events"
        );
    }

    #[test]
    fn json_lines_records_are_consistent_across_execution_plans() {
        let input = br#"{"items":[{"x":1},{"x":2}],"values":[1,2]}
{"items":[{"x":3}],"values":[3]}
"#;
        let cases = [
            (
                ".",
                "document",
                "{\"items\":[{\"x\":1},{\"x\":2}],\"values\":[1,2]}\n{\"items\":[{\"x\":3}],\"values\":[3]}\n",
            ),
            (".values[] | numbers", "events", "1\n2\n3\n"),
            (".items[] | .x", "subtree", "1\n2\n3\n"),
            (".items | sort | .[].x", "blocking-document", "1\n2\n3\n"),
        ];
        for (query, expected_plan, expected_output) in cases {
            let (status, output, explain) =
                execute(&["-ijsonl", "-ojsonl", "--explain-json", query], input);
            assert_eq!(status.unwrap(), ExitStatus::Success, "{query}");
            assert_eq!(output, expected_output.as_bytes(), "{query}");
            let explain: serde_json::Value = serde_json::from_slice(&explain).unwrap();
            assert_eq!(explain["execution"]["plan"], expected_plan, "{query}");
        }

        let (status, output, explain) = execute(
            &[
                "-ijsonl",
                "-ojsonl",
                "--slurp",
                "--explain-json",
                "map(.items[]) | map(.x)",
            ],
            input,
        );
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(output, b"[1,2,3]\n");
        let explain: serde_json::Value = serde_json::from_slice(&explain).unwrap();
        assert_eq!(explain["execution"]["plan"], "whole-input");
    }

    #[test]
    fn hybrid_sort_matches_document_and_preserves_json_lines_boundaries() {
        let query = "[.items[].value] | sort";
        let input = br#"{"items":[{"value":3},{"value":1},{"value":2}]}"#;
        let hybrid = execute(&["-ijson", "-ojson", "-c", "--explain-json", query], input);
        let document = execute(&["-iyaml", "-ojson", "-c", query], input);
        assert_eq!(hybrid.0.unwrap(), ExitStatus::Success);
        assert_eq!(document.0.unwrap(), ExitStatus::Success);
        assert_eq!(hybrid.1, document.1);
        let explain: serde_json::Value = serde_json::from_slice(&hybrid.2).unwrap();
        assert_eq!(explain["execution"]["plan"], "hybrid-streaming-blocking");
        assert_eq!(
            explain["execution"]["hybrid_proof"]["preparation"],
            "stable-sort-runs"
        );

        let json_lines = br#"{"items":[{"value":2},{"value":1}]}
{"items":[{"value":4},{"value":3}]}
"#;
        let (status, output, _) = execute(&["-ijsonl", "-ojsonl", query], json_lines);
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(output, b"[1,2]\n[3,4]\n");

        let (status, output, _) = execute(
            &["-ijson", "-ojson", "-c", query],
            br#"{"items":[{"value":2},{"value":1},"#,
        );
        assert_eq!(status.unwrap_err().status(), ExitStatus::Input);
        assert_eq!(output, [] as [u8; 0]);

        let (status, output, _) = execute(
            &[
                "-ijson",
                "-ojson",
                "-c",
                "--hybrid-in-flight-bytes",
                "1",
                query,
            ],
            input,
        );
        assert_eq!(status.unwrap_err().status(), ExitStatus::Resource);
        assert_eq!(output, [] as [u8; 0]);

        let (status, output, _) = execute(
            &["-ijson", "-ojson", "-c", query],
            br#"{"items":[{"value":1,"value":2},{}]}"#,
        );
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(output, b"[null,2]\n");
    }

    #[test]
    fn hybrid_and_forced_document_agree_on_projected_sort_edge_cases() {
        let arguments = [
            "-ijson",
            "-ojson",
            "-c",
            "--hybrid-batch-values",
            "1",
            "--decode-batch-values",
            "1",
            "[.items[].value] | sort",
        ];
        for input in [
            br#"{"items":[]}"#.as_slice(),
            br#"{"items":[{"value":null},{"value":false},{"value":1},{"value":"x"},{"value":[]},{"value":{}}]}"#.as_slice(),
            br#"{"items":[{"value":{"a":1,"b":2}},{"value":{"b":2,"a":1}}]}"#.as_slice(),
            br#"{"items":[{"discarded":{"x":1,"x":2},"value":3},{"value":1,"value":2},{}]}"#.as_slice(),
        ] {
            let hybrid = execute_with_override(&arguments, input, ExecutionOverride::Automatic);
            let document =
                execute_with_override(&arguments, input, ExecutionOverride::Document);
            assert_eq!(hybrid.0.unwrap(), document.0.unwrap());
            assert_eq!(hybrid.1, document.1);
            assert_eq!(hybrid.2, [] as [u8; 0]);
            assert_eq!(document.2, [] as [u8; 0]);
        }

        let input = br#"{"items":{"value":1}}"#;
        let hybrid = execute_with_override(&arguments, input, ExecutionOverride::Automatic);
        let document = execute_with_override(&arguments, input, ExecutionOverride::Document);
        assert_eq!(
            hybrid.0.unwrap_err().status(),
            document.0.unwrap_err().status()
        );
        assert_eq!(hybrid.1, document.1);
    }

    #[test]
    fn hybrid_vm_step_limit_is_shared_across_document_suffixes() {
        let arguments = [
            "-ijsonl",
            "-ojsonl",
            "--max-vm-steps",
            "1",
            "[.items[].value] | sort",
        ];
        let input = br#"{"items":[{"value":2},{"value":1}]}
{"items":[{"value":4},{"value":3}]}
"#;
        let hybrid = execute_with_override(&arguments, input, ExecutionOverride::Automatic);

        assert_eq!(hybrid.0.unwrap_err().status(), ExitStatus::Resource);
        assert_eq!(hybrid.1, b"[1,2]\n");
    }

    #[test]
    fn parallel_decode_preserves_cross_batch_stability_and_downstream_errors() {
        let stable_arguments = [
            "-ijson",
            "-ojson",
            "-c",
            "--decode-batch-values",
            "1",
            "--hybrid-batch-values",
            "1",
            "[.items[]] | sort_by(.key)",
        ];
        let input =
            br#"{"items":[{"key":1,"id":"first"},{"key":0,"id":"zero"},{"key":1,"id":"second"}]}"#;
        let automatic =
            execute_with_override(&stable_arguments, input, ExecutionOverride::Automatic);
        let document = execute_with_override(&stable_arguments, input, ExecutionOverride::Document);
        assert_eq!(automatic.0.unwrap(), document.0.unwrap());
        assert_eq!(automatic.1, document.1);
        assert_eq!(automatic.1, b"[{\"key\":0,\"id\":\"zero\"},{\"key\":1,\"id\":\"first\"},{\"key\":1,\"id\":\"second\"}]\n");

        let fallible_arguments = [
            "-ijson",
            "-ojson",
            "-c",
            "--decode-batch-values",
            "1",
            "[.items[].value] | map(if . == 2 then error(\"boom\") else . end) | sort",
        ];
        let input = br#"{"items":[{"value":3},{"value":2},{"value":1}]}"#;
        let automatic =
            execute_with_override(&fallible_arguments, input, ExecutionOverride::Automatic);
        let document =
            execute_with_override(&fallible_arguments, input, ExecutionOverride::Document);
        assert_eq!(
            automatic.0.unwrap_err().status(),
            document.0.unwrap_err().status()
        );
        assert_eq!(automatic.1, document.1);
    }

    #[test]
    fn nested_static_prefix_is_parallel_eligible_and_dynamic_dependency_is_explained() {
        let (status, output, explain) = execute(
            &[
                "-ijson",
                "-ojson",
                "-c",
                "--explain-json",
                "--decode-batch-values",
                "1",
                "[.root.features[].value] | sort",
            ],
            br#"{"root":{"features":[{"value":2},{"value":1}]}}"#,
        );
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(output, b"[1,2]\n");
        let explain: serde_json::Value = serde_json::from_slice(&explain).unwrap();
        assert_eq!(
            explain["execution"]["parallel_selected_decode"]["eligible"],
            parallel_worker_count() > 1
        );
        if parallel_worker_count() > 1 {
            assert_eq!(
                explain["execution"]["parallel_selected_decode"]["reason"],
                "static-array-prefix"
            );
        }

        let (status, _, explain) = execute(
            &[
                "-ijson",
                "-ojson",
                "-c",
                "--arg",
                "key",
                "items",
                "--explain-json",
                "[.[$key][]] | sort",
            ],
            br#"{"items":[2,1]}"#,
        );
        assert_eq!(status.unwrap(), ExitStatus::Success);
        let explain: serde_json::Value = serde_json::from_slice(&explain).unwrap();
        assert_eq!(
            explain["execution"]["parallel_selected_decode"]["eligible"],
            false
        );
        assert!(
            explain["execution"]["parallel_selected_decode"]["reason"]
                .as_str()
                .is_some_and(|reason| reason.contains("not statically partitionable"))
        );
    }

    #[test]
    fn hybrid_report_and_dead_sort_explain_are_machine_readable() {
        let directory = tempfile::tempdir().unwrap();
        let report_path = directory.path().join("hybrid.json");
        let command = parse_args([
            "-ijson",
            "-ojson",
            "-c",
            "--explain-json",
            "--report-file",
            report_path.to_str().unwrap(),
            "[.items[].value] | sort",
        ])
        .unwrap();
        let mut input = br#"{"items":[{"value":2},{"value":1}]}"#.as_slice();
        let mut output = Vec::new();
        let mut error = Vec::new();
        assert_eq!(
            run_with_io(command, &mut input, &mut output, &mut error).unwrap(),
            ExitStatus::Success
        );
        let explain: serde_json::Value = serde_json::from_slice(&error).unwrap();
        assert_eq!(explain["execution"]["plan"], "hybrid-streaming-blocking");
        assert_eq!(
            explain["execution"]["parallel_selected_decode"]["eligible"],
            parallel_worker_count() > 1
        );
        let report: serde_json::Value =
            serde_json::from_slice(&fs::read(report_path).unwrap()).unwrap();
        assert_eq!(report["execution"]["plan"], "hybrid-streaming-blocking");
        assert_eq!(
            report["execution"]["retention_high_water"]["root_materialized"],
            false
        );
        assert!(report["execution"]["retention_high_water"]["sort_runs"].is_u64());
        assert!(report["execution"]["retention_high_water"]["worker_count"].is_u64());
        assert_eq!(
            report["execution"]["retention_high_water"]["retained_result_count"],
            2
        );
        assert!(
            report["execution"]["retention_high_water"]["retained_estimated_bytes"]
                .as_u64()
                .is_some_and(|bytes| bytes > 0)
        );
        assert!(
            report["execution"]["retention_high_water"]["decoder_depth"]
                .as_u64()
                .is_some_and(|depth| depth >= 3)
        );
        assert_eq!(
            report["execution"]["retention_high_water"]["blocking_state"],
            "projected-collection-and-blocking-suffix"
        );
        assert_eq!(report["execution"]["resource_outcome"], "success");
        assert_eq!(
            report["execution"]["retention_high_water"]["parallel_decode_active"],
            parallel_worker_count() > 1
        );

        let (status, _, explain) = execute(
            &[
                "-ijson",
                "-ojson",
                "-c",
                "--explain-json",
                "[.items[].value] | sort | length",
            ],
            br#"{"items":[{"value":2},{"value":1}]}"#,
        );
        assert_eq!(status.unwrap(), ExitStatus::Success);
        let explain: serde_json::Value = serde_json::from_slice(&explain).unwrap();
        assert_eq!(
            explain["execution"]["optimizer_rewrites"][0]["name"],
            "array-sort-before-length"
        );
        assert_eq!(
            explain["execution"]["hybrid_proof"]["preparation"],
            "collect"
        );
    }

    #[test]
    fn hybrid_report_records_resource_failure_outcome() {
        let directory = tempfile::tempdir().unwrap();
        let failed_report_path = directory.path().join("hybrid-failed.json");
        let command = parse_args([
            "-ijson",
            "-ojson",
            "-c",
            "--hybrid-in-flight-bytes",
            "1",
            "--report-file",
            failed_report_path.to_str().unwrap(),
            "[.items[].value] | sort",
        ])
        .unwrap();
        let mut input = br#"{"items":[{"value":2},{"value":1}]}"#.as_slice();
        let mut output = Vec::new();
        let mut error = Vec::new();
        assert_eq!(
            run_with_io(command, &mut input, &mut output, &mut error)
                .unwrap_err()
                .status(),
            ExitStatus::Resource
        );
        assert_eq!(output, [] as [u8; 0]);
        let failed_report: serde_json::Value =
            serde_json::from_slice(&fs::read(failed_report_path).unwrap()).unwrap();
        assert_eq!(
            failed_report["execution"]["resource_outcome"],
            "resource-limit"
        );
        assert!(
            failed_report["execution"]["retention_high_water"]["retained_result_count"]
                .as_u64()
                .is_some_and(|count| count > 0)
        );
    }
}
