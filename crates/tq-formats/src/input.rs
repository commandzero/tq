//! Native input ownership after format commitment.

use std::{
    cell::RefCell,
    collections::VecDeque,
    io::{self, BufReader, Read},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use crate::adapters::{JsonDocumentSource, JsonLinesDocumentSource};
use crate::{DecodeOptions, Document, FormatError, NativeFormat, decode_bytes};
use std::ops::ControlFlow;
use tq_toon::EventConsumer;

/// Representation requested before semantic input is consumed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputRepresentation {
    /// Pull complete root values on demand.
    Documents,
    /// Push structural events without materializing the complete root.
    Events,
}

/// A native input selection with no automatic or unsupported representation.
#[derive(Clone, Copy, Debug)]
pub struct SelectedInput {
    format: NativeFormat,
    options: DecodeOptions,
    representation: InputRepresentation,
}

impl NativeFormat {
    /// Commits a format and verifies its requested representation.
    ///
    /// # Errors
    ///
    /// Rejects unsupported structural-event decoding before opening input.
    pub fn select_input(
        self,
        mut options: DecodeOptions,
        representation: InputRepresentation,
    ) -> Result<SelectedInput, FormatError> {
        if representation == InputRepresentation::Events && !self.descriptor().events {
            return Err(FormatError::Parse {
                format: self.descriptor().input,
                message: "selected format cannot supply structural events".to_owned(),
            });
        }
        options.format = self.descriptor().input;
        options.toon.maximum_depth = options.toon.maximum_depth.min(options.maximum_depth);
        options.toon.maximum_token_bytes = options
            .toon
            .maximum_token_bytes
            .min(options.maximum_token_bytes);
        options.toon.maximum_line_bytes = options
            .toon
            .maximum_line_bytes
            .min(options.maximum_line_bytes);
        Ok(SelectedInput {
            format: self,
            options,
            representation,
        })
    }
}

/// An ordered observation from committed native input.
#[derive(Debug)]
pub enum NativeInputObservation {
    /// A published complete Document.
    Document(Document),
    /// One published structural event.
    Event(tq_toon::Event),
    /// A rejected document followed by an unambiguous recovery boundary.
    Failure(NativeInputFailure),
}

/// Source context for an ordered recoverable document failure.
#[derive(Debug)]
pub struct NativeInputFailure {
    /// User-visible source name.
    pub identity: String,
    /// Zero-based RS segment index, independent of successful documents.
    pub recovery_segment_index: u64,
    /// Index assigned to the next published complete document.
    pub document_index: u64,
    /// Decoder diagnostic, without warning rendering or output policy.
    pub message: String,
}

type SharedFrames<R> = Rc<RefCell<crate::rs_framing::RsFramer<BufReader<BoundedInput<R>>>>>;

struct SegmentReader<R>(SharedFrames<R>);

impl<R: Read> Read for SegmentReader<R> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        self.0.borrow_mut().read(output)
    }
}

struct JsonSequenceInput<R: Read> {
    frames: SharedFrames<R>,
    current: Option<crate::json_recovery::JsonRecoveryDecoder<SegmentReader<R>>>,
    segment_index: u64,
    document_index: u64,
    maximum_depth: usize,
    maximum_token_bytes: usize,
    representation: InputRepresentation,
}

