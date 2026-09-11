//! Manual-audit storage with TOON preferred over legacy JSON ledgers.

use std::{collections::BTreeSet, fs, io, path::Path};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::CompatibilityCatalog;

/// The baseline disposition assigned to one frozen manual gap.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum GapBaselineVerdict {
    /// The baseline execution did not satisfy the contract.
    Failure,
    /// The baseline was recorded as a former policy difference.
    ExpectedDifference,
    /// The imported manual and pinned executable disagree independently of tq.
    ReferenceDiscrepancy,
}

/// One row in the frozen manual-gap closure inventory.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ManualGapEntry {
    /// Stable executable case ID.
    pub case_id: String,
    /// Baseline comparison disposition.
    pub baseline_verdict: GapBaselineVerdict,
    /// Owning implementation workstream.
    pub workstream: String,
}

/// The versioned closure inventory for the manual comparison baseline.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ManualGapInventory {
    /// Commit from which the baseline comparison was imported.
    pub baseline_commit: String,
    /// Source report from which the rows were derived.
    pub source: String,
    /// Every non-matching baseline case, including reference discrepancies.
    pub entries: Vec<ManualGapEntry>,
}

/// Exact baseline counts proved by the closure inventory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GapInventorySummary {
    /// Total inventory rows.
    pub total: usize,
    /// Failure rows.
    pub failures: usize,
    /// Former policy-difference rows.
    pub expected_differences: usize,
    /// Reference-discrepancy rows.
    pub reference_discrepancies: usize,
}

/// One explicitly reviewed safe-library or platform disparity.
///
/// The case ID, declared contract, observed difference summary, and stable
/// evidence are all exact-match requirements for an approval. Empty scope
/// values are rejected so an approval cannot silently broaden to another
/// observation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReviewedDisparity {
    /// Stable executable case ID.
    pub case_id: String,
    /// Declared contract of the observed case.
    pub contract: super::ContractKind,
    /// Exact semantic-difference summary observed for the case.
    pub difference_summary: String,
    /// Human explanation and reconsideration rationale.
    pub rationale: String,
    /// Exact stable evidence captured by the comparison report.
    pub evidence: DisparityEvidence,
}

/// Stable jq/tq observations and identities supporting one disparity review.
///
/// Timing and harness notes are intentionally absent. The catalog fingerprint
/// covers the query, fixture, adapters, and declared contract, while the tool
/// identities bind the observation to the discovered reference and target.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DisparityEvidence {
    /// SHA-256 fingerprint of the serialized compatibility catalog case.
    pub case_fingerprint: String,
    /// Identity of the jq reference executable.
    pub reference_identity: super::ToolIdentity,
    /// Identity of the tq target executable.
    pub target_identity: super::ToolIdentity,
    /// Stable jq observation projection.
    pub jq: ReviewedObservation,
    /// Stable tq observation projection.
    pub tq: ReviewedObservation,
    /// Stable tq TOON observation projection.
    pub tq_toon: Option<ReviewedObservation>,
    /// Stable compact jq observation projection.
    pub compact_jq: Option<ReviewedObservation>,
    /// Stable compact tq observation projection.
    pub compact_tq: Option<ReviewedObservation>,
    /// Whether the TOON execution contract matched tq's JSON execution.
    pub toon_contract_match: Option<bool>,
    /// Whether compact jq and tq stdout bytes matched exactly.
    pub compact_exact: Option<bool>,
    /// Stable compact semantic differences.
    pub compact_differences: Option<Vec<Value>>,
    /// Stable semantic differences in the primary JSON comparison.
    pub semantic_differences: Vec<Value>,
}

