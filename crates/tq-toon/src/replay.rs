//! Private structural replay records for bounded container preparation.

use std::sync::Arc;

use tq_core::{Number, Object, Value};

use crate::ScalarToken;

const NULL: u8 = 0;
const FALSE: u8 = 1;
const TRUE: u8 = 2;
const NUMBER: u8 = 3;
const STRING: u8 = 4;
const ARRAY_START: u8 = 5;
const ARRAY_END: u8 = 6;
const OBJECT_START: u8 = 7;
const KEY: u8 = 8;
const OBJECT_END: u8 = 9;

pub(crate) fn encode(value: &Value) -> Vec<u8> {
    let mut output = Vec::new();
    encode_into(value, &mut output);
    output
}

pub(crate) fn encode_scalar(value: ScalarToken<'_>) -> Vec<u8> {
    let mut output = Vec::new();
    match value {
        ScalarToken::Null => output.push(NULL),
        ScalarToken::Bool(false) => output.push(FALSE),
        ScalarToken::Bool(true) => output.push(TRUE),
        ScalarToken::Number(value) => {
            output.push(NUMBER);
            write_bytes(value.as_bytes(), &mut output);
        }
        ScalarToken::String(value) => {
            output.push(STRING);
            write_bytes(value.as_bytes(), &mut output);
        }
    }
    output
}

pub(crate) fn decode_scalar(bytes: &[u8]) -> Result<ScalarToken<'_>, &'static str> {
    let mut decoder = Decoder { bytes, position: 0 };
    let value = match decoder.byte()? {
        NULL => ScalarToken::Null,
        FALSE => ScalarToken::Bool(false),
        TRUE => ScalarToken::Bool(true),
        NUMBER => ScalarToken::Number(decoder.text()?),
        STRING => ScalarToken::String(decoder.text()?),
        _ => return Err("replay value is not a scalar"),
    };
    if decoder.position != bytes.len() {
        return Err("trailing structural replay bytes");
    }
    Ok(value)
}

pub(crate) fn decode(bytes: &[u8]) -> Result<Value, &'static str> {
    let mut decoder = Decoder { bytes, position: 0 };
    let value = decoder.value()?;
    if decoder.position != bytes.len() {
        return Err("trailing structural replay bytes");
    }
    Ok(value)
}

fn encode_into(value: &Value, output: &mut Vec<u8>) {
    match value {
        Value::Null => output.push(NULL),
        Value::Bool(false) => output.push(FALSE),
        Value::Bool(true) => output.push(TRUE),
        Value::Number(value) => {
            output.push(NUMBER);
            write_bytes(value.to_string().as_bytes(), output);
        }
        Value::String(value) => {
            output.push(STRING);
            write_bytes(value.as_bytes(), output);
        }
        Value::Array(values) => {
            output.push(ARRAY_START);
            write_length(values.len(), output);
            for value in values.iter() {
                encode_into(value, output);
            }
            output.push(ARRAY_END);
        }
        Value::Object(values) => {
            output.push(OBJECT_START);
            write_length(values.len(), output);
            for (key, value) in values.iter() {
                output.push(KEY);
                write_bytes(key.as_bytes(), output);
                encode_into(value, output);
            }
            output.push(OBJECT_END);
        }
    }
}

fn write_bytes(bytes: &[u8], output: &mut Vec<u8>) {
    write_length(bytes.len(), output);
    output.extend_from_slice(bytes);
}

fn write_length(length: usize, output: &mut Vec<u8>) {
    output.extend_from_slice(&u64::try_from(length).unwrap_or(u64::MAX).to_le_bytes());
}

