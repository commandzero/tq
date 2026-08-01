//! Canonical, ordered TOON v3 writer over tq's exact value model.

use std::io::{self, Write};

use thiserror::Error;
use tq_core::{Object, Value};

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
#[must_use]
pub fn encode(value: &Value, config: WriterConfig) -> String {
    Encoder::new(config).encode(value)
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
    writer.write_all(encode(value, config).as_bytes())?;
    Ok(())
}

struct Encoder {
    config: WriterConfig,
    lines: Vec<String>,
}

impl Encoder {
    fn new(config: WriterConfig) -> Self {
        Self {
            config,
            lines: Vec::new(),
        }
    }

    fn encode(mut self, value: &Value) -> String {
        match value {
            Value::Object(object) => self.object(object, 0, true),
            Value::Array(values) => self.array(None, values, 0, None, 1),
            _ => self.lines.push(self.scalar(value, ScalarContext::Root)),
        }
        self.lines.join("\n")
    }

    fn object(&mut self, object: &Object, depth: usize, allow_folding: bool) {
        for (key, value) in object {
            let member_folding = allow_folding && self.fold_allowed(object, key, value);
            self.member(key, value, depth, None, member_folding);
        }
    }

    fn member(
        &mut self,
        key: &str,
        value: &Value,
        depth: usize,
        prefix: Option<&str>,
        allow_folding: bool,
    ) {
        let (folded_key, folded_value) = if allow_folding {
            self.folded(key, value)
        } else {
            (key.to_owned(), value)
        };
        let folded_here = folded_key != key;
        let key = Self::key(&folded_key);
        let mut head = self.indent(depth);
        head.push_str(prefix.unwrap_or(""));
        let logical_depth = depth + usize::from(prefix.is_some());
        match folded_value {
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
                head.push_str(&key);
                head.push_str(": ");
                head.push_str(&self.scalar(folded_value, ScalarContext::Object));
                self.lines.push(head);
            }
            Value::Object(object) => {
                head.push_str(&key);
                head.push(':');
                self.lines.push(head);
                self.object(object, logical_depth + 1, allow_folding && !folded_here);
            }
            Value::Array(values) => {
                self.array(Some(&key), values, depth, prefix, logical_depth + 1);
            }
        }
    }

    fn folded<'a>(&self, first: &str, value: &'a Value) -> (String, &'a Value) {
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
    ) {
        let mut head = self.indent(depth);
        head.push_str(prefix.unwrap_or(""));
        if let Some(key) = key {
            head.push_str(key);
        }
        head.push('[');
        head.push_str(&values.len().to_string());
        head.push_str(self.config.delimiter.header_suffix());
        head.push(']');

        if values.iter().all(is_scalar) {
            head.push(':');
            if !values.is_empty() {
                head.push(' ');
                let delimiter = self.config.delimiter.character().to_string();
                head.push_str(
                    &values
                        .iter()
                        .map(|value| self.scalar(value, ScalarContext::Array))
                        .collect::<Vec<_>>()
                        .join(&delimiter),
                );
            }
            self.lines.push(head);
            return;
        }

        if let Some(fields) = tabular_fields(values) {
            head.push('{');
            let delimiter = self.config.delimiter.character().to_string();
            head.push_str(
                &fields
                    .iter()
                    .map(|field| Self::key(field))
                    .collect::<Vec<_>>()
                    .join(&delimiter),
            );
            head.push_str("}:");
            self.lines.push(head);
            for value in values {
                let Value::Object(object) = value else {
                    unreachable!("tabular eligibility checked")
                };
                let mut row = self.indent(content_depth);
                row.push_str(
                    &fields
                        .iter()
                        .map(|field| self.scalar(&object[*field], ScalarContext::Array))
                        .collect::<Vec<_>>()
                        .join(&delimiter),
                );
                self.lines.push(row);
            }
            return;
        }

        head.push(':');
        self.lines.push(head);
        for value in values {
            match value {
                Value::Object(object) if object.is_empty() => {
                    let mut line = self.indent(content_depth);
                    line.push('-');
                    self.lines.push(line);
                }
                Value::Object(object) => {
                    let mut members = object.iter();
                    let (first, value) = members.next().expect("non-empty object");
                    let allow_folding = self.fold_allowed(object, first, value);
                    self.member(first, value, content_depth, Some("- "), allow_folding);
                    for (key, value) in members {
                        let allow_folding = self.fold_allowed(object, key, value);
                        self.member(key, value, content_depth + 1, None, allow_folding);
                    }
                }
                Value::Array(nested) => {
                    self.array(None, nested, content_depth, Some("- "), content_depth + 1);
                }
                _ => {
                    let mut line = self.indent(content_depth);
                    line.push_str("- ");
                    line.push_str(&self.scalar(value, ScalarContext::Array));
                    self.lines.push(line);
                }
            }
        }
    }

    fn indent(&self, depth: usize) -> String {
        " ".repeat(depth.saturating_mul(self.config.indent_size))
    }

    fn key(key: &str) -> String {
        if safe_key(key) {
            key.to_owned()
        } else {
            quote(key)
        }
    }

    fn scalar(&self, value: &Value, context: ScalarContext) -> String {
        match value {
            Value::Null => "null".to_owned(),
            Value::Bool(value) => value.to_string(),
            Value::Number(value) => value.to_string(),
            Value::String(value) => {
                if safe_string(value, self.config.delimiter, context) {
                    value.to_string()
                } else {
                    quote(value)
                }
            }
            Value::Array(_) | Value::Object(_) => unreachable!("scalar context"),
        }
    }
}

