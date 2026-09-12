//! Compatibility report and reviewed-baseline tests.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use serde::Deserialize;
use serde_json::json;
use tq_test_support::{
    compatibility::{
        CapabilityCounts, CapabilityDisposition, CompatibilityBaseline, CompatibilityReport,
        CoverageCount, ErrorClass, FinalStatus, FixtureFormat, ObservationState, ProcessStatus,
        ToolKind, ToolObservation, accept_reviewed_candidate, diff_baselines,
    },
    corpus::ArtifactIdentity,
};

#[derive(Debug, Deserialize)]
struct HistoricalCoverageSummary {
    source_artifact: HistoricalArtifact,
    historical_scope: String,
    case_ids: Vec<String>,
    jq_target_diffs: Vec<HistoricalDiff>,
    tq_error_classes: Vec<HistoricalErrorClass>,
    capability_counts: CapabilityCounts,
}

#[derive(Debug, Deserialize)]
struct HistoricalArtifact {
    path: String,
    sha256: String,
}

#[derive(Debug, Deserialize)]
struct HistoricalDiff {
    case_id: String,
    summary: String,
}

#[derive(Debug, Deserialize)]
struct HistoricalErrorClass {
    case_id: String,
    error_class: ErrorClass,
}

fn report(value: i64, duration: u128) -> CompatibilityReport {
    CompatibilityReport {
        schema_version: 1,
        profile: "full".to_owned(),
        corpus: ArtifactIdentity {
            path: "tests/compatibility/cases".to_owned(),
            bytes: 42,
            sha256: "a".repeat(64),
        },
        tools: Vec::new(),
        cases: vec![tq_test_support::compatibility::CaseReport {
            id: "common.identity".to_owned(),
            capabilities: vec!["value.identity".to_owned()],
            observations: vec![ToolObservation {
                tool: ToolKind::Jq,
                input_format: Some(FixtureFormat::Json),
                state: ObservationState::Executed,
                results: vec![json!(value)],
                stdout_hex: Some(format!("{value:02x}")),
                raw_stdout_hex: None,
                stderr_hex: None,
                process_status: Some(ProcessStatus::Exited),
                exit_code: Some(0),
                error_class: None,
                wall_time_micros: Some(duration),
                note: None,
            }],
            semantic_diffs: Vec::new(),
        }],
        coverage: BTreeMap::from([(
            "value.identity".to_owned(),
            CoverageCount {
                cases: 1,
                executed: 1,
                skipped: 2,
                harness_errors: 0,
            },
        )]),
        capability_matrix: BTreeMap::from([(
            "value.identity".to_owned(),
            CapabilityDisposition::Supported,
        )]),
        capability_counts: CapabilityCounts {
            supported: 1,
            ..CapabilityCounts::default()
        },
        final_status: FinalStatus::Passed,
    }
}

#[test]
fn machine_and_human_reports_include_required_sections() {
    let report = report(1, 10);
    let json = serde_json::to_value(&report).expect("report JSON");
    for field in [
        "corpus",
        "tools",
        "cases",
        "coverage",
        "capability_matrix",
        "capability_counts",
        "final_status",
    ] {
        assert!(json.get(field).is_some(), "missing report field {field}");
    }
    let human = report.render_human();
    assert!(human.contains("compatibility full"));
    assert!(human.contains("cases: 1"));
}

#[test]
fn baseline_diffs_ignore_timing_but_expose_every_observation_change() {
    let old = CompatibilityBaseline::from(&report(1, 10));
    let same_value = CompatibilityBaseline::from(&report(1, 999_999));
    assert_eq!(diff_baselines(Some(&old), &same_value).len(), 0);

    let candidate = CompatibilityBaseline::from(&report(2, 20));
    let differences = diff_baselines(Some(&old), &candidate);
    assert_eq!(differences.len(), 1);
    assert_eq!(differences[0].case_id, "common.identity");
    assert_eq!(differences[0].tool, "jq/json");
}

