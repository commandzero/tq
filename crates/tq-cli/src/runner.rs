//! End-to-end command runner with strict stdout/stderr separation.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{self, BufReader, IsTerminal, Read, Write},
    path::Path,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

use thiserror::Error;
use tq_core::{
    Analysis, AnalysisContext, Analyzed, AutomaticPlan, Compiled, Diagnostic, Events, Number,
    PathComponent, Plan, PlanKind, Query, ResolveOptions, Resolved, Value, Vm, VmError, VmLimits,
    VmObservations, analyze_with_context, parse_bytes, resolve,
};
use tq_formats::{
    DecodeOptions, FormatError, InputFormat, OutputError, OutputOptions, StreamOptions,
    ToonFraming, decode_bytes, decode_json, decode_toon, probe_reader, stream_json, stream_toon,
    write_results,
};

use crate::{
    CliError, ColorMode, Command, ExitStatus, ExplainFormat, ExternalArgumentKind, FilterSource,
    PositionalArgumentKind, RunOptions, generated_help,
};

static CANCELLATION: OnceLock<Arc<AtomicBool>> = OnceLock::new();

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
            Self::Runtime(_) | Self::Output(_) | Self::Json(_) | Self::RawOutput(_) => {
                ExitStatus::Runtime
            }
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
    let mut stdin = io::stdin().lock();
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
pub fn run_with_io<R: Read, W: Write, E: Write>(
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
                "../../../tests/compatibility/reviews/coverage-v1.json"
            ))?;
            stdout.write_all(b"\n")?;
            Ok(ExitStatus::Success)
        }
        Command::BuildConfiguration => {
            writeln!(
                stdout,
                "target={} binary-stdio={} formats=toon,yaml,json jq-target=1.8.x",
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
fn run_filter<R: Read, W: Write, E: Write>(
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
        let file_events = auto_file_events_available(options)?;
        if options.files.is_empty() || options.files.iter().any(|path| path == Path::new("-")) {
            let reader = LimitedReader::new(&mut *stdin, options.limits.input_bytes, "<stdin>");
            let (probe, mut replay) = probe_reader(reader, options.limits.lookahead_bytes)?;
            return run_resolved_filter(
                options,
                resolved,
                &variables,
                file_events && decoder_events_available(probe.selected),
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
            stdin,
            stdout,
            stderr,
        );
    }
    run_resolved_filter(
        options,
        resolved,
        &variables,
        automatic_mode && matches!(options.input_format, InputFormat::Json | InputFormat::Toon),
        stdin,
        stdout,
        stderr,
    )
}

const fn decoder_events_available(format: InputFormat) -> bool {
    matches!(format, InputFormat::Json | InputFormat::Toon)
}

fn auto_file_events_available(options: &RunOptions) -> Result<bool, RunError> {
    let mut available = true;
    for path in options.files.iter().filter(|path| *path != Path::new("-")) {
        let identity = path.display().to_string();
        let reader = LimitedReader::new(open_path(path)?, options.limits.input_bytes, &identity);
        let (probe, _) = probe_reader(reader, options.limits.lookahead_bytes)?;
        available &= decoder_events_available(probe.selected);
    }
    Ok(available)
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "the command lifecycle keeps pre-input planning visibly ahead of decoder execution"
)]
fn run_resolved_filter<R: Read, W: Write, E: Write>(
    options: &RunOptions,
    resolved: Query<Resolved>,
    variables: &BTreeMap<Arc<str>, Value>,
    automatic_streaming: bool,
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
    if let Some(explain) = options.explain {
        write_explain(explain, options, &analyzed, stderr)?;
    }
    let analysis = analyzed.analysis().clone();
    let program = analyzed.compile().map_err(RunError::Compile)?;

    if options.stream {
        let plan = program.event_plan().map_err(RunError::Compile)?;
        return run_event_filter(options, &plan, variables, &analysis, stdin, stdout, stderr);
    }
    if matches!(analysis.selected_plan, PlanKind::Events | PlanKind::Subtree) {
        return match program.automatic_plan().map_err(RunError::Compile)? {
            AutomaticPlan::Events(plan) => {
                run_automatic_filter(options, &plan, variables, &analysis, stdin, stdout, stderr)
            }
            AutomaticPlan::Subtree(plan) => {
                run_automatic_filter(options, &plan, variables, &analysis, stdin, stdout, stderr)
            }
            _ => unreachable!("bounded selection returns a bounded typed plan"),
        };
    }
    let plan = program.document_plan();

    let inputs = load_inputs(options, stdin)?;

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
    let mut result_output = ResultOutput::new(stdout, options);
    let mut result_count = 0_usize;
    let mut last = None;
    let mut observations = Vec::new();
    let mut runtime_error = None;
    'documents: for input in values {
        let mut vm = Vm::new_with_variables(&plan, input, vm_limits(options), variables.clone())
            .with_trace_limit(options.trace_limit);
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
        if runtime_error.is_some() {
            break 'documents;
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
            },
        )?;
    }
    if let Some(error) = runtime_error {
        return Err(RunError::Runtime(error));
    }
    Ok(exit_status(options.exit_status, last.as_ref()))
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

