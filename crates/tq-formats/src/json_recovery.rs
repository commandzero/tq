//! Incremental JSON payload parsing with token-local recovery.
//!
//! A failed token consumes its terminating delimiter, as jq 1.8 does. Resetting
//! clears the unfinished value, not the unread remainder of the RS segment.

use std::{collections::VecDeque, io::Read, sync::Arc};

use tq_core::{Object, SourceId, Span, Value};
use tq_toon::{Event, Scalar};

use crate::{FormatError, InputFormat};

enum Container {
    Array { values: Vec<Value>, count: u64 },
    Object { values: Object, count: u64 },
    Key(Arc<str>),
}

#[derive(Default)]
enum LexicalState {
    #[default]
    Normal,
    String,
    Escape,
}

pub(crate) struct JsonRecoveryDecoder<R> {
    reader: R,
    containers: Vec<Container>,
    pending: Option<Value>,
    token: Vec<u8>,
    lexical: LexicalState,
    maximum_depth: usize,
    maximum_token_bytes: usize,
    ended: bool,
    retain_documents: bool,
    events: VecDeque<Event>,
    document_started: bool,
    line: u64,
    document_line: u64,
}

impl<R: Read> JsonRecoveryDecoder<R> {
    pub(crate) fn new_with_line(
        reader: R,
        maximum_depth: usize,
        maximum_token_bytes: usize,
        line: u64,
    ) -> Self {
        Self {
            reader,
            containers: Vec::new(),
            pending: None,
            token: Vec::new(),
            lexical: LexicalState::Normal,
            maximum_depth,
            maximum_token_bytes,
            ended: false,
            retain_documents: true,
            events: VecDeque::new(),
            document_started: false,
            line,
            document_line: line,
        }
    }

    pub(crate) fn next_value(&mut self) -> Result<Option<Value>, FormatError> {
        let result = self.advance();
        if result.is_err() {
            self.containers.clear();
            self.pending = None;
            self.token.clear();
            self.lexical = LexicalState::Normal;
            self.events.clear();
            self.document_started = false;
        }
        result
    }

    pub(crate) fn events_only(mut self) -> Self {
        self.retain_documents = false;
        self
    }

    pub(crate) fn next_event(&mut self) -> Result<Option<Event>, FormatError> {
        loop {
            if let Some(event) = self.events.pop_front() {
                return Ok(Some(event));
            }
            if self.ended {
                return Ok(None);
            }
            self.next_value()?;
        }
    }

    fn emit(&mut self, event: Event) {
        if self.retain_documents {
            return;
        }
        if !self.document_started {
            self.events.push_back(Event::DocumentStart { span: span() });
            self.document_started = true;
        }
        self.events.push_back(event);
    }

    fn emit_scalar(&mut self, value: &Value) {
        if self.retain_documents {
            return;
        }
        let scalar = match value {
            Value::Null => Scalar::Null,
            Value::Bool(value) => Scalar::Bool(*value),
            Value::Number(value) => Scalar::Number(value.clone()),
            Value::String(value) => Scalar::String(Arc::clone(value)),
            Value::Array(_) | Value::Object(_) => return,
        };
        self.emit(Event::Scalar {
            span: span(),
            value: scalar,
        });
    }

    fn complete_document(&mut self, value: &Value) {
        if self.retain_documents {
            return;
        }
        self.emit_scalar(value);
        self.events.push_back(Event::DocumentEnd { span: span() });
        self.document_started = false;
    }

    pub(crate) const fn ended(&self) -> bool {
        self.ended
    }

    pub(crate) const fn document_line(&self) -> u64 {
        self.document_line
    }

    fn complete_document_at(&mut self, value: &Value, line: u64) {
        self.document_line = line;
        self.complete_document(value);
    }

