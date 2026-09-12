//! Incremental jq path/value stream projection for JSON and TOON.

use std::{
    cell::{Cell, RefCell},
    fmt::Display,
    io::{self, BufRead, Read},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

const STREAM_ERROR_CONTEXT_BYTES: usize = 64 * 1024;

use tq_core::{JsonInputError, JsonLimit, Number, Object, PathComponent, SourceId, Value};
use tq_toon::{DecodeIntoError, Decoder, Event, EventConsumer, Scalar};

use crate::structural::{
    decode_json_event_stream_typed, decode_json_events_selected_with_options_control_typed,
};
use crate::{FormatError, InputFormat, JsonEventOptions, decode_json_events_with_options};

/// Limits and error behavior for explicit jq path/value streaming.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StreamOptions {
    /// Maximum active JSON containers.
    pub maximum_depth: usize,
    /// Maximum decoded bytes in one string, key, or number token.
    pub maximum_token_bytes: usize,
    /// Emit JSON parse failures as jq-shaped stream error values.
    pub errors_as_values: bool,
}

/// Work observations from one selected structural decode.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SelectedStreamObservations {
    /// Largest number of simultaneously active source containers.
    ///
    /// This includes containers validated by the selection-aware parser even
    /// when their values are discarded before materialization.
    pub depth_high_water: usize,
}

impl Default for StreamOptions {
    fn default() -> Self {
        Self {
            maximum_depth: 256,
            maximum_token_bytes: 8 * 1024 * 1024,
            errors_as_values: false,
        }
    }
}

/// One decoded jq stream record before wrapper-value allocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamRecord {
    /// Ordered key and index path.
    pub path: Vec<PathComponent>,
    /// Leaf or empty-container value. `None` marks a completed non-empty container.
    pub value: Option<Value>,
    raw: bool,
}

/// Static decoder path admitted by an automatic projection proof.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamSelection {
    prefix: Vec<PathComponent>,
    mode: SelectionMode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum SelectionMode {
    Complete,
    Projection(Vec<PathComponent>),
    ItemCapture(Vec<Vec<PathComponent>>),
}

/// A selected root value that cannot be represented by selected child records
/// alone.  Non-empty containers continue to arrive as records; these variants
/// therefore never require retaining a complete input document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SelectionFallback {
    /// The selected path was absent from the root.
    Missing {
        /// Path of the absent selected prefix.
        path: Vec<PathComponent>,
    },
    /// The selected path contained a scalar, including JSON null.
    PresentScalar {
        /// Path of the selected scalar.
        path: Vec<PathComponent>,
        /// Scalar value at the selected path.
        value: Value,
    },
    /// The selected path contained an empty array.
    PresentEmptyArray {
        /// Path of the empty array.
        path: Vec<PathComponent>,
    },
    /// The selected path contained an empty object.
    PresentEmptyObject {
        /// Path of the empty object.
        path: Vec<PathComponent>,
    },
}

/// A replacement observed before the replacement value's selected children.
/// Array and object variants mean that a container has started; its eventual
/// emptiness is reported by the normal selected records.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SelectionReplacement {
    /// The replacement has no value at this path.
    Missing,
    /// The replacement is a scalar, including JSON null.
    Scalar(Value),
    /// An array replacement has started.
    ArrayStart,
    /// An object replacement has started.
    ObjectStart,
}

/// Receives selected records grouped by validated JSON root boundaries.
pub trait SelectedRootSink {
    /// Sink-specific error mapped through its display representation.
    type Error: Display;

    /// Begins one root before its selected records are delivered.
    ///
    /// # Errors
    ///
    /// Returns a sink error when the root cannot be opened.
    fn begin_root(&mut self, index: u64) -> Result<(), Self::Error>;

    /// Reports an absent or scalar/empty selected prefix.
    ///
    /// # Errors
    ///
    /// Returns a sink error when the fallback cannot be recorded.
    fn fallback(&mut self, fallback: SelectionFallback) -> Result<(), Self::Error>;

    /// Reports replacement of the selected prefix or one of its ancestors.
    ///
    /// # Errors
    ///
    /// Returns a sink error when the replacement cannot be recorded.
    fn replace_selected_prefix(
        &mut self,
        path: &[PathComponent],
        replacement: SelectionReplacement,
    ) -> Result<(), Self::Error>;

    /// Reports replacement inside an already selected input item.
    ///
    /// # Errors
    ///
    /// Returns a sink error when the replacement cannot be recorded.
    fn replace_item_path(
        &mut self,
        path: &[PathComponent],
        replacement: SelectionReplacement,
    ) -> Result<(), Self::Error>;

    /// Delivers one selected structural record.
    ///
    /// # Errors
    ///
    /// Returns a sink error when the record cannot be retained.
    fn record(&mut self, record: StreamRecord) -> Result<(), Self::Error>;

    /// Commits the current root.  This is the only commit point.
    ///
    /// # Errors
    ///
    /// Returns a sink error when the root cannot be committed.
    fn finish_root(&mut self) -> Result<(), Self::Error>;

    /// Discards all state accumulated for the current malformed root.
    fn abort_root(&mut self);
}

impl StreamSelection {
    /// Creates a selection for direct children below `prefix` and an optional
    /// path projected from each child.
    #[must_use]
    pub fn new(prefix: Vec<PathComponent>, projection: Option<Vec<PathComponent>>) -> Self {
        let mode = match projection {
            Some(projection) => SelectionMode::Projection(projection),
            None => SelectionMode::Complete,
        };
        Self { prefix, mode }
    }

    /// Creates a JSON selection that retains only the statically proven paths
    /// inside each selected item.  This mode is deliberately separate from
    /// projection: callers cannot accidentally combine two independent
    /// retention descriptions and lose a required value.  An empty path list
    /// conservatively selects complete items.
    #[must_use]
    pub fn with_item_capture_paths(
        prefix: Vec<PathComponent>,
        capture_paths: Vec<Vec<PathComponent>>,
    ) -> Self {
        if capture_paths.is_empty() {
            return Self::new(prefix, None);
        }
        Self {
            prefix,
            mode: SelectionMode::ItemCapture(capture_paths),
        }
    }

    /// Returns the statically selected array path.
    #[must_use]
    pub fn prefix(&self) -> &[PathComponent] {
        &self.prefix
    }

    /// Returns the static element-local projection, when present.
    #[must_use]
    pub fn projection(&self) -> Option<&[PathComponent]> {
        match &self.mode {
            SelectionMode::Projection(projection) => Some(projection),
            SelectionMode::Complete | SelectionMode::ItemCapture(_) => None,
        }
    }

    fn item_capture_paths(&self) -> Option<&[Vec<PathComponent>]> {
        match &self.mode {
            SelectionMode::ItemCapture(paths) => Some(paths),
            SelectionMode::Complete | SelectionMode::Projection(_) => None,
        }
    }

    fn capture_shell_required(&self, path: &[PathComponent]) -> bool {
        let Some(capture_paths) = self.item_capture_paths() else {
            return false;
        };
        if path.len() <= self.prefix.len() || !path.starts_with(&self.prefix) {
            return false;
        }
        let item_relative = &path[self.prefix.len() + 1..];
        capture_paths
            .iter()
            .any(|capture_path| capture_path.starts_with(item_relative))
    }

    fn keeps(&self, path: &[PathComponent]) -> bool {
        if path.len() <= self.prefix.len() || !path.starts_with(&self.prefix) {
            return false;
        }
        let relative = &path[self.prefix.len()..];
        if relative.len() == 1 {
            return true;
        }
        let item_relative = &relative[1..];
        match &self.mode {
            SelectionMode::Complete => true,
            SelectionMode::Projection(projection) => {
                projection.starts_with(item_relative)
                    || item_relative.starts_with(projection)
                    || path_kind_mismatch(item_relative, projection)
            }
            SelectionMode::ItemCapture(capture_paths) => capture_paths.iter().any(|capture_path| {
                capture_path.starts_with(item_relative)
                    || item_relative.starts_with(capture_path)
                    || path_kind_mismatch(item_relative, capture_path)
            }),
        }
    }

    fn tracks(&self, path: &[PathComponent]) -> bool {
        if path.len() <= self.prefix.len() {
            return self.prefix.starts_with(path);
        }
        self.keeps(path)
    }
}

impl StreamRecord {
    /// Decomposes a normal structural record into its path and optional value.
    #[must_use]
    pub fn into_parts(self) -> (Vec<PathComponent>, Option<Value>) {
        (self.path, self.value)
    }

    /// Materializes the jq path/value record, or its error value.
    #[must_use]
    pub fn into_value(self) -> Value {
        if self.raw {
            return self.value.unwrap_or(Value::Null);
        }
        let mut parts = vec![path_value(&self.path)];
        if let Some(value) = self.value {
            parts.push(value);
        }
        Value::array(parts)
    }

    fn path(path: Vec<PathComponent>, value: Option<Value>) -> Self {
        Self {
            path,
            value,
            raw: false,
        }
    }

    pub(crate) fn rebase_array_item(
        mut self,
        prefix: &[PathComponent],
        first_index: usize,
    ) -> Self {
        if self.raw || self.path.is_empty() {
            return self;
        }
        let PathComponent::Index(local_index) = self.path[0] else {
            return self;
        };
        let mut path = Vec::with_capacity(prefix.len().saturating_add(self.path.len()));
        path.extend_from_slice(prefix);
        path.push(PathComponent::Index(
            first_index.saturating_add(local_index),
        ));
        path.extend(self.path.drain(1..));
        self.path = path;
        self
    }

    fn raw(value: Value) -> Self {
        Self {
            path: Vec::new(),
            value: Some(value),
            raw: true,
        }
    }
}

/// Streams one JSON document as jq-compatible `[path,value]` leaf records and
/// `[path]` container-end records without constructing its DOM.
///
/// The callback is invoked before more input is consumed. Returning an error
/// stops decoding immediately; callers that need a typed callback error can
/// retain it beside the closure and return a bounded marker string here.
///
/// # Errors
///
/// Returns JSON syntax, numeric-envelope, depth, or callback failures.
pub fn stream_json<R, F>(reader: R, options: StreamOptions, mut emit: F) -> Result<(), FormatError>
where
    R: std::io::Read,
    F: FnMut(Value) -> Result<(), String>,
{
    let mut emit_record = |record: StreamRecord| emit(record.into_value());
    let mut consumer = EventProjector {
        projector: Projector::new(
            options.maximum_depth,
            options.maximum_token_bytes,
            &mut emit_record,
        ),
    };
    if options.errors_as_values {
        consumer.projector.defer_error_record();
    }
    let source_bytes = Arc::new(Mutex::new(StreamContext::default()));
    let decoded = decode_json_event_stream_typed(
        TrackingReader::new(reader, Arc::clone(&source_bytes)),
        SourceId::new(0),
        &mut consumer,
        JsonEventOptions {
            maximum_depth: options.maximum_depth,
            maximum_token_bytes: options.maximum_token_bytes,
        },
    );
    let source_context = source_bytes
        .lock()
        .map_err(|_| FormatError::Parse {
            format: InputFormat::Json,
            message: "JSON stream error context unavailable".to_owned(),
        })?
        .clone();
    match decoded {
        Ok(_) => consumer
            .projector
            .flush_deferred()
            .map_err(|message| FormatError::Parse {
                format: InputFormat::Json,
                message,
            }),
        Err(JsonInputError::Syntax { position, message }) if options.errors_as_values => consumer
            .projector
            .error_value(
                jq_stream_error_from_input(position, &message, &source_context),
                false,
                stream_error_path_is_null(&message),
                stream_error_path_prefers_last_value(&message),
            )
            .and_then(|()| consumer.projector.flush_deferred())
            .map_err(|message| FormatError::Parse {
                format: InputFormat::Json,
                message,
            }),
        Err(JsonInputError::Consumer(error) | JsonInputError::Io(error)) => {
            Err(FormatError::Parse {
                format: InputFormat::Json,
                message: error.to_string(),
            })
        }
        Err(error) => Err(FormatError::Parse {
            format: InputFormat::Json,
            message: error.to_string(),
        }),
    }
}