fn write_explain(
    format: ExplainFormat,
    options: &RunOptions,
    analyzed: &Query<Analyzed>,
    stderr: &mut impl Write,
) -> Result<(), RunError> {
    let capabilities = analyzed.capabilities();
    let plan = analyzed.analysis().selected_plan;
    let retained = match plan {
        PlanKind::Events if capabilities.fold_state => {
            "decoder frames, current path, one event value, and one fold accumulator"
        }
        PlanKind::Events => "decoder frames, current path, and one scalar event value",
        PlanKind::WholeInput => "all input documents",
        PlanKind::Blocking => "one document plus blocking operator state",
        PlanKind::Subtree => "one selected complete subtree",
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
            writeln!(stderr, "spool-required: false")?;
            if let Some(proof) = &analyzed.analysis().stream_proof {
                writeln!(
                    stderr,
                    "required-path-prefix: {:?}",
                    proof.required_path_prefix
                )?;
                writeln!(stderr, "subtree-complete: {}", proof.subtree_complete)?;
                writeln!(stderr, "value-escapes: {}", proof.value_escapes)?;
                writeln!(stderr, "retention-high-water: available in --report-file")?;
            }
            if let Some(rejection) = &analyzed.analysis().stream_rejection {
                writeln!(stderr, "stream-rejection: {rejection}")?;
            }
            writeln!(
                stderr,
                "limits: input={} depth={} token={} line={} lookahead={} vm-steps={} results={} output={} prepare-memory={} spool={}",
                options.limits.input_bytes,
                options.limits.depth,
                options.limits.token_bytes,
                options.limits.line_bytes,
                options.limits.lookahead_bytes,
                options.limits.vm_steps,
                options.limits.results,
                options.limits.output_bytes,
                options.limits.preparation_memory_bytes,
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
                "spool_required": false,
                "proof": analyzed.analysis().stream_proof,
                "stream_rejection": analyzed.analysis().stream_rejection,
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
                    "spool_bytes": options.limits.spool_bytes,
                }
            });
            serde_json::to_writer_pretty(&mut *stderr, &report)?;
            stderr.write_all(b"\n")?;
        }
    }
    Ok(())
}

const fn input_format_name(format: InputFormat) -> &'static str {
    match format {
        InputFormat::Auto => "auto:toon/json/yaml-bounded-probe",
        InputFormat::Toon => "override:toon",
        InputFormat::Yaml => "override:yaml",
        InputFormat::Json => "override:json",
        InputFormat::ToonSequence => "override:toon-sequence",
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
            stream_reader(options, &mut *stdin, "<stdin>", &mut executor)?;
        } else {
            let identity = path.display().to_string();
            stream_reader(options, open_path(&path)?, &identity, &mut executor)?;
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
            },
        )?;
    }
    Ok(exit_status(options.exit_status, executor.last.as_ref()))
}

#[derive(Clone, Copy, Debug, Default)]
struct RetentionObservations {
    bytes_high_water: usize,
    depth_high_water: usize,
    completed_subtrees: u64,
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
            automatic_reader(options, &mut *stdin, "<stdin>", &mut executor)?;
        } else {
            let identity = path.display().to_string();
            automatic_reader(options, open_path(&path)?, &identity, &mut executor)?;
        }
        executor.finish_source()?;
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
            },
        )?;
    }
    Ok(exit_status(options.exit_status, executor.last.as_ref()))
}

