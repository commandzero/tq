//! Regression coverage for manual comparison output contracts.

#![cfg(unix)]
#![allow(missing_docs)]

use std::{fs, os::unix::fs::PermissionsExt, path::Path, time::Duration};

use tq_test_support::compatibility::{CompatibilityCatalog, ExecutableConfig, compare_manual};
use tq_test_support::corpus::ArtifactIdentity;

fn fake(directory: &Path, name: &str, body: &str) -> std::path::PathBuf {
    let path = directory.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write fake executable");
    let mut permissions = fs::metadata(&path).expect("fake metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("make fake executable");
    path
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
    let executable = fake(
        directory.path(),
        "encoder",
        r#"
if [ "$1" = --version ]; then printf 'fake-1\n'; exit 0; fi
format=json
sequence=false
while [ "$#" -gt 0 ]; do
    case "$1" in
        -o|--output-format) format="$2"; shift 2 ;;
        -o=*|--output-format=*) format="${1#*=}"; shift ;;
        -o*) format="${1#-o}"; shift ;;
        --seq) sequence=true; shift ;;
        *) shift ;;
    esac
done
if [ "$ROLE" = jq ]; then printf '{"a":1}\n'; exit "${JQ_CODE:-0}"; fi
case "$format" in
    json)
        if [ "$MODE" = mismatch ]; then printf '{"a":2}\n'; else printf '{"a":1}\n'; fi
        ;;
    toon)
        if [ "$sequence" = true ]; then printf '\036'; fi
        if [ "$MODE" = mismatch ]; then printf 'a: 2\n'; else printf 'a: 1\n'; fi
        ;;
    *) exit 91 ;;
esac
exit "${TQ_CODE:-0}"
"#,
    );
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
    [base, raw, mismatch, failed, sequence]
        .into_iter()
        .map(|value| serde_json::from_value(value).expect("comparison case"))
        .collect()
}