impl<R: Read> JsonSequenceInput<R> {
    fn next(&mut self, identity: &str) -> Result<Option<NativeInputObservation>, FormatError> {
        loop {
            if let Some(current) = &mut self.current {
                let mut result = if self.representation == InputRepresentation::Events {
                    current
                        .next_event()
                        .map(|event| event.map(NativeInputObservation::Event))
                } else {
                    current.next_value().map(|value| {
                        value.map(|value| {
                            NativeInputObservation::Document(Document {
                                value,
                                identity: identity.to_owned(),
                                format: crate::InputFormat::JsonSequence,
                                index: self.document_index,
                            })
                        })
                    })
                };
                if self.frames.borrow().exceeded() {
                    return Err(FormatError::Resource("frame-bytes"));
                }
                let ended = current.ended();
                let at_separator = ended && self.frames.borrow_mut().at_separator()?;
                if at_separator && matches!(&result, Ok(Some(_))) {
                    result = Err(FormatError::Parse {
                        format: crate::InputFormat::JsonSequence,
                        message: "Truncated value".to_owned(),
                    });
                }
                match result {
                    Ok(Some(observation)) => {
                        if matches!(
                            observation,
                            NativeInputObservation::Document(_)
                                | NativeInputObservation::Event(tq_toon::Event::DocumentEnd { .. })
                        ) {
                            self.document_index = self.document_index.saturating_add(1);
                        }
                        return Ok(Some(observation));
                    }
                    Ok(None) => self.current = None,
                    Err(FormatError::Parse { message, .. }) => {
                        if at_separator {
                            self.current = None;
                        }
                        let (line, column) = self.frames.borrow().location();
                        let message = if at_separator {
                            let reason = if message.starts_with("Potentially truncated") {
                                message.as_str()
                            } else {
                                "Truncated value"
                            };
                            format!("{reason} at line {line}, column {}", column + 1)
                        } else if ended {
                            format!("{message} at EOF at line {line}, column {column}")
                        } else {
                            format!("{message} at line {line}, column {column} (need RS to resync)")
                        };
                        return Ok(Some(NativeInputObservation::Failure(NativeInputFailure {
                            identity: identity.to_owned(),
                            recovery_segment_index: self.segment_index,
                            document_index: self.document_index,
                            message,
                        })));
                    }
                    Err(error) => return Err(error),
                }
            }
            let Some(index) = self.frames.borrow_mut().begin_segment()? else {
                return Ok(None);
            };
            self.segment_index = index;
            let decoder = crate::json_recovery::JsonRecoveryDecoder::new(
                SegmentReader(Rc::clone(&self.frames)),
                self.maximum_depth,
                self.maximum_token_bytes,
            );
            self.current = Some(if self.representation == InputRepresentation::Events {
                decoder.events_only()
            } else {
                decoder
            });
        }
    }
}

/// Fatal input failure remains distinct from a caller's processing failure.
#[derive(Debug)]
pub enum InputDeliveryError<E> {
    /// Source, decoder, or resource failure.
    Input(FormatError),
    /// The caller stopped processing with its own error.
    Consumer(E),
}

enum InputState<R: Read> {
    Delimited(crate::delimited_input::DelimitedInput<BufReader<BoundedInput<R>>>),
    JsonSequence(JsonSequenceInput<R>),
    Json(JsonDocumentSource<BufReader<BoundedInput<R>>>),
    JsonLines(JsonLinesDocumentSource<BufReader<BoundedInput<R>>>),
    ToonSequence(crate::rs_framing::RsFramer<BufReader<BoundedInput<R>>>),
    Materialized {
        reader: Option<BoundedInput<R>>,
        documents: VecDeque<Document>,
    },
    Events(BoundedInput<R>),
}

/// One source's decoder, identity, resource controls, and publication state.
pub struct CommittedInput<R: Read> {
    selection: SelectedInput,
    identity: String,
    state: InputState<R>,
    terminal: bool,
    exceeded: Arc<AtomicBool>,
}