fn automatic_reader<R: Read, W: Write, E: Write, M>(
    options: &RunOptions,
    reader: R,
    identity: &str,
    executor: &mut AutomaticExecutor<'_, W, E, M>,
) -> Result<(), RunError> {
    let stream_options = StreamOptions {
        maximum_depth: options.limits.depth,
        errors_as_values: false,
    };
    let reader = LimitedReader::new(reader, options.limits.input_bytes, identity);
    match options.input_format {
        InputFormat::Json => automatic_json_into(reader, stream_options, executor),
        InputFormat::Toon => automatic_toon_into(reader, options, stream_options, executor),
        InputFormat::Auto => {
            let (report, replay) = probe_reader(reader, options.limits.lookahead_bytes)?;
            match report.selected {
                InputFormat::Json => automatic_json_into(replay, stream_options, executor),
                InputFormat::Toon => automatic_toon_into(replay, options, stream_options, executor),
                InputFormat::Yaml => Err(RunError::Unsupported(
                    "auto-detected YAML cannot execute a decoder-event plan".to_owned(),
                )),
                InputFormat::Auto | InputFormat::ToonSequence => unreachable!("probe candidate"),
            }
        }
        InputFormat::Yaml | InputFormat::ToonSequence => Err(RunError::Unsupported(
            "automatic bounded plans require JSON or TOON decoder events".to_owned(),
        )),
    }
}

fn automatic_json_into<R: Read, W: Write, E: Write, M>(
    reader: R,
    options: StreamOptions,
    executor: &mut AutomaticExecutor<'_, W, E, M>,
) -> Result<(), RunError> {
    let mut execution_error = None;
    let decoded = stream_json(reader, options, |record| match executor.accept(record) {
        Ok(()) => Ok(()),
        Err(error) => {
            execution_error = Some(error);
            Err("automatic stream consumer stopped".to_owned())
        }
    });
    if let Some(error) = execution_error {
        return Err(error);
    }
    decoded.map_err(RunError::Input)
}

fn automatic_toon_into<R: Read, W: Write, E: Write, M>(
    reader: R,
    options: &RunOptions,
    stream_options: StreamOptions,
    executor: &mut AutomaticExecutor<'_, W, E, M>,
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
                Err("automatic stream consumer stopped".to_owned())
            }
        },
    );
    if let Some(error) = execution_error {
        return Err(error);
    }
    decoded.map_err(RunError::Input)
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
    last: Option<Value>,
    results: usize,
}

impl<W: Write, E: Write, M> AutomaticExecutor<'_, W, E, M> {
    fn accept(&mut self, record: Value) -> Result<(), RunError> {
        if cancellation().is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err(RunError::Interrupted);
        }
        let (path, value) = decode_event_record(record)?;
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
            return self.complete_projection_item();
        }
        self.complete_capture()
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
        let mut vm = if base {
            Vm::new_automatic_base(
                self.plan,
                input,
                vm_limits(self.output.options),
                self.variables.clone(),
            )
        } else {
            Vm::new_automatic_item(
                self.plan,
                input,
                vm_limits(self.output.options),
                self.variables.clone(),
            )
        }
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

