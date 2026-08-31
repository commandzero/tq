//! Shared structured-result output dispatch.

use std::{borrow::Borrow, io::Write};

use serde::Serialize;
use thiserror::Error;
use tq_core::Value;
use tq_toon::{SequenceError, WriterConfig, write_sequence, write_unframed};

use crate::OutputFormat;

/// TOON result framing choice.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ToonFraming {
    /// RS-prefix/LF-suffix TOON Text Sequence, including for one result.
    #[default]
    Sequence,
    /// Exactly one standalone TOON document.
    Unframed,
}

/// JSON pretty-print indentation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JsonIndent {
    /// A reviewed number of spaces per nesting level.
    Spaces(u8),
    /// One tab per nesting level.
    Tabs,
}

impl Default for JsonIndent {
    fn default() -> Self {
        Self::Spaces(2)
    }
}

/// Structured output controls.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "independent wire-format controls are assembled after CLI validation"
)]
pub struct OutputOptions {
    /// Selected structured syntax.
    pub format: OutputFormat,
    /// Pretty JSON rather than compact JSON.
    pub pretty_json: bool,
    /// Pretty JSON indentation.
    pub json_indent: JsonIndent,
    /// Escape non-ASCII JSON codepoints.
    pub ascii_json: bool,
    /// Wrap JSON output in tq's deterministic ANSI color.
    pub color_json: bool,
    /// Prefix this YAML value with an explicit document separator.
    pub yaml_document_start: bool,
    /// TOON framing mode.
    pub toon_framing: ToonFraming,
    /// Canonical TOON options.
    pub toon: WriterConfig,
}

impl Default for OutputOptions {
    fn default() -> Self {
        Self {
            format: OutputFormat::Toon,
            pretty_json: false,
            json_indent: JsonIndent::default(),
            ascii_json: false,
            color_json: false,
            yaml_document_start: false,
            toon_framing: ToonFraming::Sequence,
            toon: WriterConfig::default(),
        }
    }
}

