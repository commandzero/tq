//! End-to-end harness boundaries using controlled fake executables.

#![cfg(unix)]

use std::{fs, os::unix::fs::PermissionsExt, path::Path, time::Duration};

use tq_test_support::compatibility::{
    CampaignProfile, CompatibilityCatalog, ErrorClass, ExecutableConfig, Invocation, ProcessStatus,
    ToolKind, normalize_jq, normalize_raw, run_campaign, run_process,
};
use tq_test_support::corpus::ArtifactIdentity;

#[test]
fn case_adapters_preserve_argument_order_and_child_environment() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let executable = fake(
        directory.path(),
        "jq",
        "if [ \"$1\" = --version ]; then printf 'jq-test\\n'; exit 0; fi\nprintf '\"%s\"\\n' \"$@\" \"$TQ_COMPAT_ADAPTER_TEST\"\nprintf '%s' \"$TQ_COMPAT_DIAGNOSTIC\" >&2",
    );
    let mut cases = Vec::new();
    for omit_query in [false, true] {
        cases.push(
            serde_json::from_value(serde_json::json!({
                "schema_version": 1,
                "id": format!("adapter.omit-{omit_query}"),
                "title": "Argument placement",
                "classification": "jq-target",
                "capabilities": ["manual.adapter"],
                "status": "mvp",
                "fixture": {"format": "none", "inline": ""},
                "query": "query",
                "adapters": {"jq": {
                    "supported": true,
                    "args": ["before"],
                    "trailing_args": ["after"],
                    "omit_query": omit_query,
                    "env": {"TQ_COMPAT_ADAPTER_TEST": "isolated"}
                }},
                "invocation_mode": "null-input",
                "expected": {"contract": "result-sequence", "baseline": "required"}
            }))
            .expect("adapter case"),
        );
    }
    for compare_stderr in [false, true] {
        cases.push(serde_json::from_value(serde_json::json!({
            "schema_version": 1,
            "id": format!("adapter.stderr-{compare_stderr}"),
            "title": "Observable stderr",
            "classification": "jq-target",
            "capabilities": ["manual.stderr"],
            "status": "mvp",
            "fixture": {"format": "none", "inline": ""},
            "query": "query",
            "adapters": {
                "jq": {"supported": true, "env": {"TQ_COMPAT_DIAGNOSTIC": "left"}},
                "tq": {"supported": true, "env": {"TQ_COMPAT_DIAGNOSTIC": "right"}}
            },
            "invocation_mode": "null-input",
            "expected": {"contract": "raw-bytes", "baseline": "required", "compare_stderr": compare_stderr}
        })).expect("stderr case"));
    }
    let catalog = CompatibilityCatalog {
        cases,
        identity: ArtifactIdentity {
            path: "test".to_owned(),
            bytes: 0,
            sha256: String::new(),
        },
    };
    let report = run_campaign(
        &catalog,
        CampaignProfile::Full,
        &ExecutableConfig {
            jq: Some(executable.clone()),
            yq: Some(executable.clone()),
            tq: Some(executable),
        },
        directory.path(),
        Duration::from_secs(2),
    )
    .expect("adapter campaign");
    for (case, expected) in report.cases.iter().zip([
        vec!["before", "query", "after", "isolated"],
        vec!["before", "after", "isolated"],
    ]) {
        let observed = case
            .observations
            .iter()
            .find(|value| value.tool == ToolKind::Jq)
            .expect("jq observation");
        assert_eq!(
            observed.results,
            expected
                .into_iter()
                .map(serde_json::Value::from)
                .collect::<Vec<_>>()
        );
    }
    assert!(report.cases[2].semantic_diffs.is_empty());
    assert_eq!(report.cases[3].semantic_diffs.len(), 1);
    assert_eq!(report.cases[3].semantic_diffs[0].summary, "raw stderr");
}

