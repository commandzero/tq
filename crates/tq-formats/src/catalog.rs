//! Closed native-format metadata, independent of query planning.

use crate::{InputFormat, OutputFormat};

/// Syntax of one document, independent of sequence framing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentFormat {
    /// Delimited scalar fields in a logical row.
    Delimited,
    /// JSON syntax.
    Json,
    /// JSON5 syntax with tq's documented multiline strings.
    Json5,
    /// YAML syntax.
    Yaml,
    /// TOON syntax.
    Toon,
}

/// Rules separating neighboring documents.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Framing {
    /// Header followed by quote-aware logical rows.
    DelimitedRows,
    /// One standalone document.
    Single,
    /// Whitespace-separated JSON roots.
    JsonValues,
    /// One root per physical line.
    Lines,
    /// YAML document markers.
    YamlDocuments,
    /// Record-separator boundaries.
    RecordSeparator,
}

/// Mapping between native syntax and ordered tq values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FormatProfile {
    /// Header-shaped objects with typed scalar fields.
    Delimited,
    /// Ordered exact JSON-shaped values.
    Json,
    /// Finite JSON5 values normalized into JSON-shaped values.
    Json5,
    /// YAML's supported JSON-shaped subset.
    Yaml,
    /// Canonical TOON values.
    Toon,
}

/// A concrete native format. Automatic detection is not a format.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeFormat {
    /// Comma-separated row documents.
    Csv,
    /// Tab-separated row documents.
    Tsv,
    /// Standalone TOON input, configurable TOON output framing.
    Toon,
    /// YAML documents.
    Yaml,
    /// JSON roots.
    Json,
    /// One JSON5 document, input only.
    Json5,
    /// JSON Lines / NDJSON.
    JsonLines,
    /// RS-framed TOON documents.
    ToonSequence,
    /// RS-framed JSON recovery segments.
    JsonSequence,
}

/// Directional capabilities and spellings owned by the format catalog.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormatDescriptor {
    /// Canonical selector.
    pub name: &'static str,
    /// Additional accepted selectors.
    pub aliases: &'static [&'static str],
    /// Recognized filename extensions, without dots.
    pub extensions: &'static [&'static str],
    /// Concrete input selector.
    pub input: InputFormat,
    /// Output selector, absent for input-only formats.
    pub output: Option<OutputFormat>,
    /// Per-document syntax.
    pub document: DocumentFormat,
    /// Input framing.
    pub framing: Framing,
    /// Input mapping.
    pub input_profile: FormatProfile,
    /// Output mapping, absent for input-only formats.
    pub output_profile: Option<FormatProfile>,
    /// Bounded content probing can select this format.
    pub probeable: bool,
    /// Structural events can be decoded without materializing the document.
    pub events: bool,
    /// Compatible output option family, absent for input-only formats.
    pub output_controls: Option<OutputControls>,
}

/// Format-specific output controls accepted before input is consumed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputControls {
    /// Scalar row encoding without JSON or TOON controls.
    Delimited,
    /// TOON indentation, folding, and framing.
    Toon,
    /// JSON indentation, ASCII escaping, and color.
    Json,
    /// Compact JSON with ASCII escaping, without color or indentation.
    JsonLines,
    /// YAML document encoding.
    Yaml,
}

impl NativeFormat {
    /// All supported concrete native formats in help-display order.
    pub const ALL: [Self; 9] = [
        Self::Toon,
        Self::Yaml,
        Self::Json,
        Self::Json5,
        Self::JsonLines,
        Self::ToonSequence,
        Self::JsonSequence,
        Self::Csv,
        Self::Tsv,
    ];

