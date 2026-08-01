//! Stable compatibility campaign reports.

use std::{collections::BTreeMap, fmt::Write as _};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{ErrorClass, ProcessStatus, ToolIdentity, ToolKind};
use crate::corpus::ArtifactIdentity;

/// Report schema version.
pub const REPORT_SCHEMA_VERSION: u32 = 1;

/// Complete machine-readable campaign report.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompatibilityReport {
    /// Report schema version.
    pub schema_version: u32,
    /// Smoke or full campaign profile.
    pub profile: String,
    /// Compatibility catalog identity.
    pub corpus: ArtifactIdentity,
    /// Discovered executable identities.
    pub tools: Vec<ToolIdentity>,
    /// Per-case observations.
    pub cases: Vec<CaseReport>,
    /// Aggregate capability coverage.
    pub coverage: BTreeMap<String, CoverageCount>,
    /// Final campaign result.
    pub final_status: FinalStatus,
}

/// One logical case across tools.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CaseReport {
    /// Stable case ID.
    pub id: String,
    /// Capability tags.
    pub capabilities: Vec<String>,
    /// Tool observations.
    pub observations: Vec<ToolObservation>,
    /// Pairwise semantic differences.
    pub semantic_diffs: Vec<SemanticDiff>,
}

/// Execution, skip, or harness failure for one tool.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToolObservation {
    /// Tool role.
    pub tool: ToolKind,
    /// Observation state.
    pub state: ObservationState,
    /// Ordered structured values.
    pub results: Vec<Value>,
    /// Exact raw bytes encoded as lowercase hexadecimal.
    pub raw_stdout_hex: Option<String>,
    /// Lossless stderr bytes encoded as lowercase hexadecimal.
    pub stderr_hex: Option<String>,
    /// Process status.
    pub process_status: Option<ProcessStatus>,
    /// Exit code.
    pub exit_code: Option<i32>,
    /// Stable error class.
    pub error_class: Option<ErrorClass>,
    /// Wall time in microseconds.
    pub wall_time_micros: Option<u128>,
    /// Adapter or harness explanation.
    pub note: Option<String>,
}

/// Observation disposition.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObservationState {
    /// Tool ran and produced a normalized observation.
    Executed,
    /// Adapter declares the case inapplicable.
    Unsupported,
    /// Executable was not available.
    Unavailable,
    /// Harness or normalization failed.
    HarnessError,
}

/// A semantic mismatch between two executed tools.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SemanticDiff {
    /// Left tool.
    pub left: ToolKind,
    /// Right tool.
    pub right: ToolKind,
    /// Compact mismatch explanation.
    pub summary: String,
}

/// Per-capability counts.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CoverageCount {
    /// Logical cases carrying this capability.
    pub cases: usize,
    /// Tool observations executed.
    pub executed: usize,
    /// Unsupported or unavailable observations.
    pub skipped: usize,
    /// Harness failures.
    pub harness_errors: usize,
}

/// Overall campaign status.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FinalStatus {
    /// Campaign ran without harness errors or semantic differences.
    Passed,
    /// Campaign ran successfully and recorded reference differences.
    ObservedDifferences,
    /// Harness failures made results incomplete.
    Failed,
}

impl CompatibilityReport {
    /// Renders a concise stable human report.
    #[must_use]
    pub fn render_human(&self) -> String {
        let executed = self
            .cases
            .iter()
            .flat_map(|case| &case.observations)
            .filter(|observation| observation.state == ObservationState::Executed)
            .count();
        let differences = self
            .cases
            .iter()
            .map(|case| case.semantic_diffs.len())
            .sum::<usize>();
        let mut output = format!(
            "compatibility {}: {:?}\ncases: {}  observations: {}  differences: {}\n",
            self.profile,
            self.final_status,
            self.cases.len(),
            executed,
            differences
        );
        for case in self.cases.iter().filter(|case| {
            !case.semantic_diffs.is_empty()
                || case
                    .observations
                    .iter()
                    .any(|observation| observation.state == ObservationState::HarnessError)
        }) {
            writeln!(output, "- {}", case.id).expect("write report to string");
            for difference in &case.semantic_diffs {
                writeln!(
                    output,
                    "  {:?} vs {:?}: {}",
                    difference.left, difference.right, difference.summary
                )
                .expect("write report to string");
            }
            for observation in case
                .observations
                .iter()
                .filter(|value| value.state == ObservationState::HarnessError)
            {
                writeln!(
                    output,
                    "  {:?}: {}",
                    observation.tool,
                    observation.note.as_deref().unwrap_or("harness error")
                )
                .expect("write report to string");
            }
        }
        output
    }
}

/// Lowercase hexadecimal encoding used to preserve arbitrary report bytes.
#[must_use]
pub fn encode_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut hex, byte| {
            write!(hex, "{byte:02x}").expect("write bytes to string");
            hex
        })
}