    fn advance(&mut self) -> Result<Option<Value>, FormatError> {
        while !self.ended {
            let mut byte = [0];
            if self.reader.read(&mut byte)? == 0 {
                self.ended = true;
                if !matches!(self.lexical, LexicalState::Normal) {
                    return Err(parse_error("Unfinished string at the end of the input"));
                }
                self.finish_token()?;
                if !self.containers.is_empty() {
                    return Err(parse_error("Unfinished JSON term at the end of the input"));
                }
                if matches!(self.pending, Some(Value::Number(_))) {
                    return Err(parse_error("Potentially truncated top-level numeric value"));
                }
                let completed = self.pending.take();
                if let Some(value) = &completed {
                    self.complete_document_at(value, self.line);
                }
                return Ok(completed);
            }
            let byte = byte[0];
            let byte_line = self.line;
            if byte == b'\n' {
                self.line = self.line.saturating_add(1);
            }
            match self.lexical {
                LexicalState::Escape => {
                    self.append(byte)?;
                    self.lexical = LexicalState::String;
                }
                LexicalState::String => {
                    self.append(byte)?;
                    if byte == b'"' {
                        self.finish_token()?;
                        self.lexical = LexicalState::Normal;
                        if self.containers.is_empty() {
                            let completed = self.pending.take();
                            if let Some(value) = &completed {
                                self.complete_document_at(value, byte_line);
                            }
                            return Ok(completed);
                        }
                    } else if byte == b'\\' {
                        self.lexical = LexicalState::Escape;
                    }
                }
                LexicalState::Normal => {
                    if !is_delimiter(byte) {
                        self.append(byte)?;
                        continue;
                    }
                    self.finish_token()?;
                    let completed = if self.containers.is_empty() {
                        self.pending.take()
                    } else {
                        None
                    };
                    if let Some(value) = &completed {
                        self.complete_document_at(value, byte_line);
                    }
                    if byte == b'"' {
                        self.append(byte)?;
                        self.lexical = LexicalState::String;
                    } else {
                        self.structure(byte)?;
                    }
                    if completed.is_some() {
                        return Ok(completed);
                    }
                    if self.containers.is_empty() && self.pending.is_some() {
                        let completed = self.pending.take();
                        if let Some(value) = &completed {
                            self.complete_document_at(value, byte_line);
                        }
                        return Ok(completed);
                    }
                }
            }
            if !self.retain_documents && !self.events.is_empty() {
                return Ok(None);
            }
        }
        Ok(None)
    }

    fn append(&mut self, byte: u8) -> Result<(), FormatError> {
        // Every decoded UTF-8 byte needs at most six source bytes in a JSON
        // escape. Bound source storage too, then check decoded content below.
        let limit = if self.token.first() == Some(&b'"') || self.token.is_empty() && byte == b'"' {
            self.maximum_token_bytes.saturating_mul(6).saturating_add(2)
        } else {
            self.maximum_token_bytes.max(5)
        };
        if self.token.len() >= limit {
            return Err(FormatError::Resource("token-bytes"));
        }
        self.token.push(byte);
        Ok(())
    }

    fn finish_token(&mut self) -> Result<(), FormatError> {
        if self.token.is_empty() {
            return Ok(());
        }
        if self.pending.is_some() {
            return Err(parse_error("Expected separator between values"));
        }
        let value: Value = serde_json::from_slice(&self.token).map_err(|error| {
            if self.token.first() == Some(&b'"') {
                parse_error(&error.to_string())
            } else {
                parse_error("Invalid numeric literal")
            }
        })?;
        if matches!(&value, Value::String(text) if text.len() > self.maximum_token_bytes) {
            return Err(FormatError::Resource("token-bytes"));
        }
        if matches!(&value, Value::Number(_)) && self.token.len() > self.maximum_token_bytes {
            return Err(FormatError::Resource("token-bytes"));
        }
        self.token.clear();
        self.pending = Some(value);
        Ok(())
    }