struct Decoder<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Decoder<'a> {
    fn value(&mut self) -> Result<Value, &'static str> {
        match self.byte()? {
            NULL => Ok(Value::Null),
            FALSE => Ok(Value::Bool(false)),
            TRUE => Ok(Value::Bool(true)),
            NUMBER => {
                let literal = self.text()?;
                Number::parse(literal)
                    .map(Value::Number)
                    .map_err(|_| "invalid replay number")
            }
            STRING => Ok(Value::string(self.text()?.to_owned())),
            ARRAY_START => {
                let count = self.length()?;
                let available_values = self
                    .bytes
                    .len()
                    .saturating_sub(self.position)
                    .saturating_sub(1);
                if count > available_values {
                    return Err("truncated structural replay array");
                }
                let mut values = Vec::with_capacity(count);
                for _ in 0..count {
                    values.push(self.value()?);
                }
                if self.byte()? != ARRAY_END {
                    return Err("missing replay array boundary");
                }
                Ok(Value::array(values))
            }
            OBJECT_START => {
                let count = self.length()?;
                let available_entries = self
                    .bytes
                    .len()
                    .saturating_sub(self.position)
                    .saturating_sub(1)
                    / 10;
                if count > available_entries {
                    return Err("truncated structural replay object");
                }
                let mut values = Object::with_capacity(count);
                for _ in 0..count {
                    if self.byte()? != KEY {
                        return Err("missing replay object key");
                    }
                    let key: Arc<str> = Arc::from(self.text()?);
                    values.insert(key, self.value()?);
                }
                if self.byte()? != OBJECT_END {
                    return Err("missing replay object boundary");
                }
                Ok(Value::object(values))
            }
            _ => Err("unknown structural replay tag"),
        }
    }

    fn byte(&mut self) -> Result<u8, &'static str> {
        let byte = *self
            .bytes
            .get(self.position)
            .ok_or("truncated structural replay record")?;
        self.position += 1;
        Ok(byte)
    }

    fn length(&mut self) -> Result<usize, &'static str> {
        let end = self
            .position
            .checked_add(8)
            .ok_or("invalid structural replay length")?;
        let bytes: [u8; 8] = self
            .bytes
            .get(self.position..end)
            .ok_or("truncated structural replay length")?
            .try_into()
            .map_err(|_| "invalid structural replay length")?;
        self.position = end;
        usize::try_from(u64::from_le_bytes(bytes)).map_err(|_| "structural replay length overflow")
    }

    fn text(&mut self) -> Result<&'a str, &'static str> {
        let length = self.length()?;
        let end = self
            .position
            .checked_add(length)
            .ok_or("invalid structural replay text length")?;
        let bytes = self
            .bytes
            .get(self.position..end)
            .ok_or("truncated structural replay text")?;
        self.position = end;
        std::str::from_utf8(bytes).map_err(|_| "invalid structural replay UTF-8")
    }
}

#[cfg(test)]
mod tests {
    use tq_core::Value;

    use super::{ARRAY_START, OBJECT_START, decode, encode};

    #[test]
    fn nested_values_round_trip_without_json_records() {
        let value: Value =
            serde_json::from_str(r#"{"z":9007199254740993,"a":[null,true,"x"],"empty":{}}"#)
                .unwrap();
        let encoded = encode(&value);
        assert_eq!(decode(&encoded).unwrap(), value);
        assert_ne!(encoded.first(), Some(&b'{'));
    }

    #[test]
    fn malformed_records_fail_deterministically() {
        assert_eq!(decode(&[255]).unwrap_err(), "unknown structural replay tag");
        assert!(decode(&[5]).is_err());
        let mut trailing = encode(&Value::Null);
        trailing.push(0);
        assert_eq!(
            decode(&trailing).unwrap_err(),
            "trailing structural replay bytes"
        );

        let mut huge_array = vec![ARRAY_START];
        huge_array.extend_from_slice(&u64::try_from(usize::MAX).unwrap().to_le_bytes());
        assert_eq!(
            decode(&huge_array).unwrap_err(),
            "truncated structural replay array"
        );

        let mut huge_object = vec![OBJECT_START];
        huge_object.extend_from_slice(&u64::try_from(usize::MAX).unwrap().to_le_bytes());
        assert_eq!(
            decode(&huge_object).unwrap_err(),
            "truncated structural replay object"
        );
    }
}
