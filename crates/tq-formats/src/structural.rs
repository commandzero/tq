//! Query-independent JSON structural events without root materialization.

use std::{
    fmt, io,
    io::Read,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use tq_core::{
    JsonEvent, JsonInput, JsonInputError, JsonInputOptions, JsonPosition, SourceId, Span, Value,
};
use tq_toon::{DecoderCapabilities, Event, EventConsumer, Scalar};

/// Resource bounds for query-independent JSON structural decoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JsonEventOptions {
    /// Maximum structured container nesting depth.
    pub maximum_depth: usize,
    /// Maximum decoded bytes in one string, key, or number token.
    pub maximum_token_bytes: usize,
}

impl Default for JsonEventOptions {
    fn default() -> Self {
        Self {
            maximum_depth: 256,
            maximum_token_bytes: 8 * 1024 * 1024,
        }
    }
}

/// JSON structural behavior known before semantic input consumption.
#[must_use]
pub const fn json_decoder_capabilities() -> DecoderCapabilities {
    DecoderCapabilities::json()
}

/// Emits one JSON document as ordered structural events.
///
/// # Errors
///
/// Returns JSON syntax, numeric-envelope, or consumer failures.
pub fn decode_json_events<R, C>(reader: R, source: SourceId, consumer: &mut C) -> Result<(), String>
where
    R: Read,
    C: EventConsumer,
    C::Error: fmt::Display,
{
    decode_json_events_with_options(reader, source, consumer, JsonEventOptions::default())
}

/// Emits one bounded JSON document as ordered structural events.
///
/// # Errors
///
/// Returns JSON syntax, numeric-envelope, resource-limit, or consumer failures.
pub fn decode_json_events_with_options<R, C>(
    reader: R,
    source: SourceId,
    consumer: &mut C,
    options: JsonEventOptions,
) -> Result<(), String>
where
    R: Read,
    C: EventConsumer,
    C::Error: fmt::Display,
{
    let mut checkpoint = || Ok(());
    decode_json_events_with_options_control(reader, source, consumer, options, &mut checkpoint)
}

