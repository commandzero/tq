//! Shared structured-result output dispatch.

use std::{borrow::Borrow, io::Write};

use thiserror::Error;
use tq_core::{Value, presentation::ColorPalette};
use tq_toon::{SequenceError, WriterConfig, WriterError};

use crate::{NativeFormat, OutputFormat};

/// Compatibility name retained for callers of the JSON-only color API.
pub type JsonColorPalette = ColorPalette;

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
#[derive(Clone, Debug, Eq, PartialEq)]
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
    /// Compatibility storage for the former JSON-only color option. When true,
    /// the resolved palette applies to every supported structured format.
    pub color_json: bool,
    /// jq-compatible eight-slot ANSI palette for colored output.
    pub color_palette: ColorPalette,
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
            strict_conversion: false,
            delimited_limits: crate::DelimitedLimits::default(),
            pretty_json: false,
            json_indent: JsonIndent::default(),
            ascii_json: false,
            color_json: false,
            color_palette: ColorPalette::default(),
            yaml_document_start: false,
            toon_framing: ToonFraming::Values,
            json_sequence: false,
            toon: WriterConfig::default(),
        }
    }
}

impl OutputOptions {
    /// Selects semantic color presentation for every supported output format.
    #[must_use]
    pub fn with_color(mut self, enabled: bool) -> Self {
        self.color_json = enabled;
        self
    }

    /// Returns whether semantic color presentation is enabled.
    #[must_use]
    pub const fn color_enabled(&self) -> bool {
        self.color_json
    }

    fn palette(&self) -> Option<&ColorPalette> {
        self.color_enabled().then_some(&self.color_palette)
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
#[derive(Clone, Debug)]
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
        if self == Self::ToonSequence {
            options.toon_framing = ToonFraming::Sequence;
        }
        let json = matches!(
            options.format,
            OutputFormat::Json | OutputFormat::JsonSequence
        );
        let json_lines = options.format == OutputFormat::JsonLines;
        if (!json && (options.pretty_json || options.json_indent != JsonIndent::default()))
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
        if json && let JsonIndent::Spaces(count) = options.json_indent {
            compatible_toon.indent_size = usize::from(count);
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
    pub fn new(selection: SelectedOutput) -> Self {
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
        let mut options = self.options.clone();
        options.yaml_document_start |= self.count > 0;
        if options.strict_conversion && options.format == OutputFormat::Yaml {
            let mut encoded = Vec::new();
            // Validate the YAML parser path even for the first Result. A later
            // document marker disables the input's whole-source JSON shortcut.
            write_document(
                &mut encoded,
                value,
                &OutputOptions {
                    yaml_document_start: true,
                    color_json: false,
                    ..options.clone()
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
                    options.palette(),
                )?;
        } else {
            write_document(writer, value, &options)?;
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
            tq_toon::write_value_colored(writer, &value, self.options.toon, self.options.palette())
                .map_err(|WriterError::Io(error)| OutputError::Io(error))?;
        }
        Ok(())
    }
}

fn write_document(
    writer: &mut impl Write,
    value: &Value,
    options: &OutputOptions,
) -> Result<(), OutputError> {
    match options.format {
        OutputFormat::Csv | OutputFormat::Tsv => {
            unreachable!("delimited output requires its command-scoped row shape")
        }
        OutputFormat::Toon => match options.toon_framing {
            ToonFraming::Sequence => {
                crate::rs_framing::write_frame(writer, |writer| {
                    tq_toon::write_value_colored(writer, value, options.toon, options.palette())
                        .map_err(|WriterError::Io(error)| OutputError::Io(error))
                })?;
            }
            ToonFraming::Values => {
                tq_toon::write_value_colored(&mut *writer, value, options.toon, options.palette())
                    .map_err(|WriterError::Io(error)| OutputError::Io(error))?;
                writer.write_all(b"\n")?;
            }
            ToonFraming::Unframed => unreachable!("unframed TOON is buffered by the sequence"),
        },
        OutputFormat::JsonSequence => {
            crate::rs_framing::write_frame(writer, |writer| {
                write_json_document(writer, value, options)
            })?;
        }
        OutputFormat::Json | OutputFormat::JsonLines => {
            if options.format == OutputFormat::JsonLines {
                let mut options = options.clone();
                options.pretty_json = false;
                write_json_document(writer, value, &options)?;
            } else {
                write_json_document(writer, value, options)?;
            }
            writer.write_all(b"\n")?;
        }
        OutputFormat::Yaml => {
            if options.yaml_document_start {
                crate::output_color::write_span_bounded(
                    writer,
                    options.palette(),
                    tq_core::presentation::ColorRole::Object,
                    b"---",
                )?;
                writer.write_all(b"\n")?;
            }
            crate::output_yaml::write_yaml_value(
                writer,
                value,
                0,
                tq_core::presentation::ColorRole::Object,
                options.palette(),
            )?;
            writer.write_all(b"\n")?;
        }
    }
    Ok(())
}

fn write_json_document(
    writer: &mut impl Write,
    value: &Value,
    options: &OutputOptions,
) -> Result<(), OutputError> {
    let indentation = match options.json_indent {
        JsonIndent::Spaces(count) => vec![b' '; usize::from(count)],
        JsonIndent::Tabs => vec![b'\t'],
    };
    // Preserve the existing document publication boundary. The formatter
    // writes semantic spans into this existing staging buffer; it never
    // reparses a completed document or creates a second result-sized buffer.
    let mut encoded = Vec::new();
    crate::output_json::write_json_value(
        &mut encoded,
        value,
        options.pretty_json,
        &indentation,
        options.ascii_json,
        options.palette(),
    )?;
    if options.palette().is_some() {
        tq_core::presentation::write_styled_bytes(writer, &encoded)?;
    } else {
        writer.write_all(&encoded)?;
    }
    Ok(())
}

/// Writes the compact JSON fallback used for non-string raw-mode results.
///
/// This preserves the existing raw-mode encoding, adds no framing, and accepts
/// the same invocation palette as the native output writers.
///
/// # Errors
/// Returns serialization or output failures.
pub fn write_raw_json_value(
    writer: &mut impl Write,
    value: &Value,
    palette: Option<&ColorPalette>,
) -> Result<(), serde_json::Error> {
    crate::output_json::write_json_value(writer, value, false, b"", false, palette)
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
    let mut options = options;
    if options.json_sequence && options.format == OutputFormat::Json {
        options.format = OutputFormat::JsonSequence;
    }
    let selection = NativeFormat::from_output(options.format).select_output(options)?;
    let mut sequence = NativeOutputSequence::new(selection);
    for value in values {
        sequence.write_result(&mut writer, value.borrow())?;
    }
    sequence.finish(&mut writer)
}

#[cfg(test)]
mod tests {
    use serde::{Serialize, Serializer};
    use tq_core::{Number, Object, Value};

    use super::{NativeOutputSequence, OutputOptions, ToonFraming, write_results};
    use crate::{NativeFormat, OutputFormat};

    #[test]
    fn explicit_native_toon_sequence_overrides_the_values_default() {
        let selection = NativeFormat::ToonSequence
            .select_output(OutputOptions::default())
            .unwrap();
        let mut sequence = NativeOutputSequence::new(selection);
        let mut output = Vec::new();
        sequence.write_result(&mut output, &Value::Null).unwrap();
        sequence
            .write_result(&mut output, &Value::Bool(true))
            .unwrap();
        sequence.finish(&mut output).unwrap();
        assert_eq!(output, b"\x1enull\n\x1etrue\n");
    }

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
