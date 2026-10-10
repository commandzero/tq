//! Canonical, ordered TOON 4.1 writer over tq's exact value model.

use std::io::{self, Write};

use thiserror::Error;
use tq_core::{
    Object, Value,
    presentation::{ColorPalette, ColorRole, write_span},
};

use crate::{
    ScalarToken,
    schema::{RowSchema, SchemaError, SchemaLimits},
};

const INDENT: &[u8; 64] = b"                                                                ";

pub(crate) const MAX_WRITER_DEPTH: usize = 256;

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
    pub(crate) const fn character(self) -> char {
        match self {
            Self::Comma => ',',
            Self::Tab => '\t',
            Self::Pipe => '|',
        }
    }

    pub(crate) const fn header_suffix(self) -> &'static str {
        match self {
            Self::Comma => "",
            Self::Tab => "\t",
            Self::Pipe => "|",
        }
    }
}

/// Canonical writer options.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WriterConfig {
    /// Spaces per indentation level.
    pub indent_size: usize,
    /// Active array delimiter.
    pub delimiter: Delimiter,
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            indent_size: 2,
            delimiter: Delimiter::Comma,
        }
    }
}

/// Canonical writer failure.
#[derive(Debug, Error)]
pub enum WriterError {
    /// Output I/O failed.
    #[error("TOON output I/O failed: {0}")]
    Io(#[from] io::Error),
    /// Recursive schema inference exceeded its bounded resource envelope.
    #[error("TOON schema {resource} exceeds limit {limit}")]
    Schema {
        /// Resource whose bound was exceeded.
        resource: &'static str,
        /// Maximum admitted resource amount.
        limit: usize,
    },
}

impl From<SchemaError> for WriterError {
    fn from(error: SchemaError) -> Self {
        let (resource, limit) = match error {
            SchemaError::Depth(limit) => ("depth", limit),
            SchemaError::Fields(limit) => ("field count", limit),
            SchemaError::Bytes(limit) => ("retained bytes", limit),
        };
        Self::Schema { resource, limit }
    }
}

/// Encodes one standalone value with no trailing newline.
///
/// # Panics
///
/// Panics if the in-memory output fails, if encoding violates its UTF-8
/// invariant, or if recursive schema inference exceeds its resource bounds.
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
/// Returns an output I/O error or a bounded recursive-schema error.
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
/// Returns the output sink's I/O error or a bounded recursive-schema error.
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
            Value::Object(object) => {
                Self::check_container_depth(1)?;
                if let Some(schema) = keyed_schema(object)? {
                    self.keyed_object(None, object, &schema, 0, None, 1)?;
                } else {
                    self.object(object, 0, 1)?;
                }
            }
            Value::Array(values) => self.array(None, values, 0, None, 1, 1)?,
            _ => {
                self.start_line(0, None)?;
                self.write_scalar(value, ScalarContext::Root)?;
            }
        }
        Ok(())
    }

    fn check_container_depth(depth: usize) -> Result<(), WriterError> {
        if depth > MAX_WRITER_DEPTH {
            return Err(SchemaError::Depth(MAX_WRITER_DEPTH).into());
        }
        Ok(())
    }

    fn object(
        &mut self,
        object: &Object,
        depth: usize,
        container_depth: usize,
    ) -> Result<(), WriterError> {
        Self::check_container_depth(container_depth)?;
        for (key, value) in object {
            self.member(key, value, depth, None, container_depth)?;
        }
        Ok(())
    }

    fn member(
        &mut self,
        key: &str,
        value: &Value,
        depth: usize,
        prefix: Option<&str>,
        container_depth: usize,
    ) -> Result<(), WriterError> {
        let content_depth = depth + usize::from(prefix.is_some()) + 1;
        let child_depth = container_depth.saturating_add(1);
        match value {
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
                self.start_line(depth, prefix)?;
                self.write_key(key)?;
                self.writer.write_all(b": ")?;
                self.write_scalar(value, ScalarContext::Object)?;
            }
            Value::Object(object) => {
                Self::check_container_depth(child_depth)?;
                if let Some(schema) = keyed_schema(object)? {
                    self.keyed_object(Some(key), object, &schema, depth, prefix, child_depth)?;
                } else {
                    self.start_line(depth, prefix)?;
                    self.write_key(key)?;
                    self.writer.write_all(b":")?;
                    self.object(object, content_depth, child_depth)?;
                }
            }
            Value::Array(values) => {
                self.array(Some(key), values, depth, prefix, content_depth, child_depth)?;
            }
        }
        Ok(())
    }
    fn keyed_object(
        &mut self,
        key: Option<&str>,
        object: &Object,
        schema: &RowSchema,
        depth: usize,
        prefix: Option<&str>,
        container_depth: usize,
    ) -> Result<(), WriterError> {
        Self::check_container_depth(container_depth)?;
        let content_depth = depth + usize::from(prefix.is_some()) + 1;
        self.start_line(depth, prefix)?;
        self.header(key, object.len(), true, Some(schema))?;
        for (entry_key, value) in object {
            let Value::Object(row) = value else {
                unreachable!("keyed eligibility checked")
            };
            let row_depth = container_depth.saturating_add(1);
            Self::check_container_depth(row_depth)?;
            self.start_line(content_depth, None)?;
            self.write_key(entry_key)?;
            self.writer.write_all(b": ")?;
            self.row(row, schema, &mut false, row_depth)?;
        }
        Ok(())
    }

    fn header(
        &mut self,
        key: Option<&str>,
        count: usize,
        keyed: bool,
        schema: Option<&RowSchema>,
    ) -> Result<(), WriterError> {
        write_table_header_colored(
            &mut self.writer,
            key,
            count,
            keyed,
            schema,
            self.config,
            self.palette,
        )
    }

    fn row(
        &mut self,
        object: &Object,
        schema: &RowSchema,
        wrote_cell: &mut bool,
        container_depth: usize,
    ) -> Result<(), WriterError> {
        Self::check_container_depth(container_depth)?;
        for field in &schema.fields {
            let value = &object[&field.key];
            if let Some(children) = &field.children {
                let Value::Object(nested) = value else {
                    unreachable!("recursive eligibility checked")
                };
                self.row(
                    nested,
                    children,
                    wrote_cell,
                    container_depth.saturating_add(1),
                )?;
            } else {
                if *wrote_cell {
                    self.write_delimiter(ColorRole::Object)?;
                }
                self.write_scalar(value, ScalarContext::Object)?;
                *wrote_cell = true;
            }
        }
        Ok(())
    }

    fn array(
        &mut self,
        key: Option<&str>,
        values: &[Value],
        depth: usize,
        prefix: Option<&str>,
        content_depth: usize,
        container_depth: usize,
    ) -> Result<(), WriterError> {
        Self::check_container_depth(container_depth)?;

        self.start_line(depth, prefix)?;
        if values.is_empty() && (key.is_some() || prefix.is_none()) {
            if let Some(key) = key {
                self.write_key(key)?;
                self.writer.write_all(b": ")?;
            }
            self.write_structure(ColorRole::Array, b"[]")?;
            return Ok(());
        }

        if values.iter().all(is_scalar) {
            self.header(key, values.len(), false, None)?;
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

        // Fields-bearing keyless headers are root-only, never anonymous list items.
        if (key.is_some() || prefix.is_none())
            && let Some(schema) = schema_for_values(values.iter())?
        {
            self.header(key, values.len(), false, Some(&schema))?;
            for value in values {
                let Value::Object(object) = value else {
                    unreachable!("tabular eligibility checked")
                };
                let row_depth = container_depth.saturating_add(1);
                Self::check_container_depth(row_depth)?;
                self.start_line(content_depth, None)?;
                self.row(object, &schema, &mut false, row_depth)?;
            }
            return Ok(());
        }

        self.header(key, values.len(), false, None)?;
        let item_depth = container_depth.saturating_add(1);
        for value in values {
            self.list_item(value, content_depth, item_depth)?;
        }
        Ok(())
    }

    fn list_item(
        &mut self,
        value: &Value,
        depth: usize,
        container_depth: usize,
    ) -> Result<(), WriterError> {
        match value {
            Value::Object(object) if object.is_empty() => {
                Self::check_container_depth(container_depth)?;
                self.start_line(depth, Some("-"))
            }
            Value::Object(object) => {
                Self::check_container_depth(container_depth)?;
                let mut members = object.iter();
                let (first, value) = members.next().expect("non-empty object");
                self.member(first, value, depth, Some("- "), container_depth)?;
                for (key, value) in members {
                    self.member(key, value, depth + 1, None, container_depth)?;
                }
                Ok(())
            }
            Value::Array(values) => {
                self.array(None, values, depth, Some("- "), depth + 1, container_depth)
            }
            _ => {
                self.start_line(depth, Some("- "))?;
                self.write_scalar(value, ScalarContext::Array)
            }
        }
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
    const CHUNK_SIZE: usize = 256;
    const HEX: &[u8; 16] = b"0123456789abcdef";

    write_span(writer, palette, quote_role, b"\"")?;
    let mut chunk = [0_u8; CHUNK_SIZE];
    let mut length = 0;
    for character in value.chars() {
        let mut encoded = [0_u8; 6];
        let mut utf8 = [0_u8; 4];
        let escaped: &[u8] = match character {
            '"' => b"\\\"",
            '\\' => b"\\\\",
            '\n' => b"\\n",
            '\r' => b"\\r",
            '\t' => b"\\t",
            character if character.is_control() => {
                let code = character as u32;
                encoded[0..2].copy_from_slice(b"\\u");
                encoded[2] = HEX[((code >> 12) & 0xf) as usize];
                encoded[3] = HEX[((code >> 8) & 0xf) as usize];
                encoded[4] = HEX[((code >> 4) & 0xf) as usize];
                encoded[5] = HEX[(code & 0xf) as usize];
                &encoded
            }
            _ => character.encode_utf8(&mut utf8).as_bytes(),
        };
        if length + escaped.len() > chunk.len() {
            write_span(writer, palette, content_role, &chunk[..length])?;
            length = 0;
        }
        chunk[length..length + escaped.len()].copy_from_slice(escaped);
        length += escaped.len();
    }
    if length != 0 {
        write_span(writer, palette, content_role, &chunk[..length])?;
    }
    write_span(writer, palette, quote_role, b"\"")
}

pub(crate) fn write_table_header_colored(
    output: &mut (impl Write + ?Sized),
    key: Option<&str>,
    count: usize,
    keyed: bool,
    schema: Option<&RowSchema>,
    config: WriterConfig,
    palette: Option<&ColorPalette>,
) -> Result<(), WriterError> {
    if let Some(key) = key {
        write_key(output, key, palette)?;
    }
    write_span(output, palette, ColorRole::Array, b"[")?;
    let mut digits = [0_u8; 20];
    let mut remaining = count;
    let mut start = digits.len();
    loop {
        start -= 1;
        digits[start] = b'0' + u8::try_from(remaining % 10).expect("decimal digit fits u8");
        remaining /= 10;
        if remaining == 0 {
            break;
        }
    }
    write_span(output, palette, ColorRole::Number, &digits[start..])?;
    if keyed {
        output.write_all(b":")?;
    }
    write_span(
        output,
        palette,
        ColorRole::Array,
        config.delimiter.header_suffix().as_bytes(),
    )?;
    write_span(output, palette, ColorRole::Array, b"]")?;
    if let Some(schema) = schema {
        write_field_group_colored(output, schema, config, palette)?;
    }
    output.write_all(b":")?;
    Ok(())
}

pub(crate) fn write_field_group_colored(
    output: &mut (impl Write + ?Sized),
    schema: &RowSchema,
    config: WriterConfig,
    palette: Option<&ColorPalette>,
) -> Result<(), WriterError> {
    write_span(output, palette, ColorRole::Object, b"{")?;
    for (index, field) in schema.fields.iter().enumerate() {
        if index != 0 {
            let mut bytes = [0_u8; 4];
            write_span(
                output,
                palette,
                ColorRole::Object,
                config
                    .delimiter
                    .character()
                    .encode_utf8(&mut bytes)
                    .as_bytes(),
            )?;
        }
        write_key(output, &field.key, palette)?;
        if let Some(children) = &field.children {
            write_field_group_colored(output, children, config, palette)?;
        }
    }
    write_span(output, palette, ColorRole::Object, b"}")?;
    Ok(())
}

fn keyed_schema(object: &Object) -> Result<Option<RowSchema>, WriterError> {
    if object.len() < 2 {
        return Ok(None);
    }
    let Some(schema) = schema_for_values(object.values())? else {
        return Ok(None);
    };
    Ok(Some(schema))
}

fn schema_for_values<'a>(
    mut values: impl Iterator<Item = &'a Value>,
) -> Result<Option<RowSchema>, WriterError> {
    let Some(Value::Object(first)) = values.next() else {
        return Ok(None);
    };
    let Some(schema) = RowSchema::from_object(first, SchemaLimits::default())? else {
        return Ok(None);
    };
    if values.all(|value| schema.matches_value(value)) {
        Ok(Some(schema))
    } else {
        Ok(None)
    }
}

