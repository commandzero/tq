//! Benchmark report diagnostic compatibility tests.

use std::{fs, path::Path};

use serde_json::json;

#[test]
fn campaign_schema_accepts_optional_bounded_row_diagnostics() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let schema: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("schemas/benchmark-campaign-v1.schema.json")).expect("campaign schema"),
    )
    .expect("campaign schema JSON");
    let validator = jsonschema::Validator::new(&schema).expect("campaign validator");

    let mut report = report_with_cases(&json!([]));
    assert!(validator.is_valid(&report));

    report["cases"] = json!([{
        "case_id": "benchmark.example",
        "outcome": "incorrect",
        "diagnostic": "malformed TOON values: record does not end with LF"
    }]);
    assert!(validator.is_valid(&report));

    report["cases"][0]["diagnostic"] = json!("x".repeat(1025));
    assert!(!validator.is_valid(&report));
}

#[test]
fn campaign_schema_accepts_explicit_rss_provenance_and_rejects_unknown_sources() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let schema: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("schemas/benchmark-campaign-v1.schema.json")).expect("campaign schema"),
    )
    .expect("campaign schema JSON");
    let validator = jsonschema::Validator::new(&schema).expect("campaign validator");
    let mut report = report_with_cases(&json!([{
        "samples": [{
            "peak_rss_bytes": 4096,
            "rss_provenance": "gnu-time-v"
        }]
    }]));
    assert!(validator.is_valid(&report));

    report["cases"][0]["samples"][0]["rss_provenance"] = json!("linux-host");
    assert!(!validator.is_valid(&report));
}

#[test]
fn old_rows_without_diagnostics_still_deserialize() {
    let row = json!({
        "case_id": "benchmark.example",
        "adapter_id": "tq-json",
        "source_id": "source",
        "tier": "startup",
        "input_format": "json",
        "execution_class": "startup",
        "comparison_families": ["same-format"],
        "command": ["tq", "."],
        "outcome": "timed",
        "warmups": 1,
        "requested_samples": 1,
        "timeout_seconds": 1,
        "limits": {"output_bytes": 1024, "rss_bytes": null},
        "samples": [],
        "summary": null,
        "reference_ratios": {},
        "reference_peak_rss_ratios": {},
        "soft_performance_objective": null
    });
    let row: tq_test_support::benchmark::BenchmarkRow =
        serde_json::from_value(row).expect("legacy benchmark row");
    assert_eq!(row.diagnostic, None);
}

#[test]
fn native_samples_require_protocol_positive_rss_and_cpu() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let schema = serde_json::from_slice::<serde_json::Value>(
        &fs::read(root.join("schemas/benchmark-campaign-v1.schema.json")).unwrap(),
    )
    .unwrap();
    let validator = jsonschema::Validator::new(&schema).unwrap();
    for source in ["darwin-wait4", "linux-wait4"] {
        let mut report = report_with_cases(&json!([{"samples": [{
            "rss_provenance": source, "peak_rss_bytes": 4096,
            "user_cpu_micros": 0, "system_cpu_micros": 12,
            "measurement_protocol": {
                "timing_method": "direct-spawn-to-exit-observation",
                "input_delivery": "prepared-file",
                "rss_scope": "wait4-exact-child",
                "exit_poll_interval_micros": 100,
                "rss_poll_interval_micros": null,
                "validated_accuracy_micros": null
            }
        }]}]));
        assert!(validator.is_valid(&report));
        let sample = report["cases"][0]["samples"][0].clone();
        for field in [
            "measurement_protocol",
            "peak_rss_bytes",
            "user_cpu_micros",
            "system_cpu_micros",
        ] {
            report["cases"][0]["samples"][0] = sample.clone();
            report["cases"][0]["samples"][0]
                .as_object_mut()
                .unwrap()
                .remove(field);
            assert!(!validator.is_valid(&report), "missing {field}");
        }
        report["cases"][0]["samples"][0] = sample;
        report["cases"][0]["samples"][0]["peak_rss_bytes"] = json!(0);
        assert!(!validator.is_valid(&report));
    }
}

#[test]
fn regression_gate_persists_disclosures_and_unavailable_reasons() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let schema: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("schemas/benchmark-campaign-v1.schema.json")).expect("campaign schema"),
    )
    .expect("campaign schema JSON");
    let validator = jsonschema::Validator::new(&schema).expect("campaign validator");
    let mut report = report_with_cases(&json!([]));
    report["regression_gate"] = json!({
        "evaluated": true,
        "thresholds": {"wall_time_percent": 50.0, "peak_rss_percent": 50.0, "minimum_samples": 3},
        "failures": [],
        "disclosures": ["benchmark.example/tq-json wall time increased (baseline=1000, candidate=1201, baseline_samples=3, candidate_samples=3, baseline_dispersion=..., candidate_dispersion=...)"],
        "unavailable": ["benchmark.other has no baseline row"]
    });
    assert!(validator.is_valid(&report));

    report["regression_gate"]["disclosures"] = json!([42]);
    assert!(!validator.is_valid(&report));
}

fn report_with_cases(cases: &serde_json::Value) -> serde_json::Value {
    json!({
        "schema_version": 1,
        "campaign_id": "local",
        "profile": "rapid",
        "environment": {
            "collected_at": "2026-09-10T00:00:00Z",
            "os": "test", "kernel": "test", "architecture": "test",
            "logical_cpus": 1, "physical_cpus": 1, "cpu_model": "test",
            "memory_bytes": 1024, "filesystem": "test",
            "power_settings": "test", "compiler_profile": "debug"
        },
        "corpus": [],
        "tools": [],
        "cases": cases,
        "comparability": {},
        "regression_gate": {},
        "final_status": "observed-failures"
    })
}