impl DisparityEvidence {
    /// Extracts stable approval evidence from one generated report row.
    ///
    /// # Errors
    ///
    /// Returns a report-shape error when identities, fingerprints, or
    /// observations are missing or malformed.
    #[allow(clippy::too_many_lines)]
    pub fn from_report(report: &Value, case: &Value) -> Result<Self, DisparityValidationError> {
        let case_fingerprint = case["case_fingerprint"].as_str().ok_or_else(|| {
            DisparityValidationError::Report(
                "disparity case needs a catalog fingerprint".to_owned(),
            )
        })?;
        let tools = report["tools"].as_array().ok_or_else(|| {
            DisparityValidationError::Report("disparity report needs tool identities".to_owned())
        })?;
        let mut reference_identity = None;
        let mut target_identity = None;
        for tool in tools {
            let identity = serde_json::from_value::<super::ToolIdentity>(tool.clone())
                .map_err(|error| DisparityValidationError::Report(error.to_string()))?;
            match identity.tool {
                super::ToolKind::Jq => {
                    if reference_identity.replace(identity).is_some() {
                        return Err(DisparityValidationError::Report(
                            "disparity report has duplicate jq identities".to_owned(),
                        ));
                    }
                }
                super::ToolKind::Tq => {
                    if target_identity.replace(identity).is_some() {
                        return Err(DisparityValidationError::Report(
                            "disparity report has duplicate tq identities".to_owned(),
                        ));
                    }
                }
                super::ToolKind::Yq => {}
            }
        }
        let reference_identity = reference_identity.ok_or_else(|| {
            DisparityValidationError::Report("disparity report is missing jq identity".to_owned())
        })?;
        let target_identity = target_identity.ok_or_else(|| {
            DisparityValidationError::Report("disparity report is missing tq identity".to_owned())
        })?;
        let tq_toon = (!case["tq_toon"].is_null())
            .then(|| ReviewedObservation::from_report(&case["tq_toon"]))
            .transpose()?;
        let toon_contract_match = case["toon_contract_match"].as_bool();
        let (
            compact_reference_observation,
            compact_target_observation,
            compact_exact,
            compact_differences,
        ) = match case["compact"].as_object() {
            Some(compact) => {
                let exact = compact
                    .get("exact")
                    .and_then(Value::as_bool)
                    .ok_or_else(|| {
                        DisparityValidationError::Report(
                            "disparity compact campaign needs a boolean exact result".to_owned(),
                        )
                    })?;
                let differences = compact
                    .get("differences")
                    .and_then(Value::as_array)
                    .cloned()
                    .ok_or_else(|| {
                        DisparityValidationError::Report(
                            "disparity compact campaign needs a differences array".to_owned(),
                        )
                    })?;
                let jq = ReviewedObservation::from_report(compact.get("jq").ok_or_else(|| {
                    DisparityValidationError::Report(
                        "disparity compact campaign is missing jq observation".to_owned(),
                    )
                })?)?;
                let tq = ReviewedObservation::from_report(compact.get("tq").ok_or_else(|| {
                    DisparityValidationError::Report(
                        "disparity compact campaign is missing tq observation".to_owned(),
                    )
                })?)?;
                (Some(jq), Some(tq), Some(exact), Some(differences))
            }
            None if case["compact"].is_null() => (None, None, None, None),
            None => {
                return Err(DisparityValidationError::Report(
                    "disparity case needs a compact campaign object".to_owned(),
                ));
            }
        };
        let semantic_differences = case["differences"].as_array().cloned().ok_or_else(|| {
            DisparityValidationError::Report("disparity case needs a differences array".to_owned())
        })?;
        if semantic_differences.is_empty() {
            return Err(DisparityValidationError::Report(
                "disparity case needs at least one semantic difference".to_owned(),
            ));
        }
        Ok(Self {
            case_fingerprint: case_fingerprint.to_owned(),
            reference_identity,
            target_identity,
            jq: ReviewedObservation::from_report(&case["jq"])?,
            tq: ReviewedObservation::from_report(&case["tq"])?,
            tq_toon,
            compact_jq: compact_reference_observation,
            compact_tq: compact_target_observation,
            toon_contract_match,
            compact_exact,
            compact_differences,
            semantic_differences,
        })
    }
}

/// Stable, non-timing projection of one tool observation.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReviewedObservation {
    /// Tool role.
    pub tool: super::ToolKind,
    /// Physical input representation.
    pub input_format: Option<super::FixtureFormat>,
    /// Observation state.
    pub state: super::ObservationState,
    /// Ordered structured values.
    pub results: Vec<Value>,
    /// Exact stdout bytes.
    pub stdout_hex: Option<String>,
    /// Exact raw stdout bytes.
    pub raw_stdout_hex: Option<String>,
    /// Exact stderr bytes.
    pub stderr_hex: Option<String>,
    /// Process status.
    pub process_status: Option<super::ProcessStatus>,
    /// Exit code.
    pub exit_code: Option<i32>,
    /// Stable normalized error class.
    pub error_class: Option<super::ErrorClass>,
}

impl PartialEq for ReviewedObservation {
    fn eq(&self, other: &Self) -> bool {
        self.tool == other.tool
            && self.input_format == other.input_format
            && self.state == other.state
            && exact_json_arrays_equal(&self.results, &other.results)
            && self.stdout_hex == other.stdout_hex
            && self.raw_stdout_hex == other.raw_stdout_hex
            && self.stderr_hex == other.stderr_hex
            && self.process_status == other.process_status
            && self.exit_code == other.exit_code
            && self.error_class == other.error_class
    }
}

impl Eq for ReviewedObservation {}

/// Compares structured observation values by exact decimal value rather than
/// by `serde_json`'s lexical arbitrary-precision representation. The TOON
/// decoder can represent the same finite maximum as either its expanded
/// decimal or its scientific spelling depending on whether the source value
/// arrived as a retained literal or a runtime f64. Both spellings are exact
/// decimal values; this is not a tolerance or binary64 comparison.
fn exact_json_values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => {
            exact_decimal_key(left) == exact_decimal_key(right)
        }
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| exact_json_values_equal(left, right))
        }
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left.iter().all(|(key, value)| {
                    right
                        .get(key)
                        .is_some_and(|other| exact_json_values_equal(value, other))
                })
        }
        _ => left == right,
    }
}

fn exact_json_arrays_equal(left: &[Value], right: &[Value]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| exact_json_values_equal(left, right))
}

fn exact_decimal_key(number: &serde_json::Number) -> String {
    let spelling = number.to_string();
    let canonical = tq_core::Number::canonicalize_literal_numeric(&spelling)
        .unwrap_or_else(|_| spelling.clone());
    if spelling.starts_with('-') && canonical == "0" {
        "-0".to_owned()
    } else {
        canonical
    }
}

