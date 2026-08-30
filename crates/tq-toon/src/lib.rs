//! Incremental TOON input and canonical output support for `tq`.

use std::{fmt, sync::Arc};

use thiserror::Error;
use tq_core::{Number, SourcePosition, Span};

mod decoder;
mod dom;
mod replay;
mod sequence;
mod spool;
mod transcode;
mod writer;

pub use decoder::{DecodeIntoError, Decoder};
pub use dom::{DomBuilder, DomDecodeError, DomError, decode_to_value};
pub use sequence::{CardinalityError, SequenceError, write_sequence, write_unframed};
pub use spool::{
    ArrayPreparationConfig, PreparationArena, PreparationFrame, PreparationLimits,
    PreparationMemory, PreparationObservations, PreparedArray, PreparedKeySet, PreparedObject,
    PublicationBuffer, PublicationError, SpoolError,
};
pub use transcode::{TranscodeCommitment, TranscodeConsumer, TranscodeError};
pub use writer::{Delimiter, KeyFolding, WriterConfig, WriterError, encode, write_value};

/// Bounded decoder configuration. Declared collection lengths never directly
/// become allocation capacities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecoderConfig {
    /// Spaces in one indentation level.
    pub indent_size: usize,
    /// Enforce counts, indentation, delimiters, and blank-line rules.
    pub strict: bool,
    /// Safe dotted-key expansion policy used by DOM consumers.
    pub path_expansion: PathExpansion,
    /// Maximum structural nesting.
    pub maximum_depth: usize,
    /// Maximum bytes in one scalar/key token.
    pub maximum_token_bytes: usize,
    /// Maximum bytes in one physical line.
    pub maximum_line_bytes: usize,
    /// Maximum bounded detection lookahead.
    pub maximum_lookahead_bytes: usize,
}

impl Default for DecoderConfig {
    fn default() -> Self {
        Self {
            indent_size: 2,
            strict: true,
            path_expansion: PathExpansion::Off,
            maximum_depth: 256,
            maximum_token_bytes: 8 * 1024 * 1024,
            maximum_line_bytes: 16 * 1024 * 1024,
            maximum_lookahead_bytes: 64 * 1024,
        }
    }
}

/// Dotted object-key expansion mode for materialized values.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PathExpansion {
    /// Preserve decoded keys literally.
    #[default]
    Off,
    /// Expand unquoted dotted keys only when every segment is an identifier.
    Safe,
}

