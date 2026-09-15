//! Regression coverage for manual comparison output contracts.

#![cfg(unix)]
#![allow(missing_docs)]

use std::{fs, os::unix::fs::PermissionsExt, path::Path, time::Duration};

use tq_test_support::compatibility::{
    CompatibilityCatalog, ExecutableConfig, compare_manual, encode_hex,
};
use tq_test_support::corpus::ArtifactIdentity;

fn fake(directory: &Path, name: &str, body: &str) -> std::path::PathBuf {
    let path = directory.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write fake executable");
    let mut permissions = fs::metadata(&path).expect("fake metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("make fake executable");
    path
}

fn comparison_encoder(directory: &Path) -> std::path::PathBuf {
    fake(
        directory,
        "encoder",
        r#"
if [ "$1" = --version ]; then printf 'fake-1\n'; exit 0; fi
format=json
sequence=false
unframed=false
while [ "$#" -gt 0 ]; do
    case "$1" in
        -o|--output-format) format="$2"; shift 2 ;;
        -o=*|--output-format=*) format="${1#*=}"; shift ;;
        -o*) format="${1#-o}"; shift ;;
        --seq) sequence=true; shift ;;
        --unframed) unframed=true; shift ;;
        *) shift ;;
    esac
done
if [ "$ROLE" = jq ]; then printf '{"a":1}\n'; exit "${JQ_CODE:-0}"; fi
case "$format" in
    json|jsonl|json-seq)
        if [ "$unframed" = true ]; then exit 92; fi
        if [ "$format" = jsonl ]; then
            if [ "$MODE" = mismatch ]; then printf '{"a":2}\n'; else printf '{"a":1}\n'; fi
        elif [ "$sequence" = true ] || [ "$format" = json-seq ]; then
            printf '\036'
            if [ "$MODE" = mismatch ]; then printf '{"a":2}\n'; else printf '{"a":1}\n'; fi
        elif [ "$MODE" = mismatch ]; then
            printf '{"a":2}\n'
        else
            printf '{"a":1}\n'
        fi
        ;;
    toon|toon-seq)
        if [ "$sequence" = true ] || [ "$format" = toon-seq ]; then printf '\036'; fi
        if [ "$MODE" = mismatch ]; then printf 'a: 2\n'; else printf 'a: 1\n'; fi
        ;;
    *) exit 91 ;;
esac
exit "${TQ_CODE:-0}"
"#,
    )
}

#[test]
fn manual_comparison_forces_authored_tq_output_flags_to_each_contract() {
    let directory = tempfile::tempdir().expect("temporary directory");
    fs::create_dir_all(
        directory
            .path()
            .join("tests/compatibility/reviews/jq-manual"),
    )
    .expect("create manual review directory");
    let executable = comparison_encoder(directory.path());
    let cases = comparison_cases();
    let report = compare_manual(
        &CompatibilityCatalog {
            cases,
            identity: ArtifactIdentity {
                path: "fake".into(),
                bytes: 0,
                sha256: String::new(),
            },
        },
        &ExecutableConfig {
            jq: Some(executable.clone()),
            tq: Some(executable),
            yq: None,
        },
        directory.path(),
        Duration::from_secs(2),
    )
    .expect("comparison report");

    let row = report["cases"]
        .as_array()
        .expect("comparison rows")
        .iter()
        .find(|row| row["id"] == "manual.test-forced-output-format")
        .expect("successful comparison row");
    assert_eq!(row["verdict"], "match");
    assert_eq!(row["json_equivalent"], true);
    assert_eq!(row["toon_equivalent"], true);
    assert!(row["tokens"].is_object());

    let raw_row = report["cases"]
        .as_array()
        .expect("comparison rows")
        .iter()
        .find(|row| row["id"] == "manual.test-raw-contract")
        .expect("raw comparison row");
    assert_eq!(raw_row["verdict"], "match");
    assert!(raw_row.get("tokens").is_none());

    let rows = report["cases"].as_array().expect("comparison rows");
    let mismatch_row = rows
        .iter()
        .find(|row| row["id"] == "manual.test-mismatched-structured")
        .expect("mismatch comparison row");
    assert_eq!(mismatch_row["verdict"], "failure");
    assert_eq!(mismatch_row["json_equivalent"], false);
    assert!(mismatch_row.get("tokens").is_none());

    let failed_row = rows
        .iter()
        .find(|row| row["id"] == "manual.test-failed-structured")
        .expect("failed comparison row");
    assert_eq!(failed_row["verdict"], "failure");
    assert!(failed_row.get("tokens").is_none());

    let sequence_row = rows
        .iter()
        .find(|row| row["id"] == "manual.test-sequence-no-tokens")
        .expect("sequence comparison row");
    assert_eq!(sequence_row["toon_sequence"], true);
    assert_eq!(sequence_row["verdict"], "match");
    assert_eq!(sequence_row["toon_equivalent"], true);
    assert!(sequence_row.get("tokens").is_none());

    for id in [
        "manual.test-unframed-output",
        "manual.test-json-lines-input-sequence",
    ] {
        let row = rows
            .iter()
            .find(|row| row["id"] == id)
            .expect("framing comparison row");
        assert_eq!(row["verdict"], "match", "{id}");
        assert_eq!(row["json_equivalent"], true, "{id}");
        assert_eq!(row["toon_equivalent"], true, "{id}");
    }
    let json_lines_row = rows
        .iter()
        .find(|row| row["id"] == "manual.test-json-lines-input-sequence")
        .expect("JSON Lines framing comparison row");
    assert_eq!(json_lines_row["toon_sequence"], true);
}

