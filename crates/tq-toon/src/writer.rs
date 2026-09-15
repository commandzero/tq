//! Canonical, ordered TOON v3 writer over tq's exact value model.

use std::io::{self, Write};

use thiserror::Error;
use tq_core::{
    Object, Value,
    presentation::{ColorPalette, ColorRole, write_span},
};

use crate::ScalarToken;

const INDENT: &[u8; 64] = b"                                                                ";

/// Delimiter used by inline and tabular arrays.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Delimiter {
    /// Comma-delimited arrays; the canonical default.
    #[default]
    Comma,
    /// Tab-delimited arrays.
    Tab,
    /// Pipe-delimited arrays.
    Pipe,
}

impl Delimiter {
    const fn character(self) -> char {
        match self {
            Self::Comma => ',',
            Self::Tab => '\t',
            Self::Pipe => '|',
        }
    }

    const fn header_suffix(self) -> &'static str {
        match self {
            Self::Comma => "",
            Self::Tab => "\t",
            Self::Pipe => "|",
        }
    }
}

/// Safe dotted-key folding policy.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum KeyFolding {
    /// Preserve explicit object nesting.
    #[default]
    Off,
    /// Fold safe single-key object chains.
    Safe,
}

/// Canonical writer options.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WriterConfig {
    /// Spaces per indentation level.
    pub indent_size: usize,
    /// Active array delimiter.
    pub delimiter: Delimiter,
    /// Safe dotted-key folding mode.
    pub key_folding: KeyFolding,
    /// Maximum number of segments in one folded key.
    pub flatten_depth: usize,
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            indent_size: 2,
            delimiter: Delimiter::Comma,
            key_folding: KeyFolding::Off,
            flatten_depth: usize::MAX,
        }
    }
}

/// Canonical writer failure.
#[derive(Debug, Error)]
pub enum WriterError {
    /// Output I/O failed.
    #[error("TOON output I/O failed: {0}")]
    Io(#[from] io::Error),
}

/// Encodes one standalone value with no trailing newline.
///
/// # Panics
///
/// Panics only if writing UTF-8 TOON bytes to an in-memory `Vec<u8>` fails or
/// the encoder violates its UTF-8 output invariant.
#[must_use]
pub fn encode(value: &Value, config: WriterConfig) -> String {
    let mut output = Vec::new();
    write_value(&mut output, value, config).expect("writing TOON to memory cannot fail");
    String::from_utf8(output).expect("TOON output is UTF-8")
}

/// Writes one standalone value with no trailing newline.
///
/// # Errors
///
/// Returns an output I/O error.
pub fn write_value<W: Write>(
    mut writer: W,
    value: &Value,
    config: WriterConfig,
) -> Result<(), WriterError> {
    write_value_colored(&mut writer, value, config, None)
}

/// Writes one standalone value with optional semantic ANSI presentation.
///
/// The palette is a presentation-only sink selection. None is the exact
/// plain writer path.
///
/// # Errors
///
/// Returns the output sink's I/O error.
pub fn write_value_colored<W: Write + ?Sized>(
    writer: &mut W,
    value: &Value,
    config: WriterConfig,
    palette: Option<&ColorPalette>,
) -> Result<(), WriterError> {
    Encoder::new(writer, config, palette).encode(value)
}

struct Encoder<'a, W> {
    config: WriterConfig,
    writer: W,
    palette: Option<&'a ColorPalette>,
    wrote_line: bool,
}

impl<'a, W: Write> Encoder<'a, W> {
    fn new(writer: W, config: WriterConfig, palette: Option<&'a ColorPalette>) -> Self {
        Self {
            config,
            writer,
            palette,
            wrote_line: false,
        }
    }

