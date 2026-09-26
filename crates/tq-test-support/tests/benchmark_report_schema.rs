//! Benchmark report diagnostic compatibility tests.

use std::{fs, path::Path};

use serde_json::json;
use tq_test_support::benchmark::BenchmarkRow;

#[test]
fn suite_identity_is_required_and_independent_of_run_profile() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let schema: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("schemas/benchmark-campaign-v1.schema.json")).expect("campaign schema"),
    )
    .expect("schema JSON");
    let validator = jsonschema::Validator::new(&schema).expect("campaign validator");
    let mut report = report_with_cases(&json!([]));
    report["suite"] = json!("large-input");
    report["profile"] = json!("extended");
    assert!(validator.is_valid(&report));
    report
        .as_object_mut()
        .expect("report object")
        .remove("suite");
    assert!(!validator.is_valid(&report));
    report["suite"] = json!("stack-overflow");
    report["profile"] = json!("standard");
    assert!(validator.is_valid(&report));
    report["profile"] = json!("stack-overflow");
    assert!(!validator.is_valid(&report));
}

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

    report["cases"] = json!([complete_row(&json!({
        "case_id": "benchmark.example",
        "outcome": "incorrect",
        "diagnostic": "malformed TOON values: record does not end with LF"
    }))]);
    assert!(validator.is_valid(&report));

    report["cases"][0]["diagnostic"] = json!("x".repeat(1025));
    assert!(!validator.is_valid(&report));

    let mut unknown = report_with_cases(&json!([{}]));
    unknown["cases"][0]["unexpected"] = json!(true);
    assert!(!validator.is_valid(&unknown));

    let mut empty = report_with_cases(&json!([]));
    empty["cases"] = json!([{}]);
    assert!(!validator.is_valid(&empty));
}

#[test]
fn incomplete_checkpoint_cannot_claim_a_passing_campaign() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let schema: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("schemas/benchmark-campaign-v1.schema.json")).expect("schema"),
    )
    .expect("JSON");
    let validator = jsonschema::Validator::new(&schema).expect("validator");
    let mut report = report_with_cases(&json!([]));
    report["execution"] = json!({
        "mode": "fast", "sampling": "screen", "instrument_rss": false,
        "campaign_budget_seconds": 900, "case_budget_seconds": 300,
        "planned_rows": 6, "elapsed_seconds": 3.5, "complete": false,
        "interruptions": ["campaign cancelled"]
    });
    report["final_status"] = json!("incomplete");
    assert!(validator.is_valid(&report));
    report["final_status"] = json!("passed");
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
        "samples": [complete_sample(&json!({
            "peak_rss_bytes": 4096,
            "rss_provenance": "gnu-time-v"
        }))]
    }]));
    assert!(validator.is_valid(&report));

    report["cases"][0]["samples"][0]["rss_provenance"] = json!("linux-host");
    assert!(!validator.is_valid(&report));
}

#[test]
fn campaign_schema_requires_complete_samples_and_rejects_unknown_fields() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let schema: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("schemas/benchmark-campaign-v1.schema.json")).expect("campaign schema"),
    )
    .expect("campaign schema JSON");
    let validator = jsonschema::Validator::new(&schema).expect("campaign validator");
    let mut report = report_with_cases(&json!([{
        "samples": [complete_sample(&json!({}))]
    }]));
    assert!(validator.is_valid(&report));

    let mut empty = report.clone();
    empty["cases"][0]["samples"][0] = json!({});
    assert!(!validator.is_valid(&empty));

    for field in ["wall_time_micros", "output_bytes"] {
        let mut missing = report.clone();
        missing["cases"][0]["samples"][0]
            .as_object_mut()
            .expect("sample object")
            .remove(field);
        assert!(!validator.is_valid(&missing), "missing {field}");
    }

    report["cases"][0]["samples"][0]["wall_time_microseconds"] = json!(1);
    assert!(!validator.is_valid(&report));

    report["cases"][0]["samples"][0]
        .as_object_mut()
        .expect("sample object")
        .remove("wall_time_microseconds");
    report["cases"][0]["samples"][0]["unexpected"] = json!(true);
    assert!(!validator.is_valid(&report));
}