#[test]
fn manual_comparison_validates_unframed_cardinality_and_errors() {
    let directory = tempfile::tempdir().expect("temporary directory");
    fs::create_dir_all(
        directory
            .path()
            .join("tests/compatibility/reviews/jq-manual"),
    )
    .expect("create manual review directory");
    let executable = unframed_comparison_encoder(directory.path());
    let cases = [
        unframed_case("manual.test-unframed-no-lf", "success"),
        unframed_case("manual.test-unframed-empty-error", "error"),
        unframed_case("manual.test-unframed-zero-results", "zero"),
    ];
    let report = compare_manual(
        &CompatibilityCatalog {
            cases: cases.into_iter().collect(),
            identity: ArtifactIdentity {
                path: "fake-unframed".into(),
                bytes: 0,
                sha256: String::new(),
            },
        },
        &ExecutableConfig {
            jq: Some(executable.clone()),
            tq: Some(executable),
            yq: None,
        },
        directory.path(),
        Duration::from_secs(2),
    )
    .expect("unframed comparison report");
    let rows = report["cases"].as_array().expect("comparison rows");

    let success = rows
        .iter()
        .find(|row| row["id"] == "manual.test-unframed-no-lf")
        .expect("unframed scalar row");
    assert_eq!(success["verdict"], "match");
    assert_eq!(success["tq_toon"]["state"], "executed");
    assert_eq!(success["tq_toon"]["results"], serde_json::json!([1]));
    assert_eq!(success["tq_toon"]["stdout_hex"], encode_hex(b"1"));

    let error = rows
        .iter()
        .find(|row| row["id"] == "manual.test-unframed-empty-error")
        .expect("unframed error row");
    assert_eq!(error["verdict"], "match");
    assert_eq!(error["tq_toon"]["state"], "executed");
    assert_eq!(error["tq_toon"]["results"], serde_json::json!([]));
    assert_eq!(error["tq_toon"]["exit_code"], 5);
    assert_eq!(error["tq_toon"]["error_class"], "runtime-type-path");

    let zero = rows
        .iter()
        .find(|row| row["id"] == "manual.test-unframed-zero-results")
        .expect("unframed zero-result row");
    assert_eq!(zero["verdict"], "match");
    assert_eq!(zero["tq_toon"]["state"], "harness-error");
    assert_eq!(zero["tq_toon"]["error_class"], "malformed-output");
}