#[derive(Clone, Debug)]
struct StreamContext {
    bytes: Vec<u8>,
    first_line: usize,
    first_column: usize,
}

impl Default for StreamContext {
    fn default() -> Self {
        Self {
            bytes: Vec::new(),
            first_line: 1,
            first_column: 1,
        }
    }
}

impl StreamContext {
    fn append(&mut self, input: &[u8]) {
        let drop = self
            .bytes
            .len()
            .saturating_add(input.len())
            .saturating_sub(STREAM_ERROR_CONTEXT_BYTES);
        let existing_drop = drop.min(self.bytes.len());
        let incoming_drop = drop.saturating_sub(existing_drop);
        advance_context_position(
            &mut self.first_line,
            &mut self.first_column,
            &self.bytes[..existing_drop],
        );
        advance_context_position(
            &mut self.first_line,
            &mut self.first_column,
            &input[..incoming_drop],
        );
        if existing_drop != 0 {
            self.bytes.drain(..existing_drop);
        }
        self.bytes.extend_from_slice(&input[incoming_drop..]);
    }
}

fn advance_context_position(line: &mut usize, column: &mut usize, bytes: &[u8]) {
    for byte in bytes {
        if *byte == b'\n' {
            *line = (*line).saturating_add(1);
            *column = 1;
        } else {
            *column = (*column).saturating_add(1);
        }
    }
}

struct TrackingReader<R> {
    reader: R,
    bytes: Arc<Mutex<StreamContext>>,
}

impl<R> TrackingReader<R> {
    fn new(reader: R, bytes: Arc<Mutex<StreamContext>>) -> Self {
        Self { reader, bytes }
    }
}

impl<R: Read> Read for TrackingReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let count = self.reader.read(buffer)?;
        if count != 0 {
            let mut bytes = self
                .bytes
                .lock()
                .map_err(|_| io::Error::other("JSON stream context unavailable"))?;
            bytes.append(&buffer[..count]);
        }
        Ok(count)
    }
}

/// Streams JSON through the shared structural decoder without constructing jq
/// `[path, value]` wrapper values.
///
/// # Errors
///
/// Returns JSON syntax, numeric-envelope, depth, token, or callback failures.
pub fn stream_json_records<R, F>(
    reader: R,
    options: StreamOptions,
    mut emit: F,
) -> Result<(), FormatError>
where
    R: std::io::Read,
    F: FnMut(StreamRecord) -> Result<(), String>,
{
    let mut consumer = EventProjector {
        projector: Projector::new(
            options.maximum_depth,
            options.maximum_token_bytes,
            &mut emit,
        ),
    };
    decode_json_events_with_options(
        reader,
        SourceId::new(0),
        &mut consumer,
        JsonEventOptions {
            maximum_depth: options.maximum_depth,
            maximum_token_bytes: options.maximum_token_bytes,
        },
    )
    .map_err(|message| FormatError::Parse {
        format: InputFormat::Json,
        message,
    })
}

/// Streams only records needed by a proven static automatic projection.
/// Discarded values are validated by the shared JSON grammar so syntax and
/// resource failures remain observable without retaining discarded subtrees
/// or strings. Numeric envelope validation may still construct a temporary
/// number for a discarded numeric token.
///
/// # Errors
///
/// Returns JSON syntax, numeric-envelope, depth, token, or callback failures.
pub fn stream_json_selected_records<R, F>(
    reader: R,
    options: StreamOptions,
    selection: StreamSelection,
    emit: F,
) -> Result<(), FormatError>
where
    R: std::io::Read,
    F: FnMut(StreamRecord) -> Result<(), String>,
{
    let mut observations = SelectedStreamObservations::default();
    stream_json_selected_records_with_control(
        reader,
        options,
        selection,
        None,
        &mut observations,
        emit,
    )
}

/// Streams selected JSON records with cooperative cancellation and observations.
///
/// Cancellation is checked throughout lexical scanning, including while the
/// shared parser validates a discarded subtree.
///
/// # Errors
///
/// Returns JSON syntax, numeric-envelope, depth, token, cancellation, or callback failures.
pub fn stream_json_selected_records_with_control<R, F>(
    reader: R,
    options: StreamOptions,
    selection: StreamSelection,
    cancellation: Option<Arc<AtomicBool>>,
    observations: &mut SelectedStreamObservations,
    mut emit: F,
) -> Result<(), FormatError>
where
    R: Read,
    F: FnMut(StreamRecord) -> Result<(), String>,
{
    decode_selected_json(
        reader,
        options,
        selection,
        cancellation,
        observations,
        &mut emit,
    )
}

/// Streams selected JSON records one validated root at a time.
///
/// Records are delivered to the sink while the current root is being parsed,
/// but the sink's `finish_root` callback is the only commit point.  A syntax,
/// resource, cancellation, or sink error calls `abort_root` for the current
/// root before the error is returned.  Existing selected-record APIs retain
/// their one-document behavior and are intentionally unchanged.
///
/// # Errors
///
/// Returns JSON syntax, numeric-envelope, resource-limit, cancellation, or
/// sink failures.
pub fn stream_json_selected_roots_with_control<R, S>(
    reader: R,
    options: StreamOptions,
    selection: StreamSelection,
    cancellation: Option<Arc<AtomicBool>>,
    observations: &mut SelectedStreamObservations,
    sink: &mut S,
) -> Result<u64, FormatError>
where
    R: Read,
    S: SelectedRootSink,
{
    let selected_prefix = selection.prefix().to_vec();
    let parser_selection = selection.clone();
    let sink = RefCell::new(sink);
    let mut emit = |record: StreamRecord| {
        sink.borrow_mut()
            .record(record)
            .map_err(|error| error.to_string())
    };
    let mut fallback = |fallback: SelectionFallback| {
        sink.borrow_mut()
            .fallback(fallback)
            .map_err(|error| error.to_string())
    };
    let mut replacement = |path: &[PathComponent], replacement: SelectionReplacement| {
        if path == selected_prefix.as_slice() || selected_prefix.starts_with(path) {
            sink.borrow_mut().replace_selected_prefix(path, replacement)
        } else {
            sink.borrow_mut().replace_item_path(path, replacement)
        }
        .map_err(|error| error.to_string())
    };
    let mut projector = Projector::selected_with_lifecycle(
        options.maximum_depth,
        options.maximum_token_bytes,
        selection,
        &mut emit,
        &mut fallback,
        &mut replacement,
    );
    projector.cancellation.clone_from(&cancellation);
    let mut consumer = EventProjector { projector };
    let mut begin_root = |index| {
        sink.borrow_mut()
            .begin_root(index)
            .map_err(|error| error.to_string())
    };
    let mut finish_root = || {
        sink.borrow_mut()
            .finish_root()
            .map_err(|error| error.to_string())
    };
    let mut abort_root = || sink.borrow_mut().abort_root();
    let mut retain = move |path: &[PathComponent]| parser_selection.tracks(path);
    let mut checkpoint = || Ok(());
    let result = crate::structural::decode_json_events_selected_roots_with_options_control(
        reader,
        SourceId::new(0),
        &mut consumer,
        JsonEventOptions {
            maximum_depth: options.maximum_depth,
            maximum_token_bytes: options.maximum_token_bytes,
        },
        cancellation,
        &mut begin_root,
        &mut finish_root,
        &mut abort_root,
        &mut checkpoint,
        &mut retain,
        Some(&mut observations.depth_high_water),
    );
    result.map_err(map_json_input_error)
}

fn map_json_input_error(error: JsonInputError) -> FormatError {
    match error {
        JsonInputError::Io(error)
            if error.to_string().contains("selected decoding interrupted") =>
        {
            FormatError::Resource("interrupted")
        }
        JsonInputError::Io(error) => FormatError::Io(error),
        JsonInputError::Limit { limit, .. } => FormatError::Resource(match limit {
            JsonLimit::TokenBytes => "token-bytes",
            JsonLimit::Depth => "depth",
        }),
        JsonInputError::Consumer(error)
            if error.to_string().contains("selected decoding interrupted") =>
        {
            FormatError::Resource("interrupted")
        }
        JsonInputError::Consumer(error) => FormatError::Parse {
            format: InputFormat::Json,
            message: error.to_string(),
        },
        error @ JsonInputError::Syntax { .. } => FormatError::Parse {
            format: InputFormat::Json,
            message: error.to_string(),
        },
    }
}

fn decode_selected_json<R, F>(
    reader: R,
    options: StreamOptions,
    selection: StreamSelection,
    cancellation: Option<Arc<AtomicBool>>,
    observations: &mut SelectedStreamObservations,
    emit: &mut F,
) -> Result<(), FormatError>
where
    R: Read,
    F: FnMut(StreamRecord) -> Result<(), String>,
{
    let checkpoint_cancellation = cancellation.clone();
    let parser_selection = selection.clone();
    let mut consumer = EventProjector {
        projector: Projector::selected(
            options.maximum_depth,
            options.maximum_token_bytes,
            selection,
            emit,
        ),
    };
    consumer.projector.cancellation = cancellation;
    let mut checkpoint = || {
        if checkpoint_cancellation
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
        {
            Err(io::Error::other("selected decoding interrupted"))
        } else {
            Ok(())
        }
    };
    let mut retain = |path: &[PathComponent]| parser_selection.tracks(path);
    let result = decode_json_events_selected_with_options_control_typed(
        reader,
        SourceId::new(0),
        &mut consumer,
        JsonEventOptions {
            maximum_depth: options.maximum_depth,
            maximum_token_bytes: options.maximum_token_bytes,
        },
        &mut checkpoint,
        &mut retain,
        true,
        Some(&mut observations.depth_high_water),
    );
    observations.depth_high_water = observations
        .depth_high_water
        .max(consumer.projector.depth_high_water.get());
    result.map_err(|error| match error {
        JsonInputError::Io(error) | JsonInputError::Consumer(error)
            if error.to_string().contains("selected decoding interrupted") =>
        {
            FormatError::Parse {
                format: InputFormat::Json,
                message: error.to_string(),
            }
        }
        error => map_json_input_error(error),
    })
}

fn jq_stream_error_message(message: String) -> String {
    if let Some(position) = message.strip_prefix("expected value at line ")
        && let Some((line, column)) = position.split_once(" column ")
        && let Ok(column) = column.parse::<usize>()
    {
        // jq consumes the invalid bare token before reporting its endpoint.
        // serde_json stops at the first byte; the MVP stream-error grammar's
        // only bare-token class is a three-byte JSON literal prefix.
        return format!(
            "Invalid numeric literal at line {}, column {}",
            line,
            column.saturating_add(3)
        );
    }
    message
}

