//! Structural-event to canonical TOON transcode consumer.

use std::{
    io::{BufWriter, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use thiserror::Error;
use tq_core::presentation::ColorPalette;

use crate::{
    ArrayPreparationConfig, DuplicateKeyPolicy, Event, EventConsumer, PreparationArena,
    PreparationMemory, PreparationObservations, Scalar, ScalarToken, SpoolError, WriterConfig,
    WriterError,
    spool::event_tape::{ContainerKind, PreparedContainer, PreparedNode, PreparedTape, TapeError},
};
/// Output commitment selected before structural decoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TranscodeCommitment {
    /// Completed RS-prefixed records publish independently.
    DirectSequence,
    /// LF-terminated values without RS framing.
    DirectValues,
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
    tape: Option<PreparedTape>,
    frames_memory: PreparationMemory,
    frames: Vec<Frame>,
    root: Option<PreparedNode>,
    document_active: bool,
    root_complete: bool,
    documents: u64,
    current_truthy: Option<bool>,
    last_truthy: Option<bool>,
    maximum_documents: u64,
    cancellation: Option<Arc<AtomicBool>>,
    palette: Option<ColorPalette>,
}

struct Frame {
    kind: ContainerKind,
    container: PreparedContainer,
}

struct StagedOutput<W> {
    committed: W,
    pending: Option<BufWriter<crate::PublicationBuffer>>,
    pending_memory: Option<PreparationMemory>,
}

impl<W> StagedOutput<W> {
    const fn new(committed: W) -> Self {
        Self {
            committed,
            pending: None,
            pending_memory: None,
        }
    }

    fn begin(
        &mut self,
        config: ArrayPreparationConfig,
        arena: PreparationArena,
    ) -> Result<(), TranscodeError> {
        const CAPACITY: usize = 64 * 1024;
        if self.pending.is_some() {
            return Err(TranscodeError::Structure("nested output publication"));
        }
        let mut memory = arena.memory_charge();
        memory.grow(CAPACITY)?;
        self.pending_memory = Some(memory);
        // Token-sized writes must not become individual spool-file writes.
        self.pending = Some(BufWriter::with_capacity(
            CAPACITY,
            crate::PublicationBuffer::new(config, arena),
        ));
        Ok(())
    }

    fn commit(&mut self, palette: Option<&ColorPalette>) -> Result<(), TranscodeError>
    where
        W: Write,
    {
        let mut pending = self
            .pending
            .take()
            .ok_or(TranscodeError::Structure("missing output publication"))?;
        pending.flush()?;
        if let Err(error) = pending
            .get_mut()
            .publish_colored(&mut self.committed, palette)
        {
            return Err(match error {
                crate::PublicationError::Cardinality(_) => {
                    TranscodeError::Structure("invalid sequence publication cardinality")
                }
                crate::PublicationError::Spool(error) => TranscodeError::Spool(error),
                crate::PublicationError::Io(error) => TranscodeError::Io(error),
            });
        }
        drop(pending);
        drop(self.pending_memory.take());
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
        let frames_memory = arena.memory_charge();
        Self {
            output: StagedOutput::new(output),
            writer,
            preparation,
            arena,
            duplicate_keys,
            commitment,
            tape: None,
            frames_memory,
            frames: Vec::new(),
            root: None,
            document_active: false,
            root_complete: false,
            documents: 0,
            current_truthy: None,
            last_truthy: None,
            maximum_documents: u64::MAX,
            cancellation: None,
            palette: None,
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

    /// Applies an invocation-owned semantic palette to all TOON output paths.
    #[must_use]
    pub fn with_palette(mut self, palette: ColorPalette) -> Self {
        self.palette = Some(palette);
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

    fn tape(&mut self) -> Result<&mut PreparedTape, TranscodeError> {
        self.tape
            .as_mut()
            .ok_or(TranscodeError::Structure("value outside a document"))
    }

    fn start_container(&mut self, kind: ContainerKind) -> Result<(), TranscodeError> {
        if self.root_complete {
            return Err(TranscodeError::Structure("multiple roots in one document"));
        }
        if self.frames.len() >= crate::writer::MAX_WRITER_DEPTH {
            return Err(WriterError::Schema {
                resource: "depth",
                limit: crate::writer::MAX_WRITER_DEPTH,
            }
            .into());
        }
        if self.frames.len() == self.frames.capacity() {
            let next_capacity = self.frames.capacity().max(4).saturating_mul(2);
            let additional = next_capacity.saturating_sub(self.frames.capacity());
            self.frames_memory
                .grow(additional.saturating_mul(std::mem::size_of::<Frame>()))?;
            self.frames.reserve_exact(additional);
        }
        let container = self.tape()?.begin(kind).map_err(tape_error)?;
        self.frames.push(Frame { kind, container });
        Ok(())
    }

    fn key(&mut self, key: Arc<str>) -> Result<(), TranscodeError> {
        let tape = self
            .tape
            .as_mut()
            .ok_or(TranscodeError::Structure("key outside a document"))?;
        let frame = self
            .frames
            .last_mut()
            .ok_or(TranscodeError::Structure("key outside an object"))?;
        if frame.kind != ContainerKind::Object {
            return Err(TranscodeError::Structure("key outside an object"));
        }
        tape.key(&mut frame.container, key).map_err(tape_error)
    }

    fn attach(&mut self, node: PreparedNode) -> Result<(), TranscodeError> {
        if self.frames.is_empty() {
            if self.root.is_some() || self.root_complete {
                return Err(TranscodeError::Structure("multiple roots in one document"));
            }
            self.current_truthy = Some(node.truthy());
            let tape = self
                .tape
                .as_mut()
                .ok_or(TranscodeError::Structure("value outside a document"))?;
            tape.write(&node, &mut self.output, self.writer, self.palette.as_ref())
                .map_err(tape_error)?;
            self.root = Some(node);
            self.root_complete = true;
            return Ok(());
        }
        let tape = self
            .tape
            .as_mut()
            .ok_or(TranscodeError::Structure("value outside a document"))?;
        let parent = self
            .frames
            .last_mut()
            .ok_or(TranscodeError::Structure("missing prepared parent"))?;
        tape.push(&mut parent.container, node).map_err(tape_error)
    }

    fn complete_scalar(&mut self, value: ScalarToken<'_>) -> Result<(), TranscodeError> {
        let node = self.tape()?.scalar_token(value).map_err(tape_error)?;
        self.attach(node)
    }

    fn complete_decoded_scalar(&mut self, value: Scalar) -> Result<(), TranscodeError> {
        let node = self.tape()?.scalar(value).map_err(tape_error)?;
        self.attach(node)
    }

    fn end_container(&mut self, kind: ContainerKind) -> Result<(), TranscodeError> {
        let frame = self
            .frames
            .pop()
            .ok_or(TranscodeError::Structure("container end without start"))?;
        if frame.kind != kind {
            return Err(match kind {
                ContainerKind::Object => TranscodeError::Structure("object end closed an array"),
                ContainerKind::Array => TranscodeError::Structure("array end closed an object"),
            });
        }
        let node = self.tape()?.finish(frame.container).map_err(tape_error)?;
        self.attach(node)
    }
}

fn tape_error(error: TapeError) -> TranscodeError {
    match error {
        TapeError::Spool(error) => TranscodeError::Spool(error),
        TapeError::Writer(error) => TranscodeError::Writer(error),
        TapeError::Duplicate(key) => TranscodeError::Duplicate(key),
        TapeError::Structure(message) => TranscodeError::Structure(message),
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
                self.tape = Some(PreparedTape::new(
                    self.preparation.clone(),
                    self.arena.clone(),
                    self.duplicate_keys,
                ));
                self.root = None;
                self.document_active = true;
                self.root_complete = false;
                self.current_truthy = None;
                if self.commitment == TranscodeCommitment::DirectSequence {
                    self.output.write_all(b"\x1e")?;
                } else if self.commitment == TranscodeCommitment::DirectValues {
                    self.output
                        .begin(self.preparation.clone(), self.arena.clone())?;
                }
            }
            Event::DocumentEnd { .. } => {
                if !self.document_active || !self.root_complete || !self.frames.is_empty() {
                    return Err(TranscodeError::Structure("incomplete document"));
                }
                if self.commitment == TranscodeCommitment::DirectSequence {
                    self.output.write_all(b"\n")?;
                    self.output.committed.flush().map_err(TranscodeError::Io)?;
                } else if self.commitment == TranscodeCommitment::DirectValues {
                    self.output.write_all(b"\n")?;
                    self.output.commit(self.palette.as_ref())?;
                }
                self.document_active = false;
                self.tape = None;
                self.root = None;
                self.documents = self.documents.saturating_add(1);
                self.last_truthy = self.current_truthy;
            }
            Event::ObjectStart { .. } => self.start_container(ContainerKind::Object)?,
            Event::ObjectEnd { .. } => self.end_container(ContainerKind::Object)?,
            Event::Key { value, .. } => self.key(value)?,
            Event::ArrayStart { .. } => self.start_container(ContainerKind::Array)?,
            Event::ArrayEnd { .. } => self.end_container(ContainerKind::Array)?,
            Event::Scalar { value, .. } => self.complete_decoded_scalar(value)?,
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
        self.complete_decoded_scalar(Scalar::String(Arc::from(value)))
            .map_err(|error| error.to_string())
    }

    fn consume_number_literal(
        &mut self,
        _span: tq_core::Span,
        literal: String,
    ) -> Result<(), String> {
        self.check_cancellation()
            .map_err(|error| error.to_string())?;
        self.complete_scalar(ScalarToken::Number(&literal))
            .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{self, BufWriter, Write},
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
        },
    };

    use tq_core::{SourceId, Span};

    use super::{TranscodeCommitment, TranscodeConsumer};
    use crate::{
        ArrayPreparationConfig, DuplicateKeyPolicy, Event, EventConsumer, PreparationArena,
        PreparationLimits, PublicationBuffer, Scalar, WriterConfig,
    };
    #[derive(Clone, Default)]
    struct SharedSink(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedSink {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn true_member(consumer: &mut TranscodeConsumer<SharedSink>, span: Span, key: &str) {
        consumer
            .consume(Event::Key {
                span,
                value: Arc::from(key),
                quoted: false,
            })
            .unwrap();
        consumer
            .consume(Event::Scalar {
                span,
                value: Scalar::Bool(true),
            })
            .unwrap();
    }

    #[test]
    fn sequence_commits_prefix_before_root_and_keeps_prior_results_on_late_error() {
        let span = Span::new(SourceId::new(1), 0, 0);
        let sink = SharedSink::default();
        let observed = Arc::clone(&sink.0);
        let mut consumer = TranscodeConsumer::new(
            sink,
            WriterConfig::default(),
            ArrayPreparationConfig::default(),
            PreparationArena::new(PreparationLimits::default()),
            DuplicateKeyPolicy::Reject,
            TranscodeCommitment::DirectSequence,
        );

        consumer.consume(Event::DocumentStart { span }).unwrap();
        assert_eq!(
            &*observed
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
            b"\x1e"
        );
        consumer
            .consume(Event::ArrayStart {
                span,
                declared_count: Some(2),
            })
            .unwrap();
        for key in ["a", "b"] {
            consumer.consume(Event::ObjectStart { span }).unwrap();
            true_member(&mut consumer, span, key);
            consumer.consume(Event::ObjectEnd { span }).unwrap();
            if key == "a" {
                assert_eq!(
                    &*observed
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner),
                    b"\x1e"
                );
            }
        }
        consumer
            .consume(Event::ArrayEnd {
                span,
                observed_count: 2,
            })
            .unwrap();
        let completed_body = observed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        assert!(!completed_body.windows(4).any(|window| window == b"[2]{"));
        consumer.consume(Event::DocumentEnd { span }).unwrap();
        let first_result = observed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        assert_eq!(first_result.first(), Some(&b'\x1e'));
        assert_eq!(first_result.last(), Some(&b'\n'));

        consumer.consume(Event::DocumentStart { span }).unwrap();
        assert!(
            observed
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .ends_with(b"\x1e")
        );
        consumer.consume(Event::ObjectStart { span }).unwrap();
        true_member(&mut consumer, span, "x");
        assert!(
            consumer
                .consume(Event::Key {
                    span,
                    value: Arc::from("x"),
                    quoted: false,
                })
                .is_err()
        );
        let after_error = observed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        assert!(after_error.starts_with(&first_result));
        assert!(after_error.ends_with(b"\x1e"));
    }

    #[test]
    fn unframed_resource_failure_and_cardinality_rejection_publish_nothing() {
        let span = Span::new(SourceId::new(1), 0, 0);
        let preparation = ArrayPreparationConfig {
            memory_threshold_bytes: 0,
            allow_spool: false,
            ..ArrayPreparationConfig::default()
        };
        let arena = PreparationArena::new(PreparationLimits::default());
        let publication = PublicationBuffer::new(preparation.clone(), arena.clone());
        let mut consumer = TranscodeConsumer::new(
            BufWriter::with_capacity(64 * 1024, publication),
            WriterConfig::default(),
            preparation,
            arena,
            DuplicateKeyPolicy::LastValueFirstPosition,
            TranscodeCommitment::AtomicUnframed,
        );

        consumer.consume(Event::DocumentStart { span }).unwrap();
        consumer
            .consume(Event::ArrayStart {
                span,
                declared_count: Some(1),
            })
            .unwrap();
        assert!(
            consumer
                .consume(Event::Scalar {
                    span,
                    value: Scalar::Bool(true),
                })
                .is_err()
        );
        let mut publication = consumer.into_inner().into_inner().unwrap();
        let mut visible = Vec::new();
        assert!(publication.publish_single(&mut visible, 0).is_err());
        assert_eq!(visible, [] as [u8; 0]);
    }

    #[test]
    fn root_object_selects_keyed_layout_from_final_normalized_rows() {
        let span = Span::new(SourceId::new(1), 0, 0);
        let arena = PreparationArena::new(PreparationLimits::default());
        let preparation = ArrayPreparationConfig {
            memory_threshold_bytes: 1,
            ..ArrayPreparationConfig::default()
        };
        let sink = SharedSink::default();
        let observed = Arc::clone(&sink.0);
        let mut consumer = TranscodeConsumer::new(
            sink,
            WriterConfig::default(),
            preparation,
            arena.clone(),
            DuplicateKeyPolicy::LastValueFirstPosition,
            TranscodeCommitment::DirectSequence,
        );
        consumer.consume(Event::DocumentStart { span }).unwrap();
        assert_eq!(
            &*observed
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
            b"\x1e"
        );
        consumer.consume(Event::ObjectStart { span }).unwrap();
        consumer
            .consume(Event::Key {
                span,
                value: Arc::from("one"),
                quoted: false,
            })
            .unwrap();
        consumer
            .consume(Event::Scalar {
                span,
                value: Scalar::Bool(false),
            })
            .unwrap();
        for (entry, value) in [("two", false), ("one", true)] {
            consumer
                .consume(Event::Key {
                    span,
                    value: Arc::from(entry),
                    quoted: false,
                })
                .unwrap();
            consumer.consume(Event::ObjectStart { span }).unwrap();
            consumer
                .consume(Event::Key {
                    span,
                    value: Arc::from("x"),
                    quoted: false,
                })
                .unwrap();
            consumer.consume(Event::ObjectStart { span }).unwrap();
            consumer
                .consume(Event::Key {
                    span,
                    value: Arc::from("y"),
                    quoted: false,
                })
                .unwrap();
            consumer
                .consume(Event::Scalar {
                    span,
                    value: Scalar::Bool(value),
                })
                .unwrap();
            consumer.consume(Event::ObjectEnd { span }).unwrap();
            consumer.consume(Event::ObjectEnd { span }).unwrap();
            assert_eq!(
                &*observed
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner),
                b"\x1e"
            );
        }
        consumer.consume(Event::ObjectEnd { span }).unwrap();
        assert_eq!(
            &*observed
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
            b"\x1e[2:]{x{y}}:\n  one: true\n  two: false"
        );
        consumer.consume(Event::DocumentEnd { span }).unwrap();
        assert!(
            observed
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .ends_with(b"\n")
        );
        assert!(arena.observations().spool_bytes_written > 0);
        let bytes = consumer.into_inner();
        assert_eq!(
            bytes
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .as_slice(),
            b"\x1e[2:]{x{y}}:\n  one: true\n  two: false\n"
        );
    }
    fn assert_spilled_object(
        emit: impl FnOnce(&mut TranscodeConsumer<SharedSink>, Span, &str) -> String,
    ) {
        const MEMORY_BUDGET: usize = 48 * 1024;
        let span = Span::new(SourceId::new(1), 0, 0);
        let payload = "x".repeat(8192);
        let arena = PreparationArena::new(PreparationLimits {
            memory_bytes: MEMORY_BUDGET,
            ..PreparationLimits::default()
        });
        let sink = SharedSink::default();
        let observed = Arc::clone(&sink.0);
        let mut consumer = TranscodeConsumer::new(
            sink,
            WriterConfig::default(),
            ArrayPreparationConfig {
                memory_threshold_bytes: 1024,
                ..ArrayPreparationConfig::default()
            },
            arena.clone(),
            DuplicateKeyPolicy::Reject,
            TranscodeCommitment::DirectSequence,
        );
        consumer.consume(Event::DocumentStart { span }).unwrap();
        consumer.consume(Event::ObjectStart { span }).unwrap();
        let expected_json = emit(&mut consumer, span, &payload);
        consumer.consume(Event::ObjectEnd { span }).unwrap();
        consumer.consume(Event::DocumentEnd { span }).unwrap();
        let expected_value: tq_core::Value = serde_json::from_str(&expected_json).unwrap();
        let mut expected = b"\x1e".to_vec();
        crate::write_value(&mut expected, &expected_value, WriterConfig::default()).unwrap();
        expected.push(b'\n');
        assert_eq!(
            *observed
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
            expected,
        );
        let observations = arena.observations();
        assert!(observations.spool_bytes_written > 0);
        assert!(observations.spool_bytes_replayed > 0);
        assert!(observations.memory_high_water_bytes <= MEMORY_BUDGET);
    }

    #[test]
    fn wide_keyed_values_replay_from_spilled_tape() {
        assert_spilled_object(|consumer, span, payload| {
            for row in 0..12 {
                consumer
                    .consume(Event::Key {
                        span,
                        value: Arc::from(format!("row{row:02}")),
                        quoted: false,
                    })
                    .unwrap();
                consumer.consume(Event::ObjectStart { span }).unwrap();
                consumer
                    .consume(Event::Key {
                        span,
                        value: Arc::from("blob"),
                        quoted: false,
                    })
                    .unwrap();
                consumer
                    .consume(Event::Scalar {
                        span,
                        value: Scalar::String(Arc::from(payload)),
                    })
                    .unwrap();
                true_member(consumer, span, "flag");
                consumer.consume(Event::ObjectEnd { span }).unwrap();
            }
            let rows = (0..12)
                .map(|row| format!(r#""row{row:02}":{{"blob":"{payload}","flag":true}}"#))
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{rows}}}")
        });
    }

    #[test]
    fn generic_nested_values_replay_from_spilled_tape() {
        assert_spilled_object(|consumer, span, payload| {
            consumer
                .consume(Event::Key {
                    span,
                    value: Arc::from("outer"),
                    quoted: false,
                })
                .unwrap();
            consumer.consume(Event::ObjectStart { span }).unwrap();
            for index in 0..12 {
                consumer
                    .consume(Event::Key {
                        span,
                        value: Arc::from(format!("field{index:02}")),
                        quoted: false,
                    })
                    .unwrap();
                consumer
                    .consume(Event::Scalar {
                        span,
                        value: Scalar::String(Arc::from(payload)),
                    })
                    .unwrap();
            }
            consumer.consume(Event::ObjectEnd { span }).unwrap();
            true_member(consumer, span, "tail");
            format!(
                r#"{{"outer":{{{}}},"tail":true}}"#,
                (0..12)
                    .map(|index| format!(r#""field{index:02}":"{payload}""#))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        });
    }
    #[test]
    fn cancellation_keeps_prior_record_and_live_sequence_prefix() {
        let span = Span::new(SourceId::new(1), 0, 0);
        let sink = SharedSink::default();
        let observed = Arc::clone(&sink.0);
        let cancellation = Arc::new(AtomicBool::new(false));
        let mut consumer = TranscodeConsumer::new(
            sink,
            WriterConfig::default(),
            ArrayPreparationConfig::default(),
            PreparationArena::new(PreparationLimits::default()),
            DuplicateKeyPolicy::Reject,
            TranscodeCommitment::DirectSequence,
        )
        .with_cancellation(Arc::clone(&cancellation));
        consumer.consume(Event::DocumentStart { span }).unwrap();
        consumer
            .consume(Event::Scalar {
                span,
                value: Scalar::Bool(true),
            })
            .unwrap();
        consumer.consume(Event::DocumentEnd { span }).unwrap();
        consumer.consume(Event::DocumentStart { span }).unwrap();
        cancellation.store(true, Ordering::Relaxed);
        assert!(
            consumer
                .consume(Event::Scalar {
                    span,
                    value: Scalar::Bool(false),
                })
                .is_err()
        );
        assert_eq!(
            observed
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .as_slice(),
            b"\x1etrue\n\x1e"
        );
    }
    #[test]
    fn denied_tape_spill_preserves_only_live_record_prefix() {
        let span = Span::new(SourceId::new(1), 0, 0);
        let sink = SharedSink::default();
        let observed = Arc::clone(&sink.0);
        let mut consumer = TranscodeConsumer::new(
            sink,
            WriterConfig::default(),
            ArrayPreparationConfig {
                memory_threshold_bytes: 8,
                allow_spool: false,
                ..ArrayPreparationConfig::default()
            },
            PreparationArena::new(PreparationLimits {
                memory_bytes: 16 * 1024,
                ..PreparationLimits::default()
            }),
            DuplicateKeyPolicy::Reject,
            TranscodeCommitment::DirectSequence,
        );
        consumer.consume(Event::DocumentStart { span }).unwrap();
        assert!(
            consumer
                .consume(Event::Scalar {
                    span,
                    value: Scalar::String(Arc::from("x".repeat(4096))),
                })
                .is_err()
        );
        assert_eq!(
            observed
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .as_slice(),
            b"\x1e"
        );
        drop(consumer);
    }
}
