//! Synthetic report tests; no recorded campaign is a test dependency.

use super::*;
use tq_test_support::benchmark::{RESULTS_END_MARKER, RESULTS_START_MARKER};

fn sample() -> MeasuredOutcome {
    serde_json::from_value(serde_json::json!({
        "status":"exited", "exit_code":0, "signal":null, "wall_time_micros":1000,
        "first_result_micros":100, "user_cpu_micros":500, "system_cpu_micros":100,
        "peak_rss_bytes":1_048_576, "rss_provenance":"darwin-wait4",
        "measurement_protocol": {
            "timing_method":"synthetic", "input_delivery":"synthetic", "rss_scope":"synthetic",
            "exit_poll_interval_micros":100, "rss_poll_interval_micros":null,
            "validated_accuracy_micros":null,
            "worker":{"executable_sha256":"a".repeat(64), "launch_protocol":"synthetic", "collector_source_sha256":"b".repeat(64)}
        },
        "process_group_peak_rss_bytes":null, "output_bytes":12, "stderr_bytes":0,
        "stdout_path":null, "stderr_path":null, "process_group_rss_observed":false
    })).unwrap()
}

fn report() -> ColorReport {
    ColorReport {
        source: "synthetic.json".into(),
        source_bytes: 100,
        source_sha256: None,
        binary: "synthetic-tq".into(),
        binary_sha256: None,
        environment: None,
        preflight: "darwin-wait4".into(),
        warmups: 2,
        measured_samples: 2,
        rows: CASES
            .iter()
            .flat_map(|(case, _, _)| {
                ["-M", "-C"].map(|flag| ColorRow {
                    case: (*case).into(),
                    flag: flag.into(),
                    stripped_correctness: "byte-identical".into(),
                    color_present: Some(flag == "-C"),
                    samples: vec![sample(), sample()],
                })
            })
            .collect(),
    }
}

#[test]
fn regeneration_is_idempotent_and_updates_metrics_without_touching_authored_text() {
    let dir = tempfile::tempdir().unwrap();
    let page = dir.path().join("colors.md");
    let authored = format!(
        "# Color test\n\nKeep introduction.\n\n## Results\n{RESULTS_START_MARKER}\nstale\n{RESULTS_END_MARKER}\n\nKeep method.\n"
    );
    fs::write(&page, &authored).unwrap();
    let mut reports = vec![report(), report()];
    render_reports(&page, &reports).unwrap();
    let first = fs::read_to_string(&page).unwrap();
    assert!(first.starts_with("# Color test\n\nKeep introduction."));
    assert!(first.ends_with("Keep method.\n"));
    assert!(!first.contains("stale"));
    assert!(!first.contains("synthetic-tq"));
    render_reports(&page, &reports).unwrap();
    assert_eq!(fs::read_to_string(&page).unwrap(), first);
    for sample in &mut reports[0].rows[1].samples {
        sample.wall_time_micros = 3000;
    }
    render_reports(&page, &reports).unwrap();
    let updated = fs::read_to_string(&page).unwrap();
    assert!(updated.contains("| document-json-identity | 1.0 | 3.0 | 3.00× |"));
}

#[test]
fn invalid_samples_or_markers_do_not_modify_the_page() {
    let dir = tempfile::tempdir().unwrap();
    let page = dir.path().join("colors.md");
    fs::write(&page, "authored text without markers").unwrap();
    assert!(render_reports(&page, &[report(), report()]).is_err());
    let authored = format!(
        "# Test\n\n## Results\n{RESULTS_START_MARKER}\nkeep previous results\n{RESULTS_END_MARKER}\n"
    );
    fs::write(&page, &authored).unwrap();
    let good = report();
    for mutation in 0..5 {
        let mut bad = report();
        match mutation {
            0 => {
                bad.rows.pop();
            }
            1 => bad.rows[0].samples[0].exit_code = Some(1),
            2 => bad.rows[0].samples[0].peak_rss_bytes = None,
            3 => bad.rows[0].stripped_correctness = "incorrect".into(),
            _ => bad.rows[0].samples[0].output_bytes = 99,
        }
        assert!(validate(&bad).is_err());
        assert!(render_reports(&page, &[report(), bad]).is_err());
        assert_eq!(fs::read_to_string(&page).unwrap(), authored);
    }
    assert!(render_reports(&page, &[good]).is_err());
    assert_eq!(fs::read_to_string(&page).unwrap(), authored);
}

#[test]
fn strips_only_complete_sgr_and_preserves_frame_bytes() {
    assert_eq!(
        strip_sgr(b"\x1e\x1b[0;32mhello\x1b[0m\n").unwrap(),
        b"\x1ehello\n"
    );
    for bad in [b"\x1b[31".as_slice(), b"\x1b[2J", b"\x1bX"] {
        assert!(strip_sgr(bad).is_err());
    }
}

#[test]
fn missing_legacy_metadata_is_not_filled_from_the_current_host() {
    let text = render(&[report(), report()]).unwrap();
    assert!(text.contains("not recorded in this legacy raw report"));
    assert!(!text.contains("Recorded:"));
}

#[test]
fn render_only_rejects_missing_or_false_color_evidence_without_changing_page() {
    let dir = tempfile::tempdir().unwrap();
    let page = dir.path().join("colors.md");
    let authored = format!(
        "# Test\n\n## Results\n{RESULTS_START_MARKER}\nkeep results\n{RESULTS_END_MARKER}\n"
    );
    fs::write(&page, &authored).unwrap();
    for evidence in [None, Some(false)] {
        let mut raw = serde_json::to_value(report()).unwrap();
        let row = raw["rows"][1].as_object_mut().unwrap();
        row.remove("color_present");
        if let Some(present) = evidence {
            row.insert("color_present".into(), present.into());
        }
        let bad: ColorReport = serde_json::from_value(raw).unwrap();
        assert!(render_reports(&page, &[report(), bad]).is_err());
        assert_eq!(fs::read_to_string(&page).unwrap(), authored);
    }
}