/// Emits one bounded JSON document with a cooperative cancellation/resource
/// checkpoint invoked throughout lexical scanning.
///
/// # Errors
///
/// Returns JSON syntax, numeric-envelope, resource-limit, or consumer failures.
pub fn decode_json_events_with_options_control<R, C, F>(
    reader: R,
    source: SourceId,
    consumer: &mut C,
    options: JsonEventOptions,
    checkpoint: &mut F,
) -> Result<(), String>
where
    R: Read,
    C: EventConsumer,
    C::Error: fmt::Display,
    F: FnMut() -> io::Result<()>,
{
    let mut retain = |_path: &[tq_core::PathComponent]| true;
    decode_json_events_selected_with_options_control(
        reader,
        source,
        consumer,
        options,
        checkpoint,
        &mut retain,
        false,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn decode_json_events_selected_with_options_control<R, C, F, S>(
    reader: R,
    source: SourceId,
    consumer: &mut C,
    options: JsonEventOptions,
    checkpoint: &mut F,
    retain: &mut S,
    selection_enabled: bool,
    depth_high_water: Option<&mut usize>,
) -> Result<(), String>
where
    R: Read,
    C: EventConsumer,
    C::Error: fmt::Display,
    F: FnMut() -> io::Result<()>,
    S: FnMut(&[tq_core::PathComponent]) -> bool,
{
    // `retain` and the projector's `StreamSelection` are the same selection
    // state. The adapter may translate a `Skipped` event into a private null
    // scalar only to advance the projector's array/object indexes; that path
    // is rejected by the projector and can never become an output record.
    let mut input = JsonInput::new(
        reader,
        JsonInputOptions {
            maximum_depth: options.maximum_depth,
            maximum_token_bytes: options.maximum_token_bytes,
        },
    );
    let mut adapter = JsonEventAdapter::new(consumer, source, selection_enabled);
    let started = if selection_enabled {
        input.next_events_selected(&mut |event| adapter.consume(event), checkpoint, retain)
    } else {
        input.next_events(&mut |event| adapter.consume(event), checkpoint)
    };
    if let Some(output) = depth_high_water {
        *output = (*output).max(input.depth_high_water());
    }
    let started = started.map_err(|error| json_input_error(&error))?;
    if !started {
        return Err("JSON document contained no value".to_owned());
    }
    input
        .check_end(checkpoint)
        .map_err(|error| json_input_error(&error))?;
    adapter.finish(input.position())
}

/// Decodes a whitespace-separated JSON stream one root at a time while
/// retaining the selected structural consumer between roots.
#[allow(clippy::too_many_arguments)]
pub(crate) fn decode_json_events_selected_roots_with_options_control<R, C, B, E, A, F, S>(
    reader: R,
    source: SourceId,
    consumer: &mut C,
    options: JsonEventOptions,
    cancellation: Option<Arc<AtomicBool>>,
    on_begin: &mut B,
    on_finish: &mut E,
    on_abort: &mut A,
    checkpoint: &mut F,
    retain: &mut S,
    depth_high_water: Option<&mut usize>,
) -> Result<u64, JsonInputError>
where
    R: Read,
    C: EventConsumer,
    C::Error: fmt::Display,
    B: FnMut(u64) -> Result<(), String>,
    E: FnMut() -> Result<(), String>,
    A: FnMut(),
    F: FnMut() -> io::Result<()>,
    S: FnMut(&[tq_core::PathComponent]) -> bool,
{
    let cancellation = cancellation.map(|flag| Arc::clone(&flag));
    let mut input = JsonInput::new(
        reader,
        JsonInputOptions {
            maximum_depth: options.maximum_depth,
            maximum_token_bytes: options.maximum_token_bytes,
        },
    );
    let mut documents = 0_u64;
    let mut depth_high_water = depth_high_water;
    loop {
        let mut adapter = JsonEventAdapter::new(consumer, source, true);
        let mut root_started = false;
        let mut root_opened = false;
        let cancellation = cancellation.clone();
        let mut root_checkpoint = || {
            if cancellation
                .as_ref()
                .is_some_and(|flag| flag.load(Ordering::Relaxed))
            {
                return Err(io::Error::other("selected decoding interrupted"));
            }
            checkpoint()
        };
        let started = input.next_events_selected(
            &mut |event| {
                if !root_started {
                    root_started = true;
                    on_begin(documents).map_err(io::Error::other)?;
                    root_opened = true;
                }
                adapter.consume(event)
            },
            &mut root_checkpoint,
            retain,
        );
        if let Some(output) = depth_high_water.as_deref_mut() {
            *output = (*output).max(input.depth_high_water());
        }
        let started = match started {
            Ok(started) => started,
            Err(error) => {
                if root_opened {
                    on_abort();
                }
                return Err(error);
            }
        };
        if !started {
            return Ok(documents);
        }
        if let Err(error) = adapter.finish(input.position()) {
            if root_opened {
                on_abort();
            }
            return Err(JsonInputError::Consumer(io::Error::other(error)));
        }
        if let Err(error) = on_finish() {
            if root_opened {
                on_abort();
            }
            return Err(JsonInputError::Consumer(io::Error::other(error)));
        }
        documents = documents.saturating_add(1);
    }
}

/// Emits a whitespace-separated stream of bounded JSON documents.
///
/// # Errors
///
/// Returns JSON syntax, numeric-envelope, resource-limit, or consumer failures.
pub fn decode_json_event_stream<R, C>(
    reader: R,
    source: SourceId,
    consumer: &mut C,
    options: JsonEventOptions,
) -> Result<u64, String>
where
    R: Read,
    C: EventConsumer,
    C::Error: fmt::Display,
{
    decode_json_event_stream_typed(reader, source, consumer, options)
        .map_err(|error| json_input_error(&error))
}

/// Typed counterpart to [`decode_json_event_stream`] for adapters that must
/// distinguish syntax/resource failures from consumer or I/O failures.
pub(crate) fn decode_json_event_stream_typed<R, C>(
    reader: R,
    source: SourceId,
    consumer: &mut C,
    options: JsonEventOptions,
) -> Result<u64, JsonInputError>
where
    R: Read,
    C: EventConsumer,
    C::Error: fmt::Display,
{
    let mut input = JsonInput::new(
        reader,
        JsonInputOptions {
            maximum_depth: options.maximum_depth,
            maximum_token_bytes: options.maximum_token_bytes,
        },
    );
    let mut documents = 0_u64;
    loop {
        let mut adapter = JsonEventAdapter::new(consumer, source, false);
        let mut checkpoint = || Ok(());
        let started = input.next_events(&mut |event| adapter.consume(event), &mut checkpoint)?;
        if !started {
            return Ok(documents);
        }
        adapter
            .finish(input.position())
            .map_err(|error| JsonInputError::Consumer(io::Error::other(error)))?;
        documents = documents.saturating_add(1);
    }
}

#[derive(Clone, Copy, Debug)]
enum JsonEventFrame {
    Array { count: u64 },
    Object,
}

struct JsonEventAdapter<'a, C> {
    consumer: &'a mut C,
    source: SourceId,
    selected: bool,
    started: bool,
    frames: Vec<JsonEventFrame>,
    pending_key: Option<(std::sync::Arc<str>, JsonPosition)>,
    pending_root_scalar: Option<(Value, JsonPosition)>,
}

impl<'a, C> JsonEventAdapter<'a, C>
where
    C: EventConsumer,
    C::Error: fmt::Display,
{
    fn new(consumer: &'a mut C, source: SourceId, selected: bool) -> Self {
        Self {
            consumer,
            source,
            selected,
            started: false,
            frames: Vec::new(),
            pending_key: None,
            pending_root_scalar: None,
        }
    }

    fn consume(&mut self, event: JsonEvent) -> io::Result<()> {
        let position = match &event {
            JsonEvent::StartArray(position)
            | JsonEvent::EndArray(position)
            | JsonEvent::StartObject(position)
            | JsonEvent::EndObject(position)
            | JsonEvent::Key { position, .. }
            | JsonEvent::Scalar { position, .. }
            | JsonEvent::Skipped { position } => *position,
        };
        if !self.started {
            self.send(Event::DocumentStart {
                span: self.start_span(position),
            })?;
            self.started = true;
        }
        match event {
            JsonEvent::StartArray(position) => {
                self.flush_pending_key()?;
                self.send(Event::ArrayStart {
                    span: self.start_span(position),
                    declared_count: None,
                })?;
                self.frames.push(JsonEventFrame::Array { count: 0 });
            }
            JsonEvent::EndArray(position) => {
                let Some(JsonEventFrame::Array { count }) = self.frames.pop() else {
                    return Err(io::Error::other("JSON array frame underflow"));
                };
                self.send(Event::ArrayEnd {
                    span: self.end_span(position),
                    observed_count: count,
                })?;
                self.complete_value();
            }
            JsonEvent::StartObject(position) => {
                self.flush_pending_key()?;
                self.send(Event::ObjectStart {
                    span: self.start_span(position),
                })?;
                self.frames.push(JsonEventFrame::Object);
            }
            JsonEvent::EndObject(position) => {
                let Some(JsonEventFrame::Object) = self.frames.pop() else {
                    return Err(io::Error::other("JSON object frame underflow"));
                };
                self.send(Event::ObjectEnd {
                    span: self.end_span(position),
                })?;
                self.complete_value();
            }
            JsonEvent::Key { value, position } => {
                if !matches!(self.frames.last(), Some(JsonEventFrame::Object)) {
                    return Err(io::Error::other("JSON key outside object"));
                }
                if self.selected {
                    if self.pending_key.replace((value, position)).is_some() {
                        return Err(io::Error::other("JSON object key has no value"));
                    }
                } else {
                    self.consumer
                        .consume_text_key(self.start_span(position), value.to_string(), true)
                        .map_err(io::Error::other)?;
                }
            }
            JsonEvent::Scalar { value, position } => {
                self.flush_pending_key()?;
                if self.frames.is_empty() {
                    self.pending_root_scalar = Some((value, position));
                } else {
                    self.consume_scalar(value, position)?;
                }
                self.complete_value();
            }
            JsonEvent::Skipped { position } => {
                if !self.selected {
                    return Err(io::Error::other("skipped event outside selected adapter"));
                }
                if let Some((value, key_position)) = self.pending_key.take() {
                    self.send(Event::Key {
                        span: self.start_span(key_position),
                        value,
                        quoted: true,
                    })?;
                }
                self.send(Event::Scalar {
                    span: self.start_span(position),
                    value: Scalar::Null,
                })?;
                self.complete_value();
            }
        }
        Ok(())
    }

    fn flush_pending_key(&mut self) -> io::Result<()> {
        let Some((value, position)) = self.pending_key.take() else {
            return Ok(());
        };
        self.consumer
            .consume_text_key(self.start_span(position), value.to_string(), true)
            .map_err(io::Error::other)
    }

    fn consume_scalar(&mut self, value: Value, position: JsonPosition) -> io::Result<()> {
        let span = self.start_span(position);
        match value {
            Value::Null => self.consumer.consume_null(span).map_err(io::Error::other),
            Value::Bool(value) => self
                .consumer
                .consume_bool(span, value)
                .map_err(io::Error::other),
            Value::String(value) => self
                .consumer
                .consume_text_string(span, value.to_string())
                .map_err(io::Error::other),
            Value::Number(number) => {
                if let Some(literal) = number.exact_literal() {
                    self.consumer
                        .consume_number_literal(span, literal.to_owned())
                        .map_err(io::Error::other)
                } else {
                    self.send(Event::Scalar {
                        span,
                        value: Scalar::Number(number),
                    })
                }
            }
            Value::Array(_) | Value::Object(_) => {
                Err(io::Error::other("JSON scalar contained a composite value"))
            }
        }
    }

    fn complete_value(&mut self) {
        if let Some(JsonEventFrame::Array { count }) = self.frames.last_mut() {
            *count = count.saturating_add(1);
        }
    }

    fn finish(mut self, position: JsonPosition) -> Result<(), String> {
        if !self.started {
            return Err("JSON document contained no value".to_owned());
        }
        if !self.frames.is_empty() {
            return Err("JSON document ended with an open container".to_owned());
        }
        self.flush_pending_key()
            .map_err(|error| error.to_string())?;
        if let Some((value, position)) = self.pending_root_scalar.take() {
            self.consume_scalar(value, position)
                .map_err(|error| error.to_string())?;
        }
        self.consumer
            .consume(Event::DocumentEnd {
                span: self.end_span(position),
            })
            .map_err(|error| error.to_string())
    }

    fn send(&mut self, event: Event) -> io::Result<()> {
        self.consumer
            .consume(event)
            .map_err(|error| io::Error::other(error.to_string()))
    }

    const fn start_span(&self, position: JsonPosition) -> Span {
        Span::new(self.source, position.offset as u64, position.offset as u64)
    }

    const fn end_span(&self, position: JsonPosition) -> Span {
        Span::new(
            self.source,
            position.offset.saturating_sub(1) as u64,
            position.offset as u64,
        )
    }
}

fn json_input_error(error: &JsonInputError) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use std::{
        cell::Cell,
        convert::Infallible,
        io::{self, Cursor, Read},
        rc::Rc,
    };

    use tq_core::SourceId;
    use tq_toon::{Event, EventConsumer, Scalar};

    use super::{JsonEventOptions, decode_json_event_stream, decode_json_events};

    #[derive(Default)]
    struct Collector(Vec<Event>);

    impl EventConsumer for Collector {
        type Error = Infallible;

        fn consume(&mut self, event: Event) -> Result<(), Self::Error> {
            self.0.push(event);
            Ok(())
        }
    }

    #[derive(Default)]
    struct TokenCollector {
        events: Vec<Event>,
        keys: Vec<String>,
        strings: Vec<String>,
        numbers: Vec<String>,
        nulls: usize,
        booleans: Vec<bool>,
    }

    impl EventConsumer for TokenCollector {
        type Error = Infallible;

        fn consume(&mut self, event: Event) -> Result<(), Self::Error> {
            self.events.push(event);
            Ok(())
        }

        fn consume_text_key(
            &mut self,
            _span: tq_core::Span,
            value: String,
            _quoted: bool,
        ) -> Result<(), String> {
            self.keys.push(value);
            Ok(())
        }

        fn consume_null(&mut self, _span: tq_core::Span) -> Result<(), String> {
            self.nulls += 1;
            Ok(())
        }

        fn consume_bool(&mut self, _span: tq_core::Span, value: bool) -> Result<(), String> {
            self.booleans.push(value);
            Ok(())
        }

        fn consume_text_string(
            &mut self,
            _span: tq_core::Span,
            value: String,
        ) -> Result<(), String> {
            self.strings.push(value);
            Ok(())
        }

        fn consume_number_literal(
            &mut self,
            _span: tq_core::Span,
            literal: String,
        ) -> Result<(), String> {
            self.numbers.push(literal);
            Ok(())
        }
    }

    #[test]
    fn emits_ordered_duplicate_keys_and_exact_numbers_without_a_root_value() {
        let mut collector = Collector::default();
        decode_json_events(
            br#"{"b":1,"a":[9007199254740993],"b":2}"#.as_slice(),
            SourceId::new(1),
            &mut collector,
        )
        .unwrap();

        let keys = collector
            .0
            .iter()
            .filter_map(|event| match event {
                Event::Key { value, .. } => Some(value.as_ref()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(keys, ["b", "a", "b"]);
        assert!(collector.0.iter().any(|event| matches!(
            event,
            Event::Scalar {
                value: Scalar::Number(number),
                ..
            } if number.to_string() == "9007199254740993"
        )));
    }

    #[test]
    fn delivers_json_tokens_without_scalar_or_key_events_when_supported() {
        let mut collector = TokenCollector::default();
        decode_json_events(
            br#"{"key":["value",1e2,true,null]}"#.as_slice(),
            SourceId::new(1),
            &mut collector,
        )
        .unwrap();

        assert_eq!(collector.keys, ["key"]);
        assert_eq!(collector.strings, ["value"]);
        // The shared core reader canonicalizes finite number literals before
        // delivering them to the structural consumer.
        assert_eq!(collector.numbers, ["1E+2"]);
        assert_eq!(collector.booleans, [true]);
        assert_eq!(collector.nulls, 1);
        assert!(
            collector
                .events
                .iter()
                .all(|event| !matches!(event, Event::Key { .. } | Event::Scalar { .. }))
        );
    }

    #[test]
    fn malformed_late_input_does_not_emit_document_end() {
        let mut collector = Collector::default();
        assert!(
            decode_json_events(
                br#"{"a":1,"b":"#.as_slice(),
                SourceId::new(1),
                &mut collector
            )
            .is_err()
        );
        assert!(
            !collector
                .0
                .iter()
                .any(|event| matches!(event, Event::DocumentEnd { .. }))
        );
    }

    #[test]
    fn malformed_containers_do_not_publish_unconfirmed_scalars() {
        for input in [
            br"[1 2]".as_slice(),
            br#"["a" 2]"#.as_slice(),
            br"[true 2]".as_slice(),
            br#"{"a":1 "b":2}"#.as_slice(),
            br"[1".as_slice(),
        ] {
            let mut collector = Collector::default();
            assert!(decode_json_events(input, SourceId::new(1), &mut collector).is_err());
            assert!(
                !collector
                    .0
                    .iter()
                    .any(|event| matches!(event, Event::Scalar { .. }))
            );
        }

        let mut collector = Collector::default();
        assert!(decode_json_events(br"[1,".as_slice(), SourceId::new(1), &mut collector).is_err());
        assert!(
            collector
                .0
                .iter()
                .any(|event| matches!(event, Event::Scalar { .. }))
        );
    }

    #[test]
    fn exact_one_rejects_a_trailing_root_without_draining_it() {
        struct OneByteReader {
            input: Cursor<Vec<u8>>,
            bytes_read: Rc<Cell<usize>>,
        }

        impl Read for OneByteReader {
            fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
                if buffer.is_empty() {
                    return Ok(0);
                }
                let count = self.input.read(&mut buffer[..1])?;
                self.bytes_read
                    .set(self.bytes_read.get().saturating_add(count));
                Ok(count)
            }
        }

        let input = b"1 {this second root is intentionally unfinished".to_vec();
        let bytes_read = Rc::new(Cell::new(0));
        let reader = OneByteReader {
            input: Cursor::new(input.clone()),
            bytes_read: Rc::clone(&bytes_read),
        };
        let mut collector = Collector::default();
        let error = decode_json_events(reader, SourceId::new(1), &mut collector).unwrap_err();
        assert!(error.contains("trailing JSON input"));
        assert!(bytes_read.get() < input.len());
    }

    #[test]
    fn streams_multiple_documents_without_a_phantom_eof_document() {
        let mut collector = Collector::default();
        assert_eq!(
            decode_json_event_stream(
                b"1 {\"x\":2}  \n".as_slice(),
                SourceId::new(1),
                &mut collector,
                JsonEventOptions::default(),
            )
            .unwrap(),
            2
        );
        assert_eq!(
            collector
                .0
                .iter()
                .filter(|event| matches!(event, Event::DocumentStart { .. }))
                .count(),
            2
        );
    }

    #[test]
    fn enforces_depth_and_token_limits() {
        let options = JsonEventOptions {
            maximum_depth: 1,
            maximum_token_bytes: 3,
        };
        let mut collector = Collector::default();
        assert!(
            super::decode_json_events_with_options(
                b"[[1]]".as_slice(),
                SourceId::new(1),
                &mut collector,
                options,
            )
            .unwrap_err()
            .contains("depth")
        );
        let mut collector = Collector::default();
        assert!(
            super::decode_json_events_with_options(
                br#""oversized""#.as_slice(),
                SourceId::new(1),
                &mut collector,
                options,
            )
            .unwrap_err()
            .contains("token-bytes")
        );
    }
}
