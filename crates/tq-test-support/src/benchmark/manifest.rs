//! Typed benchmark workload manifests.

use std::{fs, io, path::Path};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// One benchmark workload.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BenchmarkCase {
    /// Schema version.
    pub schema_version: u32,
    /// Stable workload ID.
    pub id: String,
    /// Compatibility case that must pass first.
    pub compatibility_gate: String,
    /// Natural dataset selection.
    pub dataset_selector: DatasetSelector,
    /// Logical jq-like query.
    pub query: String,
    /// Streaming/blocking execution class.
    pub execution_class: ExecutionClass,
    /// Whether first-result latency is required.
    pub measure_first_result: bool,
    /// Warmup and tier sample counts.
    pub sampling: BenchmarkSampling,
    /// Per-invocation timeout.
    pub timeout_seconds: u64,
    /// Output and memory limits.
    pub limits: BenchmarkLimits,
    /// Correctness contract.
    pub output_contract: OutputContract,
    /// Explicit tool/format matrix.
    pub adapters: Vec<BenchmarkAdapter>,
}

/// Dataset family and natural tier selection.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DatasetSelector {
    /// Source family.
    pub family: DatasetFamily,
    /// Natural tiers to run.
    pub tiers: Vec<DatasetTier>,
}

/// Dataset family.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DatasetFamily {
    /// Every configured natural dataset.
    Natural,
    /// USGS earthquake feeds.
    Usgs,
    /// Approximately-GiB natural dataset.
    LargeNatural,
    /// Deterministic startup/helper fixture.
    SyntheticHelper,
    /// Reviewed deterministic multi-document sequence for `inputs`.
    Issue5InputSequence,
}

/// Natural, non-resized dataset category.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DatasetTier {
    /// Natural small feed.
    Small,
    /// Natural medium feed.
    Medium,
    /// Natural large source.
    Large,
    /// Trivial startup fixture.
    Startup,
}

/// Workload materialization behavior.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionClass {
    /// Process startup and compilation.
    Startup,
    /// Incremental processing is possible.
    Streaming,
    /// One document is materialized.
    Document,
    /// Whole collection/input is required.
    Blocking,
}

/// Default sample policy.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BenchmarkSampling {
    /// Untimed warmups.
    pub warmups: usize,
    /// Small/startup measured samples.
    pub small: usize,
    /// Medium measured samples.
    pub medium: usize,
    /// Large measured samples.
    pub large: usize,
}

impl BenchmarkSampling {
    /// Sample count for a dataset tier.
    #[must_use]
    pub const fn measured(&self, tier: DatasetTier) -> usize {
        match tier {
            DatasetTier::Small | DatasetTier::Startup => self.small,
            DatasetTier::Medium => self.medium,
            DatasetTier::Large => self.large,
        }
    }
}

/// Per-process resource limits.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BenchmarkLimits {
    /// Maximum stdout bytes written before the harness stops the process.
    pub output_bytes: u64,
    /// Peak RSS objective, when applicable.
    pub rss_bytes: Option<u64>,
}

/// Correctness gate shape.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OutputContract {
    /// Comparison mode.
    pub kind: OutputContractKind,
    /// Adapter that establishes the result.
    pub reference_adapter: String,
}

/// Correctness comparison mode.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputContractKind {
    /// Compare ordered structured result values.
    SemanticSequence,
    /// Compare exact output bytes.
    RawBytes,
    /// Compare exit behavior only.
    ExitOnly,
}

/// One direct tool/format command adapter.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BenchmarkAdapter {
    /// Stable row adapter ID.
    pub id: String,
    /// Executable role.
    pub tool: BenchmarkTool,
    /// Exact on-disk representation.
    pub input_format: InputFormat,
    /// Whether the parser/execution combination applies.
    pub applicable: bool,
    /// Tool arguments before the query.
    pub args: Vec<String>,
    /// Tool-specific expression when its language differs from jq syntax.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// Report comparison group membership.
    pub comparison_families: Vec<ComparisonFamily>,
}

/// Benchmark tool.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BenchmarkTool {
    /// jq.
    Jq,
    /// yq.
    Yq,
    /// tq.
    Tq,
}

/// Input representation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum InputFormat {
    /// JSON.
    Json,
    /// YAML.
    Yaml,
    /// TOON.
    Toon,
}

/// Report comparison family.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ComparisonFamily {
    /// Identical format across tools.
    SameFormat,
    /// Each tool's native format.
    NativeFormat,
    /// tq parser-to-parser view.
    ParserSpecific,
}

/// Loaded sorted benchmark manifest.
#[derive(Clone, Debug)]
pub struct BenchmarkCatalog {
    /// Workloads sorted by ID.
    pub cases: Vec<BenchmarkCase>,
}

/// Benchmark manifest loading failures.
#[derive(Debug, Error)]
pub enum BenchmarkCatalogError {
    /// Filesystem failure.
    #[error("benchmark catalog I/O failed: {0}")]
    Io(#[from] io::Error),
    /// Invalid JSONL line.
    #[error("invalid benchmark case at {path}:{line}: {source}")]
    Json {
        /// Source path.
        path: String,
        /// One-based line.
        line: usize,
        /// JSON failure.
        source: serde_json::Error,
    },
    /// Duplicate ID.
    #[error("duplicate benchmark case ID: {0}")]
    DuplicateId(String),
}

/// Loads sorted JSONL benchmark cases.
///
/// # Errors
///
/// Returns source-positioned I/O, JSON, and duplicate-ID errors.
pub fn load_benchmark_catalog(directory: &Path) -> Result<BenchmarkCatalog, BenchmarkCatalogError> {
    let mut paths = fs::read_dir(directory)?
        .map(|entry| entry.map(|value| value.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| {
        path.extension()
            .is_some_and(|extension| extension == "jsonl")
    });
    paths.sort();
    let mut ids = std::collections::BTreeSet::new();
    let mut cases = Vec::new();
    for path in paths {
        for (index, line) in fs::read(&path)?.split(|byte| *byte == b'\n').enumerate() {
            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            let case: BenchmarkCase =
                serde_json::from_slice(line).map_err(|source| BenchmarkCatalogError::Json {
                    path: path.display().to_string(),
                    line: index + 1,
                    source,
                })?;
            if !ids.insert(case.id.clone()) {
                return Err(BenchmarkCatalogError::DuplicateId(case.id));
            }
            cases.push(case);
        }
    }
    cases.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(BenchmarkCatalog { cases })
}