/// Scalar emitted by the query-independent TOON decoder boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Scalar {
    /// Null.
    Null,
    /// Boolean.
    Bool(bool),
    /// Exact-literal hybrid number.
    Number(Number),
    /// Shared decoded string.
    String(Arc<str>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ScalarToken<'a> {
    Null,
    Bool(bool),
    Number(&'a str),
    String(&'a str),
}

/// Source-spanned structural decoder event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Event {
    /// Document boundary.
    DocumentStart {
        /// Source range establishing the document.
        span: Span,
    },
    /// Successful document end.
    DocumentEnd {
        /// Source range at completion.
        span: Span,
    },
    /// Object boundary.
    ObjectStart {
        /// Source range establishing the object.
        span: Span,
    },
    /// Object end.
    ObjectEnd {
        /// Source range at completion.
        span: Span,
    },
    /// Object key in encounter order.
    Key {
        /// Key token range.
        span: Span,
        /// Decoded key.
        value: Arc<str>,
        /// Whether the source key was explicitly quoted.
        quoted: bool,
    },
    /// Array boundary with optional declared count.
    ArrayStart {
        /// Header range.
        span: Span,
        /// Optional count declared by TOON syntax.
        declared_count: Option<u64>,
    },
    /// Array end and observed item count.
    ArrayEnd {
        /// Source range at completion.
        span: Span,
        /// Items actually consumed.
        observed_count: u64,
    },
    /// Primitive value.
    Scalar {
        /// Scalar token range.
        span: Span,
        /// Decoded scalar.
        value: Scalar,
    },
}

/// Query-independent event consumer suitable for later extraction to
/// `toon-rust` after this contract stabilizes.
pub trait EventConsumer {
    /// Consumer error.
    type Error;

    /// Consumes one ordered decoder event.
    ///
    /// # Errors
    ///
    /// Returns the consumer's bounded processing failure.
    fn consume(&mut self, event: Event) -> Result<(), Self::Error>;

    /// Consumes a decoded object key before the decoder converts it to shared
    /// event storage.
    ///
    /// # Errors
    ///
    /// Returns the consumer failure as text.
    fn consume_text_key(&mut self, span: Span, value: String, quoted: bool) -> Result<(), String>
    where
        Self::Error: fmt::Display,
    {
        self.consume(Event::Key {
            span,
            value: Arc::from(value),
            quoted,
        })
        .map_err(|error| error.to_string())
    }

    /// Consumes null without constructing a scalar event.
    ///
    /// # Errors
    ///
    /// Returns the consumer failure as text.
    fn consume_null(&mut self, span: Span) -> Result<(), String>
    where
        Self::Error: fmt::Display,
    {
        self.consume(Event::Scalar {
            span,
            value: Scalar::Null,
        })
        .map_err(|error| error.to_string())
    }

    /// Consumes a Boolean without constructing a scalar event.
    ///
    /// # Errors
    ///
    /// Returns the consumer failure as text.
    fn consume_bool(&mut self, span: Span, value: bool) -> Result<(), String>
    where
        Self::Error: fmt::Display,
    {
        self.consume(Event::Scalar {
            span,
            value: Scalar::Bool(value),
        })
        .map_err(|error| error.to_string())
    }

    /// Consumes decoded string text before the decoder converts it to shared
    /// event storage.
    ///
    /// # Errors
    ///
    /// Returns the consumer failure as text.
    fn consume_text_string(&mut self, span: Span, value: String) -> Result<(), String>
    where
        Self::Error: fmt::Display,
    {
        self.consume(Event::Scalar {
            span,
            value: Scalar::String(Arc::from(value)),
        })
        .map_err(|error| error.to_string())
    }

    /// Consumes a JSON numeric literal before the decoder constructs a runtime
    /// number. The default adapter preserves the owned-event contract.
    ///
    /// # Errors
    ///
    /// Returns numeric admission or consumer failure as text.
    fn consume_number_literal(&mut self, span: Span, literal: String) -> Result<(), String>
    where
        Self::Error: fmt::Display,
    {
        let value = Number::parse(&literal).map_err(|error| error.to_string())?;
        self.consume(Event::Scalar {
            span,
            value: Scalar::Number(value),
        })
        .map_err(|error| error.to_string())
    }
}

/// Decoder behavior that controls safe structural transcode commitment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecoderCapabilities {
    /// Object duplicate-key behavior.
    pub duplicate_keys: DuplicateKeyPolicy,
    /// Availability of declared array lengths before their elements.
    pub array_lengths: ArrayLengthCapability,
    /// Whether syntax failure can follow already emitted structural events.
    pub late_errors_after_events: bool,
}

impl DecoderCapabilities {
    /// Strict TOON decoder behavior.
    #[must_use]
    pub const fn strict_toon() -> Self {
        Self {
            duplicate_keys: DuplicateKeyPolicy::Reject,
            array_lengths: ArrayLengthCapability::Required,
            late_errors_after_events: true,
        }
    }

    /// jq-compatible JSON decoder behavior.
    #[must_use]
    pub const fn json() -> Self {
        Self {
            duplicate_keys: DuplicateKeyPolicy::LastValueFirstPosition,
            array_lengths: ArrayLengthCapability::Unknown,
            late_errors_after_events: true,
        }
    }
}

/// Object duplicate handling exposed before semantic decoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DuplicateKeyPolicy {
    /// Reject a repeated key.
    Reject,
    /// Keep the final value at the key's first encounter position.
    LastValueFirstPosition,
}

/// Array length information available at the opening event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArrayLengthCapability {
    /// The decoder cannot declare the final length at array start.
    Unknown,
    /// Every accepted array declares its final length.
    Required,
}

/// Strict TOON decoder failure.
#[derive(Debug, Error)]
pub enum DecodeError {
    /// Syntax violation at the best available source position.
    #[error("invalid TOON at {position:?}: {message}")]
    Syntax {
        /// Source position.
        position: SourcePosition,
        /// Concise reason.
        message: Arc<str>,
    },
    /// Configured resource limit.
    #[error("TOON decoder resource limit exceeded: {resource}")]
    Resource {
        /// Stable resource name.
        resource: Arc<str>,
    },
    /// Underlying reader failed.
    #[error("TOON input I/O failed: {message}")]
    Io {
        /// Reader error without platform backtrace/noise.
        message: Arc<str>,
    },
}
