//! Untimed corpus materialization and ordered semantic comparison.

use std::{
    fs,
    io::{self, Write},
    path::Path,
};

use serde_json::Value;
use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;
use thiserror::Error;

use super::{ArtifactIdentity, GeneratedArtifacts, encode_hex};

/// Kind of ordered JSON-model divergence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DifferenceKind {
    /// JSON-model types differ.
    Type,
    /// Primitive values differ.
    Value,
    /// Numeric literals no longer retain the same exact value.
    NumericFidelity,
    /// Array lengths differ.
    Length,
    /// Object member encounter order or keys differ.
    ObjectOrder,
}

/// First ordered semantic difference between two JSON-model values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticDifference {
    /// RFC 6901-style path to the difference, with empty string for root.
    pub path: String,
    /// Difference classification.
    pub kind: DifferenceKind,
    /// Compact expected observation.
    pub expected: String,
    /// Compact actual observation.
    pub actual: String,
}

/// Stable cross-format generation and validation failures.
#[derive(Debug, Error)]
pub enum ConversionError {
    /// Source or destination I/O failed.
    #[error("cross-format I/O failed: {0}")]
    Io(#[from] io::Error),
    /// Source JSON could not be decoded without numeric loss.
    #[error("source JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    /// YAML generation or decoding failed.
    #[error("YAML failed: {0}")]
    Yaml(String),
    /// TOON generation or decoding failed.
    #[error("TOON failed: {0}")]
    Toon(String),
    /// A generated representation changed ordered JSON semantics.
    #[error("{format} semantic difference at {difference:?}")]
    Semantic {
        /// Generated format name.
        format: String,
        /// First exact difference.
        difference: SemanticDifference,
    },
    /// A verified temporary output could not be installed.
    #[error("could not atomically install generated artifact: {0}")]
    Persist(#[from] tempfile::PersistError),
}

/// Generates YAML and TOON representations from natural source JSON.
///
/// Both representations are fully prepared before either atomic output write;
/// callers invoke this outside benchmark timing.
///
/// # Errors
///
/// Returns a JSON, YAML, TOON, or filesystem error.
pub fn generate_representations(
    source_json: &Path,
    yaml_output: &Path,
    toon_output: &Path,
    yaml_manifest_path: &str,
    toon_manifest_path: &str,
) -> Result<GeneratedArtifacts, ConversionError> {
    let source: Value = serde_json::from_reader(fs::File::open(source_json)?)?;
    // Serializing `serde_json::Value` directly is not interoperable when
    // serde_json's `arbitrary_precision` feature is enabled: its private
    // number marker is serialized as a YAML mapping. Convert into the YAML
    // library's own value model first so external readers observe scalars.
    let yaml = match json_to_yaml(&source) {
        Ok(yaml_value) => yaml_serde::to_string(&yaml_value)
            .map(String::into_bytes)
            .map_err(|error| ConversionError::Yaml(error.to_string()))?,
        Err(ConversionError::Yaml(_)) => {
            // JSON is a YAML 1.2 subset. Retaining the original JSON text is
            // the lossless YAML profile when yaml_serde's public number model
            // cannot represent an arbitrary decimal scalar. The YAML parser
            // acceptance and exact JSON-model validation happen below.
            fs::read(source_json)?
        }
        Err(error) => return Err(error),
    };
    let toon = encode_toon_exact(&source)?;

    let yaml = write_generated(yaml_output, yaml_manifest_path, &yaml)?;
    let toon = write_generated(toon_output, toon_manifest_path, toon.as_bytes())?;
    Ok(GeneratedArtifacts { yaml, toon })
}

/// Validates generated YAML and TOON against ordered source JSON semantics.
///
/// # Errors
///
/// Returns a parse error or the first type, value, number, array, or object-order
/// divergence. It never sorts objects or arrays before comparison.
pub fn validate_generated_representations(
    source_json: &Path,
    yaml_input: &Path,
    toon_input: &Path,
) -> Result<(), ConversionError> {
    let source: Value = serde_json::from_reader(fs::File::open(source_json)?)?;
    let yaml_model: yaml_serde::Value = yaml_serde::from_reader(fs::File::open(yaml_input)?)
        .map_err(|error| ConversionError::Yaml(error.to_string()))?;
    // A JSON-subset YAML artifact is the exact-decimal fallback. Decode that
    // with the arbitrary-precision ordered JSON path after proving the YAML
    // parser accepts it; ordinary block YAML uses yaml_serde's value model.
    let yaml: Value = match serde_json::from_reader(fs::File::open(yaml_input)?) {
        Ok(value) => value,
        Err(_) => yaml_to_json(yaml_model)?,
    };
    compare_ordered(&source, &yaml).map_err(|difference| ConversionError::Semantic {
        format: "yaml".to_owned(),
        difference,
    })?;

    let toon_text = fs::read_to_string(toon_input)?;
    let expected_toon = encode_toon_exact(&source)?;
    if toon_text != expected_toon {
        return Err(ConversionError::Toon(
            "generated TOON differs from its exact-decimal structural encoding".to_owned(),
        ));
    }
    let toon: Value = toon_format::decode_strict(&toon_text)
        .map_err(|error| ConversionError::Toon(error.to_string()))?;
    let toon_model = binary64_number_model(&source)?;
    compare_ordered(&toon_model, &toon).map_err(|difference| ConversionError::Semantic {
        format: "toon".to_owned(),
        difference,
    })
}

const NUMBER_MARKER_PREFIX: &str = "tqnumf4c6a91b7e2d";
const NUMBER_MARKER_SUFFIX: char = 'z';

fn encode_toon_exact(source: &Value) -> Result<String, ConversionError> {
    let mut numbers = Vec::new();
    let marked = mark_numbers(source, &mut numbers)?;
    let template = toon_format::encode(&marked, &toon_format::EncodeOptions::default())
        .map_err(|error| ConversionError::Toon(error.to_string()))?;
    replace_number_markers(&template, &numbers)
}

fn mark_numbers(value: &Value, numbers: &mut Vec<String>) -> Result<Value, ConversionError> {
    Ok(match value {
        Value::Number(number) => {
            let index = numbers.len();
            numbers.push(number.to_string());
            Value::String(format!(
                "{NUMBER_MARKER_PREFIX}{index}{NUMBER_MARKER_SUFFIX}"
            ))
        }
        Value::Array(values) => Value::Array(
            values
                .iter()
                .map(|value| mark_numbers(value, numbers))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        Value::Object(values) => {
            let mut object = serde_json::Map::with_capacity(values.len());
            for (key, value) in values {
                if key.contains(NUMBER_MARKER_PREFIX) {
                    return Err(ConversionError::Toon(
                        "source key collides with exact-number marker namespace".to_owned(),
                    ));
                }
                object.insert(key.clone(), mark_numbers(value, numbers)?);
            }
            Value::Object(object)
        }
        Value::String(value) => {
            if value.contains(NUMBER_MARKER_PREFIX) {
                return Err(ConversionError::Toon(
                    "source string collides with exact-number marker namespace".to_owned(),
                ));
            }
            value.clone().into()
        }
        Value::Null => Value::Null,
        Value::Bool(value) => Value::Bool(*value),
    })
}

fn replace_number_markers(template: &str, numbers: &[String]) -> Result<String, ConversionError> {
    let mut output = String::with_capacity(template.len());
    let mut remaining = template;
    while let Some(start) = remaining.find(NUMBER_MARKER_PREFIX) {
        output.push_str(&remaining[..start]);
        let marker = &remaining[start + NUMBER_MARKER_PREFIX.len()..];
        let Some(end) = marker.find(NUMBER_MARKER_SUFFIX) else {
            return Err(ConversionError::Toon(
                "unterminated exact-number marker in TOON encoding".to_owned(),
            ));
        };
        let index = marker[..end]
            .parse::<usize>()
            .map_err(|error| ConversionError::Toon(error.to_string()))?;
        output.push_str(numbers.get(index).ok_or_else(|| {
            ConversionError::Toon("unknown exact-number marker in TOON encoding".to_owned())
        })?);
        remaining = &marker[end + NUMBER_MARKER_SUFFIX.len_utf8()..];
    }
    output.push_str(remaining);
    Ok(output)
}

fn binary64_number_model(value: &Value) -> Result<Value, ConversionError> {
    Ok(match value {
        Value::Number(number) => {
            if let Some(integer) = number.as_i64() {
                Value::Number(integer.into())
            } else if let Some(integer) = number.as_u64() {
                Value::Number(integer.into())
            } else {
                let float = number.as_f64().ok_or_else(|| {
                    ConversionError::Toon(format!("number is outside binary64: {number}"))
                })?;
                if float.is_finite() && float.fract() == 0.0 {
                    if let Ok(integer) = format!("{float:.0}").parse::<i64>() {
                        Value::Number(integer.into())
                    } else {
                        Value::Number(serde_json::Number::from_f64(float).ok_or_else(|| {
                            ConversionError::Toon(format!("number is non-finite: {number}"))
                        })?)
                    }
                } else {
                    Value::Number(serde_json::Number::from_f64(float).ok_or_else(|| {
                        ConversionError::Toon(format!("number is non-finite: {number}"))
                    })?)
                }
            }
        }
        Value::Array(values) => Value::Array(
            values
                .iter()
                .map(binary64_number_model)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        Value::Object(values) => {
            let mut object = serde_json::Map::with_capacity(values.len());
            for (key, value) in values {
                object.insert(key.clone(), binary64_number_model(value)?);
            }
            Value::Object(object)
        }
        Value::String(value) => Value::String(value.clone()),
        Value::Null => Value::Null,
        Value::Bool(value) => Value::Bool(*value),
    })
}

fn json_to_yaml(value: &Value) -> Result<yaml_serde::Value, ConversionError> {
    use yaml_serde::Value as Yaml;

    Ok(match value {
        Value::Null => Yaml::Null,
        Value::Bool(value) => Yaml::Bool(*value),
        Value::String(value) => Yaml::String(value.clone()),
        Value::Number(value) => {
            let number = if let Some(value) = value.as_i64() {
                yaml_serde::Number::from(value)
            } else if let Some(value) = value.as_u64() {
                yaml_serde::Number::from(value)
            } else {
                let float = value.as_f64().ok_or_else(|| {
                    ConversionError::Yaml(format!(
                        "number is outside yaml_serde's lossless envelope: {value}"
                    ))
                })?;
                let round_trip = serde_json::Number::from_f64(float).ok_or_else(|| {
                    ConversionError::Yaml(format!("non-finite JSON number: {value}"))
                })?;
                if round_trip.to_string() != value.to_string() {
                    return Err(ConversionError::Yaml(format!(
                        "number would lose precision in YAML: {value}"
                    )));
                }
                yaml_serde::Number::from(float)
            };
            Yaml::Number(number)
        }
        Value::Array(values) => Yaml::Sequence(
            values
                .iter()
                .map(json_to_yaml)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        Value::Object(values) => {
            let mut mapping = yaml_serde::Mapping::new();
            for (key, value) in values {
                mapping.insert(Yaml::String(key.clone()), json_to_yaml(value)?);
            }
            Yaml::Mapping(mapping)
        }
    })
}

fn yaml_to_json(value: yaml_serde::Value) -> Result<Value, ConversionError> {
    use yaml_serde::Value as Yaml;

    Ok(match value {
        Yaml::Null => Value::Null,
        Yaml::Bool(value) => Value::Bool(value),
        Yaml::String(value) => Value::String(value),
        Yaml::Number(value) => {
            let number = if let Some(value) = value.as_i64() {
                serde_json::Number::from(value)
            } else if let Some(value) = value.as_u64() {
                serde_json::Number::from(value)
            } else {
                serde_json::Number::from_f64(value.as_f64().ok_or_else(|| {
                    ConversionError::Yaml("YAML number has no finite representation".to_owned())
                })?)
                .ok_or_else(|| ConversionError::Yaml("non-finite YAML number".to_owned()))?
            };
            Value::Number(number)
        }
        Yaml::Sequence(values) => Value::Array(
            values
                .into_iter()
                .map(yaml_to_json)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        Yaml::Mapping(values) => {
            let mut object = serde_json::Map::with_capacity(values.len());
            for (key, value) in values {
                let Yaml::String(key) = key else {
                    return Err(ConversionError::Yaml(
                        "generated YAML contains a non-string mapping key".to_owned(),
                    ));
                };
                object.insert(key, yaml_to_json(value)?);
            }
            Value::Object(object)
        }
        Yaml::Tagged(_) => {
            return Err(ConversionError::Yaml(
                "generated YAML contains a tagged value".to_owned(),
            ));
        }
    })
}

/// Compares two values without sorting objects, arrays, or result sequences.
///
/// # Errors
///
/// Returns the first ordered semantic difference with its exact path.
pub fn compare_ordered(expected: &Value, actual: &Value) -> Result<(), SemanticDifference> {
    compare_at(expected, actual, "")
}

fn compare_at(expected: &Value, actual: &Value, path: &str) -> Result<(), SemanticDifference> {
    match (expected, actual) {
        (Value::Null, Value::Null) => Ok(()),
        (Value::Bool(left), Value::Bool(right)) => primitive(left, right, path),
        (Value::String(left), Value::String(right)) => primitive(left, right, path),
        (Value::Number(left), Value::Number(right)) => {
            let left = left.to_string();
            let right = right.to_string();
            if left == right {
                Ok(())
            } else {
                Err(SemanticDifference {
                    path: path.to_owned(),
                    kind: DifferenceKind::NumericFidelity,
                    expected: left,
                    actual: right,
                })
            }
        }
        (Value::Array(left), Value::Array(right)) => {
            if left.len() != right.len() {
                return Err(SemanticDifference {
                    path: path.to_owned(),
                    kind: DifferenceKind::Length,
                    expected: left.len().to_string(),
                    actual: right.len().to_string(),
                });
            }
            for (index, (left, right)) in left.iter().zip(right).enumerate() {
                compare_at(left, right, &join(path, &index.to_string()))?;
            }
            Ok(())
        }
        (Value::Object(left), Value::Object(right)) => {
            let left_keys = left.keys().collect::<Vec<_>>();
            let right_keys = right.keys().collect::<Vec<_>>();
            if left_keys != right_keys {
                return Err(SemanticDifference {
                    path: path.to_owned(),
                    kind: DifferenceKind::ObjectOrder,
                    expected: left_keys
                        .iter()
                        .map(|key| key.as_str())
                        .collect::<Vec<_>>()
                        .join(","),
                    actual: right_keys
                        .iter()
                        .map(|key| key.as_str())
                        .collect::<Vec<_>>()
                        .join(","),
                });
            }
            for key in left_keys {
                compare_at(&left[key], &right[key], &join(path, &escape(key)))?;
            }
            Ok(())
        }
        _ => Err(SemanticDifference {
            path: path.to_owned(),
            kind: DifferenceKind::Type,
            expected: type_name(expected).to_owned(),
            actual: type_name(actual).to_owned(),
        }),
    }
}

fn primitive<T: PartialEq + std::fmt::Debug>(
    expected: &T,
    actual: &T,
    path: &str,
) -> Result<(), SemanticDifference> {
    if expected == actual {
        Ok(())
    } else {
        Err(SemanticDifference {
            path: path.to_owned(),
            kind: DifferenceKind::Value,
            expected: format!("{expected:?}"),
            actual: format!("{actual:?}"),
        })
    }
}

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn join(path: &str, component: &str) -> String {
    format!("{path}/{component}")
}

fn escape(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

fn write_generated(
    destination: &Path,
    manifest_path: &str,
    bytes: &[u8],
) -> Result<ArtifactIdentity, ConversionError> {
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let mut temporary = NamedTempFile::new_in(parent)?;
    temporary.write_all(bytes)?;
    temporary.flush()?;
    temporary.as_file().sync_all()?;
    temporary.persist(destination)?;
    Ok(ArtifactIdentity {
        path: manifest_path.to_owned(),
        bytes: u64::try_from(bytes.len())
            .map_err(|_| io::Error::other("generated length does not fit in u64"))?,
        sha256: encode_hex(&Sha256::digest(bytes)),
    })
}
