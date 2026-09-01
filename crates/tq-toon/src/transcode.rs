//! Structural-event to canonical TOON transcode consumer.

use std::{
    io::Write,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use thiserror::Error;
use tq_core::{Number, Object, Value};

use crate::{
    ArrayPreparationConfig, DuplicateKeyPolicy, Event, EventConsumer, PreparationArena,
    PreparationMemory, PreparationObservations, PreparedArray, PreparedKeySet, PreparedObject,
    Scalar, ScalarToken, SpoolError, WriterConfig, WriterError, write_value,
    writer::{self, ScalarContext, render_key},
};

/// Output commitment selected before structural decoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TranscodeCommitment {
    /// Completed RS-prefixed records publish independently.
    DirectSequence,
    /// Output targets an atomic publication buffer.
    AtomicUnframed,
}

/// Structural transcode failure.
#[derive(Debug, Error)]
pub enum TranscodeError {
    /// Event order violated the structural consumer contract.
    #[error("invalid structural event sequence: {0}")]
    Structure(&'static str),
    /// Strict input repeated an object key.
    #[error("duplicate object key '{0}'")]
    Duplicate(Arc<str>),
    /// Bounded preparation failed.
    #[error(transparent)]
    Spool(#[from] SpoolError),
    /// Canonical output failed.
    #[error(transparent)]
    Writer(#[from] WriterError),
    /// Framing output failed.
    #[error("TOON transcode output failed: {0}")]
    Io(#[from] std::io::Error),
    /// Invocation result count was exceeded before a new document published.
    #[error("transcode result-count limit exceeded")]
    ResultLimit,
    /// Cooperative cancellation was observed between structural events.
    #[error("transcode interrupted")]
    Cancelled,
}

/// Canonical TOON consumer over query-independent structural events.
pub struct TranscodeConsumer<W> {
    output: StagedOutput<W>,
    writer: WriterConfig,
    preparation: ArrayPreparationConfig,
    arena: PreparationArena,
    duplicate_keys: DuplicateKeyPolicy,
    commitment: TranscodeCommitment,
    frames: Vec<Frame>,
    document_active: bool,
    root_complete: bool,
    documents: u64,
    current_truthy: Option<bool>,
    last_truthy: Option<bool>,
    maximum_documents: u64,
    cancellation: Option<Arc<AtomicBool>>,
}

struct StagedOutput<W> {
    committed: W,
    pending: Option<crate::PublicationBuffer>,
}

impl<W> StagedOutput<W> {
    const fn new(committed: W) -> Self {
        Self {
            committed,
            pending: None,
        }
    }

    fn begin(
        &mut self,
        config: ArrayPreparationConfig,
        arena: PreparationArena,
    ) -> Result<(), TranscodeError> {
        if self.pending.is_some() {
            return Err(TranscodeError::Structure("nested output publication"));
        }
        self.pending = Some(crate::PublicationBuffer::new(config, arena));
        Ok(())
    }

    fn commit(&mut self) -> Result<(), TranscodeError>
    where
        W: Write,
    {
        let mut pending = self
            .pending
            .take()
            .ok_or(TranscodeError::Structure("missing output publication"))?;
        pending
            .publish(&mut self.committed)
            .map_err(|error| match error {
                crate::PublicationError::Cardinality(_) => {
                    TranscodeError::Structure("invalid sequence publication cardinality")
                }
                crate::PublicationError::Spool(error) => TranscodeError::Spool(error),
                crate::PublicationError::Io(error) => TranscodeError::Io(error),
            })?;
        self.committed.flush().map_err(TranscodeError::Io)
    }

    fn into_inner(self) -> W {
        self.committed
    }
}

impl<W: Write> Write for StagedOutput<W> {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        if let Some(pending) = &mut self.pending {
            pending.write(buffer)
        } else {
            self.committed.write(buffer)
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if let Some(pending) = &mut self.pending {
            pending.flush()
        } else {
            self.committed.flush()
        }
    }
}

enum Frame {
    DirectObject {
        _charge: crate::PreparationFrame,
        parent_key: Option<Arc<str>>,
        depth: usize,
        pending_key: Option<Arc<str>>,
        seen: PreparedKeySet,
        wrote_member: bool,
        header_published: bool,
    },
    RootObjectNormalized {
        _charge: crate::PreparationFrame,
        pending_key: Option<Arc<str>>,
        object: PreparedObject,
    },
    RootArray {
        _charge: crate::PreparationFrame,
        array: PreparedArray,
    },
    DirectArray {
        _charge: crate::PreparationFrame,
        parent_key: Arc<str>,
        depth: usize,
        array: PreparedArray,
    },
    Object {
        _charge: crate::PreparationFrame,
        memory: PreparationMemory,
        pending_key: Option<Arc<str>>,
        values: Object,
    },
    Array {
        _charge: crate::PreparationFrame,
        memory: PreparationMemory,
        values: Vec<Value>,
    },
}

struct IndentingWriter<'a, W> {
    output: &'a mut W,
    indentation: usize,
    line_start: bool,
}

impl<W: Write> Write for IndentingWriter<'_, W> {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let mut start = 0;
        while start < buffer.len() {
            if self.line_start {
                for _ in 0..self.indentation {
                    self.output.write_all(b" ")?;
                }
                self.line_start = false;
            }
            if let Some(relative) = buffer[start..].iter().position(|byte| *byte == b'\n') {
                let end = start + relative + 1;
                self.output.write_all(&buffer[start..end])?;
                self.line_start = true;
                start = end;
            } else {
                self.output.write_all(&buffer[start..])?;
                break;
            }
        }
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.output.flush()
    }
}

impl<W: Write> TranscodeConsumer<W> {
    /// Creates a consumer for one selected decoder and output commitment.
    #[must_use]
    pub fn new(
        output: W,
        writer: WriterConfig,
        preparation: ArrayPreparationConfig,
        arena: PreparationArena,
        duplicate_keys: DuplicateKeyPolicy,
        commitment: TranscodeCommitment,
    ) -> Self {
        Self {
            output: StagedOutput::new(output),
            writer,
            preparation,
            arena,
            duplicate_keys,
            commitment,
            frames: Vec::new(),
            document_active: false,
            root_complete: false,
            documents: 0,
            current_truthy: None,
            last_truthy: None,
            maximum_documents: u64::MAX,
            cancellation: None,
        }
    }

