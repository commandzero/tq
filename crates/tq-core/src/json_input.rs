//! Incremental jq-compatible JSON input.
//!
//! This module deliberately owns the JSON grammar used by the core and the
//! format adapters.  In particular, it does not route through `serde_json`:
//! `serde_json` rejects jq's non-finite input tokens before a custom visitor can
//! observe them.

use std::{
    fmt,
    io::{self, BufRead, BufReader, Cursor, Read},
    sync::Arc,
};

use thiserror::Error;

use crate::{Number, NumberError, Object, PathComponent, Value};

const ASCII_FAST_CHUNK: usize = 4096;

/// Position of the next unread byte in a JSON input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JsonPosition {
    /// Zero-based byte offset.
    pub offset: usize,
    /// One-based line number.
    pub line: usize,
    /// One-based column number in bytes.
    pub column: usize,
}

impl fmt::Display for JsonPosition {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "byte {}, line {}, column {}",
            self.offset, self.line, self.column
        )
    }
}

impl Default for JsonPosition {
    fn default() -> Self {
        Self {
            offset: 0,
            line: 1,
            column: 1,
        }
    }
}

/// JSON parser resource limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JsonInputOptions {
    /// Maximum active array/object nesting depth.
    pub maximum_depth: usize,
    /// Maximum decoded bytes in one string, key, or scalar token.
    pub maximum_token_bytes: usize,
}

impl Default for JsonInputOptions {
    fn default() -> Self {
        Self {
            maximum_depth: 256,
            maximum_token_bytes: 8 * 1024 * 1024,
        }
    }
}

/// Resource categories reported by the incremental parser.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JsonLimit {
    /// A string, key, or scalar token exceeded its byte budget.
    TokenBytes,
    /// A nested array or object exceeded the depth budget.
    Depth,
}

impl fmt::Display for JsonLimit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TokenBytes => "token-bytes",
            Self::Depth => "depth",
        })
    }
}

/// Failure while incrementally decoding JSON.
#[derive(Debug, Error)]
pub enum JsonInputError {
    /// The source reader failed.
    #[error("JSON I/O error: {0}")]
    Io(#[source] io::Error),
    /// The event consumer failed.
    #[error("JSON consumer error: {0}")]
    Consumer(#[source] io::Error),
    /// The source exceeded a parser resource limit.
    #[error("JSON input limit exceeded ({limit}) at byte {position}")]
    Limit {
        /// Limit category.
        limit: JsonLimit,
        /// Position where the limit became observable.
        position: usize,
    },
    /// The source was not valid jq-compatible JSON.
    #[error("invalid JSON at {position}: {message}")]
    Syntax {
        /// Position at which the syntax error was found.
        position: JsonPosition,
        /// Bounded diagnostic text.
        message: Arc<str>,
    },
}

/// One incrementally decoded JSON event.
///
/// Container events carry no materialized subtree.  A consumer receives one
/// scalar at a time and sees every object key, including keys later replaced
/// by a duplicate key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JsonEvent {
    /// An array opener was consumed.
    StartArray(JsonPosition),
    /// An array closer was consumed.
    EndArray(JsonPosition),
    /// An object opener was consumed.
    StartObject(JsonPosition),
    /// An object closer was consumed.
    EndObject(JsonPosition),
    /// An object key was decoded.
    Key {
        /// Decoded key.
        value: Arc<str>,
        /// Position of the opening quote.
        position: JsonPosition,
    },
    /// A scalar was decoded.  This is never an array or object.
    Scalar {
        /// Decoded scalar value.
        value: Value,
        /// Position at which the scalar started.
        position: JsonPosition,
    },
    /// A selected value was validated and intentionally not retained.
    ///
    /// This marker has no value payload. A structural adapter may use it as
    /// private bookkeeping to advance array/object position, but must not
    /// expose it as a stream result.
    Skipped {
        /// Position at which the skipped value started.
        position: JsonPosition,
    },
}

/// Incremental jq-compatible JSON reader.
///
/// The reader consumes at most one root value per call.  It may read ahead in
/// its private buffered reader, but its public position is always the number
/// of bytes actually consumed by the parser; a later malformed root is not
/// inspected until a later pull.
pub struct JsonInput<R> {
    reader: BufReader<R>,
    options: JsonInputOptions,
    position: JsonPosition,
    bom_checked: bool,
    depth_high_water: usize,
}

impl<R: Read> JsonInput<R> {
    /// Creates an incremental reader with the supplied limits.
    #[must_use]
    pub fn new(reader: R, options: JsonInputOptions) -> Self {
        Self {
            reader: BufReader::new(reader),
            options,
            position: JsonPosition::default(),
            bom_checked: false,
            depth_high_water: 0,
        }
    }

    /// Returns the position of the next parser byte.
    #[must_use]
    pub const fn position(&self) -> JsonPosition {
        self.position
    }

    /// Returns the largest number of source containers active during parsing.
    ///
    /// This includes containers whose contents were validated without being
    /// materialized by [`Self::next_events_selected`].
    #[must_use]
    pub const fn depth_high_water(&self) -> usize {
        self.depth_high_water
    }

    /// Pulls one root value, materializing only that value.
    ///
    /// # Errors
    ///
    /// Returns syntax, I/O, consumer, cancellation, or resource-limit errors.
    pub fn next_value(
        &mut self,
        checkpoint: &mut impl FnMut() -> io::Result<()>,
    ) -> Result<Option<Value>, JsonInputError> {
        let mut builder = ValueBuilder::default();
        let mut emit = |event| builder.accept(event);
        if !self.next_events(&mut emit, checkpoint)? {
            return Ok(None);
        }
        builder.finish().map(Some)
    }

    /// Pulls one root value as incremental events.
    ///
    /// # Errors
    ///
    /// Returns syntax, I/O, consumer, cancellation, or resource-limit errors.
    #[allow(clippy::too_many_lines)]
    pub fn next_events(
        &mut self,
        consumer: &mut impl FnMut(JsonEvent) -> io::Result<()>,
        checkpoint: &mut impl FnMut() -> io::Result<()>,
    ) -> Result<bool, JsonInputError> {
        let mut retain = |_path: &[PathComponent]| true;
        self.next_events_with_selection(consumer, checkpoint, &mut retain, false)
    }

    /// Pulls one root value as events while skipping values outside a selected
    /// path. The selector is called before each value is parsed; returning
    /// `false` validates that value with the same grammar and limits but emits
    /// no materialized subtree or scalar. A selected structural adapter may
    /// still receive surrounding `Key` and `Skipped` events so it can advance
    /// its independent array/object position state.
    ///
    /// # Errors
    ///
    /// Returns syntax, I/O, consumer, cancellation, or resource-limit errors.
    pub fn next_events_selected(
        &mut self,
        consumer: &mut impl FnMut(JsonEvent) -> io::Result<()>,
        checkpoint: &mut impl FnMut() -> io::Result<()>,
        retain: &mut impl FnMut(&[PathComponent]) -> bool,
    ) -> Result<bool, JsonInputError> {
        self.next_events_with_selection(consumer, checkpoint, retain, true)
    }

