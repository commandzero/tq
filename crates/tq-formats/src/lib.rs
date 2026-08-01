//! Ordered JSON, YAML, and TOON document-source adapters for `tq`.

use std::io;

use thiserror::Error;
use tq_core::{Diagnostic, Value};

/// Supported structured input syntax.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputFormat {
    /// Best-effort TOON, then YAML, then JSON probing.
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
}