#[test]
fn campaign_schema_preserves_legacy_sample_optionality() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let schema: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("schemas/benchmark-campaign-v1.schema.json")).expect("campaign schema"),
    )
    .expect("campaign schema JSON");
    let validator = jsonschema::Validator::new(&schema).expect("campaign validator");
    let mut sample = complete_sample(&json!({}));
    for field in [
        "measurement_protocol",
        "user_cpu_micros",
        "system_cpu_micros",
        "peak_rss_bytes",
        "rss_provenance",
        "process_group_peak_rss_bytes",
        "first_result_micros",
    ] {
        sample.as_object_mut().expect("sample object").remove(field);
    }
    let report = report_with_cases(&json!([{"samples": [sample]}]));
    assert!(validator.is_valid(&report));
}

#[test]
fn campaign_schema_keeps_instrumented_fields_optional_but_typed_when_present() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let schema: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("schemas/benchmark-campaign-v1.schema.json")).expect("campaign schema"),
    )
    .expect("campaign schema JSON");
    let validator = jsonschema::Validator::new(&schema).expect("campaign validator");

    let mut legacy = report_with_cases(&json!([{}]));
    for field in ["instrumented_samples", "reference_peak_rss_ratios"] {
        legacy["cases"][0]
            .as_object_mut()
            .expect("case object")
            .remove(field);
    }
    assert!(validator.is_valid(&legacy));

    let mut invalid_samples = report_with_cases(&json!([{}]));
    invalid_samples["cases"][0]["instrumented_samples"] = json!([{
        "wall_time_micros": 1,
        "output_bytes": "not-a-byte-count"
    }]);
    assert!(!validator.is_valid(&invalid_samples));

    let mut invalid_ratios = report_with_cases(&json!([{}]));
    invalid_ratios["cases"][0]["reference_peak_rss_ratios"] = json!({"jq": -1});
    assert!(!validator.is_valid(&invalid_ratios));

    let serialized = serde_json::to_value(
        serde_json::from_value::<BenchmarkRow>(complete_row(&json!({})))
            .expect("complete row should deserialize"),
    )
    .expect("benchmark row serialization");
    assert!(serialized["instrumented_samples"].is_array());
    assert!(serialized["reference_peak_rss_ratios"].is_object());
}

#[test]
fn campaign_schema_accepts_stack_overflow_output_report_and_rejects_invalid_capture_fields() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let schema: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("schemas/benchmark-campaign-v1.schema.json")).expect("campaign schema"),
    )
    .expect("campaign schema JSON");
    let validator = jsonschema::Validator::new(&schema).expect("campaign validator");
    let mut report = report_with_cases(&json!([]));
    report["suite"] = json!("stack-overflow");
    report["output_comparison"] = json!({
        "captured_at": "2026-09-11T00:00:00Z",
        "tools": [],
        "cases": {
            "stack-overflow.01": {
                "query": ".",
                "input_sha256": "abc123",
                "compact": {
                    "command": ["jq", "-c", "."],
                    "status": "exited",
                    "exit_code": 0,
                    "stdout": [91, 93, 10],
                    "stderr": []
                },
                "expanded": {
                    "command": ["jq", "."],
                    "status": "timed-out",
                    "exit_code": null,
                    "stdout": [],
                    "stderr": [116]
                },
                "toon": {
                    "command": ["tq", "."],
                    "status": null,
                    "exit_code": null,
                    "stdout": [91, 93, 10],
                    "stderr": []
                }
            }
        }
    });
    assert!(validator.is_valid(&report));

    report["output_comparison"]["cases"]["stack-overflow.01"]["compact"]["output_limit"] =
        json!({"stream": "stdout", "limit": 32, "observed_bytes": 33});
    assert!(validator.is_valid(&report));
    report["output_comparison"]["cases"]["stack-overflow.01"]["compact"]["output_limit"]["stream"] =
        json!("other");
    assert!(!validator.is_valid(&report));
    report["output_comparison"]["cases"]["stack-overflow.01"]["compact"]["output_limit"]["stream"] =
        json!("stdout");
    assert!(validator.is_valid(&report));

    report["output_comparison"]["cases"]["stack-overflow.01"]["toon"]["stdout"][0] = json!(256);
    assert!(!validator.is_valid(&report));

    report["output_comparison"]["cases"]["stack-overflow.01"]["toon"]["stdout"][0] = json!(91);
    report["output_comparison"]["cases"]["stack-overflow.01"]["compact"]["status"] =
        json!("failed");
    assert!(!validator.is_valid(&report));
}