fn decode_event_record(record: Value) -> Result<(Vec<PathComponent>, Option<Value>), RunError> {
    let Value::Array(parts) = record else {
        return Err(invalid_automatic_event());
    };
    if !(1..=2).contains(&parts.len()) {
        return Err(invalid_automatic_event());
    }
    let Value::Array(path) = &parts[0] else {
        return Err(invalid_automatic_event());
    };
    let path = path
        .iter()
        .map(|component| match component {
            Value::String(key) => Ok(PathComponent::Key(Arc::clone(key))),
            Value::Number(index) => index
                .to_string()
                .parse::<usize>()
                .map(PathComponent::Index)
                .map_err(|_| invalid_automatic_event()),
            _ => Err(invalid_automatic_event()),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((path, parts.get(1).cloned()))
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
    reader: R,
    identity: &str,
    executor: &mut StreamExecutor<'_, W, E>,
) -> Result<(), RunError> {
    let stream_options = StreamOptions {
        maximum_depth: options.limits.depth,
        errors_as_values: options.stream_errors,
    };
    let reader = LimitedReader::new(reader, options.limits.input_bytes, identity);
    match options.input_format {
        InputFormat::Json => stream_json_into(reader, stream_options, executor),
        InputFormat::Toon => stream_toon_into(reader, options, stream_options, executor),
        InputFormat::Auto => {
            let (report, replay) = probe_reader(reader, options.limits.lookahead_bytes)?;
            match report.selected {
                InputFormat::Json => stream_json_into(replay, stream_options, executor),
                InputFormat::Toon => stream_toon_into(replay, options, stream_options, executor),
                InputFormat::Yaml => Err(RunError::Unsupported(
                    "auto-detection selected YAML, which is document-at-a-time and cannot satisfy --stream; use --input-format json for JSON syntax".to_owned(),
                )),
                InputFormat::Auto | InputFormat::ToonSequence => unreachable!("probe candidate"),
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
}

impl<'a, W: Write> ResultOutput<'a, W> {
    fn new(writer: &'a mut W, options: &'a RunOptions) -> Self {
        Self {
            writer,
            options,
            unframed: None,
            written: 0,
            emitted: 0,
        }
    }

    fn emit(&mut self, value: &Value) -> Result<(), RunError> {
        if self.emitted >= self.options.limits.results {
            return Err(RunError::Resource("result-count"));
        }
        self.emitted = self.emitted.saturating_add(1);
        let sorted;
        let value = if self.options.sort_keys {
            sorted = sort_value_keys(value);
            &sorted
        } else {
            value
        };
        if self.options.raw_output {
            let mut writer = LimitedWriter::new(
                &mut self.writer,
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
            if self.unframed.replace(value.clone()).is_some() {
                return Err(OutputError::Toon(tq_toon::SequenceError::Cardinality(
                    tq_toon::CardinalityError::Multiple,
                ))
                .into());
            }
            return Ok(());
        }
        let mut writer = LimitedWriter::new(
            &mut self.writer,
            &mut self.written,
            self.options.limits.output_bytes,
        );
        if self.options.output_format == tq_formats::OutputFormat::Json
            && !self.options.pretty_json
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

    fn finish(&mut self) -> Result<(), RunError> {
        if self.options.output_format == tq_formats::OutputFormat::Toon
            && self.options.framing == ToonFraming::Unframed
        {
            let mut writer = LimitedWriter::new(
                &mut self.writer,
                &mut self.written,
                self.options.limits.output_bytes,
            );
            write_results(
                &mut writer,
                self.unframed.iter(),
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
        }
        if self.options.unbuffered {
            self.writer.flush()?;
        }
        Ok(())
    }

    const fn written(&self) -> u64 {
        self.written
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

fn load_inputs<R: Read>(
    options: &RunOptions,
    stdin: &mut R,
) -> Result<Vec<tq_formats::Document>, RunError> {
    if options.null_input {
        return Ok(vec![tq_formats::Document {
            value: Value::Null,
            identity: "<null-input>".to_owned(),
            format: InputFormat::Auto,
            index: 0,
        }]);
    }
    let files = if options.files.is_empty() {
        vec![Path::new("-").to_owned()]
    } else {
        options.files.clone()
    };
    let mut documents = Vec::new();
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
            let decode = DecodeOptions {
                format: options.input_format,
                maximum_source_bytes: usize::try_from(options.limits.input_bytes)
                    .unwrap_or(usize::MAX),
                toon: tq_toon::DecoderConfig {
                    strict: options.strict,
                    maximum_depth: options.limits.depth,
                    maximum_token_bytes: options.limits.token_bytes,
                    maximum_line_bytes: options.limits.line_bytes,
                    maximum_lookahead_bytes: options.limits.lookahead_bytes,
                    ..tq_toon::DecoderConfig::default()
                },
            };
            documents.extend(decode_bytes(&bytes, identity, decode)?);
        }
    }
    Ok(documents)
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
    } = execution;
    let report = serde_json::json!({
        "schema_version": 1,
        "documents": observations.len(),
        "results": results,
        "output_bytes": output_bytes,
        "execution": {
            "plan": plan.to_string(),
            "proof": analysis.stream_proof,
            "stream_rejection": analysis.stream_rejection,
            "retained_working_set": match plan {
                PlanKind::Events => "decoder-events",
                PlanKind::Subtree => "selected-subtree",
                PlanKind::Document => "document",
                PlanKind::WholeInput => "whole-input",
                PlanKind::Blocking => "document-and-blocking-state",
            },
            "retention_high_water": {
                "bytes": retention.bytes_high_water,
                "depth": retention.depth_high_water,
                "completed_subtrees": retention.completed_subtrees,
            },
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
struct ReportExecution<'a> {
    analysis: &'a Analysis,
    retention: RetentionObservations,
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
        io::{self, Write},
    };

    use super::run_with_io;
    use crate::{Command, ExitStatus, parse_args};

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

    #[test]
    fn ambient_builtins_are_denied_by_default_and_admitted_explicitly() {
        let (denied, output, error) = execute(&["--output-format", "json", "-c", "env"], b"null\n");
        assert!(denied.is_err());
        assert!(output.is_empty());
        let error = String::from_utf8(error).expect("UTF-8 stderr");
        assert!(error.is_empty(), "run_with_io returns errors to its caller");

        for query in ["now", "input_filename"] {
            let (denied, output, error) =
                execute(&["--output-format", "json", "-c", query], b"null\n");
            let denied = denied.expect_err("platform access should be denied by default");
            assert_eq!(denied.status(), ExitStatus::Runtime);
            assert!(denied.to_string().contains("capability policy"));
            assert!(output.is_empty());
            assert!(error.is_empty());
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
        assert!(error.is_empty());

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
        assert!(error.is_empty());
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

    #[test]
    fn identity_uses_toon_sequence_and_keeps_stderr_clean() {
        let (status, stdout, stderr) = execute(&["."], b"name: Ada");
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(stdout, b"\x1ename: Ada\n");
        assert!(stderr.is_empty());
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
        assert!(stderr.is_empty());

        let cycle = ["-L", root.as_str(), "-n", "include \"a\"; ."];
        let (status, stdout, _) = execute(&cycle, b"must not be consumed");
        let error = status.unwrap_err().to_string();
        assert!(error.contains("cyclic module import"));
        assert!(error.contains("a.jq") && error.contains("b.jq"));
        assert!(stdout.is_empty());

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
        assert!(error.is_empty());
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
            assert!(!output.is_empty());
            assert!(error.is_empty());
        }
    }

    #[test]
    fn prior_framed_results_survive_a_later_runtime_error() {
        let (status, stdout, stderr) = execute(&["-n", "1, error(\"later\")"], b"");
        assert!(matches!(status, Err(super::RunError::Runtime(_))));
        assert_eq!(stdout, b"\x1e1\n");
        assert!(stderr.is_empty());
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
        assert!(stdout.is_empty());
    }

    #[test]
    fn explain_json_publishes_plan_detection_and_limits() {
        let (status, stdout, stderr) = execute(
            &["--input-format", "json", "--stream", "--explain-json", "."],
            b"[1]",
        );
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert!(!stdout.is_empty());
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
        assert!(stderr.is_empty());

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
        assert!(stderr.is_empty());
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
        assert!(stderr.is_empty());
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
        assert!(stderr.is_empty());
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
        assert!(stderr.is_empty());
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
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
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
            assert!(automatic.2.is_empty());
            assert!(document.2.is_empty());
        }
    }

    #[test]
    fn non_event_input_reports_deterministic_pre_input_fallback() {
        let (status, stdout, stderr) = execute(
            &["--input-format", "yaml", "--explain-json", ".items[] | .x"],
            br#"{"items":[{"x":1}]}"#,
        );
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert!(!stdout.is_empty());
        let explain: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
        assert_eq!(explain["execution"]["plan"], "document");
        assert_eq!(
            explain["execution"]["stream_rejection"],
            "the selected input or CLI mode does not expose automatic decoder events"
        );
    }
}
