//! Migration guard decisions, immutable inputs, and correctness evidence boundaries.

use std::fs;

use serde_json::{Value, json};
use tq_test_support::{
    benchmark::{
        CorrectnessObservation, CorrectnessPayload,
        migration_guard::{
            CONFIRMATION_PAIRS, Decision, INITIAL_PAIRS, RoundSummary, check_observation,
            check_run_report, decide_rounds, exit_code, freeze_inputs, matrix, needs_confirmation,
            summarize_round, verify_inputs,
        },
        semantic_digest,
    },
    compatibility::ProcessStatus,
};

fn round(baseline: &[u128], candidate: &[u128]) -> RoundSummary {
    summarize_round(baseline, candidate, baseline.len()).expect("valid repeated timings")
}

fn observation(
    value: &Value,
    status: ProcessStatus,
    exit_code: Option<i32>,
) -> CorrectnessObservation {
    CorrectnessObservation {
        payload: CorrectnessPayload::SemanticSequence(
            semantic_digest([value]).expect("JSON digest"),
        ),
        process_status: status,
        exit_code,
        error_class: None,
    }
}

#[test]
fn exact_ten_percent_with_zero_dispersion_passes_without_confirmation() {
    let initial = round(&[100, 100, 100], &[110, 110, 110]);

    assert_eq!(initial.ratio, 1.10);
    assert_eq!(initial.ratio_band, Some([1.10, 1.10]));
    assert!(initial.review_note.is_none());
    assert!(!initial.breach);
    assert!(!initial.noisy);
    assert!(!needs_confirmation(Some(&initial)));
    assert_eq!(decide_rounds(Some(&initial), None), Decision::Passed);
}

#[test]
fn exact_twenty_percent_passes_with_an_advisory_note() {
    let initial = round(&[100; 3], &[120; 3]);

    assert_eq!(initial.ratio, 1.20);
    assert_eq!(initial.ratio_band, Some([1.20, 1.20]));
    assert!(initial.review_note.is_some());
    assert!(!needs_confirmation(Some(&initial)));
    assert_eq!(decide_rounds(Some(&initial), None), Decision::Passed);
}

#[test]
fn advisory_only_slowdown_does_not_trigger_confirmation_or_failure() {
    let initial = round(&[1_000_000; 3], &[1_100_001; 3]);

    assert!(initial.review_note.is_some());
    assert!(!needs_confirmation(Some(&initial)));
    assert_eq!(decide_rounds(Some(&initial), None), Decision::Passed);
}

#[test]
fn advisory_boundary_dispersion_does_not_block_the_hard_limit() {
    let initial = round(&[100; 3], &[109, 110, 111]);

    assert_eq!(initial.ratio_band, Some([1.09, 1.11]));
    assert!(initial.review_note.is_none());
    assert!(!needs_confirmation(Some(&initial)));
    assert_eq!(decide_rounds(Some(&initial), None), Decision::Passed);
}

#[test]
fn stable_confirmed_slowdown_fails_per_workload() {
    let initial = round(&[100, 100, 100], &[121, 121, 121]);
    let confirmation = round(&[100; 7], &[121; 7]);

    assert!(needs_confirmation(Some(&initial)));
    assert_eq!(
        decide_rounds(Some(&initial), Some(&confirmation)),
        Decision::Failed
    );
}

#[test]
fn noisy_initial_breach_with_stable_above_threshold_confirmation_fails() {
    let initial = round(&[100; 3], &[120, 121, 123]);
    let confirmation = round(&[100; 7], &[121; 7]);

    assert!(initial.noisy && initial.breach);
    assert_eq!(
        decide_rounds(Some(&initial), Some(&confirmation)),
        Decision::Failed
    );
}

#[test]
fn nonpositive_initial_baseline_lower_bound_remains_inconclusive() {
    let mut initial = round(&[100; 3], &[121; 3]);
    initial.baseline_mad_micros = 100.0;
    initial.ratio_band = None;
    initial.noisy = true;
    let confirmation = round(&[100; 7], &[121; 7]);

    assert_eq!(
        decide_rounds(Some(&initial), Some(&confirmation)),
        Decision::Inconclusive
    );
}

#[test]
fn improvements_elsewhere_cannot_offset_a_failure() {
    assert_eq!(
        exit_code(&[Decision::Passed, Decision::Failed, Decision::Passed], 3),
        1,
    );
    assert_eq!(exit_code(&[Decision::Passed], 3), 2);
}

#[test]
fn threshold_straddling_mad_band_remains_inconclusive_after_confirmation() {
    let initial = round(&[100, 100, 100], &[119, 120, 121]);
    let confirmation = round(&[100; 7], &[120; 7]);

    assert_eq!(initial.ratio_band, Some([1.19, 1.21]));
    assert!(initial.noisy);
    assert_eq!(
        decide_rounds(Some(&initial), Some(&confirmation)),
        Decision::Inconclusive
    );
    assert_eq!(exit_code(&[Decision::Inconclusive], 1), 2);
}

#[test]
fn disagreement_between_round_threshold_classifications_is_inconclusive() {
    let initial = round(&[100, 100, 100], &[121, 121, 121]);
    let confirmation = round(&[100; 7], &[120; 7]);

    assert!(initial.breach);
    assert!(!confirmation.breach);
    assert_eq!(
        decide_rounds(Some(&initial), Some(&confirmation)),
        Decision::Inconclusive
    );
}