impl SelectedInput {
    /// Attaches source bytes without reading them.
    pub fn open<R: Read>(self, reader: R, identity: impl Into<String>) -> CommittedInput<R> {
        let identity = identity.into();
        let exceeded = Arc::new(AtomicBool::new(false));
        let reader = BoundedInput {
            reader,
            remaining: self.options.maximum_source_bytes,
            exceeded: Arc::clone(&exceeded),
        };
        let state = if self.representation == InputRepresentation::Events
            && self.format != NativeFormat::JsonSequence
        {
            InputState::Events(reader)
        } else {
            match self.format {
                NativeFormat::Csv | NativeFormat::Tsv => {
                    InputState::Delimited(crate::delimited_input::DelimitedInput::new(
                        BufReader::new(reader),
                        identity.clone(),
                        self.options,
                    ))
                }
                NativeFormat::JsonSequence => InputState::JsonSequence(JsonSequenceInput {
                    frames: Rc::new(RefCell::new(crate::rs_framing::RsFramer::with_policy(
                        BufReader::new(reader),
                        self.options.maximum_frame_bytes,
                        crate::rs_framing::RsPolicy::JsonRecovery,
                    ))),
                    current: None,
                    segment_index: 0,
                    document_index: 0,
                    maximum_depth: self.options.maximum_depth,
                    maximum_token_bytes: self.options.maximum_token_bytes,
                    representation: self.representation,
                }),
                NativeFormat::Json => InputState::Json(JsonDocumentSource::new(
                    BufReader::with_capacity(64 * 1024, reader),
                    identity.clone(),
                    self.options,
                )),
                NativeFormat::JsonLines => InputState::JsonLines(JsonLinesDocumentSource::new(
                    BufReader::new(reader),
                    identity.clone(),
                    self.options,
                )),
                NativeFormat::ToonSequence => {
                    InputState::ToonSequence(crate::rs_framing::RsFramer::new(
                        BufReader::new(reader),
                        self.options.maximum_frame_bytes,
                    ))
                }
                NativeFormat::Yaml | NativeFormat::Json5 | NativeFormat::Toon => {
                    InputState::Materialized {
                        reader: Some(reader),
                        documents: VecDeque::new(),
                    }
                }
            }
        };
        CommittedInput {
            selection: self,
            identity,
            state,
            terminal: false,
            exceeded,
        }
    }
}

impl<R: Read> CommittedInput<R> {
    /// Sends JSON or TOON events directly to a codec consumer, retaining its
    /// allocation-avoiding scalar/key hooks and explicit source spans.
    ///
    /// # Errors
    ///
    /// Rejects other representations before input, and separates native failures
    /// from the consumer trait's typed event errors and text-hook errors.
    pub fn consume_codec_events<C: EventConsumer>(
        self,
        source: tq_core::SourceId,
        consumer: &mut C,
    ) -> Result<(), InputDeliveryError<crate::CodecConsumerError<C::Error>>>
    where
        C::Error: std::fmt::Display,
    {
        let InputState::Events(reader) = self.state else {
            return Err(InputDeliveryError::Input(FormatError::Parse {
                format: self.selection.options.format,
                message: "direct codec consumption requires event input".to_owned(),
            }));
        };
        let result = crate::codec_input::consume(reader, self.selection.options, source, consumer);
        if matches!(result, Err(InputDeliveryError::Consumer(_))) {
            return result;
        }
        if self.exceeded.load(Ordering::Relaxed) {
            return Err(InputDeliveryError::Input(FormatError::Resource(
                "source-bytes",
            )));
        }
        result
    }

    /// Consumes selected structural records with native Document boundaries.
    ///
    /// Selection may skip unrelated values and use bounded parallel JSON batches.
    /// It is a decoder optimization, not a query execution plan.
    ///
    /// # Errors
    ///
    /// Rejects unsupported selections before reading input and preserves consumer errors.
    pub fn consume_selected<E>(
        self,
        selection: crate::StreamSelection,
        parallel: Option<crate::ParallelJsonOptions>,
        cancellation: Option<Arc<AtomicBool>>,
        consume: impl FnMut(crate::SelectedInputObservation) -> Result<ControlFlow<()>, E>,
    ) -> Result<crate::ParallelJsonObservations, InputDeliveryError<E>> {
        let InputState::Events(reader) = self.state else {
            return Err(InputDeliveryError::Input(FormatError::Parse {
                format: self.selection.options.format,
                message: "selected structural decoding requires event input".to_owned(),
            }));
        };
        let result = crate::selected_input::consume(
            reader,
            &self.identity,
            self.selection.options,
            selection,
            parallel,
            cancellation,
            consume,
        );
        if matches!(result, Err(InputDeliveryError::Consumer(_))) {
            return result;
        }
        if self.exceeded.load(Ordering::Relaxed) {
            return Err(InputDeliveryError::Input(FormatError::Resource(
                "source-bytes",
            )));
        }
        result
    }

