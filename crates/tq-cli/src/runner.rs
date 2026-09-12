//! End-to-end command runner with strict stdout/stderr separation.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fs::{self, File},
    io::{self, BufReader, BufWriter, IsTerminal, Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, SyncSender, sync_channel},
    },
    thread,
    time::Duration,
};

use thiserror::Error;
use tq_core::{
    Analysis, AnalysisContext, Analyzed, AutomaticPlan, Compiled, Diagnostic, EffectSink, Events,
    HybridBlocking, HybridPreparation, InputCursor, InputValue, JsonInputOptions, Label, Number,
    PathComponent, Plan, PlanKind, Query, ResolveOptions, Resolved, SourceId, StableSortPipeline,
    StableSortPipelineObservations, Transcode, TranscodeCommitment, TranscodeDuplicatePolicy,
    TranscodeInput, TranscodeLimits, TranscodeProof, Value, Vm, VmError, VmLimits, VmObservations,
    analyze_with_context, parallel_worker_count, parse_bytes, parse_json_bytes, parse_with_startup,
    resolve,
};
use tq_formats::{
    DecodeOptions, FormatError, InputFormat, InputRepresentation, JsonColorPalette,
    JsonDocumentSource, JsonLinesDocumentSource, NativeFormat, NativeInputObservation,
    NativeOutputSequence, OutputError, OutputFormat, OutputOptions, ParallelJsonOptions,
    ProbeReport, SelectedRootSink, SelectedStreamObservations, SelectionFallback,
    SelectionReplacement, StreamOptions, StreamRecord, StreamSelection, ToonFraming, decode_bytes,
    decode_json, decode_toon, probe_format, probe_reader, stream_json,
    stream_json_selected_roots_with_control, stream_toon,
};
use tq_toon::{
    ArrayPreparationConfig, DuplicateKeyPolicy, KeyFolding, PreparationArena, PreparationLimits,
    PreparationObservations, PublicationBuffer, PublicationError, SpoolError, TranscodeConsumer,
    TranscodeError, WriterError,
};

use crate::runtime_spool::{
    REPLAY_BUFFER_BYTES, RuntimeRecordId, RuntimeSpool, RuntimeSpoolConfig, RuntimeSpoolError,
    RuntimeSpoolItem, RuntimeSpoolReplayError,
};
use crate::{
    CliError, ColorMode, Command, ExecutionOverride, ExitStatus, ExplainFormat,
    ExternalArgumentKind, FilterSource, PositionalArgumentKind, RunOptions, generated_help,
    parse_args,
};

static CANCELLATION: OnceLock<Arc<AtomicBool>> = OnceLock::new();
const INPUT_BUFFER_BYTES: usize = 64 * 1024;
const RUN_TEST_FILE_BYTES: u64 = 64 * 1024 * 1024;
// The root object index owns one insertion-order Vec slot and two hash-table
// entries (latest record and membership).  Charge a conservative node budget
// for both tables in addition to the shared Arc/key slots before insertion.
const ROOT_INDEX_BTREE_NODE_BYTES: usize = 512;

const AMBIENT_ENVIRONMENT: &str = "__tq_ambient_environment";
const AMBIENT_PLATFORM: &str = "__tq_ambient_platform";
const INPUT_FILENAME: &str = "__tq_input_filename";
const INPUT_LINE_NUMBER: &str = "__tq_input_line_number";
const JSON5_STREAM_UNSUPPORTED: &str =
    "JSON5 input is document-at-a-time and cannot satisfy --stream";

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
    /// A recoverable document-root runtime failure whose diagnostic was
    /// already written while input processing continued.
    #[error(transparent)]
    ReportedRuntime(VmError),
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
            Self::Cli(CliError::Unsupported(_)) | Self::Unsupported(_) => ExitStatus::Unsupported,
            Self::Cli(_) | Self::Io(_) | Self::IoPath { .. } => ExitStatus::Usage,
            Self::Compile(_) => ExitStatus::Compile,
            Self::Resource(_)
            | Self::ResourceSource { .. }
            | Self::Input(FormatError::Resource(_))
            | Self::Output(OutputError::Resource(_)) => ExitStatus::Resource,
            Self::Interrupted => ExitStatus::Interrupted,
            Self::Runtime(error) | Self::ReportedRuntime(error) => match error {
                VmError::Unsupported { .. } => ExitStatus::Unsupported,
                VmError::Resource { .. } => ExitStatus::Resource,
                VmError::Interrupted => ExitStatus::Interrupted,
                VmError::Halt { status, .. } => ExitStatus::Halt(*status),
                _ => ExitStatus::Runtime,
            },
            Self::Input(error) if error.to_string().contains("resource limit exceeded") => {
                ExitStatus::Resource
            }
            Self::Output(error) if error.to_string().contains("resource limit exceeded") => {
                ExitStatus::Resource
            }
            Self::Input(_) => ExitStatus::Input,
            Self::Output(_) | Self::Json(_) | Self::RawOutput(_) | Self::Cardinality(_) => {
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
        let json_color = options.output_format == OutputFormat::Json;
        options.color = if json_color && terminal && !no_color {
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
    // `run` is the process-facing entry point.  Keep ambient capabilities
    // enabled for `--run-tests` here, while the injectable `run_with_io`
    // entry point remains confined by default.
    let result = run_with_io_policy(command, &mut stdin, &mut stdout, &mut stderr, true);
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
            if let RunError::Runtime(VmError::Halt {
                status,
                stderr: bytes,
            }) = &error
            {
                if stderr.write_all(bytes).is_err() {
                    return ExitStatus::Runtime;
                }
                return ExitStatus::Halt(*status);
            }
            if !matches!(&error, RunError::ReportedRuntime(_)) {
                let _ = writeln!(stderr, "tq: {}", process_error_message(&error));
            }
            error.status()
        }
    }
}

fn process_error_message(error: &RunError) -> String {
    match error {
        RunError::Input(FormatError::Parse {
            format: InputFormat::JsonSequence,
            message,
        }) => format!("ignoring parse error: {message}"),
        _ => error.to_string(),
    }
}

fn is_recoverable_document_runtime_error(error: &VmError) -> bool {
    matches!(
        error,
        VmError::Runtime { .. }
            | VmError::Raised { .. }
            | VmError::NumericRange { .. }
            | VmError::RecoverableInput { .. }
    )
}

fn report_document_runtime_error<E: Write>(
    stderr: &mut E,
    error: &VmError,
) -> Result<(), RunError> {
    let error = RunError::Runtime(error.clone());
    writeln!(stderr, "tq: {}", process_error_message(&error))?;
    Ok(())
}

fn process_input_diagnostic(diagnostic: &str) -> String {
    diagnostic
        .strip_prefix("JsonSequence input rejected: ")
        .map_or_else(
            || diagnostic.to_owned(),
            |message| format!("ignoring parse error: {message}"),
        )
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

fn flush_vm_effects<E: Write>(vm: &Vm, stderr: &mut E) -> Result<(), RunError> {
    let effects = vm.take_effects();
    if effects.is_empty() {
        return Ok(());
    }
    stderr.write_all(&effects)?;
    Ok(())
}

fn flush_effect_sink<E: Write>(sink: &EffectSink, stderr: &mut E) -> Result<(), RunError> {
    sink.write_to(stderr)?;
    Ok(())
}

enum LiveVmMessage {
    Value {
        value: Value,
        acknowledged: SyncSender<()>,
    },
    Complete {
        result: Result<(), VmError>,
        observations: VmObservations,
        trace: Vec<String>,
    },
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
    run_with_io_policy(command, stdin, stdout, stderr, false)
}

fn run_with_io_policy<R: Read + Send, W: Write, E: Write>(
    command: Command,
    stdin: &mut R,
    stdout: &mut W,
    stderr: &mut E,
    process_ambient: bool,
) -> Result<ExitStatus, RunError> {
    run_with_io_policy_capture(command, stdin, stdout, stderr, process_ambient, None)
}

fn run_with_io_policy_capture<R: Read + Send, W: Write, E: Write>(
    command: Command,
    stdin: &mut R,
    stdout: &mut W,
    stderr: &mut E,
    process_ambient: bool,
    capture: Option<RunTestCaptureHandle>,
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
            let formats = NativeFormat::ALL
                .map(|format| format.descriptor().name)
                .join(",");
            writeln!(
                stdout,
                "target={} binary-stdio={} formats={formats} jq-target=1.8.x",
                std::env::consts::OS,
                if cfg!(windows) {
                    "requested-with--binary"
                } else {
                    "native"
                },
            )?;
            Ok(ExitStatus::Success)
        }
        Command::RunTests(path) => {
            run_tests(path.as_deref(), stdin, stdout, stderr, process_ambient)
        }
        Command::Run(options) => match run_filter(&options, stdin, stdout, stderr, capture) {
            // All execution plans return an exit status once the diagnostic
            // has already been emitted; callers must not print it again.
            Err(error @ RunError::ReportedRuntime(_)) => Ok(error.status()),
            result => result,
        },
    }
}

struct RunTestCase {
    line: usize,
    filter: String,
    input: String,
    expected: Vec<String>,
    compile_failure: bool,
}

struct RunTestOutcome {
    status: ExitStatus,
    diagnostic: String,
    structured: Option<Box<Diagnostic>>,
    output: Vec<u8>,
    typed_capture: Option<RunTestCaptureSnapshot>,
}

#[derive(Clone)]
struct RunTestCaptureSnapshot {
    values: Vec<Value>,
    overflowed: bool,
}

struct RunTestCapture {
    values: Vec<Value>,
    retained_bytes: usize,
    maximum_values: usize,
    maximum_bytes: usize,
    overflowed: bool,
}

type RunTestCaptureHandle = Arc<Mutex<RunTestCapture>>;

impl RunTestCapture {
    fn new(expected_values: usize) -> Self {
        Self {
            values: Vec::new(),
            retained_bytes: 0,
            maximum_values: expected_values.saturating_add(1),
            maximum_bytes: usize::try_from(RUN_TEST_FILE_BYTES)
                .expect("run-tests file cap fits in usize"),
            overflowed: false,
        }
    }

    fn record(&mut self, value: &Value) {
        if self.overflowed {
            return;
        }
        let estimated = estimate_capture_value_bytes(value);
        if self.values.len() >= self.maximum_values
            || self.retained_bytes.saturating_add(estimated) > self.maximum_bytes
            || self.values.try_reserve_exact(1).is_err()
        {
            self.overflowed = true;
            return;
        }
        self.retained_bytes = self.retained_bytes.saturating_add(estimated);
        self.values.push(value.clone());
    }

    fn snapshot(&self) -> RunTestCaptureSnapshot {
        RunTestCaptureSnapshot {
            values: self.values.clone(),
            overflowed: self.overflowed,
        }
    }
}

fn run_tests<R: Read, W: Write, E: Write>(
    path: Option<&Path>,
    stdin: &mut R,
    stdout: &mut W,
    _stderr: &mut E,
    process_ambient: bool,
) -> Result<ExitStatus, RunError> {
    let bytes = if let Some(path) = path.filter(|path| *path != Path::new("-")) {
        read_limited(
            open_path(path)?,
            RUN_TEST_FILE_BYTES,
            &path.display().to_string(),
        )?
    } else {
        read_limited(&mut *stdin, RUN_TEST_FILE_BYTES, "<run-tests>")?
    };
    let source = String::from_utf8(bytes)
        .map_err(|_| RunError::Cli(CliError::Usage("test file must be UTF-8".to_owned())))?;
    let (cases, mut malformed) = parse_run_test_cases(&source);
    let mut passed = 0_usize;
    for (number, case) in cases.iter().enumerate() {
        writeln!(
            stdout,
            "Test #{}: '{}' at line number {}",
            number + 1,
            case.filter,
            case.line
        )?;
        let outcome = run_test_case(case, process_ambient)?;
        let RunTestOutcome {
            status,
            diagnostic,
            structured,
            output,
            typed_capture,
        } = outcome;
        let good = if case.compile_failure {
            status == ExitStatus::Compile
                && compile_diagnostic_matches(
                    &diagnostic,
                    structured.as_deref(),
                    &case.expected,
                    &case.filter,
                )
        } else {
            status == ExitStatus::Success
                && run_test_output_matches(&output, &case.expected, typed_capture.as_ref())
        };
        if good {
            passed = passed.saturating_add(1);
        } else if case.compile_failure {
            malformed = malformed.saturating_add(1);
            if status == ExitStatus::Compile {
                let displayed_diagnostic =
                    compile_diagnostic_report(&diagnostic, structured.as_deref(), &case.filter);
                writeln!(
                    stdout,
                    "*** Erroneous program failed with '{}', but expected '{}' at line number {}: {}",
                    displayed_diagnostic,
                    case.expected.join("\\n"),
                    case.line.saturating_add(1),
                    case.filter
                )?;
            } else {
                writeln!(
                    stdout,
                    "*** Test program did not fail to compile at line {}: {}",
                    case.line, case.filter
                )?;
            }
        } else {
            writeln!(
                stdout,
                "*** Expected {}, but got {} for test at line number {}: {}",
                case.expected.join("\\n"),
                String::from_utf8_lossy(&output).trim_end(),
                case.line,
                case.filter
            )?;
        }
    }
    writeln!(
        stdout,
        "{} of {} tests passed ({} malformed, 0 skipped)",
        passed,
        cases.len(),
        malformed
    )?;
    let self_checks_passed = passed == cases.len() && malformed == 0 && run_tests_self_checks();
    if self_checks_passed {
        writeln!(stdout, "Test jq_state: .[]")?;
        writeln!(
            stdout,
            "Test jq_state: .[] | if .%2 == 0 then halt_error else . end"
        )?;
    }
    if self_checks_passed {
        Ok(ExitStatus::Success)
    } else {
        Ok(ExitStatus::FalseOrNull)
    }
}

fn compile_diagnostic_report(
    diagnostic: &str,
    structured: Option<&Diagnostic>,
    filter: &str,
) -> String {
    render_jq_compile_diagnostic(filter, diagnostic, structured)
        .and_then(|rendered| rendered.lines().next().map(str::to_owned))
        .unwrap_or_else(|| diagnostic.to_owned())
}

fn run_test_case(case: &RunTestCase, process_ambient: bool) -> Result<RunTestOutcome, RunError> {
    let arguments = vec![
        "--input-format".to_owned(),
        "json".to_owned(),
        "--output-format".to_owned(),
        "json".to_owned(),
        "--compact-output".to_owned(),
        case.filter.clone(),
    ];
    let mut command = parse_args(arguments)?;
    if process_ambient && let Command::Run(options) = &mut command {
        // `--run-tests` is a process-facing command. Its test programs
        // observe the same ambient environment and platform metadata as
        // an ordinary process invocation; embedded `run_with_io` callers
        // retain deny-by-default policy.
        options.allow_environment = true;
        options.allow_platform = true;
    }
    let input = if case.input.is_empty() {
        b"null\n".to_vec()
    } else {
        format!("{}\n", case.input).into_bytes()
    };
    let mut output = Vec::new();
    let mut error = Vec::new();
    let mut input_reader = input.as_slice();
    let capture = Arc::new(Mutex::new(RunTestCapture::new(case.expected.len())));
    let result = run_with_io_policy_capture(
        command,
        &mut input_reader,
        &mut output,
        &mut error,
        false,
        Some(Arc::clone(&capture)),
    );
    let (status, diagnostic, structured) = match result {
        Ok(status) => (status, String::new(), None),
        Err(RunError::Compile(error)) => {
            let diagnostic = format!("query compilation failed: {error}");
            (ExitStatus::Compile, diagnostic, Some(error))
        }
        Err(error) => (error.status(), error.to_string(), None),
    };
    Ok(RunTestOutcome {
        status,
        diagnostic,
        structured,
        output,
        typed_capture: capture.lock().ok().map(|capture| capture.snapshot()),
    })
}

fn run_tests_self_checks() -> bool {
    let first = run_tests_self_check(".[]", b"[]\n");
    let second = run_tests_self_check(".[] | if .%2 == 0 then halt_error else . end", b"[1,2,3]\n");
    first.0 == ExitStatus::Success
        && first.1.is_empty()
        && second.0 == ExitStatus::Halt(5)
        && second.1 == b"1\n"
}

fn run_tests_self_check(filter: &str, input: &[u8]) -> (ExitStatus, Vec<u8>) {
    let arguments = vec![
        "--input-format".to_owned(),
        "json".to_owned(),
        "--output-format".to_owned(),
        "json".to_owned(),
        "--compact-output".to_owned(),
        filter.to_owned(),
    ];
    let Ok(command) = parse_args(arguments) else {
        return (ExitStatus::Compile, Vec::new());
    };
    let mut input_reader = input;
    let mut output = Vec::new();
    let mut error = Vec::new();
    let status = run_with_io(command, &mut input_reader, &mut output, &mut error)
        .unwrap_or_else(|failure| failure.status());
    (status, output)
}

fn parse_run_test_cases(source: &str) -> (Vec<RunTestCase>, usize) {
    let mut cases = Vec::new();
    let mut malformed = 0_usize;
    let mut block: Vec<(usize, &str)> = Vec::new();
    let mut finish = |block: &mut Vec<(usize, &str)>| {
        if block.is_empty() {
            return;
        }
        let (_, first) = block[0];
        let compile_failure = first == "%%FAIL";
        let filter_offset = usize::from(compile_failure);
        // jq's compile-failure form is `%%FAIL`, filter, expected diagnostic;
        // it deliberately has no input line because compilation stops first.
        let required = if compile_failure {
            filter_offset + 2
        } else {
            2
        };
        if block.len() < required {
            malformed = malformed.saturating_add(1);
        } else {
            cases.push(RunTestCase {
                line: block[filter_offset].0,
                filter: block[filter_offset].1.to_owned(),
                input: if compile_failure {
                    String::new()
                } else {
                    block[1].1.to_owned()
                },
                expected: block[if compile_failure {
                    filter_offset + 1
                } else {
                    2
                }..]
                    .iter()
                    .map(|(_, value)| (*value).to_owned())
                    .collect(),
                compile_failure,
            });
        }
        block.clear();
    };
    for (index, line) in source.lines().enumerate() {
        let line_number = index + 1;
        if line.trim().is_empty() {
            finish(&mut block);
        } else if !line.trim_start().starts_with('#') {
            block.push((line_number, line));
        }
    }
    finish(&mut block);
    (cases, malformed)
}

fn compile_diagnostic_matches(
    diagnostic: &str,
    structured: Option<&Diagnostic>,
    expected: &[String],
    filter: &str,
) -> bool {
    let actual = diagnostic.lines().collect::<Vec<_>>();
    if actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected)
            .all(|(actual, expected)| *actual == expected)
    {
        return true;
    }

    if !expected
        .first()
        .is_some_and(|line| line.starts_with("jq: error: "))
    {
        return false;
    }
    let Some(rendered) = render_jq_compile_diagnostic(filter, diagnostic, structured) else {
        return false;
    };
    rendered.lines().eq(expected.iter().map(String::as_str))
}

struct JqDiagnosticParts<'a> {
    code: &'a str,
    message: &'a str,
    label_start: Option<usize>,
    label_end: Option<usize>,
    if_context: Option<(usize, usize)>,
    if_condition: Option<(usize, usize)>,
    object_value: Option<(usize, usize)>,
    construct_context: Option<(usize, usize)>,
}

fn jq_diagnostic_parts<'a>(
    diagnostic: &'a str,
    structured: Option<&'a Diagnostic>,
) -> Option<JqDiagnosticParts<'a>> {
    if let Some(diagnostic) = structured {
        let primary = diagnostic.labels.iter().find(|label| label.primary);
        let span_for = |message| {
            diagnostic
                .labels
                .iter()
                .find(|label| !label.primary && label.message == message)
                .and_then(label_byte_span)
        };
        Some(JqDiagnosticParts {
            code: diagnostic.code.as_str(),
            message: diagnostic.message.as_str(),
            label_start: primary.and_then(|label| usize::try_from(label.span.start).ok()),
            label_end: primary.and_then(|label| usize::try_from(label.span.end).ok()),
            if_context: span_for("unterminated if expression"),
            if_condition: span_for("if condition"),
            object_value: span_for("object value"),
            construct_context: span_for("try expression")
                .or_else(|| span_for("definition context"))
                .or_else(|| span_for("label context")),
        })
    } else {
        let diagnostic = diagnostic.strip_prefix("query compilation failed: ")?;
        let (code, message) = diagnostic.split_once(": ")?;
        Some(JqDiagnosticParts {
            code,
            message,
            label_start: None,
            label_end: None,
            if_context: None,
            if_condition: None,
            object_value: None,
            construct_context: None,
        })
    }
}

fn render_jq_compile_diagnostic(
    filter: &str,
    diagnostic: &str,
    structured: Option<&Diagnostic>,
) -> Option<String> {
    let parts = jq_diagnostic_parts(diagnostic, structured)?;
    let (summary, needle) = jq_diagnostic_summary(filter, &parts)?;
    let JqDiagnosticParts {
        code,
        message,
        label_start,
        label_end,
        if_context,
        if_condition,
        object_value,
        construct_context,
    } = parts;
    let span = JqDiagnosticSpan {
        filter,
        code,
        message,
        label_start,
        label_end,
        needle: &needle,
        if_context,
        if_condition,
        object_value,
        construct_context,
    };
    let (start, end, byte_width) = jq_diagnostic_span(&span)?;
    let line_start = filter[..start].rfind('\n').map_or(0, |index| index + 1);
    let line_end = filter[start..]
        .find('\n')
        .map_or(filter.len(), |offset| start + offset);
    let line = filter[..line_start]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1;
    let column = filter[line_start..start].chars().count() + 1;
    let width = if byte_width {
        end.min(line_end).saturating_sub(start).max(1)
    } else {
        filter[start..end.min(line_end)].chars().count().max(1)
    };
    let source_line = &filter[line_start..line_end];
    let caret = format!(
        "{}{}",
        " ".repeat(column.saturating_sub(1)),
        "^".repeat(width)
    );
    Some(format!(
        "jq: error: {summary} at <top-level>, line {line}, column {column}:\n    {source_line}\n    {caret}"
    ))
}