    /// Number of successfully completed documents.
    #[must_use]
    pub const fn documents(&self) -> u64 {
        self.documents
    }

    /// Aggregate preparation observations.
    #[must_use]
    pub fn observations(&self) -> PreparationObservations {
        self.arena.observations()
    }

    /// jq-compatible truthiness of the last completed identity result.
    #[must_use]
    pub const fn last_truthy(&self) -> Option<bool> {
        self.last_truthy
    }

    /// Applies an invocation-wide result count limit.
    #[must_use]
    pub const fn with_document_limit(mut self, maximum_documents: u64) -> Self {
        self.maximum_documents = maximum_documents;
        self
    }

    /// Applies cooperative cancellation checks between decoder events.
    #[must_use]
    pub fn with_cancellation(mut self, cancellation: Arc<AtomicBool>) -> Self {
        self.cancellation = Some(cancellation);
        self
    }

    /// Returns the output sink after decoding.
    #[must_use]
    pub fn into_inner(self) -> W {
        self.output.into_inner()
    }

    fn check_cancellation(&self) -> Result<(), TranscodeError> {
        if self
            .cancellation
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
        {
            Err(TranscodeError::Cancelled)
        } else {
            Ok(())
        }
    }

    fn accepts_lightweight_scalar(&self) -> bool {
        self.frames.last().is_none_or(|frame| {
            matches!(
                frame,
                Frame::DirectObject { .. } | Frame::RootArray { .. } | Frame::DirectArray { .. }
            )
        })
    }

