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