    fn palette(&self) -> Option<&'a ColorPalette> {
        self.palette
    }

    fn encode(&mut self, value: &Value) -> Result<(), WriterError> {
        match value {
            Value::Object(object) => self.object(object, 0, true)?,
            Value::Array(values) => self.array(None, values, 0, None, 1)?,
            _ => {
                self.start_line(0, None)?;
                self.write_scalar(value, ScalarContext::Root)?;
            }
        }
        Ok(())
    }

    fn object(
        &mut self,
        object: &Object,
        depth: usize,
        allow_folding: bool,
    ) -> Result<(), WriterError> {
        for (key, value) in object {
            let member_folding = allow_folding && self.fold_allowed(object, key, value);
            self.member(key, value, depth, None, member_folding)?;
        }
        Ok(())
    }

    fn member(
        &mut self,
        key: &str,
        value: &Value,
        depth: usize,
        prefix: Option<&str>,
        allow_folding: bool,
    ) -> Result<(), WriterError> {
        let (folded_key, folded_value) = if allow_folding {
            self.folded(key, value)
        } else {
            (key.to_owned(), value)
        };
        let folded_here = folded_key != key;
        let logical_depth = depth + usize::from(prefix.is_some());
        match folded_value {
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
                self.start_line(depth, prefix)?;
                self.write_key(&folded_key)?;
                self.writer.write_all(b":")?;
                self.writer.write_all(b" ")?;
                self.write_scalar(folded_value, ScalarContext::Object)?;
            }
            Value::Object(object) => {
                self.start_line(depth, prefix)?;
                self.write_key(&folded_key)?;
                self.writer.write_all(b":")?;
                self.object(object, logical_depth + 1, allow_folding && !folded_here)?;
            }
            Value::Array(values) => {
                self.array(Some(&folded_key), values, depth, prefix, logical_depth + 1)?;
            }
        }
        Ok(())
    }

    fn folded<'v>(&self, first: &str, value: &'v Value) -> (String, &'v Value) {
        if self.config.key_folding != KeyFolding::Safe
            || self.config.flatten_depth < 2
            || !identifier_segment(first)
        {
            return (first.to_owned(), value);
        }
        let mut segments = vec![first];
        let mut current = value;
        while segments.len() < self.config.flatten_depth {
            let Value::Object(object) = current else {
                break;
            };
            if object.len() != 1 {
                break;
            }
            let (next, value) = object.first().expect("single-key object");
            if !identifier_segment(next) {
                break;
            }
            segments.push(next);
            current = value;
        }
        (segments.join("."), current)
    }

    fn fold_allowed(&self, siblings: &Object, key: &str, value: &Value) -> bool {
        let (folded, _) = self.folded(key, value);
        folded == key || !siblings.contains_key(folded.as_str())
    }

    fn array(
        &mut self,
        key: Option<&str>,
        values: &[Value],
        depth: usize,
        prefix: Option<&str>,
        content_depth: usize,
    ) -> Result<(), WriterError> {
        self.start_line(depth, prefix)?;
        if let Some(key) = key {
            self.write_key(key)?;
        }
        self.write_structure(ColorRole::Array, b"[")?;
        self.write_span(ColorRole::Number, values.len().to_string().as_bytes())?;
        self.write_structure(
            ColorRole::Array,
            self.config.delimiter.header_suffix().as_bytes(),
        )?;
        self.write_structure(ColorRole::Array, b"]")?;

        if values.iter().all(is_scalar) {
            self.writer.write_all(b":")?;
            if !values.is_empty() {
                self.writer.write_all(b" ")?;
                for (index, value) in values.iter().enumerate() {
                    if index != 0 {
                        self.write_delimiter(ColorRole::Array)?;
                    }
                    self.write_scalar(value, ScalarContext::Array)?;
                }
            }
            return Ok(());
        }

        if let Some(fields) = tabular_fields(values) {
            self.write_structure(ColorRole::Object, b"{")?;
            for (index, field) in fields.iter().enumerate() {
                if index != 0 {
                    self.write_delimiter(ColorRole::Object)?;
                }
                self.write_key(field)?;
            }
            self.write_structure(ColorRole::Object, b"}")?;
            self.writer.write_all(b":")?;
            for value in values {
                let Value::Object(object) = value else {
                    unreachable!("tabular eligibility checked")
                };
                self.start_line(content_depth, None)?;
                for (index, field) in fields.iter().enumerate() {
                    if index != 0 {
                        self.write_delimiter(ColorRole::Object)?;
                    }
                    self.write_scalar(&object[*field], ScalarContext::Object)?;
                }
            }
            return Ok(());
        }

        self.writer.write_all(b":")?;
        for value in values {
            match value {
                Value::Object(object) if object.is_empty() => {
                    self.start_line(content_depth, None)?;
                    self.write_structure(ColorRole::Array, b"-")?;
                }
                Value::Object(object) => {
                    let mut members = object.iter();
                    let (first, value) = members.next().expect("non-empty object");
                    let allow_folding = self.fold_allowed(object, first, value);
                    self.member(first, value, content_depth, Some("- "), allow_folding)?;
                    for (key, value) in members {
                        let allow_folding = self.fold_allowed(object, key, value);
                        self.member(key, value, content_depth + 1, None, allow_folding)?;
                    }
                }
                Value::Array(nested) => {
                    self.array(None, nested, content_depth, Some("- "), content_depth + 1)?;
                }
                _ => {
                    self.start_line(content_depth, Some("- "))?;
                    self.write_scalar(value, ScalarContext::Array)?;
                }
            }
        }
        Ok(())
    }

    fn start_line(&mut self, depth: usize, prefix: Option<&str>) -> Result<(), WriterError> {
        if self.wrote_line {
            self.writer.write_all(b"\n")?;
        }
        self.wrote_line = true;
        let mut spaces = depth.saturating_mul(self.config.indent_size);
        while spaces != 0 {
            let count = spaces.min(INDENT.len());
            self.writer.write_all(&INDENT[..count])?;
            spaces -= count;
        }
        if let Some(prefix) = prefix {
            if matches!(prefix, "-" | "- ") {
                self.write_structure(ColorRole::Array, b"-")?;
                if prefix == "- " {
                    self.writer.write_all(b" ")?;
                }
            } else {
                self.writer.write_all(prefix.as_bytes())?;
            }
        }
        Ok(())
    }

    fn write_scalar(&mut self, value: &Value, context: ScalarContext) -> Result<(), WriterError> {
        match value {
            Value::Null => self.write_span(ColorRole::Null, b"null"),
            Value::Bool(false) => self.write_span(ColorRole::False, b"false"),
            Value::Bool(true) => self.write_span(ColorRole::True, b"true"),
            Value::Number(value) => {
                let rendered = value.canonical_numeric();
                self.write_span(ColorRole::Number, rendered.as_bytes())
            }
            Value::String(value) => {
                if safe_string(value, self.config.delimiter, context) {
                    self.write_span(ColorRole::String, value.as_bytes())
                } else {
                    let palette = self.palette();
                    write_quoted(
                        &mut self.writer,
                        value,
                        structural_role(context),
                        ColorRole::String,
                        palette,
                    )?;
                    Ok(())
                }
            }
            Value::Array(_) | Value::Object(_) => unreachable!("scalar context"),
        }
    }

    fn write_key(&mut self, key: &str) -> Result<(), WriterError> {
        let palette = self.palette();
        write_key(&mut self.writer, key, palette)
    }

    fn write_structure(&mut self, role: ColorRole, bytes: &[u8]) -> Result<(), WriterError> {
        self.write_span(role, bytes)
    }

    fn write_span(&mut self, role: ColorRole, bytes: &[u8]) -> Result<(), WriterError> {
        let palette = self.palette();
        write_span(&mut self.writer, palette, role, bytes)?;
        Ok(())
    }

    fn write_delimiter(&mut self, role: ColorRole) -> Result<(), WriterError> {
        let mut bytes = [0_u8; 4];
        self.write_structure(
            role,
            self.config
                .delimiter
                .character()
                .encode_utf8(&mut bytes)
                .as_bytes(),
        )
    }
}

