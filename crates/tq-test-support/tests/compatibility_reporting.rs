//! Compatibility report and reviewed-baseline tests.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::json;
use tq_test_support::{
    compatibility::{
        CompatibilityBaseline, CompatibilityReport, CoverageCount, FinalStatus, ObservationState,
        ProcessStatus, ToolKind, ToolObservation, accept_reviewed_candidate, diff_baselines,
    },
    corpus::ArtifactIdentity,
};

fn report(value: i64, duration: u128) -> CompatibilityReport {
    CompatibilityReport {
        schema_version: 1,
        profile: "full".to_owned(),
        corpus: ArtifactIdentity {
            path: "compatibility/cases".to_owned(),
            bytes: 42,
            sha256: "a".repeat(64),
        },
        tools: Vec::new(),
        cases: vec![tq_test_support::compatibility::CaseReport {
            id: "common.identity".to_owned(),
            capabilities: vec!["value.identity".to_owned()],
            observations: vec![ToolObservation {
                tool: ToolKind::Jq,
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
        final_status: FinalStatus::Passed,
    }
}

#[test]
fn machine_and_human_reports_include_required_sections() {
    let report = report(1, 10);
    let json = serde_json::to_value(&report).expect("report JSON");
    for field in ["corpus", "tools", "cases", "coverage", "final_status"] {
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
    assert!(diff_baselines(Some(&old), &same_value).is_empty());

    let candidate = CompatibilityBaseline::from(&report(2, 20));
    let differences = diff_baselines(Some(&old), &candidate);
    assert_eq!(differences.len(), 1);
    assert_eq!(differences[0].case_id, "common.identity");
    assert_eq!(differences[0].tool, "jq");
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