#[test]
fn missing_zero_and_forged_timing_summaries_never_pass() {
    let breach = round(&[100, 100, 100], &[121, 121, 121]);
    let mut forged = round(&[100, 100, 100], &[100, 100, 100]);
    forged.ratio = f64::NAN;

    assert!(summarize_round(&[0, 100, 100], &[100, 100, 100], 3).is_none());
    assert!(summarize_round(&[100, 100], &[100, 100, 100], 3).is_none());
    assert!(summarize_round(&[1_u128 << 54; 3], &[100; 3], 3).is_none());
    assert_eq!(decide_rounds(None, None), Decision::Inconclusive);
    assert_eq!(decide_rounds(Some(&breach), None), Decision::Incomplete);
    assert!(needs_confirmation(Some(&forged)));
    assert_eq!(decide_rounds(Some(&forged), None), Decision::Inconclusive);
    assert_eq!(exit_code(&[Decision::Incomplete], 1), 2);
}

#[test]
fn rounds_must_be_finite_and_consistent_with_public_summary_fields() {
    let mut inconsistent = round(&[100, 100, 100], &[121, 121, 121]);
    inconsistent.ratio = 1.0;
    let mut non_finite_band = round(&[100, 100, 100], &[100, 100, 100]);
    non_finite_band.ratio_band = Some([f64::NAN, 1.0]);
    let mut zero_time = round(&[100, 100, 100], &[100, 100, 100]);
    zero_time.baseline_median_micros = 0.0;
    let passing = round(&[100, 100, 100], &[100, 100, 100]);

    assert_eq!(
        decide_rounds(Some(&inconsistent), None),
        Decision::Inconclusive
    );
    assert_eq!(
        decide_rounds(Some(&non_finite_band), None),
        Decision::Inconclusive
    );
    assert_eq!(
        decide_rounds(Some(&zero_time), None),
        Decision::Inconclusive
    );
    assert_eq!(
        decide_rounds(Some(&passing), Some(&zero_time)),
        Decision::Inconclusive
    );
}

#[test]
fn pair_order_alternates_for_initial_and_confirmation_rounds() {
    assert_eq!(INITIAL_PAIRS, 3);
    assert_eq!(CONFIRMATION_PAIRS, 7);
    assert_eq!(
        (0..INITIAL_PAIRS + CONFIRMATION_PAIRS)
            .map(tq_test_support::benchmark::migration_guard::baseline_first)
            .collect::<Vec<_>>(),
        vec![
            true, false, true, false, true, false, true, false, true, false
        ],
    );
}

#[test]
fn semantic_gate_rejects_changed_values_and_process_contracts() {
    let expected = json!({"id": 1, "nested": ["same", true]});
    let wrong_value = json!({"id": 2, "nested": ["same", true]});

    assert!(
        check_observation(
            &expected,
            &observation(&expected, ProcessStatus::Exited, Some(0))
        )
        .is_ok()
    );
    assert!(
        check_observation(
            &expected,
            &observation(&wrong_value, ProcessStatus::Exited, Some(0))
        )
        .is_err()
    );
    assert!(
        check_observation(
            &expected,
            &observation(&expected, ProcessStatus::TimedOut, Some(0))
        )
        .is_err()
    );
    assert!(
        check_observation(
            &expected,
            &observation(&expected, ProcessStatus::Exited, Some(1))
        )
        .is_err()
    );
}

#[test]
fn document_and_spool_rows_require_their_actual_report_evidence() {
    let workloads = matrix();
    let document = workloads
        .iter()
        .find(|workload| workload.id == "document-flat")
        .expect("document row");
    let spool = workloads
        .iter()
        .find(|workload| workload.id == "spool-flat")
        .expect("spool row");

    assert!(
        check_run_report(
            document,
            &json!({"execution": {"plan": "blocking-document"}})
        )
        .is_ok()
    );
    assert!(check_run_report(document, &json!({"execution": {"plan": "transcode"}})).is_err());
    assert!(check_run_report(spool, &json!({"execution": {
        "plan": "transcode", "preparation_high_water_bytes": 262_128, "array_preparations": 1,
        "spool_bytes_written": 9_496_912, "spool_bytes_replayed": 9_496_912
    }})).is_ok());
    assert!(check_run_report(spool, &json!({"execution": {
        "plan": "transcode", "preparation_high_water_bytes": 262_145, "array_preparations": 1,
        "spool_bytes_written": 9_496_912, "spool_bytes_replayed": 9_496_912
    }})).is_err());
    assert!(check_run_report(spool, &json!({"execution": {
        "plan": "transcode", "preparation_high_water_bytes": 262_128, "array_preparations": 1,
        "spool_bytes_written": 262_144, "spool_bytes_replayed": 9_496_912
    }})).is_err());
    assert!(check_run_report(spool, &json!({"execution": {
        "plan": "transcode", "preparation_high_water_bytes": 262_128, "array_preparations": 1,
        "spool_bytes_written": 9_496_912, "spool_bytes_replayed": 0
    }})).is_err());
    assert!(check_run_report(spool, &json!({"execution": {
        "plan": "transcode", "preparation_high_water_bytes": 262_128, "array_preparations": 0,
        "spool_bytes_written": 9_496_912, "spool_bytes_replayed": 9_496_912
    }})).is_err());
}

#[test]
fn immutable_input_verification_rejects_changes_without_rewriting_them() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let manifest = freeze_inputs(directory.path()).expect("freeze deterministic matrix");
    verify_inputs(directory.path(), &manifest).expect("freshly frozen matrix verifies");

    let path = directory.path().join("flat.json");
    let mut changed = fs::read(&path).expect("read frozen flat input");
    changed[0] ^= 1;
    fs::write(&path, &changed).expect("change frozen input");

    assert!(verify_inputs(directory.path(), &manifest).is_err());
    assert_eq!(fs::read(&path).expect("read changed frozen input"), changed);
    assert!(freeze_inputs(directory.path()).is_err());
    assert_eq!(
        fs::read(&path).expect("changed bytes remain untouched"),
        changed
    );
}