    /// Pushes structural events until EOF, failure, or caller-directed stop.
    ///
    /// # Errors
    ///
    /// Distinguishes fatal input failures from the caller's error type.
    pub fn consume_events<E>(
        mut self,
        mut consume: impl FnMut(NativeInputObservation) -> Result<ControlFlow<()>, E>,
    ) -> Result<(), InputDeliveryError<E>> {
        if self.selection.representation == InputRepresentation::Events
            && let InputState::JsonSequence(source) = &mut self.state
        {
            loop {
                let observation = source.next(&self.identity);
                if self.exceeded.load(Ordering::Relaxed) {
                    return Err(InputDeliveryError::Input(FormatError::Resource(
                        "source-bytes",
                    )));
                }
                let Some(observation) = observation.map_err(InputDeliveryError::Input)? else {
                    return Ok(());
                };
                if consume(observation)
                    .map_err(InputDeliveryError::Consumer)?
                    .is_break()
                {
                    return Ok(());
                }
            }
        }
        let InputState::Events(reader) = self.state else {
            return Err(InputDeliveryError::Input(FormatError::Parse {
                format: self.selection.options.format,
                message: "document input was not selected for structural events".to_owned(),
            }));
        };
        let mut consumer = ObservationConsumer {
            consume,
            stopped: None,
        };
        let source = tq_core::SourceId::new(0);
        let options = crate::JsonEventOptions {
            maximum_depth: self.selection.options.maximum_depth,
            maximum_token_bytes: self.selection.options.maximum_token_bytes,
        };
        let result = match self.selection.format {
            NativeFormat::Json => crate::structural::decode_json_event_stream_classified(
                BufReader::with_capacity(64 * 1024, reader),
                source,
                &mut consumer,
                options,
            )
            .map(|_| ()),
            NativeFormat::JsonLines => {
                let mut lines = JsonLinesDocumentSource::new(
                    BufReader::new(reader),
                    &self.identity,
                    self.selection.options,
                );
                (|| {
                    while let Some((bytes, line)) = lines.next_record()? {
                        crate::structural::decode_json_events_classified(
                            bytes.as_slice(),
                            source,
                            &mut consumer,
                            options,
                        )
                        .map_err(|error| match error {
                            FormatError::Parse { message, .. } => FormatError::Parse {
                                format: self.selection.options.format,
                                message: format!("{}:{line}: {message}", self.identity),
                            },
                            error => error,
                        })?;
                    }
                    Ok(())
                })()
            }
            NativeFormat::Toon => {
                tq_toon::Decoder::new(BufReader::new(reader), source, self.selection.options.toon)
                    .decode_into(&mut consumer)
                    .map_err(|error| match error {
                        tq_toon::DecodeIntoError::Decode(error) => {
                            crate::adapters::toon_input_error(error)
                        }
                        tq_toon::DecodeIntoError::Consumer(error) => FormatError::Parse {
                            format: self.selection.options.format,
                            message: error.to_owned(),
                        },
                    })
            }
            NativeFormat::Json5
            | NativeFormat::Yaml
            | NativeFormat::ToonSequence
            | NativeFormat::JsonSequence
            | NativeFormat::Csv
            | NativeFormat::Tsv => {
                unreachable!("selection checked event support")
            }
        };
        if let Some(stopped) = consumer.stopped {
            return stopped.map_err(InputDeliveryError::Consumer);
        }
        if self.exceeded.load(Ordering::Relaxed) {
            return Err(InputDeliveryError::Input(FormatError::Resource(
                "source-bytes",
            )));
        }
        result.map_err(InputDeliveryError::Input)
    }

