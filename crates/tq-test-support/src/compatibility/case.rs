//! Typed compatibility-case catalog loading.

use std::{collections::BTreeMap, fs, io, path::Path};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::corpus::ArtifactIdentity;

/// One versioned compatibility case.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompatibilityCase {
    /// Schema version.
    pub schema_version: u32,
    /// Stable case identifier.
    pub id: String,
    /// Human title.
    pub title: String,
    /// Compatibility classification.
    pub classification: CaseClassification,
    /// Capability coverage tags.
    pub capabilities: Vec<String>,
    /// MVP or deferred status.
    pub status: CaseStatus,
    /// Input fixture.
    pub fixture: CaseFixture,
    /// Default jq-like query.
    pub query: String,
    /// Per-tool invocation adapters.
    pub adapters: ToolAdapters,
    /// Input transport.
    pub invocation_mode: InvocationMode,
    /// Result contract.
    pub expected: ExpectedContract,
}

/// Catalog classification.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CaseClassification {
    /// Shared jq/yq/tq behavior.
    Common,
    /// jq compatibility target.
    JqTarget,
    /// Command-line behavior.
    Cli,
    /// Explicitly out of scope.
    Deferred,
}

/// Implementation status.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CaseStatus {
    /// Required by the MVP.
    Mvp,
    /// Intentionally deferred.
    Deferred,
}

/// Inline or file-backed test input.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CaseFixture {
    /// Declared fixture format.
    pub format: FixtureFormat,
    /// Inline bytes, when present.
    pub inline: Option<String>,
    /// Repository-relative path, when present.
    pub path: Option<String>,
}

/// Fixture encoding.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FixtureFormat {
    /// JSON.
    Json,
    /// YAML.
    Yaml,
    /// TOON.
    Toon,
    /// Unstructured text.
    Raw,
    /// No fixture.
    None,
}

/// Tool-specific query and argument overrides.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CaseAdapter {
    /// Tool-specific query.
    pub query: Option<String>,
    /// Tool-specific CLI arguments before the query.
    #[serde(default)]
    pub args: Vec<String>,
    /// Arguments after the query, such as values for `--args`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trailing_args: Vec<String>,
    /// Commands such as `--help` and `--from-file` need no query argument.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub omit_query: bool,
    /// Per-process environment overrides, without mutating the test environment.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// Whether the case applies to this tool.
    #[serde(default)]
    pub supported: bool,
    /// Applicability explanation.
    pub note: Option<String>,
}

/// Adapters for all three tools.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolAdapters {
    /// jq adapter.
    #[serde(default)]
    pub jq: CaseAdapter,
    /// yq adapter.
    #[serde(default)]
    pub yq: CaseAdapter,
    /// tq adapter.
    #[serde(default)]
    pub tq: CaseAdapter,
}

/// Input transport mode.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InvocationMode {
    /// Fixture is piped to stdin.
    Stdin,
    /// Fixture is materialized as a file argument.
    File,
    /// No input is supplied.
    NullInput,
}

/// Expected output shape and baseline applicability.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExpectedContract {
    /// Output or failure shape.
    pub contract: ContractKind,
    /// Reference-baseline policy.
    pub baseline: BaselinePolicy,
    /// Expected stable error class.
    pub error_class: Option<String>,
    /// Compare stderr bytes for observable I/O such as `debug` and `stderr`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub compare_stderr: bool,
}

/// Output contract.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContractKind {
    /// Ordered structured values.
    ResultSequence,
    /// Exact stdout bytes.
    RawBytes,
    /// Classified failure.
    Error,
    /// Exit code observation.
    ExitStatus,
}

/// Whether a baseline is required.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BaselinePolicy {
    /// Must have a reviewed baseline.
    Required,
    /// Recorded for comparison but not gating.
    Informative,
    /// No reference tool can establish it.
    NotApplicable,
}

/// Loaded catalog and its stable byte identity.
#[derive(Clone, Debug)]
pub struct CompatibilityCatalog {
    /// Sorted cases.
    pub cases: Vec<CompatibilityCase>,
    /// Identity of the concatenated sorted catalog files.
    pub identity: ArtifactIdentity,
}

/// Catalog loading errors with source context.
#[derive(Debug, Error)]
pub enum CatalogError {
    /// Filesystem failure.
    #[error("catalog I/O failed: {0}")]
    Io(#[from] io::Error),
    /// Invalid JSONL record.
    #[error("invalid compatibility case at {path}:{line}: {source}")]
    Json {
        /// File path.
        path: String,
        /// One-based line number.
        line: usize,
        /// JSON error.
        source: serde_json::Error,
    },
    /// Duplicate stable identifier.
    #[error("duplicate compatibility case ID: {0}")]
    DuplicateId(String),
}

/// Loads TOON case arrays and legacy JSONL files, rejecting duplicate IDs.
///
/// # Errors
///
/// Returns source-positioned I/O, JSON, or duplicate-ID errors.
pub fn load_catalog(directory: &Path) -> Result<CompatibilityCatalog, CatalogError> {
    let mut paths = fs::read_dir(directory)?
        .map(|entry| entry.map(|value| value.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| {
        path.extension()
            .is_some_and(|ext| ext == "toon" || ext == "jsonl")
    });
    paths.sort();

    let mut cases = Vec::new();
    let mut ids = std::collections::BTreeSet::new();
    let mut digest = Sha256::new();
    let mut bytes = 0_u64;
    for path in paths {
        let contents = fs::read(&path)?;
        bytes = bytes.saturating_add(u64::try_from(contents.len()).unwrap_or(u64::MAX));
        digest.update(path.file_name().unwrap_or_default().as_encoded_bytes());
        digest.update([0]);
        digest.update(&contents);
        let records = if path.extension().is_some_and(|ext| ext == "toon") {
            crate::fixture_data::case_lines(&path)?.into_bytes()
        } else {
            contents
        };
        for (index, line) in records.split(|byte| *byte == b'\n').enumerate() {
            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            let case: CompatibilityCase =
                serde_json::from_slice(line).map_err(|source| CatalogError::Json {
                    path: path.display().to_string(),
                    line: index + 1,
                    source,
                })?;
            if !ids.insert(case.id.clone()) {
                return Err(CatalogError::DuplicateId(case.id));
            }
            cases.push(case);
        }
    }
    cases.sort_by(|left, right| left.id.cmp(&right.id));
    let sha256 = digest
        .finalize()
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            use std::fmt::Write as _;
            write!(hex, "{byte:02x}").expect("write digest to string");
            hex
        });
    Ok(CompatibilityCatalog {
        cases,
        identity: ArtifactIdentity {
            path: directory.display().to_string(),
            bytes,
            sha256,
        },
    })
}
