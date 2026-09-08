//! Manual-audit storage with TOON preferred over legacy JSON ledgers.

use std::{fs, io, path::Path};

use serde_json::Value;

/// Reads a section ledger, preferring its canonical TOON sibling when present.
/// External JSON-only ledgers remain readable; repository ledgers use TOON.
///
/// # Errors
///
/// Returns file, decoding, or value-conversion errors. Invalid TOON never falls
/// back silently to the comparison JSON copy.
pub fn read_manual_ledger(path: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let toon = path.with_extension("toon");
    match fs::read(&toon) {
        Ok(bytes) => Ok(crate::fixture_data::from_toon(&bytes)?),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Ok(serde_json::from_slice(&fs::read(path)?)?)
        }
        Err(error) => Err(error.into()),
    }
}

/// Collects an entry's scalar case reference.
/// Examples have one `case_id`; coverage notes have optional evidence.
///
/// # Errors
///
/// Returns an error for absent or malformed reference fields.
pub fn manual_case_ids(entry: &Value) -> Result<Vec<&str>, &'static str> {
    if let Some(id) = entry.get("case_id") {
        return Ok(vec![id.as_str().ok_or("case_id must be a string")?]);
    }
    if let Some(id) = entry.get("evidence_case_id") {
        return if id.is_null() {
            Ok(Vec::new())
        } else {
            Ok(vec![
                id.as_str()
                    .ok_or("evidence_case_id must be a string or null")?,
            ])
        };
    }
    Err("expected scalar case_id or nullable evidence_case_id")
}