    fn start_object(&mut self) -> Result<(), TranscodeError> {
        let charge = self.arena.enter()?;
        if self.duplicate_keys == DuplicateKeyPolicy::Reject
            && (self.frames.is_empty()
                || matches!(self.frames.last(), Some(Frame::DirectObject { .. })))
        {
            self.current_truthy = Some(true);
            let (parent_key, depth) = if let Some(Frame::DirectObject {
                pending_key, depth, ..
            }) = self.frames.last_mut()
            {
                (
                    Some(
                        pending_key
                            .take()
                            .ok_or(TranscodeError::Structure("object value without a key"))?,
                    ),
                    depth.saturating_add(1),
                )
            } else {
                (None, 0)
            };
            self.frames.push(Frame::DirectObject {
                _charge: charge,
                parent_key,
                depth,
                pending_key: None,
                seen: PreparedKeySet::new(self.preparation.clone(), self.arena.clone()),
                wrote_member: false,
                header_published: false,
            });
        } else if self.frames.is_empty() && !self.root_complete {
            self.current_truthy = Some(true);
            self.frames.push(Frame::RootObjectNormalized {
                _charge: charge,
                pending_key: None,
                object: PreparedObject::new(self.preparation.clone(), self.arena.clone()),
            });
        } else {
            self.frames.push(Frame::Object {
                _charge: charge,
                memory: self.arena.memory_charge(),
                pending_key: None,
                values: Object::new(),
            });
        }
        Ok(())
    }

    fn start_array(&mut self) -> Result<(), TranscodeError> {
        let charge = self.arena.enter()?;
        if self.frames.is_empty() && !self.root_complete {
            self.current_truthy = Some(true);
            self.frames.push(Frame::RootArray {
                _charge: charge,
                array: PreparedArray::in_arena(self.preparation.clone(), self.arena.clone()),
            });
        } else if matches!(self.frames.last(), Some(Frame::DirectObject { .. })) {
            let (parent_key, depth) = match self.frames.last_mut() {
                Some(Frame::DirectObject {
                    pending_key, depth, ..
                }) => (
                    pending_key
                        .take()
                        .ok_or(TranscodeError::Structure("object value without a key"))?,
                    *depth,
                ),
                _ => unreachable!(),
            };
            self.frames.push(Frame::DirectArray {
                _charge: charge,
                parent_key,
                depth,
                array: PreparedArray::in_arena(self.preparation.clone(), self.arena.clone()),
            });
        } else {
            self.frames.push(Frame::Array {
                _charge: charge,
                memory: self.arena.memory_charge(),
                values: Vec::new(),
            });
        }
        Ok(())
    }

    fn key(&mut self, key: Arc<str>) -> Result<(), TranscodeError> {
        match self.frames.last_mut() {
            Some(Frame::DirectObject {
                pending_key, seen, ..
            }) => {
                if !seen.insert(Arc::clone(&key))? {
                    return Err(TranscodeError::Duplicate(key));
                }
                if pending_key.replace(key).is_some() {
                    return Err(TranscodeError::Structure("object key without a value"));
                }
            }
            Some(Frame::RootObjectNormalized { pending_key, .. }) => {
                if pending_key.replace(key).is_some() {
                    return Err(TranscodeError::Structure("object key without a value"));
                }
            }
            Some(Frame::Object {
                memory,
                pending_key,
                ..
            }) => {
                memory.grow(key.len().saturating_add(64))?;
                if pending_key.replace(key).is_some() {
                    return Err(TranscodeError::Structure("object key without a value"));
                }
            }
            _ => return Err(TranscodeError::Structure("key outside an object")),
        }
        Ok(())
    }

    fn complete_value(&mut self, value: Value) -> Result<(), TranscodeError> {
        match self.frames.last_mut() {
            Some(Frame::DirectObject { pending_key, .. }) => {
                let key = pending_key
                    .take()
                    .ok_or(TranscodeError::Structure("object value without a key"))?;
                let index = self.frames.len() - 1;
                self.prepare_direct_member_line(index)?;
                self.output.write_all(render_key(&key).as_bytes())?;
                if matches!(value, Value::Array(_)) {
                    // Canonical TOON joins an object key directly to an array header.
                } else {
                    self.output.write_all(b": ")?;
                }
                let depth = match &self.frames[index] {
                    Frame::DirectObject { depth, .. } => *depth,
                    _ => unreachable!(),
                };
                let mut indented = IndentingWriter {
                    output: &mut self.output,
                    indentation: depth.saturating_mul(self.writer.indent_size),
                    line_start: false,
                };
                write_value(&mut indented, &value, self.writer)?;
                let Frame::DirectObject { wrote_member, .. } = &mut self.frames[index] else {
                    unreachable!()
                };
                *wrote_member = true;
            }
            Some(Frame::RootObjectNormalized {
                pending_key,
                object,
                ..
            }) => {
                let key = pending_key
                    .take()
                    .ok_or(TranscodeError::Structure("object value without a key"))?;
                object.push(key, &value)?;
            }
            Some(Frame::RootArray { array, .. } | Frame::DirectArray { array, .. }) => {
                array.push(&value)?;
            }
            Some(Frame::Object {
                pending_key,
                memory,
                values,
                ..
            }) => {
                let key = pending_key
                    .take()
                    .ok_or(TranscodeError::Structure("object value without a key"))?;
                if self.duplicate_keys == DuplicateKeyPolicy::Reject && values.contains_key(&key) {
                    return Err(TranscodeError::Duplicate(key));
                }
                memory.grow(retained_value_bytes(&value))?;
                values.insert(key, value);
            }
            Some(Frame::Array { memory, values, .. }) => {
                memory.grow(retained_value_bytes(&value))?;
                values.push(value);
            }
            None if !self.root_complete => {
                self.current_truthy = Some(!matches!(value, Value::Null | Value::Bool(false)));
                write_value(&mut self.output, &value, self.writer)?;
                self.root_complete = true;
            }
            None => return Err(TranscodeError::Structure("multiple roots in one document")),
        }
        Ok(())
    }

