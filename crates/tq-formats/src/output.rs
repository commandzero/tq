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
    /// RS-prefix/LF-suffix TOON Text Sequence selected explicitly.
    Sequence,
    /// LF-terminated canonical values without record separators.
    #[default]
    Values,
    /// Exactly one standalone TOON document, selected explicitly.
    Unframed,
}

/// JSON pretty-print indentation.
#[derive(Clone, Debug, Eq, PartialEq)]
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
#[derive(Clone, Debug, Eq, PartialEq)]
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
    /// jq-compatible eight-slot ANSI palette for colored JSON.
    pub color_palette: JsonColorPalette,
    /// Prefix this YAML value with an explicit document separator.
    pub yaml_document_start: bool,
    /// TOON framing mode.
    pub toon_framing: ToonFraming,
    /// Prefix JSON results with ASCII RS for JSON Text Sequence output.
    pub json_sequence: bool,
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
            color_palette: JsonColorPalette::default(),
            yaml_document_start: false,
            toon_framing: ToonFraming::Values,
            json_sequence: false,
            toon: WriterConfig::default(),
        }
    }
}

/// ANSI styles used by jq's JSON colorizer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JsonColorPalette {
    styles: [String; 8],
    key_style: usize,
}

impl Default for JsonColorPalette {
    fn default() -> Self {
        Self {
            styles: std::array::from_fn(|index| {
                [
                    "0;90", "0;39", "0;39", "0;39", "0;32", "1;39", "1;39", "1;34",
                ][index]
                    .to_owned()
            }),
            key_style: 7,
        }
    }
}

impl JsonColorPalette {
    /// Parses jq's colon-separated seven- or eight-style `JQ_COLORS` value.
    #[must_use]
    pub fn from_jq_colors(value: &str) -> Self {
        let styles = value.split(':').collect::<Vec<_>>();
        if !matches!(styles.len(), 7 | 8)
            || styles.iter().any(|style| {
                style.is_empty()
                    || style
                        .bytes()
                        .any(|byte| byte != b';' && !byte.is_ascii_digit())
            })
        {
            return Self::default();
        }
        let key_style = if styles.len() == 7 { 3 } else { 7 };
        let fallback = styles.get(key_style).copied().unwrap_or_default();
        Self {
            styles: std::array::from_fn(|index| {
                styles.get(index).copied().unwrap_or(fallback).to_string()
            }),
            key_style,
        }
    }

    fn style(&self, index: usize) -> &str {
        &self.styles[index]
    }