/// Structured output failure.
#[derive(Debug, Error)]
pub enum OutputError {
    /// TOON framing/cardinality or output error.
    #[error(transparent)]
    Toon(#[from] SequenceError),
    /// JSON serialization or output error.
    #[error("JSON output failed: {0}")]
    Json(#[from] serde_json::Error),
    /// YAML scalar serialization failure.
    #[error("YAML output failed: {0}")]
    Yaml(#[from] yaml_serde::Error),
    /// Direct output I/O error.
    #[error("structured output failed: {0}")]
    Io(#[from] std::io::Error),
}

impl OutputError {
    /// Whether output stopped because the downstream reader closed its pipe.
    #[must_use]
    pub fn is_broken_pipe(&self) -> bool {
        match self {
            Self::Toon(SequenceError::Io(error)) | Self::Io(error) => {
                error.kind() == std::io::ErrorKind::BrokenPipe
            }
            Self::Json(error) => error.io_error_kind() == Some(std::io::ErrorKind::BrokenPipe),
            Self::Yaml(_) | Self::Toon(SequenceError::Cardinality(_)) => false,
        }
    }
}

/// Writes ordered results in the selected structured format.
///
/// JSON emits one jq-compatible JSON text plus LF per result. TOON defaults to
/// explicit text-sequence framing and supports exactly-one unframed output.
///
/// # Errors
///
/// Returns serialization, framing/cardinality, or output I/O failures.
pub fn write_results<W, I, V>(
    mut writer: W,
    values: I,
    options: OutputOptions,
) -> Result<(), OutputError>
where
    W: Write,
    I: IntoIterator<Item = V>,
    V: Borrow<Value>,
{
    match options.format {
        OutputFormat::Toon => match options.toon_framing {
            ToonFraming::Sequence => write_sequence(writer, values, options.toon)?,
            ToonFraming::Unframed => write_unframed(writer, values, options.toon)?,
        },
        OutputFormat::Json => {
            for value in values {
                let mut encoded = Vec::new();
                if options.pretty_json {
                    let indentation = match options.json_indent {
                        JsonIndent::Spaces(count) => vec![b' '; usize::from(count)],
                        JsonIndent::Tabs => vec![b'\t'],
                    };
                    let formatter = serde_json::ser::PrettyFormatter::with_indent(&indentation);
                    let mut serializer =
                        serde_json::Serializer::with_formatter(&mut encoded, formatter);
                    value.borrow().serialize(&mut serializer)?;
                } else {
                    serde_json::to_writer(&mut encoded, value.borrow())?;
                }
                if options.ascii_json {
                    encoded = escape_non_ascii(&encoded);
                }
                if options.color_json {
                    writer.write_all(b"\x1b[36m")?;
                }
                writer.write_all(&encoded)?;
                if options.color_json {
                    writer.write_all(b"\x1b[0m")?;
                }
                writer.write_all(b"\n")?;
            }
        }
        OutputFormat::JsonLines => {
            for value in values {
                let mut encoded = serde_json::to_vec(value.borrow())?;
                if options.ascii_json {
                    encoded = escape_non_ascii(&encoded);
                }
                writer.write_all(&encoded)?;
                writer.write_all(b"\n")?;
            }
        }
        OutputFormat::Yaml => {
            for value in values {
                if options.yaml_document_start {
                    writer.write_all(b"---\n")?;
                }
                write_yaml_value(&mut writer, value.borrow(), 0)?;
                writer.write_all(b"\n")?;
            }
        }
    }
    Ok(())
}

fn write_yaml_value(
    writer: &mut impl Write,
    value: &Value,
    indent: usize,
) -> Result<(), OutputError> {
    match value {
        Value::Null => writer.write_all(b"null")?,
        Value::Bool(value) => writer.write_all(if *value { b"true" } else { b"false" })?,
        Value::Number(value) => writer.write_all(value.to_string().as_bytes())?,
        Value::String(value) => write_yaml_string(writer, value)?,
        Value::Array(values) if values.is_empty() => writer.write_all(b"[]")?,
        Value::Object(values) if values.is_empty() => writer.write_all(b"{}")?,
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    writer.write_all(b"\n")?;
                }
                write_indent(writer, indent)?;
                writer.write_all(b"-")?;
                if matches!(value, Value::Array(values) if !values.is_empty())
                    || matches!(value, Value::Object(values) if !values.is_empty())
                {
                    writer.write_all(b"\n")?;
                    write_yaml_value(writer, value, indent + 2)?;
                } else {
                    writer.write_all(b" ")?;
                    write_yaml_value(writer, value, indent + 2)?;
                }
            }
        }
        Value::Object(values) => {
            for (index, (key, value)) in values.iter().enumerate() {
                if index > 0 {
                    writer.write_all(b"\n")?;
                }
                write_indent(writer, indent)?;
                write_yaml_string(writer, key)?;
                writer.write_all(b":")?;
                if matches!(value, Value::Array(values) if !values.is_empty())
                    || matches!(value, Value::Object(values) if !values.is_empty())
                {
                    writer.write_all(b"\n")?;
                    write_yaml_value(writer, value, indent + 2)?;
                } else {
                    writer.write_all(b" ")?;
                    write_yaml_value(writer, value, indent + 2)?;
                }
            }
        }
    }
    Ok(())
}

fn write_yaml_string(writer: &mut impl Write, value: &str) -> Result<(), OutputError> {
    let encoded = yaml_serde::to_string(value)?;
    let encoded = encoded.strip_suffix('\n').unwrap_or(&encoded);
    if encoded.contains('\n') {
        serde_json::to_writer(writer, value)?;
    } else {
        writer.write_all(encoded.as_bytes())?;
    }
    Ok(())
}

fn write_indent(writer: &mut impl Write, indent: usize) -> Result<(), std::io::Error> {
    for _ in 0..indent {
        writer.write_all(b" ")?;
    }
    Ok(())
}

fn escape_non_ascii(encoded: &[u8]) -> Vec<u8> {
    let text = std::str::from_utf8(encoded).expect("JSON serializer emits UTF-8");
    let mut escaped = Vec::with_capacity(encoded.len());
    for character in text.chars() {
        if character.is_ascii() {
            escaped.push(character as u8);
            continue;
        }
        let mut units = [0_u16; 2];
        for unit in character.encode_utf16(&mut units).iter() {
            escaped.extend_from_slice(format!("\\u{unit:04x}").as_bytes());
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use tq_core::Value;

    use super::{OutputOptions, ToonFraming, write_results};
    use crate::OutputFormat;

    #[test]
    fn json_preserves_exact_literals_and_toon_defaults_to_sequence() {
        let value: Value = serde_json::from_str(r#"{"n":9007199254740993}"#).unwrap();
        let mut json = Vec::new();
        write_results(
            &mut json,
            [&value],
            OutputOptions {
                format: OutputFormat::Json,
                ..OutputOptions::default()
            },
        )
        .unwrap();
        assert_eq!(
            json,
            br#"{"n":9007199254740993}
"#
        );

        let mut toon = Vec::new();
        write_results(&mut toon, [&value], OutputOptions::default()).unwrap();
        assert_eq!(toon, b"\x1en: 9007199254740993\n");
    }

    #[test]
    fn unframed_rejects_zero_and_multiple_results_without_output() {
        let values = [Value::Null, Value::Bool(true)];
        for selection in [Vec::<&Value>::new(), values.iter().collect()] {
            let mut output = Vec::new();
            assert!(
                write_results(
                    &mut output,
                    selection,
                    OutputOptions {
                        toon_framing: ToonFraming::Unframed,
                        ..OutputOptions::default()
                    }
                )
                .is_err()
            );
            assert_eq!(output, [] as [u8; 0]);
        }
    }

    #[test]
    fn yaml_uses_block_layout_and_preserves_exact_numbers() {
        let value: Value =
            serde_json::from_str(r#"{"name":"Ada","items":[1,{"n":9007199254740993}],"empty":[]}"#)
                .unwrap();
        let mut yaml = Vec::new();
        write_results(
            &mut yaml,
            [&value],
            OutputOptions {
                format: OutputFormat::Yaml,
                ..OutputOptions::default()
            },
        )
        .unwrap();
        assert_eq!(
            String::from_utf8(yaml.clone()).unwrap(),
            "name: Ada\nitems:\n  - 1\n  -\n    n: 9007199254740993\nempty: []\n"
        );
        let decoded = crate::decode_yaml(&yaml, "round-trip").unwrap();
        assert_eq!(decoded[0].value, value);
    }

    #[test]
    fn json_lines_is_compact_lf_terminated_and_exact() {
        let values = [
            serde_json::from_str::<Value>(r#"{"n":9007199254740993}"#).unwrap(),
            Value::array(vec![Value::Bool(true)]),
        ];
        let mut output = Vec::new();
        write_results(
            &mut output,
            &values,
            OutputOptions {
                format: OutputFormat::JsonLines,
                pretty_json: true,
                color_json: true,
                ..OutputOptions::default()
            },
        )
        .unwrap();
        assert_eq!(output, b"{\"n\":9007199254740993}\n[true]\n");

        let mut empty = Vec::new();
        write_results(
            &mut empty,
            Vec::<&Value>::new(),
            OutputOptions {
                format: OutputFormat::JsonLines,
                ..OutputOptions::default()
            },
        )
        .unwrap();
        assert_eq!(empty, [] as [u8; 0]);
    }
}