    #[allow(clippy::too_many_lines)]
    fn next_events_with_selection(
        &mut self,
        consumer: &mut impl FnMut(JsonEvent) -> io::Result<()>,
        checkpoint: &mut impl FnMut() -> io::Result<()>,
        retain: &mut impl FnMut(&[PathComponent]) -> bool,
        selection_enabled: bool,
    ) -> Result<bool, JsonInputError> {
        self.skip_whitespace(checkpoint)?;
        if self.peek_byte()?.is_none() {
            return Ok(false);
        }

        let mut frames = Vec::new();
        let mut pending_scalar = None;
        let mut root_complete = false;
        while !root_complete {
            Self::checkpoint(checkpoint)?;
            let state = frames.last().map(Frame::state);
            match state {
                None => self.parse_value(
                    &mut frames,
                    &mut root_complete,
                    &mut pending_scalar,
                    consumer,
                    checkpoint,
                    retain,
                    selection_enabled,
                )?,
                Some(FrameState::ArrayValueOrEnd) => {
                    self.skip_whitespace(checkpoint)?;
                    if self.peek_byte()? == Some(b']') {
                        self.consume_byte(checkpoint)?;
                        let frame = frames.pop().expect("array state has a frame");
                        if frame.materialize() {
                            Self::flush_pending_scalar(&mut pending_scalar, consumer)?;
                            consumer(JsonEvent::EndArray(self.position))
                                .map_err(JsonInputError::Consumer)?;
                        } else if let Some(position) = frame.skip_position() {
                            consumer(JsonEvent::Skipped { position })
                                .map_err(JsonInputError::Consumer)?;
                        }
                        self.complete_value(&mut frames, &mut root_complete)?;
                    } else {
                        self.parse_value(
                            &mut frames,
                            &mut root_complete,
                            &mut pending_scalar,
                            consumer,
                            checkpoint,
                            retain,
                            selection_enabled,
                        )?;
                    }
                }
                Some(FrameState::ArrayCommaOrEnd) => {
                    self.skip_whitespace(checkpoint)?;
                    match self.peek_byte()? {
                        Some(b',') => {
                            self.consume_byte(checkpoint)?;
                            Self::flush_pending_scalar(&mut pending_scalar, consumer)?;
                            if let Some(Frame::Array { state, .. }) = frames.last_mut() {
                                *state = FrameState::ArrayValue;
                            }
                        }
                        Some(b']') => {
                            self.consume_byte(checkpoint)?;
                            let frame = frames.pop().expect("array state has a frame");
                            if frame.materialize() {
                                Self::flush_pending_scalar(&mut pending_scalar, consumer)?;
                                consumer(JsonEvent::EndArray(self.position))
                                    .map_err(JsonInputError::Consumer)?;
                            } else if let Some(position) = frame.skip_position() {
                                consumer(JsonEvent::Skipped { position })
                                    .map_err(JsonInputError::Consumer)?;
                            }
                            self.complete_value(&mut frames, &mut root_complete)?;
                        }
                        _ => return self.syntax("expected ',' or ']'"),
                    }
                }
                Some(FrameState::ArrayValue) => {
                    self.skip_whitespace(checkpoint)?;
                    self.parse_value(
                        &mut frames,
                        &mut root_complete,
                        &mut pending_scalar,
                        consumer,
                        checkpoint,
                        retain,
                        selection_enabled,
                    )?;
                }
                Some(FrameState::ObjectKeyOrEnd) => {
                    self.skip_whitespace(checkpoint)?;
                    if self.peek_byte()? == Some(b'}') {
                        self.consume_byte(checkpoint)?;
                        let frame = frames.pop().expect("object state has a frame");
                        if frame.materialize() {
                            Self::flush_pending_scalar(&mut pending_scalar, consumer)?;
                            consumer(JsonEvent::EndObject(self.position))
                                .map_err(JsonInputError::Consumer)?;
                        } else if let Some(position) = frame.skip_position() {
                            consumer(JsonEvent::Skipped { position })
                                .map_err(JsonInputError::Consumer)?;
                        }
                        self.complete_value(&mut frames, &mut root_complete)?;
                    } else if self.peek_byte()? == Some(b'"') {
                        if frames.last().is_some_and(Frame::materialize) {
                            let position = self.position;
                            let key = self.read_string(checkpoint)?;
                            consumer(JsonEvent::Key {
                                value: Arc::clone(&key),
                                position,
                            })
                            .map_err(JsonInputError::Consumer)?;
                            if let Some(Frame::Object {
                                state, pending_key, ..
                            }) = frames.last_mut()
                            {
                                *pending_key = Some(key);
                                *state = FrameState::ObjectColon;
                            }
                        } else {
                            self.skip_string(checkpoint)?;
                            if let Some(Frame::Object {
                                state, pending_key, ..
                            }) = frames.last_mut()
                            {
                                *pending_key = None;
                                *state = FrameState::ObjectColon;
                            }
                        }
                    } else {
                        return self.syntax("expected object key or '}'");
                    }
                }
                Some(FrameState::ObjectKey) => {
                    self.skip_whitespace(checkpoint)?;
                    if self.peek_byte()? != Some(b'"') {
                        return self.syntax("expected object key after ','");
                    }
                    if frames.last().is_some_and(Frame::materialize) {
                        let position = self.position;
                        let key = self.read_string(checkpoint)?;
                        consumer(JsonEvent::Key {
                            value: Arc::clone(&key),
                            position,
                        })
                        .map_err(JsonInputError::Consumer)?;
                        if let Some(Frame::Object {
                            state, pending_key, ..
                        }) = frames.last_mut()
                        {
                            *pending_key = Some(key);
                            *state = FrameState::ObjectColon;
                        }
                    } else {
                        self.skip_string(checkpoint)?;
                        if let Some(Frame::Object {
                            state, pending_key, ..
                        }) = frames.last_mut()
                        {
                            *pending_key = None;
                            *state = FrameState::ObjectColon;
                        }
                    }
                }
                Some(FrameState::ObjectColon) => {
                    self.skip_whitespace(checkpoint)?;
                    if self.peek_byte()? != Some(b':') {
                        return self.syntax("expected ':' after object key");
                    }
                    self.consume_byte(checkpoint)?;
                    if let Some(Frame::Object { state, .. }) = frames.last_mut() {
                        *state = FrameState::ObjectValue;
                    }
                }
                Some(FrameState::ObjectValue) => {
                    self.parse_value(
                        &mut frames,
                        &mut root_complete,
                        &mut pending_scalar,
                        consumer,
                        checkpoint,
                        retain,
                        selection_enabled,
                    )?;
                }
                Some(FrameState::ObjectCommaOrEnd) => {
                    self.skip_whitespace(checkpoint)?;
                    match self.peek_byte()? {
                        Some(b',') => {
                            self.consume_byte(checkpoint)?;
                            Self::flush_pending_scalar(&mut pending_scalar, consumer)?;
                            if let Some(Frame::Object { state, .. }) = frames.last_mut() {
                                *state = FrameState::ObjectKey;
                            }
                        }
                        Some(b'}') => {
                            self.consume_byte(checkpoint)?;
                            let frame = frames.pop().expect("object state has a frame");
                            if frame.materialize() {
                                Self::flush_pending_scalar(&mut pending_scalar, consumer)?;
                                consumer(JsonEvent::EndObject(self.position))
                                    .map_err(JsonInputError::Consumer)?;
                            } else if let Some(position) = frame.skip_position() {
                                consumer(JsonEvent::Skipped { position })
                                    .map_err(JsonInputError::Consumer)?;
                            }
                            self.complete_value(&mut frames, &mut root_complete)?;
                        }
                        _ => return self.syntax("expected ',' or '}'"),
                    }
                }
            }
        }
        Self::flush_pending_scalar(&mut pending_scalar, consumer)?;
        Ok(true)
    }

    fn flush_pending_scalar(
        pending: &mut Option<(Value, JsonPosition)>,
        consumer: &mut impl FnMut(JsonEvent) -> io::Result<()>,
    ) -> Result<(), JsonInputError> {
        if let Some((value, position)) = pending.take() {
            consumer(JsonEvent::Scalar { value, position }).map_err(JsonInputError::Consumer)?;
        }
        Ok(())
    }