    /// Pulls the next complete Document observation, stopping after fatal failure.
    ///
    /// # Errors
    ///
    /// Returns input, resource, or representation errors.
    pub fn next_observation(&mut self) -> Result<Option<NativeInputObservation>, FormatError> {
        if self.terminal {
            return Ok(None);
        }
        self.terminal = true;
        if self.selection.representation == InputRepresentation::Events {
            return Err(FormatError::Parse {
                format: self.selection.options.format,
                message: "event input requires push consumption".to_owned(),
            });
        }
        let result = match &mut self.state {
            InputState::JsonSequence(source) => source.next(&self.identity),
            _ => self
                .next_document()
                .map(|document| document.map(NativeInputObservation::Document)),
        };
        if self.exceeded.load(Ordering::Relaxed) {
            return Err(FormatError::Resource("source-bytes"));
        }
        let observation = result?;
        // JSON and JSON Lines enforce these limits during parsing. Do not walk
        // their completed trees again, especially when most values are unused.
        if !matches!(
            self.selection.format,
            NativeFormat::Json | NativeFormat::JsonLines
        ) && let Some(NativeInputObservation::Document(document)) = &observation
        {
            crate::adapters::validate_document_value(&document.value, 0, self.selection.options)
                .map_err(FormatError::Resource)?;
        }
        self.terminal = observation.is_none();
        Ok(observation)
    }

    fn next_document(&mut self) -> Result<Option<Document>, FormatError> {
        let document = match &mut self.state {
            InputState::Delimited(source) => source.next_document()?,
            InputState::JsonSequence(_) => {
                unreachable!("sequence observations are handled separately")
            }
            InputState::Json(source) => source.next_document()?,
            InputState::JsonLines(source) => source.next_document()?,
            InputState::ToonSequence(source) => {
                let Some((index, bytes)) = source.next_segment()? else {
                    return Ok(None);
                };
                Some(crate::adapters::decode_toon_segment(
                    &bytes,
                    &self.identity,
                    self.selection.options.toon,
                    index,
                )?)
            }
            InputState::Materialized { reader, documents } => {
                if let Some(reader) = reader.take() {
                    let mut bytes = Vec::new();
                    reader
                        .take(
                            self.selection
                                .options
                                .maximum_source_bytes
                                .saturating_add(1) as u64,
                        )
                        .read_to_end(&mut bytes)?;
                    *documents =
                        decode_bytes(&bytes, &self.identity, self.selection.options)?.into();
                }
                documents.pop_front()
            }
            InputState::Events(_) => {
                return Err(FormatError::Parse {
                    format: self.selection.options.format,
                    message: "event input does not supply complete Document observations"
                        .to_owned(),
                });
            }
        };
        Ok(document)
    }
}

struct ObservationConsumer<F, E> {
    consume: F,
    stopped: Option<Result<(), E>>,
}

impl<F, E> EventConsumer for ObservationConsumer<F, E>
where
    F: FnMut(NativeInputObservation) -> Result<ControlFlow<()>, E>,
{
    type Error = &'static str;
    fn consume(&mut self, event: tq_toon::Event) -> Result<(), Self::Error> {
        match (self.consume)(NativeInputObservation::Event(event)) {
            Ok(ControlFlow::Continue(())) => Ok(()),
            Ok(ControlFlow::Break(())) => {
                self.stopped = Some(Ok(()));
                Err("input consumer stopped")
            }
            Err(error) => {
                self.stopped = Some(Err(error));
                Err("input consumer failed")
            }
        }
    }
}

struct BoundedInput<R> {
    reader: R,
    remaining: usize,
    exceeded: Arc<AtomicBool>,
}

impl<R: Read> Read for BoundedInput<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        if self.remaining == 0 {
            let mut probe = [0];
            if self.reader.read(&mut probe)? == 0 {
                return Ok(0);
            }
            self.exceeded.store(true, Ordering::Relaxed);
            return Err(io::Error::other("native input source-byte limit exceeded"));
        }
        let available = buffer.len().min(self.remaining);
        let count = self.reader.read(&mut buffer[..available])?;
        self.remaining -= count;
        Ok(count)
    }
}
