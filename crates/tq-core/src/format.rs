//! Bounded jq-compatible text serialization and format filters.

use std::{io, sync::Arc};

use base64::{Engine as _, engine::general_purpose::STANDARD};

use crate::{Number, Object, Value, VmError};

pub(crate) fn is_supported(name: &str) -> bool {
    matches!(
        name,
        "@text" | "@json" | "@html" | "@uri" | "@csv" | "@tsv" | "@sh" | "@base64" | "@base64d"
    )
}

pub(crate) fn apply(name: &str, value: &Value, output_limit: usize) -> Result<Value, VmError> {
    let output = match name {
        "@text" => return text(value, output_limit).map(Value::String),
        "@json" => bounded_json(value, output_limit)?,
        "@html" => escape_html(&text(value, output_limit)?, output_limit)?,
        "@uri" => escape_uri(&text(value, output_limit)?, output_limit)?,
        "@csv" => format_csv(value, output_limit)?,
        "@tsv" => format_tsv(value, output_limit)?,
        "@sh" => format_shell(value, output_limit)?,
        "@base64" => format_base64(value, output_limit)?,
        "@base64d" => decode_base64(value, output_limit)?,
        _ => {
            return Err(VmError::InvalidProgram {
                message: "unknown resolved format",
            });
        }
    };
    Ok(Value::string(output))
}

pub(crate) fn text(value: &Value, output_limit: usize) -> Result<Arc<str>, VmError> {
    match value {
        Value::String(value) if value.len() <= output_limit => Ok(Arc::clone(value)),
        Value::String(_) => Err(resource()),
        value => bounded_json(value, output_limit).map(Arc::from),
    }
}

struct BoundedWriter {
    output: String,
    limit: usize,
}

impl BoundedWriter {
    fn new(limit: usize) -> Self {
        Self {
            output: String::new(),
            limit,
        }
    }

    fn push(&mut self, value: &str) -> Result<(), VmError> {
        let length = self
            .output
            .len()
            .checked_add(value.len())
            .filter(|length| *length <= self.limit)
            .ok_or_else(resource)?;
        self.output
            .try_reserve_exact(length - self.output.len())
            .map_err(|_| resource())?;
        self.output.push_str(value);
        Ok(())
    }

    fn finish(self) -> String {
        self.output
    }
}

impl std::fmt::Write for BoundedWriter {
    fn write_str(&mut self, value: &str) -> std::fmt::Result {
        self.push(value).map_err(|_| std::fmt::Error)
    }
}

impl io::Write for BoundedWriter {
    fn write(&mut self, value: &[u8]) -> io::Result<usize> {
        let value = std::str::from_utf8(value)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        self.push(value)
            .map_err(|_| io::Error::other("output byte limit exceeded"))?;
        Ok(value.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn escape_html(input: &str, output_limit: usize) -> Result<String, VmError> {
    let mut output = BoundedWriter::new(output_limit);
    let mut copied = 0;
    for (index, character) in input.char_indices() {
        let escaped = match character {
            '<' => "&lt;",
            '>' => "&gt;",
            '&' => "&amp;",
            '\'' => "&apos;",
            '"' => "&quot;",
            _ => continue,
        };
        output.push(&input[copied..index])?;
        output.push(escaped)?;
        copied = index + character.len_utf8();
    }
    output.push(&input[copied..])?;
    Ok(output.finish())
}

fn escape_uri(input: &str, output_limit: usize) -> Result<String, VmError> {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut output = BoundedWriter::new(output_limit);
    for &byte in input.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            let character = [byte];
            output.push(std::str::from_utf8(&character).expect("ASCII byte is valid UTF-8"))?;
        } else {
            let encoded = [
                b'%',
                HEX[usize::from(byte >> 4)],
                HEX[usize::from(byte & 0x0f)],
            ];
            output.push(std::str::from_utf8(&encoded).expect("URI escape is valid ASCII"))?;
        }
    }
    Ok(output.finish())
}

fn format_csv(value: &Value, output_limit: usize) -> Result<String, VmError> {
    let Value::Array(values) = value else {
        return Err(type_error("@csv", value, "array"));
    };
    let mut output = BoundedWriter::new(output_limit);
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            output.push(",")?;
        }
        match value {
            Value::String(value) => {
                output.push("\"")?;
                let mut copied = 0;
                for (index, _) in value.match_indices('"') {
                    output.push(&value[copied..index])?;
                    output.push("\"\"")?;
                    copied = index + 1;
                }
                output.push(&value[copied..])?;
                output.push("\"")?;
            }
            Value::Null => {}
            Value::Bool(_) | Value::Number(_) => write_plain_scalar(&mut output, value)?,
            Value::Array(_) | Value::Object(_) => {
                return Err(type_error("@csv field", value, "scalar"));
            }
        }
    }
    Ok(output.finish())
}