#[test]
fn baseline_candidate_requires_exact_explicit_case_reviews() {
    let old = CompatibilityBaseline::from(&report(1, 10));
    let changed = CompatibilityBaseline::from(&report(2, 20));
    let none = BTreeSet::new();
    assert!(accept_reviewed_candidate(Some(&old), changed.clone(), &none).is_err());

    let exact = BTreeSet::from(["common.identity".to_owned()]);
    assert!(accept_reviewed_candidate(Some(&old), changed.clone(), &exact).is_ok());

    let extra = BTreeSet::from(["common.identity".to_owned(), "unchanged.case".to_owned()]);
    assert!(accept_reviewed_candidate(Some(&old), changed, &extra).is_err());
}

#[test]
fn historical_coverage_summary_preserves_reviewed_jq_target_divergences() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/compatibility/reviews/coverage-summary.toon");
    let summary: HistoricalCoverageSummary =
        tq_test_support::fixture_data::read(&path).expect("valid compatibility report");

    assert_eq!(
        summary.source_artifact.path,
        "tests/compatibility/reviews/coverage-v1.toon"
    );
    assert_eq!(
        summary.source_artifact.sha256,
        "ec8a35b85bcc7e6709ac289a57b015045fbaea156278971e90f6f8c237050a18"
    );
    assert!(summary.historical_scope.contains("coverage-v1"));

    assert_eq!(summary.case_ids.len(), 200, "historical case count changed");
    let unique_case_ids = summary
        .case_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        unique_case_ids.len(),
        summary.case_ids.len(),
        "historical case IDs must be complete and unique"
    );

    let expected = BTreeMap::from([
        ("cli.sequence-framing".to_owned(), "raw stdout".to_owned()),
        (
            "numeric.policy-digits-over".to_owned(),
            "result sequence, exit code, error class".to_owned(),
        ),
        (
            "numeric.policy-exponent-over".to_owned(),
            "result sequence, exit code, error class".to_owned(),
        ),
        (
            "numeric.policy-index-over".to_owned(),
            "result sequence, exit code, error class".to_owned(),
        ),
        (
            "date.range-error".to_owned(),
            "result sequence, exit code, error class".to_owned(),
        ),
        (
            "regex.unsupported-lookaround".to_owned(),
            "result sequence, exit code, error class".to_owned(),
        ),
    ]);

    assert_eq!(summary.jq_target_diffs.len(), expected.len());
    let actual = summary
        .jq_target_diffs
        .iter()
        .map(|difference| (difference.case_id.clone(), difference.summary.clone()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(actual, expected, "unreviewed jq-target compatibility drift");

    let expected_capability_counts = CapabilityCounts {
        supported: 226,
        partial: 5,
        divergent: 10,
        unsupported: 2,
        deferred: 2,
        untested: 0,
    };
    assert_eq!(summary.capability_counts, expected_capability_counts);
    assert_eq!(summary.capability_counts.untested, 0);

    for (case_id, expected_class) in [
        (
            "regex.unsupported-lookaround",
            ErrorClass::UnsupportedCapability,
        ),
        ("date.range-error", ErrorClass::RuntimeRange),
        ("environment.denied", ErrorClass::RuntimePolicy),
        ("platform.denied", ErrorClass::RuntimePolicy),
    ] {
        assert!(
            unique_case_ids.contains(case_id),
            "missing historical case ID {case_id}"
        );
        let observation = summary
            .tq_error_classes
            .iter()
            .find(|observation| observation.case_id == case_id)
            .unwrap_or_else(|| panic!("missing tq evidence for {case_id}"));
        assert!(
            observation.error_class == expected_class,
            "wrong tq error class for {case_id}"
        );
    }

    assert_eq!(summary.tq_error_classes.len(), 4);
    let unique_error_case_ids = summary
        .tq_error_classes
        .iter()
        .map(|observation| observation.case_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(unique_error_case_ids.len(), summary.tq_error_classes.len());
}