fn jq_stream_error_from_input(
    position: tq_core::JsonPosition,
    detail: &str,
    source: &StreamContext,
) -> String {
    let line = position.line;
    let column = position.column;
    let Some((line_bytes, line_start_column)) = source_line(source, line) else {
        return jq_stream_error_without_context(position, detail);
    };
    let Some(raw_index) = column.checked_sub(line_start_column) else {
        return jq_stream_error_without_context(position, detail);
    };
    if raw_index > line_bytes.len() {
        return jq_stream_error_without_context(position, detail);
    }
    let index = raw_index;
    let at_eof = index >= line_bytes.len();
    if at_eof {
        if detail.starts_with("unterminated string") {
            return format!(
                "Unfinished string at EOF at line {line}, column {}",
                column.saturating_sub(1)
            );
        }
        if detail.starts_with("expected ',' or ']'") || detail.starts_with("expected ',' or '}'") {
            return format_separator_error(line_bytes, index, line, line_start_column, true);
        }
        if detail.starts_with("expected object key")
            || detail.starts_with("expected ':' after object key")
            || detail.starts_with("expected JSON value")
        {
            return format!(
                "Unfinished JSON term at EOF at line {line}, column {}",
                column.saturating_sub(1)
            );
        }
    }
    if detail.starts_with("unpaired high surrogate") || detail.starts_with("invalid surrogate pair")
    {
        return format!(
            "Invalid \\uXXXX\\uXXXX surrogate pair escape at line {line}, column {column}"
        );
    }
    if detail.starts_with("expected object key after ','") && line_bytes.get(index) == Some(&b'}') {
        return format!("Expected another key:value pair at line {line}, column {column}");
    }
    if detail.starts_with("expected ':' after object key") {
        return format_separator_error(line_bytes, index, line, line_start_column, false);
    }
    if detail.starts_with("expected ',' or ']'") || detail.starts_with("expected ',' or '}'") {
        return format_separator_error(line_bytes, index, line, line_start_column, at_eof);
    }
    if detail.starts_with("expected JSON value") {
        if line_bytes.get(index) == Some(&b'}') && line_bytes[..index].contains(&b':') {
            return format!("Missing value in key:value pair at line {line}, column {column}");
        }
        if line_bytes.get(index) == Some(&b']') {
            return format!("Expected another array element at line {line}, column {column}");
        }
        if let Some((start, end)) = invalid_token_bounds(line_bytes, index) {
            return format!(
                "{} at line {line}, column {}",
                invalid_token_kind(&line_bytes[start..end]),
                line_start_column.saturating_add(end)
            );
        }
    }
    if detail.starts_with("expected ',' or ']'") {
        return format!("Expected another array element at line {line}, column {column}");
    }
    if detail.starts_with("invalid numeric literal") {
        if let Some((start, end)) = invalid_token_bounds(line_bytes, index) {
            return format!(
                "{} at line {line}, column {}",
                invalid_token_kind(&line_bytes[start..end]),
                line_start_column.saturating_add(end)
            );
        }
        return format!("Invalid numeric literal at line {line}, column {column}");
    }
    if detail.starts_with("invalid literal") {
        return format!("Invalid literal at line {line}, column {column}");
    }
    jq_stream_error_without_context(position, detail)
}

fn format_separator_error(
    line_bytes: &[u8],
    index: usize,
    line: usize,
    line_start_column: usize,
    at_eof: bool,
) -> String {
    if at_eof {
        return format!(
            "Expected separator between values at EOF at line {line}, column {}",
            line_start_column
                .saturating_add(line_bytes.len())
                .saturating_sub(1)
        );
    }
    let column = if line_bytes.get(index) == Some(&b'"') {
        quoted_token_end(line_bytes, index).map_or(line_start_column.saturating_add(index), |end| {
            line_start_column.saturating_add(end)
        })
    } else {
        invalid_token_bounds(line_bytes, index)
            .map_or(line_start_column.saturating_add(index), |(_, end)| {
                line_start_column.saturating_add(end)
            })
    };
    format!("Expected separator between values at line {line}, column {column}")
}

fn quoted_token_end(line: &[u8], start: usize) -> Option<usize> {
    let mut escaped = false;
    for (index, byte) in line.iter().enumerate().skip(start.saturating_add(1)) {
        if escaped {
            escaped = false;
        } else if *byte == b'\\' {
            escaped = true;
        } else if *byte == b'"' {
            return Some(index);
        }
    }
    None
}

fn stream_error_path_is_null(detail: &str) -> bool {
    detail.starts_with("expected object key after ','")
        || detail.starts_with("expected ':' after object key")
        || detail.starts_with("expected object key or '}'")
}

fn stream_error_path_prefers_last_value(detail: &str) -> bool {
    detail.starts_with("expected ',' or ']'") || detail.starts_with("expected ',' or '}'")
}

fn jq_stream_error_without_context(position: tq_core::JsonPosition, detail: &str) -> String {
    let line = position.line;
    let column = position.column;
    if detail.starts_with("expected ',' or ']'") {
        return format!("Expected another array element at line {line}, column {column}");
    }
    if detail.starts_with("invalid numeric literal") {
        return format!("Invalid numeric literal at line {line}, column {column}");
    }
    if detail.starts_with("invalid literal") {
        return format!("Invalid literal at line {line}, column {column}");
    }
    format!("{detail} at line {line}, column {column}")
}

fn source_line(source: &StreamContext, line: usize) -> Option<(&[u8], usize)> {
    if line < source.first_line {
        return None;
    }
    let mut current = source.first_line;
    let mut start = 0_usize;
    if current == line {
        for (index, byte) in source.bytes.iter().enumerate() {
            if *byte == b'\n' {
                return Some((&source.bytes[..=index], source.first_column));
            }
        }
        return Some((&source.bytes, source.first_column));
    }
    for (index, byte) in source.bytes.iter().enumerate() {
        if *byte == b'\n' {
            if current == line {
                return Some((&source.bytes[start..=index], 1));
            }
            current = current.saturating_add(1);
            start = index.saturating_add(1);
        }
    }
    (current == line).then_some((&source.bytes[start..], 1))
}

fn invalid_token_bounds(line: &[u8], error_index: usize) -> Option<(usize, usize)> {
    let mut start = error_index.min(line.len());
    while start > 0 && !matches!(line[start - 1], b'[' | b'{' | b',' | b':' | b' ' | b'\t') {
        start -= 1;
    }
    let mut end = error_index.min(line.len());
    while end < line.len() && !matches!(line[end], b']' | b'}' | b',' | b' ' | b'\t' | b'\n') {
        end += 1;
    }
    (start < end).then_some((start, end))
}

fn invalid_token_kind(token: &[u8]) -> &'static str {
    if token == b"n" {
        "Invalid numeric literal"
    } else if matches!(token.first(), Some(b't' | b'f' | b'n')) {
        "Invalid literal"
    } else {
        "Invalid numeric literal"
    }
}

/// Streams one TOON document through its query-independent event decoder and
/// projects the same jq path/value contract as [`stream_json`].
///
/// # Errors
///
/// Returns strict TOON decoding, resource, structural, or callback failures.
pub fn stream_toon<R, F>(
    reader: R,
    config: tq_toon::DecoderConfig,
    options: StreamOptions,
    mut emit: F,
) -> Result<(), FormatError>
where
    R: BufRead,
    F: FnMut(Value) -> Result<(), String>,
{
    let mut emit_record = |record: StreamRecord| emit(record.into_value());
    let mut decoder = Decoder::new(reader, SourceId::new(0), config);
    let mut consumer = EventProjector {
        projector: Projector::new(
            options.maximum_depth,
            options.maximum_token_bytes,
            &mut emit_record,
        ),
    };
    decoder.decode_into(&mut consumer).map_err(|error| {
        let message = match error {
            DecodeIntoError::Decode(error) => error.to_string(),
            DecodeIntoError::Consumer(error) => error,
        };
        FormatError::Parse {
            format: InputFormat::Toon,
            message,
        }
    })
}

/// Streams TOON structural events without constructing jq wrapper values.
///
/// # Errors
///
/// Returns strict decoding, resource, structural, or callback failures.
pub fn stream_toon_records<R, F>(
    reader: R,
    config: tq_toon::DecoderConfig,
    options: StreamOptions,
    mut emit: F,
) -> Result<(), FormatError>
where
    R: BufRead,
    F: FnMut(StreamRecord) -> Result<(), String>,
{
    let mut decoder = Decoder::new(reader, SourceId::new(0), config);
    let mut consumer = EventProjector {
        projector: Projector::new(
            options.maximum_depth,
            options.maximum_token_bytes,
            &mut emit,
        ),
    };
    decoder.decode_into(&mut consumer).map_err(|error| {
        let message = match error {
            DecodeIntoError::Decode(error) => error.to_string(),
            DecodeIntoError::Consumer(error) => error,
        };
        FormatError::Parse {
            format: InputFormat::Toon,
            message,
        }
    })
}

/// Streams only TOON records needed by a proven static automatic projection.
///
/// # Errors
///
/// Returns strict decoding, resource, structural, or callback failures.
pub fn stream_toon_selected_records<R, F>(
    reader: R,
    config: tq_toon::DecoderConfig,
    options: StreamOptions,
    selection: StreamSelection,
    emit: F,
) -> Result<(), FormatError>
where
    R: BufRead,
    F: FnMut(StreamRecord) -> Result<(), String>,
{
    let mut observations = SelectedStreamObservations::default();
    stream_toon_selected_records_with_control(
        reader,
        config,
        options,
        selection,
        None,
        &mut observations,
        emit,
    )
}