fn format_tsv(value: &Value, output_limit: usize) -> Result<String, VmError> {
    let Value::Array(values) = value else {
        return Err(type_error("@tsv", value, "array"));
    };
    let mut output = BoundedWriter::new(output_limit);
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            output.push("\t")?;
        }
        match value {
            Value::String(value) => escape_tsv_string(&mut output, value)?,
            Value::Null => {}
            Value::Bool(_) | Value::Number(_) => write_plain_scalar(&mut output, value)?,
            Value::Array(_) | Value::Object(_) => {
                return Err(type_error("@tsv field", value, "scalar"));
            }
        }
    }
    Ok(output.finish())
}

fn escape_tsv_string(output: &mut BoundedWriter, input: &str) -> Result<(), VmError> {
    let mut copied = 0;
    for (index, character) in input.char_indices() {
        let escaped = match character {
            '\\' => "\\\\",
            '\t' => "\\t",
            '\r' => "\\r",
            '\n' => "\\n",
            _ => continue,
        };
        output.push(&input[copied..index])?;
        output.push(escaped)?;
        copied = index + character.len_utf8();
    }
    output.push(&input[copied..])
}

fn format_shell(value: &Value, output_limit: usize) -> Result<String, VmError> {
    let mut output = BoundedWriter::new(output_limit);
    if let Value::Array(values) = value {
        for (index, value) in values.iter().enumerate() {
            if index > 0 {
                output.push(" ")?;
            }
            write_shell_scalar(&mut output, value)?;
        }
    } else {
        write_shell_scalar(&mut output, value)?;
    }
    Ok(output.finish())
}