fn safe_key(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.'))
}

pub(crate) fn string_quote_mask(value: &str) -> u8 {
    if looks_like_number(value) {
        return 8;
    }
    let mut mask = 0;
    let mut first = None;
    let mut last = None;
    let mut structural = false;
    let mut controls = false;
    for byte in value.bytes() {
        first.get_or_insert(byte);
        last = Some(byte);
        mask |= match byte {
            b',' => 1,
            b'\t' => 2,
            b'|' => 4,
            _ => 0,
        };
        structural |= matches!(byte, b'"' | b'\\' | b':' | b'[' | b']' | b'{' | b'}');
        controls |= byte < 0x20;
    }
    let unsafe_string = value.is_empty()
        || matches!(first, Some(b'-' | b'#' | b' ' | b'\t'))
        || matches!(last, Some(b' ' | b'\t'))
        || matches!(value, "null" | "true" | "false")
        || controls
        || structural;
    mask | if unsafe_string { 8 } else { 0 } | if value.starts_with('\u{feff}') { 16 } else { 0 }
}

fn safe_string(value: &str, delimiter: Delimiter, context: ScalarContext) -> bool {
    let forbidden_delimiter = match delimiter {
        Delimiter::Comma => 1,
        Delimiter::Tab => 2,
        Delimiter::Pipe => 4,
    };
    string_quote_mask(value)
        & (forbidden_delimiter
            | 8
            | if matches!(context, ScalarContext::Root) {
                16
            } else {
                0
            })
        == 0
}

