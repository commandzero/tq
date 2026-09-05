//! Shared structured-result output dispatch.

use std::{borrow::Borrow, io::Write};

use serde::Serialize;
use thiserror::Error;
use tq_core::Value;
use tq_toon::{SequenceError, WriterConfig, WriterError, write_value};

use crate::{NativeFormat, OutputFormat};

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
    /// Reject profile normalization that would change the decoded shared value.
    pub strict_conversion: bool,
    /// Logical-row, field-size, and field-count bounds for CSV and TSV output.
    pub delimited_limits: crate::DelimitedLimits,
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
            strict_conversion: false,
            delimited_limits: crate::DelimitedLimits::default(),
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
    /// A native encoding resource bound was exceeded.
    #[error("output resource limit exceeded: {0}")]
    Resource(&'static str),
    /// A complete Result is outside the selected output profile or row shape.
    #[error("output profile rejection: {0}")]
    Profile(&'static str),
    /// The selected native format cannot encode results.
    #[error("unsupported output format: {0}")]
    UnsupportedFormat(&'static str),
    /// Selected framing or controls conflict.
    #[error("incompatible output options: {0}")]
    InvalidOptions(&'static str),
    /// A completed or failed sequence cannot accept another operation.
    #[error("native output sequence is no longer active")]
    Terminal,
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
            Self::Yaml(_)
            | Self::Toon(SequenceError::Cardinality(_))
            | Self::UnsupportedFormat(_)
            | Self::InvalidOptions(_)
            | Self::Profile(_)
            | Self::Resource(_)
            | Self::Terminal => false,
        }
    }
}

/// An output format whose directional capabilities have been checked.
#[derive(Clone, Copy, Debug)]
pub struct SelectedOutput {
    options: OutputOptions,
}

impl NativeFormat {
    /// Selects native output before any bytes are committed.
    ///
    /// # Errors
    ///
    /// Rejects input-only formats and incompatible explicit framing.
    pub fn select_output(self, mut options: OutputOptions) -> Result<SelectedOutput, OutputError> {
        options.format = self
            .descriptor()
            .output
            .ok_or(OutputError::UnsupportedFormat(self.descriptor().name))?;
        if self == Self::ToonSequence && options.toon_framing == ToonFraming::Unframed {
            return Err(OutputError::InvalidOptions("toon-seq cannot be unframed"));
        }
        let json = matches!(
            options.format,
            OutputFormat::Json | OutputFormat::JsonSequence
        );
        let json_lines = options.format == OutputFormat::JsonLines;
        if (!json
            && (options.pretty_json
                || options.color_json
                || options.json_indent != JsonIndent::default()))
            || (!json && !json_lines && options.ascii_json)
        {
            return Err(OutputError::InvalidOptions(
                "JSON controls are incompatible with the selected format",
            ));
        }
        if options.format != OutputFormat::Toon && options.toon_framing == ToonFraming::Unframed {
            return Err(OutputError::InvalidOptions(
                "unframed output is a TOON control",
            ));
        }
        if options.format != OutputFormat::Yaml && options.yaml_document_start {
            return Err(OutputError::InvalidOptions(
                "YAML document markers require YAML output",
            ));
        }
        let mut compatible_toon = WriterConfig::default();
        if json {
            if let JsonIndent::Spaces(count) = options.json_indent {
                compatible_toon.indent_size = usize::from(count);
            }
        }
        if options.format != OutputFormat::Toon
            && options.toon != WriterConfig::default()
            && options.toon != compatible_toon
        {
            return Err(OutputError::InvalidOptions(
                "TOON controls are incompatible with the selected format",
            ));
        }
        Ok(SelectedOutput { options })
    }
}

/// One native document sequence spanning all structured Results of a command.
pub struct NativeOutputSequence {
    options: OutputOptions,
    count: u64,
    active: bool,
    unframed: Option<Value>,
    delimited: Option<crate::delimited_output::DelimitedOutput>,
}

impl NativeOutputSequence {
    /// Starts a sequence without writing any bytes.
    #[must_use]
    pub const fn new(selection: SelectedOutput) -> Self {
        Self {
            options: selection.options,
            count: 0,
            active: true,
            unframed: None,
            delimited: None,
        }
    }

    /// Encodes one Result using the sequence's framing and context.
    ///
    /// # Errors
    ///
    /// Returns encoding or I/O failure. Any failure terminates the sequence.
    pub fn write_result(
        &mut self,
        writer: &mut impl Write,
        value: &Value,
    ) -> Result<(), OutputError> {
        if !self.active {
            return Err(OutputError::Terminal);
        }
        self.active = false;
        if self.options.format == OutputFormat::Toon
            && self.options.toon_framing == ToonFraming::Unframed
        {
            if self.count != 0 {
                self.unframed = None;
                return Err(SequenceError::Cardinality(tq_toon::CardinalityError::Multiple).into());
            }
            self.unframed = Some(value.clone());
            self.count = 1;
            self.active = true;
            return Ok(());
        }
        let mut options = self.options;
        options.yaml_document_start |= self.count > 0;
        if options.strict_conversion && options.format == OutputFormat::Yaml {
            let mut encoded = Vec::new();
            // Validate the YAML parser path even for the first Result. A later
            // document marker disables the input's whole-source JSON shortcut.
            write_document(
                &mut encoded,
                value,
                OutputOptions {
                    yaml_document_start: true,
                    ..options
                },
            )?;
            let decoded =
                crate::adapters::decode_yaml(&encoded, "strict conversion").map_err(|_| {
                    OutputError::Profile("strict conversion cannot decode the YAML representation")
                })?;
            if decoded.len() != 1 || decoded[0].value != *value {
                return Err(OutputError::Profile(
                    "strict conversion would change the shared value through YAML",
                ));
            }
        }
        if matches!(options.format, OutputFormat::Csv | OutputFormat::Tsv) {
            let delimiter = if options.format == OutputFormat::Csv {
                b','
            } else {
                b'\t'
            };
            self.delimited
                .get_or_insert_with(|| crate::delimited_output::DelimitedOutput::new(delimiter))
                .write_result(
                    writer,
                    value,
                    options.strict_conversion,
                    options.delimited_limits,
                )?;
        } else {
            write_document(writer, value, options)?;
        }
        self.count = self.count.saturating_add(1);
        self.active = true;
        Ok(())
    }

    /// Completes the sequence. Callers own flushing and the shared byte budget.
    ///
    /// # Errors
    ///
    /// Rejects completion after an earlier failure or successful completion.
    pub fn finish(&mut self, writer: &mut impl Write) -> Result<(), OutputError> {
        if !self.active {
            return Err(OutputError::Terminal);
        }
        self.active = false;
        if self.options.format == OutputFormat::Toon
            && self.options.toon_framing == ToonFraming::Unframed
        {
            let value = self
                .unframed
                .take()
                .ok_or(SequenceError::Cardinality(tq_toon::CardinalityError::Zero))?;
            write_value(writer, &value, self.options.toon)
                .map_err(|WriterError::Io(error)| OutputError::Io(error))?;
        }
        Ok(())
    }
}

fn write_document(
    writer: &mut impl Write,
    value: &Value,
    options: OutputOptions,
) -> Result<(), OutputError> {
    match options.format {
        OutputFormat::Csv | OutputFormat::Tsv => {
            unreachable!("delimited output requires its command-scoped row shape")
        }
        OutputFormat::Toon => {
            crate::rs_framing::write_frame(writer, |writer| {
                write_value(writer, value, options.toon)
                    .map_err(|WriterError::Io(error)| OutputError::Io(error))
            })?;
        }
        OutputFormat::JsonSequence => {
            crate::rs_framing::write_frame(writer, |writer| {
                write_json_document(writer, value, options)
            })?;
        }
        OutputFormat::Json | OutputFormat::JsonLines => {
            let mut options = options;
            if options.format == OutputFormat::JsonLines {
                options.pretty_json = false;
                options.color_json = false;
            }
            write_json_document(writer, value, options)?;
            writer.write_all(b"\n")?;
        }
        OutputFormat::Yaml => {
            if options.yaml_document_start {
                writer.write_all(b"---\n")?;
            }
            write_yaml_value(writer, value, 0)?;
            writer.write_all(b"\n")?;
        }
    }
    Ok(())
}

fn write_json_document(
    writer: &mut impl Write,
    value: &Value,
    options: OutputOptions,
) -> Result<(), OutputError> {
    if options.color_json {
        writer.write_all(b"\x1b[36m")?;
    }
    if options.ascii_json {
        let mut encoded = Vec::new();
        write_json_value(&mut encoded, value, options)?;
        writer.write_all(&escape_non_ascii(&encoded))?;
    } else {
        write_json_value(writer, value, options)?;
    }
    if options.color_json {
        writer.write_all(b"\x1b[0m")?;
    }
    Ok(())
}

fn write_json_value(
    writer: &mut impl Write,
    value: &Value,
    options: OutputOptions,
) -> Result<(), serde_json::Error> {
    if options.pretty_json {
        let indentation = match options.json_indent {
            JsonIndent::Spaces(count) => vec![b' '; usize::from(count)],
            JsonIndent::Tabs => vec![b'\t'],
        };
        let formatter = serde_json::ser::PrettyFormatter::with_indent(&indentation);
        value.serialize(&mut serde_json::Serializer::with_formatter(
            writer, formatter,
        ))
    } else {
        serde_json::to_writer(writer, value)
    }
}

/// Writes an iterator of Results through one native output sequence.
///
/// # Errors
///
/// Returns format selection, cardinality, encoding, or I/O failures.
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
    let selection = NativeFormat::from_output(options.format).select_output(options)?;
    let mut sequence = NativeOutputSequence::new(selection);
    for value in values {
        sequence.write_result(&mut writer, value.borrow())?;
    }
    sequence.finish(&mut writer)
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