fn jq_diagnostic_summary(filter: &str, parts: &JqDiagnosticParts<'_>) -> Option<(String, String)> {
    let JqDiagnosticParts {
        code,
        message,
        label_start,
        label_end,
        if_context,
        if_condition,
        object_value,
        ..
    } = parts;
    let (summary, needle) = if *code == "TQ-RESOLVE-VARIABLE-001" {
        let variable = message.strip_prefix("unknown variable ")?;
        (format!("{variable} is not defined"), variable.to_owned())
    } else if *code == "TQ-RESOLVE-BUILTIN-001" {
        let filter = message.strip_prefix("unknown filter ")?;
        let name = filter.split_once('/').map_or(filter, |(name, _)| name);
        (format!("{filter} is not defined"), name.to_owned())
    } else if *code == "TQ-LEX-STRING-001" && *message == "invalid string escape" {
        let slash = filter.find('\\').unwrap_or(0);
        let column = filter[..slash].chars().count() + 3;
        (
            format!("Invalid escape at line 1, column {column} (while parsing '{filter}')"),
            String::new(),
        )
    } else if object_value.is_some() {
        let (start, end) = object_value.expect("object context was checked");
        (
            jq_unexpected_literal_summary(filter, start, Some(end)),
            String::new(),
        )
    } else if *code == "TQ-PARSE-EXPRESSION-001" {
        if if_context.is_some() {
            (
                "Possibly unterminated 'if' statement".to_owned(),
                String::new(),
            )
        } else {
            match (*label_start).filter(|start| *start < filter.len()) {
                Some(start) => (
                    jq_unexpected_token_summary(filter, start, *label_end),
                    String::new(),
                ),
                None => (
                    "syntax error, unexpected end of file".to_owned(),
                    String::new(),
                ),
            }
        }
    } else if *code == "TQ-PARSE-UNEXPECTED-001"
        && *message == "expected 'then'"
        && (*label_start).is_some_and(|start| start < filter.len())
    {
        let start = label_start.expect("token label was checked");
        (
            format!(
                "syntax error, unexpected {}, expecting then or '|' or ','",
                jq_unexpected_word_summary(filter, start, *label_end)
            ),
            String::new(),
        )
    } else if *code == "TQ-PARSE-UNEXPECTED-001"
        && (*label_start).is_none_or(|start| start >= filter.len())
    {
        if if_context.is_some() && *message != "expected 'then'" {
            (
                "Possibly unterminated 'if' statement".to_owned(),
                String::new(),
            )
        } else if *message == "expected 'then'" && if_condition.is_some() {
            (
                "syntax error, unexpected end of file, expecting then or '|' or ','".to_owned(),
                String::new(),
            )
        } else {
            (jq_unexpected_eof_summary(message)?, String::new())
        }
    } else {
        jq_simple_diagnostic_summary(filter, code, message, *label_start, *label_end)?
    };
    Some((summary, needle))
}

fn jq_simple_diagnostic_summary(
    filter: &str,
    code: &str,
    message: &str,
    label_start: Option<usize>,
    label_end: Option<usize>,
) -> Option<(String, String)> {
    if code == "TQ-LEX-FORMAT-001" {
        return Some((
            format!(
                "syntax error, unexpected INVALID_CHARACTER{}",
                if label_start == Some(0) {
                    ", expecting end of file"
                } else {
                    ""
                }
            ),
            String::new(),
        ));
    }
    if code == "TQ-LEX-STRING-001" && message == "unterminated string" {
        return Some((
            "syntax error, unexpected end of file, expecting QQSTRING_TEXT or QQSTRING_INTERP_START or QQSTRING_END".to_owned(),
            String::new(),
        ));
    }
    if let Some(start) = label_start.filter(|start| *start < filter.len()) {
        let expected = match code {
            "TQ-PARSE-DEF-PARAMETER-001" => Some("IDENT or BINDING"),
            "TQ-PARSE-DEF-001" => Some("IDENT"),
            "TQ-PARSE-LABEL-001" => Some("BINDING"),
            _ => None,
        };
        if let Some(expected) = expected {
            return Some((
                format!(
                    "syntax error, unexpected {}, expecting {expected}",
                    jq_parser_token_name(filter, start, label_end)
                ),
                String::new(),
            ));
        }
    }
    if code == "TQ-LEX-VARIABLE-001" {
        return Some((
            "syntax error, unexpected end of file, expecting '$'".to_owned(),
            String::new(),
        ));
    }
    if label_start.is_none_or(|start| start >= filter.len()) {
        let summary = match code {
            "TQ-PARSE-OBJECT-001" => "syntax error, unexpected end of file",
            "TQ-PARSE-DEF-PARAMETER-001" => {
                "syntax error, unexpected end of file, expecting IDENT or BINDING"
            }
            "TQ-PARSE-DEF-001" => "syntax error, unexpected end of file, expecting IDENT",
            "TQ-PARSE-LABEL-001" => "syntax error, unexpected end of file, expecting BINDING",
            _ => return None,
        };
        return Some((summary.to_owned(), String::new()));
    }
    None
}

#[derive(Clone, Copy)]
struct JqDiagnosticSpan<'a> {
    filter: &'a str,
    code: &'a str,
    message: &'a str,
    label_start: Option<usize>,
    label_end: Option<usize>,
    needle: &'a str,
    if_context: Option<(usize, usize)>,
    if_condition: Option<(usize, usize)>,
    object_value: Option<(usize, usize)>,
    construct_context: Option<(usize, usize)>,
}

fn jq_diagnostic_span(span: &JqDiagnosticSpan<'_>) -> Option<(usize, usize, bool)> {
    let JqDiagnosticSpan {
        filter,
        code,
        message,
        label_start,
        label_end,
        needle,
        if_context,
        if_condition,
        object_value,
        construct_context,
    } = *span;
    let byte_width = code == "TQ-LEX-STRING-001"
        && matches!(message, "unterminated string" | "invalid string escape");
    let span_override = if if_context.is_some()
        && !(code == "TQ-PARSE-UNEXPECTED-001" && message == "expected 'then'")
    {
        if_context
    } else if code == "TQ-PARSE-UNEXPECTED-001"
        && message == "expected 'then'"
        && if_condition.is_some()
        && label_start.is_none_or(|start| start >= filter.len())
    {
        if_condition
    } else if object_value.is_some() {
        object_value
    } else if construct_context.is_some() && label_start.is_none_or(|start| start >= filter.len()) {
        construct_context
    } else if byte_width {
        let start = label_start.unwrap_or(0).min(filter.len());
        let content_start = if filter.as_bytes().get(start) == Some(&b'"') {
            start.saturating_add(1)
        } else {
            start
        };
        if message == "invalid string escape" {
            let slash = filter[content_start..]
                .find('\\')
                .map_or(content_start, |offset| content_start + offset);
            Some((slash, slash.saturating_add(2).min(filter.len())))
        } else {
            Some((
                content_start,
                label_end
                    .unwrap_or(filter.len())
                    .min(filter.len())
                    .max(content_start),
            ))
        }
    } else {
        None
    };
    let (start, end) = if let Some((start, end)) = span_override {
        (start, end)
    } else if needle.is_empty() {
        let start = label_start
            .filter(|start| *start < filter.len())
            .unwrap_or_else(|| jq_eof_token_span(filter).0);
        (
            start,
            label_end.unwrap_or_else(|| jq_eof_token_span(filter).1),
        )
    } else {
        let hint = label_start.unwrap_or(0).min(filter.len());
        let start = filter[hint..]
            .find(needle)
            .map_or_else(|| filter.find(needle), |offset| Some(hint + offset))?;
        (start, start.saturating_add(needle.len()))
    };
    Some((start, end, byte_width))
}

fn label_byte_span(label: &Label) -> Option<(usize, usize)> {
    Some((
        usize::try_from(label.span.start).ok()?,
        usize::try_from(label.span.end).ok()?,
    ))
}

fn jq_unexpected_token_summary(filter: &str, start: usize, end: Option<usize>) -> String {
    let end = end
        .unwrap_or_else(|| start.saturating_add(1))
        .min(filter.len());
    let token = filter[start..end].chars().next().unwrap_or('\0');
    let token = match token {
        ']' | '}' | ')' => "INVALID_CHARACTER".to_owned(),
        character => format!("'{character}'"),
    };
    let expectation = if filter[..start].trim().is_empty() {
        ", expecting end of file"
    } else {
        ""
    };
    format!("syntax error, unexpected {token}{expectation}")
}

fn jq_unexpected_literal_summary(filter: &str, start: usize, end: Option<usize>) -> String {
    let end = end
        .unwrap_or_else(|| start.saturating_add(1))
        .min(filter.len());
    let token = filter[start..end].chars().next().unwrap_or('\0');
    format!("syntax error, unexpected '{token}'")
}

fn jq_unexpected_word_summary(filter: &str, start: usize, end: Option<usize>) -> String {
    let end = end
        .unwrap_or_else(|| start.saturating_add(1))
        .min(filter.len());
    let token = &filter[start..end];
    if token.chars().all(char::is_alphanumeric) {
        token.to_owned()
    } else {
        jq_unexpected_literal_summary(filter, start, Some(end))
            .trim_start_matches("syntax error, unexpected ")
            .to_owned()
    }
}

fn jq_parser_token_name(filter: &str, start: usize, end: Option<usize>) -> String {
    let end = end
        .unwrap_or_else(|| start.saturating_add(1))
        .min(filter.len());
    let token = &filter[start..end];
    if !token.is_empty()
        && token.chars().all(|character| {
            character.is_ascii_digit() || matches!(character, '.' | '-' | '+' | 'e' | 'E')
        })
    {
        "LITERAL".to_owned()
    } else if token.chars().all(char::is_alphanumeric) {
        "IDENT".to_owned()
    } else {
        jq_unexpected_word_summary(filter, start, Some(end))
    }
}

fn jq_eof_token_span(filter: &str) -> (usize, usize) {
    let Some((last_start, last)) = filter.char_indices().next_back() else {
        return (0, 1);
    };
    if !(last.is_alphanumeric() || matches!(last, '_' | '$')) {
        return (last_start, last_start + last.len_utf8());
    }
    let mut start = last_start;
    for (index, character) in filter[..last_start].char_indices().rev() {
        if character.is_alphanumeric() || matches!(character, '_' | '$') {
            start = index;
        } else {
            break;
        }
    }
    (start, filter.len())
}

fn jq_unexpected_eof_summary(message: &str) -> Option<String> {
    let expectation = match message {
        "expected ']' after array constructor" => "'|' or ',' or ']'".to_owned(),
        "expected '}' after object constructor" => "'}'".to_owned(),
        "expected ')' after grouped expression" | "expected ')' after computed object key" => {
            "'|' or ',' or ')'".to_owned()
        }
        "expected ')' after function arguments" => "';' or ')'".to_owned(),
        "expected ')' after string interpolation" => "QQSTRING_INTERP_END or '|' or ','".to_owned(),
        "expected 'then'" => "then or '|' or ','".to_owned(),
        "expected '|' after label variable" => "'|'".to_owned(),
        message => message.strip_prefix("expected ")?.to_owned(),
    };
    Some(format!(
        "syntax error, unexpected end of file, expecting {expectation}"
    ))
}

fn run_test_output_matches(
    actual: &[u8],
    expected: &[String],
    typed_capture: Option<&RunTestCaptureSnapshot>,
) -> bool {
    let Some(capture) = typed_capture else {
        return false;
    };
    if capture.overflowed {
        return false;
    }
    let actual_lines = actual
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if actual_lines.len() != expected.len() {
        return false;
    }
    let parsed_actual = actual_lines
        .iter()
        .map(|line| parse_json_bytes(line, JsonInputOptions::default()))
        .collect::<Result<Vec<_>, _>>();
    let parsed_expected = expected
        .iter()
        .map(|line| parse_json_bytes(line.as_bytes(), JsonInputOptions::default()))
        .collect::<Result<Vec<_>, _>>();
    let expected_is_json = parsed_expected.is_ok();
    match (parsed_actual, parsed_expected) {
        (Ok(actual), Ok(expected)) => {
            if capture.values != expected {
                return false;
            }
            actual == expected
        }
        _ if expected_is_json => false,
        _ => actual_lines
            .iter()
            .zip(expected)
            .all(|(actual, expected)| actual == &expected.as_bytes()),
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
    capture: Option<RunTestCaptureHandle>,
) -> Result<ExitStatus, RunError> {
    validate_capability_policy(options)?;
    validate_stream_input_formats(options)?;
    let (query_name, query, startup) = load_filter(options)?;
    let variables = parse_external_arguments(options)?;
    let resolve_options = ResolveOptions {
        variables: variables
            .keys()
            .filter(|name| !name.starts_with("__tq_"))
            .cloned()
            .collect::<BTreeSet<_>>(),
        module_roots: module_roots(options, &query_name),
        module_limit: options.limits.depth,
        module_bytes: usize::try_from(options.limits.input_bytes)
            .unwrap_or(usize::MAX)
            .min(ResolveOptions::default().module_bytes),
    };
    let parsed = startup
        .map_or_else(
            || parse_bytes(&query_name, &query),
            |(startup_name, startup)| {
                parse_with_startup(&query_name, &query, &startup_name, &startup)
            },
        )
        .map_err(RunError::Compile)?;
    let resolved = resolve(parsed, &resolve_options).map_err(RunError::Compile)?;
    let query_capabilities = analyze_with_context(
        resolved.clone(),
        AnalysisContext {
            automatic_streaming: true,
            ..AnalysisContext::default()
        },
    )
    .capabilities();
    let automatic_mode = !options.stream
        && !options.slurp
        && !options.raw_input
        && !options.null_input
        && !query_capabilities.whole_input
        && !json_sequence_input_requested(options);
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
                    capture.clone(),
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
                capture.clone(),
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
                capture.clone(),
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
            capture,
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
            InputFormat::Json | InputFormat::Toon if !json_sequence_input_requested(options) => {
                Some(options.input_format)
            }
            _ => None,
        },
        &[],
        stdin,
        stdout,
        stderr,
        capture,
    )
}

