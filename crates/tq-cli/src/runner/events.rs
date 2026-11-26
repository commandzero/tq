//! Drive explicit stream plans and project native event sources.

use super::automatic::RetentionObservations;
use super::detection::requires_staged_toon_events;
use super::input::{
    LimitedReader, StructuredInput, decode_options, open_path, proxyable_format_error,
    read_limited, render_native_failure, selected_input_format, toon_decoder_config,
    validate_proxy_event_source,
};
use super::output::ResultOutput;
use super::reports::{ReportExecution, write_report};
use super::test_file::RunTestCaptureHandle;
use super::{
    INPUT_BUFFER_BYTES, JSON5_STREAM_UNSUPPORTED, RunError, cancellation, flush_vm_effects,
    merge_observations, vm_limits,
};
use crate::{ExitStatus, RunOptions};
use std::collections::BTreeMap;
use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tq_core::{Analysis, Compiled, Events, Plan, PlanKind, SourceId, Value, Vm, VmObservations};
use tq_formats::{
    FormatError, InputFormat, InputRepresentation, JsonLinesDocumentSource, NativeFormat,
    NativeInputObservation, StreamOptions, StreamRecord, probe_reader, stream_json, stream_toon,
};

#[allow(
    clippy::too_many_arguments,
    reason = "the event route keeps input, output, diagnostics, analysis, and test capture explicit"
)]
pub(super) fn run_event_filter<R: Read, W: Write, E: Write>(
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
        variables: Arc::new(variables.clone()),
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
    if requires_staged_toon_events(options, format) {
        return stream_projected_input(options, format, reader, identity, executor);
    }

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
                InputFormat::Toon if requires_staged_toon_events(options, InputFormat::Toon) => {
                    stream_projected_input(options, InputFormat::Toon, replay, identity, executor)
                }
                InputFormat::Toon => stream_toon_into(replay, options, stream_options, executor),
                InputFormat::JsonSequence => {
                    stream_projected_input(options, InputFormat::JsonSequence, replay, identity, executor)
                }
                InputFormat::Yaml => Err(RunError::Unsupported(
                    "auto-detection selected YAML, which is document-at-a-time and cannot satisfy --stream; use --input-format json for JSON syntax".to_owned(),
                )),
                InputFormat::ToonSequence
                    if requires_staged_toon_events(options, InputFormat::ToonSequence) =>
                {
                    stream_projected_input(
                        options,
                        InputFormat::ToonSequence,
                        replay,
                        identity,
                        executor,
                    )
                }
                InputFormat::ToonSequence => Err(RunError::Unsupported(
                    "TOON sequence input cannot currently be nested inside --stream".to_owned(),
                )),
                InputFormat::Auto | InputFormat::Json5 | InputFormat::JsonLines => {
                    unreachable!("probe candidate")
                }
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
        InputFormat::ToonSequence
            if requires_staged_toon_events(options, InputFormat::ToonSequence) =>
        {
            stream_projected_input(
                options,
                InputFormat::ToonSequence,
                reader,
                identity,
                executor,
            )
        }
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
        toon_decoder_config(options),
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

pub(super) fn buffered_input<R: Read>(reader: R) -> BufReader<R> {
    BufReader::with_capacity(INPUT_BUFFER_BYTES, reader)
}

pub(super) fn project_committed_input<R: Read, F>(
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
    let staged_toon_events = requires_staged_toon_events(options, format.descriptor().input);
    if staged_toon_events {
        return project_staged_toon_input(options, format, reader, identity, emit);
    }
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
fn project_staged_toon_input<R: Read, F>(
    options: &RunOptions,
    format: NativeFormat,
    reader: R,
    identity: &str,
    emit: &mut F,
) -> Result<bool, RunError>
where
    F: FnMut(StructuredInput) -> Result<bool, RunError>,
{
    let mut source = format
        .select_input(
            decode_options(options, format.descriptor().input),
            InputRepresentation::Documents,
        )?
        .open(reader, identity);
    let execution_error = std::cell::RefCell::new(None);
    let stopped = std::cell::Cell::new(false);
    let emit = std::cell::RefCell::new(emit);
    let mut emit_record =
        |record: StreamRecord| match emit.borrow_mut()(StructuredInput::Value(record.into_value()))
        {
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
    let mut projector = tq_formats::EventProjector::new(
        StreamOptions {
            maximum_depth: options.limits.depth,
            maximum_token_bytes: options.limits.token_bytes,
            errors_as_values: options.stream_errors,
        },
        &mut emit_record,
    );
    while let Some(observation) = source.next_observation()? {
        let result = match observation {
            NativeInputObservation::Document(document) => {
                emit_value_events(&document.value, &mut projector)
            }
            NativeInputObservation::Failure(failure) if options.stream_errors => {
                projector.error_value(failure.message)
            }
            NativeInputObservation::Failure(failure) => {
                projector.reset();
                match emit.borrow_mut()(StructuredInput::Failure(failure)) {
                    Ok(true) => Ok(()),
                    Ok(false) => {
                        stopped.set(true);
                        Err("input consumer stopped".to_owned())
                    }
                    Err(error) => {
                        execution_error.replace(Some(error));
                        Err("input consumer failed".to_owned())
                    }
                }
            }
            NativeInputObservation::Event(_) => unreachable!("document representation"),
        };
        if let Some(error) = execution_error.borrow_mut().take() {
            return Err(error);
        }
        if stopped.get() {
            return Ok(false);
        }
        result.map_err(|message| {
            RunError::Input(FormatError::Parse {
                format: format.descriptor().input,
                message,
            })
        })?;
    }
    Ok(true)
}

fn emit_value_events(
    value: &Value,
    consumer: &mut impl tq_toon::EventConsumer<Error = String>,
) -> Result<(), String> {
    use tq_toon::Event;

    fn emit(
        value: &Value,
        span: tq_core::Span,
        consumer: &mut impl tq_toon::EventConsumer<Error = String>,
    ) -> Result<(), String> {
        use tq_toon::{Event, Scalar};
        match value {
            Value::Null => consumer.consume(Event::Scalar {
                span,
                value: Scalar::Null,
            }),
            Value::Bool(value) => consumer.consume(Event::Scalar {
                span,
                value: Scalar::Bool(*value),
            }),
            Value::Number(value) => consumer.consume(Event::Scalar {
                span,
                value: Scalar::Number(value.clone()),
            }),
            Value::String(value) => consumer.consume(Event::Scalar {
                span,
                value: Scalar::String(Arc::clone(value)),
            }),
            Value::Array(values) => {
                consumer.consume(Event::ArrayStart {
                    span,
                    declared_count: Some(u64::try_from(values.len()).unwrap_or(u64::MAX)),
                })?;
                for value in values.iter() {
                    emit(value, span, consumer)?;
                }
                consumer.consume(Event::ArrayEnd {
                    span,
                    observed_count: u64::try_from(values.len()).unwrap_or(u64::MAX),
                })
            }
            Value::Object(values) => {
                consumer.consume(Event::ObjectStart { span })?;
                for (key, value) in values.iter() {
                    consumer.consume(Event::Key {
                        span,
                        value: Arc::clone(key),
                        quoted: false,
                    })?;
                    emit(value, span, consumer)?;
                }
                consumer.consume(Event::ObjectEnd { span })
            }
        }
    }
    let span = tq_core::Span::new(SourceId::new(0), 0, 0);

    consumer.consume(Event::DocumentStart { span })?;
    emit(value, span, consumer)?;
    consumer.consume(Event::DocumentEnd { span })
}
struct StreamExecutor<'a, W, E> {
    plan: &'a Plan<Compiled, Events>,
    variables: Arc<BTreeMap<Arc<str>, Value>>,
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
            Arc::clone(&self.variables),
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