fn write_shell_scalar(output: &mut BoundedWriter, value: &Value) -> Result<(), VmError> {
    match value {
        Value::String(value) => {
            output.push("'")?;
            let mut copied = 0;
            for (index, _) in value.match_indices('\'') {
                output.push(&value[copied..index])?;
                output.push("'\\''")?;
                copied = index + 1;
            }
            output.push(&value[copied..])?;
            output.push("'")
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => write_plain_scalar(output, value),
        Value::Array(_) | Value::Object(_) => Err(type_error("@sh", value, "scalar")),
    }
}

fn write_plain_scalar(output: &mut BoundedWriter, value: &Value) -> Result<(), VmError> {
    match value {
        Value::Null => output.push("null"),
        Value::Bool(value) => output.push(if *value { "true" } else { "false" }),
        Value::Number(value) => write_jq_number(output, value),
        Value::String(_) | Value::Array(_) | Value::Object(_) => Err(VmError::InvalidProgram {
            message: "non-plain scalar reached scalar formatter",
        }),
    }
}

fn format_base64(value: &Value, output_limit: usize) -> Result<String, VmError> {
    let input = text(value, output_limit)?;
    let encoded_length = input
        .len()
        .checked_add(2)
        .and_then(|length| length.checked_div(3))
        .and_then(|groups| groups.checked_mul(4))
        .filter(|length| *length <= output_limit)
        .ok_or_else(resource)?;
    let mut output = String::new();
    output
        .try_reserve_exact(encoded_length)
        .map_err(|_| resource())?;
    STANDARD.encode_string(input.as_bytes(), &mut output);
    Ok(output)
}

fn decode_base64(value: &Value, output_limit: usize) -> Result<String, VmError> {
    let maximum_input = output_limit
        .checked_add(2)
        .and_then(|length| length.checked_div(3))
        .and_then(|groups| groups.checked_mul(4))
        .ok_or_else(resource)?;
    let input = match value {
        Value::String(value) => Arc::clone(value),
        value => bounded_json(value, maximum_input)?.into(),
    };
    if input.len() > maximum_input {
        return Err(resource());
    }
    let decoded_length = decoded_base64_length(&input)?;
    if decoded_length > output_limit {
        return Err(resource());
    }
    let mut decoded = vec![0; decoded_length];
    let written = STANDARD
        .decode_slice(input.as_bytes(), &mut decoded)
        .map_err(|_| invalid_base64())?;
    debug_assert_eq!(written, decoded_length);
    String::from_utf8(decoded).map_err(|_| VmError::Runtime {
        message: "@base64d decoded bytes are not valid UTF-8".into(),
    })
}

fn decoded_base64_length(input: &str) -> Result<usize, VmError> {
    let bytes = input.as_bytes();
    if !bytes.len().is_multiple_of(4) {
        return Err(invalid_base64());
    }
    let padding = usize::from(bytes.ends_with(b"=")) + usize::from(bytes.ends_with(b"=="));
    let data_length = bytes.len() - padding;
    if bytes[..data_length]
        .iter()
        .any(|byte| !byte.is_ascii_alphanumeric() && !matches!(byte, b'+' | b'/'))
        || bytes[data_length..].iter().any(|byte| *byte != b'=')
    {
        return Err(invalid_base64());
    }
    bytes
        .len()
        .checked_div(4)
        .and_then(|groups| groups.checked_mul(3))
        .and_then(|maximum| maximum.checked_sub(padding))
        .ok_or_else(resource)
}

fn invalid_base64() -> VmError {
    VmError::Runtime {
        message: "@base64d input is not valid RFC 4648 base64".into(),
    }
}

fn write_jq_number(writer: &mut BoundedWriter, value: &Number) -> Result<(), VmError> {
    let literal = value.to_string();
    let Some(exponent) = literal.find(['e', 'E']) else {
        return writer.push(&literal);
    };
    let sign = exponent + 1;
    if literal
        .as_bytes()
        .get(sign)
        .is_some_and(|byte| *byte == b'+' || *byte == b'-')
    {
        return writer.push(&literal);
    }
    writer.push(&literal[..sign])?;
    writer.push("+")?;
    writer.push(&literal[sign..])
}

enum JsonFrame<'a> {
    Value(&'a Value),
    Array(&'a [Value], usize),
    Object(&'a Object, usize),
}

pub(crate) fn bounded_json(value: &Value, output_limit: usize) -> Result<String, VmError> {
    let mut writer = BoundedWriter::new(output_limit);
    let mut frames = vec![JsonFrame::Value(value)];
    while let Some(frame) = frames.pop() {
        match frame {
            JsonFrame::Value(Value::Null) => writer.push("null")?,
            JsonFrame::Value(Value::Bool(value)) => {
                writer.push(if *value { "true" } else { "false" })?;
            }
            JsonFrame::Value(Value::Number(value)) => write_jq_number(&mut writer, value)?,
            JsonFrame::Value(Value::String(value)) => {
                serde_json::to_writer(&mut writer, value.as_ref()).map_err(|_| resource())?;
            }
            JsonFrame::Value(Value::Array(values)) => {
                writer.push("[")?;
                frames.push(JsonFrame::Array(values, 0));
            }
            JsonFrame::Value(Value::Object(values)) => {
                writer.push("{")?;
                frames.push(JsonFrame::Object(values, 0));
            }
            JsonFrame::Array(values, next) => {
                if next == values.len() {
                    writer.push("]")?;
                } else {
                    if next > 0 {
                        writer.push(",")?;
                    }
                    frames.push(JsonFrame::Array(values, next + 1));
                    frames.push(JsonFrame::Value(&values[next]));
                }
            }
            JsonFrame::Object(values, next) => {
                if next == values.len() {
                    writer.push("}")?;
                } else {
                    if next > 0 {
                        writer.push(",")?;
                    }
                    let Some((key, value)) = values.get_index(next) else {
                        return Err(VmError::InvalidProgram {
                            message: "object entry missing during serialization",
                        });
                    };
                    serde_json::to_writer(&mut writer, key.as_ref()).map_err(|_| resource())?;
                    writer.push(":")?;
                    frames.push(JsonFrame::Object(values, next + 1));
                    frames.push(JsonFrame::Value(value));
                }
            }
        }
    }
    Ok(writer.finish())
}

const fn resource() -> VmError {
    VmError::Resource {
        resource: "output-bytes",
    }
}

fn type_error(operation: &str, value: &Value, expected: &str) -> VmError {
    let kind = match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    };
    VmError::Runtime {
        message: format!("{operation} requires {expected}, got {kind}").into(),
    }
}

#[cfg(test)]
mod tests {
    use crate::{Value, VmError};

    use super::{apply, decoded_base64_length};

    fn value(json: serde_json::Value) -> Value {
        Value::from_json(json).unwrap()
    }

    fn formatted(name: &str, input: serde_json::Value) -> String {
        let Value::String(output) = apply(name, &value(input), 4096).unwrap() else {
            panic!("format output must be a string");
        };
        output.to_string()
    }

    #[test]
    fn text_json_html_and_uri_match_jq_literals() {
        assert_eq!(
            formatted("@text", serde_json::json!({"a": 1})),
            r#"{"a":1}"#
        );
        assert_eq!(formatted("@text", serde_json::json!("x")), "x");
        assert_eq!(formatted("@json", serde_json::json!("x")), r#""x""#);
        assert_eq!(
            formatted("@html", serde_json::json!("<>&'\"")),
            "&lt;&gt;&amp;&apos;&quot;"
        );
        assert_eq!(
            formatted("@uri", serde_json::json!("a b/é?~")),
            "a%20b%2F%C3%A9%3F~"
        );
    }

    #[test]
    fn escaping_obeys_exact_output_limit() {
        assert_eq!(formatted("@uri", serde_json::json!(" ")), "%20");
        assert!(matches!(
            apply("@uri", &Value::string(" "), 2),
            Err(VmError::Resource {
                resource: "output-bytes"
            })
        ));
    }

    #[test]
    fn csv_tsv_and_shell_match_jq_literals_and_type_errors() {
        let row = serde_json::json!(["a,\"b\n", 1, null, true]);
        assert_eq!(formatted("@csv", row.clone()), "\"a,\"\"b\n\",1,,true");
        assert_eq!(formatted("@csv", serde_json::json!([])), "");
        assert_eq!(
            formatted("@csv", serde_json::json!(["", "é"])),
            "\"\",\"é\""
        );
        assert_eq!(
            formatted("@tsv", serde_json::json!(["a\\b\t\r\n", 1, null, false])),
            "a\\\\b\\t\\r\\n\t1\t\tfalse"
        );
        assert_eq!(formatted("@tsv", serde_json::json!([])), "");
        assert_eq!(formatted("@tsv", serde_json::json!(["é"])), "é");
        assert_eq!(
            formatted("@sh", serde_json::json!(["O'Hara", 1, null, true])),
            "'O'\\''Hara' 1 null true"
        );
        assert_eq!(formatted("@sh", serde_json::json!("")), "''");
        assert_eq!(formatted("@sh", serde_json::json!([])), "");
        for (name, input) in [
            ("@csv", serde_json::json!(1)),
            ("@tsv", serde_json::json!([{}])),
            ("@sh", serde_json::json!({"a": 1})),
        ] {
            assert!(matches!(
                apply(name, &value(input), 4096),
                Err(VmError::Runtime { .. })
            ));
        }
    }

    #[test]
    fn base64_round_trips_and_rejects_invalid_or_oversized_data() {
        assert_eq!(formatted("@base64", serde_json::json!("hello")), "aGVsbG8=");
        assert_eq!(
            formatted("@base64d", serde_json::json!("aGVsbG8=")),
            "hello"
        );
        assert_eq!(
            formatted("@base64", serde_json::json!({"a": 1})),
            "eyJhIjoxfQ=="
        );
        for encoded in ["not base64", "//8="] {
            assert!(matches!(
                apply("@base64d", &Value::string(encoded), 4096),
                Err(VmError::Runtime { .. })
            ));
        }
        assert!(apply("@base64", &Value::string("a"), 4).is_ok());
        assert!(matches!(
            apply("@base64", &Value::string("a"), 3),
            Err(VmError::Resource {
                resource: "output-bytes"
            })
        ));
        assert!(matches!(
            apply("@base64d", &Value::string("aGVsbG8="), 4),
            Err(VmError::Resource {
                resource: "output-bytes"
            })
        ));
        assert_eq!(decoded_base64_length("YWI=").unwrap(), 2);
        assert!(matches!(
            apply("@base64d", &Value::string("YWI="), 1),
            Err(VmError::Resource {
                resource: "output-bytes"
            })
        ));
    }
}
