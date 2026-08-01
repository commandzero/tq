//! Explicitly reviewed compatibility baseline updates.

use std::{collections::BTreeMap, fs, io, io::Write, path::Path};

use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use thiserror::Error;

use super::{
    CompatibilityReport, ObservationState, ProcessStatus, ToolIdentity, ToolKind, ToolObservation,
};
use crate::corpus::ArtifactIdentity;

/// Baseline schema version.
pub const BASELINE_SCHEMA_VERSION: u32 = 1;

/// Reviewed, timing-independent reference observations.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompatibilityBaseline {
    /// Baseline schema version.
    pub schema_version: u32,
    /// Exact compatibility catalog identity.
    pub corpus: ArtifactIdentity,
    /// Exact reference-tool identities.
    pub tools: Vec<ToolIdentity>,
    /// Case ID, then tool name, to observation.
    pub observations: BTreeMap<String, BTreeMap<String, BaselineObservation>>,
}

/// Stable observation fields; wall-clock duration is deliberately excluded.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct BaselineObservation {
    /// Execution disposition.
    pub state: ObservationState,
    /// Ordered structured values.
    pub results: Vec<serde_json::Value>,
    /// Exact raw process stdout in hexadecimal.
    pub stdout_hex: Option<String>,
    /// Exact raw stdout in hexadecimal.
    pub raw_stdout_hex: Option<String>,
    /// Exact stderr in hexadecimal.
    pub stderr_hex: Option<String>,
    /// Process completion class.
    pub process_status: Option<ProcessStatus>,
    /// Exit code.
    pub exit_code: Option<i32>,
    /// Stable error class.
    pub error_class: Option<super::ErrorClass>,
    /// Applicability or harness note.
    pub note: Option<String>,
}

impl From<&ToolObservation> for BaselineObservation {
    fn from(value: &ToolObservation) -> Self {
        Self {
            state: value.state,
            results: value.results.clone(),
            stdout_hex: value.stdout_hex.clone(),
            raw_stdout_hex: value.raw_stdout_hex.clone(),
            stderr_hex: value.stderr_hex.clone(),
            process_status: value.process_status,
            exit_code: value.exit_code,
            error_class: value.error_class,
            note: value.note.clone(),
        }
    }
}

impl From<&CompatibilityReport> for CompatibilityBaseline {
    fn from(report: &CompatibilityReport) -> Self {
        let observations = report
            .cases
            .iter()
            .map(|case| {
                let tools = case
                    .observations
                    .iter()
                    .map(|observation| {
                        (
                            tool_name(observation.tool).to_owned(),
                            BaselineObservation::from(observation),
                        )
                    })
                    .collect();
                (case.id.clone(), tools)
            })
            .collect();
        Self {
            schema_version: BASELINE_SCHEMA_VERSION,
            corpus: report.corpus.clone(),
            tools: report.tools.clone(),
            observations,
        }
    }
}

/// One added, removed, or changed tool observation.
#[derive(Clone, Debug, Serialize)]
pub struct BaselineChange {
    /// Case ID requiring review.
    pub case_id: String,
    /// Tool name.
    pub tool: String,
    /// Prior observation, if any.
    pub before: Option<BaselineObservation>,
    /// Candidate observation, if any.
    pub after: Option<BaselineObservation>,
}

/// Reviewed update validation and persistence errors.
#[derive(Debug, Error)]
pub enum BaselineError {
    /// Filesystem failure.
    #[error("baseline I/O failed: {0}")]
    Io(#[from] io::Error),
    /// JSON failure.
    #[error("baseline JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    /// Candidate contains changes not explicitly reviewed.
    #[error("changed cases lack explicit review: {0}")]
    MissingReviews(String),
    /// Review list names cases with no change.
    #[error("review list contains unchanged or unknown cases: {0}")]
    UnknownReviews(String),
    /// Atomic persistence failed.
    #[error("could not atomically install baseline: {0}")]
    Persist(#[from] tempfile::PersistError),
}

/// Returns every per-tool observation change in stable order.
#[must_use]
pub fn diff_baselines(
    current: Option<&CompatibilityBaseline>,
    candidate: &CompatibilityBaseline,
) -> Vec<BaselineChange> {
    let empty = BTreeMap::new();
    let current_cases = current.map_or(&empty, |value| &value.observations);
    let mut cases = current_cases
        .keys()
        .chain(candidate.observations.keys())
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let mut changes = Vec::new();
    for case_id in &cases {
        let old = current_cases.get(case_id);
        let new = candidate.observations.get(case_id);
        let tools = old
            .into_iter()
            .flat_map(BTreeMap::keys)
            .chain(new.into_iter().flat_map(BTreeMap::keys))
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        for tool in tools {
            let before = old.and_then(|values| values.get(&tool));
            let after = new.and_then(|values| values.get(&tool));
            if before != after {
                changes.push(BaselineChange {
                    case_id: case_id.clone(),
                    tool,
                    before: before.cloned(),
                    after: after.cloned(),
                });
            }
        }
    }
    cases.clear();
    changes
}

/// Accepts a candidate only when every changed case ID is explicitly listed.
///
/// This deliberately provides no `accept all` path. Callers must present and
/// enumerate the exact reviewed case IDs.
///
/// # Errors
///
/// Returns missing or spurious review IDs.
pub fn accept_reviewed_candidate(
    current: Option<&CompatibilityBaseline>,
    candidate: CompatibilityBaseline,
    reviewed_case_ids: &std::collections::BTreeSet<String>,
) -> Result<CompatibilityBaseline, BaselineError> {
    let changed = diff_baselines(current, &candidate)
        .into_iter()
        .map(|change| change.case_id)
        .collect::<std::collections::BTreeSet<_>>();
    let missing = changed
        .difference(reviewed_case_ids)
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(BaselineError::MissingReviews(missing.join(", ")));
    }
    let unknown = reviewed_case_ids
        .difference(&changed)
        .cloned()
        .collect::<Vec<_>>();
    if !unknown.is_empty() {
        return Err(BaselineError::UnknownReviews(unknown.join(", ")));
    }
    Ok(candidate)
}

/// Reads a reviewed baseline JSON file.
///
/// # Errors
///
/// Returns I/O or JSON errors.
pub fn read_baseline(path: &Path) -> Result<CompatibilityBaseline, BaselineError> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

/// Writes a newline-terminated baseline via atomic rename.
///
/// # Errors
///
/// Returns serialization or atomic persistence errors.
pub fn write_baseline_atomic(
    path: &Path,
    baseline: &CompatibilityBaseline,
) -> Result<(), BaselineError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let mut temporary = NamedTempFile::new_in(parent)?;
    serde_json::to_writer_pretty(&mut temporary, baseline)?;
    temporary.write_all(b"\n")?;
    temporary.flush()?;
    temporary.as_file().sync_all()?;
    temporary.persist(path)?;
    Ok(())
}

const fn tool_name(tool: ToolKind) -> &'static str {
    match tool {
        ToolKind::Jq => "jq",
        ToolKind::Yq => "yq",
        ToolKind::Tq => "tq",
    }
}