    fn complete_scalar(&mut self, value: ScalarToken<'_>) -> Result<(), TranscodeError> {
        match self.frames.last_mut() {
            Some(Frame::DirectObject { pending_key, .. }) => {
                let key = pending_key
                    .take()
                    .ok_or(TranscodeError::Structure("object value without a key"))?;
                let index = self.frames.len() - 1;
                self.prepare_direct_member_line(index)?;
                self.output.write_all(render_key(&key).as_bytes())?;
                self.output.write_all(b": ")?;
                let depth = match &self.frames[index] {
                    Frame::DirectObject { depth, .. } => *depth,
                    _ => unreachable!(),
                };
                let mut indented = IndentingWriter {
                    output: &mut self.output,
                    indentation: depth.saturating_mul(self.writer.indent_size),
                    line_start: false,
                };
                writer::write_scalar_token(&mut indented, value, self.writer, ScalarContext::Root)?;
                let Frame::DirectObject { wrote_member, .. } = &mut self.frames[index] else {
                    unreachable!()
                };
                *wrote_member = true;
            }
            Some(Frame::RootArray { array, .. } | Frame::DirectArray { array, .. }) => {
                array.push_scalar(value)?;
            }
            None if !self.root_complete => {
                self.current_truthy = Some(!matches!(
                    value,
                    ScalarToken::Null | ScalarToken::Bool(false)
                ));
                writer::write_scalar_token(
                    &mut self.output,
                    value,
                    self.writer,
                    ScalarContext::Root,
                )?;
                self.root_complete = true;
            }
            _ => self.complete_value(scalar_token_value(value)?)?,
        }
        Ok(())
    }

    fn end_object(&mut self) -> Result<(), TranscodeError> {
        let frame = self
            .frames
            .pop()
            .ok_or(TranscodeError::Structure("object end without start"))?;
        match frame {
            Frame::DirectObject {
                pending_key,
                parent_key,
                header_published,
                ..
            } => {
                if pending_key.is_some() {
                    return Err(TranscodeError::Structure("object key without a value"));
                }
                if let Some(key) = parent_key {
                    if !header_published {
                        let parent =
                            self.frames
                                .len()
                                .checked_sub(1)
                                .ok_or(TranscodeError::Structure(
                                    "nested object without direct parent",
                                ))?;
                        self.prepare_direct_member_line(parent)?;
                        self.output.write_all(render_key(&key).as_bytes())?;
                        self.output.write_all(b":")?;
                        let Frame::DirectObject { wrote_member, .. } = &mut self.frames[parent]
                        else {
                            return Err(TranscodeError::Structure(
                                "nested object parent is not direct",
                            ));
                        };
                        *wrote_member = true;
                    }
                } else {
                    self.root_complete = true;
                }
            }
            Frame::RootObjectNormalized {
                pending_key,
                mut object,
                ..
            } => {
                if pending_key.is_some() {
                    return Err(TranscodeError::Structure("object key without a value"));
                }
                let mut wrote_member = false;
                object.for_each_member(|key, value| {
                    if wrote_member {
                        self.output.write_all(b"\n")?;
                    }
                    let mut member = Object::new();
                    member.insert(Arc::from(key), value.clone());
                    write_value(&mut self.output, &Value::object(member), self.writer)
                        .map_err(|WriterError::Io(error)| SpoolError::Io(error))?;
                    wrote_member = true;
                    Ok(())
                })?;
                self.root_complete = true;
            }
            Frame::Object {
                pending_key,
                memory,
                values,
                ..
            } => {
                if pending_key.is_some() {
                    return Err(TranscodeError::Structure("object key without a value"));
                }
                drop(memory);
                self.complete_value(Value::object(values))?;
            }
            Frame::RootArray { .. } | Frame::DirectArray { .. } | Frame::Array { .. } => {
                return Err(TranscodeError::Structure("object end closed an array"));
            }
        }
        Ok(())
    }

