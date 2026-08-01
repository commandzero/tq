//! Streaming validation of the corpus `GeoJSON` profile.

use std::{fmt, io::Read};

use serde::{
    Deserialize, Deserializer,
    de::{Error as _, IgnoredAny, SeqAccess, Visitor},
};
use thiserror::Error;

/// Structural identity calculated for a `GeoJSON` corpus document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeoJsonMetadata {
    /// Exact root `type` value.
    pub root_type: String,
    /// Number of entries in the root `features` array.
    pub logical_records: u64,
}

/// Stable failure classes for corpus `GeoJSON` validation.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum GeoJsonError {
    /// The byte stream is not one complete JSON document.
    #[error("invalid JSON: {0}")]
    InvalidJson(String),
    /// JSON input or the underlying reader failed.
    #[error("could not read JSON: {0}")]
    Io(String),
    /// JSON is valid but does not have the required `GeoJSON` fields and types.
    #[error("unexpected GeoJSON shape: {0}")]
    UnexpectedShape(String),
    /// The root is not a feature collection.
    #[error("expected GeoJSON FeatureCollection, got {0}")]
    WrongRootType(String),
}

#[derive(Deserialize)]
struct GeoJsonRoot {
    #[serde(rename = "type")]
    root_type: String,
    #[serde(deserialize_with = "count_features")]
    features: u64,
}

/// Validates one `GeoJSON` feature collection and counts its features incrementally.
///
/// Feature values are consumed as ignored Serde values, so memory usage follows
/// nesting/token size rather than the total feature-array size.
///
/// # Errors
///
/// Returns a syntax, I/O, shape, or root-type error for invalid input.
pub fn validate_geojson(reader: impl Read) -> Result<GeoJsonMetadata, GeoJsonError> {
    let mut deserializer = serde_json::Deserializer::from_reader(reader);
    let root = GeoJsonRoot::deserialize(&mut deserializer).map_err(|error| classify(&error))?;
    deserializer
        .end()
        .map_err(|error| GeoJsonError::InvalidJson(error.to_string()))?;

    if root.root_type != "FeatureCollection" {
        return Err(GeoJsonError::WrongRootType(root.root_type));
    }

    Ok(GeoJsonMetadata {
        root_type: root.root_type,
        logical_records: root.features,
    })
}

fn classify(error: &serde_json::Error) -> GeoJsonError {
    match error.classify() {
        serde_json::error::Category::Io => GeoJsonError::Io(error.to_string()),
        serde_json::error::Category::Syntax | serde_json::error::Category::Eof => {
            GeoJsonError::InvalidJson(error.to_string())
        }
        serde_json::error::Category::Data => GeoJsonError::UnexpectedShape(error.to_string()),
    }
}

fn count_features<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_seq(FeatureCountVisitor)
}

struct FeatureCountVisitor;

impl<'de> Visitor<'de> for FeatureCountVisitor {
    type Value = u64;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a GeoJSON features array")
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut count = 0_u64;
        while sequence.next_element::<IgnoredAny>()?.is_some() {
            count = count
                .checked_add(1)
                .ok_or_else(|| A::Error::custom("feature count exceeds u64"))?;
        }
        Ok(count)
    }
}