fn unframed_comparison_encoder(directory: &Path) -> std::path::PathBuf {
    fake(
        directory,
        "unframed-encoder",
        r#"
if [ "$1" = --version ]; then printf 'fake-1\n'; exit 0; fi
format=json
while [ "$#" -gt 0 ]; do
    case "$1" in
        -o|--output-format) format="$2"; shift 2 ;;
        -o=*|--output-format=*) format="${1#*=}"; shift ;;
        -o*) format="${1#-o}"; shift ;;
        *) shift ;;
    esac
done
if [ "$SCENARIO" = success ]; then
    if [ "$ROLE" = jq ] || [ "$format" != toon ]; then printf '1\n'; else printf '1'; fi
    exit 0
fi
if [ "$SCENARIO" = error ]; then exit 5; fi
exit 0
"#,
    )
}

fn unframed_case(id: &str, scenario: &str) -> tq_test_support::compatibility::CompatibilityCase {
    let expected = if scenario == "error" {
        serde_json::json!({
            "contract": "error",
            "baseline": "required",
            "error_class": "runtime-type-path"
        })
    } else {
        serde_json::json!({"contract": "result-sequence", "baseline": "required"})
    };
    serde_json::from_value(serde_json::json!({
        "schema_version": 1,
        "id": id,
        "title": "Unframed comparison contract",
        "classification": "jq-target",
        "capabilities": ["manual.comparison"],
        "status": "mvp",
        "fixture": {"format": "none", "inline": ""},
        "query": ".",
        "invocation_mode": "null-input",
        "expected": expected,
        "adapters": {
            "jq": {"supported": true, "env": {"ROLE": "jq", "SCENARIO": scenario}},
            "tq": {
                "supported": true,
                "args": ["--unframed"],
                "env": {"ROLE": "tq", "SCENARIO": scenario}
            }
        }
    }))
    .expect("unframed comparison case")
}

fn comparison_cases() -> Vec<tq_test_support::compatibility::CompatibilityCase> {
    let base = serde_json::json!({
        "schema_version": 1,
        "id": "manual.test-forced-output-format",
        "title": "Forced output format",
        "classification": "jq-target",
        "capabilities": ["manual.comparison"],
        "status": "mvp",
        "fixture": {"format": "none", "inline": ""},
        "query": ".",
        "invocation_mode": "null-input",
        "expected": {"contract": "result-sequence", "baseline": "required"},
        "adapters": {
            "jq": {"supported": true, "env": {"ROLE": "jq"}},
            "tq": {
                "supported": true,
                "args": ["--output-format", "toon"],
                "env": {"ROLE": "tq"}
            }
        }
    });
    let mut raw = base.clone();
    raw["id"] = "manual.test-raw-contract".into();
    raw["expected"]["contract"] = "raw-bytes".into();
    raw["adapters"]["tq"]["args"] = serde_json::json!(["-o", "json"]);
    let mut mismatch = base.clone();
    mismatch["id"] = "manual.test-mismatched-structured".into();
    mismatch["adapters"]["tq"]["env"]["MODE"] = "mismatch".into();
    let mut failed = base.clone();
    failed["id"] = "manual.test-failed-structured".into();
    failed["adapters"]["tq"]["env"]["TQ_CODE"] = "5".into();
    let mut sequence = base.clone();
    sequence["id"] = "manual.test-sequence-no-tokens".into();
    sequence["adapters"]["tq"]["args"] = serde_json::json!(["--seq"]);
    let mut unframed = base.clone();
    unframed["id"] = "manual.test-unframed-output".into();
    unframed["adapters"]["tq"]["args"] = serde_json::json!(["--unframed"]);
    let mut json_lines = base.clone();
    json_lines["id"] = "manual.test-json-lines-input-sequence".into();
    json_lines["adapters"]["tq"]["args"] = serde_json::json!(["--seq", "-o", "jsonl"]);
    [base, raw, mismatch, failed, sequence, unframed, json_lines]
        .into_iter()
        .map(|value| serde_json::from_value(value).expect("comparison case"))
        .collect()
}