#[derive(Clone, Copy)]
pub(crate) enum ScalarContext {
    Root,
    Object,
    Array,
}

fn structural_role(context: ScalarContext) -> ColorRole {
    match context {
        ScalarContext::Array => ColorRole::Array,
        ScalarContext::Root | ScalarContext::Object => ColorRole::Object,
    }
}

fn write_quoted<W: Write + ?Sized>(
    writer: &mut W,
    value: &str,
    quote_role: ColorRole,
    content_role: ColorRole,
    palette: Option<&ColorPalette>,
) -> io::Result<()> {
    write_span(writer, palette, quote_role, b"\"")?;
    let mut chunk = Vec::with_capacity(4096);
    for character in value.chars() {
        let mut encoded = [0_u8; 4];
        let escaped = match character {
            '"' => b"\\\"",
            '\\' => b"\\\\",
            '\n' => b"\\n",
            '\r' => b"\\r",
            '\t' => b"\\t",
            _ => character.encode_utf8(&mut encoded).as_bytes(),
        };
        if chunk.len().saturating_add(escaped.len()) > 4096 {
            write_span(writer, palette, content_role, &chunk)?;
            chunk.clear();
        }
        chunk.extend_from_slice(escaped);
    }
    write_span(writer, palette, content_role, &chunk)?;
    write_span(writer, palette, quote_role, b"\"")
}

pub(crate) fn write_key<W: Write + ?Sized>(
    writer: &mut W,
    key: &str,
    palette: Option<&ColorPalette>,
) -> Result<(), WriterError> {
    if safe_key(key) {
        write_span(writer, palette, ColorRole::Key, key.as_bytes())?;
    } else {
        write_quoted(writer, key, ColorRole::Object, ColorRole::Key, palette)?;
    }
    Ok(())
}