    fn end_array(&mut self) -> Result<(), TranscodeError> {
        let frame = self
            .frames
            .pop()
            .ok_or(TranscodeError::Structure("array end without start"))?;
        match frame {
            Frame::RootArray { mut array, .. } => {
                array.write_to(&mut self.output, self.writer)?;
                self.root_complete = true;
            }
            Frame::DirectArray {
                parent_key,
                depth,
                mut array,
                ..
            } => {
                let parent = self
                    .frames
                    .len()
                    .checked_sub(1)
                    .ok_or(TranscodeError::Structure("nested array without parent"))?;
                self.prepare_direct_member_line(parent)?;
                self.output.write_all(render_key(&parent_key).as_bytes())?;
                let mut indented = IndentingWriter {
                    output: &mut self.output,
                    indentation: depth.saturating_mul(self.writer.indent_size),
                    line_start: false,
                };
                array.write_to(&mut indented, self.writer)?;
                let Frame::DirectObject { wrote_member, .. } = &mut self.frames[parent] else {
                    return Err(TranscodeError::Structure(
                        "nested array parent is not direct",
                    ));
                };
                *wrote_member = true;
            }
            Frame::Array { memory, values, .. } => {
                drop(memory);
                self.complete_value(Value::array(values))?;
            }
            Frame::DirectObject { .. }
            | Frame::RootObjectNormalized { .. }
            | Frame::Object { .. } => {
                return Err(TranscodeError::Structure("array end closed an object"));
            }
        }
        Ok(())
    }

    fn prepare_direct_member_line(&mut self, index: usize) -> Result<(), TranscodeError> {
        let (parent_key, depth, wrote_member, header_published) = match &self.frames[index] {
            Frame::DirectObject {
                parent_key,
                depth,
                wrote_member,
                header_published,
                ..
            } => (parent_key.clone(), *depth, *wrote_member, *header_published),
            _ => return Err(TranscodeError::Structure("member outside direct object")),
        };
        if !header_published {
            if let Some(ref key) = parent_key {
                let parent = index
                    .checked_sub(1)
                    .ok_or(TranscodeError::Structure("nested object without parent"))?;
                self.prepare_direct_member_line(parent)?;
                self.output.write_all(render_key(key).as_bytes())?;
                self.output.write_all(b":")?;
                let Frame::DirectObject { wrote_member, .. } = &mut self.frames[parent] else {
                    return Err(TranscodeError::Structure(
                        "nested object parent is not direct",
                    ));
                };
                *wrote_member = true;
                let Frame::DirectObject {
                    header_published, ..
                } = &mut self.frames[index]
                else {
                    unreachable!()
                };
                *header_published = true;
            }
        }
        let nested_header = parent_key.is_some();
        if wrote_member || nested_header {
            self.output.write_all(b"\n")?;
        }
        for _ in 0..depth.saturating_mul(self.writer.indent_size) {
            self.output.write_all(b" ")?;
        }
        Ok(())
    }
}

fn scalar_token_value(value: ScalarToken<'_>) -> Result<Value, TranscodeError> {
    Ok(match value {
        ScalarToken::Null => Value::Null,
        ScalarToken::Bool(value) => Value::Bool(value),
        ScalarToken::Number(value) => Value::Number(
            Number::parse(value).map_err(|_| TranscodeError::Structure("invalid number token"))?,
        ),
        ScalarToken::String(value) => Value::string(value.to_owned()),
    })
}

fn retained_value_bytes(value: &Value) -> usize {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) => std::mem::size_of::<Value>(),
        Value::String(value) => std::mem::size_of::<Value>().saturating_add(value.len()),
        Value::Array(values) => values
            .iter()
            .fold(std::mem::size_of::<Value>(), |bytes, value| {
                bytes.saturating_add(retained_value_bytes(value))
            }),
        Value::Object(values) => {
            values
                .iter()
                .fold(std::mem::size_of::<Value>(), |bytes, (key, value)| {
                    bytes
                        .saturating_add(key.len())
                        .saturating_add(64)
                        .saturating_add(retained_value_bytes(value))
                })
        }
    }
}

