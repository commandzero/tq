//! End-to-end command runner with strict stdout/stderr separation.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{self, Read, Write},
    path::Path,
    sync::Arc,
};

use thiserror::Error;
use tq_core::{
    AnalysisContext, Diagnostic, ResolveOptions, Value, Vm, VmError, VmLimits,
    analyze_with_context, parse_bytes, resolve,
};
use tq_formats::{
    DecodeOptions, FormatError, InputFormat, OutputError, OutputOptions, decode_bytes, decode_json,
    decode_toon, write_results,
};

use crate::{
    CliError, Command, ExitStatus, ExplainFormat, ExternalArgument, ExternalArgumentKind,
    FilterSource, RunOptions,
};

const HELP: &str = "tq - jq-compatible queries over TOON, YAML, and JSON\n\n\
Usage: tq [OPTIONS] FILTER [FILE...]\n       tq [OPTIONS] -f FILE [INPUT...]\n       tq compatibility\n\n\
Input:  --input-format auto|toon|yaml|json|toon-seq (default: auto)\n\
Output: --output-format toon|json, --unframed, -r/--raw-output, -j/--join-output\n\
Modes:  -n/--null-input, -R/--raw-input, -s/--slurp, --stream, --stream-errors\n\
Args:   --arg NAME VALUE, --argjson NAME JSON, --argtoon NAME TOON\n\
TOON:   --indent N, --delimiter comma|tab|pipe, --fold-keys, --non-strict\n\
Other:  -e/--exit-status, --explain, --explain-json, --trace, --report-file FILE\n";

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
    /// Mode recognized but not admitted by the current plan.
    #[error("unsupported mode: {0}")]
    Unsupported(String),
}

impl RunError {
    /// Process status category for this failure.
    #[must_use]
    pub fn status(&self) -> ExitStatus {
        match self {
            Self::Cli(CliError::Unsupported(_)) | Self::Unsupported(_) => ExitStatus::Unsupported,
            Self::Cli(_) | Self::Io(_) => ExitStatus::Usage,
            Self::Compile(_) => ExitStatus::Compile,
            Self::Input(FormatError::Resource(_)) | Self::Runtime(VmError::Resource { .. }) => {
                ExitStatus::Resource
            }
            Self::Input(_) => ExitStatus::Input,
            Self::Runtime(_) | Self::Output(_) | Self::Json(_) => ExitStatus::Runtime,
        }
    }
}

/// Runs a parsed command against process stdio and writes diagnostics only to
/// stderr.
#[must_use]
pub fn run(command: Command) -> ExitStatus {
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();
    match run_with_io(command, &mut stdin, &mut stdout, &mut stderr) {
        Ok(status) => status,
        Err(error) => {
            let _ = writeln!(stderr, "tq: {error}");
            error.status()
        }
    }
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
            stdout.write_all(HELP.as_bytes())?;
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
                "../../../compatibility/reviews/coverage-v1.json"
            ))?;
            stdout.write_all(b"\n")?;
            Ok(ExitStatus::Success)
        }
        Command::Run(options) => run_filter(&options, stdin, stdout, stderr),
    }
}