#[test]
fn campaign_schema_accepts_profiled_color_captures_and_requires_provenance() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let schema: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("schemas/benchmark-campaign-v1.schema.json")).expect("campaign schema"),
    )
    .expect("campaign schema JSON");
    let validator = jsonschema::Validator::new(&schema).expect("campaign validator");
    let capture = |tool: &str| {
        json!({
            "command": [tool, "-C", "."],
            "status": "exited",
            "exit_code": 0,
            "stdout": [27, 91, 51, 49, 109, 49, 27, 91, 48, 109, 10],
            "stderr": []
        })
    };
    let mut report = report_with_cases(&json!([]));
    report["suite"] = json!("stack-overflow");
    report["output_comparison"] = json!({
        "captured_at": "2026-09-11T00:00:00Z",
        "tools": [],
        "cases": {
            "stack-overflow.01": {
                "query": ".",
                "input_sha256": "abc123",
                "output_mode": "color",
                "provenance": {
                    "fixture": "tests/stack-overflow/01.toon",
                    "input_bytes": 2,
                    "output_mode": "color",
                    "environment": {},
                    "contract_sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                },
                "scenario_captures": {
                    "jq": capture("jq"),
                    "yq": capture("yq"),
                    "tq": capture("tq")
                }
            }
        }
    });
    assert!(validator.is_valid(&report));
    let mut missing_provenance = report.clone();
    missing_provenance["output_comparison"]["cases"]["stack-overflow.01"]["provenance"] =
        serde_json::Value::Null;
    assert!(!validator.is_valid(&missing_provenance));
    let mut missing_tool = report.clone();
    missing_tool["output_comparison"]["cases"]["stack-overflow.01"]["scenario_captures"]
        .as_object_mut()
        .expect("scenario captures")
        .remove("yq");
    assert!(!validator.is_valid(&missing_tool));
    let mut unknown_tool = report;
    unknown_tool["output_comparison"]["cases"]["stack-overflow.01"]["scenario_captures"]
        .as_object_mut()
        .expect("scenario captures")
        .insert("other".to_owned(), capture("other"));
    assert!(!validator.is_valid(&unknown_tool));
}

