//! End-to-end command runner with strict stdout/stderr separation.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{self, BufReader, Read, Write},
    path::Path,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

use thiserror::Error;
use tq_core::{
    AnalysisContext, Analyzed, Compiled, Diagnostic, Program, Query, ResolveOptions, Value, Vm,
    VmError, VmLimits, VmObservations, analyze_with_context, parse_bytes, resolve,
};
use tq_formats::{
    DecodeOptions, FormatError, InputFormat, OutputError, OutputOptions, StreamOptions,
    ToonFraming, decode_bytes, decode_json, decode_toon, probe_reader, stream_json, stream_toon,
    write_results,
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
Other:  -e/--exit-status, --explain, --explain-json, --trace, --report-file FILE\n\
Resources: --max-input-bytes N, --max-depth N, --max-token-bytes N,\n\
           --max-line-bytes N, --max-lookahead-bytes N, --max-vm-steps N,\n\
           --max-results N, --max-output-bytes N, --prepare-memory-bytes N,\n\
           --max-spool-bytes N\n";

static CANCELLATION: OnceLock<Arc<AtomicBool>> = OnceLock::new();

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
    /// A CLI-level input, result, or output envelope was exceeded.
    #[error("resource limit exceeded: {0}")]
    Resource(&'static str),
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
            Self::Cli(_) | Self::Io(_) => ExitStatus::Usage,
            Self::Compile(_) => ExitStatus::Compile,
            Self::Resource(_)
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
            Self::Runtime(_) | Self::Output(_) | Self::Json(_) => ExitStatus::Runtime,
        }
    }

    fn is_broken_pipe(&self) -> bool {
        match self {
            Self::Io(error) => error.kind() == io::ErrorKind::BrokenPipe,
            Self::Output(error) => error.is_broken_pipe(),
            _ => false,
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
    if let Err(error) = install_interrupt_handler() {
        let _ = writeln!(stderr, "tq: could not install interrupt handler: {error}");
        return ExitStatus::Usage;
    }
    match run_with_io(command, &mut stdin, &mut stdout, &mut stderr) {
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
                "../../../tests/compatibility/reviews/coverage-v1.json"
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
        write_explain(explain, options, &analyzed, stderr)?;
    }
    let program = analyzed.compile().map_err(RunError::Compile)?;

    if options.stream {
        let plan = program.event_plan().map_err(RunError::Compile)?;
        return run_event_filter(options, plan.program(), &variables, stdin, stdout, stderr);
    }

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
        let mut vm = Vm::new_with_variables(&program, input, vm_limits(options), variables.clone())
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
        )?;
    }
    if let Some(error) = runtime_error {
        return Err(RunError::Runtime(error));
    }
    Ok(exit_status(options.exit_status, last.as_ref()))
}

fn write_explain(
    format: ExplainFormat,
    options: &RunOptions,
    analyzed: &Query<Analyzed>,
    stderr: &mut impl Write,
) -> Result<(), RunError> {
    let capabilities = analyzed.capabilities();
    let plan = if capabilities.event_stream {
        "events"
    } else if capabilities.whole_input {
        "whole-input"
    } else if capabilities.blocking {
        "blocking-document"
    } else if capabilities.subtree {
        "subtree"
    } else {
        "document"
    };
    let retained = match plan {
        "events" => "decoder frames, current path, and one event value",
        "whole-input" => "all input documents",
        "blocking-document" => "one document plus blocking operator state",
        "subtree" => "the selected complete subtree",
        _ => "one complete document",
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
                "plan": plan,
                "input_detection": detection,
                "retained_working_set": retained,
                "blocking": capabilities.blocking,
                "spool_required": false,
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
        InputFormat::Auto => "auto:toon->yaml->json",
        InputFormat::Toon => "override:toon",
        InputFormat::Yaml => "override:yaml",
        InputFormat::Json => "override:json",
        InputFormat::ToonSequence => "override:toon-sequence",
    }
}

fn run_event_filter<R: Read, W: Write, E: Write>(
    options: &RunOptions,
    program: &Program<Compiled>,
    variables: &BTreeMap<Arc<str>, Value>,
    stdin: &mut R,
    stdout: &mut W,
    stderr: &mut E,
) -> Result<ExitStatus, RunError> {
    let mut executor = StreamExecutor {
        program,
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
            stream_reader(options, &mut *stdin, &mut executor)?;
        } else {
            stream_reader(options, File::open(path)?, &mut executor)?;
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
        )?;
    }
    Ok(exit_status(options.exit_status, executor.last.as_ref()))
}

fn stream_reader<R: Read, W: Write, E: Write>(
    options: &RunOptions,
    reader: R,
    executor: &mut StreamExecutor<'_, W, E>,
) -> Result<(), RunError> {
    let stream_options = StreamOptions {
        maximum_depth: options.limits.depth,
        errors_as_values: options.stream_errors,
    };
    let reader = LimitedReader::new(reader, options.limits.input_bytes);
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
}

impl<R> LimitedReader<R> {
    fn new(reader: R, limit: u64) -> Self {
        Self {
            reader,
            remaining: limit,
            exhausted: false,
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
            return Err(io::Error::other(
                "input resource limit exceeded: input-bytes",
            ));
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
    program: &'a Program<Compiled>,
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
        let mut vm = Vm::new_with_variables(
            self.program,
            input,
            vm_limits(self.output.options),
            self.variables.clone(),
        )
        .with_trace_limit(self.trace_remaining);
        if let Some(flag) = cancellation() {
            vm = vm.with_cancellation(flag);
        }
        loop {
            match vm.next_result() {
                Ok(Some(value)) => {
                    self.last = Some(value.clone());
                    self.output.emit(&value)?;
                    self.results = self.results.saturating_add(1);
                }
                Ok(None) => break,
                Err(error) => return Err(RunError::Runtime(error)),
            }
        }
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
        if self.options.raw_output {
            let mut writer = LimitedWriter::new(
                &mut self.writer,
                &mut self.written,
                self.options.limits.output_bytes,
            );
            return write_raw(
                &mut writer,
                std::slice::from_ref(value),
                self.options.join_output,
            );
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
        write_results(
            &mut writer,
            [value],
            OutputOptions {
                format: self.options.output_format,
                pretty_json: self.options.pretty_json,
                toon_framing: self.options.framing,
                toon: self.options.toon_writer,
            },
        )?;
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
                    toon_framing: self.options.framing,
                    toon: self.options.toon_writer,
                },
            )?;
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
            (
                "<stdin>".to_owned(),
                read_limited(&mut *stdin, options.limits.input_bytes)?,
            )
        } else {
            (
                path.display().to_string(),
                read_limited(File::open(&path)?, options.limits.input_bytes)?,
            )
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

fn read_limited(mut reader: impl Read, limit: u64) -> Result<Vec<u8>, RunError> {
    let mut bytes = Vec::new();
    reader
        .by_ref()
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limit {
        return Err(RunError::Resource("input-bytes"));
    }
    Ok(bytes)
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
    output_bytes: u64,
    options: &RunOptions,
) -> Result<(), RunError> {
    let report = serde_json::json!({
        "schema_version": 1,
        "documents": observations.len(),
        "results": results,
        "output_bytes": output_bytes,
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
}