    /// Confirms that only whitespace remains after one root value.
    ///
    /// This performs a single lookahead rather than pulling a second value, so
    /// an exact-one consumer does not scan or materialize a later root.
    ///
    /// # Errors
    ///
    /// Returns a source, cancellation, or trailing-data error.
    pub fn check_end(
        &mut self,
        checkpoint: &mut impl FnMut() -> io::Result<()>,
    ) -> Result<(), JsonInputError> {
        self.skip_whitespace(checkpoint)?;
        if self.peek_byte()?.is_some() {
            return self.syntax("trailing JSON input");
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn parse_value(
        &mut self,
        frames: &mut Vec<Frame>,
        root_complete: &mut bool,
        pending_scalar: &mut Option<(Value, JsonPosition)>,
        consumer: &mut impl FnMut(JsonEvent) -> io::Result<()>,
        checkpoint: &mut impl FnMut() -> io::Result<()>,
        retain: &mut impl FnMut(&[PathComponent]) -> bool,
        selection_enabled: bool,
    ) -> Result<(), JsonInputError> {
        self.skip_whitespace(checkpoint)?;
        let position = self.position;
        let parent_materialize = frames.last().is_none_or(Frame::materialize);
        let path = if selection_enabled && parent_materialize {
            Some(Self::value_path(frames))
        } else {
            None
        };
        let materialize = parent_materialize && path.as_deref().is_none_or(retain);
        let notify_skip = selection_enabled && parent_materialize && !materialize;
        match self.peek_byte()? {
            Some(b'[') => {
                if frames.len() >= self.options.maximum_depth {
                    return Err(JsonInputError::Limit {
                        limit: JsonLimit::Depth,
                        position: self.position.offset,
                    });
                }
                self.consume_byte(checkpoint)?;
                self.depth_high_water = self.depth_high_water.max(frames.len().saturating_add(1));
                if materialize {
                    consumer(JsonEvent::StartArray(position)).map_err(JsonInputError::Consumer)?;
                }
                frames.try_reserve(1).map_err(|_| JsonInputError::Limit {
                    limit: JsonLimit::Depth,
                    position: self.position.offset,
                })?;
                frames.push(Frame::Array {
                    state: FrameState::ArrayValueOrEnd,
                    path: path.filter(|_| materialize),
                    next_index: 0,
                    materialize,
                    skip_position: notify_skip.then_some(position),
                });
            }
            Some(b'{') => {
                if frames.len() >= self.options.maximum_depth {
                    return Err(JsonInputError::Limit {
                        limit: JsonLimit::Depth,
                        position: self.position.offset,
                    });
                }
                self.consume_byte(checkpoint)?;
                self.depth_high_water = self.depth_high_water.max(frames.len().saturating_add(1));
                if materialize {
                    consumer(JsonEvent::StartObject(position)).map_err(JsonInputError::Consumer)?;
                }
                frames.try_reserve(1).map_err(|_| JsonInputError::Limit {
                    limit: JsonLimit::Depth,
                    position: self.position.offset,
                })?;
                frames.push(Frame::Object {
                    state: FrameState::ObjectKeyOrEnd,
                    path: path.filter(|_| materialize),
                    pending_key: None,
                    materialize,
                    skip_position: notify_skip.then_some(position),
                });
            }
            Some(b'"') => {
                if materialize {
                    let value = Value::string(self.read_string(checkpoint)?);
                    pending_scalar.replace((value, position));
                } else {
                    self.skip_string(checkpoint)?;
                }
                self.complete_value(frames, root_complete)?;
                if notify_skip {
                    consumer(JsonEvent::Skipped { position }).map_err(JsonInputError::Consumer)?;
                }
            }
            Some(b't' | b'f' | b'n') => {
                let token = self.read_word(checkpoint)?;
                if materialize {
                    let value = parse_scalar_token(&token).map_err(|error| match error {
                        ScalarError::Invalid(message) => JsonInputError::Syntax {
                            position: self.position,
                            message: message.into(),
                        },
                        ScalarError::Limit(_message) => JsonInputError::Limit {
                            limit: JsonLimit::TokenBytes,
                            position: self.position.offset,
                        },
                    })?;
                    pending_scalar.replace((value, position));
                } else {
                    self.validate_scalar_token(&token)?;
                }
                self.complete_value(frames, root_complete)?;
                if notify_skip {
                    consumer(JsonEvent::Skipped { position }).map_err(JsonInputError::Consumer)?;
                }
            }
            Some(b'N' | b's' | b'S' | b'I' | b'i' | b'+' | b'-' | b'.' | b'0'..=b'9') => {
                let token = self.read_token(checkpoint)?;
                if materialize {
                    let value = parse_scalar_token(&token).map_err(|error| match error {
                        ScalarError::Invalid(message) => JsonInputError::Syntax {
                            position: self.position,
                            message: message.into(),
                        },
                        ScalarError::Limit(_message) => JsonInputError::Limit {
                            limit: JsonLimit::TokenBytes,
                            position: self.position.offset,
                        },
                    })?;
                    pending_scalar.replace((value, position));
                } else {
                    self.validate_scalar_token(&token)?;
                }
                self.complete_value(frames, root_complete)?;
                if notify_skip {
                    consumer(JsonEvent::Skipped { position }).map_err(JsonInputError::Consumer)?;
                }
            }
            _ => return self.syntax("expected JSON value"),
        }
        Ok(())
    }

    fn validate_scalar_token(&self, token: &str) -> Result<(), JsonInputError> {
        if matches!(token, "null" | "true" | "false") {
            return Ok(());
        }
        if token.len() > self.options.maximum_token_bytes {
            return Err(JsonInputError::Limit {
                limit: JsonLimit::TokenBytes,
                position: self.position.offset,
            });
        }
        validate_jq_number(token).map_err(|error| match number_error(&error) {
            ScalarError::Invalid(message) => JsonInputError::Syntax {
                position: self.position,
                message: message.into(),
            },
            ScalarError::Limit(_) => JsonInputError::Limit {
                limit: JsonLimit::TokenBytes,
                position: self.position.offset,
            },
        })
    }

    fn complete_value(
        &self,
        frames: &mut [Frame],
        root_complete: &mut bool,
    ) -> Result<(), JsonInputError> {
        let Some(frame) = frames.last() else {
            *root_complete = true;
            return Ok(());
        };
        match frame {
            Frame::Array {
                state: FrameState::ArrayValueOrEnd | FrameState::ArrayValue,
                ..
            } => {
                if let Some(Frame::Array {
                    state, next_index, ..
                }) = frames.last_mut()
                {
                    *state = FrameState::ArrayCommaOrEnd;
                    *next_index = next_index.saturating_add(1);
                }
                Ok(())
            }
            Frame::Object {
                state: FrameState::ObjectValue,
                ..
            } => {
                if let Some(Frame::Object {
                    state, pending_key, ..
                }) = frames.last_mut()
                {
                    *state = FrameState::ObjectCommaOrEnd;
                    pending_key.take();
                }
                Ok(())
            }
            _ => Err(JsonInputError::Syntax {
                position: self.position,
                message: "unexpected value".into(),
            }),
        }
    }

    fn value_path(frames: &[Frame]) -> Vec<PathComponent> {
        let Some(frame) = frames.last() else {
            return Vec::new();
        };
        match frame {
            Frame::Array {
                path, next_index, ..
            } => {
                let mut path = path.clone().unwrap_or_default();
                path.push(PathComponent::Index(*next_index));
                path
            }
            Frame::Object {
                path,
                pending_key: Some(key),
                ..
            } => {
                let mut path = path.clone().unwrap_or_default();
                path.push(PathComponent::Key(Arc::clone(key)));
                path
            }
            Frame::Object { path, .. } => path.clone().unwrap_or_default(),
        }
    }

    fn read_string(
        &mut self,
        checkpoint: &mut impl FnMut() -> io::Result<()>,
    ) -> Result<Arc<str>, JsonInputError> {
        let mut value = String::new();
        self.read_string_into(&mut value, checkpoint)?;
        Ok(value.into())
    }

    fn skip_string(
        &mut self,
        checkpoint: &mut impl FnMut() -> io::Result<()>,
    ) -> Result<(), JsonInputError> {
        let mut value = DiscardedString::default();
        self.read_string_into(&mut value, checkpoint)
    }

    fn read_string_into<S: DecodedStringSink>(
        &mut self,
        value: &mut S,
        checkpoint: &mut impl FnMut() -> io::Result<()>,
    ) -> Result<(), JsonInputError> {
        if self.consume_byte(checkpoint)? != b'"' {
            return self.syntax("expected string");
        }
        let mut pending = Vec::new();
        loop {
            Self::checkpoint(checkpoint)?;
            if pending.is_empty() {
                let position = self.position.offset;
                let span_result = {
                    let buffer = self.reader.fill_buf().map_err(JsonInputError::Io)?;
                    let span_len = buffer
                        .iter()
                        .take(ASCII_FAST_CHUNK)
                        .position(|&byte| !is_fast_ascii_string_byte(byte))
                        .unwrap_or(buffer.len().min(ASCII_FAST_CHUNK));
                    if span_len != 0 {
                        match value.append_ascii(
                            &buffer[..span_len],
                            self.options.maximum_token_bytes,
                            position,
                        ) {
                            Ok(()) => Ok(span_len),
                            Err(error) => Err((error, span_len)),
                        }
                    } else {
                        Ok(0)
                    }
                };
                match span_result {
                    Ok(span_len) if span_len != 0 => {
                        self.reader.consume(span_len);
                        self.position.offset = self.position.offset.saturating_add(span_len);
                        self.position.column = self.position.column.saturating_add(span_len);
                        continue;
                    }
                    Ok(_) => {}
                    Err((
                        JsonInputError::Limit {
                            limit: JsonLimit::TokenBytes,
                            position: violation,
                        },
                        span_len,
                    )) => {
                        let consumed = violation
                            .saturating_sub(position)
                            .saturating_add(1)
                            .min(span_len);
                        self.reader.consume(consumed);
                        self.position.offset = self.position.offset.saturating_add(consumed);
                        self.position.column = self.position.column.saturating_add(consumed);
                        return Err(JsonInputError::Limit {
                            limit: JsonLimit::TokenBytes,
                            position: self.position.offset,
                        });
                    }
                    Err((error, _)) => return Err(error),
                }
            }
            let byte = self
                .peek_byte()?
                .ok_or_else(|| self.syntax_error("unterminated string"))?;
            match byte {
                b'"' => {
                    self.consume_byte(checkpoint)?;
                    self.flush_utf8(value, &mut pending, true)?;
                    return Ok(());
                }
                b'\\' => {
                    self.consume_byte(checkpoint)?;
                    let escaped = self.consume_byte(checkpoint)?;
                    match escaped {
                        b'"' | b'\\' | b'/' => {
                            self.append_utf8_byte(value, &mut pending, escaped)?;
                        }
                        b'b' => self.append_utf8_byte(value, &mut pending, 0x08)?,
                        b'f' => self.append_utf8_byte(value, &mut pending, 0x0c)?,
                        b'n' => self.append_utf8_byte(value, &mut pending, b'\n')?,
                        b'r' => self.append_utf8_byte(value, &mut pending, b'\r')?,
                        b't' => self.append_utf8_byte(value, &mut pending, b'\t')?,
                        b'u' => {
                            self.flush_utf8(value, &mut pending, true)?;
                            self.read_unicode_escape(&mut pending, checkpoint)?;
                            self.flush_utf8(value, &mut pending, false)?;
                        }
                        _ => return self.syntax("invalid string escape"),
                    }
                }
                0..=0x1f => return self.syntax("unescaped control character in string"),
                _ => {
                    self.consume_byte(checkpoint)?;
                    self.append_utf8_byte(value, &mut pending, byte)?;
                }
            }
        }
    }

    fn append_utf8_byte<S: DecodedStringSink>(
        &self,
        value: &mut S,
        pending: &mut Vec<u8>,
        byte: u8,
    ) -> Result<(), JsonInputError> {
        if pending.is_empty() {
            match byte {
                0x00..=0x7f => value.append_decoded(
                    char::from(byte),
                    self.options.maximum_token_bytes,
                    self.position.offset,
                ),
                0xc2..=0xf4 => {
                    pending.push(byte);
                    Ok(())
                }
                _ => value.append_decoded(
                    '�',
                    self.options.maximum_token_bytes,
                    self.position.offset,
                ),
            }
        } else {
            let expected = utf8_width(pending[0]);
            if (0x80..=0xbf).contains(&byte) {
                pending.push(byte);
                if pending.len() == expected {
                    if let Ok(text) = std::str::from_utf8(pending) {
                        let character = text
                            .chars()
                            .next()
                            .expect("a complete UTF-8 sequence has one scalar");
                        value.append_decoded(
                            character,
                            self.options.maximum_token_bytes,
                            self.position.offset,
                        )?;
                        pending.clear();
                        Ok(())
                    } else {
                        pending.clear();
                        value.append_decoded(
                            '�',
                            self.options.maximum_token_bytes,
                            self.position.offset,
                        )
                    }
                } else {
                    Ok(())
                }
            } else {
                pending.clear();
                value.append_decoded(
                    '�',
                    self.options.maximum_token_bytes,
                    self.position.offset,
                )?;
                self.append_utf8_byte(value, pending, byte)
            }
        }
    }

    fn flush_utf8<S: DecodedStringSink>(
        &self,
        value: &mut S,
        pending: &mut Vec<u8>,
        final_chunk: bool,
    ) -> Result<(), JsonInputError> {
        while !pending.is_empty() {
            let expected = utf8_width(pending[0]);
            if pending.len() < expected {
                if final_chunk {
                    pending.clear();
                    value.append_decoded(
                        '�',
                        self.options.maximum_token_bytes,
                        self.position.offset,
                    )?;
                }
                return Ok(());
            }
            let sequence = &pending[..expected];
            match std::str::from_utf8(sequence) {
                Ok(text) => {
                    let character = text
                        .chars()
                        .next()
                        .expect("a complete UTF-8 sequence has one scalar");
                    value.append_decoded(
                        character,
                        self.options.maximum_token_bytes,
                        self.position.offset,
                    )?;
                }
                Err(_) => value.append_decoded(
                    '�',
                    self.options.maximum_token_bytes,
                    self.position.offset,
                )?,
            }
            pending.drain(..expected);
        }
        Ok(())
    }

    fn read_unicode_escape(
        &mut self,
        bytes: &mut Vec<u8>,
        checkpoint: &mut impl FnMut() -> io::Result<()>,
    ) -> Result<(), JsonInputError> {
        let first = self.read_hex_quad(checkpoint)?;
        match first {
            0xd800..=0xdbff => {
                if self.peek_byte()? != Some(b'\\') {
                    return self.syntax("unpaired high surrogate");
                }
                self.consume_byte(checkpoint)?;
                if self.consume_byte(checkpoint)? != b'u' {
                    return self.syntax("unpaired high surrogate");
                }
                let second = self.read_hex_quad(checkpoint)?;
                if !(0xdc00..=0xdfff).contains(&second) {
                    return self.syntax("invalid surrogate pair");
                }
                let codepoint =
                    0x1_0000 + (u32::from(first - 0xd800) << 10) + u32::from(second - 0xdc00);
                let character = char::from_u32(codepoint)
                    .ok_or_else(|| self.syntax_error("invalid Unicode scalar"))?;
                let mut encoded = [0; 4];
                bytes.extend_from_slice(character.encode_utf8(&mut encoded).as_bytes());
            }
            0xdc00..=0xdfff => bytes.extend_from_slice("�".as_bytes()),
            value => {
                let character = char::from_u32(u32::from(value))
                    .ok_or_else(|| self.syntax_error("invalid Unicode scalar"))?;
                let mut encoded = [0; 4];
                bytes.extend_from_slice(character.encode_utf8(&mut encoded).as_bytes());
            }
        }
        Ok(())
    }

    fn read_hex_quad(
        &mut self,
        checkpoint: &mut impl FnMut() -> io::Result<()>,
    ) -> Result<u16, JsonInputError> {
        let mut value = 0u16;
        for _ in 0..4 {
            let byte = self.consume_byte(checkpoint)?;
            let digit = match byte {
                b'0'..=b'9' => u16::from(byte - b'0'),
                b'a'..=b'f' => u16::from(byte - b'a' + 10),
                b'A'..=b'F' => u16::from(byte - b'A' + 10),
                _ => return self.syntax("invalid Unicode escape"),
            };
            value = (value << 4) | digit;
        }
        Ok(value)
    }

    fn read_token(
        &mut self,
        checkpoint: &mut impl FnMut() -> io::Result<()>,
    ) -> Result<String, JsonInputError> {
        let mut bytes = Vec::new();
        loop {
            Self::checkpoint(checkpoint)?;
            let Some(byte) = self.peek_byte()? else { break };
            if is_token_delimiter(byte) {
                break;
            }
            if bytes.len() >= self.options.maximum_token_bytes {
                return Err(JsonInputError::Limit {
                    limit: JsonLimit::TokenBytes,
                    position: self.position.offset,
                });
            }
            bytes.push(self.consume_byte(checkpoint)?);
        }
        String::from_utf8(bytes).map_err(|_| self.syntax_error("invalid UTF-8 token"))
    }

    fn read_word(
        &mut self,
        checkpoint: &mut impl FnMut() -> io::Result<()>,
    ) -> Result<String, JsonInputError> {
        const KEYWORD_MAX_BYTES: usize = 5;
        let mut prefix = [0; KEYWORD_MAX_BYTES];
        let mut prefix_len = 0;
        while prefix_len < KEYWORD_MAX_BYTES {
            Self::checkpoint(checkpoint)?;
            let Some(byte) = self.peek_byte()? else { break };
            if is_token_delimiter(byte) {
                break;
            }
            prefix[prefix_len] = self.consume_byte(checkpoint)?;
            prefix_len += 1;
        }
        let prefix_is_keyword = match &prefix[..prefix_len] {
            b"true" | b"null" | b"false" => self.peek_byte()?.is_none_or(is_token_delimiter),
            _ => false,
        };
        if prefix_is_keyword {
            return String::from_utf8(prefix[..prefix_len].to_vec())
                .map_err(|_| self.syntax_error("invalid UTF-8 literal"));
        }

        if prefix_len > self.options.maximum_token_bytes {
            return Err(JsonInputError::Limit {
                limit: JsonLimit::TokenBytes,
                position: self.position.offset,
            });
        }
        let mut bytes = Vec::with_capacity(prefix_len);
        bytes.extend_from_slice(&prefix[..prefix_len]);
        loop {
            Self::checkpoint(checkpoint)?;
            let Some(byte) = self.peek_byte()? else { break };
            if is_token_delimiter(byte) {
                break;
            }
            if bytes.len() >= self.options.maximum_token_bytes {
                return Err(JsonInputError::Limit {
                    limit: JsonLimit::TokenBytes,
                    position: self.position.offset,
                });
            }
            bytes.push(self.consume_byte(checkpoint)?);
        }
        String::from_utf8(bytes).map_err(|_| self.syntax_error("invalid UTF-8 literal"))
    }

    fn skip_whitespace(
        &mut self,
        checkpoint: &mut impl FnMut() -> io::Result<()>,
    ) -> Result<(), JsonInputError> {
        if !self.bom_checked {
            self.bom_checked = true;
            if self.peek_byte()? == Some(0xef) {
                let saved = self.position;
                let first = self.consume_byte(checkpoint)?;
                let second = self.consume_byte(checkpoint)?;
                let third = self.consume_byte(checkpoint)?;
                if [first, second, third] != [0xef, 0xbb, 0xbf] {
                    return Err(JsonInputError::Syntax {
                        position: saved,
                        message: "invalid UTF-8 byte-order mark".into(),
                    });
                }
            }
        }
        loop {
            Self::checkpoint(checkpoint)?;
            match self.peek_byte()? {
                Some(b' ' | b'\n' | b'\r' | b'\t') => {
                    self.consume_byte(checkpoint)?;
                }
                _ => return Ok(()),
            }
        }
    }

    fn peek_byte(&mut self) -> Result<Option<u8>, JsonInputError> {
        self.reader
            .fill_buf()
            .map(|buffer| buffer.first().copied())
            .map_err(JsonInputError::Io)
    }

    fn consume_byte(
        &mut self,
        checkpoint: &mut impl FnMut() -> io::Result<()>,
    ) -> Result<u8, JsonInputError> {
        Self::checkpoint(checkpoint)?;
        let byte = self
            .peek_byte()?
            .ok_or_else(|| self.syntax_error("unexpected end of input"))?;
        self.reader.consume(1);
        self.position.offset = self.position.offset.saturating_add(1);
        if byte == b'\n' {
            self.position.line = self.position.line.saturating_add(1);
            self.position.column = 1;
        } else {
            self.position.column = self.position.column.saturating_add(1);
        }
        Ok(byte)
    }

    fn checkpoint(checkpoint: &mut impl FnMut() -> io::Result<()>) -> Result<(), JsonInputError> {
        checkpoint().map_err(JsonInputError::Io)
    }

    fn syntax<T>(&self, message: &str) -> Result<T, JsonInputError> {
        Err(self.syntax_error(message))
    }

    fn syntax_error(&self, message: &str) -> JsonInputError {
        JsonInputError::Syntax {
            position: self.position,
            message: message.into(),
        }
    }
}

/// Parses exactly one jq-compatible JSON value from an in-memory string.
///
/// # Errors
///
/// Returns syntax, numeric-envelope, or resource-limit errors.
pub fn parse_json(input: &str, options: JsonInputOptions) -> Result<Value, JsonInputError> {
    parse_json_bytes(input.as_bytes(), options)
}

/// Parses exactly one jq-compatible JSON value from bytes.
///
/// # Errors
///
/// Returns syntax, numeric-envelope, or resource-limit errors.
pub fn parse_json_bytes(input: &[u8], options: JsonInputOptions) -> Result<Value, JsonInputError> {
    let mut reader = JsonInput::new(Cursor::new(input), options);
    let mut checkpoint = || Ok(());
    let value = reader
        .next_value(&mut checkpoint)?
        .ok_or_else(|| reader.syntax_error("expected JSON value"))?;
    reader.check_end(&mut checkpoint)?;
    Ok(value)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FrameState {
    ArrayValueOrEnd,
    ArrayValue,
    ArrayCommaOrEnd,
    ObjectKeyOrEnd,
    ObjectKey,
    ObjectColon,
    ObjectValue,
    ObjectCommaOrEnd,
}

enum Frame {
    Array {
        state: FrameState,
        path: Option<Vec<PathComponent>>,
        next_index: usize,
        materialize: bool,
        skip_position: Option<JsonPosition>,
    },
    Object {
        state: FrameState,
        path: Option<Vec<PathComponent>>,
        pending_key: Option<Arc<str>>,
        materialize: bool,
        skip_position: Option<JsonPosition>,
    },
}

trait DecodedStringSink {
    fn append_decoded(
        &mut self,
        character: char,
        maximum_token_bytes: usize,
        position: usize,
    ) -> Result<(), JsonInputError>;

    fn append_ascii(
        &mut self,
        bytes: &[u8],
        maximum_token_bytes: usize,
        position: usize,
    ) -> Result<(), JsonInputError>;
}

impl DecodedStringSink for String {
    fn append_decoded(
        &mut self,
        character: char,
        maximum_token_bytes: usize,
        position: usize,
    ) -> Result<(), JsonInputError> {
        if self.len().saturating_add(character.len_utf8()) > maximum_token_bytes {
            return Err(JsonInputError::Limit {
                limit: JsonLimit::TokenBytes,
                position,
            });
        }
        self.push(character);
        Ok(())
    }

    fn append_ascii(
        &mut self,
        bytes: &[u8],
        maximum_token_bytes: usize,
        position: usize,
    ) -> Result<(), JsonInputError> {
        let available = maximum_token_bytes.saturating_sub(self.len());
        if bytes.len() > available {
            return Err(JsonInputError::Limit {
                limit: JsonLimit::TokenBytes,
                position: position.saturating_add(available),
            });
        }
        self.push_str(std::str::from_utf8(bytes).expect("ASCII fast-path bytes are valid UTF-8"));
        Ok(())
    }
}

#[derive(Default)]
struct DiscardedString {
    decoded_bytes: usize,
}

impl DecodedStringSink for DiscardedString {
    fn append_decoded(
        &mut self,
        character: char,
        maximum_token_bytes: usize,
        position: usize,
    ) -> Result<(), JsonInputError> {
        self.decoded_bytes =
            self.decoded_bytes
                .checked_add(character.len_utf8())
                .ok_or(JsonInputError::Limit {
                    limit: JsonLimit::TokenBytes,
                    position,
                })?;
        if self.decoded_bytes > maximum_token_bytes {
            return Err(JsonInputError::Limit {
                limit: JsonLimit::TokenBytes,
                position,
            });
        }
        Ok(())
    }

    fn append_ascii(
        &mut self,
        bytes: &[u8],
        maximum_token_bytes: usize,
        position: usize,
    ) -> Result<(), JsonInputError> {
        let available = maximum_token_bytes.saturating_sub(self.decoded_bytes);
        if bytes.len() > available {
            return Err(JsonInputError::Limit {
                limit: JsonLimit::TokenBytes,
                position: position.saturating_add(available),
            });
        }
        self.decoded_bytes =
            self.decoded_bytes
                .checked_add(bytes.len())
                .ok_or(JsonInputError::Limit {
                    limit: JsonLimit::TokenBytes,
                    position,
                })?;
        Ok(())
    }
}

impl Frame {
    const fn state(&self) -> FrameState {
        match self {
            Self::Array { state, .. } | Self::Object { state, .. } => *state,
        }
    }

    const fn materialize(&self) -> bool {
        match self {
            Self::Array { materialize, .. } | Self::Object { materialize, .. } => *materialize,
        }
    }

    const fn skip_position(&self) -> Option<JsonPosition> {
        match self {
            Self::Array { skip_position, .. } | Self::Object { skip_position, .. } => {
                *skip_position
            }
        }
    }
}

#[derive(Default)]
struct ValueBuilder {
    frames: Vec<BuilderFrame>,
    root: Option<Value>,
}

enum BuilderFrame {
    Array(Vec<Value>),
    Object {
        values: Object,
        pending_key: Option<Arc<str>>,
    },
}

impl ValueBuilder {
    fn accept(&mut self, event: JsonEvent) -> Result<(), io::Error> {
        match event {
            JsonEvent::StartArray(_) => self.frames.push(BuilderFrame::Array(Vec::new())),
            JsonEvent::StartObject(_) => self.frames.push(BuilderFrame::Object {
                values: Object::new(),
                pending_key: None,
            }),
            JsonEvent::Key { value, .. } => match self.frames.last_mut() {
                Some(BuilderFrame::Object { pending_key, .. }) => *pending_key = Some(value),
                _ => return Err(io::Error::other("JSON key outside object")),
            },
            JsonEvent::Scalar { value, .. } => self.push_value(value)?,
            JsonEvent::Skipped { .. } => {
                return Err(io::Error::other("skipped event cannot build a value"));
            }
            JsonEvent::EndArray(_) | JsonEvent::EndObject(_) => {
                let frame = self
                    .frames
                    .pop()
                    .ok_or_else(|| io::Error::other("JSON container underflow"))?;
                let value = match frame {
                    BuilderFrame::Array(values) => Value::array(values),
                    BuilderFrame::Object { values, .. } => Value::object(values),
                };
                self.push_value(value)?;
            }
        }
        Ok(())
    }

    fn push_value(&mut self, value: Value) -> Result<(), io::Error> {
        match self.frames.last_mut() {
            Some(BuilderFrame::Array(values)) => values.push(value),
            Some(BuilderFrame::Object {
                values,
                pending_key,
            }) => {
                let key = pending_key
                    .take()
                    .ok_or_else(|| io::Error::other("JSON value without object key"))?;
                values.insert(key, value);
            }
            None => {
                if self.root.replace(value).is_some() {
                    return Err(io::Error::other("multiple JSON roots"));
                }
            }
        }
        Ok(())
    }

    fn finish(self) -> Result<Value, JsonInputError> {
        self.root.ok_or_else(|| JsonInputError::Syntax {
            position: JsonPosition::default(),
            message: "missing JSON root".into(),
        })
    }
}

fn is_token_delimiter(byte: u8) -> bool {
    matches!(
        byte,
        b' ' | b'\n' | b'\r' | b'\t' | b'"' | b'[' | b']' | b'{' | b'}' | b',' | b':'
    )
}

fn is_fast_ascii_string_byte(byte: u8) -> bool {
    byte.is_ascii() && byte >= 0x20 && byte != b'"' && byte != b'\\'
}

fn utf8_width(first: u8) -> usize {
    match first {
        0xc2..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf4 => 4,
        _ => 1,
    }
}

enum ScalarError {
    Invalid(&'static str),
    Limit(&'static str),
}

fn parse_scalar_token(token: &str) -> Result<Value, ScalarError> {
    match token {
        "null" => Ok(Value::Null),
        "true" => Ok(Value::Bool(true)),
        "false" => Ok(Value::Bool(false)),
        _ => parse_jq_number(token)
            .map(Value::Number)
            .map_err(|error| number_error(&error)),
    }
}

/// Parses one jq numeric token without accepting surrounding whitespace or
/// containers. This is shared by JSON input and the `tonumber` builtin.
///
/// # Errors
///
/// Returns `NumberError::Invalid` for unsupported numeric syntax and the
/// corresponding numeric-envelope error when the token exceeds configured
/// numeric limits.
pub fn parse_jq_number(token: &str) -> Result<Number, NumberError> {
    if let Some(value) = jq_nonfinite(token) {
        return Ok(Number::from_runtime_f64(value));
    }
    match Number::parse(token) {
        Ok(number) => Ok(number),
        Err(NumberError::Invalid) => {
            let normalized = normalize_finite_token(token).ok_or(NumberError::Invalid)?;
            Number::parse(&normalized)
        }
        Err(error) => Err(error),
    }
}

fn validate_jq_number(token: &str) -> Result<(), NumberError> {
    if jq_nonfinite(token).is_some() {
        return Ok(());
    }
    match Number::validate_literal(token) {
        Ok(()) => Ok(()),
        Err(NumberError::Invalid) => {
            let normalized = normalize_finite_token(token).ok_or(NumberError::Invalid)?;
            Number::validate_literal(&normalized)
        }
        Err(error) => Err(error),
    }
}

fn jq_nonfinite(token: &str) -> Option<f64> {
    if ["nan", "+nan", "-nan", "snan", "+snan", "-snan"]
        .iter()
        .any(|candidate| token.eq_ignore_ascii_case(candidate))
    {
        return Some(f64::NAN);
    }
    if ["inf", "+inf", "infinity", "+infinity"]
        .iter()
        .any(|candidate| token.eq_ignore_ascii_case(candidate))
    {
        return Some(f64::INFINITY);
    }
    if ["-inf", "-infinity"]
        .iter()
        .any(|candidate| token.eq_ignore_ascii_case(candidate))
    {
        return Some(f64::NEG_INFINITY);
    }
    None
}

fn number_error(error: &NumberError) -> ScalarError {
    match error {
        NumberError::Invalid => ScalarError::Invalid("invalid numeric literal"),
        NumberError::CoefficientDigits { .. }
        | NumberError::Exponent { .. }
        | NumberError::RenderedBytes { .. } => {
            ScalarError::Limit("numeric token exceeds input limit")
        }
        NumberError::NonFinite | NumberError::DivisionByZero => {
            ScalarError::Invalid("invalid numeric literal")
        }
    }
}

fn normalize_finite_token(token: &str) -> Option<String> {
    let (negative, unsigned) = match token.as_bytes().first().copied() {
        Some(b'-') => (true, &token[1..]),
        Some(b'+') => (false, &token[1..]),
        _ => (false, token),
    };
    if unsigned.is_empty() {
        return None;
    }
    let (mantissa, exponent) = unsigned
        .split_once(['e', 'E'])
        .map_or((unsigned, None), |(mantissa, exponent)| {
            (mantissa, Some(exponent))
        });
    if let Some(exponent) = exponent {
        let exponent_digits = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
        if exponent_digits.is_empty() || !exponent_digits.bytes().all(|byte| byte.is_ascii_digit())
        {
            return None;
        }
    }
    let (integer, fraction, had_dot) = match mantissa.split_once('.') {
        Some((integer, fraction)) => (integer, fraction, true),
        None => (mantissa, "", false),
    };
    if integer.is_empty() && fraction.is_empty()
        || !integer.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let has_digit = !integer.is_empty() || !fraction.is_empty();
    if !has_digit {
        return None;
    }
    let trimmed_integer = integer.trim_start_matches('0');
    let normalized_integer = if trimmed_integer.is_empty() {
        "0"
    } else {
        trimmed_integer
    };
    let mut normalized = String::new();
    if negative {
        normalized.push('-');
    }
    normalized.push_str(normalized_integer);
    if had_dot && !fraction.is_empty() {
        normalized.push('.');
        normalized.push_str(fraction);
    }
    if let Some(exponent) = exponent {
        normalized.push('e');
        normalized.push_str(exponent);
    }
    Some(normalized)
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, io, io::Cursor, io::Read};

    use super::{
        JsonEvent, JsonInput, JsonInputError, JsonInputOptions, JsonLimit, JsonPosition,
        parse_json, parse_json_bytes,
    };
    use crate::{PathComponent, Value, ValueKind};

    struct ChunkedRead {
        bytes: Vec<u8>,
        offset: usize,
        chunk_size: usize,
    }

    impl Read for ChunkedRead {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if self.offset == self.bytes.len() {
                return Ok(0);
            }
            let count = self
                .chunk_size
                .min(buffer.len())
                .min(self.bytes.len() - self.offset);
            buffer[..count].copy_from_slice(&self.bytes[self.offset..self.offset + count]);
            self.offset += count;
            Ok(count)
        }
    }

    #[test]
    fn parses_jq_nonfinite_and_relaxed_numbers() {
        let mut reader = JsonInput::new(
            Cursor::new(b"NaN Infinity -Infinity +.1 01 1."),
            JsonInputOptions::default(),
        );
        let mut checkpoint = || Ok(());
        let mut values = Vec::new();
        while let Some(value) = reader.next_value(&mut checkpoint).unwrap() {
            values.push(value);
        }
        assert_eq!(values.len(), 6);
        assert!(matches!(values[0], Value::Number(ref number) if number.as_f64().is_nan()));
        assert!(
            matches!(values[1], Value::Number(ref number) if number.as_f64().is_infinite() && number.as_f64().is_sign_positive())
        );
        assert!(
            matches!(values[2], Value::Number(ref number) if number.as_f64().is_infinite() && number.as_f64().is_sign_negative())
        );
        assert_eq!(values[3].to_string(), "0.1");
        assert_eq!(values[4].to_string(), "1");
        assert_eq!(values[5].to_string(), "1");
    }

    #[test]
    fn rejects_invalid_suffix_but_allows_adjacent_self_delimiting_values() {
        assert!(parse_json("NaNtrue", JsonInputOptions::default()).is_err());
        assert!(parse_json("1true", JsonInputOptions::default()).is_err());
        let mut reader = JsonInput::new(Cursor::new(b"NaN{}NaN"), JsonInputOptions::default());
        let mut checkpoint = || Ok(());
        assert!(matches!(
            reader.next_value(&mut checkpoint).unwrap(),
            Some(Value::Number(_))
        ));
        assert!(matches!(
            reader.next_value(&mut checkpoint).unwrap(),
            Some(Value::Object(_))
        ));
        assert!(matches!(
            reader.next_value(&mut checkpoint).unwrap(),
            Some(Value::Number(_))
        ));
        assert_eq!(reader.next_value(&mut checkpoint).unwrap(), None);
    }

    #[test]
    fn parses_multiple_and_nested_container_values() {
        assert_eq!(
            parse_json("[1,2,[3],{\"x\":[true,null]}]", JsonInputOptions::default())
                .unwrap()
                .to_string(),
            "[1,2,[3],{\"x\":[true,null]}]"
        );
        assert!(parse_json("[1,]", JsonInputOptions::default()).is_err());
        assert!(parse_json("{\"x\":1,}", JsonInputOptions::default()).is_err());
    }

    #[test]
    fn emits_incremental_events_and_preserves_duplicate_keys() {
        let mut reader = JsonInput::new(
            Cursor::new(br#"{"a":1,"a":2}"#),
            JsonInputOptions::default(),
        );
        let mut events = Vec::new();
        let mut checkpoint = || Ok(());
        assert!(
            reader
                .next_events(
                    &mut |event| {
                        events.push(event);
                        Ok(())
                    },
                    &mut checkpoint
                )
                .unwrap()
        );
        assert!(matches!(
            events.first(),
            Some(JsonEvent::StartObject(JsonPosition { offset: 0, .. }))
        ));
        assert!(matches!(
            events.last(),
            Some(JsonEvent::EndObject(JsonPosition { offset: 13, .. }))
        ));
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, JsonEvent::Key { .. }))
                .count(),
            2
        );
        let value = parse_json(r#"{"a":1,"a":2}"#, JsonInputOptions::default()).unwrap();
        assert!(matches!(value, Value::Object(_)));
        assert_eq!(value.to_string(), r#"{"a":2}"#);
    }

    #[test]
    fn holds_scalars_until_container_syntax_is_confirmed() {
        fn scalar_count(input: &[u8]) -> (usize, Result<bool, super::JsonInputError>) {
            let mut reader = JsonInput::new(Cursor::new(input), JsonInputOptions::default());
            let mut events = Vec::new();
            let mut checkpoint = || Ok(());
            let result = reader.next_events(
                &mut |event| {
                    events.push(event);
                    Ok(())
                },
                &mut checkpoint,
            );
            let scalars = events
                .iter()
                .filter(|event| matches!(event, JsonEvent::Scalar { .. }))
                .count();
            (scalars, result)
        }

        let (scalars, result) = scalar_count(b"[1 2]");
        assert_eq!(scalars, 0);
        assert!(result.is_err());

        let (scalars, result) = scalar_count(b"[1,");
        assert_eq!(scalars, 1);
        assert!(result.is_err());

        let (scalars, result) = scalar_count(b"[\"a\"");
        assert_eq!(scalars, 0);
        assert!(result.is_err());

        let (scalars, result) = scalar_count(b"{\"a\":1 \"b\":2}");
        assert_eq!(scalars, 0);
        assert!(result.is_err());

        let (scalars, result) = scalar_count(b"[{} 2]");
        assert_eq!(scalars, 0);
        assert!(result.is_err());
    }

    #[test]
    fn exact_one_check_peeks_without_pulling_a_later_root() {
        let mut reader =
            JsonInput::new(Cursor::new(b"1 [unterminated"), JsonInputOptions::default());
        let mut checkpoint = || Ok(());
        assert_eq!(
            reader.next_value(&mut checkpoint).unwrap().unwrap().kind(),
            ValueKind::Number
        );
        assert!(matches!(
            reader.check_end(&mut checkpoint),
            Err(super::JsonInputError::Syntax { position, .. })
                if position.offset == 2 && position.line == 1 && position.column == 3
        ));
    }

    #[test]
    fn string_surrogates_and_bom_follow_jq_boundary() {
        assert_eq!(
            parse_json("\u{feff}\"\\udc00\"", JsonInputOptions::default())
                .unwrap()
                .to_string(),
            "\"�\""
        );
        assert!(parse_json(r#""\ud800""#, JsonInputOptions::default()).is_err());
        for (index, (bytes, expected)) in [
            (b"\"\xff\"".as_slice(), "\"�\""),
            (b"\"\xc0\xaf\"".as_slice(), "\"��\""),
            (b"\"\xed\xa0\x80\"".as_slice(), "\"�\""),
            (b"\"\xe2\x82\"".as_slice(), "\"�\""),
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(
                parse_json_bytes(bytes, JsonInputOptions::default())
                    .unwrap()
                    .to_string(),
                expected,
                "malformed UTF-8 case {index}"
            );
        }
        assert_eq!(
            parse_json_bytes(b"\"\xe2\\u0061b\"", JsonInputOptions::default())
                .unwrap()
                .to_string(),
            "\"�ab\""
        );
    }

    #[test]
    fn limits_and_checkpoint_stop_before_later_root() {
        let options = JsonInputOptions {
            maximum_depth: 1,
            maximum_token_bytes: 2,
        };
        assert!(parse_json("[1]", options).is_ok());
        assert!(matches!(
            parse_json("[[1]]", options),
            Err(super::JsonInputError::Limit {
                limit: JsonLimit::Depth,
                ..
            })
        ));
        assert!(matches!(
            parse_json("123", options),
            Err(super::JsonInputError::Limit {
                limit: JsonLimit::TokenBytes,
                ..
            })
        ));
        let keyword_options = JsonInputOptions {
            maximum_depth: 1,
            maximum_token_bytes: 0,
        };
        assert!(parse_json("true", keyword_options).is_ok());
        assert!(parse_json("null", keyword_options).is_ok());
        assert!(matches!(
            parse_json("nan", keyword_options),
            Err(super::JsonInputError::Limit {
                limit: JsonLimit::TokenBytes,
                ..
            })
        ));
        assert!(matches!(
            parse_json("truefalse", JsonInputOptions::default()),
            Err(super::JsonInputError::Syntax { position, .. })
                if position.offset == 9 && position.column == 10
        ));
        assert!(matches!(
            parse_json(
                "truefalse",
                JsonInputOptions {
                    maximum_depth: 1,
                    maximum_token_bytes: 4,
                }
            ),
            Err(super::JsonInputError::Limit {
                limit: JsonLimit::TokenBytes,
                position: 5,
            })
        ));
        let mut reader = JsonInput::new(Cursor::new(b"1 NaNtrue"), JsonInputOptions::default());
        let calls = Cell::new(0);
        let mut checkpoint = || {
            calls.set(calls.get() + 1);
            Ok(())
        };
        assert_eq!(
            reader.next_value(&mut checkpoint).unwrap().unwrap().kind(),
            ValueKind::Number
        );
        assert!(calls.get() > 0);
        assert!(reader.next_value(&mut checkpoint).is_err());

        let mut reader = JsonInput::new(Cursor::new(b"1\nNaNtrue"), JsonInputOptions::default());
        let mut checkpoint = || Ok(());
        assert_eq!(
            reader.next_value(&mut checkpoint).unwrap().unwrap().kind(),
            ValueKind::Number
        );
        assert!(matches!(
            reader.next_value(&mut checkpoint),
            Err(super::JsonInputError::Syntax { position, .. })
                if position.offset == 9 && position.line == 2 && position.column == 8
        ));
    }

    #[test]
    fn preserves_checkpoint_reader_and_position_errors() {
        struct ReadError;
        impl Read for ReadError {
            fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::new(io::ErrorKind::BrokenPipe, "source failed"))
            }
        }

        let mut reader = JsonInput::new(Cursor::new(b"1"), JsonInputOptions::default());
        let mut checkpoint = || Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
        assert!(matches!(
            reader.next_value(&mut checkpoint),
            Err(super::JsonInputError::Io(error))
                if error.kind() == io::ErrorKind::Interrupted && error.to_string() == "cancelled"
        ));

        let mut reader = JsonInput::new(ReadError, JsonInputOptions::default());
        let mut checkpoint = || Ok(());
        assert!(matches!(
            reader.next_value(&mut checkpoint),
            Err(super::JsonInputError::Io(error))
                if error.kind() == io::ErrorKind::BrokenPipe && error.to_string() == "source failed"
        ));
    }

    #[test]
    fn preserves_errors_inside_unicode_escape() {
        struct EscapeReadError {
            delivered: bool,
        }
        impl Read for EscapeReadError {
            fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
                if self.delivered {
                    return Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "escape source failed",
                    ));
                }
                self.delivered = true;
                buffer[..3].copy_from_slice(b"\"\\u");
                Ok(3)
            }
        }

        let mut reader = JsonInput::new(Cursor::new(b"\"\\u"), JsonInputOptions::default());
        let calls = Cell::new(0);
        let mut checkpoint = || {
            let call = calls.get();
            calls.set(call + 1);
            if call >= 7 {
                Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "escape cancelled",
                ))
            } else {
                Ok(())
            }
        };
        assert!(matches!(
            reader.next_value(&mut checkpoint),
            Err(super::JsonInputError::Io(error))
                if error.kind() == io::ErrorKind::Interrupted
                    && error.to_string() == "escape cancelled"
        ));
        assert!(calls.get() >= 8);