    fn key_style(&self) -> &str {
        self.style(self.key_style)
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
/// LF-terminated values and supports explicit text-sequence framing.
///
/// # Errors
///
/// Returns serialization, framing/cardinality, or output I/O failures.
#[allow(
    clippy::needless_pass_by_value,
    reason = "the public writer API owns options for the selected output operation"
)]
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
            ToonFraming::Values => {
                for value in values {
                    write_unframed(&mut writer, [value], options.toon)?;
                    writer.write_all(b"\n")?;
                }
            }
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
                if options.json_sequence {
                    writer.write_all(b"\x1e")?;
                }
                if options.color_json {
                    writer.write_all(&colorize_json(&encoded, &options.color_palette))?;
                } else {
                    writer.write_all(&encoded)?;
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

fn colorize_json(encoded: &[u8], palette: &JsonColorPalette) -> Vec<u8> {
    let mut colored = Vec::with_capacity(encoded.len().saturating_add(64));
    let mut containers = Vec::new();
    let mut index = 0;
    while index < encoded.len() {
        match encoded[index] {
            b'{' => {
                if encoded.get(index + 1) == Some(&b'}') {
                    color_token(&mut colored, b"{}", palette.style(6));
                    index += 2;
                    continue;
                }
                color_token(&mut colored, b"{", palette.style(6));
                containers.push(b'{');
                index += 1;
            }
            b'}' => {
                color_token(&mut colored, b"}", palette.style(6));
                containers.pop();
                index += 1;
            }
            b'[' => {
                if encoded.get(index + 1) == Some(&b']') {
                    color_token(&mut colored, b"[]", palette.style(5));
                    index += 2;
                    continue;
                }
                color_token(&mut colored, b"[", palette.style(5));
                containers.push(b'[');
                index += 1;
            }
            b']' => {
                color_token(&mut colored, b"]", palette.style(5));
                containers.pop();
                index += 1;
            }
            b':' | b',' => {
                let style = containers
                    .last()
                    .map_or(3, |container| if *container == b'{' { 6 } else { 5 });
                color_token(&mut colored, &encoded[index..=index], palette.style(style));
                index += 1;
            }
            b'"' => {
                let end = json_string_end(encoded, index);
                let is_key = encoded[end..]
                    .iter()
                    .copied()
                    .find(|byte| !byte.is_ascii_whitespace())
                    == Some(b':');
                let style = if is_key {
                    palette.key_style()
                } else {
                    palette.style(4)
                };
                color_token(&mut colored, &encoded[index..end], style);
                index = end;
            }
            b'n' if encoded[index..].starts_with(b"null") => {
                color_token(&mut colored, b"null", palette.style(0));
                index += 4;
            }
            b'f' if encoded[index..].starts_with(b"false") => {
                color_token(&mut colored, b"false", palette.style(1));
                index += 5;
            }
            b't' if encoded[index..].starts_with(b"true") => {
                color_token(&mut colored, b"true", palette.style(2));
                index += 4;
            }
            byte if byte == b'-' || byte.is_ascii_digit() => {
                let end = encoded[index..]
                    .iter()
                    .position(|byte| {
                        !byte.is_ascii_digit() && !matches!(byte, b'-' | b'+' | b'.' | b'e' | b'E')
                    })
                    .map_or(encoded.len(), |offset| index + offset);
                color_token(&mut colored, &encoded[index..end], palette.style(3));
                index = end;
            }
            _ => {
                colored.push(encoded[index]);
                index += 1;
            }
        }
    }
    colored
}

fn json_string_end(encoded: &[u8], start: usize) -> usize {
    let mut escaped = false;
    for (offset, byte) in encoded[start + 1..].iter().copied().enumerate() {
        if escaped {
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        } else if byte == b'"' {
            return start + offset + 2;
        }
    }
    encoded.len()
}

fn color_token(output: &mut Vec<u8>, token: &[u8], style: &str) {
    output.extend_from_slice(b"\x1b[");
    output.extend_from_slice(style.as_bytes());
    output.extend_from_slice(b"m");
    output.extend_from_slice(token);
    output.extend_from_slice(b"\x1b[0m");
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
    use serde::{Serialize, Serializer};
    use tq_core::{Number, Object, Value};

    use super::{OutputOptions, ToonFraming, write_results};
    use crate::OutputFormat;

    #[test]
    fn json_preserves_exact_literals_and_toon_defaults_to_lf() {
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
        assert_eq!(toon, b"n: 9007199254740993\n");

        let mut sequence = Vec::new();
        write_results(
            &mut sequence,
            [&value],
            OutputOptions {
                toon_framing: ToonFraming::Sequence,
                ..OutputOptions::default()
            },
        )
        .unwrap();
        assert_eq!(sequence, b"\x1en: 9007199254740993\n");
    }

    #[test]
    fn json_output_uses_jq_negative_zero_spelling_without_rewriting_strings() {
        let value = Value::array(vec![
            Value::Number(Number::parse("-0.0").unwrap()),
            Value::Number(Number::parse("0.0").unwrap()),
            Value::Number(Number::from_runtime_f64(-0.0)),
            Value::Number(Number::parse("100e-2").unwrap()),
            Value::Number(Number::parse("1E+3").unwrap()),
            Value::string("-0.0"),
        ]);
        let mut compact = Vec::new();
        write_results(
            &mut compact,
            [&value],
            OutputOptions {
                format: OutputFormat::Json,
                ..OutputOptions::default()
            },
        )
        .unwrap();
        assert_eq!(compact, b"[-0.0,0.0,-0,1.00,1E+3,\"-0.0\"]\n");

        let mut pretty = Vec::new();
        write_results(
            &mut pretty,
            [&value],
            OutputOptions {
                format: OutputFormat::Json,
                pretty_json: true,
                ..OutputOptions::default()
            },
        )
        .unwrap();
        assert_eq!(
            pretty,
            b"[\n  -0.0,\n  0.0,\n  -0,\n  1.00,\n  1E+3,\n  \"-0.0\"\n]\n"
        );

        let mut lines = Vec::new();
        write_results(
            &mut lines,
            [&value],
            OutputOptions {
                format: OutputFormat::JsonLines,
                ..OutputOptions::default()
            },
        )
        .unwrap();
        assert_eq!(lines, b"[-0.0,0.0,-0,1.00,1E+3,\"-0.0\"]\n");
    }

    #[test]
    fn json_output_keeps_literal_numbers_around_nonfinite_values() {
        let mut object = Object::new();
        object.insert("z".into(), Value::Number(Number::parse("1.000").unwrap()));
        object.insert("a".into(), Value::Number(Number::parse("100e-2").unwrap()));
        let value = Value::array(vec![
            Value::Number(Number::from_runtime_f64(f64::NAN)),
            Value::Number(Number::from_runtime_f64(f64::INFINITY)),
            Value::object(object),
        ]);
        let mut output = Vec::new();
        write_results(
            &mut output,
            [&value],
            OutputOptions {
                format: OutputFormat::Json,
                ..OutputOptions::default()
            },
        )
        .unwrap();
        assert_eq!(
            output,
            b"[null,1.7976931348623157e+308,{\"z\":1.000,\"a\":1.00}]\n"
        );
    }

    #[test]
    fn raw_value_preserves_tokens_through_generic_json_serializer() {
        use serde_json::value::RawValue;

        struct GenericRaw;

        impl Serialize for GenericRaw {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                let raw = RawValue::from_string("1E+3".to_owned()).unwrap();
                raw.serialize(serializer)
            }
        }

        let raw = RawValue::from_string("1E+3".to_owned()).unwrap();
        assert_eq!(serde_json::to_vec(&raw).unwrap(), b"1E+3");
        assert_eq!(serde_json::to_vec(&GenericRaw).unwrap(), b"1E+3");
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
