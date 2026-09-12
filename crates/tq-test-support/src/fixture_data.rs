//! TOON storage for repository-owned fixture metadata.

use serde::{Serialize, de::DeserializeOwned};
use std::{fs, io, path::Path};

/// Decodes a TOON document into a typed metadata value.
///
/// # Errors
/// Returns malformed TOON, numeric-conversion, or schema errors.
pub fn from_toon<T: DeserializeOwned>(bytes: &[u8]) -> io::Result<T> {
    let value = tq_toon::decode_to_value(
        bytes,
        tq_core::SourceId::new(0),
        tq_toon::DecoderConfig::default(),
    )
    .map_err(io::Error::other)?;
    let mut value = value.to_json().map_err(io::Error::other)?;
    restore_literals(&mut value)?;
    serde_json::from_slice(&serde_json::to_vec(&value).map_err(io::Error::other)?)
        .map_err(io::Error::other)
}

/// Reads TOON metadata; JSON remains supported for external/temporary inputs.
///
/// # Errors
/// Returns I/O or decoding errors.
pub fn read<T: DeserializeOwned>(path: &Path) -> io::Result<T> {
    let bytes = fs::read(path)?;
    if path.extension().is_some_and(|ext| ext == "toon") {
        from_toon(&bytes)
    } else {
        serde_json::from_slice(&bytes).map_err(io::Error::other)
    }
}

/// Encodes metadata as one newline-terminated, unframed TOON document.
///
/// # Errors
/// Returns serialization or numeric-model errors.
pub fn to_toon<T: Serialize>(value: &T) -> io::Result<String> {
    let mut value = serde_json::from_slice(&serde_json::to_vec(value).map_err(io::Error::other)?)
        .map_err(io::Error::other)?;
    preserve_literals(&mut value)?;
    let value = tq_core::Value::from_json(value).map_err(io::Error::other)?;
    Ok(format!(
        "{}\n",
        tq_toon::encode(&value, tq_toon::WriterConfig::default())
    ))
}

const NUMBER_LITERAL: &str = "$tq.fixture.number";
const STRING_BYTES: &str = "$tq.fixture.utf8hex";

fn preserve_literals(value: &mut serde_json::Value) -> io::Result<()> {
    match value {
        serde_json::Value::String(text)
            if text
                .chars()
                .any(|c| c < ' ' && !matches!(c, '\n' | '\r' | '\t')) =>
        {
            *value = serde_json::json!({STRING_BYTES: crate::compatibility::encode_hex(text.as_bytes())});
        }
        serde_json::Value::Number(_) => {
            let original = value.clone();
            let round_trip = tq_core::Value::from_json(original.clone())
                .ok()
                .and_then(|v| {
                    let text = tq_toon::encode(&v, tq_toon::WriterConfig::default());
                    tq_toon::decode_to_value(
                        text.as_bytes(),
                        tq_core::SourceId::new(0),
                        tq_toon::DecoderConfig::default(),
                    )
                    .ok()?
                    .to_json()
                    .ok()
                });
            if round_trip.as_ref() != Some(&original) {
                *value = serde_json::json!({NUMBER_LITERAL: original.to_string()});
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                preserve_literals(value)?;
            }
        }
        serde_json::Value::Object(values) => {
            if values.contains_key(NUMBER_LITERAL) || values.contains_key(STRING_BYTES) {
                return Err(io::Error::other("reserved fixture-literal key in source"));
            }
            for value in values.values_mut() {
                preserve_literals(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn restore_literals(value: &mut serde_json::Value) -> io::Result<()> {
    match value {
        serde_json::Value::Object(values)
            if values.len() == 1 && values.contains_key(STRING_BYTES) =>
        {
            let hex = values[STRING_BYTES]
                .as_str()
                .ok_or_else(|| io::Error::other("fixture string bytes must be hex text"))?;
            let bytes = (0..hex.len())
                .step_by(2)
                .map(|i| {
                    hex.get(i..i + 2)
                        .ok_or_else(|| io::Error::other("odd hex length"))
                        .and_then(|s| u8::from_str_radix(s, 16).map_err(io::Error::other))
                })
                .collect::<io::Result<Vec<_>>>()?;
            *value = serde_json::Value::String(String::from_utf8(bytes).map_err(io::Error::other)?);
        }
        serde_json::Value::Object(values)
            if values.len() == 1 && values.contains_key(NUMBER_LITERAL) =>
        {
            let literal = values[NUMBER_LITERAL]
                .as_str()
                .ok_or_else(|| io::Error::other("fixture-number literal must be a string"))?;
            let number: serde_json::Number =
                serde_json::from_str(literal).map_err(io::Error::other)?;
            *value = serde_json::Value::Number(number);
        }
        serde_json::Value::Object(values) => {
            for value in values.values_mut() {
                restore_literals(value)?;
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                restore_literals(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Presents JSONL catalogs or TOON case arrays as JSON records to schema assertions.
/// This does not write JSON files or change the cases' declared input format.
///
/// # Errors
/// Returns I/O, decoding, or serialization errors.
pub fn case_lines(path: &Path) -> io::Result<String> {
    if path.extension().is_some_and(|ext| ext == "jsonl") {
        return fs::read_to_string(path);
    }
    let values: Vec<serde_json::Value> = read(path)?;
    values
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()
        .map(|lines| lines.join("\n"))
        .map_err(io::Error::other)
}

#[cfg(test)]
mod tests {
    #[test]
    fn exact_numbers_beyond_runtime_limits_round_trip_without_rounding() {
        let source: serde_json::Value =
            serde_json::from_str(&format!("[{},1.000,9007199254740993]", "1".repeat(4097)))
                .unwrap();
        let toon = super::to_toon(&source).unwrap();
        let decoded: serde_json::Value = super::from_toon(toon.as_bytes()).unwrap();
        assert_eq!(decoded, source);
        assert!(toon.contains(super::NUMBER_LITERAL));
    }

    #[test]
    fn oversized_numeric_strings_remain_strings() {
        let value = serde_json::json!(["1".repeat(4097), "1e1000001"]);
        let encoded = super::to_toon(&value).unwrap();
        let decoded: serde_json::Value = super::from_toon(encoded.as_bytes()).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn control_strings_round_trip_losslessly() {
        let value = serde_json::json!(["\u{0}\u{1e}\u{1b}", "é😀"]);
        assert_eq!(
            super::from_toon::<serde_json::Value>(super::to_toon(&value).unwrap().as_bytes())
                .unwrap(),
            value
        );
    }
}