        let mut reader = JsonInput::new(
            EscapeReadError { delivered: false },
            JsonInputOptions::default(),
        );
        let mut checkpoint = || Ok(());
        assert!(matches!(
            reader.next_value(&mut checkpoint),
            Err(super::JsonInputError::Io(error))
                if error.kind() == io::ErrorKind::BrokenPipe
                    && error.to_string() == "escape source failed"
        ));
    }

    #[test]
    fn long_ascii_strings_match_across_short_reader_boundaries() {
        let mut input = Vec::with_capacity(32_770);
        input.push(b'"');
        input.extend(std::iter::repeat_n(b'a', 32_768));
        input.push(b'"');
        let expected = parse_json_bytes(&input, JsonInputOptions::default()).unwrap();

        for chunk_size in [1, 2, 3, 7, 31] {
            let mut reader = JsonInput::new(
                ChunkedRead {
                    bytes: input.clone(),
                    offset: 0,
                    chunk_size,
                },
                JsonInputOptions::default(),
            );
            let mut checkpoint = || Ok(());
            assert_eq!(
                reader.next_value(&mut checkpoint).unwrap(),
                Some(expected.clone())
            );
            assert_eq!(reader.next_value(&mut checkpoint).unwrap(), None);
        }
    }

    #[test]
    fn ascii_fast_path_preserves_escape_and_utf8_boundaries() {
        let input = b"\"prefix\\\\quote\\\"slash\\/tab\\tline\\nutf8\xc3\xa9tail\"";
        let expected = parse_json_bytes(input, JsonInputOptions::default()).unwrap();
        assert_eq!(
            expected,
            Value::string("prefix\\quote\"slash/tab\tline\nutf8étail")
        );

        for chunk_size in [1, 2, 4, 5] {
            let mut reader = JsonInput::new(
                ChunkedRead {
                    bytes: input.to_vec(),
                    offset: 0,
                    chunk_size,
                },
                JsonInputOptions::default(),
            );
            let mut checkpoint = || Ok(());
            assert_eq!(
                reader.next_value(&mut checkpoint).unwrap(),
                Some(expected.clone())
            );
        }
    }

    #[test]
    fn discarded_ascii_strings_validate_limits_without_retaining_text() {
        let mut input = br#"{"keep":1,"drop":""#.to_vec();
        input.extend(std::iter::repeat_n(b'x', 128));
        input.extend_from_slice(br#""}"#);
        let mut reader = JsonInput::new(
            ChunkedRead {
                bytes: input,
                offset: 0,
                chunk_size: 2,
            },
            JsonInputOptions::default(),
        );
        let mut events = Vec::new();
        let mut checkpoint = || Ok(());
        let mut retain =
            |path: &[PathComponent]| path.is_empty() || path == [PathComponent::Key("keep".into())];
        assert!(
            reader
                .next_events_selected(
                    &mut |event| {
                        events.push(event);
                        Ok(())
                    },
                    &mut checkpoint,
                    &mut retain,
                )
                .unwrap()
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, JsonEvent::Scalar { .. }))
                .count(),
            1
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, JsonEvent::Skipped { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn ascii_limit_reports_first_unadmitted_byte_for_retained_and_discarded_strings() {
        let mut input = vec![b'"'];
        input.extend(std::iter::repeat_n(b'x', 9));
        input.push(b'"');
        let options = JsonInputOptions {
            maximum_depth: 256,
            maximum_token_bytes: 8,
        };
        assert!(matches!(
            parse_json_bytes(&input, options),
            Err(JsonInputError::Limit {
                limit: JsonLimit::TokenBytes,
                position: 10,
            })
        ));

        let mut reader = JsonInput::new(Cursor::new(br#"{"drop":"xxxxxxxxx"}"#), options);
        let mut checkpoint = || Ok(());
        let mut retain =
            |path: &[PathComponent]| path.is_empty() || path == [PathComponent::Key("keep".into())];
        assert!(matches!(
            reader.next_events_selected(&mut |_| Ok(()), &mut checkpoint, &mut retain),
            Err(JsonInputError::Limit {
                limit: JsonLimit::TokenBytes,
                position: 18,
            })
        ));
    }

    #[test]
    fn long_ascii_scan_checkpoints_and_cancels_between_bounded_chunks() {
        let mut input = Vec::with_capacity(32_770);
        input.push(b'"');
        input.extend(std::iter::repeat_n(b'a', 32_768));
        input.push(b'"');
        let calls = Cell::new(0);
        let mut reader = JsonInput::new(Cursor::new(input), JsonInputOptions::default());
        let mut checkpoint = || {
            calls.set(calls.get() + 1);
            if calls.get() >= 8 {
                Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "ASCII scan cancelled",
                ))
            } else {
                Ok(())
            }
        };
        assert!(matches!(
            reader.next_value(&mut checkpoint),
            Err(JsonInputError::Io(error))
                if error.kind() == io::ErrorKind::Interrupted
                    && error.to_string() == "ASCII scan cancelled"
        ));
        assert!(
            calls.get() < 100,
            "ASCII scan used {} checkpoints",
            calls.get()
        );
        assert!(reader.position().offset > super::ASCII_FAST_CHUNK);
    }

    #[test]
    fn ascii_scan_reports_control_character_at_its_source_position() {
        assert!(matches!(
            parse_json_bytes(b"\"abc\n\"", JsonInputOptions::default()),
            Err(JsonInputError::Syntax { position, .. })
                if position.offset == 4 && position.line == 1 && position.column == 5
        ));
    }
}