pub(crate) fn write_scalar_token_colored(
    output: &mut (impl Write + ?Sized),
    value: ScalarToken<'_>,
    config: WriterConfig,
    context: ScalarContext,
    palette: Option<&ColorPalette>,
) -> Result<(), WriterError> {
    match value {
        ScalarToken::Null => write_span(output, palette, ColorRole::Null, b"null")?,
        ScalarToken::Bool(false) => write_span(output, palette, ColorRole::False, b"false")?,
        ScalarToken::Bool(true) => write_span(output, palette, ColorRole::True, b"true")?,
        ScalarToken::Number(value) => {
            write_span(output, palette, ColorRole::Number, value.as_bytes())?;
        }
        ScalarToken::String(value) => {
            if safe_string(value, config.delimiter, context) {
                write_span(output, palette, ColorRole::String, value.as_bytes())?;
            } else {
                write_quoted(
                    output,
                    value,
                    structural_role(context),
                    ColorRole::String,
                    palette,
                )?;
            }
        }
    }
    Ok(())
}

fn is_scalar(value: &Value) -> bool {
    matches!(
        value,
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)
    )
}

fn tabular_fields(values: &[Value]) -> Option<Vec<&str>> {
    let Value::Object(first) = values.first()? else {
        return None;
    };
    if first.is_empty() || first.values().any(|value| !is_scalar(value)) {
        return None;
    }
    let fields = first.keys().map(AsRef::as_ref).collect::<Vec<_>>();
    if values.iter().all(|value| {
        let Value::Object(object) = value else {
            return false;
        };
        object.len() == fields.len()
            && fields.iter().all(|field| object.contains_key(*field))
            && object.values().all(is_scalar)
    }) {
        Some(fields)
    } else {
        None
    }
}

fn safe_key(value: &str) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|character| character.is_alphabetic() || character == '_')
        && characters.all(|character| character.is_alphanumeric() || matches!(character, '_' | '.'))
}

fn safe_string(value: &str, delimiter: Delimiter, context: ScalarContext) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !matches!(value, "null" | "true" | "false")
        && !looks_like_number(value)
        && !value.contains(['\n', '\r', '\t', '"', '\\', ':', '[', ']', '{', '}'])
        && !value.contains(delimiter.character())
        && !value.starts_with("- ")
        && !(matches!(context, ScalarContext::Object | ScalarContext::Array)
            && value.starts_with('-'))
}

fn looks_like_number(value: &str) -> bool {
    !matches!(
        tq_core::Number::parse(value),
        Err(tq_core::NumberError::Invalid)
    ) || value
        .strip_prefix('0')
        .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_digit()))
}

fn identifier_segment(value: &str) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|character| character.is_alphabetic() || character == '_')
        && characters.all(|character| character.is_alphanumeric() || character == '_')
}

pub(crate) fn write_tabular_row_colored(
    mut output: impl Write,
    object: &Object,
    fields: &[std::sync::Arc<str>],
    config: WriterConfig,
    palette: Option<&ColorPalette>,
) -> Result<(), WriterError> {
    for (index, field) in fields.iter().enumerate() {
        if index != 0 {
            let mut bytes = [0_u8; 4];
            write_span(
                &mut output,
                palette,
                ColorRole::Object,
                config
                    .delimiter
                    .character()
                    .encode_utf8(&mut bytes)
                    .as_bytes(),
            )?;
        }
        let mut encoder = Encoder::new(&mut output, config, palette);
        encoder.write_scalar(&object[field], ScalarContext::Object)?;
    }
    Ok(())
}

pub(crate) fn write_list_item_colored(
    output: &mut (impl Write + ?Sized),
    value: &Value,
    config: WriterConfig,
    palette: Option<&ColorPalette>,
) -> Result<(), WriterError> {
    let mut encoder = Encoder::new(output, config, palette);
    match value {
        Value::Object(object) if object.is_empty() => encoder.start_line(0, Some("-")),
        Value::Object(object) => {
            let mut members = object.iter();
            let (first, value) = members.next().expect("non-empty object");
            let allow_folding = encoder.fold_allowed(object, first, value);
            encoder.member(first, value, 0, Some("- "), allow_folding)?;
            for (key, value) in members {
                let allow_folding = encoder.fold_allowed(object, key, value);
                encoder.member(key, value, 1, None, allow_folding)?;
            }
            Ok(())
        }
        Value::Array(values) => encoder.array(None, values, 0, Some("- "), 1),
        _ => {
            encoder.start_line(0, Some("- "))?;
            encoder.write_scalar(value, ScalarContext::Array)
        }
    }
}