/// Streams selected TOON records with cooperative cancellation and observations.
///
/// # Errors
///
/// Returns strict decoding, resource, structural, cancellation, or callback failures.
pub fn stream_toon_selected_records_with_control<R, F>(
    reader: R,
    config: tq_toon::DecoderConfig,
    options: StreamOptions,
    selection: StreamSelection,
    cancellation: Option<Arc<AtomicBool>>,
    observations: &mut SelectedStreamObservations,
    mut emit: F,
) -> Result<(), FormatError>
where
    R: BufRead,
    F: FnMut(StreamRecord) -> Result<(), String>,
{
    let mut decoder = Decoder::new(reader, SourceId::new(0), config);
    let mut projector = Projector::selected(
        options.maximum_depth,
        options.maximum_token_bytes,
        selection,
        &mut emit,
    );
    projector.cancellation = cancellation;
    let mut consumer = EventProjector { projector };
    let result = decoder.decode_into(&mut consumer);
    observations.depth_high_water = observations
        .depth_high_water
        .max(consumer.projector.depth_high_water.get());
    result.map_err(|error| match error {
        DecodeIntoError::Decode(error) => crate::adapters::toon_input_error(error),
        DecodeIntoError::Consumer(message) => FormatError::Parse {
            format: InputFormat::Toon,
            message,
        },
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ContainerKind {
    Array,
    Object,
}

#[derive(Debug)]
struct Frame {
    kind: ContainerKind,
    path: Option<Vec<PathComponent>>,
    children: usize,
    retained_child: bool,
    pending_key: Option<Arc<str>>,
}

type ReplacementCallback<'a> =
    &'a mut dyn FnMut(&[PathComponent], SelectionReplacement) -> Result<(), String>;

struct Projector<'a, F> {
    frames: Vec<Frame>,
    last_path: Vec<PathComponent>,
    deferred_error_record: bool,
    pending_record: Option<StreamRecord>,
    maximum_depth: usize,
    maximum_token_bytes: usize,
    selection: Option<StreamSelection>,
    depth_high_water: Cell<usize>,
    cancellation: Option<Arc<AtomicBool>>,
    values_until_cancellation_check: Cell<usize>,
    emit: &'a mut F,
    fallback: Option<&'a mut dyn FnMut(SelectionFallback) -> Result<(), String>>,
    replacement: Option<ReplacementCallback<'a>>,
    lifecycle: bool,
    selection_observed: bool,
    pending_replacement: Option<Vec<PathComponent>>,
}

impl<'a, F> Projector<'a, F>
where
    F: FnMut(StreamRecord) -> Result<(), String>,
{
    fn new(maximum_depth: usize, maximum_token_bytes: usize, emit: &'a mut F) -> Self {
        Self {
            frames: Vec::new(),
            last_path: Vec::new(),
            deferred_error_record: false,
            pending_record: None,
            maximum_depth,
            maximum_token_bytes,
            selection: None,
            depth_high_water: Cell::new(0),
            cancellation: None,
            values_until_cancellation_check: Cell::new(0),
            emit,
            fallback: None,
            replacement: None,
            lifecycle: false,
            selection_observed: false,
            pending_replacement: None,
        }
    }

    fn selected(
        maximum_depth: usize,
        maximum_token_bytes: usize,
        selection: StreamSelection,
        emit: &'a mut F,
    ) -> Self {
        Self {
            frames: Vec::new(),
            last_path: Vec::new(),
            deferred_error_record: false,
            pending_record: None,
            maximum_depth,
            maximum_token_bytes,
            selection: Some(selection),
            depth_high_water: Cell::new(0),
            cancellation: None,
            values_until_cancellation_check: Cell::new(0),
            emit,
            fallback: None,
            replacement: None,
            lifecycle: false,
            selection_observed: false,
            pending_replacement: None,
        }
    }

    fn selected_with_lifecycle(
        maximum_depth: usize,
        maximum_token_bytes: usize,
        selection: StreamSelection,
        emit: &'a mut F,
        fallback: &'a mut dyn FnMut(SelectionFallback) -> Result<(), String>,
        replacement: ReplacementCallback<'a>,
    ) -> Self {
        Self {
            frames: Vec::new(),
            last_path: Vec::new(),
            deferred_error_record: false,
            pending_record: None,
            maximum_depth,
            maximum_token_bytes,
            selection: Some(selection),
            depth_high_water: Cell::new(0),
            cancellation: None,
            values_until_cancellation_check: Cell::new(0),
            emit,
            fallback: Some(fallback),
            replacement: Some(replacement),
            lifecycle: true,
            selection_observed: false,
            pending_replacement: None,
        }
    }

    fn reset_root(&mut self) {
        debug_assert!(self.lifecycle);
        self.frames.clear();
        self.last_path.clear();
        self.deferred_error_record = false;
        self.pending_record = None;
        self.selection_observed = false;
        self.pending_replacement = None;
    }

    fn defer_error_record(&mut self) {
        self.deferred_error_record = true;
    }

    fn emit_record(&mut self, record: StreamRecord) -> Result<(), String> {
        if !self.deferred_error_record {
            return (self.emit)(record);
        }
        if let Some(previous) = self.pending_record.replace(record) {
            (self.emit)(previous)?;
        }
        Ok(())
    }

    fn flush_deferred(&mut self) -> Result<(), String> {
        if let Some(record) = self.pending_record.take() {
            (self.emit)(record)?;
        }
        Ok(())
    }

    fn begin(&mut self, kind: ContainerKind) -> Result<(), String> {
        if self.frames.len() >= self.maximum_depth {
            return Err("stream depth limit exceeded".to_owned());
        }
        let path = self.take_value_path()?;
        let replacement_path = self.pending_replacement.take();
        if let Some(path) = path.as_ref() {
            let replacement = match kind {
                ContainerKind::Array => SelectionReplacement::ArrayStart,
                ContainerKind::Object => SelectionReplacement::ObjectStart,
            };
            let selected_ancestor = self.lifecycle
                && self
                    .selection
                    .as_ref()
                    .is_some_and(|selection| selection.prefix.starts_with(path));
            if replacement_path.as_deref() == Some(path) || selected_ancestor {
                self.notify_replacement(path, replacement)?;
            }
            if self.lifecycle
                && self
                    .selection
                    .as_ref()
                    .is_some_and(|selection| path.as_slice() == selection.prefix.as_slice())
            {
                self.selection_observed = true;
            }
        }
        self.frames.push(Frame {
            kind,
            path,
            children: 0,
            retained_child: false,
            pending_key: None,
        });
        self.depth_high_water
            .set(self.depth_high_water.get().max(self.frames.len()));
        Ok(())
    }

    fn check_cancellation(&self) -> Result<(), String> {
        let remaining = self.values_until_cancellation_check.get();
        if remaining != 0 {
            self.values_until_cancellation_check
                .set(remaining.saturating_sub(1));
            return Ok(());
        }
        if self
            .cancellation
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
        {
            return Err("selected decoding interrupted".to_owned());
        }
        self.values_until_cancellation_check.set(4096);
        Ok(())
    }

    fn key(&mut self, key: String) -> Result<(), String> {
        self.check_token(key.len())?;
        let key: Arc<str> = key.into();
        let parent = {
            let frame = self
                .frames
                .last_mut()
                .ok_or_else(|| "object key outside a container".to_owned())?;
            if frame.kind != ContainerKind::Object || frame.pending_key.is_some() {
                return Err("object key arrived in an invalid stream state".to_owned());
            }
            frame.pending_key = Some(Arc::clone(&key));
            frame.path.clone()
        };
        if self.lifecycle {
            let mut path = parent.unwrap_or_default();
            path.push(PathComponent::Key(key));
            if self
                .selection
                .as_ref()
                .is_some_and(|selection| selection.tracks(&path))
            {
                self.pending_replacement = Some(path);
            }
        }
        Ok(())
    }

    fn structural_scalar(&mut self, value: Scalar) -> Result<(), String> {
        let Some(path) = self.take_value_path()? else {
            self.pending_replacement = None;
            return Ok(());
        };
        let value = match value {
            Scalar::Null => Value::Null,
            Scalar::Bool(value) => Value::Bool(value),
            Scalar::Number(value) => Value::Number(value),
            Scalar::String(value) => Value::string(value),
        };
        let replacement_path = self.pending_replacement.take();
        let selected_ancestor = self.lifecycle
            && self
                .selection
                .as_ref()
                .is_some_and(|selection| selection.prefix.starts_with(&path));
        if replacement_path.as_deref() == Some(path.as_slice()) || selected_ancestor {
            self.notify_replacement(&path, SelectionReplacement::Scalar(value.clone()))?;
        }
        self.emit_pair(path, value)
    }

    fn check_token(&self, bytes: usize) -> Result<(), String> {
        if bytes > self.maximum_token_bytes {
            return Err("input resource limit exceeded: token-bytes".to_owned());
        }
        Ok(())
    }

    fn end(&mut self, kind: ContainerKind) -> Result<(), String> {
        let frame = self
            .frames
            .pop()
            .ok_or_else(|| "container end without a matching start".to_owned())?;
        if frame.kind != kind || frame.pending_key.is_some() {
            return Err("mismatched container event".to_owned());
        }
        let Some(path) = frame.path else {
            return Ok(());
        };
        if frame.children == 0 {
            let empty = match kind {
                ContainerKind::Array => Value::array(Vec::new()),
                ContainerKind::Object => Value::object(Object::new()),
            };
            self.emit_pair(path, empty)
        } else if let Some(selection) = &self.selection {
            if selection.keeps(&path) {
                if frame.retained_child || !selection.capture_shell_required(&path) {
                    self.emit_selected_record(StreamRecord::path(path.clone(), None))?;
                } else {
                    let empty = match kind {
                        ContainerKind::Array => Value::array(Vec::new()),
                        ContainerKind::Object => Value::object(Object::new()),
                    };
                    self.emit_selected_record(StreamRecord::path(path.clone(), Some(empty)))?;
                }
            }
            self.last_path = path;
            Ok(())
        } else {
            let end_path = self.last_path.clone();
            self.emit_record(StreamRecord::path(end_path, None))?;
            self.last_path = path;
            Ok(())
        }
    }

    fn error_value(
        &mut self,
        message: String,
        discard_partial_value: bool,
        null_path: bool,
        prefer_last_value_path: bool,
    ) -> Result<(), String> {
        let mut path = if prefer_last_value_path && !self.last_path.is_empty() {
            self.last_path.clone()
        } else {
            self.expected_path()
        };
        if self.deferred_error_record
            && let Some(pending) = self.pending_record.take()
        {
            if discard_partial_value {
                path.clone_from(&pending.path);
            } else {
                (self.emit)(pending)?;
            }
        }
        let error_path = if null_path {
            // jq uses a null path component for syntax errors whose location
            // cannot be associated with a completed object/array member.
            // Retain the containing path before appending that marker.
            self.null_slot_path()
        } else {
            path_value(&path)
        };
        let value = Value::array(vec![Value::string(message), error_path]);
        (self.emit)(StreamRecord::raw(value))
    }

    fn null_slot_path(&self) -> Value {
        let path = self
            .frames
            .last()
            .and_then(|frame| frame.path.clone())
            .unwrap_or_default();
        let mut values = match path_value(&path) {
            Value::Array(values) => values.to_vec(),
            _ => Vec::new(),
        };
        values.push(Value::Null);
        Value::array(values)
    }

    fn take_value_path(&mut self) -> Result<Option<Vec<PathComponent>>, String> {
        let Some(frame) = self.frames.last_mut() else {
            return Ok(Some(Vec::new()));
        };
        let component = match frame.kind {
            ContainerKind::Array => PathComponent::Index(frame.children),
            ContainerKind::Object => PathComponent::Key(
                frame
                    .pending_key
                    .take()
                    .ok_or_else(|| "object value has no key".to_owned())?,
            ),
        };
        frame.children = frame.children.saturating_add(1);
        let Some(parent) = frame.path.as_ref() else {
            return Ok(None);
        };
        let mut path = parent.clone();
        path.push(component);
        let selected_ancestor = self.lifecycle
            && self
                .selection
                .as_ref()
                .is_some_and(|selection| selection.prefix.starts_with(&path));
        if self.lifecycle
            && self
                .selection
                .as_ref()
                .is_some_and(|selection| !selection.tracks(&path))
            && !selected_ancestor
        {
            return Ok(None);
        }
        Ok(Some(path))
    }

    fn expected_path(&self) -> Vec<PathComponent> {
        let Some(frame) = self.frames.last() else {
            return Vec::new();
        };
        let Some(mut path) = frame.path.clone() else {
            return Vec::new();
        };
        match frame.kind {
            ContainerKind::Array => path.push(PathComponent::Index(frame.children)),
            ContainerKind::Object => {
                if let Some(key) = &frame.pending_key {
                    path.push(PathComponent::Key(Arc::clone(key)));
                }
            }
        }
        path
    }

    fn emit_pair(&mut self, path: Vec<PathComponent>, value: Value) -> Result<(), String> {
        if self.lifecycle
            && self
                .selection
                .as_ref()
                .is_some_and(|selection| path == selection.prefix)
        {
            self.selection_observed = true;
            let fallback = match value {
                Value::Array(values) if values.is_empty() => {
                    SelectionFallback::PresentEmptyArray { path }
                }
                Value::Object(values) if values.is_empty() => {
                    SelectionFallback::PresentEmptyObject { path }
                }
                value => SelectionFallback::PresentScalar { path, value },
            };
            if let Some(fallback_callback) = self.fallback.as_mut() {
                fallback_callback(fallback)?;
            }
            return Ok(());
        }
        if self
            .selection
            .as_ref()
            .is_none_or(|selection| selection.keeps(&path))
        {
            if self.lifecycle {
                self.selection_observed = true;
            }
            self.emit_selected_record(StreamRecord::path(path.clone(), Some(value)))?;
        }
        self.last_path = path;
        Ok(())
    }

    fn emit_selected_record(&mut self, record: StreamRecord) -> Result<(), String> {
        if let Some(frame) = self.frames.last_mut() {
            frame.retained_child = true;
        }
        self.emit_record(record)
    }

    fn notify_replacement(
        &mut self,
        path: &[PathComponent],
        replacement: SelectionReplacement,
    ) -> Result<(), String> {
        if self.lifecycle
            && self.selection.as_ref().is_some_and(|selection| {
                path == selection.prefix || selection.prefix.starts_with(path)
            })
        {
            self.selection_observed = false;
        }
        if let Some(callback) = self.replacement.as_mut() {
            callback(path, replacement)?;
        }
        Ok(())
    }

    fn finish_selection(&mut self) -> Result<(), String> {
        if self.selection.is_some()
            && !self.selection_observed
            && let Some(fallback_callback) = self.fallback.as_mut()
        {
            fallback_callback(SelectionFallback::Missing {
                path: self
                    .selection
                    .as_ref()
                    .expect("selection checked")
                    .prefix
                    .clone(),
            })?;
            self.selection_observed = true;
        }
        Ok(())
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

fn path_value(path: &[PathComponent]) -> Value {
    Value::array(
        path.iter()
            .map(|component| match component {
                PathComponent::Key(key) => Value::string(Arc::clone(key)),
                PathComponent::Index(index) => Value::Number(
                    Number::parse(&index.to_string()).expect("usize is a valid bounded number"),
                ),
            })
            .collect::<Vec<_>>(),
    )
}

/// Projects native structural events into jq path/value records.
///
/// Source decoding and the caller's warning/error policy remain separate.
pub struct EventProjector<'a, F> {
    projector: Projector<'a, F>,
}

impl<'a, F> EventProjector<'a, F>
where
    F: FnMut(StreamRecord) -> Result<(), String>,
{
    /// Starts a projection without consuming input or emitting a record.
    pub fn new(options: StreamOptions, emit: &'a mut F) -> Self {
        Self {
            projector: Projector::new(options.maximum_depth, options.maximum_token_bytes, emit),
        }
    }

    /// Emits a jq error value at the next expected value path.
    ///
    /// # Errors
    /// Returns the output consumer's failure. Recovery clears the failed path.
    pub fn error_value(&mut self, message: String) -> Result<(), String> {
        let result =
            self.projector
                .error_value(jq_stream_error_message(message), false, false, false);
        self.reset();
        result
    }

    /// Clears a failed Document's path without publishing successful completion.
    pub fn reset(&mut self) {
        self.projector.frames.clear();
        self.projector.last_path.clear();
    }
}

impl<F> EventConsumer for EventProjector<'_, F>
where
    F: FnMut(StreamRecord) -> Result<(), String>,
{
    type Error = String;

    fn consume(&mut self, event: Event) -> Result<(), Self::Error> {
        self.projector.check_cancellation()?;
        match event {
            Event::DocumentStart { .. } => {
                if self.projector.lifecycle {
                    self.projector.reset_root();
                } else {
                    self.reset();
                }
                Ok(())
            }
            Event::DocumentEnd { .. } => {
                if self.projector.lifecycle {
                    self.projector.finish_selection()
                } else {
                    Ok(())
                }
            }
            Event::ObjectStart { .. } => self.projector.begin(ContainerKind::Object),
            Event::ObjectEnd { .. } => self.projector.end(ContainerKind::Object),
            Event::ArrayStart { .. } => self.projector.begin(ContainerKind::Array),
            Event::ArrayEnd { .. } => self.projector.end(ContainerKind::Array),
            Event::Key { value, .. } => self.projector.key(value.to_string()),
            Event::Scalar { value, .. } => self.projector.structural_scalar(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{BufReader, Cursor, Read},
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };

    use tq_core::{Number, Object, PathComponent, SourceId, Value};

    use crate::{JsonEventOptions, decode_json_events_with_options};

    use super::{
        EventProjector, Projector, SelectedRootSink, SelectedStreamObservations, SelectionFallback,
        SelectionReplacement, StreamOptions, StreamRecord, StreamSelection, stream_json,
        stream_json_records, stream_json_selected_records,
        stream_json_selected_records_with_control, stream_json_selected_roots_with_control,
        stream_toon,
    };

    fn json_lines(values: &[tq_core::Value]) -> Vec<String> {
        values.iter().map(ToString::to_string).collect()
    }

    #[derive(Default)]
    struct RootSink {
        begun: Vec<u64>,
        current: Vec<StreamRecord>,
        committed: Vec<Vec<StreamRecord>>,
        observed_records: Vec<StreamRecord>,
        fallbacks: Vec<SelectionFallback>,
        replacements: Vec<(Vec<PathComponent>, SelectionReplacement)>,
        aborted: usize,
        cancel_on_first_begin: Option<Arc<AtomicBool>>,
    }

    impl SelectedRootSink for RootSink {
        type Error = String;

        fn begin_root(&mut self, index: u64) -> Result<(), Self::Error> {
            self.begun.push(index);
            self.current.clear();
            if index == 0
                && let Some(cancelled) = &self.cancel_on_first_begin
            {
                cancelled.store(true, Ordering::Relaxed);
            }
            Ok(())
        }

        fn fallback(&mut self, fallback: SelectionFallback) -> Result<(), Self::Error> {
            self.fallbacks.push(fallback);
            Ok(())
        }

        fn replace_selected_prefix(
            &mut self,
            path: &[PathComponent],
            replacement: SelectionReplacement,
        ) -> Result<(), Self::Error> {
            self.current.clear();
            self.replacements.push((path.to_vec(), replacement));
            Ok(())
        }

        fn replace_item_path(
            &mut self,
            path: &[PathComponent],
            replacement: SelectionReplacement,
        ) -> Result<(), Self::Error> {
            self.current.clear();
            self.replacements.push((path.to_vec(), replacement));
            Ok(())
        }

        fn record(&mut self, record: StreamRecord) -> Result<(), Self::Error> {
            self.observed_records.push(record.clone());
            self.current.push(record);
            Ok(())
        }

        fn finish_root(&mut self) -> Result<(), Self::Error> {
            self.committed.push(std::mem::take(&mut self.current));
            Ok(())
        }

        fn abort_root(&mut self) {
            self.current.clear();
            self.aborted = self.aborted.saturating_add(1);
        }
    }

    fn feature_selection() -> StreamSelection {
        StreamSelection::new(
            vec![PathComponent::Key(Arc::from("features"))],
            Some(vec![PathComponent::Key(Arc::from("id"))]),
        )
    }

    #[test]
    fn capture_selection_skips_unneeded_fields_and_keeps_empty_ancestors() {
        let selection = StreamSelection::with_item_capture_paths(
            vec![PathComponent::Key(Arc::from("features"))],
            vec![
                vec![PathComponent::Key(Arc::from("id"))],
                vec![
                    PathComponent::Key(Arc::from("properties")),
                    PathComponent::Key(Arc::from("mag")),
                ],
            ],
        );
        let (records, result) = selected_outcome(
            br#"{"features":[{"id":1,"properties":{"mag":2,"label":"discard","unused":{"x":1}}},{"id":2,"properties":{"label":"discard"}}]}"#,
            root_options(),
            selection,
        );
        assert_eq!(result, Ok(()));

        let records = records
            .into_iter()
            .map(StreamRecord::into_parts)
            .collect::<Vec<_>>();
        assert_eq!(
            records,
            vec![
                (
                    vec![
                        PathComponent::Key(Arc::from("features")),
                        PathComponent::Index(0),
                        PathComponent::Key(Arc::from("id")),
                    ],
                    Some(Value::Number(Number::parse("1").expect("one"))),
                ),
                (
                    vec![
                        PathComponent::Key(Arc::from("features")),
                        PathComponent::Index(0),
                        PathComponent::Key(Arc::from("properties")),
                        PathComponent::Key(Arc::from("mag")),
                    ],
                    Some(Value::Number(Number::parse("2").expect("two"))),
                ),
                (
                    vec![
                        PathComponent::Key(Arc::from("features")),
                        PathComponent::Index(0),
                        PathComponent::Key(Arc::from("properties")),
                    ],
                    None,
                ),
                (
                    vec![
                        PathComponent::Key(Arc::from("features")),
                        PathComponent::Index(0),
                    ],
                    None,
                ),
                (
                    vec![
                        PathComponent::Key(Arc::from("features")),
                        PathComponent::Index(1),
                        PathComponent::Key(Arc::from("id")),
                    ],
                    Some(Value::Number(Number::parse("2").expect("two"))),
                ),
                (
                    vec![
                        PathComponent::Key(Arc::from("features")),
                        PathComponent::Index(1),
                        PathComponent::Key(Arc::from("properties")),
                    ],
                    Some(Value::object(Object::new())),
                ),
                (
                    vec![
                        PathComponent::Key(Arc::from("features")),
                        PathComponent::Index(1),
                    ],
                    None,
                ),
            ]
        );
    }

    #[test]
    fn empty_capture_paths_fall_back_to_complete_item_retention() {
        let selection = StreamSelection::with_item_capture_paths(
            vec![PathComponent::Key(Arc::from("features"))],
            Vec::new(),
        );
        let (records, result) = selected_outcome(
            br#"{"features":[{"id":1,"unused":{"value":2}}]}"#,
            root_options(),
            selection,
        );
        assert_eq!(result, Ok(()));
        assert!(records.iter().any(|record| {
            record
                .path
                .ends_with(&[PathComponent::Key(Arc::from("unused"))])
        }));
    }

    #[test]
    fn capture_selection_keeps_terminal_subtrees_and_wrong_kind_values() {
        let terminal = StreamSelection::with_item_capture_paths(
            vec![PathComponent::Key(Arc::from("features"))],
            vec![vec![PathComponent::Key(Arc::from("meta"))]],
        );
        let (records, result) = selected_outcome(
            br#"{"features":[{"meta":{"x":1,"nested":{"y":2}},"discard":true}]}"#,
            root_options(),
            terminal,
        );
        assert_eq!(result, Ok(()));
        let terminal_paths = records
            .iter()
            .map(|record| record.path.clone())
            .collect::<Vec<_>>();
        assert!(terminal_paths.iter().any(|path| {
            path == &[
                PathComponent::Key(Arc::from("features")),
                PathComponent::Index(0),
                PathComponent::Key(Arc::from("meta")),
                PathComponent::Key(Arc::from("nested")),
                PathComponent::Key(Arc::from("y")),
            ]
        }));
        assert!(
            !terminal_paths
                .iter()
                .any(|path| { path.ends_with(&[PathComponent::Key(Arc::from("discard"))]) })
        );

        let wrong_kind = StreamSelection::with_item_capture_paths(
            vec![PathComponent::Key(Arc::from("features"))],
            vec![vec![
                PathComponent::Key(Arc::from("values")),
                PathComponent::Index(0),
            ]],
        );
        let (records, result) = selected_outcome(
            br#"{"features":[{"values":{"x":1,"y":2}}]}"#,
            root_options(),
            wrong_kind,
        );
        assert_eq!(result, Ok(()));
        let wrong_kind_paths = records
            .iter()
            .map(|record| record.path.clone())
            .collect::<Vec<_>>();
        assert!(
            wrong_kind_paths
                .iter()
                .any(|path| { path.ends_with(&[PathComponent::Key(Arc::from("x"))]) })
        );
        assert!(
            wrong_kind_paths
                .iter()
                .any(|path| { path.ends_with(&[PathComponent::Key(Arc::from("y"))]) })
        );

        let wrong_kind_array = StreamSelection::with_item_capture_paths(
            vec![PathComponent::Key(Arc::from("features"))],
            vec![vec![
                PathComponent::Key(Arc::from("values")),
                PathComponent::Key(Arc::from("first")),
            ]],
        );
        let (records, result) = selected_outcome(
            br#"{"features":[{"values":[1,2]}]}"#,
            root_options(),
            wrong_kind_array,
        );
        assert_eq!(result, Ok(()));
        assert!(
            records
                .iter()
                .any(|record| record.path.ends_with(&[PathComponent::Index(1)]))
        );
    }

    #[test]
    fn capture_selection_duplicate_ancestor_emits_replacement_and_empty_shell() {
        let selection = StreamSelection::with_item_capture_paths(
            vec![PathComponent::Key(Arc::from("features"))],
            vec![vec![
                PathComponent::Key(Arc::from("outer")),
                PathComponent::Key(Arc::from("needed")),
            ]],
        );
        let mut sink = RootSink::default();
        stream_json_selected_roots_with_control(
            br#"{"features":[{"outer":{"needed":1}},{"outer":{"needed":4},"outer":{"discard":2},"other":3}]}"#
                .as_slice(),
            root_options(),
            selection,
            None,
            &mut SelectedStreamObservations::default(),
            &mut sink,
        )
        .unwrap();

        let outer_path = [
            PathComponent::Key(Arc::from("features")),
            PathComponent::Index(1),
            PathComponent::Key(Arc::from("outer")),
        ];
        assert!(sink.replacements.iter().any(|(path, replacement)| {
            path == &outer_path && *replacement == SelectionReplacement::ObjectStart
        }));
        assert!(sink.observed_records.iter().any(|record| {
            record
                .path
                .ends_with(&[PathComponent::Key(Arc::from("needed"))])
                && record.value == Some(Value::Number(Number::parse("1").expect("one")))
        }));
        let committed = sink
            .committed
            .into_iter()
            .flatten()
            .map(StreamRecord::into_parts)
            .collect::<Vec<_>>();
        assert!(committed.iter().any(|(path, value)| {
            path.ends_with(&[PathComponent::Key(Arc::from("outer"))])
                && matches!(value, Some(Value::Object(object)) if object.is_empty())
        }));
    }

    fn root_options() -> StreamOptions {
        StreamOptions {
            maximum_depth: 16,
            maximum_token_bytes: 1024,
            errors_as_values: false,
        }
    }

    #[test]
    fn selected_roots_commit_only_after_each_root_finishes() {
        let mut sink = RootSink::default();
        let roots = stream_json_selected_roots_with_control(
            br#"{"features":[{"id":1}]} {"features":[{"id":2}]}"#.as_slice(),
            root_options(),
            feature_selection(),
            None,
            &mut SelectedStreamObservations::default(),
            &mut sink,
        )
        .unwrap();

        assert_eq!(roots, 2);
        assert_eq!(sink.begun, [0, 1]);
        assert_eq!(sink.committed.len(), 2);
        assert!(sink.committed.iter().all(|records| !records.is_empty()));
        assert_eq!(sink.aborted, 0);
    }

    #[test]
    fn malformed_root_aborts_without_committing_partial_records() {
        let mut sink = RootSink::default();
        let result = stream_json_selected_roots_with_control(
            br#"{"features":[{"id":1}]} {"features":[{"id":2}],"discarded":[1,]}"#.as_slice(),
            root_options(),
            feature_selection(),
            None,
            &mut SelectedStreamObservations::default(),
            &mut sink,
        );

        assert!(result.is_err());
        assert_eq!(sink.committed.len(), 1);
        assert_eq!(sink.committed[0].len(), 2);
        assert_eq!(sink.aborted, 1);
    }

    #[test]
    fn duplicate_selected_prefix_replaces_prior_records_before_commit() {
        let mut sink = RootSink::default();
        let roots = stream_json_selected_roots_with_control(
            br#"{"features":[{"id":1}],"features":[]}"#.as_slice(),
            root_options(),
            feature_selection(),
            None,
            &mut SelectedStreamObservations::default(),
            &mut sink,
        )
        .unwrap();

        assert_eq!(roots, 1);
        assert_eq!(sink.committed, [Vec::new()]);
        assert_eq!(
            sink.replacements
                .iter()
                .filter(|(path, _)| path == &[PathComponent::Key(Arc::from("features"))])
                .count(),
            2
        );
        assert_eq!(
            sink.replacements.last().expect("replacement").0,
            [PathComponent::Key(Arc::from("features"))]
        );
        assert_eq!(
            sink.replacements.last().expect("replacement").1,
            SelectionReplacement::ArrayStart
        );
        assert_eq!(
            sink.fallbacks,
            [SelectionFallback::PresentEmptyArray {
                path: vec![PathComponent::Key(Arc::from("features"))]
            }]
        );
    }

    #[test]
    fn duplicate_ancestor_reports_missing_selected_descendant() {
        let mut sink = RootSink::default();
        let selection = StreamSelection::new(
            vec![
                PathComponent::Key(Arc::from("outer")),
                PathComponent::Key(Arc::from("features")),
            ],
            Some(vec![PathComponent::Key(Arc::from("id"))]),
        );
        let roots = stream_json_selected_roots_with_control(
            br#"{"outer":{"features":[{"id":1}]},"outer":{}}"#.as_slice(),
            root_options(),
            selection,
            None,
            &mut SelectedStreamObservations::default(),
            &mut sink,
        )
        .unwrap();

        assert_eq!(roots, 1);
        assert_eq!(sink.committed, [Vec::new()]);
        assert_eq!(
            sink.fallbacks,
            [SelectionFallback::Missing {
                path: vec![
                    PathComponent::Key(Arc::from("outer")),
                    PathComponent::Key(Arc::from("features")),
                ]
            }]
        );
        assert!(sink.replacements.iter().any(|(path, replacement)| {
            path == &[PathComponent::Key(Arc::from("outer"))]
                && *replacement == SelectionReplacement::ObjectStart
        }));
    }

    #[test]
    fn duplicate_nested_selected_item_reports_container_replacement() {
        let mut sink = RootSink::default();
        let selection = StreamSelection::new(
            vec![PathComponent::Key(Arc::from("features"))],
            Some(vec![
                PathComponent::Key(Arc::from("item")),
                PathComponent::Key(Arc::from("id")),
            ]),
        );
        stream_json_selected_roots_with_control(
            br#"{"features":[{"item":{"id":1},"item":{}}]}"#.as_slice(),
            root_options(),
            selection,
            None,
            &mut SelectedStreamObservations::default(),
            &mut sink,
        )
        .unwrap();

        let item_path = [
            PathComponent::Key(Arc::from("features")),
            PathComponent::Index(0),
            PathComponent::Key(Arc::from("item")),
        ];
        assert_eq!(
            sink.replacements
                .iter()
                .filter(|(path, replacement)| {
                    path == &item_path && *replacement == SelectionReplacement::ObjectStart
                })
                .count(),
            2
        );
        assert!(sink.committed.iter().flatten().any(|record| {
            let (path, value) = record.clone().into_parts();
            path == item_path
                && matches!(value, Some(Value::Object(ref object)) if object.is_empty())
        }));
    }

    #[test]
    fn duplicate_ancestor_scalar_reports_wrong_type_without_dom_fallback() {
        let mut sink = RootSink::default();
        let selection = StreamSelection::new(
            vec![
                PathComponent::Key(Arc::from("outer")),
                PathComponent::Key(Arc::from("features")),
            ],
            None,
        );
        stream_json_selected_roots_with_control(
            br#"{"outer":{"features":[1]},"outer":0}"#.as_slice(),
            root_options(),
            selection,
            None,
            &mut SelectedStreamObservations::default(),
            &mut sink,
        )
        .unwrap();

        let expected_replacement = (
            vec![PathComponent::Key(Arc::from("outer"))],
            SelectionReplacement::Scalar(Value::Number(Number::parse("0").expect("literal zero"))),
        );
        assert_eq!(sink.replacements.last(), Some(&expected_replacement));
        assert_eq!(
            sink.fallbacks.last(),
            Some(&SelectionFallback::Missing {
                path: vec![
                    PathComponent::Key(Arc::from("outer")),
                    PathComponent::Key(Arc::from("features")),
                ]
            })
        );
    }

    #[test]
    fn lifecycle_preserves_root_and_array_ancestor_shapes() {
        let root_cases = [
            (
                b"0".as_slice(),
                SelectionReplacement::Scalar(Value::Number(
                    Number::parse("0").expect("literal zero"),
                )),
            ),
            (b"[]".as_slice(), SelectionReplacement::ArrayStart),
            (b"[1]".as_slice(), SelectionReplacement::ArrayStart),
        ];
        for (input, expected_replacement) in root_cases {
            let mut sink = RootSink::default();
            stream_json_selected_roots_with_control(
                input,
                root_options(),
                feature_selection(),
                None,
                &mut SelectedStreamObservations::default(),
                &mut sink,
            )
            .unwrap();

            assert_eq!(sink.replacements, [(Vec::new(), expected_replacement)]);
            assert_eq!(
                sink.fallbacks,
                [SelectionFallback::Missing {
                    path: vec![PathComponent::Key(Arc::from("features"))]
                }]
            );
        }

        let mut sink = RootSink::default();
        let selection = StreamSelection::new(
            vec![
                PathComponent::Key(Arc::from("items")),
                PathComponent::Index(1),
                PathComponent::Key(Arc::from("features")),
            ],
            None,
        );
        stream_json_selected_roots_with_control(
            br#"{"items":[0,7]}"#.as_slice(),
            root_options(),
            selection,
            None,
            &mut SelectedStreamObservations::default(),
            &mut sink,
        )
        .unwrap();

        assert_eq!(
            sink.replacements,
            [
                (Vec::new(), SelectionReplacement::ObjectStart),
                (
                    vec![PathComponent::Key(Arc::from("items"))],
                    SelectionReplacement::ArrayStart,
                ),
                (
                    vec![
                        PathComponent::Key(Arc::from("items")),
                        PathComponent::Index(1),
                    ],
                    SelectionReplacement::Scalar(Value::Number(
                        Number::parse("7").expect("literal seven"),
                    )),
                ),
            ]
        );
        assert_eq!(
            sink.fallbacks,
            [SelectionFallback::Missing {
                path: vec![
                    PathComponent::Key(Arc::from("items")),
                    PathComponent::Index(1),
                    PathComponent::Key(Arc::from("features")),
                ]
            }]
        );
    }

    #[test]
    fn reports_object_item_replacements_in_source_order() {
        let mut sink = RootSink::default();
        let selection = StreamSelection::new(
            vec![PathComponent::Key(Arc::from("features"))],
            Some(vec![PathComponent::Key(Arc::from("id"))]),
        );
        stream_json_selected_roots_with_control(
            br#"{"features":{"a":{"id":1},"b":{"id":2},"a":{"id":3}}}"#.as_slice(),
            root_options(),
            selection,
            None,
            &mut SelectedStreamObservations::default(),
            &mut sink,
        )
        .unwrap();

        let expected = ["a", "b", "a"];
        let paths = sink
            .replacements
            .iter()
            .filter_map(|(path, replacement)| {
                if *replacement != SelectionReplacement::ObjectStart {
                    return None;
                }
                match path.last() {
                    Some(PathComponent::Key(key)) if path.len() == 2 => Some(key.as_ref()),
                    _ => None,
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(paths, expected);
        let records = sink
            .observed_records
            .into_iter()
            .map(StreamRecord::into_parts)
            .collect::<Vec<_>>();
        assert_eq!(
            records,
            vec![
                (
                    vec![
                        PathComponent::Key(Arc::from("features")),
                        PathComponent::Key(Arc::from("a")),
                        PathComponent::Key(Arc::from("id")),
                    ],
                    Some(Value::Number(Number::parse("1").expect("one"))),
                ),
                (
                    vec![
                        PathComponent::Key(Arc::from("features")),
                        PathComponent::Key(Arc::from("a")),
                    ],
                    None,
                ),
                (
                    vec![
                        PathComponent::Key(Arc::from("features")),
                        PathComponent::Key(Arc::from("b")),
                        PathComponent::Key(Arc::from("id")),
                    ],
                    Some(Value::Number(Number::parse("2").expect("two"))),
                ),
                (
                    vec![
                        PathComponent::Key(Arc::from("features")),
                        PathComponent::Key(Arc::from("b")),
                    ],
                    None,
                ),
                (
                    vec![
                        PathComponent::Key(Arc::from("features")),
                        PathComponent::Key(Arc::from("a")),
                        PathComponent::Key(Arc::from("id")),
                    ],
                    Some(Value::Number(Number::parse("3").expect("three"))),
                ),
                (
                    vec![
                        PathComponent::Key(Arc::from("features")),
                        PathComponent::Key(Arc::from("a")),
                    ],
                    None,
                ),
            ]
        );
    }

    #[test]
    fn cancellation_after_first_root_begins_aborts_current_root() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let mut sink = RootSink {
            cancel_on_first_begin: Some(Arc::clone(&cancelled)),
            ..RootSink::default()
        };
        let result = stream_json_selected_roots_with_control(
            br#"{"features":[{"id":1}]} {"features":[{"id":2}]}"#.as_slice(),
            root_options(),
            feature_selection(),
            Some(cancelled),
            &mut SelectedStreamObservations::default(),
            &mut sink,
        );

        assert!(matches!(
            result,
            Err(crate::FormatError::Resource("interrupted"))
        ));
        assert_eq!(sink.committed.len(), 0);
        assert_eq!(sink.aborted, 1);
    }

    #[test]
    fn missing_and_scalar_selected_prefixes_are_reported_without_a_dom_fallback() {
        let cases = [
            (
                b"{}".as_slice(),
                SelectionFallback::Missing {
                    path: vec![PathComponent::Key(Arc::from("features"))],
                },
            ),
            (
                br#"{"features":null}"#.as_slice(),
                SelectionFallback::PresentScalar {
                    path: vec![PathComponent::Key(Arc::from("features"))],
                    value: tq_core::Value::Null,
                },
            ),
        ];
        for (input, expected) in cases {
            let mut sink = RootSink::default();
            let roots = stream_json_selected_roots_with_control(
                input,
                root_options(),
                feature_selection(),
                None,
                &mut SelectedStreamObservations::default(),
                &mut sink,
            )
            .unwrap();
            assert_eq!(roots, 1);
            assert_eq!(sink.fallbacks, [expected]);
            assert_eq!(sink.committed, [Vec::new()]);
        }
    }

    #[test]
    fn cancellation_aborts_before_opening_a_root() {
        let cancelled = Arc::new(AtomicBool::new(true));
        let mut sink = RootSink::default();
        let result = stream_json_selected_roots_with_control(
            br#"{"features":[1]}"#.as_slice(),
            root_options(),
            feature_selection(),
            Some(cancelled),
            &mut SelectedStreamObservations::default(),
            &mut sink,
        );

        assert!(matches!(
            result,
            Err(crate::FormatError::Resource("interrupted"))
        ));
        assert!(sink.begun.is_empty());
        assert_eq!(sink.aborted, 0);
    }

    fn selected_outcome(
        input: &[u8],
        options: StreamOptions,
        selection: StreamSelection,
    ) -> (Vec<StreamRecord>, Result<(), String>) {
        let mut records = Vec::new();
        let result = stream_json_selected_records(input, options, selection, |record| {
            records.push(record);
            Ok(())
        })
        .map_err(|error| error.to_string());
        (records, result)
    }

    fn structural_selected_outcome(
        input: &[u8],
        options: StreamOptions,
        selection: StreamSelection,
    ) -> (Vec<StreamRecord>, Result<(), String>) {
        let mut records = Vec::new();
        let result = {
            let mut emit = |record| {
                records.push(record);
                Ok(())
            };
            let mut consumer = EventProjector {
                projector: Projector::selected(
                    options.maximum_depth,
                    options.maximum_token_bytes,
                    selection,
                    &mut emit,
                ),
            };
            decode_json_events_with_options(
                input,
                SourceId::new(0),
                &mut consumer,
                JsonEventOptions {
                    maximum_depth: options.maximum_depth,
                    maximum_token_bytes: options.maximum_token_bytes,
                },
            )
        };
        (records, result)
    }

    fn release_selection() -> StreamSelection {
        StreamSelection::new(
            vec![PathComponent::Key(Arc::from("features"))],
            Some(vec![
                PathComponent::Key(Arc::from("properties")),
                PathComponent::Key(Arc::from("release")),
            ]),
        )
    }

    fn capture_release_selection() -> StreamSelection {
        StreamSelection::with_item_capture_paths(
            vec![PathComponent::Key(Arc::from("features"))],
            vec![vec![
                PathComponent::Key(Arc::from("properties")),
                PathComponent::Key(Arc::from("release")),
            ]],
        )
    }

    #[test]
    fn json_and_toon_form_identical_jq_stream_records() {
        let expected = [
            r#"[["a",0],1]"#,
            r#"[["a",1],{}]"#,
            r#"[["a",1]]"#,
            r#"[["b"],[]]"#,
            r#"[["b"]]"#,
        ];
        let mut json = Vec::new();
        stream_json(
            br#"{"a":[1,{}],"b":[]}"#.as_slice(),
            StreamOptions::default(),
            |value| {
                json.push(value);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(json_lines(&json), expected);

        let mut toon = Vec::new();
        stream_toon(
            BufReader::new(Cursor::new(b"a[2]:\n  - 1\n  -\nb[0]:")),
            tq_toon::DecoderConfig::default(),
            StreamOptions::default(),
            |value| {
                toon.push(value);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(json_lines(&toon), expected);
    }

    #[test]
    fn stream_errors_include_the_next_value_path() {
        let mut values = Vec::new();
        stream_json(
            b"[1, bad, 2]".as_slice(),
            StreamOptions {
                errors_as_values: true,
                ..StreamOptions::default()
            },
            |value| {
                values.push(value);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(values[0].to_string(), "[[0],1]");
        assert!(values[1].to_string().ends_with(",[1]]"));
    }

    #[test]
    fn stream_errors_normalize_bare_identifier_as_jq_numeric_literal() {
        let mut values = Vec::new();
        stream_json(
            br#"["a",n]"#.as_slice(),
            StreamOptions {
                errors_as_values: true,
                ..StreamOptions::default()
            },
            |value| {
                values.push(value);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(
            values[1].to_string(),
            r#"["Invalid numeric literal at line 1, column 7",[1]]"#
        );
    }

    #[test]
    fn stream_errors_discard_partial_tokens_and_match_jq_boundaries() {
        let cases = [
            (
                br"[foo]".as_slice(),
                r#"["Invalid literal at line 1, column 5",[0]]"#,
            ),
            (
                br"[invalid]".as_slice(),
                r#"["Invalid numeric literal at line 1, column 9",[0]]"#,
            ),
            (
                br"[tru]".as_slice(),
                r#"["Invalid literal at line 1, column 5",[0]]"#,
            ),
            (
                br"[truee]".as_slice(),
                r#"["Invalid literal at line 1, column 7",[0]]"#,
            ),
            (
                br"[1,]".as_slice(),
                r#"["Expected another array element at line 1, column 4",[1]]"#,
            ),
            (
                br#"{"a":}"#.as_slice(),
                r#"["Missing value in key:value pair at line 1, column 6",["a"]]"#,
            ),
            (
                br#"{"a":1,}"#.as_slice(),
                r#"["Expected another key:value pair at line 1, column 8",[null]]"#,
            ),
            (
                br#"{"a" 1}"#.as_slice(),
                r#"["Expected separator between values at line 1, column 7",[null]]"#,
            ),
            (
                br"[1 2]".as_slice(),
                r#"["Expected separator between values at line 1, column 5",[0]]"#,
            ),
            (
                br"{".as_slice(),
                r#"["Unfinished JSON term at EOF at line 1, column 1",[null]]"#,
            ),
            (
                br"[".as_slice(),
                r#"["Unfinished JSON term at EOF at line 1, column 1",[0]]"#,
            ),
            (
                br#""abc"#.as_slice(),
                r#"["Unfinished string at EOF at line 1, column 4",[]]"#,
            ),
            (
                br#"{"o":{"#.as_slice(),
                r#"["Unfinished JSON term at EOF at line 1, column 6",["o",null]]"#,
            ),
            (
                br#"{"o":{"a":1,}}"#.as_slice(),
                r#"["Expected another key:value pair at line 1, column 13",["o",null]]"#,
            ),
            (
                br"[{".as_slice(),
                r#"["Unfinished JSON term at EOF at line 1, column 2",[0,null]]"#,
            ),
            (
                br#"[{"a" 1}]"#.as_slice(),
                r#"["Expected separator between values at line 1, column 8",[0,null]]"#,
            ),
        ];
        for (input, expected_error) in cases {
            let mut values = Vec::new();
            stream_json(
                input,
                StreamOptions {
                    errors_as_values: true,
                    ..StreamOptions::default()
                },
                |value| {
                    values.push(value);
                    Ok(())
                },
            )
            .unwrap();
            assert_eq!(
                values.last().expect("stream error value").to_string(),
                expected_error
            );
        }
    }

    #[test]
    fn stream_enforces_token_limits_before_emitting() {
        let mut values = Vec::new();
        let error = stream_json(
            b"12345".as_slice(),
            StreamOptions {
                maximum_token_bytes: 3,
                ..StreamOptions::default()
            },
            |value| {
                values.push(value);
                Ok(())
            },
        )
        .unwrap_err()
        .to_string();
        assert_eq!(values, [] as [tq_core::Value; 0]);
        assert!(error.contains("token-bytes"));
        assert!(error.contains("JSON input limit exceeded"));
    }

    #[test]
    fn errors_as_values_does_not_convert_callback_failures_or_call_back_again() {
        let mut calls = 0_usize;
        let error = stream_json(
            b"[1,2]".as_slice(),
            StreamOptions {
                errors_as_values: true,
                ..StreamOptions::default()
            },
            |_value| {
                calls = calls.saturating_add(1);
                Err("stop after first output".to_owned())
            },
        )
        .unwrap_err()
        .to_string();
        assert_eq!(calls, 1);
        assert!(error.contains("stop after first output"));
    }

    #[test]
    fn stream_error_mapping_keeps_line_after_a_truncated_long_prefix() {
        let input = format!("\"{}\"\n[1,bad]", "a".repeat(70_000));
        let mut values = Vec::new();
        stream_json(
            input.as_bytes(),
            StreamOptions {
                errors_as_values: true,
                ..StreamOptions::default()
            },
            |value| {
                values.push(value);
                Ok(())
            },
        )
        .unwrap();
        assert!(
            values
                .last()
                .is_some_and(|value| { value.to_string().contains("line 2") })
        );
    }

    #[test]
    fn stream_error_mapping_keeps_column_after_a_truncated_single_line() {
        let prefix = "a".repeat(70_000);
        let input = format!("[\"{prefix}\",1,]");
        let mut values = Vec::new();
        stream_json(
            input.as_bytes(),
            StreamOptions {
                errors_as_values: true,
                ..StreamOptions::default()
            },
            |value| {
                values.push(value);
                Ok(())
            },
        )
        .unwrap();
        let expected_column = prefix.len().saturating_add(7);
        assert_eq!(
            values.last().expect("stream error value").to_string(),
            format!(
                r#"["Expected another array element at line 1, column {expected_column}",[2]]"#
            )
        );
    }

    #[test]
    fn structural_json_records_avoid_jq_wrapper_values_and_keep_duplicates() {
        let mut records = Vec::new();
        stream_json_records(
            br#"{"items":[{"x":1,"x":2}]}"#.as_slice(),
            StreamOptions::default(),
            |record| {
                records.push(record.into_parts());
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(records.len(), 5);
        assert_eq!(records[0].0.len(), 3);
        assert_eq!(records[0].1.as_ref().unwrap().to_string(), "1");
        assert_eq!(records[1].1.as_ref().unwrap().to_string(), "2");
    }

    #[test]
    fn selected_records_discard_unrelated_object_members_but_keep_item_boundaries() {
        let mut records = Vec::new();
        stream_json_selected_records(
            br#"{"features":[{"geometry":{"coordinates":[1,2]},"properties":{"release":3,"other":4}},{"geometry":{"coordinates":[5,6]}}]}"#.as_slice(),
            StreamOptions::default(),
            StreamSelection::new(
                vec![PathComponent::Key(Arc::from("features"))],
                Some(vec![
                    PathComponent::Key(Arc::from("properties")),
                    PathComponent::Key(Arc::from("release")),
                ]),
            ),
            |record| {
                records.push(record.into_parts());
                Ok(())
            },
        )
        .unwrap();
        assert!(records.iter().all(|(path, _)| {
            !path
                .iter()
                .any(|component| matches!(component, PathComponent::Key(key) if &**key == "geometry" || &**key == "other"))
        }));
        assert!(records.iter().any(|(path, value)| {
            path.last().is_some_and(
                |component| matches!(component, PathComponent::Key(key) if &**key == "release"),
            ) && value.as_ref().is_some_and(|value| value.to_string() == "3")
        }));
        assert_eq!(
            records
                .iter()
                .filter(|(path, value)| path.len() == 2 && value.is_none())
                .count(),
            2
        );
    }

    #[test]
    fn selected_fast_discard_matches_structural_projection() {
        let input = br#"{"features":[{"geometry":{"coordinates":[1,-2.50e3],"label":"discard"},"properties":{"release":3,"other":4,"release":5}},{"geometry":{"coordinates":[]},"properties":{}},{"geometry":null}]}"#;
        let fast = selected_outcome(input, StreamOptions::default(), release_selection());
        let structural =
            structural_selected_outcome(input, StreamOptions::default(), release_selection());
        assert_eq!(fast, structural);
    }

    #[test]
    fn selected_fast_discard_preserves_nested_array_indexes() {
        let input = br#"{"features":[{"values":[10,20,30]}]}"#;
        let selection = StreamSelection::new(
            vec![PathComponent::Key(Arc::from("features"))],
            Some(vec![
                PathComponent::Key(Arc::from("values")),
                PathComponent::Index(1),
            ]),
        );
        let fast = selected_outcome(input, StreamOptions::default(), selection.clone());
        let structural = structural_selected_outcome(input, StreamOptions::default(), selection);
        assert_eq!(fast, structural);
        assert!(fast.0.iter().any(|record| {
            record.path
                == [
                    PathComponent::Key(Arc::from("features")),
                    PathComponent::Index(0),
                    PathComponent::Key(Arc::from("values")),
                    PathComponent::Index(1),
                ]
                && record
                    .value
                    .as_ref()
                    .is_some_and(|value| value.to_string() == "20")
        }));
    }

    #[test]
    fn selected_fast_discard_preserves_indexed_prefix_slots() {
        let input = br#"[{"features":[{"id":1}]},{"features":[{"id":2}]}]"#;
        let selection = StreamSelection::new(
            vec![
                PathComponent::Index(1),
                PathComponent::Key(Arc::from("features")),
            ],
            Some(vec![PathComponent::Key(Arc::from("id"))]),
        );
        let fast = selected_outcome(input, StreamOptions::default(), selection.clone());
        let structural = structural_selected_outcome(input, StreamOptions::default(), selection);
        assert_eq!(fast, structural);
        assert!(fast.0.iter().any(|record| {
            record.path
                == [
                    PathComponent::Index(1),
                    PathComponent::Key(Arc::from("features")),
                    PathComponent::Index(0),
                    PathComponent::Key(Arc::from("id")),
                ]
                && record
                    .value
                    .as_ref()
                    .is_some_and(|value| value.to_string() == "2")
        }));
    }

    #[test]
    fn selected_fast_discard_preserves_discarded_subtree_failures() {
        let default = StreamOptions::default();
        let depth_limited = StreamOptions {
            maximum_depth: 6,
            ..default
        };
        let token_limited = StreamOptions {
            maximum_token_bytes: 10,
            ..default
        };
        let oversized_coefficient = format!(
            "{{\"features\":[{{\"geometry\":[{}],\"properties\":{{\"release\":1}}}}]}}",
            "1".repeat(4097)
        );
        let cases = [
            (
                br#"{"features":[{"properties":{"release":1},"geometry":[1,]}]}"#.to_vec(),
                default,
                "malformed",
            ),
            (
                br#"{"features":[{"properties":{"release":1},"geometry":[[[[[[0]]]]]]}]}"#.to_vec(),
                depth_limited,
                "depth",
            ),
            (
                br#"{"features":[{"properties":{"release":1},"geometry":{"coordinates":1}}]}"#
                    .to_vec(),
                token_limited,
                "token",
            ),
            (
                br#"{"features":[{"properties":{"release":1},"geometry":[1e1000001]}]}"#.to_vec(),
                StreamOptions {
                    maximum_token_bytes: 8,
                    ..default
                },
                "exponent-token",
            ),
            (
                br#"{"features":[{"properties":{"release":1},"geometry":[1e2000000001]}]}"#
                    .to_vec(),
                default,
                "exponent-envelope",
            ),
            (oversized_coefficient.into_bytes(), default, "coefficient"),
        ];

        for (input, options, label) in cases {
            let fast = selected_outcome(&input, options, release_selection());
            let structural = structural_selected_outcome(&input, options, release_selection());
            assert_eq!(fast.0, structural.0, "records for {label}");
            assert!(fast.1.is_err(), "fast path accepted {label}");
            assert!(structural.1.is_err(), "structural path accepted {label}");

            let captured = selected_outcome(&input, options, capture_release_selection());
            assert!(captured.1.is_err(), "capture path accepted {label}");
        }
    }

    #[test]
    fn selected_fast_discard_observes_cancellation_inside_a_large_subtree() {
        struct CancelAfter {
            input: Cursor<Vec<u8>>,
            cancellation: Arc<AtomicBool>,
            trigger: u64,
        }

        impl Read for CancelAfter {
            fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
                if self.input.position() >= self.trigger {
                    self.cancellation.store(true, Ordering::Relaxed);
                }
                let length = output.len().min(1024);
                self.input.read(&mut output[..length])
            }
        }

        let cancellation = Arc::new(AtomicBool::new(false));
        let input = format!(
            "{{\"features\":[{{\"discarded\":[{}],\"properties\":{{\"release\":1}}}}]}}",
            std::iter::repeat_n("0", 8192).collect::<Vec<_>>().join(",")
        )
        .into_bytes();
        let reader = CancelAfter {
            input: Cursor::new(input),
            cancellation: Arc::clone(&cancellation),
            trigger: 1024,
        };
        let mut observations = SelectedStreamObservations::default();
        let error = stream_json_selected_records_with_control(
            BufReader::new(reader),
            StreamOptions::default(),
            release_selection(),
            Some(Arc::clone(&cancellation)),
            &mut observations,
            |_| Ok(()),
        )
        .unwrap_err();

        assert!(error.to_string().contains("selected decoding interrupted"));
        assert!(cancellation.load(Ordering::Relaxed));
        assert_eq!(observations.depth_high_water, 4);

        let capture_cancellation = Arc::new(AtomicBool::new(false));
        let capture_input = format!(
            "{{\"features\":[{{\"discarded\":[{}],\"properties\":{{\"release\":1}}}}]}}",
            std::iter::repeat_n("0", 8192).collect::<Vec<_>>().join(",")
        )
        .into_bytes();
        let capture_reader = CancelAfter {
            input: Cursor::new(capture_input),
            cancellation: Arc::clone(&capture_cancellation),
            trigger: 1024,
        };
        let capture_error = stream_json_selected_records_with_control(
            BufReader::new(capture_reader),
            StreamOptions::default(),
            capture_release_selection(),
            Some(Arc::clone(&capture_cancellation)),
            &mut SelectedStreamObservations::default(),
            |_| Ok(()),
        )
        .unwrap_err();
        assert!(
            capture_error
                .to_string()
                .contains("selected decoding interrupted")
        );
        assert!(capture_cancellation.load(Ordering::Relaxed));
    }

    #[test]
    fn selected_fast_discard_observes_source_depth_beyond_retained_depth() {
        let input = br#"{"discarded":{"deep":[[0]]},"features":[1]}"#;
        let mut records = Vec::new();
        let mut observations = SelectedStreamObservations::default();
        stream_json_selected_records_with_control(
            Cursor::new(input),
            StreamOptions::default(),
            StreamSelection::new(vec![PathComponent::Key(Arc::from("features"))], None),
            None,
            &mut observations,
            |record| {
                records.push(record);
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(observations.depth_high_water, 4);
    }
}