#[derive(Clone, Copy)]
enum ScalarContext {
    Root,
    Object,
    Array,
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
    tq_core::Number::parse(value).is_ok()
        || value
            .strip_prefix('0')
            .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_digit()))
}

fn quote(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            _ => output.push(character),
        }
    }
    output.push('"');
    output
}

fn identifier_segment(value: &str) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|character| character.is_alphabetic() || character == '_')
        && characters.all(|character| character.is_alphanumeric() || character == '_')
}

pub(crate) fn encode_array_scalar(value: &Value, config: WriterConfig) -> String {
    Encoder::new(config).scalar(value, ScalarContext::Array)
}

pub(crate) fn encode_tabular_row(
    object: &Object,
    fields: &[std::sync::Arc<str>],
    config: WriterConfig,
) -> String {
    let encoder = Encoder::new(config);
    let delimiter = config.delimiter.character().to_string();
    fields
        .iter()
        .map(|field| encoder.scalar(&object[field], ScalarContext::Array))
        .collect::<Vec<_>>()
        .join(&delimiter)
}

pub(crate) fn encode_list_item(value: &Value, config: WriterConfig) -> String {
    let mut encoder = Encoder::new(config);
    match value {
        Value::Object(object) if object.is_empty() => encoder.lines.push("-".to_owned()),
        Value::Object(object) => {
            let mut members = object.iter();
            let (first, value) = members.next().expect("non-empty object");
            let allow_folding = encoder.fold_allowed(object, first, value);
            encoder.member(first, value, 0, Some("- "), allow_folding);
            for (key, value) in members {
                let allow_folding = encoder.fold_allowed(object, key, value);
                encoder.member(key, value, 1, None, allow_folding);
            }
        }
        Value::Array(values) => encoder.array(None, values, 0, Some("- "), 1),
        _ => encoder
            .lines
            .push(format!("- {}", encoder.scalar(value, ScalarContext::Array))),
    }
    encoder.lines.join("\n")
}

pub(crate) fn render_key(key: &str) -> String {
    Encoder::key(key)
}

#[cfg(test)]
mod tests {
    use tq_core::Value;

    use super::{Delimiter, WriterConfig, encode};

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
}