#[cfg(test)]
mod tests {
    use tq_core::{Value, presentation::ColorPalette};

    use super::{Delimiter, WriterConfig, encode, write_value_colored};

    fn strip_sgr(bytes: &[u8]) -> Vec<u8> {
        let mut output = Vec::with_capacity(bytes.len());
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index..].starts_with(b"\x1b[") {
                index += 2;
                while index < bytes.len() && bytes[index] != b'm' {
                    index += 1;
                }
                assert!(index < bytes.len(), "unterminated SGR");
                index += 1;
            } else {
                output.push(bytes[index]);
                index += 1;
            }
        }
        output
    }

    #[test]
    fn canonical_document_has_order_tabular_layout_and_no_newline() {
        let value = Value::from_json(
            serde_json::from_str(
                r#"{"z":1,"items":[{"id":1,"name":"Ada"},{"id":2,"name":"Bob"}]}"#,
            )
            .unwrap(),
        )
        .unwrap();
        let encoded = encode(&value, WriterConfig::default());
        assert_eq!(encoded, "z: 1\nitems[2]{id,name}:\n  1,Ada\n  2,Bob");
        assert!(!encoded.ends_with('\n'));
    }

    #[test]
    fn active_delimiter_controls_only_relevant_string_quoting() {
        let value =
            Value::from_json(serde_json::from_str(r#"{"v":["a,b","c|d"]}"#).unwrap()).unwrap();
        let config = WriterConfig {
            delimiter: Delimiter::Pipe,
            ..WriterConfig::default()
        };
        assert_eq!(encode(&value, config), "v[2|]: a,b|\"c|d\"");
    }

    #[test]
    fn colored_document_preserves_bytes_and_structural_quote_context() {
        let value = Value::from_json(
            serde_json::from_str(r#"{"arr":["a,b"],"quoted key":"x:y"}"#).unwrap(),
        )
        .unwrap();
        let palette = ColorPalette::from_jq_colors("10:11:12:13:14:15:16:17");
        let mut colored = Vec::new();
        write_value_colored(
            &mut colored,
            &value,
            WriterConfig::default(),
            Some(&palette),
        )
        .unwrap();

        assert_eq!(
            strip_sgr(&colored),
            encode(&value, WriterConfig::default()).as_bytes()
        );
        assert!(
            colored
                .windows(b"\x1b[17marr".len())
                .any(|window| window == b"\x1b[17marr")
        );
        assert!(
            colored
                .windows(b"\x1b[15m[".len())
                .any(|window| window == b"\x1b[15m[")
        );
        assert!(
            colored
                .windows(b"\x1b[13m1".len())
                .any(|window| window == b"\x1b[13m1")
        );
        assert!(
            colored
                .windows(b"\x1b[15m\"\x1b[0m".len())
                .any(|window| window == b"\x1b[15m\"\x1b[0m")
        );
        assert!(
            colored
                .windows(b"\x1b[16m\"\x1b[0m".len())
                .any(|window| window == b"\x1b[16m\"\x1b[0m")
        );
        assert!(
            colored
                .windows(b"\x1b[14mx:y".len())
                .any(|window| window == b"\x1b[14mx:y")
        );
        assert!(
            colored
                .windows(b"\x1b[17mquoted".len())
                .any(|window| window == b"\x1b[17mquoted")
        );
    }

    #[test]
    fn colored_table_uses_object_context_for_field_values() {
        let value = Value::from_json(
            serde_json::from_str(r#"{"rows":[{"name":"a,b","count":1}]}"#).unwrap(),
        )
        .unwrap();
        let palette = ColorPalette::from_jq_colors("10:11:12:13:14:15:16:17");
        let mut colored = Vec::new();
        write_value_colored(
            &mut colored,
            &value,
            WriterConfig::default(),
            Some(&palette),
        )
        .unwrap();

        assert_eq!(
            strip_sgr(&colored),
            encode(&value, WriterConfig::default()).as_bytes()
        );
        assert!(
            colored
                .windows(b"\x1b[16m\"\x1b[0m".len())
                .any(|window| window == b"\x1b[16m\"\x1b[0m")
        );
        assert!(
            colored
                .windows(b"\x1b[14ma,b".len())
                .any(|window| window == b"\x1b[14ma,b")
        );
    }
}
