//! Reviewed source and executable pins for the manual campaign.

use super::{ToolIdentity, ToolKind, encode_hex};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path},
};
use thiserror::Error;

/// Versioned manual source and reference identities, independent of local paths.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ManualReferencePin {
    /// Pin schema version.
    pub schema_version: u32,
    /// Exact imported source inventory bytes.
    pub source_inventory_sha256: String,
    /// Introduction and all manual section files.
    pub sections: Vec<ManualSourcePin>,
    /// Original cases and their protected exact-match status.
    pub baseline_cases: Vec<ManualBaselineCase>,
    /// Reviewed reference builds by host architecture and OS.
    pub references: Vec<ManualBuildPin>,
}

/// One source document, relative to the companion repository root.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ManualSourcePin {
    /// Stable source section name, including introduction.
    pub section: String,
    /// Relative document path.
    pub file: String,
    /// Exact document byte digest.
    pub sha256: String,
}

/// An original executable case that may not disappear from coverage.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ManualBaselineCase {
    /// Stable case identity.
    pub id: String,
    /// Whether this case was one of the original 303 matches.
    pub exact_match: bool,
}

/// A reviewed jq binary and its observable build configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ManualBuildPin {
    /// Host identity as Rust's architecture and OS constants joined by a hyphen.
    pub target: String,
    /// Exact jq version output.
    pub version: String,
    /// Executable length.
    pub bytes: u64,
    /// Executable SHA-256.
    pub sha256: String,
    /// Exact nonempty lines from jq's build-configuration output.
    pub build_configuration: String,
    /// Hashes of dynamically linked runtime libraries, when applicable.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runtime_libraries: Vec<crate::corpus::ArtifactIdentity>,
}

/// Missing evidence or drift from a reviewed pin.
#[derive(Debug, Error)]
pub enum ManualPinError {
    /// Filesystem evidence could not be read.
    #[error("manual pin I/O failed: {0}")]
    Io(#[from] std::io::Error),
    /// A required identity or relationship differs.
    #[error("manual pin verification failed: {0}")]
    Drift(String),
}

/// Returns the platform key used by reviewed reference pins.
#[must_use]
pub fn manual_host_target() -> String {
    format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS)
}

/// Verifies imported inventory bytes and its complete source section mapping.
///
/// # Errors
/// Returns drift for changed bytes, unsupported pin schema, or changed sections.
pub fn validate_manual_source_inventory(
    pin: &ManualReferencePin,
    bytes: &[u8],
) -> Result<(), ManualPinError> {
    if pin.schema_version != 1 || encode_hex(&Sha256::digest(bytes)) != pin.source_inventory_sha256
    {
        return Err(drift("source inventory fingerprint or pin schema changed"));
    }
    let source: Value =
        crate::fixture_data::from_toon(bytes).map_err(|error| drift(error.to_string()))?;
    let sections = source["sections"]
        .as_array()
        .ok_or_else(|| drift("source sections missing"))?;
    let names = pin
        .sections
        .iter()
        .map(|section| section.section.as_str())
        .collect::<BTreeSet<_>>();
    if sections.len() < 13
        || pin.sections.len() != sections.len() + 1
        || names.len() != pin.sections.len()
    {
        return Err(drift("source section inventory changed"));
    }
    if !pin
        .sections
        .iter()
        .any(|section| section.section == "introduction" && section.file == "jq-manual/index.md")
    {
        return Err(drift("introduction source pin missing"));
    }
    for section in sections {
        if !pin.sections.iter().any(|pinned| {
            section["section"] == pinned.section
                && section["file"] == pinned.file
                && section["sha256"] == pinned.sha256
        }) {
            return Err(drift(format!(
                "source section identity changed: {}",
                section["section"]
            )));
        }
    }
    Ok(())
}

/// Verifies the companion checkout without requiring it for fixture execution.
///
/// # Errors
/// Returns missing-file or changed-content errors for any pinned source document.
pub fn validate_manual_source_checkout(
    pin: &ManualReferencePin,
    root: &Path,
) -> Result<(), ManualPinError> {
    for section in &pin.sections {
        let path = Path::new(&section.file);
        if path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(drift("source pin path must be relative and confined"));
        }
        let bytes = fs::read(root.join(path))?;
        if encode_hex(&Sha256::digest(&bytes)) != section.sha256 {
            return Err(drift(format!("source document changed: {}", section.file)));
        }
    }
    Ok(())
}

/// Verifies a reference binary regardless of its installation path.
///
/// # Errors
/// Returns drift for unverified platforms, ambiguous pins, or changed binary/build identity.
pub fn validate_manual_reference(
    pin: &ManualReferencePin,
    identity: &ToolIdentity,
    target: &str,
) -> Result<(), ManualPinError> {
    let references = pin
        .references
        .iter()
        .filter(|reference| reference.target == target)
        .collect::<Vec<_>>();
    let [reference] = references.as_slice() else {
        return Err(drift(format!(
            "reference target is unverified or ambiguous: {target}"
        )));
    };
    if identity.tool != ToolKind::Jq
        || identity.version != reference.version
        || identity.executable.bytes != reference.bytes
        || identity.executable.sha256 != reference.sha256
        || identity.build_features.join("\n") != reference.build_configuration
        || runtime_library_keys(&identity.runtime_libraries)
            != runtime_library_keys(&reference.runtime_libraries)
    {
        return Err(drift(format!(
            "jq executable or build configuration changed for {target}"
        )));
    }
    Ok(())
}

fn runtime_library_keys(
    libraries: &[crate::corpus::ArtifactIdentity],
) -> BTreeSet<(String, u64, String)> {
    libraries
        .iter()
        .map(|library| {
            let name = Path::new(&library.path)
                .file_name()
                .and_then(|name| name.to_str())
                .map_or_else(|| library.path.clone(), str::to_owned);
            (name, library.bytes, library.sha256.clone())
        })
        .collect()
}

/// Preserves every original case and prohibits regressions in original matches.
///
/// # Errors
/// Returns drift for missing original IDs, duplicated pin IDs, or any protected regression.
pub fn validate_pinned_manual_coverage(
    pin: &ManualReferencePin,
    report: &Value,
) -> Result<(), ManualPinError> {
    let ids = pin
        .baseline_cases
        .iter()
        .map(|case| case.id.as_str())
        .collect::<BTreeSet<_>>();
    if pin.baseline_cases.len() < 518
        || ids.len() != pin.baseline_cases.len()
        || pin
            .baseline_cases
            .iter()
            .filter(|case| case.exact_match)
            .count()
            < 303
    {
        return Err(drift("original case inventory is incomplete"));
    }
    let rows = report["cases"]
        .as_array()
        .ok_or_else(|| drift("report cases missing"))?;
    for original in &pin.baseline_cases {
        let row = rows
            .iter()
            .find(|row| row["id"] == original.id)
            .ok_or_else(|| drift(format!("original case missing: {}", original.id)))?;
        if original.exact_match && row["verdict"] != "match" {
            return Err(drift(format!(
                "original exact match regressed: {}",
                original.id
            )));
        }
    }
    Ok(())
}

fn drift(message: impl Into<String>) -> ManualPinError {
    ManualPinError::Drift(message.into())
}