    fn open_container(&mut self, byte: u8) -> Result<(), FormatError> {
        if self.pending.is_some() {
            return Err(parse_error("Expected separator between values"));
        }
        // Object keys occupy stack entries but do not add nesting depth.
        let depth = self
            .containers
            .iter()
            .filter(|entry| !matches!(entry, Container::Key(_)))
            .count();
        if depth >= self.maximum_depth {
            return Err(FormatError::Resource("depth"));
        }
        self.containers.push(if byte == b'[' {
            Container::Array {
                values: Vec::new(),
                count: 0,
            }
        } else {
            Container::Object {
                values: Object::new(),
                count: 0,
            }
        });
        self.emit(if byte == b'[' {
            Event::ArrayStart {
                span: span(),
                declared_count: None,
            }
        } else {
            Event::ObjectStart { span: span() }
        });
        Ok(())
    }

    fn structure(&mut self, byte: u8) -> Result<(), FormatError> {
        match byte {
            b'[' | b'{' => self.open_container(byte)?,
            b':' => {
                let key = self
                    .pending
                    .take()
                    .ok_or_else(|| parse_error("Expected string key before ':'"))?;
                if !matches!(self.containers.last(), Some(Container::Object { .. })) {
                    return Err(parse_error("':' not as part of an object"));
                }
                let Value::String(key) = key else {
                    return Err(parse_error("Object keys must be strings"));
                };
                self.emit(Event::Key {
                    span: span(),
                    value: Arc::clone(&key),
                    quoted: true,
                });
                self.containers.push(Container::Key(key));
            }
            b',' => {
                let value = self
                    .pending
                    .take()
                    .ok_or_else(|| parse_error("Expected value before ','"))?;
                self.insert(value)?;
            }
            b']' => {
                let Some(Container::Array {
                    mut values,
                    mut count,
                }) = self.containers.pop()
                else {
                    return Err(parse_error("Unmatched ']'"));
                };
                if let Some(value) = self.pending.take() {
                    self.emit_scalar(&value);
                    if self.retain_documents {
                        values.push(value);
                    }
                    count = count.saturating_add(1);
                } else if count != 0 {
                    return Err(parse_error("Expected another array element"));
                }
                self.pending = Some(Value::array(values));
                self.emit(Event::ArrayEnd {
                    span: span(),
                    observed_count: count,
                });
            }
            b'}' => {
                if let Some(value) = self.pending.take() {
                    if !matches!(self.containers.last(), Some(Container::Key(_))) {
                        return Err(parse_error("Objects must consist of key:value pairs"));
                    }
                    self.insert(value)?;
                } else if let Some(Container::Object { count, .. }) = self.containers.last()
                    && *count != 0
                {
                    return Err(parse_error("Expected another key-value pair"));
                }
                let Some(Container::Object { values, .. }) = self.containers.pop() else {
                    return Err(parse_error("Unmatched '}'"));
                };
                self.pending = Some(Value::object(values));
                self.emit(Event::ObjectEnd { span: span() });
            }
            _ => {}
        }
        Ok(())
    }

    fn insert(&mut self, value: Value) -> Result<(), FormatError> {
        self.emit_scalar(&value);
        match self.containers.last_mut() {
            Some(Container::Array { values, count }) => {
                if self.retain_documents {
                    values.push(value);
                }
                *count = count.saturating_add(1);
            }
            Some(Container::Key(_)) => {
                let Some(Container::Key(key)) = self.containers.pop() else {
                    unreachable!()
                };
                let Some(Container::Object { values, count }) = self.containers.last_mut() else {
                    unreachable!()
                };
                if self.retain_documents {
                    values.insert(key, value);
                }
                *count = count.saturating_add(1);
            }
            Some(Container::Object { .. }) => {
                return Err(parse_error("Objects must consist of key:value pairs"));
            }
            None => return Err(parse_error("',' not as part of an object or array")),
        }
        Ok(())
    }
}

const fn span() -> Span {
    Span::new(SourceId::new(0), 0, 0)
}

fn is_delimiter(byte: u8) -> bool {
    matches!(
        byte,
        b' ' | b'\t' | b'\r' | b'\n' | b'"' | b'[' | b']' | b'{' | b'}' | b':' | b','
    )
}

fn parse_error(message: &str) -> FormatError {
    FormatError::Parse {
        format: InputFormat::JsonSequence,
        message: message.to_owned(),
    }
}
