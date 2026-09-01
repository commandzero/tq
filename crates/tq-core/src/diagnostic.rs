//! Source identity and bounded, source-spanned diagnostics.

use std::{fmt, sync::Arc};

use serde::{Deserialize, Serialize};

/// Stable identity of a query or input source within one invocation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SourceId(u32);

impl SourceId {
    /// Creates an invocation-local source identity.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the underlying invocation-local integer.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Half-open byte range within one source.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Span {
    /// Source containing the range.
    pub source: SourceId,
    /// Inclusive UTF-8 byte offset.
    pub start: u64,
    /// Exclusive UTF-8 byte offset.
    pub end: u64,
}

impl Span {
    /// Creates a span, clamping an inverted end to the start.
    #[must_use]
    pub const fn new(source: SourceId, start: u64, end: u64) -> Self {
        Self {
            source,
            start,
            end: if end < start { start } else { end },
        }
    }
}

/// One-based human source position plus its byte offset.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourcePosition {
    /// Zero-based byte offset.
    pub byte: u64,
    /// One-based line number.
    pub line: u64,
    /// One-based Unicode-scalar column.
    pub column: u64,
}

/// Compact byte-to-line index shared by retained and transient source metadata.
#[derive(Clone, Debug)]
pub(crate) struct SourceLineIndex {
    starts: Arc<[u64]>,
}

impl SourceLineIndex {
    pub(crate) fn new(text: &str) -> Self {
        let mut starts = vec![0];
        for (index, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                starts.push(u64::try_from(index + 1).unwrap_or(u64::MAX));
            }
        }
        Self {
            starts: starts.into(),
        }
    }

    pub(crate) fn line(&self, byte: u64) -> u64 {
        u64::try_from(self.line_index(byte).saturating_add(1)).unwrap_or(u64::MAX)
    }

    fn line_index(&self, byte: u64) -> usize {
        self.starts
            .partition_point(|start| *start <= byte)
            .saturating_sub(1)
    }

    fn start(&self, line_index: usize) -> u64 {
        self.starts[line_index]
    }
}

/// Immutable source text with pre-indexed line starts.
#[derive(Clone, Debug)]
pub struct SourceFile {
    id: SourceId,
    name: Arc<str>,
    text: Arc<str>,
    line_index: SourceLineIndex,
}

impl SourceFile {
    /// Creates a source and indexes line starts without copying on clones.
    #[must_use]
    pub fn new(id: SourceId, name: impl Into<Arc<str>>, text: impl Into<Arc<str>>) -> Self {
        let text = text.into();
        let line_index = SourceLineIndex::new(&text);
        Self {
            id,
            name: name.into(),
            text,
            line_index,
        }
    }

    /// Source identity.
    #[must_use]
    pub const fn id(&self) -> SourceId {
        self.id
    }

    /// Display name or path.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Complete source text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn line_index(&self) -> SourceLineIndex {
        self.line_index.clone()
    }

    /// Maps a byte offset to a line and Unicode-scalar column.
    #[must_use]
    pub fn position(&self, byte: u64) -> SourcePosition {
        let byte = byte.min(u64::try_from(self.text.len()).unwrap_or(u64::MAX));
        let byte = u64::try_from(floor_char_boundary(
            &self.text,
            usize::try_from(byte).unwrap_or(self.text.len()),
        ))
        .unwrap_or(u64::MAX);
        let line_index = self.line_index.line_index(byte);
        let line_start = self.line_index.start(line_index);
        let start = usize::try_from(line_start).unwrap_or(self.text.len());
        let end = usize::try_from(byte).unwrap_or(self.text.len());
        let column = self.text[start..end].chars().count() as u64 + 1;
        SourcePosition {
            byte,
            line: line_index as u64 + 1,
            column,
        }
    }

    /// Renders bounded context around a span.
    #[must_use]
    pub fn render_context(&self, span: Span, maximum_chars: usize) -> String {
        if span.source != self.id || maximum_chars == 0 {
            return String::new();
        }
        let start = usize::try_from(span.start)
            .unwrap_or(self.text.len())
            .min(self.text.len());
        let end = usize::try_from(span.end)
            .unwrap_or(self.text.len())
            .min(self.text.len());
        let start = floor_char_boundary(&self.text, start);
        let end = floor_char_boundary(&self.text, end).max(start);
        let line_start = self.text[..start].rfind('\n').map_or(0, |index| index + 1);
        let line_end = self.text[end..]
            .find('\n')
            .map_or(self.text.len(), |index| end + index);
        let line = &self.text[line_start..line_end];
        if line.chars().count() <= maximum_chars {
            return line.to_owned();
        }
        line.chars().take(maximum_chars).collect::<String>() + "…"
    }
}

fn floor_char_boundary(text: &str, mut byte: usize) -> usize {
    while byte > 0 && !text.is_char_boundary(byte) {
        byte -= 1;
    }
    byte
}

/// Stable diagnostic family used by CLI exit classification and reports.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticClass {
    /// Invalid CLI use.
    Usage,
    /// Query lexing, parsing, resolution, analysis, or compilation.
    Compile,
    /// Input format or syntax failure.
    Input,
    /// Runtime value, type, or path failure.
    Runtime,
    /// Numeric policy failure.
    NumericRange,
    /// Configured resource limit.
    Resource,
    /// Stable deferred capability rejection.
    Unsupported,
    /// Interrupted or broken downstream I/O.
    Cancelled,
}

/// One primary or secondary diagnostic annotation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Label {
    /// Source range.
    pub span: Span,
    /// Short annotation.
    pub message: String,
    /// Whether this is the primary annotation.
    pub primary: bool,
}

/// Stable structured diagnostic.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Diagnostic {
    /// Stable machine code.
    pub code: String,
    /// Exit/report class.
    pub class: DiagnosticClass,
    /// Human summary.
    pub message: String,
    /// Source annotations.
    pub labels: Vec<Label>,
    /// Optional input document identity.
    pub input_identity: Option<String>,
    /// Optional selected input format.
    pub input_format: Option<String>,
    /// Optional jq path context.
    pub value_path: Option<String>,
}

impl Diagnostic {
    /// Constructs a diagnostic without source labels.
    #[must_use]
    pub fn new(
        code: impl Into<String>,
        class: DiagnosticClass,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            class,
            message: message.into(),
            labels: Vec::new(),
            input_identity: None,
            input_format: None,
            value_path: None,
        }
    }

    /// Adds a primary label.
    #[must_use]
    pub fn at(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label {
            span,
            message: message.into(),
            primary: true,
        });
        self
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for Diagnostic {}

#[cfg(test)]
mod tests {
    use super::{SourceFile, SourceId, Span};

    #[test]
    fn maps_unicode_positions_and_bounds_context() {
        let source = SourceFile::new(SourceId::new(3), "query", "one\ntwø three\n");
        assert_eq!(source.position(8).line, 2);
        assert_eq!(source.position(8).column, 4);
        assert_eq!(
            source.render_context(Span::new(source.id(), 4, 7), 6),
            "twø th…"
        );
    }
}