fn validate_stream_input_formats(options: &RunOptions) -> Result<(), RunError> {
    if !options.stream {
        return Ok(());
    }
    let json5_selected = options.input_format == InputFormat::Json5
        || (options.input_format == InputFormat::Auto
            && options.files.iter().any(|path| {
                path != Path::new("-") && format_from_path(path) == Some(InputFormat::Json5)
            }));
    if json5_selected {
        return Err(RunError::Unsupported(JSON5_STREAM_UNSUPPORTED.to_owned()));
    }
    Ok(())
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
    match tq_formats::NativeFormat::from_input(format) {
        // Recoverable RS payloads need complete-document validation in normal
        // mode. Explicit --stream can publish partial events before recovery.
        Some(native) => {
            native.descriptor().events
                && !matches!(
                    native.descriptor().framing,
                    tq_formats::Framing::RecordSeparator
                )
        }
        None => false,
    }
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
    capture: Option<RunTestCaptureHandle>,
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

    if options.stream && analysis.selected_plan == PlanKind::Events {
        let plan = program.event_plan().map_err(RunError::Compile)?;
        return run_event_filter(
            options,
            &plan,
            variables,
            &analysis,
            stdin,
            stdout,
            stderr,
            capture.clone(),
        );
    }
    if matches!(
        analysis.selected_plan,
        PlanKind::Events | PlanKind::Subtree | PlanKind::HybridBlocking
    ) {
        return match program.automatic_plan().map_err(RunError::Compile)? {
            AutomaticPlan::Events(plan) => run_automatic_filter(
                options,
                &plan,
                variables,
                &analysis,
                stdin,
                stdout,
                stderr,
                capture.clone(),
            ),
            AutomaticPlan::Subtree(plan) => run_automatic_filter(
                options,
                &plan,
                variables,
                &analysis,
                stdin,
                stdout,
                stderr,
                capture.clone(),
            ),
            AutomaticPlan::HybridBlocking(plan) => run_hybrid_filter(
                options,
                &plan,
                variables,
                &analysis,
                stdin,
                stdout,
                stderr,
                capture.clone(),
            ),
            _ => unreachable!("automatic selection returns an automatic typed plan"),
        };
    }
    let plan = program.document_plan();

    let mut result_output = ResultOutput::with_capture(stdout, options, capture);
    let mut result_count = 0_usize;
    let mut last = None;
    let mut observations = Vec::new();
    let mut runtime_error = None;
    let mut runtime_error_reported = false;
    let mut deferred_input_diagnostics = Vec::new();
    let continue_document_roots = !options.null_input && !options.slurp;
    {
        let mut evaluate = |input, input_cursor: Option<InputCursor>| -> Result<bool, RunError> {
            // A runtime error belongs to the current input root.  A later
            // successful root clears it, while a recoverable error is
            // reported immediately and lets the source continue.
            runtime_error = None;
            let (input, input_cursor) = match input {
                StructuredInput::Value(input) => (input, input_cursor),
                StructuredInput::Document(document) => {
                    let cursor = InputCursor::from_input_values(vec![document.clone()]);
                    let _ = cursor.next_value()?;
                    (document.value, Some(cursor))
                }
                StructuredInput::Proxy(bytes) => {
                    result_output.proxy(&bytes)?;
                    return Ok(true);
                }
                StructuredInput::Failure(failure) => {
                    render_native_failure(stderr, &failure, options.json_sequence)?;
                    return Ok(true);
                }
                StructuredInput::Warning { message, context } => {
                    render_recovery_warning(stderr, &context, &message)?;
                    return Ok(true);
                }
            };
            runtime_error = None;
            runtime_error_reported = false;
            let mut vm =
                Vm::new_with_variables(&plan, input, vm_limits(options), variables.clone())
                    .with_trace_limit(options.trace_limit);
            if let Some(cursor) = input_cursor {
                vm = vm.with_input_cursor(cursor);
            }
            if options.unbuffered {
                let worker_stop =
                    cancellation().unwrap_or_else(|| Arc::new(AtomicBool::new(false)));
                vm = vm.with_cancellation(Arc::clone(&worker_stop));
                let sink = vm.effect_sink();
                vm = vm.with_effect_acknowledgements();
                let (sender, receiver) = sync_channel(0);
                let (vm_result, vm_observations, vm_trace) = thread::scope(|scope| {
                    scope.spawn(move || {
                        let result = loop {
                            match vm.next_result() {
                                Ok(Some(value)) => {
                                    let (ready, acknowledged) = sync_channel(0);
                                    if sender
                                        .send(LiveVmMessage::Value {
                                            value,
                                            acknowledged: ready,
                                        })
                                        .is_err()
                                    {
                                        return;
                                    }
                                    if acknowledged.recv().is_err() {
                                        return;
                                    }
                                }
                                Ok(None) => break Ok(()),
                                Err(error) => break Err(error),
                            }
                        };
                        let _ = sender.send(LiveVmMessage::Complete {
                            result,
                            observations: vm.observations(),
                            trace: vm.trace().to_vec(),
                        });
                    });
                    let mut worker_result = Ok(());
                    let mut vm_observations = VmObservations::default();
                    let mut vm_trace = Vec::new();
                    let mut complete = false;
                    while !complete {
                        if let Err(error) = flush_effect_sink(&sink, stderr) {
                            worker_stop.store(true, Ordering::Relaxed);
                            sink.cancel();
                            drop(receiver);
                            return Err(error);
                        }
                        match receiver.recv_timeout(Duration::from_millis(5)) {
                            Ok(LiveVmMessage::Value {
                                value,
                                acknowledged,
                            }) => {
                                last = Some(value.clone());
                                if let Err(error) = result_output.emit(&value) {
                                    worker_stop.store(true, Ordering::Relaxed);
                                    sink.cancel();
                                    drop(receiver);
                                    return Err(error);
                                }
                                result_count = result_count.saturating_add(1);
                                if acknowledged.send(()).is_err() {
                                    worker_stop.store(true, Ordering::Relaxed);
                                    sink.cancel();
                                    drop(receiver);
                                    return Err(RunError::Interrupted);
                                }
                            }
                            Ok(LiveVmMessage::Complete {
                                result: completed,
                                observations,
                                trace,
                            }) => {
                                worker_result = completed;
                                vm_observations = observations;
                                vm_trace = trace;
                                complete = true;
                            }
                            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                                complete = true;
                            }
                        }
                    }
                    if let Err(error) = flush_effect_sink(&sink, stderr) {
                        worker_stop.store(true, Ordering::Relaxed);
                        sink.cancel();
                        drop(receiver);
                        return Err(error);
                    }
                    Ok::<_, RunError>((worker_result, vm_observations, vm_trace))
                })?;
                if options.trace_limit != 0 {
                    for entry in vm_trace {
                        writeln!(stderr, "trace: {entry}")?;
                    }
                }
                runtime_error = vm_result.err();
                if let Some(error) = runtime_error.as_ref()
                    && continue_document_roots
                    && is_recoverable_document_runtime_error(error)
                {
                    report_document_runtime_error(stderr, error)?;
                    runtime_error_reported = true;
                }
                record_document_observations(options, &mut observations, vm_observations);
            } else {
                if let Some(flag) = cancellation() {
                    vm = vm.with_cancellation(flag);
                }
                loop {
                    let next = vm.next_result();
                    flush_vm_effects(&vm, stderr)?;
                    match next {
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
                if let Some(error) = runtime_error.as_ref()
                    && continue_document_roots
                    && is_recoverable_document_runtime_error(error)
                {
                    report_document_runtime_error(stderr, error)?;
                    runtime_error_reported = true;
                }
                record_document_observations(options, &mut observations, vm.observations());
            }
            Ok(runtime_error.as_ref().is_none_or(|error| {
                continue_document_roots && is_recoverable_document_runtime_error(error)
            }))
        };

        if options.null_input && !analysis.capabilities.whole_input {
            let _ = evaluate(StructuredInput::Value(Value::Null), None)?;
        } else if options.slurp && !options.raw_input {
            let mut values = Vec::new();
            let mut had_proxy = false;
            let mut source_options = options.clone();
            source_options.null_input = false;
            for_each_structured_input(
                &source_options,
                stdin,
                &mut deferred_input_diagnostics,
                &mut |input| match input {
                    StructuredInput::Value(value) => {
                        values.push(value);
                        Ok(true)
                    }
                    StructuredInput::Document(document) => {
                        values.push(document.value);
                        Ok(true)
                    }
                    input @ StructuredInput::Proxy(_) => {
                        had_proxy = true;
                        evaluate(input, None)
                    }
                    input => evaluate(input, None),
                },
            )?;
            if !had_proxy || !values.is_empty() {
                let cursor = InputCursor::new(vec![Value::array(values)]);
                let input = if options.null_input {
                    Value::Null
                } else {
                    cursor.next_value()?.expect("slurp supplies one input")
                };
                let _ = evaluate(StructuredInput::Value(input), Some(cursor))?;
            }
        } else if analysis.capabilities.whole_input
            && !options.slurp
            && !options.raw_input
            && (!options.proxy_on_error || options.stream)
        {
            thread::scope(|scope| {
                let (request_sender, request_receiver) = sync_channel(0);
                let (sender, receiver) = sync_channel(0);
                scope.spawn(move || {
                    produce_remaining_inputs(options, stdin, &request_receiver, &sender);
                });
                let cursor = InputCursor::from_provider(move || {
                    if request_sender.send(()).is_err() {
                        return Ok(None);
                    }
                    match receiver.recv() {
                        Ok(RemainingInputMessage::Value(value)) => Ok(Some(value)),
                        Ok(RemainingInputMessage::Failure(failure)) => {
                            Err(VmError::RecoverableInput {
                                context: format!(
                                    "in '{}' at recovery segment {}",
                                    failure.identity, failure.recovery_segment_index
                                )
                                .into(),
                                message: failure.message.into(),
                            })
                        }
                        Ok(RemainingInputMessage::Error(error)) => Err(error),
                        Ok(RemainingInputMessage::Done) | Err(_) => Ok(None),
                    }
                });
                let result: Result<(), RunError> = (|| {
                    if options.null_input {
                        let _ =
                            evaluate(StructuredInput::Value(Value::Null), Some(cursor.clone()))?;
                    } else {
                        while let Some(input) = pull_remaining_input(&cursor)? {
                            if !evaluate(input, Some(cursor.clone()))? {
                                break;
                            }
                        }
                    }
                    Ok(())
                })();
                drop(cursor);
                result
            })?;
        } else if analysis.capabilities.whole_input && !options.slurp && !options.raw_input {
            let remaining_options = if options.null_input {
                let mut remaining = options.clone();
                remaining.null_input = false;
                remaining
            } else {
                options.clone()
            };
            match load_inputs(&remaining_options, stdin, false, &mut Vec::new())? {
                LoadedInputs::Proxy(bytes) => {
                    let _ = if options.null_input {
                        evaluate(StructuredInput::Value(Value::Null), None)?
                    } else {
                        evaluate(StructuredInput::Proxy(bytes), None)?
                    };
                }
                LoadedInputs::Documents(inputs) => {
                    let cursor = InputCursor::from_input_values(
                        inputs
                            .into_iter()
                            .map(|document| InputValue {
                                value: document.value,
                                identity: Arc::from(document.identity),
                                line_number: document.line_number,
                            })
                            .collect(),
                    );
                    if options.null_input {
                        let _ = evaluate(StructuredInput::Value(Value::Null), Some(cursor))?;
                    } else {
                        while let Some(input) = cursor.next_value()? {
                            if !evaluate(StructuredInput::Value(input), Some(cursor.clone()))? {
                                break;
                            }
                        }
                    }
                }
            }
        } else if options.slurp || options.raw_input {
            let remaining_options = if options.null_input {
                let mut remaining = options.clone();
                remaining.null_input = false;
                remaining
            } else {
                options.clone()
            };
            let mut input_diagnostics = Vec::new();
            match load_inputs(
                &remaining_options,
                stdin,
                options.slurp
                    && selected_input_format(options, Path::new("-")) == InputFormat::JsonSequence,
                &mut input_diagnostics,
            )? {
                LoadedInputs::Proxy(bytes) => {
                    let _ = if options.null_input {
                        evaluate(StructuredInput::Value(Value::Null), None)?
                    } else {
                        evaluate(StructuredInput::Proxy(bytes), None)?
                    };
                }
                LoadedInputs::Documents(inputs) => {
                    let input_values = if options.slurp && !options.raw_input {
                        let (identity, line_number) = inputs.last().map_or_else(
                            || ("<stdin>".to_owned(), 0),
                            |document| (document.identity.clone(), document.line_number),
                        );
                        vec![InputValue {
                            value: Value::array(
                                inputs
                                    .into_iter()
                                    .map(|document| document.value)
                                    .collect::<Vec<_>>(),
                            ),
                            identity: Arc::from(identity),
                            line_number,
                        }]
                    } else {
                        inputs
                            .into_iter()
                            .map(|document| InputValue {
                                value: document.value,
                                identity: Arc::from(document.identity),
                                line_number: document.line_number,
                            })
                            .collect()
                    };
                    let cursor = InputCursor::from_input_values(input_values);
                    if options.null_input {
                        let _ = evaluate(StructuredInput::Value(Value::Null), Some(cursor))?;
                    } else {
                        while let Some(input) = cursor.next_value()? {
                            if !evaluate(StructuredInput::Value(input), Some(cursor.clone()))? {
                                break;
                            }
                        }
                    }
                }
            }
            deferred_input_diagnostics.extend(input_diagnostics);
        } else {
            let mut sequence_diagnostics = Vec::new();
            for_each_structured_input(options, stdin, &mut sequence_diagnostics, &mut |input| {
                evaluate(input, None)
            })?;
            deferred_input_diagnostics.extend(sequence_diagnostics);
        }
        for diagnostic in deferred_input_diagnostics {
            writeln!(stderr, "tq: {}", process_input_diagnostic(&diagnostic))?;
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
        if runtime_error_reported {
            return Ok(ExitStatus::Runtime);
        }
        match error {
            VmError::Input { message } => {
                return Err(RunError::Input(FormatError::Parse {
                    format: InputFormat::Auto,
                    message: message.to_string(),
                }));
            }
            VmError::Halt { status, stderr }
                if options.null_input && stderr.as_ref() == b"null\n" =>
            {
                return Err(RunError::Runtime(VmError::Halt {
                    status,
                    stderr: Arc::from([] as [u8; 0]),
                }));
            }
            error if continue_document_roots && is_recoverable_document_runtime_error(&error) => {
                return Err(RunError::ReportedRuntime(error));
            }
            error => return Err(RunError::Runtime(error)),
        }
    }
    Ok(result_output.exit_status(options.exit_status, last.as_ref()))
}

enum StructuredInput {
    Value(Value),
    Document(InputValue),
    Proxy(Vec<u8>),
    Failure(tq_formats::NativeInputFailure),
    Warning {
        message: Arc<str>,
        context: Arc<str>,
    },
}

fn render_native_failure(
    writer: &mut impl Write,
    failure: &tq_formats::NativeInputFailure,
    jq_sequence: bool,
) -> io::Result<()> {
    if jq_sequence {
        return writeln!(writer, "tq: ignoring parse error: {}", failure.message);
    }
    render_recovery_warning(
        writer,
        &format!(
            "in '{}' at recovery segment {}",
            failure.identity, failure.recovery_segment_index
        ),
        &failure.message,
    )
}

fn render_recovery_warning(
    writer: &mut impl Write,
    context: &str,
    message: &str,
) -> io::Result<()> {
    writeln!(writer, "tq: ignoring parse error {context}: {message}")
}

fn json_sequence_input_requested(options: &RunOptions) -> bool {
    if !options.json_sequence {
        return false;
    }
    match options.input_format {
        InputFormat::Json | InputFormat::JsonSequence => true,
        InputFormat::Auto => {
            matches!(
                options.output_format,
                OutputFormat::Json | OutputFormat::JsonSequence | OutputFormat::JsonLines
            )
        }
        InputFormat::Toon
        | InputFormat::Yaml
        | InputFormat::Json5
        | InputFormat::ToonSequence
        | InputFormat::JsonLines
        | InputFormat::Csv
        | InputFormat::Tsv => false,
    }
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
            ToonFraming::Values => TranscodeCommitment::DirectValues,
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
        TranscodeCommitment::DirectValues => tq_toon::TranscodeCommitment::DirectValues,
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
        TranscodeCommitment::DirectSequence | TranscodeCommitment::DirectValues => {
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
        ExitStatus::Runtime | ExitStatus::Unsupported | ExitStatus::Halt(_) => "output-error",
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
    NativeFormat::from_input(format)
        .expect("typed transcode proof has a committed format")
        .select_input(decode_options(options, format), InputRepresentation::Events)?
        .open(reader, identity)
        .consume_codec_events(source, consumer)
        .map_err(|error| match error {
            tq_formats::InputDeliveryError::Input(error) => error.into(),
            tq_formats::InputDeliveryError::Consumer(tq_formats::CodecConsumerError::Event(
                error,
            )) => map_transcode_error(error),
            tq_formats::InputDeliveryError::Consumer(tq_formats::CodecConsumerError::Text(
                message,
            )) => map_transcode_message(format, message),
        })
}

fn map_transcode_message(format: InputFormat, message: String) -> RunError {
    if message.contains("prepared output exceeds configured byte limit") {
        RunError::Resource("output-bytes")
    } else if message.contains("resource limit")
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
            if let Some(mapped) = map_wrapped_spool_error(&error) {
                return mapped;
            }
            let message = error.to_string();
            if message.contains("output resource limit exceeded") {
                RunError::Resource("output-bytes")
            } else if message.contains("spool")
                || message.contains("resource limit")
                || message.contains("preparation")
            {
                RunError::Resource("transcode-preparation")
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

fn map_wrapped_spool_error(error: &io::Error) -> Option<RunError> {
    let source = error.get_ref()?.downcast_ref::<SpoolError>()?;
    Some(match source {
        SpoolError::OutputLimit => RunError::Resource("output-bytes"),
        SpoolError::Cancelled => RunError::Interrupted,
        SpoolError::MemoryLimit
        | SpoolError::Disabled
        | SpoolError::Limit
        | SpoolError::NestingLimit => RunError::Resource("transcode-preparation"),
        SpoolError::Io(_) | SpoolError::Decode(_) => return None,
    })
}

fn map_publication_error(error: PublicationError) -> RunError {
    match error {
        PublicationError::Cardinality(_) => RunError::Cardinality(
            "unframed TOON requires exactly one result; omit --unframed for zero or multiple results",
        ),
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
    diagnostics: &mut Vec<String>,
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
            for_each_structured_reader(options, format, &mut *stdin, "<stdin>", diagnostics, emit)?
        } else {
            let identity = path.display().to_string();
            for_each_structured_reader(
                options,
                format,
                open_path(&path)?,
                &identity,
                diagnostics,
                emit,
            )?
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
    diagnostics: &mut Vec<String>,
    emit: &mut F,
) -> Result<bool, RunError>
where
    F: FnMut(StructuredInput) -> Result<bool, RunError>,
{
    if format == InputFormat::Auto {
        if options.proxy_on_error {
            let bytes = read_limited(reader, options.limits.input_bytes, identity)?;
            return match decode_bytes(&bytes, identity, decode_options(options, format)) {
                Ok(documents) => {
                    if options.stream {
                        let selected = documents.first().map_or_else(
                            || {
                                probe_format(&bytes, options.limits.lookahead_bytes)
                                    .map(|report| report.selected)
                            },
                            |document| Ok(document.format),
                        )?;
                        if NativeFormat::from_input(selected)
                            .is_some_and(|native| native.descriptor().events)
                        {
                            return project_committed_input(
                                options,
                                selected,
                                bytes.as_slice(),
                                identity,
                                emit,
                            );
                        }
                    }
                    for document in documents {
                        if !emit(StructuredInput::Document(InputValue {
                            value: document.value,
                            identity: Arc::from(document.identity),
                            line_number: document.line_number,
                        }))? {
                            return Ok(false);
                        }
                    }
                    Ok(true)
                }
                Err(error) if proxyable_format_error(&error) => emit(StructuredInput::Proxy(bytes)),
                Err(error) => Err(error.into()),
            };
        }
        let reader = LimitedReader::new(reader, options.limits.input_bytes, identity);
        let (report, replay) = probe_reader(reader, options.limits.lookahead_bytes)?;
        return for_each_detected_structured_reader(
            options,
            report.selected,
            replay,
            identity,
            diagnostics,
            emit,
        );
    }
    for_each_detected_structured_reader(options, format, reader, identity, diagnostics, emit)
}

fn for_each_detected_structured_reader<R: Read, F>(
    options: &RunOptions,
    format: InputFormat,
    reader: R,
    identity: &str,
    _diagnostics: &mut Vec<String>,
    emit: &mut F,
) -> Result<bool, RunError>
where
    F: FnMut(StructuredInput) -> Result<bool, RunError>,
{
    if options.stream
        && !options.proxy_on_error
        && NativeFormat::from_input(format).is_some_and(|native| native.descriptor().events)
    {
        return project_committed_input(options, format, reader, identity, emit);
    }
    if options.proxy_on_error {
        let bytes = read_limited(reader, options.limits.input_bytes, identity)?;
        let (committed, documents) =
            match native_source_documents(&bytes, identity, options, format) {
                Ok(source) => source,
                Err(error) if proxyable_format_error(&error) => {
                    return emit(StructuredInput::Proxy(bytes));
                }
                Err(error) => return Err(error.into()),
            };
        if options.stream {
            return project_committed_input(options, committed, bytes.as_slice(), identity, emit);
        }
        for document in documents {
            if !emit(StructuredInput::Document(InputValue {
                value: document.value,
                identity: Arc::from(document.identity),
                line_number: document.line_number,
            }))? {
                return Ok(false);
            }
        }
        return Ok(true);
    }
    if format == InputFormat::Json {
        let reader = LimitedReader::new(reader, options.limits.input_bytes, identity);
        let mut source = JsonDocumentSource::with_options(
            BufReader::with_capacity(INPUT_BUFFER_BYTES, reader),
            identity,
            json_input_options(options),
        );
        while let Some(document) = source.next_document()? {
            if !emit(StructuredInput::Document(InputValue {
                value: document.value,
                identity: Arc::from(document.identity),
                line_number: document.line_number,
            }))? {
                return Ok(false);
            }
        }
        return Ok(true);
    }
    if format == InputFormat::Auto {
        let (probe, replay) = probe_reader(reader, options.limits.lookahead_bytes)?;
        return for_each_committed_document(options, probe.selected, replay, identity, emit);
    }
    if format == InputFormat::JsonLines {
        let reader = LimitedReader::new(reader, options.limits.input_bytes, identity);
        let mut source = JsonLinesDocumentSource::new(
            BufReader::new(reader),
            identity,
            decode_options(options, format),
        );
        while let Some(document) = source.next_document()? {
            if !emit(StructuredInput::Document(InputValue {
                value: document.value,
                identity: Arc::from(document.identity),
                line_number: document.line_number,
            }))? {
                return Ok(false);
            }
        }
        return Ok(true);
    }
    for_each_committed_document(options, format, reader, identity, emit)
}

fn for_each_committed_document<R: Read, F>(
    options: &RunOptions,
    format: InputFormat,
    reader: R,
    identity: &str,
    emit: &mut F,
) -> Result<bool, RunError>
where
    F: FnMut(StructuredInput) -> Result<bool, RunError>,
{
    if options.stream {
        return project_committed_input(options, format, reader, identity, emit);
    }
    let selection = NativeFormat::from_input(format)
        .expect("input is committed")
        .select_input(
            decode_options(options, format),
            InputRepresentation::Documents,
        )?;
    let mut source = selection.open(reader, identity);
    while let Some(observation) = source.next_observation()? {
        let input = match observation {
            NativeInputObservation::Document(document) => StructuredInput::Document(InputValue {
                value: document.value,
                identity: Arc::from(document.identity),
                line_number: document.line_number,
            }),
            NativeInputObservation::Failure(failure) => StructuredInput::Failure(failure),
            NativeInputObservation::Event(_) => unreachable!("Document representation"),
        };
        if !emit(input)? {
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
            "input_projection": if options.stream { "jq-stream" } else { "documents" },
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
            "frame_bytes": options.limits.frame_bytes,
            "fields": options.limits.fields,
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
    // Automatic JSON roots are transactional: selected values and effects are
    // staged until the decoder commits the containing root.  The current
    // executor therefore cannot safely use the old cross-root parallel
    // collector even when the static prefix would otherwise qualify.
    (false, "root-lifecycle-requires-serial-commit")
}

fn input_format_name(format: InputFormat) -> String {
    match NativeFormat::from_input(format) {
        None => "auto:toon/json/yaml-bounded-probe".to_owned(),
        Some(native) => format!("override:{}", native.report_name()),
    }
}

const fn concrete_input_format_name(format: InputFormat) -> &'static str {
    match NativeFormat::from_input(format) {
        None => "auto",
        Some(native) => native.report_name(),
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "the event route keeps input, output, diagnostics, analysis, and test capture explicit"
)]
fn run_event_filter<R: Read, W: Write, E: Write>(
    options: &RunOptions,
    plan: &Plan<Compiled, Events>,
    variables: &BTreeMap<Arc<str>, Value>,
    analysis: &Analysis,
    stdin: &mut R,
    stdout: &mut W,
    stderr: &mut E,
    capture: Option<RunTestCaptureHandle>,
) -> Result<ExitStatus, RunError> {
    let mut executor = StreamExecutor {
        plan,
        variables,
        output: ResultOutput::with_capture(stdout, options, capture),
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
    /// Fixed private-spool buffers, reported separately from the dynamic
    /// preparation budget because they are owned by the I/O layer.
    runtime_spool_fixed_io_bytes_high_water: usize,
    root_staging_encoded_bytes_high_water: usize,
    root_staging_memory_bytes_high_water: usize,
    root_staging_index_bytes_high_water: usize,
    root_staging_spool_bytes_written_high_water: u64,
}

#[allow(
    clippy::too_many_arguments,
    reason = "the automatic route keeps input, output, diagnostics, analysis, and test capture explicit"
)]
fn run_automatic_filter<R: Read, W: Write, E: Write, M>(
    options: &RunOptions,
    plan: &Plan<Compiled, M>,
    variables: &BTreeMap<Arc<str>, Value>,
    analysis: &Analysis,
    stdin: &mut R,
    stdout: &mut W,
    stderr: &mut E,
    capture: Option<RunTestCaptureHandle>,
) -> Result<ExitStatus, RunError> {
    let mut executor = AutomaticExecutor {
        plan,
        prefix: plan
            .automatic_prefix()
            .expect("automatic plan has a proven prefix")
            .to_vec(),
        projection: plan.automatic_projection().map(<[PathComponent]>::to_vec),
        capture_paths: plan.automatic_capture_paths(),
        variables,
        output: ResultOutput::with_capture(stdout, options, capture),
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
        root_spool: runtime_spool_for(options),
        root_probe: PrefixProbe::new(
            plan.automatic_prefix()
                .expect("automatic plan has a proven prefix"),
        ),
        root_fallback: None,
        root_active: false,
        root_object_order: Vec::new(),
        root_object_latest: HashMap::new(),
        root_object_seen: HashSet::new(),
        root_runtime_error: None,
        root_fatal: None,
        root_index_bytes: 0,
        root_index_limit: options.limits.preparation_memory_bytes,
        root_step_start: None,
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
    if let Some(error) = executor.root_runtime_error.take() {
        return Err(RunError::ReportedRuntime(error));
    }
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

#[allow(
    clippy::too_many_arguments,
    reason = "the hybrid route keeps input, output, diagnostics, analysis, and test capture explicit"
)]
fn run_hybrid_filter<R: Read, W: Write, E: Write>(
    options: &RunOptions,
    plan: &Plan<Compiled, HybridBlocking>,
    variables: &BTreeMap<Arc<str>, Value>,
    analysis: &Analysis,
    stdin: &mut R,
    stdout: &mut W,
    stderr: &mut E,
    capture: Option<RunTestCaptureHandle>,
) -> Result<ExitStatus, RunError> {
    let mut executor = AutomaticExecutor {
        plan,
        prefix: plan
            .automatic_prefix()
            .expect("hybrid plan has a proven producer prefix")
            .to_vec(),
        projection: plan.automatic_projection().map(<[PathComponent]>::to_vec),
        capture_paths: plan.automatic_capture_paths(),
        variables,
        output: ResultOutput::with_capture(stdout, options, capture),
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
        root_spool: runtime_spool_for(options),
        root_probe: PrefixProbe::new(
            plan.automatic_prefix()
                .expect("hybrid plan has a proven producer prefix"),
        ),
        root_fallback: None,
        root_active: false,
        root_object_order: Vec::new(),
        root_object_latest: HashMap::new(),
        root_object_seen: HashSet::new(),
        root_runtime_error: None,
        root_fatal: None,
        root_index_bytes: 0,
        root_index_limit: options.limits.preparation_memory_bytes,
        root_step_start: None,
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
        if let Some(error) = executor.root_runtime_error.take() {
            return Err(RunError::ReportedRuntime(error));
        }
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
    if format == InputFormat::Auto {
        let reader = LimitedReader::new(reader, options.limits.input_bytes, identity);
        let (report, replay) = probe_reader(reader, options.limits.lookahead_bytes)?;
        return automatic_committed_reader(
            options,
            report.selected,
            replay,
            identity,
            executor,
            true,
        );
    }
    automatic_committed_reader(options, format, reader, identity, executor, false)
}

fn automatic_committed_reader<R: Read, W: Write, E: Write, M>(
    options: &RunOptions,
    format: InputFormat,
    reader: R,
    identity: &str,
    executor: &mut AutomaticExecutor<'_, W, E, M>,
    source_limit_already_applied: bool,
) -> Result<(), RunError> {
    if format == InputFormat::Json {
        return automatic_json_root_reader(
            options,
            reader,
            identity,
            executor,
            source_limit_already_applied,
        );
    }
    let input = NativeFormat::from_input(format)
        .expect("input is committed")
        .select_input(decode_options(options, format), InputRepresentation::Events)?
        .open(reader, identity);
    let parallel = (executor.hybrid_suffix.is_some() && parallel_worker_count() > 1).then_some(
        ParallelJsonOptions {
            batch_values: options.limits.decode_batch_values,
            batch_bytes: options.limits.decode_batch_bytes,
            in_flight_batches: options.limits.decode_in_flight_batches,
            in_flight_bytes: options.limits.decode_in_flight_bytes,
        },
    );
    let decoded = input.consume_selected(
        executor.stream_selection(),
        parallel,
        cancellation(),
        |observation| {
            match observation {
                tq_formats::SelectedInputObservation::Record(record) => executor.accept(record),
                tq_formats::SelectedInputObservation::DocumentEnd => executor.finish_source(),
            }
            .map(|()| std::ops::ControlFlow::Continue(()))
        },
    );
    let observations = match decoded {
        Ok(observations) => observations,
        Err(tq_formats::InputDeliveryError::Consumer(error)) => return Err(error),
        Err(tq_formats::InputDeliveryError::Input(error)) => {
            if cancellation().is_some_and(|flag| flag.load(Ordering::Relaxed)) {
                return Err(RunError::Interrupted);
            }
            return Err(error.into());
        }
    };
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
    Ok(())
}

fn automatic_json_root_reader<R: Read, W: Write, E: Write, M>(
    options: &RunOptions,
    reader: R,
    identity: &str,
    executor: &mut AutomaticExecutor<'_, W, E, M>,
    source_limit_already_applied: bool,
) -> Result<(), RunError> {
    let mut observations = SelectedStreamObservations::default();
    let stream_options = StreamOptions {
        maximum_depth: options.limits.depth,
        maximum_token_bytes: options.limits.token_bytes,
        errors_as_values: false,
    };
    let selection = executor.stream_selection();
    let decoded = if source_limit_already_applied {
        stream_json_selected_roots_with_control(
            buffered_input(reader),
            stream_options,
            selection,
            cancellation(),
            &mut observations,
            executor,
        )
    } else {
        stream_json_selected_roots_with_control(
            buffered_input(LimitedReader::new(
                reader,
                options.limits.input_bytes,
                identity,
            )),
            stream_options,
            selection,
            cancellation(),
            &mut observations,
            executor,
        )
    };
    executor.retention.decoder_depth_high_water = executor
        .retention
        .decoder_depth_high_water
        .max(observations.depth_high_water);
    if let Some(error) = executor.root_fatal.take() {
        return Err(error);
    }
    match decoded {
        Ok(_) => Ok(()),
        Err(FormatError::Io(error))
            if error.to_string().contains("input resource limit exceeded") =>
        {
            Err(RunError::ResourceSource {
                identity: identity.to_owned(),
                resource: "input-bytes",
            })
        }
        Err(error) => Err(error.into()),
    }
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

    fn discard_document(&mut self) {
        match self {
            Self::Collect(values) => values.clear(),
            Self::StableSort { pipeline, config } => {
                *pipeline = Some(hybrid_sort_pipeline(*config));
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

fn runtime_spool_for(options: &RunOptions) -> RuntimeSpool {
    let config = RuntimeSpoolConfig {
        memory_threshold_bytes: options.limits.preparation_memory_bytes.max(1),
        // `--max-spool-bytes` bounds bytes written to the private spill file,
        // not logical records retained in memory.  Root staging and replay
        // enforce the per-root preparation budget separately, so the
        // cumulative logical counter must not make a zero-disk configuration
        // reject an otherwise in-memory root.
        maximum_total_bytes: u64::MAX,
        maximum_spool_bytes: options.limits.spool_bytes,
        maximum_item_bytes: u64::try_from(options.limits.preparation_memory_bytes.max(1))
            .unwrap_or(u64::MAX),
        maximum_depth: options.limits.depth,
        maximum_token_bytes: options.limits.token_bytes,
        maximum_decoded_bytes: options.limits.preparation_memory_bytes as u64,
        spool_directory: std::env::temp_dir(),
        allow_spool: true,
    };
    let spool = RuntimeSpool::new(config);
    match cancellation() {
        Some(flag) => spool.with_cancellation(flag),
        None => spool,
    }
}

#[derive(Clone, Debug)]
enum PrefixWitness {
    Unknown,
    /// The decoder found a scalar or an incompatible container at this
    /// boundary.  The value is the actual bounded ancestor, not a padded
    /// array synthesized for a large static index.
    Mismatch {
        boundary: usize,
        value: Value,
    },
    /// The decoder found the right container shape, but the selected path was
    /// absent below it.  The value is the smallest typed ancestor needed by
    /// the VM access operations.
    Missing {
        boundary: usize,
        value: Value,
    },
}

struct PrefixProbe {
    prefix: Vec<PathComponent>,
    witness: PrefixWitness,
}

impl PrefixProbe {
    fn new(prefix: &[PathComponent]) -> Self {
        Self {
            prefix: prefix.to_vec(),
            witness: PrefixWitness::Unknown,
        }
    }

    fn reset(&mut self) {
        self.witness = PrefixWitness::Unknown;
    }

    fn observe(&mut self, path: &[PathComponent], replacement: &SelectionReplacement) {
        if path.len() > self.prefix.len() || !self.prefix.starts_with(path) {
            return;
        }
        let boundary = path.len();
        if boundary == self.prefix.len() {
            return;
        }
        let compatible = match replacement {
            SelectionReplacement::ArrayStart => {
                matches!(self.prefix[boundary], PathComponent::Index(_))
            }
            SelectionReplacement::ObjectStart => {
                matches!(self.prefix[boundary], PathComponent::Key(_))
            }
            SelectionReplacement::Scalar(_) | SelectionReplacement::Missing => false,
        };
        let value = match replacement {
            SelectionReplacement::ArrayStart => Value::array(Vec::new()),
            SelectionReplacement::ObjectStart => Value::object(tq_core::Object::new()),
            SelectionReplacement::Scalar(value) => value.clone(),
            SelectionReplacement::Missing => Value::Null,
        };
        self.witness = if compatible {
            PrefixWitness::Missing { boundary, value }
        } else {
            PrefixWitness::Mismatch { boundary, value }
        };
    }

    fn missing(&self) -> PrefixWitness {
        match &self.witness {
            PrefixWitness::Unknown => PrefixWitness::Missing {
                boundary: 0,
                value: Value::Null,
            },
            witness => witness.clone(),
        }
    }
}

struct AutomaticExecutor<'a, W, E, M> {
    plan: &'a Plan<Compiled, M>,
    prefix: Vec<PathComponent>,
    projection: Option<Vec<PathComponent>>,
    capture_paths: Option<&'a [Vec<PathComponent>]>,
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
    root_spool: RuntimeSpool,
    root_probe: PrefixProbe,
    root_fallback: Option<SelectionFallback>,
    root_active: bool,
    root_object_order: Vec<Arc<str>>,
    root_object_latest: HashMap<Arc<str>, RuntimeRecordId>,
    root_object_seen: HashSet<Arc<str>>,
    root_runtime_error: Option<VmError>,
    root_fatal: Option<RunError>,
    root_index_bytes: usize,
    root_index_limit: usize,
    root_step_start: Option<u64>,
}

impl<W: Write, E: Write, M> AutomaticExecutor<'_, W, E, M> {
    fn stream_selection(&self) -> StreamSelection {
        StreamSelection::new(self.prefix.clone(), self.projection.clone())
    }

    fn begin_root_stage(&mut self, index: u64) -> Result<(), RunError> {
        self.root_spool
            .begin_root()
            .map_err(|error| runtime_spool_run_error(&error))?;
        self.root_probe.reset();
        self.root_fallback = None;
        self.root_active = true;
        self.root_step_start = Some(self.observations.steps);
        self.root_object_order = Vec::new();
        self.root_object_latest = HashMap::new();
        self.root_object_seen = HashSet::new();
        self.root_index_bytes = 0;
        self.current = None;
        self.current_item = None;
        self.deferred_item = None;
        let _ = index;
        Ok(())
    }

    fn reset_root_items(&mut self) -> Result<(), RunError> {
        self.finish_pending_root_item()?;
        self.root_spool
            .replace_root()
            .map_err(|error| runtime_spool_run_error(&error))?;
        self.root_object_order = Vec::new();
        self.root_object_latest = HashMap::new();
        self.root_object_seen = HashSet::new();
        self.root_index_bytes = 0;
        Ok(())
    }

    fn finish_pending_root_item(&mut self) -> Result<(), RunError> {
        if self.projection.is_some() {
            self.complete_projection_item()
        } else {
            self.complete_capture()
        }
    }

    fn prune_root_value(&self, value: &mut Value) {
        if self.root_active
            && let Some(paths) = self.capture_paths
        {
            prune_object_fields_in_place(value, paths);
        }
    }

    fn prepare_root_storage(
        &mut self,
        additional_value_bytes: usize,
        additional_index_bytes: usize,
    ) -> Result<(), RunError> {
        let io_headroom = if self.root_spool.spooled() {
            REPLAY_BUFFER_BYTES.saturating_mul(2)
        } else {
            0
        };
        self.retention.runtime_spool_fixed_io_bytes_high_water = self
            .retention
            .runtime_spool_fixed_io_bytes_high_water
            .max(io_headroom);
        self.observe_root_staging();
        let retained_memory = self.root_spool.retained_memory_capacity_bytes();
        let fixed = self
            .root_index_bytes
            .saturating_add(self.current.as_ref().map_or(0, |capture| capture.bytes))
            .saturating_add(
                self.deferred_item
                    .as_ref()
                    .map_or(0, estimate_capture_value_bytes),
            )
            .saturating_add(retained_memory);
        let required = fixed
            .saturating_add(additional_value_bytes)
            .saturating_add(additional_index_bytes);
        if required > self.root_index_limit {
            if retained_memory != 0 {
                self.root_spool
                    .spill_to_disk()
                    .map_err(|error| runtime_spool_run_error(&error))?;
                self.retention.runtime_spool_fixed_io_bytes_high_water = self
                    .retention
                    .runtime_spool_fixed_io_bytes_high_water
                    .max(REPLAY_BUFFER_BYTES.saturating_mul(2));
            }
            let fixed_after_spill = fixed.saturating_sub(retained_memory.min(fixed));
            if fixed_after_spill
                .saturating_add(additional_value_bytes)
                .saturating_add(additional_index_bytes)
                > self.root_index_limit
            {
                return Err(RunError::Resource("runtime-staging-memory"));
            }
        }
        let reserved = self
            .root_index_limit
            .saturating_sub(
                self.root_index_bytes
                    .saturating_add(self.current.as_ref().map_or(0, |capture| capture.bytes))
                    .saturating_add(
                        self.deferred_item
                            .as_ref()
                            .map_or(0, estimate_capture_value_bytes),
                    )
                    .saturating_add(additional_value_bytes)
                    .saturating_add(additional_index_bytes),
            )
            .max(1);
        self.root_spool
            .set_memory_threshold(reserved)
            .map_err(|error| runtime_spool_run_error(&error))?;
        self.observe_root_staging();
        Ok(())
    }

    fn observe_root_staging(&mut self) {
        self.retention.root_staging_encoded_bytes_high_water = self
            .retention
            .root_staging_encoded_bytes_high_water
            .max(usize::try_from(self.root_spool.retained_bytes()).unwrap_or(usize::MAX));
        self.retention.root_staging_memory_bytes_high_water = self
            .retention
            .root_staging_memory_bytes_high_water
            .max(self.root_spool.retained_memory_capacity_bytes());
        self.retention.root_staging_index_bytes_high_water = self
            .retention
            .root_staging_index_bytes_high_water
            .max(self.root_index_bytes);
        self.retention.root_staging_spool_bytes_written_high_water = self
            .retention
            .root_staging_spool_bytes_written_high_water
            .max(self.root_spool.spool_bytes_written());
    }

    fn stage_root_value(
        &mut self,
        path: Option<&[PathComponent]>,
        value: Value,
        projected: bool,
    ) -> Result<(), RunError> {
        let item = if projected {
            RuntimeSpoolItem::Projected(value)
        } else {
            RuntimeSpoolItem::Item(value)
        };
        let value_bytes = match &item {
            RuntimeSpoolItem::Projected(value)
            | RuntimeSpoolItem::Item(value)
            | RuntimeSpoolItem::Base(value) => estimate_capture_value_bytes(value),
        };
        let Some(path) = path else {
            self.prepare_root_storage(value_bytes, 0)?;
            self.root_spool
                .push_item(item)
                .map_err(|error| runtime_spool_run_error(&error))?;
            self.observe_root_staging();
            return Ok(());
        };
        let Some(component) = path.get(self.prefix.len()) else {
            self.prepare_root_storage(value_bytes, 0)?;
            self.root_spool
                .push_item(item)
                .map_err(|error| runtime_spool_run_error(&error))?;
            self.observe_root_staging();
            return Ok(());
        };
        let (new_key, latest_missing, required_index_bytes) = match component {
            PathComponent::Key(key) => {
                let new_key = !self.root_object_seen.contains(key);
                let latest_missing = !self.root_object_latest.contains_key(key);
                let required = if new_key || latest_missing {
                    key.len()
                        .saturating_add(std::mem::size_of::<Arc<str>>())
                        .saturating_add(std::mem::size_of::<(Arc<str>, RuntimeRecordId)>())
                        .saturating_add(std::mem::size_of::<Arc<str>>())
                        .saturating_add(ROOT_INDEX_BTREE_NODE_BYTES.saturating_mul(2))
                } else {
                    0
                };
                (new_key, latest_missing, required)
            }
            PathComponent::Index(_) => (false, false, 0),
        };
        let next_index_bytes = self.root_index_bytes.saturating_add(required_index_bytes);
        if next_index_bytes > self.root_index_limit {
            return Err(RunError::Resource("runtime-staging-index"));
        }
        self.prepare_root_storage(value_bytes, required_index_bytes)?;
        if new_key {
            let PathComponent::Key(key) = component else {
                unreachable!("new object key requires a key path component")
            };
            self.root_object_order
                .try_reserve_exact(1)
                .map_err(|_| RunError::Resource("runtime-staging-index"))?;
            self.root_object_seen
                .try_reserve(1)
                .map_err(|_| RunError::Resource("runtime-staging-index"))?;
            self.root_object_order.push(Arc::clone(key));
            self.root_object_seen.insert(Arc::clone(key));
        }
        if latest_missing {
            self.root_object_latest
                .try_reserve(1)
                .map_err(|_| RunError::Resource("runtime-staging-index"))?;
        }
        self.root_index_bytes = next_index_bytes;
        let id = self
            .root_spool
            .push_item(item)
            .map_err(|error| runtime_spool_run_error(&error))?;
        self.observe_root_staging();
        let PathComponent::Key(key) = component else {
            return Ok(());
        };
        self.root_object_latest.insert(Arc::clone(key), id);
        Ok(())
    }

    fn stage_root_base(&mut self, value: Value) -> Result<(), RunError> {
        self.prepare_root_storage(estimate_capture_value_bytes(&value), 0)?;
        self.root_spool
            .push_item(RuntimeSpoolItem::Base(value))
            .map_err(|error| runtime_spool_run_error(&error))?;
        self.observe_root_staging();
        Ok(())
    }

    fn commit_root_stage(&mut self) -> Result<(), RunError> {
        self.finish_pending_root_item()?;
        let fallback = self.root_fallback.take();
        let witness = self.root_probe.missing();
        self.root_spool
            .finish_root()
            .map_err(|error| runtime_spool_run_error(&error))?;
        self.root_active = false;

        let recoverable_error = self.replay_root_stage(fallback, witness)?;

        if let Some(error) = recoverable_error {
            report_document_runtime_error(self.stderr, &error)?;
            self.discard_hybrid_document();
            self.root_runtime_error = Some(error);
        } else if let Some(plan) = self.hybrid_suffix {
            match self.finish_hybrid_document(plan) {
                Ok(()) => self.root_runtime_error = None,
                Err(RunError::Runtime(error)) if is_recoverable_document_runtime_error(&error) => {
                    report_document_runtime_error(self.stderr, &error)?;
                    self.discard_hybrid_document();
                    self.root_runtime_error = Some(error);
                }
                Err(RunError::ReportedRuntime(error)) => {
                    report_document_runtime_error(self.stderr, &error)?;
                    self.discard_hybrid_document();
                    self.root_runtime_error = Some(error);
                }
                Err(error) => return Err(error),
            }
        } else {
            self.root_runtime_error = None;
        }
        self.root_step_start = None;
        Ok(())
    }

    fn replay_root_stage(
        &mut self,
        fallback: Option<SelectionFallback>,
        witness: PrefixWitness,
    ) -> Result<Option<VmError>, RunError> {
        let mut spool = std::mem::replace(
            &mut self.root_spool,
            RuntimeSpool::new(RuntimeSpoolConfig::default()),
        );
        let replay = if matches!(fallback, Some(SelectionFallback::Missing { .. })) {
            self.evaluate_prefix_tail(witness)
                .map_err(|error| match error {
                    RunError::Runtime(error) if is_recoverable_document_runtime_error(&error) => {
                        RunError::ReportedRuntime(error)
                    }
                    error => error,
                })
        } else if let Some(fallback) = fallback {
            match fallback {
                SelectionFallback::PresentScalar { value, .. } => {
                    self.run_staged_item(RuntimeSpoolItem::Base(value))
                }
                SelectionFallback::PresentEmptyArray { .. } => {
                    self.run_staged_item(RuntimeSpoolItem::Base(Value::array(Vec::new())))
                }
                SelectionFallback::PresentEmptyObject { .. } => self.run_staged_item(
                    RuntimeSpoolItem::Base(Value::object(tq_core::Object::new())),
                ),
                SelectionFallback::Missing { .. } => unreachable!("missing handled above"),
            }
        } else if !spool.is_empty() {
            let replay_index_bytes = self.root_index_bytes;
            let mut replay_memory = spool.retained_memory_capacity_bytes();
            let replay_available = self.root_index_limit.saturating_sub(replay_index_bytes);
            let replay_requirement =
                usize::try_from(spool.maximum_decoded_requirement()).unwrap_or(usize::MAX);
            // Keep encoded records in memory when a decoded value still has
            // room in the shared budget.  If the resident capacity plus the
            // largest decoded item consumes that room, spill before replay so
            // the decoded limit is not hidden by the local `spool` variable.
            if replay_memory.saturating_add(replay_requirement) > replay_available {
                spool
                    .spill_to_disk()
                    .map_err(|error| runtime_spool_run_error(&error))?;
                replay_memory = spool.retained_memory_capacity_bytes();
                self.retention.runtime_spool_fixed_io_bytes_high_water = self
                    .retention
                    .runtime_spool_fixed_io_bytes_high_water
                    .max(REPLAY_BUFFER_BYTES.saturating_mul(2));
            }
            let replay_decoded_limit = replay_available.saturating_sub(replay_memory);
            spool
                .set_replay_decoded_budget(replay_decoded_limit)
                .map_err(|error| runtime_spool_run_error(&error))?;
            let order = std::mem::take(&mut self.root_object_order);
            let latest = std::mem::take(&mut self.root_object_latest);
            self.root_object_seen.clear();
            self.root_index_bytes = 0;
            let mut run_item = |item: RuntimeSpoolItem| self.run_staged_item(item);
            if order.is_empty() {
                spool
                    .replay_items(&mut run_item)
                    .map_err(runtime_replay_error)
            } else {
                spool
                    .replay_records(
                        order
                            .into_iter()
                            .filter_map(|key| latest.get(&key).copied()),
                        &mut run_item,
                    )
                    .map_err(runtime_replay_error)
            }
        } else {
            Ok(())
        };
        self.root_spool = spool;
        self.observe_root_staging();
        match replay {
            Err(RunError::ReportedRuntime(error)) => Ok(Some(error)),
            Err(error) => Err(error),
            Ok(()) => Ok(None),
        }
    }

    fn run_staged_item(&mut self, item: RuntimeSpoolItem) -> Result<(), RunError> {
        let result = match item {
            RuntimeSpoolItem::Projected(value) => self.emit_direct(value),
            RuntimeSpoolItem::Item(value) => self.evaluate(value, false),
            RuntimeSpoolItem::Base(value) => self.evaluate(value, true),
        };
        match result {
            Ok(()) => Ok(()),
            Err(RunError::Runtime(error)) if is_recoverable_document_runtime_error(&error) => {
                Err(RunError::ReportedRuntime(error))
            }
            Err(RunError::ReportedRuntime(error)) => Err(RunError::ReportedRuntime(error)),
            Err(error) => Err(error),
        }
    }

    fn discard_hybrid_document(&mut self) {
        if let Some(collected) = self.collected.as_mut() {
            collected.discard_document();
        }
    }

    fn evaluate_prefix_tail(&mut self, witness: PrefixWitness) -> Result<(), RunError> {
        let (PrefixWitness::Mismatch { boundary, value }
        | PrefixWitness::Missing { boundary, value }) = witness
        else {
            return Ok(());
        };
        let mut value = Some(value);
        for component in boundary..self.prefix.len() {
            let Some(input) = value.take() else {
                return Ok(());
            };
            value = self.evaluate_prefix_access(component, input)?;
        }
        if let Some(value) = value {
            self.evaluate(value, true)?;
        }
        Ok(())
    }

    fn evaluate_prefix_access(
        &mut self,
        component: usize,
        input: Value,
    ) -> Result<Option<Value>, RunError> {
        let mut vm = Vm::new_automatic_prefix_access(
            self.plan,
            component,
            input,
            self.remaining_vm_limits(),
            self.variables.clone(),
        )
        .with_trace_limit(self.trace_remaining);
        if let Some(flag) = cancellation() {
            vm = vm.with_cancellation(flag);
        }
        let mut value = None;
        let evaluated = vm.for_each_result(|candidate| {
            value = Some(candidate);
            true
        });
        flush_vm_effects(&vm, self.stderr)?;
        merge_observations(&mut self.observations, vm.observations());
        evaluated.map_err(RunError::Runtime)?;
        if self.trace_remaining != 0 {
            for entry in vm.trace() {
                writeln!(self.stderr, "trace: {entry}")?;
            }
            self.trace_remaining = self.trace_remaining.saturating_sub(vm.trace().len());
        }
        Ok(value)
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
                if self.root_active {
                    self.stage_root_base(value)?;
                } else {
                    self.evaluate(value, true)?;
                }
            }
            return Ok(());
        }
        if path.len() <= self.prefix.len() || !path.starts_with(&self.prefix) {
            return Ok(());
        }
        if self.projection.is_some() {
            return self.accept_projection(&path, value, record_bytes);
        }
        self.accept_unprojected(&path, value, record_bytes)
    }

    fn accept_unprojected(
        &mut self,
        path: &[PathComponent],
        value: Option<Value>,
        record_bytes: usize,
    ) -> Result<(), RunError> {
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
                if self.root_active {
                    self.stage_root_value(Some(&target), value, false)?;
                } else {
                    self.evaluate(value, false)?;
                }
            }
            return Ok(());
        }

        if relative.is_empty() {
            if let Some(value) = value {
                self.observe_complete_value(record_bytes, 0)?;
                self.retention.completed_subtrees =
                    self.retention.completed_subtrees.saturating_add(1);
                if self.root_active {
                    let mut value = value;
                    self.prune_root_value(&mut value);
                    self.stage_root_value(Some(&target), value, false)?;
                } else {
                    self.evaluate(value, false)?;
                }
            } else if self
                .current
                .as_ref()
                .is_some_and(|capture| capture.path == target)
            {
                let capture = self.current.take().expect("capture was checked");
                self.retention.completed_subtrees =
                    self.retention.completed_subtrees.saturating_add(1);
                let mut used_bytes = capture.bytes;
                let mut value = capture.root.into_value_bounded(
                    self.output.options.limits.preparation_memory_bytes,
                    &mut used_bytes,
                )?;
                if self.root_active {
                    self.prune_root_value(&mut value);
                    self.stage_root_value(Some(&target), value, false)?;
                } else {
                    self.evaluate(value, false)?;
                }
            }
            return Ok(());
        }
        let Some(value) = value else {
            return Ok(());
        };
        let record_cost = record_bytes.saturating_add(64);
        let value_cost = estimate_capture_value_bytes(&value);
        self.prepare_root_storage(
            record_cost
                .saturating_add(value_cost)
                .saturating_add(build_node_path_bytes(relative)),
            0,
        )?;
        let capture = self.current.get_or_insert_with(|| Capture {
            path: target,
            root: BuildNode::empty_for(&relative[0]),
            bytes: 0,
        });
        capture.bytes = capture.bytes.saturating_add(record_cost);
        if capture.bytes > self.output.options.limits.preparation_memory_bytes {
            return Err(RunError::Resource("subtree-bytes"));
        }
        let relative_depth = relative.len();
        if relative_depth > self.output.options.limits.depth {
            return Err(RunError::Resource("subtree-depth"));
        }
        capture.root.insert_bounded(
            relative,
            value,
            self.output.options.limits.preparation_memory_bytes,
            &mut capture.bytes,
        )?;
        let capture_bytes = capture.bytes;
        self.retention.bytes_high_water = self.retention.bytes_high_water.max(capture_bytes);
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
        self.accept_projection_value(path, value, record_bytes, item_length, relative)
    }

    fn accept_projection_value(
        &mut self,
        path: &[PathComponent],
        value: Option<Value>,
        record_bytes: usize,
        item_length: usize,
        relative: &[PathComponent],
    ) -> Result<(), RunError> {
        if relative.is_empty() {
            if let Some(value) = value {
                self.observe_complete_value(record_bytes, 0)?;
                if self.root_active {
                    self.prepare_root_storage(estimate_capture_value_bytes(&value), 0)?;
                }
                self.deferred_item = Some(value);
            }
            return Ok(());
        }
        let projection = self
            .projection
            .as_deref()
            .map(<[PathComponent]>::to_vec)
            .expect("projection branch requires a projected path");
        if path_kind_mismatch(relative, &projection) {
            if let Some(value) = value {
                if self.root_active {
                    self.prepare_root_storage(
                        estimate_capture_value_bytes(&value)
                            .saturating_add(build_node_path_bytes(relative)),
                        0,
                    )?;
                }
                self.deferred_item = Some(synthetic_item(
                    relative,
                    value,
                    self.output.options.limits.preparation_memory_bytes,
                )?);
            }
            return Ok(());
        }
        if relative == projection {
            if let Some(value) = value {
                self.observe_complete_value(record_bytes, 0)?;
                if self.root_active {
                    self.prepare_root_storage(estimate_capture_value_bytes(&value), 0)?;
                }
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
                if self.root_active {
                    self.prepare_root_storage(
                        estimate_capture_value_bytes(&value)
                            .saturating_add(build_node_path_bytes(relative)),
                        0,
                    )?;
                }
                self.deferred_item = Some(synthetic_item(
                    relative,
                    value,
                    self.output.options.limits.preparation_memory_bytes,
                )?);
            }
            return Ok(());
        }
        if !relative.starts_with(&projection) {
            return Ok(());
        }
        let captured_path = &relative[projection.len()..];
        let Some(value) = value else {
            return Ok(());
        };
        let record_cost = record_bytes.saturating_add(64);
        self.prepare_root_storage(
            record_cost
                .saturating_add(estimate_capture_value_bytes(&value))
                .saturating_add(build_node_path_bytes(captured_path)),
            0,
        )?;
        let capture = self.current.get_or_insert_with(|| Capture {
            path: path[..item_length + projection.len()].to_vec(),
            root: BuildNode::empty_for(&captured_path[0]),
            bytes: 0,
        });
        capture.bytes = capture.bytes.saturating_add(record_cost);
        if capture.bytes > self.output.options.limits.preparation_memory_bytes {
            return Err(RunError::Resource("subtree-bytes"));
        }
        capture.root.insert_bounded(
            captured_path,
            value,
            self.output.options.limits.preparation_memory_bytes,
            &mut capture.bytes,
        )?;
        let capture_bytes = capture.bytes;
        self.retention.bytes_high_water = self.retention.bytes_high_water.max(capture_bytes);
        self.retention.depth_high_water = self.retention.depth_high_water.max(captured_path.len());
        Ok(())
    }

    fn complete_projection_item(&mut self) -> Result<(), RunError> {
        let Some(item_path) = self.current_item.take() else {
            return Ok(());
        };
        self.retention.completed_subtrees = self.retention.completed_subtrees.saturating_add(1);
        let deferred = self.deferred_item.take();
        if let Some(capture) = self.current.take() {
            let mut used_bytes = capture.bytes;
            let value = capture.root.into_value_bounded(
                self.output.options.limits.preparation_memory_bytes,
                &mut used_bytes,
            )?;
            if self.root_active {
                self.stage_root_value(Some(&item_path), value, true)?;
            } else {
                self.emit_direct(value)?;
            }
        } else if let Some(item) = deferred {
            if self.root_active {
                let mut item = item;
                self.prune_root_value(&mut item);
                self.stage_root_value(Some(&item_path), item, false)?;
            } else {
                self.evaluate(item, false)?;
            }
        } else {
            if self.root_active {
                self.stage_root_value(Some(&item_path), Value::Null, true)?;
            } else {
                self.emit_direct(Value::Null)?;
            }
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
        let mut used_bytes = capture.bytes;
        let value = capture.root.into_value_bounded(
            self.output.options.limits.preparation_memory_bytes,
            &mut used_bytes,
        )?;
        if self.root_active {
            let mut value = value;
            self.prune_root_value(&mut value);
            self.stage_root_value(Some(&capture.path), value, false)
        } else {
            self.evaluate(value, false)
        }
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
        flush_vm_effects(&vm, self.stderr)?;
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
        flush_vm_effects(&vm, self.stderr)?;
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
        let used_steps = self
            .root_step_start
            .map_or(self.observations.steps, |start| {
                self.observations.steps.saturating_sub(start)
            });
        limits.steps = limits.steps.saturating_sub(used_steps);
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

fn runtime_spool_run_error(error: &RuntimeSpoolError) -> RunError {
    match error {
        RuntimeSpoolError::Cancelled => RunError::Interrupted,
        RuntimeSpoolError::Io(_) => RunError::Resource("runtime-staging-io"),
        RuntimeSpoolError::TotalLimit
        | RuntimeSpoolError::SpoolLimit
        | RuntimeSpoolError::ItemLimit
        | RuntimeSpoolError::TokenLimit
        | RuntimeSpoolError::DepthLimit
        | RuntimeSpoolError::DecodedLimit
        | RuntimeSpoolError::SpoolDisabled => RunError::Resource("runtime-staging"),
        RuntimeSpoolError::Number(_) => RunError::Resource("runtime-staging-number"),
        RuntimeSpoolError::Decode(_)
        | RuntimeSpoolError::Utf8
        | RuntimeSpoolError::State(_)
        | RuntimeSpoolError::GenerationLimit
        | RuntimeSpoolError::StaleRecord
        | RuntimeSpoolError::RecordBounds => RunError::Unsupported(
            "automatic runtime staging produced an invalid retained record".to_owned(),
        ),
    }
}

fn runtime_replay_error(error: RuntimeSpoolReplayError<RunError>) -> RunError {
    match error {
        RuntimeSpoolReplayError::Storage(error) => runtime_spool_run_error(&error),
        RuntimeSpoolReplayError::Consumer(error) => error,
    }
}

impl<W: Write, E: Write, M> SelectedRootSink for AutomaticExecutor<'_, W, E, M> {
    type Error = String;

    fn begin_root(&mut self, index: u64) -> Result<(), Self::Error> {
        self.begin_root_stage(index).map_err(|error| {
            self.root_fatal = Some(error);
            "automatic root staging could not begin".to_owned()
        })
    }

    fn fallback(&mut self, fallback: SelectionFallback) -> Result<(), Self::Error> {
        self.finish_pending_root_item().map_err(|error| {
            self.root_fatal = Some(error);
            "automatic root staging could not finish a pending item".to_owned()
        })?;
        self.root_fallback = Some(fallback);
        Ok(())
    }

    fn replace_selected_prefix(
        &mut self,
        path: &[PathComponent],
        replacement: SelectionReplacement,
    ) -> Result<(), Self::Error> {
        self.reset_root_items().map_err(|error| {
            self.root_fatal = Some(error);
            "automatic root staging could not replace a selected prefix".to_owned()
        })?;
        self.root_probe.observe(path, &replacement);
        self.root_fallback = None;
        Ok(())
    }

    fn replace_item_path(
        &mut self,
        path: &[PathComponent],
        _replacement: SelectionReplacement,
    ) -> Result<(), Self::Error> {
        let item_length = self.prefix.len() + 1;
        let target = &path[..item_length];
        let pending = self.current_item.as_deref().or_else(|| {
            self.current
                .as_ref()
                .map(|capture| &capture.path[..item_length])
        });
        if path.len() == item_length || pending.is_some_and(|pending| pending != target) {
            self.finish_pending_root_item().map_err(|error| {
                self.root_fatal = Some(error);
                "automatic root staging could not replace an item".to_owned()
            })?;
        }
        if path.len() == item_length {
            if let Some(PathComponent::Key(key)) = path.get(self.prefix.len()) {
                self.root_object_latest.remove(key);
            }
        } else {
            if self
                .current
                .as_ref()
                .is_some_and(|capture| capture.path.starts_with(path))
            {
                self.current = None;
            } else if let Some(capture) = self.current.as_mut()
                && path.starts_with(&capture.path)
            {
                capture.root.clear_path(&path[capture.path.len()..]);
            }
            if self
                .projection
                .as_ref()
                .is_some_and(|projection| projection.starts_with(&path[item_length..]))
            {
                self.deferred_item = None;
            }
        }
        Ok(())
    }

    fn record(&mut self, record: StreamRecord) -> Result<(), Self::Error> {
        self.accept(record).map_err(|error| {
            self.root_fatal = Some(error);
            "automatic root staging rejected a selected record".to_owned()
        })
    }

    fn finish_root(&mut self) -> Result<(), Self::Error> {
        self.commit_root_stage().map_err(|error| {
            self.root_fatal = Some(error);
            "automatic root staging could not commit the validated root".to_owned()
        })
    }

    fn abort_root(&mut self) {
        self.current = None;
        self.current_item = None;
        self.deferred_item = None;
        let _ = self.root_spool.abort_root();
        self.root_active = false;
        self.root_step_start = None;
        self.root_fallback = None;
        self.root_object_order.clear();
        self.root_object_latest.clear();
        self.root_object_seen.clear();
        self.root_index_bytes = 0;
        self.root_probe.reset();
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
    fn clear_path(&mut self, path: &[PathComponent]) {
        let Some((component, tail)) = path.split_first() else {
            *self = Self::Missing;
            return;
        };
        match (self, component) {
            (Self::Array(values), PathComponent::Index(index)) => {
                if let Some(child) = values.get_mut(*index) {
                    child.clear_path(tail);
                }
            }
            (Self::Object(values), PathComponent::Key(key)) => {
                if let Some((_, child)) = values.iter_mut().find(|(candidate, _)| candidate == key)
                {
                    child.clear_path(tail);
                }
            }
            _ => {}
        }
    }

    fn empty_for(component: &PathComponent) -> Self {
        match component {
            PathComponent::Index(_) => Self::Array(Vec::new()),
            PathComponent::Key(_) => Self::Object(Vec::new()),
        }
    }

    fn insert_bounded(
        &mut self,
        path: &[PathComponent],
        value: Value,
        maximum_bytes: usize,
        used_bytes: &mut usize,
    ) -> Result<(), RunError> {
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
                    let additional = index.saturating_add(1).saturating_sub(values.len());
                    let bytes =
                        additional.saturating_mul(std::mem::size_of::<Self>().saturating_add(16));
                    charge_build_node_bytes(used_bytes, maximum_bytes, bytes)?;
                    values
                        .try_reserve_exact(additional)
                        .map_err(|_| RunError::Resource("subtree-memory"))?;
                    values.resize_with(index.saturating_add(1), || Self::Missing);
                }
                if !tail.is_empty() && matches!(values[*index], Self::Missing) {
                    values[*index] = Self::empty_for(&tail[0]);
                }
                values[*index].insert_bounded(tail, value, maximum_bytes, used_bytes)
            }
            PathComponent::Key(key) => {
                let Self::Object(values) = self else {
                    return Err(invalid_automatic_event());
                };
                let index = values.iter().position(|(candidate, _)| candidate == key);
                let child = if let Some(index) = index {
                    &mut values[index].1
                } else {
                    charge_build_node_bytes(
                        used_bytes,
                        maximum_bytes,
                        std::mem::size_of::<(Arc<str>, Self)>().saturating_add(256),
                    )?;
                    values
                        .try_reserve_exact(1)
                        .map_err(|_| RunError::Resource("subtree-memory"))?;
                    values.push((Arc::clone(key), Self::Missing));
                    &mut values.last_mut().expect("entry was pushed").1
                };
                if !tail.is_empty() && matches!(child, Self::Missing) {
                    *child = Self::empty_for(&tail[0]);
                }
                child.insert_bounded(tail, value, maximum_bytes, used_bytes)
            }
        }
    }

    fn into_value_bounded(
        self,
        maximum_bytes: usize,
        used_bytes: &mut usize,
    ) -> Result<Value, RunError> {
        match self {
            Self::Missing => Err(invalid_automatic_event()),
            Self::Value(value) => Ok(value),
            Self::Array(values) => {
                let bytes = values
                    .len()
                    .saturating_mul(std::mem::size_of::<Value>().saturating_add(16));
                charge_build_node_bytes(used_bytes, maximum_bytes, bytes)?;
                let mut output = Vec::new();
                output
                    .try_reserve_exact(values.len())
                    .map_err(|_| RunError::Resource("subtree-memory"))?;
                for value in values {
                    output.push(value.into_value_bounded(maximum_bytes, used_bytes)?);
                }
                Ok(Value::array(output))
            }
            Self::Object(values) => {
                let bytes = values
                    .len()
                    .saturating_mul(std::mem::size_of::<(Arc<str>, Value)>().saturating_add(256));
                charge_build_node_bytes(used_bytes, maximum_bytes, bytes)?;
                let mut output = tq_core::Object::new();
                output
                    .try_reserve(values.len())
                    .map_err(|_| RunError::Resource("subtree-memory"))?;
                for (key, value) in values {
                    output.insert(key, value.into_value_bounded(maximum_bytes, used_bytes)?);
                }
                Ok(Value::object(output))
            }
        }
    }
}

fn charge_build_node_bytes(
    used_bytes: &mut usize,
    maximum_bytes: usize,
    additional_bytes: usize,
) -> Result<(), RunError> {
    let Some(next) = used_bytes.checked_add(additional_bytes) else {
        return Err(RunError::Resource("subtree-memory"));
    };
    if next > maximum_bytes {
        return Err(RunError::Resource("subtree-memory"));
    }
    *used_bytes = next;
    Ok(())
}

fn build_node_path_bytes(path: &[PathComponent]) -> usize {
    path.iter().fold(0_usize, |total, component| {
        total.saturating_add(match component {
            PathComponent::Index(_) => std::mem::size_of::<BuildNode>().saturating_add(16),
            PathComponent::Key(_) => {
                std::mem::size_of::<(Arc<str>, BuildNode)>().saturating_add(256)
            }
        })
    })
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

fn synthetic_item(
    relative: &[PathComponent],
    value: Value,
    maximum_bytes: usize,
) -> Result<Value, RunError> {
    let Some(first) = relative.first() else {
        return Ok(value);
    };
    let mut root = BuildNode::empty_for(first);
    let mut used_bytes = 0;
    root.insert_bounded(relative, value, maximum_bytes, &mut used_bytes)?;
    root.into_value_bounded(maximum_bytes, &mut used_bytes)
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

fn estimate_capture_value_bytes(value: &Value) -> usize {
    estimate_value_bytes(value).saturating_add(
        value_node_count(value).saturating_mul(std::mem::size_of::<Value>().saturating_add(16)),
    )
}

/// Retains only the statically proven object fields in an already materialized
/// item.  The proof is deliberately conservative, so a shared object is left
/// untouched rather than allocating a reduced clone beside its source.
fn prune_object_fields_in_place(value: &mut Value, paths: &[Vec<PathComponent>]) {
    let Some(anchor) = paths.first().map(Vec::as_slice) else {
        return;
    };
    prune_object_fields_at_depth(value, paths, 0, anchor);
}

fn prune_object_fields_at_depth(
    value: &mut Value,
    paths: &[Vec<PathComponent>],
    depth: usize,
    anchor: &[PathComponent],
) {
    let Value::Object(values) = value else {
        return;
    };
    let Some(values) = Arc::get_mut(values) else {
        return;
    };
    values.retain(|key, child| {
        let Some(child_anchor) = paths
            .iter()
            .find(|path| path_matches_branch(path, anchor, depth, key))
            .map(Vec::as_slice)
        else {
            return false;
        };
        let has_nested_path = paths.iter().any(|path| {
            path_matches_branch(path, anchor, depth, key) && path.len() > depth.saturating_add(1)
        });
        let keeps_whole_child = paths.iter().any(|path| {
            path_matches_branch(path, anchor, depth, key) && path.len() == depth.saturating_add(1)
        });
        if has_nested_path && !keeps_whole_child {
            prune_object_fields_at_depth(child, paths, depth.saturating_add(1), child_anchor);
        }
        true
    });
}

fn path_matches_branch(
    path: &[PathComponent],
    anchor: &[PathComponent],
    depth: usize,
    key: &Arc<str>,
) -> bool {
    path.get(..depth) == anchor.get(..depth)
        && matches!(
            path.get(depth),
            Some(PathComponent::Key(candidate)) if candidate == key
        )
}

fn value_node_count(value: &Value) -> usize {
    1_usize.saturating_add(match value {
        Value::Array(values) => values.iter().fold(0_usize, |total, value| {
            total.saturating_add(value_node_count(value))
        }),
        Value::Object(values) => values.values().fold(0_usize, |total, value| {
            total.saturating_add(value_node_count(value))
        }),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => 0,
    })
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
    let reader = LimitedReader::new(reader, options.limits.input_bytes, identity);
    if format == InputFormat::JsonSequence {
        return stream_projected_input(options, format, reader, identity, executor);
    }
    let stream_options = StreamOptions {
        maximum_depth: options.limits.depth,
        maximum_token_bytes: options.limits.token_bytes,
        errors_as_values: options.stream_errors,
    };
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
                InputFormat::JsonSequence => {
                    stream_projected_input(options, InputFormat::JsonSequence, replay, identity, executor)
                }
                InputFormat::Yaml => Err(RunError::Unsupported(
                    "auto-detection selected YAML, which is document-at-a-time and cannot satisfy --stream; use --input-format json for JSON syntax".to_owned(),
                )),
                InputFormat::Auto
                | InputFormat::Json5
                | InputFormat::JsonLines
                | InputFormat::ToonSequence => unreachable!("probe candidate"),
                InputFormat::Csv | InputFormat::Tsv => Err(RunError::Unsupported(
                    "auto-detection selected delimited input, which cannot satisfy --stream"
                        .to_owned(),
                )),
            }
        }
        InputFormat::Yaml => Err(RunError::Unsupported(
            "YAML input is document-at-a-time and cannot satisfy --stream".to_owned(),
        )),
        InputFormat::Json5 => Err(RunError::Unsupported(JSON5_STREAM_UNSUPPORTED.to_owned())),
        InputFormat::ToonSequence => Err(RunError::Unsupported(
            "TOON sequence input cannot currently be nested inside --stream".to_owned(),
        )),
        InputFormat::JsonSequence => unreachable!("handled before stream decoder selection"),
        InputFormat::Csv | InputFormat::Tsv => Err(RunError::Unsupported(
            "delimited input cannot satisfy --stream".to_owned(),
        )),
    }
}

fn stream_projected_input<R: Read, W: Write, E: Write>(
    options: &RunOptions,
    format: InputFormat,
    reader: R,
    identity: &str,
    executor: &mut StreamExecutor<'_, W, E>,
) -> Result<(), RunError> {
    let mut emit = |input| match input {
        StructuredInput::Value(value) => executor.accept(value).map(|()| true),
        StructuredInput::Failure(failure) => {
            render_native_failure(executor.stderr, &failure, options.json_sequence)?;
            Ok(true)
        }
        StructuredInput::Document(_)
        | StructuredInput::Proxy(_)
        | StructuredInput::Warning { .. } => {
            unreachable!("native event projection emits values or failures")
        }
    };
    project_committed_input(options, format, reader, identity, &mut emit).map(|_| ())
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

fn buffered_input<R: Read>(reader: R) -> BufReader<R> {
    BufReader::with_capacity(INPUT_BUFFER_BYTES, reader)
}

fn project_committed_input<R: Read, F>(
    options: &RunOptions,
    format: InputFormat,
    reader: R,
    identity: &str,
    emit: &mut F,
) -> Result<bool, RunError>
where
    F: FnMut(StructuredInput) -> Result<bool, RunError>,
{
    use std::ops::ControlFlow;
    use tq_toon::EventConsumer;

    let format = NativeFormat::from_input(format).expect("input is committed");
    if !format.descriptor().events {
        return Err(RunError::Unsupported(format!(
            "{} input is document-at-a-time and cannot satisfy --stream",
            format.descriptor().name
        )));
    }
    let input = format
        .select_input(
            decode_options(options, format.descriptor().input),
            InputRepresentation::Events,
        )?
        .open(reader, identity);
    let execution_error = std::cell::RefCell::new(None);
    let stopped = std::cell::Cell::new(false);
    let emit = std::cell::RefCell::new(emit);
    let publish = |input| match emit.borrow_mut()(input) {
        Ok(true) => Ok(()),
        Ok(false) => {
            stopped.set(true);
            Err("input consumer stopped".to_owned())
        }
        Err(error) => {
            execution_error.replace(Some(error));
            Err("input consumer failed".to_owned())
        }
    };
    let mut emit_record =
        |record: StreamRecord| publish(StructuredInput::Value(record.into_value()));
    let mut projector = tq_formats::EventProjector::new(
        StreamOptions {
            maximum_depth: options.limits.depth,
            maximum_token_bytes: options.limits.token_bytes,
            errors_as_values: options.stream_errors,
        },
        &mut emit_record,
    );
    let decoded = input.consume_events(|observation| {
        match observation {
            NativeInputObservation::Event(event) => projector.consume(event),
            NativeInputObservation::Failure(failure) if options.stream_errors => {
                projector.error_value(failure.message)
            }
            NativeInputObservation::Failure(failure) => {
                projector.reset();
                publish(StructuredInput::Failure(failure))
            }
            NativeInputObservation::Document(_) => unreachable!("event representation"),
        }
        .map(|()| ControlFlow::Continue(()))
    });
    let decoded = match decoded {
        Err(tq_formats::InputDeliveryError::Input(FormatError::Parse { message, .. }))
            if options.stream_errors =>
        {
            projector
                .error_value(message)
                .map_err(tq_formats::InputDeliveryError::Consumer)
        }
        result => result,
    };
    if let Some(error) = execution_error.borrow_mut().take() {
        return Err(error);
    }
    if stopped.get() {
        return Ok(false);
    }
    match decoded {
        Ok(()) => Ok(true),
        Err(tq_formats::InputDeliveryError::Input(error)) => Err(error.into()),
        Err(tq_formats::InputDeliveryError::Consumer(message)) => {
            Err(RunError::Input(FormatError::Parse {
                format: format.descriptor().input,
                message,
            }))
        }
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
    let (selected, documents) = native_source_documents(bytes, identity, options, format)?;
    if selected == InputFormat::Json && documents.len() != 1 {
        return Err(FormatError::Parse {
            format: InputFormat::Json,
            message: format!("{identity} requires exactly one JSON value for event input"),
        });
    }
    Ok(())
}

fn native_source_documents(
    bytes: &[u8],
    identity: &str,
    options: &RunOptions,
    format: InputFormat,
) -> Result<(InputFormat, Vec<tq_formats::Document>), FormatError> {
    let format = if format == InputFormat::Auto {
        probe_format(bytes, options.limits.lookahead_bytes)?.selected
    } else {
        format
    };
    let mut input = NativeFormat::from_input(format)
        .expect("input is committed")
        .select_input(
            decode_options(options, format),
            InputRepresentation::Documents,
        )?
        .open(bytes, identity);
    let mut documents = Vec::new();
    while let Some(observation) = input.next_observation()? {
        match observation {
            NativeInputObservation::Document(document) => documents.push(document),
            NativeInputObservation::Failure(failure) => {
                return Err(FormatError::Parse {
                    format,
                    message: failure.message,
                });
            }
            NativeInputObservation::Event(_) => unreachable!("Document representation"),
        }
    }
    Ok((format, documents))
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
        flush_vm_effects(&vm, self.stderr)?;
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

fn record_document_observations(
    options: &RunOptions,
    observations: &mut Vec<VmObservations>,
    item: VmObservations,
) {
    // Observations are report payload, not retained execution state.
    if options.report_file.is_some() {
        if options.stream
            && let Some(total) = observations.first_mut()
        {
            merge_observations(total, item);
        } else {
            observations.push(item);
        }
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
    native: NativeOutputSequence,
    capture: Option<RunTestCaptureHandle>,
    written: u64,
    emitted: u64,
    last_was_proxy: bool,
}

fn color_palette(options: &RunOptions) -> JsonColorPalette {
    if options.capability_policy.environment {
        std::env::var("JQ_COLORS").map_or_else(
            |_| JsonColorPalette::default(),
            |value| JsonColorPalette::from_jq_colors(&value),
        )
    } else {
        JsonColorPalette::default()
    }
}

impl<'a, W: Write> ResultOutput<'a, W> {
    fn with_capture(
        writer: &'a mut W,
        options: &'a RunOptions,
        capture: Option<RunTestCaptureHandle>,
    ) -> Self {
        Self {
            writer,
            options,
            native: NativeOutputSequence::new(
                NativeFormat::from_output(options.output_format)
                    .select_output(OutputOptions {
                        format: options.output_format,
                        strict_conversion: options.strict_conversion,
                        delimited_limits: tq_formats::DelimitedLimits {
                            row_bytes: options.limits.line_bytes,
                            field_bytes: options.limits.token_bytes,
                            fields: options.limits.fields,
                        },
                        pretty_json: options.pretty_json
                            && matches!(
                                options.output_format,
                                OutputFormat::Json | OutputFormat::JsonSequence
                            ),
                        json_indent: if matches!(
                            options.output_format,
                            OutputFormat::Json | OutputFormat::JsonSequence
                        ) {
                            options.json_indent
                        } else {
                            tq_formats::JsonIndent::default()
                        },
                        ascii_json: options.ascii_output,
                        color_json: options.color == ColorMode::Always,
                        color_palette: color_palette(options),
                        yaml_document_start: false,
                        toon_framing: options.framing,
                        json_sequence: options.json_sequence,
                        toon: options.toon_writer,
                    })
                    .expect("CLI output selection is validated"),
            ),
            capture,
            written: 0,
            emitted: 0,
            last_was_proxy: false,
        }
    }

    fn record(&self, value: &Value) {
        let Some(capture) = &self.capture else {
            return;
        };
        if let Ok(mut capture) = capture.lock() {
            capture.record(value);
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
        let original_value = value;
        let sorted;
        let value = if self.options.sort_keys {
            sorted = sort_value_keys(original_value);
            &sorted
        } else {
            original_value
        };
        if self.options.raw_output {
            self.emit_raw(value)?;
            self.record(original_value);
            return Ok(());
        }
        let mut writer = LimitedWriter::new(
            &mut *self.writer,
            &mut self.written,
            self.options.limits.output_bytes,
        );
        self.native.write_result(&mut writer, value)?;
        if self.options.unbuffered {
            self.writer.flush()?;
        }
        self.record(original_value);
        Ok(())
    }

    fn emit_raw(&mut self, value: &Value) -> Result<(), RunError> {
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
        if !(self.options.raw_output || self.last_was_proxy && self.emitted == 0) {
            let mut writer = LimitedWriter::new(
                &mut *self.writer,
                &mut self.written,
                self.options.limits.output_bytes,
            );
            self.native.finish(&mut writer)?;
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

#[allow(
    clippy::type_complexity,
    reason = "filter loading returns the query and optional startup source together"
)]
fn load_filter(
    options: &RunOptions,
) -> Result<(String, Vec<u8>, Option<(String, Vec<u8>)>), RunError> {
    let (identity, query) = match &options.filter {
        FilterSource::Inline(query) => ("<command-line>".to_owned(), query.as_bytes().to_vec()),
        FilterSource::File(path) => {
            let identity = path.display().to_string();
            (
                identity.clone(),
                read_limited(open_path(path)?, options.limits.input_bytes, &identity)?,
            )
        }
    };
    let startup_source =
        if options.capability_policy.environment && options.capability_policy.filesystem {
            if let Some(home) = std::env::var_os("HOME") {
                let startup = PathBuf::from(home).join(".jq");
                if startup.is_file() {
                    let startup_identity = startup.display().to_string();
                    let startup = read_limited(
                        open_path(&startup)?,
                        options.limits.input_bytes,
                        &startup_identity,
                    )?;
                    let total = startup.len().saturating_add(1).saturating_add(query.len());
                    if u64::try_from(total).unwrap_or(u64::MAX) > options.limits.input_bytes {
                        return Err(RunError::ResourceSource {
                            identity: startup_identity,
                            resource: "input-bytes",
                        });
                    }
                    Some((startup_identity, startup))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
    Ok((identity, query, startup_source))
}

fn module_roots(options: &RunOptions, query_name: &str) -> Vec<PathBuf> {
    if !options.capability_policy.filesystem {
        return Vec::new();
    }
    let origin = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf));
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let substitute = |path: &Path| {
        let text = path.to_string_lossy();
        if let Some(rest) = text.strip_prefix("$ORIGIN/") {
            return origin
                .as_ref()
                .map_or_else(|| path.to_owned(), |origin| origin.join(rest));
        }
        if let Some(rest) = text.strip_prefix("~/") {
            return home
                .as_ref()
                .map_or_else(|| path.to_owned(), |home| home.join(rest));
        }
        path.to_owned()
    };

    if !options.module_paths.is_empty() {
        return options
            .module_paths
            .iter()
            .map(|path| substitute(path))
            .collect();
    }

    let mut roots = Vec::new();
    if let FilterSource::File(path) = &options.filter {
        if let Some(parent) = path.parent() {
            roots.push(parent.to_path_buf());
        }
    } else if query_name == "<command-line>" {
        roots.push(PathBuf::from("."));
    }
    if let Some(paths) = std::env::var_os("JQ_LIBRARY_PATH") {
        roots.extend(std::env::split_paths(&paths));
    }
    if let Some(home) = &home {
        roots.push(home.join(".jq"));
    }
    if let Some(origin) = origin.clone() {
        roots.push(origin.join("../lib/jq"));
        roots.push(origin.join("../lib"));
    }
    // Ambient jq roots are optional: a missing HOME or installation path must
    // not make ordinary filters fail during module-root canonicalization.
    // Explicit `-L` roots returned above remain strict and are canonicalized
    // by the resolver so misspellings are still diagnosed.
    roots
        .into_iter()
        .map(|path| substitute(&path))
        .filter(|path| path.is_dir())
        .collect()
}

fn parse_external_arguments(options: &RunOptions) -> Result<BTreeMap<Arc<str>, Value>, RunError> {
    let mut values = BTreeMap::new();
    let mut named = tq_core::Object::new();
    for argument in &options.arguments {
        validate_external_argument_name(&argument.name)?;
        let value = match argument.kind {
            ExternalArgumentKind::String => Value::string(argument.value.as_str()),
            ExternalArgumentKind::Json => {
                decode_json_argument(argument.value.as_bytes(), "--argjson")?
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
                Value::string(String::from_utf8_lossy(&bytes).into_owned())
            }
            ExternalArgumentKind::SlurpFile => {
                let bytes = read_limited(
                    open_path(Path::new(&argument.value))?,
                    options.limits.input_bytes,
                    &argument.value,
                )?;
                let documents =
                    decode_json(&bytes, &argument.value).map_err(|error| match error {
                        FormatError::Parse { .. } => RunError::Cli(CliError::Usage(format!(
                            "--slurpfile '{}' requires valid JSON values",
                            argument.value
                        ))),
                        other => RunError::Input(other),
                    })?;
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
    let positional = options
        .positional_arguments
        .iter()
        .map(|argument| match argument.kind {
            PositionalArgumentKind::String => Ok(Value::string(argument.value.as_str())),
            PositionalArgumentKind::Json => {
                decode_json_argument(argument.value.as_bytes(), "--jsonargs")
            }
        })
        .collect::<Result<Vec<_>, RunError>>()?;
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
        let environment = Value::object(environment);
        values.insert(Arc::from(AMBIENT_ENVIRONMENT), environment.clone());
        values.insert(Arc::from("ENV"), environment);
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
        Value::Number(
            Number::parse(if options.null_input { "0" } else { "1" })
                .expect("an admitted source line number"),
        ),
    );
    Ok(values)
}

fn decode_json_argument(bytes: &[u8], option: &str) -> Result<Value, RunError> {
    decode_single_json(bytes, option).map_err(|error| match error {
        RunError::Input(FormatError::Parse { .. }) => RunError::Cli(CliError::Usage(format!(
            "{option} requires one valid JSON value"
        ))),
        other => other,
    })
}

fn validate_external_argument_name(name: &str) -> Result<(), RunError> {
    if name.starts_with("__tq_") {
        return Err(CliError::Usage(
            "external variable names beginning with '__tq_' are reserved".to_owned(),
        )
        .into());
    }
    Ok(())
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
    Failure(tq_formats::NativeInputFailure),
    Error(VmError),
    Done,
}

fn produce_remaining_inputs<R: Read>(
    options: &RunOptions,
    stdin: &mut R,
    requests: &Receiver<()>,
    sender: &SyncSender<RemainingInputMessage>,
) {
    let result = produce_remaining_inputs_inner(options, stdin, requests, sender);
    let message = result.map_or_else(
        |error| RemainingInputMessage::Error(deferred_run_error(error)),
        |()| RemainingInputMessage::Done,
    );
    let _ = sender.send(message);
}

fn produce_remaining_inputs_inner<R: Read>(
    options: &RunOptions,
    stdin: &mut R,
    requests: &Receiver<()>,
    sender: &SyncSender<RemainingInputMessage>,
) -> Result<(), RunError> {
    let mut pending_request = false;
    let files = if options.files.is_empty() {
        vec![Path::new("-").to_owned()]
    } else {
        options.files.clone()
    };
    for path in files {
        if path == Path::new("-") {
            pending_request = produce_remaining_reader(
                options,
                selected_input_format(options, &path),
                &mut *stdin,
                "<stdin>",
                requests,
                sender,
                pending_request,
            )?;
        } else {
            let identity = path.display().to_string();
            pending_request = produce_remaining_reader(
                options,
                selected_input_format(options, &path),
                open_path(&path)?,
                &identity,
                requests,
                sender,
                pending_request,
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
    requests: &Receiver<()>,
    sender: &SyncSender<RemainingInputMessage>,
    pending_request: bool,
) -> Result<bool, RunError> {
    let reader = LimitedReader::new(reader, options.limits.input_bytes, identity);
    if requested == InputFormat::Auto {
        let pending_request = if pending_request {
            true
        } else {
            if requests.recv().is_err() {
                return Ok(false);
            }
            true
        };
        let (report, replay) = probe_reader(reader, options.limits.lookahead_bytes)?;
        return produce_committed_remaining_reader(
            options,
            report.selected,
            replay,
            identity,
            requests,
            sender,
            pending_request,
        );
    }
    produce_committed_remaining_reader(
        options,
        requested,
        reader,
        identity,
        requests,
        sender,
        pending_request,
    )
}

fn produce_committed_remaining_reader<R: Read>(
    options: &RunOptions,
    requested: InputFormat,
    reader: R,
    identity: &str,
    requests: &Receiver<()>,
    sender: &SyncSender<RemainingInputMessage>,
    mut pending_request: bool,
) -> Result<bool, RunError> {
    let format = requested;
    if options.stream {
        if !pending_request && requests.recv().is_err() {
            return Ok(false);
        }
        let mut index = 0_u64;
        return project_committed_input(options, format, reader, identity, &mut |input| {
            let message = match input {
                StructuredInput::Value(value) => {
                    index = index.saturating_add(1);
                    RemainingInputMessage::Value(InputValue {
                        value,
                        identity: Arc::from(identity),
                        line_number: index,
                    })
                }
                StructuredInput::Failure(failure) => RemainingInputMessage::Failure(failure),
                _ => unreachable!("native projection supplies values and failures"),
            };
            // Wait for demand before decoding further. The pending request
            // also belongs to the next source or its final EOF/error message.
            Ok(sender.send(message).is_ok() && requests.recv().is_ok())
        });
    }
    let selection = NativeFormat::from_input(format)
        .expect("input is committed")
        .select_input(
            decode_options(options, format),
            InputRepresentation::Documents,
        )?;
    let mut source = selection.open(reader, identity);
    loop {
        if !pending_request {
            if requests.recv().is_err() {
                return Ok(false);
            }
            pending_request = true;
        }
        let Some(observation) = source.next_observation()? else {
            break;
        };
        let keep_going = match observation {
            NativeInputObservation::Document(document) => document,
            NativeInputObservation::Failure(failure) => {
                let keep_going = sender.send(RemainingInputMessage::Failure(failure)).is_ok();
                pending_request = false;
                if !keep_going {
                    return Ok(false);
                }
                continue;
            }
            NativeInputObservation::Event(_) => unreachable!("Document representation"),
        };
        if !send_remaining(sender, keep_going) {
            return Ok(false);
        }
        pending_request = false;
    }
    Ok(pending_request)
}

fn send_remaining(
    sender: &SyncSender<RemainingInputMessage>,
    document: tq_formats::Document,
) -> bool {
    sender
        .send(RemainingInputMessage::Value(InputValue {
            value: document.value,
            identity: Arc::from(document.identity),
            line_number: document.line_number,
        }))
        .is_ok()
}

fn pull_remaining_input(cursor: &InputCursor) -> Result<Option<StructuredInput>, RunError> {
    match cursor.next_value() {
        Ok(value) => Ok(value.map(StructuredInput::Value)),
        Err(VmError::RecoverableInput { message, context }) => {
            Ok(Some(StructuredInput::Warning { message, context }))
        }
        Err(VmError::Input { message }) => Err(RunError::Input(FormatError::Parse {
            format: InputFormat::Auto,
            message: message.to_string(),
        })),
        Err(error) => Err(RunError::Runtime(error)),
    }
}

fn deferred_run_error(error: RunError) -> VmError {
    match error {
        RunError::Runtime(error) | RunError::ReportedRuntime(error) => error,
        RunError::Resource(resource) | RunError::Input(FormatError::Resource(resource)) => {
            VmError::Resource { resource }
        }
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

fn load_inputs<R: Read>(
    options: &RunOptions,
    stdin: &mut R,
    recover_json_sequence: bool,
    diagnostics: &mut Vec<String>,
) -> Result<LoadedInputs, RunError> {
    if options.null_input {
        return Ok(LoadedInputs::Documents(vec![tq_formats::Document {
            value: Value::Null,
            identity: "<null-input>".to_owned(),
            format: InputFormat::Auto,
            index: 0,
            line_number: 0,
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
            let format = selected_input_format(options, &path);
            let selected = format;
            if recover_json_sequence && selected == InputFormat::JsonSequence {
                let (decoded, errors) =
                    decode_json_sequence_recoverable(&bytes, &identity, options)?;
                documents.extend(decoded);
                diagnostics.extend(errors);
                if options.proxy_on_error {
                    raw_sources.push(bytes);
                }
                continue;
            }
            match native_source_documents(&bytes, &identity, options, format) {
                Ok((committed, _)) if options.stream => {
                    project_committed_input(
                        options,
                        committed,
                        bytes.as_slice(),
                        &identity,
                        &mut |input| {
                            let StructuredInput::Value(value) = input else {
                                unreachable!("validated source");
                            };
                            documents.push(tq_formats::Document {
                                value,
                                identity: identity.clone(),
                                format: committed,
                                index: u64::try_from(documents.len()).unwrap_or(u64::MAX),
                                line_number: u64::try_from(documents.len())
                                    .unwrap_or(u64::MAX)
                                    .saturating_add(1),
                            });
                            Ok(true)
                        },
                    )?;
                }
                Ok((_, decoded)) => documents.extend(decoded),
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

fn decode_json_sequence_recoverable(
    bytes: &[u8],
    identity: &str,
    options: &RunOptions,
) -> Result<(Vec<tq_formats::Document>, Vec<String>), RunError> {
    let selection = NativeFormat::from_input(InputFormat::JsonSequence)
        .expect("JSON sequence input is a native format")
        .select_input(
            decode_options(options, InputFormat::JsonSequence),
            InputRepresentation::Documents,
        )?;
    let mut source = selection.open(bytes, identity);
    let mut documents = Vec::new();
    let mut diagnostics = Vec::new();
    while let Some(observation) = source.next_observation()? {
        match observation {
            NativeInputObservation::Document(document) => documents.push(document),
            NativeInputObservation::Failure(failure) => {
                diagnostics.push(format!("JsonSequence input rejected: {}", failure.message));
            }
            NativeInputObservation::Event(_) => unreachable!("Document representation"),
        }
    }
    Ok((documents, diagnostics))
}

fn json_input_options(options: &RunOptions) -> JsonInputOptions {
    JsonInputOptions {
        maximum_depth: options.limits.depth,
        maximum_token_bytes: options.limits.token_bytes,
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

fn format_from_path(path: &Path) -> Option<InputFormat> {
    let extension = path.extension()?.to_str()?;
    tq_formats::NativeFormat::from_extension(extension).map(|format| format.descriptor().input)
}

fn selected_input_format(options: &RunOptions, path: &Path) -> InputFormat {
    if json_sequence_input_requested(options) {
        return InputFormat::JsonSequence;
    }
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
        maximum_frame_bytes: options.limits.frame_bytes,
        maximum_depth: options.limits.depth,
        maximum_token_bytes: options.limits.token_bytes,
        maximum_line_bytes: options.limits.line_bytes,
        maximum_fields: options.limits.fields,
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
            line_number: 1,
        });
        return Ok(());
    }
    for (index, line) in text.lines().enumerate() {
        documents.push(tq_formats::Document {
            value: Value::string(line.trim_end_matches('\r')),
            identity: identity.clone(),
            format: InputFormat::Auto,
            index: index as u64,
            line_number: index as u64 + 1,
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
            "input_projection": if options.stream { "jq-stream" } else { "documents" },
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
                "runtime_spool_fixed_io_bytes": retention.runtime_spool_fixed_io_bytes_high_water,
                "root_staging": {
                    "encoded_bytes_high_water": retention.root_staging_encoded_bytes_high_water,
                    "memory_bytes_high_water": retention.root_staging_memory_bytes_high_water,
                    "index_bytes_high_water": retention.root_staging_index_bytes_high_water,
                    "spool_bytes_written": retention.root_staging_spool_bytes_written_high_water,
                    "fixed_io_bytes_high_water": retention.runtime_spool_fixed_io_bytes_high_water,
                },
                "root_materialized": !matches!(plan, PlanKind::Events | PlanKind::Subtree | PlanKind::HybridBlocking | PlanKind::Transcode),
            },
            "resource_outcome": resource_outcome,
        },
        "limits": {
            "input_bytes": options.limits.input_bytes,
            "depth": options.limits.depth,
            "token_bytes": options.limits.token_bytes,
            "line_bytes": options.limits.line_bytes,
            "frame_bytes": options.limits.frame_bytes,
            "fields": options.limits.fields,
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
            "frame_bytes": options.limits.frame_bytes,
            "fields": options.limits.fields,
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
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
            mpsc::sync_channel,
        },
        thread,
        time::Duration,
    };

    use super::{
        RunTestCapture, RunTestCaptureSnapshot, prune_object_fields_in_place,
        record_document_observations, run_test_output_matches, run_with_io,
    };
    use crate::{CapabilityPolicy, Command, ExecutionOverride, ExitStatus, parse_args};
    use tq_core::{PathComponent, Value, VmObservations};

    struct NoRead;

    impl Read for NoRead {
        fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
            panic!("compile failure must not read input")
        }
    }

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

    fn assert_single_runtime_diagnostic(stderr: &[u8]) {
        let stderr = std::str::from_utf8(stderr).expect("runtime diagnostic is UTF-8");
        assert!(stderr.ends_with('\n'));
        let mut lines = stderr.lines();
        let diagnostic = lines.next().expect("runtime diagnostic is non-empty");
        assert!(lines.next().is_none(), "expected one diagnostic line");
        assert!(diagnostic.starts_with("tq: runtime error:"));
    }

    fn observed_exit_status(status: &Result<ExitStatus, super::RunError>) -> ExitStatus {
        status
            .as_ref()
            .map_or_else(super::RunError::status, |status| *status)
    }

    fn observation_options(report: bool, stream: bool) -> super::RunOptions {
        let command = parse_args(["."]).expect("baseline options parse");
        let Command::Run(mut options) = command else {
            panic!("expected run command")
        };
        options.report_file = report.then(|| std::path::PathBuf::from("unit-test-report.json"));
        options.stream = stream;
        *options
    }

    #[test]
    fn document_observations_are_retained_only_for_reports() {
        let item = VmObservations {
            value_stack_high_water: 2,
            call_stack_high_water: 3,
            path_stack_high_water: 4,
            fork_stack_high_water: 5,
            steps: 6,
            results: 7,
        };
        let mut observations = Vec::new();
        record_document_observations(&observation_options(false, false), &mut observations, item);
        assert!(observations.is_empty());

        let mut observations = Vec::new();
        record_document_observations(&observation_options(true, false), &mut observations, item);
        record_document_observations(
            &observation_options(true, false),
            &mut observations,
            VmObservations {
                steps: 8,
                results: 9,
                ..VmObservations::default()
            },
        );
        assert_eq!(
            observations,
            vec![
                item,
                VmObservations {
                    steps: 8,
                    results: 9,
                    ..VmObservations::default()
                }
            ]
        );
    }

    #[test]
    fn streamed_report_observations_merge_work_and_high_water_fields() {
        let mut observations = vec![VmObservations {
            value_stack_high_water: 2,
            call_stack_high_water: 3,
            path_stack_high_water: 4,
            fork_stack_high_water: 5,
            steps: 6,
            results: 7,
        }];
        record_document_observations(
            &observation_options(true, true),
            &mut observations,
            VmObservations {
                value_stack_high_water: 8,
                call_stack_high_water: 1,
                path_stack_high_water: 9,
                fork_stack_high_water: 0,
                steps: 10,
                results: 11,
            },
        );
        assert_eq!(
            observations,
            vec![VmObservations {
                value_stack_high_water: 8,
                call_stack_high_water: 3,
                path_stack_high_water: 9,
                fork_stack_high_water: 5,
                steps: 16,
                results: 18,
            }]
        );
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
    fn run_test_matcher_requires_typed_capture() {
        let expected = vec!["null".to_owned()];
        assert!(!run_test_output_matches(b"null\n", &expected, None));
    }

    #[test]
    fn run_test_matcher_rejects_overflowed_capture() {
        let expected = vec!["null".to_owned()];
        let capture = RunTestCaptureSnapshot {
            values: vec![Value::Null],
            overflowed: true,
        };
        assert!(!run_test_output_matches(
            b"null\n",
            &expected,
            Some(&capture)
        ));
    }

    #[test]
    fn run_test_capture_enforces_result_count_bound() {
        let mut capture = RunTestCapture::new(0);
        capture.record(&Value::Null);
        capture.record(&Value::Null);
        assert!(capture.overflowed);
        assert_eq!(capture.values, vec![Value::Null]);
    }

    #[test]
    fn run_test_matcher_accepts_a_captured_compact_value() {
        let expected = vec!["null".to_owned()];
        let capture = RunTestCaptureSnapshot {
            values: vec![Value::Null],
            overflowed: false,
        };
        assert!(run_test_output_matches(
            b"null\n",
            &expected,
            Some(&capture)
        ));
    }

    #[test]
    fn object_capture_pruning_is_in_place_and_keeps_required_order() {
        let mut properties = tq_core::Object::new();
        properties.insert("mag".into(), Value::string("3"));
        properties.insert("unused".into(), Value::string("drop"));
        let mut object = tq_core::Object::new();
        object.insert("id".into(), Value::string("feature"));
        object.insert("properties".into(), Value::object(properties));
        object.insert("unused".into(), Value::string("drop"));
        let mut value = Value::object(object);
        let root_ptr = match &value {
            Value::Object(values) => Arc::as_ptr(values),
            _ => unreachable!("test value is an object"),
        };
        let paths = vec![
            vec![
                PathComponent::Key("properties".into()),
                PathComponent::Key("mag".into()),
            ],
            vec![PathComponent::Key("id".into())],
        ];

        prune_object_fields_in_place(&mut value, &paths);

        let Value::Object(values) = &value else {
            unreachable!("pruned value is an object")
        };
        assert_eq!(Arc::as_ptr(values), root_ptr);
        assert_eq!(
            values.keys().map(AsRef::as_ref).collect::<Vec<&str>>(),
            ["id", "properties"]
        );
        let Value::Object(properties) = values.get("properties").expect("properties retained")
        else {
            unreachable!("properties remains an object")
        };
        assert_eq!(
            properties.keys().map(AsRef::as_ref).collect::<Vec<&str>>(),
            ["mag"]
        );

        let shared = value.clone();
        let mut shared_copy = value;
        prune_object_fields_in_place(&mut shared_copy, &[vec![PathComponent::Key("id".into())]]);
        let Value::Object(values) = &shared_copy else {
            unreachable!("shared value is an object")
        };
        assert!(values.contains_key("properties"));
        assert!(shared.shares_node_with(&shared_copy));
    }

    #[test]
    fn object_capture_pruning_keeps_sibling_paths_separate() {
        let mut left = tq_core::Object::new();
        left.insert("x".into(), Value::string("left-x"));
        left.insert("y".into(), Value::string("left-y"));
        let mut right = tq_core::Object::new();
        right.insert("x".into(), Value::string("right-x"));
        right.insert("y".into(), Value::string("right-y"));
        let mut object = tq_core::Object::new();
        object.insert("a".into(), Value::object(left));
        object.insert("b".into(), Value::object(right));
        let mut value = Value::object(object);

        prune_object_fields_in_place(
            &mut value,
            &[
                vec![
                    PathComponent::Key("a".into()),
                    PathComponent::Key("x".into()),
                ],
                vec![
                    PathComponent::Key("b".into()),
                    PathComponent::Key("y".into()),
                ],
            ],
        );

        let Value::Object(values) = &value else {
            unreachable!("pruned value is an object")
        };
        let Value::Object(left) = values.get("a").expect("left branch retained") else {
            unreachable!("left branch remains an object")
        };
        let Value::Object(right) = values.get("b").expect("right branch retained") else {
            unreachable!("right branch remains an object")
        };
        assert_eq!(left.keys().map(AsRef::as_ref).collect::<Vec<&str>>(), ["x"]);
        assert_eq!(
            right.keys().map(AsRef::as_ref).collect::<Vec<&str>>(),
            ["y"]
        );

        let mut complete_left = tq_core::Object::new();
        complete_left.insert("x".into(), Value::string("left-x"));
        complete_left.insert("y".into(), Value::string("left-y"));
        let mut complete_root = tq_core::Object::new();
        complete_root.insert("a".into(), Value::object(complete_left));
        complete_root.insert("b".into(), Value::string("remove"));
        let mut complete = Value::object(complete_root);
        prune_object_fields_in_place(
            &mut complete,
            &[
                vec![PathComponent::Key("a".into())],
                vec![
                    PathComponent::Key("a".into()),
                    PathComponent::Key("x".into()),
                ],
            ],
        );
        let Value::Object(values) = complete else {
            unreachable!("terminal-path value is an object")
        };
        let Value::Object(left) = values.get("a").expect("terminal branch retained") else {
            unreachable!("terminal branch remains an object")
        };
        assert_eq!(
            left.keys().map(AsRef::as_ref).collect::<Vec<&str>>(),
            ["x", "y"]
        );
    }

    #[test]
    fn user_function_executes_inside_map_before_cli_output() {
        let (status, output, error) = execute(
            &[
                "--input-format",
                "json",
                "--output-format",
                "json",
                "--compact-output",
                "def f: . + 1; map(f)",
            ],
            b"[1,2]\n",
        );
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(output, b"[2,3]\n");
        assert!(error.is_empty());
    }

    #[test]
    fn json5_input_runs_explicitly_and_by_extension_without_weakening_json() {
        let (status, output, error) = execute(
            &[
                "--input-format",
                "json5",
                "--output-format",
                "json",
                "--compact-output",
                "--explain-json",
                ".value",
            ],
            b"{value: 'ok',}",
        );
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(output, b"\"ok\"\n");
        let explanation: serde_json::Value = serde_json::from_slice(&error).unwrap();
        assert_eq!(explanation["execution"]["plan"], "document");

        let directory = tempfile::tempdir().unwrap();
        let json5 = directory.path().join("first.JSON5");
        let json = directory.path().join("second.json");
        let permissive_json = directory.path().join("strict.json");
        fs::write(&json5, b"{value: 1,}").unwrap();
        fs::write(&json, br#"{"value":2}"#).unwrap();
        fs::write(&permissive_json, b"{value: 3,}").unwrap();

        let paths = [json5.to_str().unwrap(), json.to_str().unwrap()];
        let (status, output, error) = execute(
            &["--output-format", "jsonl", ".value", paths[0], paths[1]],
            b"",
        );
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(output, b"1\n2\n");
        assert!(error.is_empty());

        let strict_path = permissive_json.to_str().unwrap();
        let (status, output, _) = execute(&[".", strict_path], b"");
        assert_eq!(status.unwrap_err().status(), ExitStatus::Input);
        assert!(output.is_empty());

        let (status, output, error) = execute(
            &[
                "--input-format",
                "json5",
                "--output-format",
                "json",
                "--compact-output",
                ".value",
                strict_path,
            ],
            b"",
        );
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(output, b"3\n");
        assert!(error.is_empty());
    }

    #[test]
    fn json5_extension_rejects_stream_mode_before_opening_input() {
        let directory = tempfile::tempdir().unwrap();
        let missing = directory.path().join("missing.json5");
        let command = parse_args(["--stream", ".", missing.to_str().unwrap()]).unwrap();
        let mut input = NoRead;
        let mut output = Vec::new();
        let mut error = Vec::new();

        let failure = run_with_io(command, &mut input, &mut output, &mut error).unwrap_err();

        assert_eq!(failure.status(), ExitStatus::Unsupported);
        assert!(failure.to_string().contains("JSON5"));
        assert!(failure.to_string().contains("document-at-a-time"));
        assert!(output.is_empty());
    }

    #[test]
    fn unknown_filter_fails_before_input() {
        let command = parse_args([
            "--input-format",
            "json",
            "--output-format",
            "json",
            "def f: .; .[0:1] | unknown_filter",
        ])
        .unwrap();
        let mut input = NoRead;
        let mut output = Vec::new();
        let mut error = Vec::new();
        let failure = run_with_io(command, &mut input, &mut output, &mut error).unwrap_err();
        assert_eq!(failure.status(), ExitStatus::Compile);
        assert!(failure.to_string().contains("TQ-RESOLVE-BUILTIN-001"));
        assert!(
            failure
                .to_string()
                .contains("unknown filter unknown_filter/0")
        );
        assert!(output.is_empty());
    }

    #[test]
    fn executable_user_function_closure_runs_after_slice() {
        let (status, output, error) = execute(
            &[
                "--input-format",
                "json",
                "--output-format",
                "json",
                "--compact-output",
                "def f: .; .[0:1] | f",
            ],
            b"[1,2]\n",
        );
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(output, b"[1]\n");
        assert!(error.is_empty());
    }

    #[test]
    fn generator_bound_report_input_preserves_cli_assignment_path() {
        let (status, output, error) = execute(
            &[
                "--input-format",
                "json",
                "--output-format",
                "json",
                "--compact-output",
                r". as $approvals
                | input as $report
                | $approvals
                | map(
                    . as $approval
                    | ($report.cases[] | select(.id == $approval.case_id)) as $case
                    | .evidence = {x: 1}
                  )",
            ],
            b"[{\"case_id\":\"x\"}]\n{\"cases\":[{\"id\":\"x\"}]}\n",
        );
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(output, b"[{\"case_id\":\"x\",\"evidence\":{\"x\":1}}]\n");
        assert!(error.is_empty());
    }

    #[test]
    fn generator_bound_report_input_preserves_cli_slice_and_filter_bindings() {
        let (status, output, error) = execute(
            &[
                "--input-format",
                "json",
                "--output-format",
                "json",
                "--compact-output",
                r". as $approvals
                   | input as $report
                   | def find(predicate):
                       $report.cases[0:1][] | select(predicate)
                     ;
                   $approvals
                   | map(
                       . as $approval
                       | find(.id == $approval.case_id) as $case
                       | .evidence = $case.value
                     )",
            ],
            b"[{\"case_id\":\"x\"}]\n{\"cases\":[{\"id\":\"x\",\"value\":1}]}\n",
        );
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(output, b"[{\"case_id\":\"x\",\"evidence\":1}]\n");
        assert!(error.is_empty());
    }

    #[test]
    fn internal_override_forces_document_plan_without_changing_output() {
        let arguments = ["--output-format", "toon-seq", "--explain-json", ".[]"];
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
    fn identity_json_transcode_rejects_a_late_duplicate_without_partial_output() {
        let (status, output, explain) = execute(
            &["--input-format", "json", "--explain-json", "."],
            br#"{"b":1,"a":2,"b":3}"#,
        );
        assert_eq!(status.unwrap_err().status(), ExitStatus::Input);
        assert_eq!(output, [] as [u8; 0]);
        let explain: serde_json::Value = serde_json::from_slice(&explain).unwrap();
        assert_eq!(explain["execution"]["plan"], "transcode");
        assert_eq!(explain["execution"]["duplicate_policy"], "reject");
        assert_eq!(explain["execution"]["commitment_mode"], "direct-values");
    }

    #[test]
    fn transcode_streams_ordered_files_and_keeps_unframed_output_atomic() {
        let directory = tempfile::tempdir().unwrap();
        let first = directory.path().join("first.json");
        let second = directory.path().join("second.json");
        fs::write(&first, b"1").unwrap();
        fs::write(&second, b"2").unwrap();
        let command = parse_args([
            "--output-format",
            "toon-seq",
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

        let command = parse_args([
            "--input-format",
            "json",
            "--output-format",
            "json",
            "--max-output-bytes",
            "1",
            ".",
        ])
        .unwrap();
        let mut input = b"10".as_slice();
        let mut output = Vec::new();
        let mut error = Vec::new();
        assert_eq!(
            run_with_io(command, &mut input, &mut output, &mut error)
                .unwrap_err()
                .status(),
            ExitStatus::Resource
        );
        assert!(output.is_empty());

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

        let (allowed_variable, output, error) = execute(
            &[
                "--allow-environment",
                "--output-format",
                "json",
                "-c",
                "$ENV | type",
            ],
            b"null\n",
        );
        assert_eq!(allowed_variable.unwrap(), ExitStatus::Success);
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

    #[test]
    fn jq_special_variables_follow_cli_source_and_capability_contracts() {
        let (denied, output, error) =
            execute(&["--output-format", "json", "-c", "$ENV"], b"null\n");
        let denied = denied.expect_err("environment access should be denied by default");
        assert_eq!(denied.status(), ExitStatus::Runtime);
        assert!(denied.to_string().contains("capability policy"));
        assert!(!denied.to_string().contains("__tq_"));
        assert_eq!(output, [] as [u8; 0]);
        assert_eq!(error, [] as [u8; 0]);

        let (allowed, output, error) = execute(
            &[
                "--allow-environment",
                "--arg",
                "ENV",
                "replacement",
                "--output-format",
                "json",
                "-c",
                "$ENV | type",
            ],
            b"null\n",
        );
        assert_eq!(allowed.unwrap(), ExitStatus::Success);
        assert_eq!(output, b"\"object\"\n");
        assert_eq!(error, [] as [u8; 0]);

        let (location, output, error) = execute(
            &[
                "--arg",
                "__loc__",
                "replacement",
                "--output-format",
                "json",
                "-c",
                "$__loc__",
            ],
            b"null\n",
        );
        assert_eq!(location.unwrap(), ExitStatus::Success);
        assert_eq!(output, b"{\"file\":\"<top-level>\",\"line\":1}\n");
        assert_eq!(error, [] as [u8; 0]);

        let directory = tempfile::tempdir().unwrap();
        let filter = directory.path().join("location.jq");
        fs::write(&filter, b"\n$__loc__\n").unwrap();
        let (location, output, error) = execute(
            &[
                "--null-input",
                "--output-format",
                "json",
                "-c",
                "--from-file",
                filter.to_str().unwrap(),
            ],
            b"",
        );
        assert_eq!(location.unwrap(), ExitStatus::Success);
        let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(value["file"], filter.display().to_string());
        assert_eq!(value["line"], 2);
        assert_eq!(error, [] as [u8; 0]);

        let module = directory.path().join("location.jq");
        fs::write(&module, b"def module_location:\n  $__loc__;\n").unwrap();
        let (location, output, error) = execute(
            &[
                "--null-input",
                "--library-path",
                directory.path().to_str().unwrap(),
                "--output-format",
                "json",
                "-c",
                "include \"location\"; module_location",
            ],
            b"",
        );
        assert_eq!(location.unwrap(), ExitStatus::Success);
        let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(
            value["file"],
            module.canonicalize().unwrap().display().to_string()
        );
        assert_eq!(value["line"], 2);
        assert_eq!(error, [] as [u8; 0]);

        let (stable, output, error) = execute(
            &[
                "--allow-environment",
                "--input-format",
                "json",
                "--output-format",
                "json",
                "-c",
                "[$__loc__, ($ENV | type)]",
            ],
            b"null null",
        );
        assert_eq!(stable.unwrap(), ExitStatus::Success);
        assert_eq!(
            output,
            b"[{\"file\":\"<top-level>\",\"line\":1},\"object\"]\n[{\"file\":\"<top-level>\",\"line\":1},\"object\"]\n"
        );
        assert_eq!(error, [] as [u8; 0]);
    }

    #[test]
    fn jq_special_variables_match_automatic_and_document_plans() {
        let arguments = [
            "--allow-environment",
            "--input-format",
            "json",
            "--output-format",
            "json",
            "-c",
            ".items[] | [$__loc__, ($ENV | type)]",
        ];
        let input = br#"{"items":[null,null]}"#;
        let automatic = execute_with_override(&arguments, input, ExecutionOverride::Automatic);
        let document = execute_with_override(&arguments, input, ExecutionOverride::Document);
        assert_eq!(automatic.0.unwrap(), ExitStatus::Success);
        assert_eq!(document.0.unwrap(), ExitStatus::Success);
        assert_eq!(automatic.1, document.1);
        assert_eq!(
            automatic.1,
            b"[{\"file\":\"<top-level>\",\"line\":1},\"object\"]\n[{\"file\":\"<top-level>\",\"line\":1},\"object\"]\n"
        );
        assert_eq!(automatic.2, [] as [u8; 0]);
        assert_eq!(document.2, [] as [u8; 0]);

        let denied_arguments = [
            "--input-format",
            "json",
            "--output-format",
            "json",
            "-c",
            ".items[] | $ENV",
        ];
        let automatic =
            execute_with_override(&denied_arguments, input, ExecutionOverride::Automatic);
        let document = execute_with_override(&denied_arguments, input, ExecutionOverride::Document);
        assert_eq!(
            automatic.0.unwrap_err().status(),
            document.0.unwrap_err().status()
        );
        assert_eq!(automatic.1, document.1);
        assert_eq!(automatic.2, [] as [u8; 0]);
        assert_eq!(document.2, [] as [u8; 0]);
    }

    #[test]
    fn external_arguments_cannot_supply_internal_ambient_values() {
        let (status, output, error) = execute(
            &[
                "--argjson",
                "__tq_ambient_environment",
                r#"{"FORGED":"value"}"#,
                "--output-format",
                "json",
                "-c",
                "$ENV",
            ],
            b"null\n",
        );
        let failure = status.expect_err("internal argument names must be rejected");
        assert_ne!(failure.status(), ExitStatus::Success);
        assert_eq!(output, [] as [u8; 0]);
        assert_eq!(error, [] as [u8; 0]);
    }

    #[test]
    fn environment_policy_denial_precedes_input_for_env_variable() {
        let mut command = parse_args(["--allow-environment", "$ENV"]).unwrap();
        let Command::Run(options) = &mut command else {
            panic!("expected run command")
        };
        options.capability_policy = CapabilityPolicy {
            environment: false,
            ..CapabilityPolicy::default()
        };
        let mut input = NoRead;
        let mut output = Vec::new();
        let mut error = Vec::new();
        let failure = run_with_io(command, &mut input, &mut output, &mut error).unwrap_err();
        assert_eq!(failure.status(), ExitStatus::Usage);
        assert!(
            failure
                .to_string()
                .contains("environment access is disabled")
        );
        assert_eq!(output, [] as [u8; 0]);
        assert_eq!(error, [] as [u8; 0]);
    }

    #[test]
    fn embedded_run_tests_keeps_ambient_policy_confined() {
        let command = Command::RunTests(None);
        let mut input = b"$ENV.TQ_EMBEDDED_RUN_TEST_SENTINEL\nnull\n\"present\"\n\n".as_slice();
        let mut output = Vec::new();
        let mut error = Vec::new();
        let status = run_with_io(command, &mut input, &mut output, &mut error).unwrap();
        assert_eq!(status, ExitStatus::FalseOrNull);
        assert!(String::from_utf8_lossy(&output).contains("0 of 1 tests passed"));
        assert!(String::from_utf8_lossy(&output).contains("Expected \"present\""));
        assert!(error.is_empty());
    }

    #[test]
    fn denied_filesystem_cannot_use_implicit_module_or_startup_roots() {
        for policy in [
            crate::CapabilityPolicy {
                filesystem: false,
                ..crate::CapabilityPolicy::default()
            },
            crate::CapabilityPolicy {
                filesystem: false,
                environment: false,
                terminal: false,
                platform: false,
            },
        ] {
            let mut command = parse_args([
                "-n",
                "--output-format",
                "json",
                r#"import "basic" as b; b::value"#,
            ])
            .expect("parse implicit module policy query");
            let Command::Run(options) = &mut command else {
                panic!("expected run command")
            };
            options.capability_policy = policy;
            let mut input = Cursor::new(Vec::<u8>::new());
            let mut output = Vec::new();
            let mut error = Vec::new();
            let result = run_with_io(command, &mut input, &mut output, &mut error)
                .expect_err("implicit module roots must be denied");
            assert!(result.to_string().contains("TQ-MODULE-ROOT-001"));
            assert!(output.is_empty());
            assert!(error.is_empty());
        }
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

    struct FailingFlushWriter {
        bytes: Vec<u8>,
    }

    impl Write for FailingFlushWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            self.bytes.extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("injected flush failure"))
        }
    }

    struct OpenReader {
        entered: Arc<AtomicBool>,
        release: Arc<AtomicBool>,
    }

    impl Read for OpenReader {
        fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
            self.entered.store(true, Ordering::Release);
            while !self.release.load(Ordering::Acquire) {
                thread::sleep(Duration::from_millis(1));
            }
            Ok(0)
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
    fn identity_uses_lf_terminated_toon_and_keeps_stderr_clean() {
        let (status, stdout, stderr) = execute(&["."], b"name: Ada");
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(stdout, b"name: Ada\n");
        assert_eq!(stderr, [] as [u8; 0]);
    }

    #[test]
    fn transcode_flushes_only_after_the_sequence_record_is_complete() {
        let command = parse_args(["--output-format", "toon-seq", "."]).unwrap();
        let mut input = br#"{"name":"Ada"}"#.as_slice();
        let mut output = FlushWriter::default();
        let mut error = Vec::new();
        assert_eq!(
            run_with_io(command, &mut input, &mut output, &mut error).unwrap(),
            ExitStatus::Success
        );
        assert_eq!(output.bytes, b"\x1ename: Ada\n");
        assert_eq!(output.flush_points.first(), Some(&output.bytes.len()));
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

        let (status, _, _) = execute(&["-n", "--output-format", "json", "-e", "empty"], b"");
        assert_eq!(status.unwrap(), ExitStatus::NoResult);
    }

    #[test]
    fn external_values_compile_and_execute() {
        let (status, stdout, _) = execute(&["-n", "--argjson", "n", "42", "$n"], b"");
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(stdout, b"42\n");
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
    fn unbuffered_effect_flush_failure_cancels_before_open_input_read() {
        let command = parse_args(["-n", "--unbuffered", "-c", "debug, inputs"]).unwrap();
        let entered = Arc::new(AtomicBool::new(false));
        let release = Arc::new(AtomicBool::new(false));
        let reader = OpenReader {
            entered: Arc::clone(&entered),
            release: Arc::clone(&release),
        };
        let (done_sender, done_receiver) = sync_channel(1);
        let worker = thread::spawn(move || {
            let mut stdin = reader;
            let mut stdout = Vec::new();
            let mut stderr = FailingFlushWriter { bytes: Vec::new() };
            let failed = run_with_io(command, &mut stdin, &mut stdout, &mut stderr).is_err();
            done_sender
                .send(failed)
                .expect("test receiver remains alive");
        });

        let Ok(failed) = done_receiver.recv_timeout(Duration::from_secs(2)) else {
            release.store(true, Ordering::Release);
            let _ = worker.join();
            panic!("flush failure did not cancel the open-input worker")
        };
        assert!(failed);
        assert!(
            !entered.load(Ordering::Acquire),
            "worker read open input before flush failure was acknowledged"
        );
        release.store(true, Ordering::Release);
        worker.join().expect("flush-failure worker should exit");
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
        let (status, stdout, stderr) = execute(
            &["-n", "--output-format", "toon-seq", "1, error(\"later\")"],
            b"",
        );
        assert!(matches!(status, Err(super::RunError::Runtime(_))));
        assert_eq!(stdout, b"\x1e1\n");
        assert_eq!(stderr, [] as [u8; 0]);
    }

    #[test]
    fn document_roots_continue_after_recoverable_runtime_errors() {
        let (status, stdout, stderr) = execute_with_override(
            &["-ijson", "-ojson", "-c", ". + 1"],
            b"1\n\"x\"\n2\n",
            ExecutionOverride::Document,
        );
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(stdout, b"2\n3\n");
        assert_single_runtime_diagnostic(&stderr);
    }

    #[test]
    fn raw_input_lines_continue_after_recoverable_runtime_errors() {
        let (status, stdout, stderr) = execute_with_override(
            &[
                "-R",
                "-ojson",
                "-c",
                "if . == \"x\" then error(\"bad\") else . end",
            ],
            b"a\nx\nb\n",
            ExecutionOverride::Document,
        );
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(stdout, b"\"a\"\n\"b\"\n");
        assert_eq!(stderr, b"tq: runtime error: bad\n");
    }

    #[test]
    fn document_runtime_diagnostics_preserve_effect_order_and_final_status() {
        let (status, stdout, stderr) = execute_with_override(
            [
                "-ijson",
                "-ojson",
                "-c",
                "if . == 1 then error(\"first\") elif . == 2 then debug(\"middle\") else error(\"last\") end",
            ]
            .as_slice(),
            b"1\n2\n3\n",
            ExecutionOverride::Document,
        );
        assert_eq!(status.unwrap(), ExitStatus::Runtime);
        assert_eq!(stdout, b"2\n");
        assert_eq!(
            stderr,
            b"tq: runtime error: first\n[\"DEBUG:\",\"middle\"]\ntq: runtime error: last\n"
        );
    }

    #[test]
    fn document_runtime_diagnostic_precedes_a_later_parse_error() {
        let (status, stdout, stderr) = execute_with_override(
            &["-ijson", "-ojson", "-c", ". + 1"],
            b"\"x\"\n{\n",
            ExecutionOverride::Document,
        );
        let error = status.unwrap_err();
        assert_eq!(error.status(), ExitStatus::Input);
        assert!(matches!(error, super::RunError::Input(_)));
        assert!(stdout.is_empty());
        let stderr = String::from_utf8(stderr).unwrap();
        assert_eq!(stderr.matches("runtime error:").count(), 1);
        assert!(stderr.starts_with("tq: runtime error:"));
        assert_eq!(stderr.lines().count(), 1);
    }

    #[test]
    fn single_document_invocations_keep_embedded_runtime_errors() {
        let (status, stdout, stderr) = execute_with_override(
            &["-n", "-ojson", "-c", "error(\"single\")"],
            b"",
            ExecutionOverride::Document,
        );
        let error = status.unwrap_err();
        assert_eq!(error.status(), ExitStatus::Runtime);
        assert!(matches!(error, super::RunError::Runtime(_)));
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());

        let (status, stdout, stderr) = execute_with_override(
            &["--slurp", "-ijson", "-ojson", "-c", ". + 1"],
            b"1\n2\n",
            ExecutionOverride::Document,
        );
        let error = status.unwrap_err();
        assert_eq!(error.status(), ExitStatus::Runtime);
        assert!(matches!(error, super::RunError::Runtime(_)));
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());

        let (status, stdout, stderr) = execute_with_override(
            &["-R", "--slurp", "-ojson", "-c", "error(\"single\")"],
            b"a\nb\n",
            ExecutionOverride::Document,
        );
        let error = status.unwrap_err();
        assert_eq!(error.status(), ExitStatus::Runtime);
        assert!(matches!(error, super::RunError::Runtime(_)));
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
    }

    #[test]
    fn document_empty_after_runtime_error_obeys_exit_status_mode() {
        let arguments = [
            "-ijson",
            "-ojson",
            "-c",
            "if type == \"string\" then . + 1 else empty end",
        ];
        let without_e =
            execute_with_override(&arguments, b"\"x\"\n2\n", ExecutionOverride::Document);
        assert_eq!(without_e.0.unwrap(), ExitStatus::Success);
        assert!(without_e.1.is_empty());
        assert_single_runtime_diagnostic(&without_e.2);

        let with_e_arguments = [
            "-ijson",
            "-ojson",
            "-c",
            "-e",
            "if type == \"string\" then . + 1 else empty end",
        ];
        let with_e = execute_with_override(
            &with_e_arguments,
            b"\"x\"\n2\n",
            ExecutionOverride::Document,
        );
        assert_eq!(with_e.0.unwrap(), ExitStatus::NoResult);
        assert!(with_e.1.is_empty());
        assert_single_runtime_diagnostic(&with_e.2);
    }

    #[test]
    fn result_and_output_limits_preserve_complete_prior_frames() {
        let (status, stdout, _) = execute(
            &[
                "-n",
                "--output-format",
                "toon-seq",
                "--max-results",
                "1",
                "1, 2",
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
                "--output-format",
                "toon-seq",
                "--max-output-bytes",
                "3",
                "1, 2",
            ],
            b"",
        );
        assert_eq!(status.unwrap_err().status(), ExitStatus::Resource);
        assert_eq!(stdout, b"\x1e1\n");

        let (status, stdout, _) = execute(
            &[
                "-n",
                "--output-format",
                "toon-seq",
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
                "--output-format",
                "toon-seq",
                "--max-output-bytes",
                "3",
                "foreach (1,2) as $x (0; . + $x; .)",
            ],
            b"",
        );
        assert_eq!(status.unwrap_err().status(), ExitStatus::Resource);
        assert_eq!(stdout, b"\x1e1\n");

        let (status, stdout, _) = execute(
            &["--output-format", "toon-seq", "--max-results", "2", ".."],
            b"[1,2]",
        );
        assert!(matches!(
            status,
            Err(super::RunError::Resource("result-count"))
        ));
        assert_eq!(stdout, b"\x1e[2]: 1,2\n\x1e1\n");

        let (status, stdout, _) = execute(
            &[
                "-n",
                "--output-format",
                "toon-seq",
                "--max-output-bytes",
                "5",
                r#"1, "abcdef=\(.)""#,
            ],
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
            &[
                "--input-format",
                "json",
                "--stream",
                "--output-format",
                "json",
                "--explain-json",
                ".",
            ],
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
        assert_eq!(status.unwrap(), ExitStatus::Runtime);
        assert_eq!(stdout, b"1\n");
        assert_eq!(
            stderr,
            b"tq: runtime error: field access cannot be applied to number\n"
        );
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
            observed_exit_status(&hybrid.0),
            observed_exit_status(&document.0)
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
            observed_exit_status(&automatic.0),
            observed_exit_status(&document.0)
        );
        assert_eq!(automatic.1, document.1);
    }

    #[test]
    fn nested_static_prefix_explains_root_transactional_serialization() {
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
            false
        );
        assert_eq!(
            explain["execution"]["parallel_selected_decode"]["reason"],
            "root-lifecycle-requires-serial-commit"
        );

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
            false
        );
        assert_eq!(
            explain["execution"]["parallel_selected_decode"]["reason"],
            "root-lifecycle-requires-serial-commit"
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
            false
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

    #[test]
    fn run_tests_self_checks_match_jq_state_contract() {
        assert!(super::run_tests_self_checks());
    }

    #[test]
    fn run_test_compile_expectations_require_all_diagnostic_lines() {
        let (cases, malformed) =
            super::parse_run_test_cases("%%FAIL\nfoo\nfirst diagnostic\nsecond diagnostic\n\n");
        assert_eq!(malformed, 0);
        assert_eq!(cases.len(), 1);
        assert!(super::compile_diagnostic_matches(
            "first diagnostic\nsecond diagnostic\n",
            None,
            &cases[0].expected,
            "foo"
        ));
        assert!(!super::compile_diagnostic_matches(
            "first diagnostic\n",
            None,
            &cases[0].expected,
            "foo"
        ));
        assert!(!super::compile_diagnostic_matches(
            "prefix first diagnostic\nsecond diagnostic\n",
            None,
            &cases[0].expected,
            "foo"
        ));
    }

    #[allow(
        clippy::too_many_lines,
        reason = "the jq diagnostic oracle matrix keeps each expected source/caret fixture explicit"
    )]
    #[test]
    fn jq_compile_expectations_render_structured_tq_diagnostics() {
        for (filter, diagnostic, expected) in [
            (
                "$missing",
                "query compilation failed: TQ-RESOLVE-VARIABLE-001: unknown variable $missing",
                "jq: error: $missing is not defined at <top-level>, line 1, column 1:\n    $missing\n    ^^^^^^^^",
            ),
            (
                "1 | foo(1)",
                "query compilation failed: TQ-RESOLVE-BUILTIN-001: unknown filter foo/1",
                "jq: error: foo/1 is not defined at <top-level>, line 1, column 5:\n    1 | foo(1)\n        ^^^",
            ),
            (
                "1 +",
                "query compilation failed: TQ-PARSE-EXPRESSION-001: expected filter expression",
                "jq: error: syntax error, unexpected end of file at <top-level>, line 1, column 3:\n    1 +\n      ^",
            ),
        ] {
            let expected = expected.lines().map(str::to_owned).collect::<Vec<_>>();
            assert!(
                super::compile_diagnostic_matches(diagnostic, None, &expected, filter),
                "{filter:?}: {expected:?}"
            );
        }
        for (filter, expected) in [
            (
                "]",
                "jq: error: syntax error, unexpected INVALID_CHARACTER, expecting end of file at <top-level>, line 1, column 1:\n    ]\n    ^",
            ),
            (
                ";",
                "jq: error: syntax error, unexpected ';', expecting end of file at <top-level>, line 1, column 1:\n    ;\n    ^",
            ),
            (
                "1 + ]",
                "jq: error: syntax error, unexpected INVALID_CHARACTER at <top-level>, line 1, column 5:\n    1 + ]\n        ^",
            ),
            (
                "1 + ;",
                "jq: error: syntax error, unexpected ';' at <top-level>, line 1, column 5:\n    1 + ;\n        ^",
            ),
            (
                "{x:}",
                "jq: error: syntax error, unexpected '}' at <top-level>, line 1, column 4:\n    {x:}\n       ^",
            ),
        ] {
            let diagnostic = super::parse_bytes("<query>", filter.as_bytes()).unwrap_err();
            let expected = expected.lines().map(str::to_owned).collect::<Vec<_>>();
            let actual = format!("query compilation failed: {diagnostic}");
            assert!(
                super::compile_diagnostic_matches(
                    &actual,
                    Some(diagnostic.as_ref()),
                    &expected,
                    filter,
                ),
                "{filter:?}: {expected:?}"
            );
        }
        for (filter, expected) in [
            (
                "[1",
                "jq: error: syntax error, unexpected end of file, expecting '|' or ',' or ']' at <top-level>, line 1, column 2:\n    [1\n     ^",
            ),
            (
                "{x:1",
                "jq: error: syntax error, unexpected end of file, expecting '}' at <top-level>, line 1, column 4:\n    {x:1\n       ^",
            ),
            (
                "(1",
                "jq: error: syntax error, unexpected end of file, expecting '|' or ',' or ')' at <top-level>, line 1, column 2:\n    (1\n     ^",
            ),
            (
                "foo(1",
                "jq: error: syntax error, unexpected end of file, expecting ';' or ')' at <top-level>, line 1, column 5:\n    foo(1\n        ^",
            ),
            (
                "{",
                "jq: error: syntax error, unexpected end of file at <top-level>, line 1, column 1:\n    {\n    ^",
            ),
        ] {
            let diagnostic = super::parse_bytes("<query>", filter.as_bytes()).unwrap_err();
            let expected = expected.lines().map(str::to_owned).collect::<Vec<_>>();
            let actual = format!("query compilation failed: {diagnostic}");
            assert!(
                super::compile_diagnostic_matches(
                    &actual,
                    Some(diagnostic.as_ref()),
                    &expected,
                    filter,
                ),
                "{filter:?}: {expected:?}"
            );
        }
        for (filter, expected) in [
            (
                "if true then 1",
                "jq: error: Possibly unterminated 'if' statement at <top-level>, line 1, column 1:\n    if true then 1\n    ^^^^^^^^^^^^^^",
            ),
            (
                "\"é",
                "jq: error: syntax error, unexpected end of file, expecting QQSTRING_TEXT or QQSTRING_INTERP_START or QQSTRING_END at <top-level>, line 1, column 2:\n    \"é\n     ^^",
            ),
        ] {
            let diagnostic = super::parse_bytes("<query>", filter.as_bytes()).unwrap_err();
            let expected = expected.lines().map(str::to_owned).collect::<Vec<_>>();
            let actual = format!("query compilation failed: {diagnostic}");
            assert!(
                super::compile_diagnostic_matches(
                    &actual,
                    Some(diagnostic.as_ref()),
                    &expected,
                    filter,
                ),
                "{filter:?}: {expected:?}"
            );
        }
        for (filter, expected) in [
            (
                "if true",
                "jq: error: syntax error, unexpected end of file, expecting then or '|' or ',' at <top-level>, line 1, column 4:\n    if true\n       ^^^^",
            ),
            (
                "if true then",
                "jq: error: Possibly unterminated 'if' statement at <top-level>, line 1, column 1:\n    if true then\n    ^^^^^^^^^^^^",
            ),
            (
                "if true then 1 else",
                "jq: error: Possibly unterminated 'if' statement at <top-level>, line 1, column 1:\n    if true then 1 else\n    ^^^^^^^^^^^^^^^^^^^",
            ),
            (
                "if",
                "jq: error: syntax error, unexpected end of file at <top-level>, line 1, column 1:\n    if\n    ^^",
            ),
            (
                "if true elif",
                "jq: error: syntax error, unexpected elif, expecting then or '|' or ',' at <top-level>, line 1, column 9:\n    if true elif\n            ^^^^",
            ),
            (
                "try",
                "jq: error: syntax error, unexpected end of file at <top-level>, line 1, column 1:\n    try\n    ^^^",
            ),
            (
                "{(1",
                "jq: error: syntax error, unexpected end of file, expecting '|' or ',' or ')' at <top-level>, line 1, column 3:\n    {(1\n      ^",
            ),
            (
                "@",
                "jq: error: syntax error, unexpected INVALID_CHARACTER, expecting end of file at <top-level>, line 1, column 1:\n    @\n    ^",
            ),
            (
                "$",
                "jq: error: syntax error, unexpected end of file, expecting '$' at <top-level>, line 1, column 1:\n    $\n    ^",
            ),
            (
                "def",
                "jq: error: syntax error, unexpected end of file, expecting IDENT at <top-level>, line 1, column 1:\n    def\n    ^^^",
            ),
            (
                "label",
                "jq: error: syntax error, unexpected end of file, expecting BINDING at <top-level>, line 1, column 1:\n    label\n    ^^^^^",
            ),
            (
                "def foo(",
                "jq: error: syntax error, unexpected end of file, expecting IDENT or BINDING at <top-level>, line 1, column 8:\n    def foo(\n           ^",
            ),
            (
                "label $x",
                "jq: error: syntax error, unexpected end of file, expecting '|' at <top-level>, line 1, column 7:\n    label $x\n          ^^",
            ),
            (
                "def 1",
                "jq: error: syntax error, unexpected LITERAL, expecting IDENT at <top-level>, line 1, column 5:\n    def 1\n        ^",
            ),
            (
                "label 1",
                "jq: error: syntax error, unexpected LITERAL, expecting BINDING at <top-level>, line 1, column 7:\n    label 1\n          ^",
            ),
            (
                "def f(1;",
                "jq: error: syntax error, unexpected LITERAL, expecting IDENT or BINDING at <top-level>, line 1, column 7:\n    def f(1;\n          ^",
            ),
            (
                "\"unterminated",
                "jq: error: syntax error, unexpected end of file, expecting QQSTRING_TEXT or QQSTRING_INTERP_START or QQSTRING_END at <top-level>, line 1, column 2:\n    \"unterminated\n     ^^^^^^^^^^^^",
            ),
            (
                "\"\\q\"",
                "jq: error: Invalid escape at line 1, column 4 (while parsing '\"\\q\"') at <top-level>, line 1, column 2:\n    \"\\q\"\n     ^^",
            ),
        ] {
            let diagnostic = super::parse_bytes("<query>", filter.as_bytes()).unwrap_err();
            let expected = expected.lines().map(str::to_owned).collect::<Vec<_>>();
            let actual = format!("query compilation failed: {diagnostic}");
            assert!(
                super::compile_diagnostic_matches(
                    &actual,
                    Some(diagnostic.as_ref()),
                    &expected,
                    filter,
                ),
                "{filter:?}: {expected:?}"
            );
        }
        for (filter, expected) in [
            (
                "1 | if true then 1",
                "jq: error: Possibly unterminated 'if' statement at <top-level>, line 1, column 5:\n    1 | if true then 1\n        ^^^^^^^^^^^^^^",
            ),
            (
                "if true then if false then 1 end",
                "jq: error: Possibly unterminated 'if' statement at <top-level>, line 1, column 1:\n    if true then if false then 1 end\n    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^",
            ),
            (
                "1 | @",
                "jq: error: syntax error, unexpected INVALID_CHARACTER at <top-level>, line 1, column 5:\n    1 | @\n        ^",
            ),
            (
                "1 | $",
                "jq: error: syntax error, unexpected end of file, expecting '$' at <top-level>, line 1, column 5:\n    1 | $\n        ^",
            ),
            (
                "1 | def",
                "jq: error: syntax error, unexpected end of file, expecting IDENT at <top-level>, line 1, column 5:\n    1 | def\n        ^^^",
            ),
            (
                "1 | label",
                "jq: error: syntax error, unexpected end of file, expecting BINDING at <top-level>, line 1, column 5:\n    1 | label\n        ^^^^^",
            ),
        ] {
            let diagnostic = super::parse_bytes("<query>", filter.as_bytes()).unwrap_err();
            let expected = expected.lines().map(str::to_owned).collect::<Vec<_>>();
            let actual = format!("query compilation failed: {diagnostic}");
            assert!(super::compile_diagnostic_matches(
                &actual,
                Some(diagnostic.as_ref()),
                &expected,
                filter,
            ));
        }
    }
}
