//! Incremental TOON input and canonical output support for `tq`.

use std::sync::Arc;

use thiserror::Error;
use tq_core::{Number, SourcePosition, Span};

/// Bounded decoder configuration. Declared collection lengths never directly
/// become allocation capacities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecoderConfig {
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
            maximum_depth: 256,
            maximum_token_bytes: 8 * 1024 * 1024,
            maximum_line_bytes: 16 * 1024 * 1024,
            maximum_lookahead_bytes: 64 * 1024,
        }
    }
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
}

/// Strict TOON decoder failure.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum DecodeError {
    /// Syntax violation at the best available source position.
    #[error("invalid TOON at line {position_line}, column {position_column}: {message}")]
    Syntax {
        /// Source position.
        position: SourcePosition,
        /// Cached line for stable error formatting.
        position_line: u64,
        /// Cached column for stable error formatting.
        position_column: u64,
        /// Concise reason.
        message: Arc<str>,
    },
    /// Configured resource limit.
    #[error("TOON decoder resource limit exceeded: {resource}")]
    Resource {
        /// Stable resource name.
        resource: Arc<str>,
    },
}
