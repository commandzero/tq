//! jq-compatible string filters and string division.

use crate::{Value, VmError, format};

pub(crate) fn ascii_upcase(input: &Value) -> Result<Value, VmError> {
    map_string(input, "ascii_upcase", str::to_ascii_uppercase)
}

pub(crate) fn ascii_downcase(input: &Value) -> Result<Value, VmError> {
    map_string(input, "ascii_downcase", str::to_ascii_lowercase)
}

pub(crate) fn trim(input: &Value) -> Result<Value, VmError> {
    map_string(input, "trim", |value| value.trim().to_owned())
}

pub(crate) fn ltrim(input: &Value) -> Result<Value, VmError> {
    map_string(input, "ltrim", |value| value.trim_start().to_owned())
}

pub(crate) fn rtrim(input: &Value) -> Result<Value, VmError> {
    map_string(input, "rtrim", |value| value.trim_end().to_owned())
}

pub(crate) fn ltrimstr(input: &Value, prefix: &Value) -> Result<Value, VmError> {
    let input = string(input, "ltrimstr")?;
    let prefix = string(prefix, "ltrimstr")?;
    Ok(Value::string(input.strip_prefix(prefix).unwrap_or(input)))
}

pub(crate) fn trimstr(input: &Value, affix: &Value) -> Result<Value, VmError> {
    let input = string(input, "trimstr")?;
    let affix = string(affix, "trimstr")?;
    let input = input.strip_prefix(affix).unwrap_or(input);
    Ok(Value::string(input.strip_suffix(affix).unwrap_or(input)))
}

pub(crate) fn rtrimstr(input: &Value, suffix: &Value) -> Result<Value, VmError> {
    let input = string(input, "rtrimstr")?;
    let suffix = string(suffix, "rtrimstr")?;
    Ok(Value::string(input.strip_suffix(suffix).unwrap_or(input)))
}

pub(crate) fn startswith(input: &Value, prefix: &Value) -> Result<Value, VmError> {
    let input = string(input, "startswith")?;
    let prefix = string(prefix, "startswith")?;
    Ok(Value::Bool(input.starts_with(prefix)))
}

pub(crate) fn endswith(input: &Value, suffix: &Value) -> Result<Value, VmError> {
    let input = string(input, "endswith")?;
    let suffix = string(suffix, "endswith")?;
    Ok(Value::Bool(input.ends_with(suffix)))
}

pub(crate) fn join(
    input: &Value,
    separator: &Value,
    output_limit: usize,
) -> Result<Value, VmError> {
    match input {
        Value::Array(values) => join_values(values.iter(), separator, output_limit),
        Value::Object(values) => join_values(values.values(), separator, output_limit),
        input => Err(type_error("join", input, "array or object")),
    }
}

fn join_values<'a>(
    values: impl Iterator<Item = &'a Value>,
    separator: &Value,
    output_limit: usize,
) -> Result<Value, VmError> {
    let mut output = String::new();
    for (index, value) in values.enumerate() {
        if index > 0 {
            let separator = match separator {
                Value::Null => "",
                Value::String(separator) => separator,
                value => return Err(cannot_add("string", value)),
            };
            push_bounded(&mut output, separator, output_limit)?;
        }
        let scalar = match value {
            Value::Null => String::new(),
            Value::Bool(value) => (if *value { "true" } else { "false" }).to_owned(),
            Value::Number(value) => value.to_string(),
            Value::String(value) => value.to_string(),
            value => return Err(cannot_add("string", value)),
        };
        push_bounded(&mut output, &scalar, output_limit)?;
    }
    Ok(Value::string(output))
}

fn push_bounded(output: &mut String, value: &str, output_limit: usize) -> Result<(), VmError> {
    let length = output
        .len()
        .checked_add(value.len())
        .filter(|length| *length <= output_limit)
        .ok_or(VmError::Resource {
            resource: "output-bytes",
        })?;
    output
        .try_reserve_exact(length - output.len())
        .map_err(|_| VmError::Resource {
            resource: "output-bytes",
        })?;
    output.push_str(value);
    Ok(())
}

pub(crate) fn toboolean(input: &Value) -> Result<Value, VmError> {
    match input {
        Value::Bool(value) => Ok(Value::Bool(*value)),
        Value::String(value) if value.as_ref() == "true" => Ok(Value::Bool(true)),
        Value::String(value) if value.as_ref() == "false" => Ok(Value::Bool(false)),
        value => Err(VmError::Runtime {
            message: format!("{} cannot be parsed as a boolean", display(value)).into(),
        }),
    }
}

pub(crate) fn divide(left: &Value, right: &Value) -> Result<Value, VmError> {
    let left = string(left, "divide")?;
    let right = string(right, "divide")?;
    Ok(Value::array(if right.is_empty() {
        left.chars()
            .map(|value| Value::string(value.to_string()))
            .collect::<Vec<_>>()
    } else {
        left.split(right)
            .map(|value| Value::string(value.to_owned()))
            .collect::<Vec<_>>()
    }))
}

pub(crate) fn urid(input: &Value, output_limit: usize) -> Result<Value, VmError> {
    let input = format::text(input, output_limit)?;
    let mut bytes = Vec::with_capacity(input.len());
    let mut chars = input.as_bytes().iter().copied();
    while let Some(byte) = chars.next() {
        if byte != b'%' {
            bytes.push(byte);
            continue;
        }
        let Some(high) = chars.next().and_then(hex) else {
            return Err(invalid_uri(&input));
        };
        let Some(low) = chars.next().and_then(hex) else {
            return Err(invalid_uri(&input));
        };
        bytes.push(high << 4 | low);
    }
    if bytes.len() > output_limit {
        return Err(VmError::Resource {
            resource: "output-bytes",
        });
    }
    String::from_utf8(bytes)
        .map(Value::string)
        .map_err(|_| invalid_uri(&input))
}

fn map_string(
    input: &Value,
    operation: &str,
    map: impl FnOnce(&str) -> String,
) -> Result<Value, VmError> {
    Ok(Value::string(map(string(input, operation)?)))
}

fn string<'a>(value: &'a Value, operation: &str) -> Result<&'a str, VmError> {
    match value {
        Value::String(value) => Ok(value),
        value => Err(type_error(operation, value, "string")),
    }
}

fn type_error(operation: &str, value: &Value, expected: &str) -> VmError {
    VmError::Runtime {
        message: format!("{operation} requires {expected}, got {}", kind(value)).into(),
    }
}

fn cannot_add(left: &str, right: &Value) -> VmError {
    VmError::Runtime {
        message: format!("cannot add {left} and {}", kind(right)).into(),
    }
}

fn invalid_uri(input: &str) -> VmError {
    VmError::Runtime {
        message: format!("string ({input:?}) is not a valid uri encoding").into(),
    }
}

fn display(value: &Value) -> String {
    match value {
        Value::String(value) => format!("string ({value:?})"),
        Value::Null => "null (null)".to_owned(),
        Value::Bool(value) => format!("boolean ({value})"),
        Value::Number(value) => format!("number ({value})"),
        Value::Array(value) => format!("array ({value:?})"),
        Value::Object(value) => format!("object ({value:?})"),
    }
}

fn kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
