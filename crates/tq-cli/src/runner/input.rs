//! Read structured, raw, and pull-driven document input.

use super::detection::json_sequence_input_requested;
use super::events::project_committed_input;
use super::{INPUT_BUFFER_BYTES, RunError};
use crate::RunOptions;
use std::fs::File;
use std::io;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, SyncSender};
use tq_core::{InputCursor, InputValue, JsonInputOptions, Value, VmError};
use tq_formats::{
    DecodeOptions, FormatError, InputFormat, InputRepresentation, JsonDocumentSource,
    JsonLinesDocumentSource, NativeFormat, NativeInputObservation, decode_bytes, probe_format,
    probe_reader,
};

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "the command lifecycle keeps pre-input planning visibly ahead of decoder execution"
)]
pub(super) enum StructuredInput {
    Value(Value),
    Document(InputValue),
    Proxy(Vec<u8>),
    Failure(tq_formats::NativeInputFailure),
    Warning {
        message: Arc<str>,
        context: Arc<str>,
    },
}

pub(super) fn render_native_failure(
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

pub(super) fn render_recovery_warning(
    writer: &mut impl Write,
    context: &str,
    message: &str,
) -> io::Result<()> {
    writeln!(writer, "tq: ignoring parse error {context}: {message}")
}
pub(super) fn for_each_structured_input<R: Read, F>(
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
pub(super) fn proxyable_format_error(error: &FormatError) -> bool {
    matches!(
        error,
        FormatError::Diagnostic(_)
            | FormatError::Parse { .. }
            | FormatError::Probe { .. }
            | FormatError::UnsupportedYaml(_)
    )
}

pub(super) fn validate_proxy_event_source(
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

pub(super) struct LimitedReader<R> {
    reader: R,
    remaining: u64,
    exhausted: bool,
    identity: String,
}

impl<R> LimitedReader<R> {
    pub(super) fn new(reader: R, limit: u64, identity: &str) -> Self {
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
pub(super) enum RemainingInputMessage {
    Value(InputValue),
    Failure(tq_formats::NativeInputFailure),
    Proxy(Vec<u8>),
    Error(VmError),
    Done,
}

pub(super) fn produce_remaining_inputs<R: Read>(
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

pub(super) fn produce_proxy_remaining_inputs<R: Read>(
    options: &RunOptions,
    stdin: &mut R,
    requests: &Receiver<()>,
    sender: &SyncSender<RemainingInputMessage>,
) {
    let result = produce_proxy_remaining_inputs_inner(options, stdin, requests, sender);
    let message = match result {
        Ok(true) => RemainingInputMessage::Done,
        Ok(false) => {
            if requests.recv().is_err() {
                return;
            }
            RemainingInputMessage::Done
        }
        Err(error) => RemainingInputMessage::Error(deferred_run_error(error)),
    };
    let _ = sender.send(message);
}

fn produce_proxy_remaining_inputs_inner<R: Read>(
    options: &RunOptions,
    stdin: &mut R,
    requests: &Receiver<()>,
    sender: &SyncSender<RemainingInputMessage>,
) -> Result<bool, RunError> {
    let mut pending_request = false;
    let files = if options.files.is_empty() {
        vec![Path::new("-").to_owned()]
    } else {
        options.files.clone()
    };
    for path in files {
        if !pending_request {
            if requests.recv().is_err() {
                return Ok(false);
            }
            pending_request = true;
        }
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
        let format = selected_input_format(options, &path);
        match native_source_documents(&bytes, &identity, options, format) {
            Ok((_, documents)) => {
                for document in documents {
                    if !send_remaining(sender, document) {
                        return Ok(false);
                    }
                    if requests.recv().is_err() {
                        return Ok(false);
                    }
                    pending_request = true;
                }
            }
            Err(error) if options.proxy_on_error && proxyable_format_error(&error) => {
                if sender.send(RemainingInputMessage::Proxy(bytes)).is_err() {
                    return Ok(false);
                }
                pending_request = false;
            }
            Err(error) => return Err(error.into()),
        }
    }
    Ok(pending_request)
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

pub(super) fn pull_remaining_input(
    cursor: &InputCursor,
) -> Result<Option<StructuredInput>, RunError> {
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

pub(super) enum LoadedInputs {
    Documents(Vec<tq_formats::Document>),
    Proxy(Vec<u8>),
}

pub(super) fn load_inputs<R: Read>(
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
    if options.raw_input && options.slurp {
        return load_raw_slurp(options, stdin, &files);
    }
    let mut documents = Vec::new();
    let mut raw_sources = Vec::new();
    let mut proxy = false;
    for path in files {
        let (identity, bytes) = read_input_source(&mut *stdin, &path, options.limits.input_bytes)?;
        if options.raw_input {
            raw_documents(&mut documents, identity, &bytes, false)?;
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

fn load_raw_slurp<R: Read>(
    options: &RunOptions,
    stdin: &mut R,
    files: &[PathBuf],
) -> Result<LoadedInputs, RunError> {
    let mut bytes = Vec::new();
    let mut identity = None;
    let mut line_number = 0;
    for path in files {
        let remaining = options
            .limits
            .input_bytes
            .saturating_sub(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
        let (source_identity, source_bytes) = read_input_source(&mut *stdin, path, remaining)?;
        append_raw_slurp_source(
            &mut bytes,
            &mut identity,
            &mut line_number,
            source_identity,
            &source_bytes,
        )?;
    }
    let mut documents = Vec::new();
    raw_documents(
        &mut documents,
        identity.unwrap_or_else(|| "<stdin>".to_owned()),
        &bytes,
        true,
    )?;
    documents[0].line_number = line_number;
    Ok(LoadedInputs::Documents(documents))
}

fn read_input_source<R: Read>(
    stdin: &mut R,
    path: &Path,
    limit: u64,
) -> Result<(String, Vec<u8>), RunError> {
    if path == Path::new("-") {
        Ok(("<stdin>".to_owned(), read_limited(stdin, limit, "<stdin>")?))
    } else {
        let identity = path.display().to_string();
        let bytes = read_limited(open_path(path)?, limit, &identity)?;
        Ok((identity, bytes))
    }
}

fn append_raw_slurp_source(
    aggregate: &mut Vec<u8>,
    identity: &mut Option<String>,
    line_number: &mut u64,
    source_identity: String,
    source_bytes: &[u8],
) -> Result<(), RunError> {
    raw_input_text(source_bytes)?;
    *line_number = raw_line_number(source_bytes);
    aggregate
        .try_reserve(source_bytes.len())
        .map_err(|_| RunError::ResourceSource {
            identity: source_identity.clone(),
            resource: "input-bytes",
        })?;
    aggregate.extend_from_slice(source_bytes);
    *identity = Some(source_identity);
    Ok(())
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

pub(super) fn read_limited(
    mut reader: impl Read,
    limit: u64,
    identity: &str,
) -> Result<Vec<u8>, RunError> {
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

pub(super) fn open_path(path: &Path) -> Result<File, RunError> {
    File::open(path).map_err(|source| RunError::IoPath {
        path: path.display().to_string(),
        source,
    })
}

pub(super) fn format_from_path(path: &Path) -> Option<InputFormat> {
    let extension = path.extension()?.to_str()?;
    tq_formats::NativeFormat::from_extension(extension).map(|format| format.descriptor().input)
}

pub(super) fn selected_input_format(options: &RunOptions, path: &Path) -> InputFormat {
    if json_sequence_input_requested(options) {
        return InputFormat::JsonSequence;
    }
    if options.input_format == InputFormat::Auto && path != Path::new("-") {
        format_from_path(path).unwrap_or(InputFormat::Auto)
    } else {
        options.input_format
    }
}

pub(super) fn toon_decoder_config(options: &RunOptions) -> tq_toon::DecoderConfig {
    tq_toon::DecoderConfig {
        strict: options.strict,
        maximum_fields: options.limits.fields,
        maximum_name_bytes: options.limits.preparation_memory_bytes,
        maximum_depth: options.limits.depth,
        maximum_token_bytes: options.limits.token_bytes,
        maximum_line_bytes: options.limits.line_bytes,
        maximum_lookahead_bytes: options.limits.lookahead_bytes,
        ..tq_toon::DecoderConfig::default()
    }
}

pub(super) fn decode_options(options: &RunOptions, format: InputFormat) -> DecodeOptions {
    DecodeOptions {
        format,
        maximum_source_bytes: usize::try_from(options.limits.input_bytes).unwrap_or(usize::MAX),
        maximum_frame_bytes: options.limits.frame_bytes,
        maximum_depth: options.limits.depth,
        maximum_token_bytes: options.limits.token_bytes,
        maximum_line_bytes: options.limits.line_bytes,
        maximum_fields: options.limits.fields,
        toon: toon_decoder_config(options),
    }
}

fn raw_documents(
    documents: &mut Vec<tq_formats::Document>,
    identity: String,
    bytes: &[u8],
    slurp: bool,
) -> Result<(), RunError> {
    let text = raw_input_text(bytes)?;
    if slurp {
        documents.push(tq_formats::Document {
            value: Value::string(text),
            identity,
            format: InputFormat::Auto,
            index: 0,
            line_number: raw_line_number(bytes),
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

fn raw_line_number(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0_u64, |count, byte| {
        if *byte == b'\n' {
            count.saturating_add(1)
        } else {
            count
        }
    })
}

fn raw_input_text(bytes: &[u8]) -> Result<&str, RunError> {
    std::str::from_utf8(bytes).map_err(|error| {
        RunError::Input(FormatError::Parse {
            format: InputFormat::Auto,
            message: format!("raw input is not UTF-8: {error}"),
        })
    })
}