fn fake(directory: &Path, name: &str, body: &str) -> std::path::PathBuf {
    let path = directory.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write fake executable");
    let mut permissions = fs::metadata(&path).expect("fake metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("make fake executable");
    path
}

#[test]
fn manual_comparison_uses_explicit_encoders_and_only_sizes_equivalent_successes() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("tests/compatibility/reviews")).unwrap();
    let executable = fake(
        directory.path(),
        "encoder",
        r#"
if [ "$1" = --version ]; then printf 'fake-1\n'; exit 0; fi
if [ "$ROLE" = jq ]; then printf '%s' "$OUTPUT"; exit "$CODE"; fi
if [ "$1" != -o ]; then exit 90; fi
case "$2" in
json) printf '%s' "$OUTPUT"; exit "$CODE" ;;
toon) printf '%s' "$TOON"; exit 0 ;;
*) exit 91 ;;
esac
"#,
    );
    let fixtures = comparison_fixtures();
    let cases = fixtures.iter().map(|(id, jq, json, toon, jq_code, tq_code, _, _)| {
        serde_json::from_value(serde_json::json!({
            "schema_version": 1, "id": format!("manual.test-{id}"), "title": id,
            "classification": "jq-target", "capabilities": ["manual.comparison"], "status": "mvp",
            "fixture": {"format": "none", "inline": ""}, "query": ".", "invocation_mode": "null-input",
            "expected": {"contract": "result-sequence", "baseline": "required"},
            "adapters": {
                "jq": {"supported": true, "env": {"ROLE": "jq", "OUTPUT": jq, "CODE": jq_code}},
                "tq": {"supported": true, "args": ["--"], "env": {"ROLE": "tq", "OUTPUT": json, "TOON": toon, "CODE": tq_code}}
            }
        })).unwrap()
    }).collect();
    let report = tq_test_support::compatibility::compare_manual(
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
    .unwrap();
    for (id, _, json, toon, _, _, verdict, sized) in fixtures {
        let row = report["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == format!("manual.test-{id}"))
            .unwrap();
        assert_eq!(row["verdict"], verdict, "{id}");
        assert_eq!(row["characters"].is_object(), sized, "{id}");
        if sized {
            assert_eq!(row["characters"]["json"], json.chars().count());
            assert_eq!(row["characters"]["toon"], toon.chars().count());
        }
    }
    assert_eq!(report["summary"]["size_samples"], 2);
}

type ComparisonFixture = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    bool,
);

fn comparison_fixtures() -> [ComparisonFixture; 7] {
    [
        (
            "equivalent",
            "{\"b\":2,\"a\":1.0}\n",
            "{\"a\":1, \"b\":2}\n",
            "\u{1e}a: 1\nb: 2\n",
            "0",
            "0",
            "match",
            true,
        ),
        (
            "ordered",
            "[1,2]\n",
            "[2,1]\n",
            "\u{1e}[2]: 2,1\n",
            "0",
            "0",
            "failure",
            false,
        ),
        (
            "unicode",
            "\"é😀\"\n",
            "\"é😀\"\n",
            "\u{1e}é😀\n",
            "0",
            "0",
            "match",
            true,
        ),
        (
            "bad-toon",
            "1\n",
            "1\n",
            "\u{1e}2\n",
            "0",
            "0",
            "match",
            false,
        ),
        (
            "partial-error",
            "1\n",
            "1\n",
            "\u{1e}1\n",
            "0",
            "5",
            "failure",
            false,
        ),
        ("both-error", "", "", "", "3", "3", "failure", false),
        (
            "malformed",
            "1\n",
            "broken",
            "\u{1e}1\n",
            "0",
            "0",
            "failure",
            false,
        ),
    ]
}

fn invoke(
    executable: std::path::PathBuf,
    timeout: Duration,
) -> tq_test_support::compatibility::ProcessOutcome {
    run_process(&Invocation {
        executable,
        args: Vec::new(),
        stdin: b"fixture".to_vec(),
        timeout,
        current_dir: None,
    })
    .expect("fake process")
}

#[test]
fn controlled_value_and_error_outputs_are_normalized() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let value = invoke(
        fake(directory.path(), "value", "printf '{\"ok\":true}\\n'"),
        Duration::from_secs(1),
    );
    assert_eq!(
        normalize_jq(&value).unwrap().results,
        [serde_json::json!({"ok": true})]
    );

    let error = invoke(
        fake(
            directory.path(),
            "error",
            "printf 'cannot index number with string' >&2; exit 5",
        ),
        Duration::from_secs(1),
    );
    assert_eq!(
        normalize_raw(tq_test_support::compatibility::ToolKind::Jq, &error).error_class,
        Some(ErrorClass::RuntimeTypePath)
    );
}

#[test]
fn controlled_timeout_signal_and_malformed_output_are_distinct() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let timeout = invoke(
        fake(directory.path(), "timeout", "while :; do :; done"),
        Duration::from_millis(30),
    );
    assert_eq!(timeout.status, ProcessStatus::TimedOut);

    let signal = invoke(
        fake(directory.path(), "signal", "kill -TERM $$"),
        Duration::from_secs(1),
    );
    assert_eq!(signal.status, ProcessStatus::Signaled);

    let malformed = invoke(
        fake(directory.path(), "malformed", "printf 'not-json'"),
        Duration::from_secs(1),
    );
    assert!(normalize_jq(&malformed).is_err());
}
