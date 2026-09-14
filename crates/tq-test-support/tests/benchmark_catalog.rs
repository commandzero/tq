//! Runtime benchmark adapter contract checks.

use std::{fs, path::Path};

use serde_json::{Value, json};
use tq_test_support::benchmark::load_benchmark_catalog;

fn case_with_adapter(applicable: bool, reason: Option<Value>) -> Value {
    let mut case = json!({
        "schema_version": 1,
        "id": "catalog-test",
        "compatibility_gate": "common.identity",
        "dataset_selector": {"family": "natural", "tiers": ["small"]},
        "query": ".",
        "execution_class": "document",
        "measure_first_result": false,
        "sampling": {"warmups": 0, "small": 30, "medium": 10, "large": 3},
        "timeout_seconds": 1,
        "limits": {"output_bytes": 1},
        "output_contract": {"kind": "semantic-sequence", "reference_adapter": "jq-json"},
        "adapters": [{
            "id": "jq-json", "tool": "jq", "input_format": "json",
            "applicable": applicable, "args": [], "comparison_families": ["same-format"]
        }]
    });
    if let Some(reason) = reason {
        case["adapters"][0]["unsupported_reason"] = reason;
    }
    case
}

#[test]
fn catalog_rejects_invalid_adapter_reasons_with_source_position() {
    let directory = tempfile::tempdir().expect("catalog directory");
    let path = directory.path().join("custom.jsonl");
    for (applicable, reason) in [
        (false, None),
        (false, Some(json!(""))),
        (false, Some(json!(" \t\n "))),
        (false, Some(Value::Null)),
        (true, Some(json!("format unavailable"))),
        (true, Some(Value::Null)),
    ] {
        let case = case_with_adapter(applicable, reason);
        fs::write(&path, format!("\n{case}\n")).expect("write catalog");
        let error = load_benchmark_catalog(directory.path())
            .expect_err(&format!(
                "invalid adapter accepted: {}",
                case["adapters"][0]
            ))
            .to_string();
        assert!(error.contains(&format!("{}:2:", path.display())), "{error}");
        assert!(error.contains("unsupported_reason"), "{error}");
    }
}

#[test]
fn catalog_rejects_missing_reference_adapter_with_source_position() {
    let directory = tempfile::tempdir().expect("catalog directory");
    let path = directory.path().join("custom.jsonl");
    let mut case = case_with_adapter(true, None);
    case["output_contract"]["reference_adapter"] = json!("missing-reference");
    fs::write(&path, format!("\n{case}\n")).expect("write catalog");

    let error = load_benchmark_catalog(directory.path())
        .expect_err("missing reference adapter was accepted")
        .to_string();
    assert!(error.contains(&format!("{}:2:", path.display())), "{error}");
    assert!(error.contains("reference adapter"), "{error}");
    assert!(error.contains("missing-reference"), "{error}");
}

#[test]
fn catalog_rejects_inapplicable_reference_adapter_with_source_position() {
    let directory = tempfile::tempdir().expect("catalog directory");
    let path = directory.path().join("custom.jsonl");
    let case = case_with_adapter(false, Some(json!("format unavailable")));
    fs::write(&path, format!("\n{case}\n")).expect("write catalog");

    let error = load_benchmark_catalog(directory.path())
        .expect_err("inapplicable reference adapter was accepted")
        .to_string();
    assert!(error.contains(&format!("{}:2:", path.display())), "{error}");
    assert!(error.contains("reference adapter"), "{error}");
    assert!(error.contains("jq-json"), "{error}");
    assert!(error.contains("applicable"), "{error}");
}

#[test]
fn catalog_preserves_valid_adapter_reasons_and_loads_shipped_cases() {
    let directory = tempfile::tempdir().expect("catalog directory");
    for (applicable, reason) in [(true, None), (false, Some("format unavailable"))] {
        let mut case = case_with_adapter(applicable, reason.map(|reason| json!(reason)));
        let mut reference = case_with_adapter(true, None)["adapters"][0].clone();
        reference["id"] = json!("reference");
        case["adapters"].as_array_mut().unwrap().push(reference);
        case["output_contract"]["reference_adapter"] = json!("reference");
        fs::write(directory.path().join("custom.jsonl"), case.to_string()).expect("write catalog");
        let catalog = load_benchmark_catalog(directory.path()).expect("valid catalog");
        assert_eq!(catalog.cases[0].adapters[0].applicable, applicable);
        assert_eq!(
            catalog.cases[0].adapters[0].unsupported_reason.as_deref(),
            reason
        );
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalog = load_benchmark_catalog(&root.join("benchmarks/cases")).expect("shipped catalog");
    assert!(!catalog.cases.is_empty());
}