#[test]
fn old_rows_without_diagnostics_still_deserialize() {
    let mut row = json!({
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
        "soft_performance_objective": null
    });
    row.as_object_mut()
        .expect("legacy row object")
        .remove("instrumented_samples");
    row.as_object_mut()
        .expect("legacy row object")
        .remove("reference_peak_rss_ratios");
    let row: tq_test_support::benchmark::BenchmarkRow =
        serde_json::from_value(row).expect("legacy benchmark row");
    assert_eq!(row.diagnostic, None);
    assert!(row.instrumented_samples.is_empty());
    assert!(row.reference_peak_rss_ratios.is_empty());
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
            "wall_time_micros": 1000, "rss_provenance": source, "peak_rss_bytes": 4096,
            "user_cpu_micros": 0, "system_cpu_micros": 12,
            "first_result_micros": null, "output_bytes": 0,
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
fn native_protocol_schema_records_worker_and_isolation_identity() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let schema = serde_json::from_slice::<serde_json::Value>(
        &fs::read(root.join("schemas/benchmark-campaign-v1.schema.json")).unwrap(),
    )
    .unwrap();
    let validator = jsonschema::Validator::new(&schema).unwrap();
    let mut report = report_with_cases(&json!([{"samples": [{
        "wall_time_micros": 1000, "rss_provenance": "linux-wait4", "peak_rss_bytes": 4096,
        "user_cpu_micros": 0, "system_cpu_micros": 12,
        "first_result_micros": null, "output_bytes": 0,
        "measurement_protocol": {
            "timing_method": "direct-spawn-to-exit-observation",
            "input_delivery": "prepared-file",
            "rss_scope": "wait4-child-lifetime-including-pre-exec-and-waited-descendants",
            "exit_poll_interval_micros": 100,
            "rss_poll_interval_micros": null,
            "validated_accuracy_micros": 1000,
            "worker": {
                "executable_sha256": "worker",
                "launch_protocol": "direct-target-v1",
                "collector_source_sha256": "collector"
            },
            "isolation_evidence": {
                "summary_sha256": "summary",
                "control_peak_rss_bytes": 4096,
                "max_parent_delta_bytes": 0,
                "tolerance_bytes": 4096
            }
        }
    }]}]));
    assert!(validator.is_valid(&report));

    report["cases"][0]["samples"][0]["measurement_protocol"]["worker"]
        .as_object_mut()
        .unwrap()
        .remove("launch_protocol");
    assert!(!validator.is_valid(&report));

    let historical: tq_test_support::benchmark::MeasurementProtocol =
        serde_json::from_value(json!({
            "timing_method": "historical",
            "input_delivery": "pipe",
            "rss_scope": "unknown",
            "exit_poll_interval_micros": 1,
            "rss_poll_interval_micros": null,
            "validated_accuracy_micros": null
        }))
        .expect("historical protocol remains readable");
    assert!(historical.worker.is_none());
    assert!(historical.isolation_evidence.is_none());
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
    let cases = serde_json::Value::Array(
        cases
            .as_array()
            .expect("case array")
            .iter()
            .map(complete_row)
            .collect(),
    );
    json!({
        "schema_version": 1,
        "campaign_id": "local",
        "suite": "natural-corpus",
        "profile": "standard",
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

fn complete_row(overrides: &serde_json::Value) -> serde_json::Value {
    let mut row = json!({
        "case_id": "benchmark.example",
        "adapter_id": "jq-json",
        "source_id": "source",
        "tier": "startup",
        "input_format": "json",
        "execution_class": "startup",
        "comparison_families": ["same-format"],
        "command": ["jq", "."],
        "outcome": "timed",
        "warmups": 1,
        "requested_samples": 1,
        "timeout_seconds": 1,
        "limits": {"output_bytes": 1024, "rss_bytes": null},
        "samples": [],
        "instrumented_samples": [],
        "summary": null,
        "reference_ratios": {},
        "reference_peak_rss_ratios": {}
    });
    for (key, value) in overrides.as_object().expect("row overrides") {
        row[key] = value.clone();
    }
    row
}

fn complete_sample(overrides: &serde_json::Value) -> serde_json::Value {
    let mut sample = json!({
        "measurement_protocol": null,
        "wall_time_micros": 1000,
        "user_cpu_micros": null,
        "system_cpu_micros": null,
        "peak_rss_bytes": null,
        "rss_provenance": null,
        "process_group_peak_rss_bytes": null,
        "first_result_micros": null,
        "output_bytes": 0
    });
    for (key, value) in overrides.as_object().expect("sample overrides") {
        sample[key] = value.clone();
    }
    sample
}
