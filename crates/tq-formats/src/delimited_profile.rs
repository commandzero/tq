//! Scalar distinctions shared by delimited decoding and output quoting.

use tq_core::Value;

/// Bounds for one delimited logical row and its scalar fields.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DelimitedLimits {
    /// Maximum encoded bytes in one logical row, including its ending.
    pub row_bytes: usize,
    /// Maximum decoded UTF-8 bytes in one field.
    pub field_bytes: usize,
    /// Maximum fields in a header or row.
    pub fields: usize,
}

impl Default for DelimitedLimits {
    fn default() -> Self {
        Self {
            row_bytes: 16 * 1024 * 1024,
            field_bytes: 8 * 1024 * 1024,
            fields: 65_536,
        }
    }
}

pub(crate) fn scalar(text: &str) -> Value {
    inferred_scalar(text).unwrap_or_else(|| Value::string(text))
}

pub(crate) fn inferred_scalar(text: &str) -> Option<Value> {
    match text {
        "" => Some(Value::Null),
        "true" => Some(Value::Bool(true)),
        "false" => Some(Value::Bool(false)),
        _ => {
            if matches!(text.as_bytes().first(), Some(b'-' | b'0'..=b'9'))
                && text.trim() == text
                && let Ok(value @ Value::Number(_)) = serde_json::from_str::<Value>(text)
            {
                return Some(value);
            }
            None
        }
    }
}
