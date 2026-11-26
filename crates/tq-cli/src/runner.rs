//! End-to-end command runner with strict stdout/stderr separation.

use std::{
    io::{self, IsTerminal, Read, Write},
    sync::{Arc, OnceLock, atomic::AtomicBool},
};

use thiserror::Error;
use tq_core::{Diagnostic, EffectSink, Vm, VmError, VmLimits, VmObservations};
use tq_formats::{FormatError, InputFormat, NativeFormat, OutputError};

use crate::{CliError, ColorMode, Command, ExitStatus, RunOptions, generated_help};

mod automatic;
mod detection;
mod document;
mod events;
mod filter;
mod input;
mod output;
mod query;
mod reports;
mod test_file;
mod transcode;

use filter::run_filter;
use test_file::{RunTestCaptureHandle, run_tests};

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
    configure_process_color(&mut command);
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
    match stdout.flush() {
        Ok(()) => result.unwrap_or_else(|error| process_exit_status(error, &mut stderr)),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitStatus::Success,
        Err(error) => {
            let _ = writeln!(stderr, "tq: system I/O failed: {error}");
            ExitStatus::Runtime
        }
    }
}

fn configure_process_color(command: &mut Command) {
    let Command::Run(options) = command else {
        return;
    };
    if options.color != ColorMode::Auto {
        return;
    }
    let terminal = options.capability_policy.terminal && io::stdout().is_terminal();
    let no_color = options.capability_policy.environment
        && std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty());
    options.color = if terminal && !no_color {
        ColorMode::Always
    } else {
        ColorMode::Never
    };
}

fn process_exit_status(error: RunError, stderr: &mut impl Write) -> ExitStatus {
    match error {
        error if error.is_broken_pipe() => ExitStatus::Success,
        RunError::Runtime(VmError::Halt {
            status,
            stderr: bytes,
        }) => stderr
            .write_all(&bytes)
            .map_or(ExitStatus::Runtime, |()| ExitStatus::Halt(status)),
        RunError::ReportedRuntime(_) => error.status(),
        _ => {
            let _ = writeln!(stderr, "tq: {}", process_error_message(&error));
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
    flush_buffered_effects(&vm.effect_sink(), stderr)
}

fn flush_buffered_effects(sink: &EffectSink, stderr: &mut impl Write) -> Result<(), RunError> {
    let effects = sink.drain();
    if effects.is_empty() {
        return Ok(());
    }
    stderr.write_all(&effects)?;
    Ok(())
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
                "tq {} (TOON v4.4; jq target 1.8.x; revision {})",
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

#[cfg(test)]
mod tests;
