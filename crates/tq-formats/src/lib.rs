//! Ordered JSON, YAML, and TOON document-source adapters for `tq`.

use std::{collections::VecDeque, io};

use thiserror::Error;
use tq_core::{Diagnostic, Value};

mod adapters;
mod output;
mod stream;

pub use adapters::{
    DecodeOptions, ProbeReport, ReplayReader, VecDocumentSource, decode_bytes, decode_json,
    decode_toon, decode_toon_sequence, decode_yaml, probe_format, probe_reader,
};
pub use output::{JsonIndent, OutputError, OutputOptions, ToonFraming, write_results};
pub use stream::{StreamOptions, stream_json, stream_toon};

/// Supported structured input syntax.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputFormat {
    /// Bounded TOON, JSON-container, then YAML syntax probing.
    Auto,
    /// TOON document.
    Toon,
    /// YAML document stream.
    Yaml,
    /// JSON text.
    Json,
    /// Record Separator framed TOON sequence.
    ToonSequence,
}

/// Structured output syntax.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputFormat {
    /// Canonical TOON or TOON Text Sequence.
    Toon,
    /// Compact or pretty JSON result text.
    Json,
    /// One YAML document per result.
    Yaml,
}

/// One decoded document and its source identity.
#[derive(Clone, Debug)]
pub struct Document {
    /// Ordered runtime value.
    pub value: Value,
    /// User-visible source name.
    pub identity: String,
    /// Committed parser.
    pub format: InputFormat,
    /// Zero-based document index for multi-document sources.
    pub index: u64,
}

/// Pull-based source that never requires all documents to be retained.
pub trait DocumentSource {
    /// Returns the next document or end of source.
    ///
    /// # Errors
    ///
    /// Returns a stable input or resource diagnostic.
    fn next_document(&mut self) -> Result<Option<Document>, FormatError>;
}

/// Structured-adapter failure.
#[derive(Debug, Error)]
pub enum FormatError {
    /// Underlying byte I/O.
    #[error("input I/O failed: {0}")]
    Io(#[from] io::Error),
    /// Stable source-spanned format diagnostic.
    #[error(transparent)]
    Diagnostic(#[from] Box<Diagnostic>),
    /// One selected parser rejected its input.
    #[error("{format:?} input rejected: {message}")]
    Parse {
        /// Parser that rejected the input.
        format: InputFormat,
        /// Bounded parser context.
        message: String,
    },
    /// Automatic probing exhausted all candidates.
    #[error("input rejected by TOON, YAML, and JSON: {summary}")]
    Probe {
        /// Bounded combined rejection context in probe order.
        summary: String,
    },
    /// YAML contains a value outside tq's JSON-shaped profile.
    #[error("unsupported YAML value: {0}")]
    UnsupportedYaml(String),
    /// A source exceeds its configured byte limit.
    #[error("input resource limit exceeded: {0}")]
    Resource(&'static str),
}