    /// Looks up an exact command-line selector or alias.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|format| {
            let descriptor = format.descriptor();
            descriptor.name == name || descriptor.aliases.contains(&name)
        })
    }

    /// Looks up a filename extension without guessing its contents.
    #[must_use]
    pub fn from_extension(extension: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|format| {
            format
                .descriptor()
                .extensions
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(extension))
        })
    }

    /// Converts a committed input selection; automatic input is not committed.
    #[must_use]
    pub const fn from_input(format: InputFormat) -> Option<Self> {
        Some(match format {
            InputFormat::Csv => Self::Csv,
            InputFormat::Tsv => Self::Tsv,
            InputFormat::Auto => return None,
            InputFormat::Toon => Self::Toon,
            InputFormat::Yaml => Self::Yaml,
            InputFormat::Json => Self::Json,
            InputFormat::Json5 => Self::Json5,
            InputFormat::JsonLines => Self::JsonLines,
            InputFormat::ToonSequence => Self::ToonSequence,
            InputFormat::JsonSequence => Self::JsonSequence,
        })
    }

    /// Converts an output syntax; framing options refine TOON selection.
    #[must_use]
    pub const fn from_output(format: OutputFormat) -> Self {
        match format {
            OutputFormat::Csv => Self::Csv,
            OutputFormat::Tsv => Self::Tsv,
            OutputFormat::JsonSequence => Self::JsonSequence,
            OutputFormat::Toon => Self::Toon,
            OutputFormat::Yaml => Self::Yaml,
            OutputFormat::Json => Self::Json,
            OutputFormat::JsonLines => Self::JsonLines,
        }
    }

    /// Stable report spelling, preserving the pre-catalog TOON sequence label.
    #[must_use]
    pub const fn report_name(self) -> &'static str {
        match self {
            Self::ToonSequence => "toon-sequence",
            _ => self.descriptor().name,
        }
    }

    /// Returns complete metadata through exhaustive matching.
    #[must_use]
    #[allow(
        clippy::too_many_lines,
        reason = "keep the exhaustive format table in one place"
    )]
    pub const fn descriptor(self) -> FormatDescriptor {
        match self {
            Self::Csv | Self::Tsv => FormatDescriptor {
                name: if matches!(self, Self::Csv) {
                    "csv"
                } else {
                    "tsv"
                },
                aliases: &[],
                extensions: if matches!(self, Self::Csv) {
                    &["csv"]
                } else {
                    &["tsv"]
                },
                input: if matches!(self, Self::Csv) {
                    InputFormat::Csv
                } else {
                    InputFormat::Tsv
                },
                output: Some(if matches!(self, Self::Csv) {
                    OutputFormat::Csv
                } else {
                    OutputFormat::Tsv
                }),
                document: DocumentFormat::Delimited,
                framing: Framing::DelimitedRows,
                input_profile: FormatProfile::Delimited,
                output_profile: Some(FormatProfile::Delimited),
                probeable: false,
                events: false,
                output_controls: Some(OutputControls::Delimited),
            },
            Self::JsonSequence => FormatDescriptor {
                name: "json-seq",
                aliases: &["jsonseq"],
                extensions: &["json-seq", "jsonseq"],
                input: InputFormat::JsonSequence,
                output: Some(OutputFormat::JsonSequence),
                document: DocumentFormat::Json,
                framing: Framing::RecordSeparator,
                input_profile: FormatProfile::Json,
                output_profile: Some(FormatProfile::Json),
                probeable: true,
                events: true,
                output_controls: Some(OutputControls::Json),
            },
            Self::Toon => FormatDescriptor {
                name: "toon",
                aliases: &[],
                extensions: &["toon"],
                input: InputFormat::Toon,
                output: Some(OutputFormat::Toon),
                document: DocumentFormat::Toon,
                framing: Framing::Single,
                input_profile: FormatProfile::Toon,
                output_profile: Some(FormatProfile::Toon),
                probeable: true,
                events: true,
                output_controls: Some(OutputControls::Toon),
            },
            Self::Yaml => FormatDescriptor {
                name: "yaml",
                aliases: &["yml"],
                extensions: &["yaml", "yml"],
                input: InputFormat::Yaml,
                output: Some(OutputFormat::Yaml),
                document: DocumentFormat::Yaml,
                framing: Framing::YamlDocuments,
                input_profile: FormatProfile::Yaml,
                output_profile: Some(FormatProfile::Yaml),
                probeable: true,
                events: false,
                output_controls: Some(OutputControls::Yaml),
            },
            Self::Json => FormatDescriptor {
                name: "json",
                aliases: &[],
                extensions: &["json"],
                input: InputFormat::Json,
                output: Some(OutputFormat::Json),
                document: DocumentFormat::Json,
                framing: Framing::JsonValues,
                input_profile: FormatProfile::Json,
                output_profile: Some(FormatProfile::Json),
                probeable: true,
                events: true,
                output_controls: Some(OutputControls::Json),
            },
            Self::Json5 => FormatDescriptor {
                name: "json5",
                aliases: &[],
                extensions: &["json5"],
                input: InputFormat::Json5,
                output: None,
                document: DocumentFormat::Json5,
                framing: Framing::Single,
                input_profile: FormatProfile::Json5,
                output_profile: None,
                probeable: false,
                events: false,
                output_controls: None,
            },
            Self::JsonLines => FormatDescriptor {
                name: "jsonl",
                aliases: &["ndjson"],
                extensions: &["jsonl", "ndjson"],
                input: InputFormat::JsonLines,
                output: Some(OutputFormat::JsonLines),
                document: DocumentFormat::Json,
                framing: Framing::Lines,
                input_profile: FormatProfile::Json,
                output_profile: Some(FormatProfile::Json),
                probeable: false,
                events: true,
                output_controls: Some(OutputControls::JsonLines),
            },
            Self::ToonSequence => FormatDescriptor {
                name: "toon-seq",
                aliases: &["toon-sequence"],
                extensions: &[],
                input: InputFormat::ToonSequence,
                output: Some(OutputFormat::Toon),
                document: DocumentFormat::Toon,
                framing: Framing::RecordSeparator,
                input_profile: FormatProfile::Toon,
                output_profile: Some(FormatProfile::Toon),
                probeable: false,
                events: false,
                output_controls: Some(OutputControls::Toon),
            },
        }
    }
}