impl<W: Write> EventConsumer for TranscodeConsumer<W> {
    type Error = TranscodeError;

    fn consume(&mut self, event: Event) -> Result<(), Self::Error> {
        self.check_cancellation()?;
        match event {
            Event::DocumentStart { .. } => {
                if self.documents >= self.maximum_documents {
                    return Err(TranscodeError::ResultLimit);
                }
                if self.document_active || !self.frames.is_empty() {
                    return Err(TranscodeError::Structure("nested document start"));
                }
                self.document_active = true;
                self.root_complete = false;
                self.current_truthy = None;
                if self.commitment == TranscodeCommitment::DirectSequence {
                    self.output
                        .begin(self.preparation.clone(), self.arena.clone())?;
                    self.output.write_all(b"\x1e")?;
                }
            }
            Event::DocumentEnd { .. } => {
                if !self.document_active || !self.root_complete || !self.frames.is_empty() {
                    return Err(TranscodeError::Structure("incomplete document"));
                }
                if self.commitment == TranscodeCommitment::DirectSequence {
                    self.output.write_all(b"\n")?;
                    self.output.commit()?;
                }
                self.document_active = false;
                self.documents = self.documents.saturating_add(1);
                self.last_truthy = self.current_truthy;
            }
            Event::ObjectStart { .. } => self.start_object()?,
            Event::ObjectEnd { .. } => self.end_object()?,
            Event::Key { value, .. } => self.key(value)?,
            Event::ArrayStart { .. } => self.start_array()?,
            Event::ArrayEnd { .. } => self.end_array()?,
            Event::Scalar { value, .. } => {
                let value = match value {
                    Scalar::Null => Value::Null,
                    Scalar::Bool(value) => Value::Bool(value),
                    Scalar::Number(value) => Value::Number(value),
                    Scalar::String(value) => Value::String(value),
                };
                self.complete_value(value)?;
            }
        }
        Ok(())
    }

    fn consume_text_key(
        &mut self,
        _span: tq_core::Span,
        value: String,
        _quoted: bool,
    ) -> Result<(), String> {
        self.check_cancellation()
            .map_err(|error| error.to_string())?;
        self.key(Arc::from(value))
            .map_err(|error| error.to_string())
    }

    fn consume_null(&mut self, _span: tq_core::Span) -> Result<(), String> {
        self.check_cancellation()
            .map_err(|error| error.to_string())?;
        self.complete_scalar(ScalarToken::Null)
            .map_err(|error| error.to_string())
    }

    fn consume_bool(&mut self, _span: tq_core::Span, value: bool) -> Result<(), String> {
        self.check_cancellation()
            .map_err(|error| error.to_string())?;
        self.complete_scalar(ScalarToken::Bool(value))
            .map_err(|error| error.to_string())
    }

    fn consume_text_string(&mut self, _span: tq_core::Span, value: String) -> Result<(), String> {
        self.check_cancellation()
            .map_err(|error| error.to_string())?;
        if self.accepts_lightweight_scalar() {
            self.complete_scalar(ScalarToken::String(&value))
                .map_err(|error| error.to_string())
        } else {
            self.complete_value(Value::string(value))
                .map_err(|error| error.to_string())
        }
    }

    fn consume_number_literal(
        &mut self,
        _span: tq_core::Span,
        literal: String,
    ) -> Result<(), String> {
        self.check_cancellation()
            .map_err(|error| error.to_string())?;
        if self.accepts_lightweight_scalar() {
            let canonical =
                Number::canonicalize_literal(&literal).map_err(|error| error.to_string())?;
            self.complete_scalar(ScalarToken::Number(&canonical))
                .map_err(|error| error.to_string())
        } else {
            let number = Number::parse(&literal).map_err(|error| error.to_string())?;
            self.complete_value(Value::Number(number))
                .map_err(|error| error.to_string())
        }
    }
}