fn looks_like_number(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut index = usize::from(
        bytes
            .first()
            .is_some_and(|byte| matches!(*byte, b'+' | b'-')),
    );
    let integer_start = index;
    while bytes.get(index).is_some_and(u8::is_ascii_digit) {
        index += 1;
    }
    if index == integer_start {
        return false;
    }
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        let fraction_start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if index == fraction_start {
            return false;
        }
    }
    if bytes
        .get(index)
        .is_some_and(|byte| matches!(byte, b'e' | b'E'))
    {
        index += 1;
        if bytes
            .get(index)
            .is_some_and(|byte| matches!(byte, b'+' | b'-'))
        {
            index += 1;
        }
        let exponent_start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if index == exponent_start {
            return false;
        }
    }
    index == bytes.len()
}

pub(crate) fn write_tabular_row_colored(
    output: &mut (impl Write + ?Sized),
    object: &Object,
    schema: &RowSchema,
    config: WriterConfig,
    palette: Option<&ColorPalette>,
) -> Result<(), WriterError> {
    let mut encoder = Encoder::new(output, config, palette);
    // PreparedArray has already emitted the enclosing root array.
    encoder.row(object, schema, &mut false, 2)
}

pub(crate) fn write_list_item_colored(
    output: &mut (impl Write + ?Sized),
    value: &Value,
    config: WriterConfig,
    palette: Option<&ColorPalette>,
) -> Result<(), WriterError> {
    Encoder::new(output, config, palette).list_item(value, 0, 2)
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
    #[test]
    fn recursive_tables_keep_first_row_field_order() {
        let value = Value::from_json(
            serde_json::from_str(
                r#"{"rows":[{"id":1,"profile":{"name":"Ada","active":true}},{"profile":{"active":false,"name":"Bob"},"id":2}]}"#,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            encode(&value, WriterConfig::default()),
            "rows[2]{id,profile{name,active}}:\n  1,Ada,true\n  2,Bob,false"
        );
    }

    #[test]
    fn keyed_tables_require_two_uniform_object_values() {
        let single =
            Value::from_json(serde_json::from_str(r#"{"one":{"id":1}}"#).unwrap()).unwrap();
        assert_eq!(encode(&single, WriterConfig::default()), "one:\n  id: 1");

        let keyed =
            Value::from_json(serde_json::from_str(r#"{"one":{"id":1},"two":{"id":2}}"#).unwrap())
                .unwrap();
        assert_eq!(
            encode(&keyed, WriterConfig::default()),
            "[2:]{id}:\n  one: 1\n  two: 2"
        );
        let nested = Value::from_json(
            serde_json::from_str(r#"{"catalog":{"one":{"id":1},"two":{"id":2}}}"#).unwrap(),
        )
        .unwrap();
        assert_eq!(
            encode(&nested, WriterConfig::default()),
            "catalog[2:]{id}:\n  one: 1\n  two: 2"
        );
    }

    #[test]
    fn strings_use_exact_numeric_hash_hyphen_and_control_quoting() {
        let values = ["05", "+1", "1e-6", "#tag", "-tag", "a\u{0001}b", "café"];
        let encoded = values
            .iter()
            .map(|value| encode(&Value::String((*value).into()), WriterConfig::default()))
            .collect::<Vec<_>>();
        assert_eq!(
            encoded,
            [
                "\"05\"",
                "\"+1\"",
                "\"1e-6\"",
                "\"#tag\"",
                "\"-tag\"",
                "\"a\\u0001b\"",
                "café",
            ]
        );
    }

    #[test]
    fn keys_are_unquoted_only_for_the_ascii_key_grammar() {
        let value =
            Value::from_json(serde_json::from_str(r#"{"café":1,"a-b":2,"a.b_2":3}"#).unwrap())
                .unwrap();
        assert_eq!(
            encode(&value, WriterConfig::default()),
            "\"café\": 1\n\"a-b\": 2\na.b_2: 3"
        );
    }
}
