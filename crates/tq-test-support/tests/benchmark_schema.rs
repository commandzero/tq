//! Benchmark manifest schema and matrix coverage tests.

use std::{collections::BTreeSet, fs, path::Path};

use serde_json::{Value, json};

#[test]
fn benchmark_schemas_are_valid_and_campaign_shape_is_versioned() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let case_schema: Value = serde_json::from_slice(
        &fs::read(root.join("schemas/benchmark-case-v1.schema.json")).expect("case schema"),
    )
    .expect("case schema JSON");
    jsonschema::Validator::new(&case_schema).expect("valid case schema");

    let campaign_schema: Value = serde_json::from_slice(
        &fs::read(root.join("schemas/benchmark-campaign-v1.schema.json")).expect("campaign schema"),
    )
    .expect("campaign schema JSON");
    assert!(
        campaign_schema["properties"]["profile"]["enum"]
            .as_array()
            .expect("profile enum")
            .iter()
            .any(|profile| profile == "rapid")
    );
    let validator = jsonschema::Validator::new(&campaign_schema).expect("valid campaign schema");
    assert!(validator.is_valid(&json!({
        "schema_version": 1,
        "campaign_id": "local-2026-07-31",
        "profile": "standard",
        "environment": {
            "collected_at": "2026-07-31T00:00:00Z",
            "os": "macos",
            "kernel": "Darwin",
            "architecture": "aarch64",
            "logical_cpus": 10,
            "physical_cpus": 10,
            "cpu_model": "Apple",
            "memory_bytes": 1,
            "filesystem": "apfs",
            "power_settings": null,
            "compiler_profile": "release"
        },
        "corpus": [],
        "tools": [],
        "cases": [],
        "comparability": {},
        "regression_gate": {},
        "final_status": "passed"
    })));
}

#[test]
fn workload_catalog_is_schema_valid_gated_and_has_the_full_adapter_matrix() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let schema: Value = serde_json::from_slice(
        &fs::read(root.join("schemas/benchmark-case-v1.schema.json")).expect("case schema"),
    )
    .expect("case schema JSON");
    let validator = jsonschema::Validator::new(&schema).expect("case validator");
    let compatibility_ids = compatibility_ids(&root);
    let cases =
        fs::read_to_string(root.join("benchmarks/cases/workloads.jsonl")).expect("benchmark cases");
    let mut ids = BTreeSet::new();
    for (index, line) in cases.lines().enumerate() {
        let case: Value = serde_json::from_str(line).expect("benchmark case JSON");
        assert!(
            validator.is_valid(&case),
            "invalid case on line {}",
            index + 1
        );
        assert!(
            ids.insert(case["id"].as_str().expect("case ID").to_owned()),
            "duplicate benchmark ID"
        );
        assert!(
            compatibility_ids.contains(case["compatibility_gate"].as_str().expect("gate")),
            "unknown compatibility gate on line {}",
            index + 1
        );
        let adapters = case["adapters"].as_array().expect("adapter matrix");
        let adapter_ids = adapters
            .iter()
            .map(|adapter| adapter["id"].as_str().expect("adapter ID"))
            .collect::<BTreeSet<_>>();
        assert_eq!(
            adapter_ids,
            BTreeSet::from([
                "jq-json", "yq-json", "yq-yaml", "tq-json", "tq-yaml", "tq-toon"
            ])
        );
        for adapter in adapters.iter().filter(|adapter| adapter["tool"] == "tq") {
            let format = adapter["input_format"].as_str().expect("input format");
            let args = adapter["args"].as_array().expect("adapter args");
            assert!(args.windows(2).any(|pair| {
                pair[0].as_str() == Some("--input-format") && pair[1].as_str() == Some(format)
            }));
        }
        for adapter in adapters.iter().filter(|adapter| adapter["tool"] == "yq") {
            if let Some(query) = adapter.get("query") {
                assert_ne!(query.as_str().expect("adapter query"), "");
            }
        }
    }
    assert_eq!(ids.len(), 17);
}

#[test]
fn workload_breadth_and_stream_resource_requirements_are_explicit() {
    let cases = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../benchmarks/cases/workloads.jsonl"),
    )
    .expect("benchmark cases");
    for workload in [
        "benchmark.startup",
        "benchmark.parse-discard",
        "benchmark.scalar-extraction",
        "benchmark.multi-result-projection",
        "benchmark.selective-filter",
        "benchmark.numeric-reduction",
        "benchmark.string-reduction",
        "benchmark.array-construction",
        "benchmark.object-construction",
        "benchmark.path-update",
        "benchmark.blocking-sort",
        "benchmark.dead-sort-length",
        "benchmark.identity-reencode",
        "benchmark.event-stream",
        "benchmark.recursive-scalars",
        "benchmark.user-filter-call",
        "benchmark.regex-test",
    ] {
        assert!(
            cases.contains(&format!("\"{workload}\"")),
            "missing {workload}"
        );
    }
    let event: Value = cases
        .lines()
        .map(|line| serde_json::from_str(line).expect("case JSON"))
        .find(|case: &Value| case["id"] == "benchmark.event-stream")
        .expect("event stream case");
    assert_eq!(event["measure_first_result"], true);
    assert_eq!(event["limits"]["rss_bytes"], 128 * 1024 * 1024);
}

fn compatibility_ids(root: &Path) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for entry in fs::read_dir(root.join("tests/compatibility/cases")).expect("compatibility cases")
    {
        let path = entry.expect("case entry").path();
        for line in fs::read_to_string(path).expect("case file").lines() {
            let case: Value = serde_json::from_str(line).expect("case JSON");
            ids.insert(case["id"].as_str().expect("case ID").to_owned());
        }
    }
    ids
}