impl ReviewedObservation {
    fn from_report(value: &Value) -> Result<Self, DisparityValidationError> {
        serde_json::from_value(value.clone())
            .map_err(|error| DisparityValidationError::Report(error.to_string()))
    }
}

/// Computes the stable catalog-case fingerprint embedded in manual reports.
///
/// # Errors
///
/// Returns a serialization error if the catalog schema cannot be encoded.
pub fn case_fingerprint(case: &super::CompatibilityCase) -> Result<String, serde_json::Error> {
    let bytes = serde_json::to_vec(case)?;
    Ok(super::encode_hex(&Sha256::digest(bytes)))
}

/// Separate report counts for exact matches, reviewed disparities, and failures.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ManualVerdictCounts {
    /// Cases whose jq and tq contracts exactly match.
    pub exact_matches: usize,
    /// Cases covered by an explicit reviewed disparity approval.
    pub reviewed_disparities: usize,
    /// Cases that are unresolved, unverified, or otherwise not exact.
    pub failures: usize,
}

/// Errors found while loading or closing the manual inventory.
#[derive(Debug, Error)]
pub enum ManualInventoryError {
    /// Inventory or review-model I/O failed.
    #[error("manual inventory I/O failed: {0}")]
    Io(#[from] io::Error),
    /// Inventory schema or review-model shape was invalid.
    #[error("manual inventory is invalid: {0}")]
    Invalid(String),
    /// The same gap ID appeared more than once.
    #[error("manual gap inventory has duplicate case ID: {0}")]
    DuplicateCaseId(String),
    /// The frozen count contract does not hold.
    #[error("manual gap inventory count mismatch: {0}")]
    CountMismatch(String),
    /// A gap row could not be connected to the executable or review model.
    #[error("manual gap inventory closure failed: {0}")]
    Closure(String),
}

/// Errors validating an explicit reviewed disparity approval.
#[derive(Debug, Error)]
pub enum DisparityValidationError {
    /// The comparison report does not have the required shape.
    #[error("disparity report is invalid: {0}")]
    Report(String),
    /// An approval refers to no executable report case.
    #[error("unknown disparity case ID: {0}")]
    UnknownCase(String),
    /// The same case was approved more than once.
    #[error("duplicate disparity approval for case ID: {0}")]
    DuplicateCase(String),
    /// The approval no longer matches the current observation.
    #[error("stale disparity approval for {case_id}: {reason}")]
    Stale {
        /// Case ID whose current observation no longer matches.
        case_id: String,
        /// Exact evidence mismatch.
        reason: String,
    },
    /// The approval omitted an exact scope field.
    #[error("overly broad disparity approval for {0}")]
    Broad(String),
}

/// Errors reported by the strict manual parity gate.
#[derive(Debug, Error)]
pub enum StrictCampaignError {
    /// The frozen inventory, catalog, or review model is incomplete.
    #[error(transparent)]
    Inventory(#[from] ManualInventoryError),
    /// The report did not have the required machine-readable shape.
    #[error("strict manual campaign report is invalid: {0}")]
    Report(String),
    /// At least one required case did not pass its executable contract.
    #[error("strict manual campaign failed:\n{0}")]
    Failed(String),
}

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

/// Reads the frozen gap inventory from its canonical TOON path.
///
/// # Errors
///
/// Returns an I/O or typed TOON/schema error.
pub fn read_gap_inventory(path: &Path) -> Result<ManualGapInventory, ManualInventoryError> {
    crate::fixture_data::read(path).map_err(ManualInventoryError::Io)
}

/// Collects executable case IDs linked from the section review ledgers.
///
/// The generated comparison report and execution metadata are intentionally
/// excluded: this set is the source-review model, not an observation ledger.
///
/// # Errors
///
/// Returns an I/O, decoding, or malformed-reference error.
pub fn read_manual_review_case_ids(
    reviews: &Path,
) -> Result<BTreeSet<String>, ManualInventoryError> {
    let mut paths = fs::read_dir(reviews)?
        .map(|entry| entry.map(|value| value.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    let mut ids = BTreeSet::new();
    for path in paths {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if !(name.ends_with(".toon") || name.ends_with(".json"))
            || name == "comparison.toon"
            || name == "comparison.json"
            || name == "execution.toon"
            || name == "execution.json"
            || name == "source-examples.toon"
            || name == "source-examples.json"
            || name == "arity-inventory.toon"
            || name == "gap-inventory.toon"
            || name == "reference-pin.toon"
            || name.ends_with(".json") && path.with_extension("toon").exists()
        {
            continue;
        }
        let ledger = read_manual_ledger(&path).map_err(|error| {
            ManualInventoryError::Invalid(format!("{}: {error}", path.display()))
        })?;
        if name != "completeness.toon" {
            for name in ["examples", "coverage_notes"] {
                let Some(rows) = ledger[name].as_array() else {
                    continue;
                };
                for row in rows {
                    if row.get("case_id").is_none() && row.get("evidence_case_id").is_none() {
                        continue;
                    }
                    for id in manual_case_ids(row).map_err(|error| {
                        ManualInventoryError::Invalid(format!("{}: {error}", path.display()))
                    })? {
                        ids.insert(id.to_owned());
                    }
                }
            }
            if let Some(edges) = ledger["coverage_evidence"].as_array() {
                for edge in edges {
                    let id = edge["case_id"].as_str().ok_or_else(|| {
                        ManualInventoryError::Invalid(format!(
                            "{}: coverage_evidence.case_id must be a string",
                            path.display()
                        ))
                    })?;
                    ids.insert(id.to_owned());
                }
            }
        }
        if name == "completeness.toon"
            && let Some(requirements) = ledger["requirements"].as_array()
        {
            for requirement in requirements {
                if requirement["evidence_kind"] == "catalog-case"
                    && let Some(id) = requirement["evidence_ref"].as_str()
                {
                    ids.insert(id.to_owned());
                }
            }
        }
    }
    Ok(ids)
}

/// Counts exact matches, reviewed disparities, and every unresolved verdict in
/// a manual comparison report.
///
/// Historical verdict labels such as `expected-difference` and
/// `reference-discrepancy` remain failures here. They are provenance, not an
/// approval to treat a case as a reviewed disparity.
///
/// # Errors
///
/// Returns an error when the report does not contain a case array with scalar
/// verdicts.
pub fn manual_verdict_counts(
    report: &Value,
) -> Result<ManualVerdictCounts, DisparityValidationError> {
    let cases = report["cases"]
        .as_array()
        .ok_or_else(|| DisparityValidationError::Report("cases must be an array".to_owned()))?;
    let mut counts = ManualVerdictCounts::default();
    for case in cases {
        match case["verdict"].as_str() {
            Some("match") => counts.exact_matches += 1,
            Some("disparity") => counts.reviewed_disparities += 1,
            Some(_) => counts.failures += 1,
            None => {
                return Err(DisparityValidationError::Report(
                    "every case needs a scalar verdict".to_owned(),
                ));
            }
        }
    }
    Ok(counts)
}

/// Applies explicit reviewed disparity approvals to a comparison report.
///
/// An approval is valid only when it names one current failed case, its exact
/// contract, and its exact observed semantic-difference summary. All primary
/// and encoding observations are validated before the report is mutated.
///
/// # Errors
///
/// Returns unknown, duplicate, broad, stale, or malformed-report errors.
pub fn apply_reviewed_disparities(
    report: &mut Value,
    approvals: &[ReviewedDisparity],
) -> Result<ManualVerdictCounts, DisparityValidationError> {
    let cases = report["cases"]
        .as_array()
        .ok_or_else(|| DisparityValidationError::Report("cases must be an array".to_owned()))?;
    let mut rows = BTreeSet::new();
    let mut row_indices = std::collections::BTreeMap::new();
    for (index, case) in cases.iter().enumerate() {
        let id = case["id"].as_str().ok_or_else(|| {
            DisparityValidationError::Report("every case needs a scalar id".to_owned())
        })?;
        if !rows.insert(id.to_owned()) {
            return Err(DisparityValidationError::Report(format!(
                "duplicate report case ID: {id}"
            )));
        }
        row_indices.insert(id.to_owned(), index);
    }

    let mut validated = Vec::with_capacity(approvals.len());
    let mut approved_ids = BTreeSet::new();
    for approval in approvals {
        validate_approval_scope(approval)?;
        if !approved_ids.insert(approval.case_id.clone()) {
            return Err(DisparityValidationError::DuplicateCase(
                approval.case_id.clone(),
            ));
        }
        let Some(&index) = row_indices.get(&approval.case_id) else {
            return Err(DisparityValidationError::UnknownCase(
                approval.case_id.clone(),
            ));
        };
        let case = &cases[index];
        let preapproved = case["verdict"].as_str() == Some("disparity");
        if !preapproved && case["verdict"].as_str() != Some("failure") {
            return Err(DisparityValidationError::Stale {
                case_id: approval.case_id.clone(),
                reason: "case is not an unresolved failure".to_owned(),
            });
        }
        if preapproved {
            let embedded =
                serde_json::from_value::<ReviewedDisparity>(case["reviewed_disparity"].clone())
                    .map_err(|error| DisparityValidationError::Report(error.to_string()))?;
            if embedded != *approval {
                return Err(DisparityValidationError::Stale {
                    case_id: approval.case_id.clone(),
                    reason: "embedded reviewed disparity approval changed".to_owned(),
                });
            }
        }
        let observed_contract =
            serde_json::from_value::<super::ContractKind>(case["contract"].clone())
                .map_err(|error| DisparityValidationError::Report(error.to_string()))?;
        if observed_contract != approval.contract {
            return Err(DisparityValidationError::Stale {
                case_id: approval.case_id.clone(),
                reason: format!("contract is {observed_contract:?}"),
            });
        }
        let differences = case["differences"].as_array().ok_or_else(|| {
            DisparityValidationError::Report(format!(
                "{}: differences must be an array",
                approval.case_id
            ))
        })?;
        if differences.is_empty() {
            return Err(DisparityValidationError::Stale {
                case_id: approval.case_id.clone(),
                reason: "observed contract must contain a semantic difference".to_owned(),
            });
        }
        let observed_summary = differences[0]["summary"].as_str().ok_or_else(|| {
            DisparityValidationError::Report(format!(
                "{}: semantic difference needs a summary",
                approval.case_id
            ))
        })?;
        if observed_summary != approval.difference_summary {
            return Err(DisparityValidationError::Stale {
                case_id: approval.case_id.clone(),
                reason: format!("observed difference is {observed_summary:?}"),
            });
        }
        validate_approval_evidence(report, case, approval)?;
        validated.push(index);
    }

    let cases = report["cases"].as_array_mut().ok_or_else(|| {
        DisparityValidationError::Report("cases changed while validating approvals".to_owned())
    })?;
    for (index, approval) in validated.into_iter().zip(approvals) {
        cases[index]["verdict"] = Value::String("disparity".to_owned());
        cases[index]["reason"] = Value::String(approval.rationale.clone());
        cases[index]["reviewed_disparity"] = serde_json::to_value(approval)
            .map_err(|error| DisparityValidationError::Report(error.to_string()))?;
    }
    manual_verdict_counts(report)
}

fn validate_approval_evidence(
    report: &Value,
    case: &Value,
    approval: &ReviewedDisparity,
) -> Result<(), DisparityValidationError> {
    if DisparityEvidence::from_report(report, case)? != approval.evidence {
        return Err(DisparityValidationError::Stale {
            case_id: approval.case_id.clone(),
            reason: "stable observation, tool identity, or catalog fingerprint changed".to_owned(),
        });
    }
    for role in ["jq", "tq"] {
        if case[role]["state"].as_str() != Some("executed") {
            return Err(DisparityValidationError::Stale {
                case_id: approval.case_id.clone(),
                reason: format!("{role} observation is not executed"),
            });
        }
        if case[role]["process_status"].as_str() != Some("exited") {
            return Err(DisparityValidationError::Stale {
                case_id: approval.case_id.clone(),
                reason: format!("{role} observation did not exit normally"),
            });
        }
    }
    Ok(())
}

fn validate_approval_scope(approval: &ReviewedDisparity) -> Result<(), DisparityValidationError> {
    if approval.case_id.trim().is_empty()
        || approval.difference_summary.trim().is_empty()
        || approval.rationale.trim().is_empty()
        || [
            approval.case_id.as_str(),
            approval.difference_summary.as_str(),
        ]
        .iter()
        .any(|value| value.contains('*'))
    {
        return Err(DisparityValidationError::Broad(approval.case_id.clone()));
    }
    Ok(())
}

/// Validates the frozen gap rows against executable cases, source-review
/// mappings, and a generated comparison report.
///
/// Baseline labels are provenance only. This function does not accept any
/// label as a successful execution verdict.
///
/// # Errors
///
/// Returns every closure problem that prevents a complete baseline inventory.
pub fn validate_gap_inventory(
    inventory: &ManualGapInventory,
    catalog: &CompatibilityCatalog,
    review_case_ids: &BTreeSet<String>,
    report_case_ids: &BTreeSet<String>,
) -> Result<GapInventorySummary, ManualInventoryError> {
    let mut seen = BTreeSet::new();
    for entry in &inventory.entries {
        if !seen.insert(entry.case_id.as_str()) {
            return Err(ManualInventoryError::DuplicateCaseId(entry.case_id.clone()));
        }
    }
    let summary = inventory.entries.iter().fold(
        GapInventorySummary {
            total: inventory.entries.len(),
            failures: 0,
            expected_differences: 0,
            reference_discrepancies: 0,
        },
        |mut summary, entry| {
            match entry.baseline_verdict {
                GapBaselineVerdict::Failure => summary.failures += 1,
                GapBaselineVerdict::ExpectedDifference => summary.expected_differences += 1,
                GapBaselineVerdict::ReferenceDiscrepancy => summary.reference_discrepancies += 1,
            }
            summary
        },
    );
    if summary.total != 215 {
        return Err(ManualInventoryError::CountMismatch(format!(
            "expected 215 rows, found {}",
            summary.total
        )));
    }
    for (name, expected, actual) in [
        ("failure", 198, summary.failures),
        ("expected-difference", 15, summary.expected_differences),
        ("reference-discrepancy", 2, summary.reference_discrepancies),
    ] {
        if actual != expected {
            return Err(ManualInventoryError::CountMismatch(format!(
                "expected {expected} {name} rows, found {actual}"
            )));
        }
    }

    for entry in &inventory.entries {
        let Some(case) = catalog.cases.iter().find(|case| case.id == entry.case_id) else {
            return Err(ManualInventoryError::Closure(format!(
                "{} is missing from executable catalog",
                entry.case_id
            )));
        };
        if case.status != super::CaseStatus::Mvp
            || !case.adapters.jq.supported
            || !case.adapters.tq.supported
        {
            return Err(ManualInventoryError::Closure(format!(
                "{} is not an executable jq/tq catalog case",
                entry.case_id
            )));
        }
        if !review_case_ids.contains(&entry.case_id) {
            return Err(ManualInventoryError::Closure(format!(
                "missing source mapping: {}",
                entry.case_id
            )));
        }
        if !report_case_ids.contains(&entry.case_id) {
            return Err(ManualInventoryError::Closure(format!(
                "{} is missing from comparison report",
                entry.case_id
            )));
        }
    }
    Ok(summary)
}

/// Runs the strict release-gate policy against a manual comparison report.
///
/// Every catalog case whose ID belongs to the manual corpus must be present,
/// source-reviewed, and executed with both jq and tq. Only an actual `match`
/// verdict passes; expected differences, reference discrepancies, skips,
/// timeouts, crashes, normalization errors, and any recorded difference fail.
///
/// # Errors
///
/// Returns a report-shape, closure, or strict-contract failure.
pub fn validate_strict_manual_report(
    report: &Value,
    catalog: &CompatibilityCatalog,
    inventory: &ManualGapInventory,
    review_case_ids: &BTreeSet<String>,
) -> Result<(), StrictCampaignError> {
    validate_manual_report(report, catalog, inventory, review_case_ids, false)
}

/// Validates a completion campaign that explicitly accounts for reviewed
/// disparities while retaining exact-match and unresolved-failure counts.
///
/// The input report is cloned before approvals are applied, so report evidence
/// remains unchanged for callers and exact-parity mode stays independently
/// strict.
///
/// # Errors
///
/// Returns stale/unknown approvals, closure failures, or unresolved cases.
pub fn validate_completion_manual_report(
    report: &Value,
    catalog: &CompatibilityCatalog,
    inventory: &ManualGapInventory,
    review_case_ids: &BTreeSet<String>,
    approvals: &[ReviewedDisparity],
) -> Result<(), StrictCampaignError> {
    // Approvals may cover newly reviewed source witnesses as well as frozen
    // gap rows. The pinned original-case check below keeps every original
    // exact match a regression requirement.
    let catalog_ids = catalog
        .cases
        .iter()
        .map(|case| case.id.as_str())
        .collect::<BTreeSet<_>>();
    for approval in approvals {
        if !review_case_ids.contains(&approval.case_id)
            || !catalog_ids.contains(approval.case_id.as_str())
        {
            return Err(StrictCampaignError::Report(format!(
                "{}: disparity approval is not a reviewed executable case",
                approval.case_id
            )));
        }
    }
    let already_reviewed = report["cases"]
        .as_array()
        .is_some_and(|cases| cases.iter().any(|case| case["verdict"] == "disparity"));
    if already_reviewed && approvals.is_empty() {
        return Err(StrictCampaignError::Report(
            "completion report contains a disparity before explicit approvals were applied"
                .to_owned(),
        ));
    }
    if let Some(cases) = report["cases"].as_array() {
        for case in cases.iter().filter(|case| case["verdict"] == "disparity") {
            if !approvals
                .iter()
                .any(|approval| case["id"] == approval.case_id)
            {
                return Err(StrictCampaignError::Report(format!(
                    "{}: embedded disparity has no explicit approval",
                    case["id"]
                )));
            }
        }
    }
    let mut report = report.clone();
    apply_reviewed_disparities(&mut report, approvals)
        .map_err(|error| StrictCampaignError::Report(error.to_string()))?;
    validate_manual_report(&report, catalog, inventory, review_case_ids, true)
}

// This function is intentionally a single integrity pass so every coverage,
// identity, and execution-contract failure is accumulated for one report.
#[allow(clippy::too_many_lines)]
fn validate_manual_report(
    report: &Value,
    catalog: &CompatibilityCatalog,
    inventory: &ManualGapInventory,
    review_case_ids: &BTreeSet<String>,
    allow_disparities: bool,
) -> Result<(), StrictCampaignError> {
    let pin: super::ManualReferencePin = crate::fixture_data::from_toon(include_bytes!(
        "../../../../tests/compatibility/reviews/jq-manual/reference-pin.toon"
    ))
    .map_err(|error| StrictCampaignError::Report(error.to_string()))?;
    let cases = report["cases"]
        .as_array()
        .ok_or_else(|| StrictCampaignError::Report("cases must be an array".to_owned()))?;
    let report_case_ids = report_case_ids(cases)?;
    validate_gap_inventory(inventory, catalog, review_case_ids, &report_case_ids)?;

    let mut expected = catalog
        .cases
        .iter()
        .filter(|case| case.id.starts_with("manual.") || case.id.starts_with("manual-"))
        .map(|case| case.id.clone())
        .collect::<BTreeSet<_>>();
    expected.extend(review_case_ids.iter().filter_map(|id| {
        catalog
            .cases
            .iter()
            .find(|case| case.id == *id)
            .filter(|case| {
                case.status == super::CaseStatus::Mvp
                    && case.adapters.jq.supported
                    && case.adapters.tq.supported
            })
            .map(|case| case.id.clone())
    }));
    let mut failures = Vec::new();
    if let Err(error) = super::validate_pinned_manual_coverage(&pin, report) {
        failures.push(error.to_string());
    }
    if expected.len() < 518 {
        failures.push(format!(
            "missing coverage: expected at least 518 manual cases, found {}",
            expected.len()
        ));
    }
    if review_case_ids.len() < 518 {
        failures.push(format!(
            "missing source mapping: expected at least 518 manual cases, found {}",
            review_case_ids.len()
        ));
    }
    if report_case_ids.len() < 518 {
        failures.push(format!(
            "missing coverage: expected at least 518 report cases, found {}",
            report_case_ids.len()
        ));
    }
    validate_process_capability_adapters(&expected, catalog, &mut failures);
    let catalog_ids = catalog
        .cases
        .iter()
        .map(|case| case.id.as_str())
        .collect::<BTreeSet<_>>();
    for id in expected.difference(review_case_ids) {
        failures.push(format!("missing source mapping: {id}"));
    }
    for id in expected.difference(&report_case_ids) {
        failures.push(format!("missing coverage: {id}"));
    }
    for id in review_case_ids
        .iter()
        .filter(|id| !catalog_ids.contains(id.as_str()))
    {
        failures.push(format!("unknown source mapping: {id}"));
    }
    for id in report_case_ids.difference(&expected) {
        failures.push(format!("unexpected report case: {id}"));
    }
    for id in review_case_ids {
        let Some(case) = catalog.cases.iter().find(|case| case.id == *id) else {
            continue;
        };
        if report_case_ids.contains(id)
            && (case.status != super::CaseStatus::Mvp
                || !case.adapters.jq.supported
                || !case.adapters.tq.supported)
        {
            failures.push(format!("{id}: skip (catalog case is not executable)"));
        }
    }
    for id in &expected {
        let Some(case) = catalog.cases.iter().find(|case| case.id == *id) else {
            continue;
        };
        if case.status != super::CaseStatus::Mvp
            || !case.adapters.jq.supported
            || !case.adapters.tq.supported
        {
            failures.push(format!("{id}: skip (catalog case is not executable)"));
        }
        if [&case.adapters.jq, &case.adapters.tq]
            .iter()
            .any(|adapter| {
                adapter
                    .query
                    .as_ref()
                    .is_some_and(|query| query != &case.query)
            })
        {
            failures.push(format!(
                "{id}: manual query must not be rewritten by an adapter"
            ));
        }
    }
    let rows = cases
        .iter()
        .filter_map(|case| case["id"].as_str().map(|id| (id, case)))
        .collect::<std::collections::BTreeMap<_, _>>();
    for id in &expected {
        let Some(case) = rows.get(id.as_str()) else {
            continue;
        };
        let catalog_case = catalog
            .cases
            .iter()
            .find(|catalog_case| catalog_case.id == *id);
        let expected_fingerprint = catalog_case.and_then(|case| case_fingerprint(case).ok());
        validate_report_case(
            id,
            case,
            ReportCaseContext {
                allow_disparities,
                expected_fingerprint: expected_fingerprint.as_deref(),
                compare_stderr: catalog_case.is_some_and(|case| case.expected.compare_stderr),
                catalog_case,
                tq_contract: catalog_case.and_then(|case| case.expected.tq_contract.as_ref()),
                failures: &mut failures,
            },
        );
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(StrictCampaignError::Failed(failures.join("\n")))
    }
}

fn validate_process_capability_adapters(
    expected: &BTreeSet<String>,
    catalog: &CompatibilityCatalog,
    failures: &mut Vec<String>,
) {
    for id in expected {
        let Some(case) = catalog.cases.iter().find(|case| case.id == *id) else {
            continue;
        };
        for flag in ["--allow-environment", "--allow-platform"] {
            if case
                .adapters
                .tq
                .args
                .iter()
                .any(|argument| argument == flag)
            {
                failures.push(format!(
                    "{id}: tq adapter must use process defaults; remove capability override {flag}"
                ));
            }
        }
    }
}

struct ReportCaseContext<'a> {
    allow_disparities: bool,
    expected_fingerprint: Option<&'a str>,
    compare_stderr: bool,
    catalog_case: Option<&'a super::CompatibilityCase>,
    tq_contract: Option<&'a super::TqContract>,
    failures: &'a mut Vec<String>,
}

fn validate_report_case(id: &str, case: &Value, context: ReportCaseContext<'_>) {
    let ReportCaseContext {
        allow_disparities,
        expected_fingerprint,
        compare_stderr,
        catalog_case,
        tq_contract,
        failures,
    } = context;
    let verdict = case["verdict"].as_str().unwrap_or("missing");
    let is_disparity = verdict == "disparity";
    if verdict != "match" && !(allow_disparities && is_disparity) {
        failures.push(format!("{id}: verdict {verdict}"));
    }
    if is_disparity {
        match expected_fingerprint {
            Some(expected) if case["case_fingerprint"] == expected => {}
            Some(_) => failures.push(format!("{id}: stale catalog fingerprint")),
            None => failures.push(format!("{id}: missing catalog fingerprint")),
        }
        if case["reviewed_disparity"].is_null() {
            failures.push(format!("{id}: missing reviewed disparity evidence"));
        }
    }
    let Some(reference) = case.get("jq") else {
        failures.push(format!("{id}: missing reference"));
        return;
    };
    let Some(actual) = case.get("tq") else {
        failures.push(format!("{id}: missing tq observation"));
        return;
    };
    validate_observation(id, "reference", reference, failures);
    validate_observation(id, "tq", actual, failures);
    if let Some(contract) = tq_contract {
        let valid_case = catalog_case.is_some_and(|case| contract.is_valid_for_case(case));
        let matches = serde_json::from_value::<super::ToolObservation>(actual.clone())
            .is_ok_and(|observation| super::tq_contract_matches(*contract, &observation));
        let reference_succeeded = reference["state"] == "executed"
            && reference["process_status"] == "exited"
            && reference["exit_code"] == 0;
        if !valid_case {
            failures.push(format!("{id}: invalid tq-native CLI contract association"));
        } else if !reference_succeeded {
            failures.push(format!("{id}: mismatch (reference CLI contract)"));
        } else if !matches {
            failures.push(format!("{id}: mismatch (tq-native CLI contract assertion)"));
        }
    }
    if tq_contract.is_none()
        && compare_stderr
        && !is_disparity
        && reference["stderr_hex"] != actual["stderr_hex"]
    {
        failures.push(format!("{id}: mismatch (explicit stderr payload)"));
    }
    let Some(differences) = case["differences"].as_array() else {
        failures.push(format!(
            "{id}: normalization error: differences must be an array"
        ));
        return;
    };
    if tq_contract.is_none() && !differences.is_empty() && !is_disparity {
        failures.push(format!("{id}: mismatch ({})", differences.len()));
    }
    let contract = case["contract"].as_str().unwrap_or_default();
    if contract == "result-sequence"
        && !is_disparity
        && case["json_equivalent"] != Value::Bool(true)
    {
        failures.push(format!("{id}: mismatch (JSON result sequence)"));
    }
    if tq_contract.is_none()
        && !is_disparity
        && matches!(contract, "result-sequence" | "error")
        && (reference["results"] != actual["results"]
            || reference["exit_code"] != actual["exit_code"]
            || reference["error_class"] != actual["error_class"])
    {
        failures.push(format!("{id}: mismatch (normalized observation)"));
    }
    if tq_contract.is_none()
        && !is_disparity
        && matches!(contract, "raw-bytes" | "exit-status")
        && (reference["raw_stdout_hex"] != actual["raw_stdout_hex"]
            || reference["exit_code"] != actual["exit_code"]
            || reference["error_class"] != actual["error_class"])
    {
        failures.push(format!("{id}: mismatch (exact process contract)"));
    }
    if matches!(contract, "result-sequence" | "error") {
        validate_encoding_campaigns(id, case, compare_stderr, is_disparity, failures);
    }
}

fn report_case_ids(cases: &[Value]) -> Result<BTreeSet<String>, StrictCampaignError> {
    let mut ids = BTreeSet::new();
    for case in cases {
        let id = case["id"].as_str().ok_or_else(|| {
            StrictCampaignError::Report("every report case needs a string id".to_owned())
        })?;
        if !ids.insert(id.to_owned()) {
            return Err(StrictCampaignError::Report(format!(
                "duplicate report case ID: {id}"
            )));
        }
    }
    Ok(ids)
}

fn validate_encoding_campaigns(
    id: &str,
    case: &Value,
    compare_stderr: bool,
    is_disparity: bool,
    failures: &mut Vec<String>,
) {
    let actual = &case["tq"];
    let toon = &case["tq_toon"];
    validate_observation(id, "TOON", toon, failures);
    if !is_disparity
        && (case["toon_contract_match"] != true
            || actual["results"] != toon["results"]
            || actual["exit_code"] != toon["exit_code"]
            || actual["error_class"] != toon["error_class"]
            || compare_stderr && actual["stderr_hex"] != toon["stderr_hex"])
    {
        failures.push(format!("{id}: mismatch (independent TOON contract)"));
    }
    let compact = &case["compact"];
    validate_observation(id, "compact reference", &compact["jq"], failures);
    validate_observation(id, "compact tq", &compact["tq"], failures);
    let Some(compact_differences) = compact["differences"].as_array() else {
        failures.push(format!(
            "{id}: normalization error: compact differences must be an array"
        ));
        return;
    };
    if !is_disparity
        && (compact["exact"] != true
            || !compact["jq"]["stdout_hex"].is_string()
            || compact["jq"]["stdout_hex"] != compact["tq"]["stdout_hex"]
            || compact["jq"]["exit_code"] != compact["tq"]["exit_code"]
            || compact["jq"]["error_class"] != compact["tq"]["error_class"]
            || compare_stderr && compact["jq"]["stderr_hex"] != compact["tq"]["stderr_hex"]
            || !compact_differences.is_empty())
    {
        failures.push(format!("{id}: mismatch (exact compact JSON contract)"));
    }
}

fn validate_observation(id: &str, role: &str, observation: &Value, failures: &mut Vec<String>) {
    let state = observation["state"].as_str();
    match state {
        Some("executed") => {}
        Some("unsupported" | "unavailable") => {
            failures.push(format!("{id}: skip ({role} observation is {state:?})"));
        }
        Some("harness-error") => {
            failures.push(format!("{id}: normalization error ({role} observation)"));
        }
        Some(other) => failures.push(format!("{id}: skip ({role} observation is {other})")),
        None => failures.push(format!("{id}: missing {role} observation state")),
    }
    if observation["process_status"].as_str() == Some("timed-out")
        || observation["error_class"].as_str() == Some("timeout")
    {
        failures.push(format!("{id}: timeout ({role} observation)"));
    } else if observation["process_status"].as_str() == Some("signaled") {
        failures.push(format!("{id}: crash ({role} observation)"));
    } else if state == Some("executed")
        && observation["process_status"] != Value::String("exited".to_owned())
    {
        failures.push(format!(
            "{id}: missing process completion ({role} observation)"
        ));
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
