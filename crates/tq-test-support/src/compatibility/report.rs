//! Stable compatibility campaign reports.

use std::{collections::BTreeMap, fmt::Write as _};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{ErrorClass, FixtureFormat, ProcessStatus, ToolIdentity, ToolKind};
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
    /// Generated tq disposition for every capability tag.
    #[serde(default)]
    pub capability_matrix: BTreeMap<String, CapabilityDisposition>,
    /// Aggregate counts from the generated capability matrix.
    #[serde(default)]
    pub capability_counts: CapabilityCounts,
    /// Final campaign result.
    pub final_status: FinalStatus,
}

/// tq implementation disposition for a capability tag.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CapabilityDisposition {
    /// Every applicable MVP case executes and has no jq-target difference.
    Supported,
    /// Some applicable MVP cases execute while others remain unsupported.
    Partial,
    /// An executed tq case has a reviewed semantic difference from jq.
    Divergent,
    /// MVP cases exist but none are supported by tq.
    Unsupported,
    /// Every case for this capability is intentionally deferred.
    Deferred,
    /// tq was unavailable, so support was not observed.
    Untested,
}

/// Counts of capability tags by tq disposition.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct CapabilityCounts {
    /// Fully supported capability tags.
    pub supported: usize,
    /// Partially supported capability tags.
    pub partial: usize,
    /// Capability tags with an observed jq-target divergence.
    pub divergent: usize,
    /// Unsupported MVP capability tags.
    pub unsupported: usize,
    /// Intentionally deferred capability tags.
    pub deferred: usize,
    /// Capability tags not exercised with tq available.
    pub untested: usize,
}

impl CapabilityCounts {
    pub(crate) fn record(&mut self, disposition: CapabilityDisposition) {
        match disposition {
            CapabilityDisposition::Supported => self.supported += 1,
            CapabilityDisposition::Partial => self.partial += 1,
            CapabilityDisposition::Divergent => self.divergent += 1,
            CapabilityDisposition::Unsupported => self.unsupported += 1,
            CapabilityDisposition::Deferred => self.deferred += 1,
            CapabilityDisposition::Untested => self.untested += 1,
        }
    }
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
    /// Physical input representation used for this observation.
    #[serde(default)]
    pub input_format: Option<FixtureFormat>,
    /// Observation state.
    pub state: ObservationState,
    /// Ordered structured values.
    pub results: Vec<Value>,
    /// Exact stdout bytes encoded as lowercase hexadecimal.
    pub stdout_hex: Option<String>,
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
    /// Left input representation.
    #[serde(default)]
    pub left_format: Option<FixtureFormat>,
    /// Right tool.
    pub right: ToolKind,
    /// Right input representation.
    #[serde(default)]
    pub right_format: Option<FixtureFormat>,
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
                    "  {:?}/{} vs {:?}/{}: {}",
                    difference.left,
                    format_name(difference.left_format),
                    difference.right,
                    format_name(difference.right_format),
                    difference.summary
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
                    "  {:?}/{}: {}",
                    observation.tool,
                    format_name(observation.input_format),
                    observation.note.as_deref().unwrap_or("harness error")
                )
                .expect("write report to string");
            }
        }
        output
    }
}

fn format_name(format: Option<FixtureFormat>) -> &'static str {
    match format {
        Some(FixtureFormat::Json) => "json",
        Some(FixtureFormat::Yaml) => "yaml",
        Some(FixtureFormat::Toon) => "toon",
        Some(FixtureFormat::Raw) => "raw",
        Some(FixtureFormat::None) => "none",
        None => "unspecified",
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
