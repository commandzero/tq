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
    let yaml =
        yaml_serde::to_string(&source).map_err(|error| ConversionError::Yaml(error.to_string()))?;
    let toon = toon_format::encode(&source, &toon_format::EncodeOptions::default())
        .map_err(|error| ConversionError::Toon(error.to_string()))?;

    let yaml = write_generated(yaml_output, yaml_manifest_path, yaml.as_bytes())?;
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
    let yaml: Value = yaml_serde::from_reader(fs::File::open(yaml_input)?)
        .map_err(|error| ConversionError::Yaml(error.to_string()))?;
    compare_ordered(&source, &yaml).map_err(|difference| ConversionError::Semantic {
        format: "yaml".to_owned(),
        difference,
    })?;

    let toon_text = fs::read_to_string(toon_input)?;
    let toon: Value = toon_format::decode_strict(&toon_text)
        .map_err(|error| ConversionError::Toon(error.to_string()))?;
    compare_ordered(&source, &toon).map_err(|difference| ConversionError::Semantic {
        format: "toon".to_owned(),
        difference,
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