fn run_filter<R: Read, W: Write, E: Write>(
    options: &RunOptions,
    stdin: &mut R,
    stdout: &mut W,
    stderr: &mut E,
) -> Result<ExitStatus, RunError> {
    let (query_name, query) = load_filter(&options.filter)?;
    let variables = parse_external_arguments(&options.arguments, options.strict)?;
    let resolve_options = ResolveOptions {
        variables: variables.keys().cloned().collect::<BTreeSet<_>>(),
    };
    let parsed = parse_bytes(&query_name, &query).map_err(RunError::Compile)?;
    let resolved = resolve(parsed, &resolve_options).map_err(RunError::Compile)?;
    let analyzed = analyze_with_context(
        resolved,
        AnalysisContext {
            event_input: options.stream,
            whole_input: options.slurp,
        },
    );
    if let Some(explain) = options.explain {
        match explain {
            ExplainFormat::Human => stderr.write_all(analyzed.explain().as_bytes())?,
            ExplainFormat::Json => {
                serde_json::to_writer_pretty(&mut *stderr, &analyzed.explain_json())?;
                stderr.write_all(b"\n")?;
            }
        }
    }
    let program = analyzed.compile().map_err(RunError::Compile)?;

    let inputs = load_inputs(options, stdin)?;
    if options.stream {
        if inputs.iter().any(|input| input.format == InputFormat::Yaml) {
            return Err(RunError::Unsupported(
                "YAML input is document-at-a-time and cannot satisfy --stream".to_owned(),
            ));
        }
        return Err(RunError::Unsupported(
            "explicit path/value stream execution is enabled in the streaming plan wave".to_owned(),
        ));
    }

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
    let mut results = Vec::new();
    let mut last = None;
    let mut observations = Vec::new();
    for input in values {
        let mut vm =
            Vm::new_with_variables(&program, input, VmLimits::default(), variables.clone())
                .with_trace_limit(options.trace_limit);
        while let Some(value) = vm.next_result()? {
            last = Some(value.clone());
            results.push(value);
        }
        if options.trace_limit != 0 {
            for entry in vm.trace() {
                writeln!(stderr, "trace: {entry}")?;
            }
        }
        observations.push(vm.observations());
    }

    if options.raw_output {
        write_raw(stdout, &results, options.join_output)?;
    } else {
        write_results(
            stdout,
            &results,
            OutputOptions {
                format: options.output_format,
                pretty_json: options.pretty_json,
                toon_framing: options.framing,
                toon: options.toon_writer,
            },
        )?;
    }
    if let Some(path) = &options.report_file {
        write_report(path, &observations, results.len())?;
    }
    Ok(exit_status(options.exit_status, last.as_ref()))
}

fn load_filter(source: &FilterSource) -> Result<(String, Vec<u8>), RunError> {
    match source {
        FilterSource::Inline(query) => Ok(("<command-line>".to_owned(), query.as_bytes().to_vec())),
        FilterSource::File(path) => Ok((path.display().to_string(), fs::read(path)?)),
    }
}

fn parse_external_arguments(
    arguments: &[ExternalArgument],
    strict: bool,
) -> Result<BTreeMap<Arc<str>, Value>, RunError> {
    let mut values = BTreeMap::new();
    for argument in arguments {
        let value = match argument.kind {
            ExternalArgumentKind::String => Value::string(argument.value.as_str()),
            ExternalArgumentKind::Json => {
                decode_json(argument.value.as_bytes(), "--argjson")?
                    .pop()
                    .ok_or_else(|| RunError::Unsupported("--argjson produced no value".to_owned()))?
                    .value
            }
            ExternalArgumentKind::Toon => {
                let config = tq_toon::DecoderConfig {
                    strict,
                    ..tq_toon::DecoderConfig::default()
                };
                decode_toon(argument.value.as_bytes(), "--argtoon", config)?
                    .pop()
                    .ok_or_else(|| RunError::Unsupported("--argtoon produced no value".to_owned()))?
                    .value
            }
        };
        values.insert(Arc::from(argument.name.as_str()), value);
    }
    Ok(values)
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
            let mut bytes = Vec::new();
            stdin.read_to_end(&mut bytes)?;
            ("<stdin>".to_owned(), bytes)
        } else {
            (path.display().to_string(), fs::read(&path)?)
        };
        if options.raw_input {
            raw_documents(&mut documents, identity, bytes, options.slurp)?;
        } else {
            let decode = DecodeOptions {
                format: options.input_format,
                toon: tq_toon::DecoderConfig {
                    strict: options.strict,
                    ..tq_toon::DecoderConfig::default()
                },
                ..DecodeOptions::default()
            };
            documents.extend(decode_bytes(&bytes, identity, decode)?);
        }
    }
    Ok(documents)
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

fn write_raw(mut output: impl Write, values: &[Value], join: bool) -> Result<(), RunError> {
    for value in values {
        match value {
            Value::String(value) => output.write_all(value.as_bytes())?,
            _ => serde_json::to_writer(&mut output, value)?,
        }
        if !join {
            output.write_all(b"\n")?;
        }
    }
    Ok(())
}

fn write_report(
    path: &Path,
    observations: &[tq_core::VmObservations],
    results: usize,
) -> Result<(), RunError> {
    let report = serde_json::json!({
        "schema_version": 1,
        "documents": observations.len(),
        "results": results,
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
    fn identity_uses_toon_sequence_and_keeps_stderr_clean() {
        let (status, stdout, stderr) = execute(&["."], b"name: Ada");
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(stdout, b"\x1ename: Ada\n");
        assert!(stderr.is_empty());
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
}
