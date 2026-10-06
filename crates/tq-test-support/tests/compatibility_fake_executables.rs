//! End-to-end harness boundaries using controlled fake executables.

#![cfg(unix)]

use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    sync::{Mutex, MutexGuard},
    time::Duration,
};

use tq_test_support::compatibility::{
    CampaignProfile, CompatibilityCase, CompatibilityCatalog, ErrorClass, ExecutableConfig,
    FixtureFormat, Invocation, InvocationMode, ProcessStatus, ToolKind, normalize_jq,
    normalize_raw, run_campaign, run_process,
};
use tq_test_support::corpus::ArtifactIdentity;

// Serialize temporary executable writers and runners to avoid native ETXTBSY
// when parallel fixture tests create and execute files concurrently.
static FIXTURE_EXECUTABLE_TEST_LOCK: Mutex<()> = Mutex::new(());

fn fixture_executable_test_lock() -> MutexGuard<'static, ()> {
    FIXTURE_EXECUTABLE_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn manual_review_directory() -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("temporary directory");
    fs::create_dir_all(
        directory
            .path()
            .join("tests/compatibility/reviews/jq-manual"),
    )
    .expect("create manual review directory");
    directory
}

#[test]
fn case_adapters_preserve_argument_order_and_child_environment() {
    let _lock = fixture_executable_test_lock();
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
            ..Default::default()
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
fn tq_only_error_contract_rejects_success_and_wrong_error_class() {
    let _lock = fixture_executable_test_lock();
    let directory = tempfile::tempdir().unwrap();
    for (name, body, expected_code) in [
        ("permissive", "printf '1\\n'", 0),
        (
            "wrong-error",
            "printf 'tq: runtime error: wrong type\\n' >&2; exit 5",
            5,
        ),
    ] {
        let executable = fake(
            directory.path(),
            name,
            &format!("if [ \"$1\" = --version ]; then printf 'tq-test\\n'; exit 0; fi\n{body}"),
        );
        let case = serde_json::from_value(serde_json::json!({
            "schema_version": 1, "id": "policy.denied", "title": "Explicit denial",
            "classification": "jq-target", "capabilities": ["policy.denied"], "status": "mvp",
            "fixture": {"format": "json", "inline": "null"}, "query": "env",
            "adapters": {"tq": {"supported": true}}, "invocation_mode": "stdin",
            "expected": {"contract": "error", "baseline": "not-applicable", "error_class": "runtime-policy"}
        })).unwrap();
        let report = run_campaign(
            &CompatibilityCatalog {
                cases: vec![case],
                identity: ArtifactIdentity {
                    path: "test".into(),
                    bytes: 0,
                    sha256: String::new(),
                },
            },
            CampaignProfile::Full,
            &ExecutableConfig {
                tq: Some(executable),
                ..Default::default()
            },
            directory.path(),
            Duration::from_secs(2),
        )
        .unwrap();
        assert_eq!(
            report.final_status,
            tq_test_support::compatibility::FinalStatus::Failed,
            "{name}"
        );
        assert_ne!(
            report.capability_matrix["policy.denied"],
            tq_test_support::compatibility::CapabilityDisposition::Supported
        );
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(
            json["cases"][0]["contract_failures"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
        for observation in report.cases[0]
            .observations
            .iter()
            .filter(|o| o.tool == ToolKind::Tq)
        {
            assert_eq!(
                observation.state,
                tq_test_support::compatibility::ObservationState::Executed
            );
            assert_eq!(observation.exit_code, Some(expected_code));
            assert_ne!(observation.error_class, Some(ErrorClass::MalformedOutput));
            if expected_code == 0 {
                assert_eq!(observation.results, [serde_json::json!(1)]);
            }
        }
        assert!(report.render_human().contains("declared error contract"));
    }
}

#[test]
fn embedded_host_must_deny_the_selected_authority_without_output_or_disclosure() {
    let _lock = fixture_executable_test_lock();
    let directory = tempfile::tempdir().unwrap();
    for (name, body) in [
        ("permissive-host", "printf '1\\n'"),
        (
            "same-line-disclosure",
            "printf 'tq: runtime error: private-sentinel-value env requires environment access permitted by capability policy\\n' >&2; exit 5",
        ),
        (
            "extra-diagnostic-line",
            "printf 'tq: runtime error: env requires environment access permitted by capability policy\\nprivate-data\\n' >&2; exit 5",
        ),
        (
            "wrong-authority",
            "printf 'tq: runtime error: now requires platform access permitted by capability policy\\n' >&2; exit 5",
        ),
        (
            "leaky-host",
            "printf 'tq: runtime error: env requires environment access permitted by capability policy present\\n' >&2; exit 5",
        ),
        (
            "partial-output",
            "printf '1\\n'; printf 'tq: runtime error: env requires environment access permitted by capability policy\\n' >&2; exit 5",
        ),
    ] {
        let host = fake(
            directory.path(),
            name,
            &format!(
                "if [ \"$1\" = --version ]; then printf 'embedded-test\\n'; exit 0; fi\n{body}"
            ),
        );
        let case = serde_json::from_value(serde_json::json!({
            "schema_version": 1, "id": "arbitrary.policy", "title": "Embedded policy contract",
            "classification": "jq-target", "capabilities": ["policy.denied"], "status": "mvp",
            "fixture": {"format": "json", "inline": "null"}, "query": "env",
            "adapters": {"tq": {"supported": true, "execution_mode": "embedded-deny-environment"}},
            "invocation_mode": "stdin",
            "expected": {"contract": "error", "baseline": "not-applicable", "error_class": "runtime-policy"}
        })).unwrap();
        let report = run_campaign(
            &CompatibilityCatalog {
                cases: vec![case],
                identity: ArtifactIdentity {
                    path: "test".into(),
                    bytes: 0,
                    sha256: String::new(),
                },
            },
            CampaignProfile::Full,
            &ExecutableConfig {
                embedded_host: Some(host.clone()),
                ..Default::default()
            },
            directory.path(),
            Duration::from_secs(2),
        )
        .unwrap();
        assert_eq!(
            report.final_status,
            tq_test_support::compatibility::FinalStatus::Failed,
            "{name}"
        );
        assert_eq!(report.cases[0].contract_failures.len(), 6, "{name}");
        assert_eq!(
            report.cases[0].tq_execution.as_ref().unwrap().host.path,
            host.canonicalize().unwrap()
        );
        assert!(
            report.cases[0]
                .observations
                .iter()
                .filter(|o| o.tool == ToolKind::Tq)
                .all(|o| o.state == tq_test_support::compatibility::ObservationState::Executed)
        );
    }
}

#[test]
fn embedded_json_companion_independently_enforces_the_denial_contract() {
    let _lock = fixture_executable_test_lock();
    let directory = tempfile::tempdir().unwrap();
    for (name, diagnostic, valid) in [
        (
            "valid",
            "env requires environment access permitted by capability policy\\n",
            true,
        ),
        (
            "wrong-authority",
            "now requires platform access permitted by capability policy\\n",
            false,
        ),
        (
            "extra-line",
            "env requires environment access permitted by capability policy\\nprivate-data\\n",
            false,
        ),
        (
            "same-line",
            "private-sentinel-value env requires environment access permitted by capability policy\\n",
            false,
        ),
    ] {
        let host = fake(
            directory.path(),
            name,
            &format!(
                "if [ \"$1\" = --version ]; then printf 'embedded-test\\n'; exit 0; fi\nprevious=''\nfor argument in \"$@\"; do\n  if [ \"$previous\" = --output-format ] || [ \"$previous\" = -o ]; then\n    if [ \"$argument\" = json ]; then printf 'tq: runtime error: {diagnostic}' >&2; exit 5; fi\n  fi\n  previous=\"$argument\"\ndone\nprintf 'tq: runtime error: env requires environment access permitted by capability policy\\n' >&2\nexit 5"
            ),
        );
        let case = serde_json::from_value(serde_json::json!({
            "schema_version": 1, "id": "arbitrary.companion", "title": "Independent companion denial",
            "classification": "jq-target", "capabilities": ["policy.denied"], "status": "mvp",
            "fixture": {"format": "json", "inline": "null"}, "query": "env",
            "adapters": {"tq": {"supported": true, "execution_mode": "embedded-deny-environment"}},
            "invocation_mode": "stdin",
            "expected": {"contract": "error", "baseline": "not-applicable", "error_class": "runtime-policy"}
        })).unwrap();
        let report = run_campaign(
            &CompatibilityCatalog {
                cases: vec![case],
                identity: ArtifactIdentity {
                    path: "test".into(),
                    bytes: 0,
                    sha256: String::new(),
                },
            },
            CampaignProfile::Full,
            &ExecutableConfig {
                embedded_host: Some(host),
                ..Default::default()
            },
            directory.path(),
            Duration::from_secs(2),
        )
        .unwrap();
        assert_eq!(
            report.final_status,
            if valid {
                tq_test_support::compatibility::FinalStatus::Passed
            } else {
                tq_test_support::compatibility::FinalStatus::Failed
            },
            "{name}"
        );
        assert_eq!(
            report.cases[0].contract_failures.len(),
            if valid { 0 } else { 3 },
            "{name}"
        );
        for failure in &report.cases[0].contract_failures {
            assert!(failure.summary.contains("JSON companion"), "{name}");
        }
        for observation in report.cases[0]
            .observations
            .iter()
            .filter(|o| o.tool == ToolKind::Tq)
        {
            assert_eq!(
                observation.state,
                tq_test_support::compatibility::ObservationState::Executed,
                "{name}"
            );
            assert_eq!(observation.exit_code, Some(5));
            assert_eq!(observation.error_class, Some(ErrorClass::RuntimePolicy));
            assert_eq!(observation.stdout_hex.as_deref(), Some(""));
            assert_eq!(
                observation.stderr_hex,
                Some(tq_test_support::compatibility::encode_hex(
                    b"tq: runtime error: env requires environment access permitted by capability policy\n"
                ))
            );
            assert_eq!(observation.results, [] as [serde_json::Value; 0]);
        }
    }
}

#[test]
fn explicit_embedded_mode_never_falls_back_to_the_process_cli() {
    let _lock = fixture_executable_test_lock();
    let directory = tempfile::tempdir().unwrap();
    let executable = fake(
        directory.path(),
        "permissive-tq",
        "if [ \"$1\" = --version ]; then printf 'tq-test\\n'; exit 0; fi\nprintf '1\\n'",
    );
    let case = serde_json::from_value(serde_json::json!({
        "schema_version": 1, "id": "arbitrary.embedded", "title": "Explicit interface",
        "classification": "jq-target", "capabilities": ["policy.denied"], "status": "mvp",
        "fixture": {"format": "json", "inline": "null"}, "query": "env",
        "adapters": {"tq": {"supported": true, "execution_mode": "embedded-deny-environment"}},
        "invocation_mode": "stdin",
        "expected": {"contract": "error", "baseline": "not-applicable", "error_class": "runtime-policy"}
    })).unwrap();
    let report = run_campaign(
        &CompatibilityCatalog {
            cases: vec![case],
            identity: ArtifactIdentity {
                path: "test".into(),
                bytes: 0,
                sha256: String::new(),
            },
        },
        CampaignProfile::Full,
        &ExecutableConfig {
            tq: Some(executable),
            ..Default::default()
        },
        directory.path(),
        Duration::from_secs(2),
    )
    .unwrap();
    assert_eq!(
        report.final_status,
        tq_test_support::compatibility::FinalStatus::Failed
    );
    for observation in report.cases[0]
        .observations
        .iter()
        .filter(|o| o.tool == ToolKind::Tq)
    {
        assert_eq!(
            observation.state,
            tq_test_support::compatibility::ObservationState::HarnessError
        );
        assert_eq!(observation.exit_code, None);
        assert!(
            observation
                .note
                .as_deref()
                .unwrap()
                .contains("embedded host")
        );
    }
}

#[test]
fn fixture_environment_paths_follow_relocation_and_reject_escape_or_ambiguity() {
    let _lock = fixture_executable_test_lock();
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("controlled-home")).unwrap();
    let executable = fake(
        directory.path(),
        "reference",
        r#"
if [ "$1" = --version ]; then printf 'jq-test\n'; exit 0; fi
printf '"%s"\n' "$TQ_FIXTURE_PATH"
"#,
    );
    let mut case: tq_test_support::compatibility::CompatibilityCase =
        serde_json::from_value(serde_json::json!({
            "schema_version": 1, "id": "adapter.relocated", "title": "Relocated path",
            "classification": "jq-target", "capabilities": ["manual.adapter"], "status": "mvp",
            "fixture": {"format": "none", "inline": ""}, "query": ".",
            "adapters": {"jq": {"supported": true,
                "env_paths": {"TQ_FIXTURE_PATH": "controlled-home"}}},
            "invocation_mode": "null-input",
            "expected": {"contract": "result-sequence", "baseline": "required"}
        }))
        .unwrap();
    let run = |case| {
        run_campaign(
            &CompatibilityCatalog {
                cases: vec![case],
                identity: ArtifactIdentity {
                    path: "relocated".into(),
                    bytes: 0,
                    sha256: String::new(),
                },
            },
            CampaignProfile::Full,
            &ExecutableConfig {
                jq: Some(executable.clone()),
                ..Default::default()
            },
            directory.path(),
            Duration::from_secs(2),
        )
    };
    let report = run(case.clone()).unwrap();
    let observation = report.cases[0]
        .observations
        .iter()
        .find(|value| value.tool == ToolKind::Jq)
        .unwrap();
    assert_eq!(
        observation.results,
        [serde_json::json!(
            directory
                .path()
                .join("controlled-home")
                .canonicalize()
                .unwrap()
        )]
    );
    case.adapters
        .jq
        .env_paths
        .insert("TQ_FIXTURE_PATH".into(), "../outside".into());
    assert!(run(case.clone()).is_err());
    case.adapters
        .jq
        .env_paths
        .insert("TQ_FIXTURE_PATH".into(), "controlled-home".into());
    case.adapters
        .jq
        .env
        .insert("TQ_FIXTURE_PATH".into(), "ambiguous".into());
    assert!(run(case).is_err());
}

#[test]
fn manual_comparison_uses_explicit_encoders_and_only_sizes_equivalent_successes() {
    let _lock = fixture_executable_test_lock();
    let directory = manual_review_directory();
    let executable = fake(
        directory.path(),
        "encoder",
        r#"
if [ "$1" = --version ]; then printf 'fake-1\n'; exit 0; fi
if [ "$ROLE" = jq ]; then printf '%s' "$OUTPUT"; exit "$CODE"; fi
sequence=false
if [ "$1" = --seq ]; then sequence=true; shift; fi
if [ "$1" != -o ]; then exit 90; fi
case "$2" in
json) printf '%s' "$OUTPUT"; exit "$CODE" ;;
toon) if [ "$sequence" = true ] && [ -n "$TOON" ]; then printf '\036'; fi; printf '%s' "$TOON"; exit "$CODE" ;;
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
            ..Default::default()
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
        if sized {
            assert!(
                row["tokens"].is_object(),
                "{id} should expose output tokens"
            );
        } else {
            assert!(
                row.get("tokens").is_none(),
                "{id} must not expose tokens for an ineligible comparison"
            );
        }
        if id == "equivalent" {
            assert_eq!(
                row["compact"]["exact"], false,
                "semantic normalization must not erase compact byte differences"
            );
        }
        if id == "bad-toon" {
            assert_eq!(
                row["toon_contract_match"], false,
                "JSON success must not approve a changed TOON result"
            );
        }
        if id == "partial-error" {
            assert_eq!(
                row["toon_sequence"], false,
                "late errors must preserve default output mode"
            );
            assert_eq!(row["tq_toon"]["results"], serde_json::json!([1]));
            assert_eq!(row["toon_contract_match"], true);
        }
        if sized {
            assert_comparison_token_counts(row, json, toon);
        }
    }
    assert_eq!(report["summary"]["size_samples"], 2);
}

fn assert_comparison_token_counts(row: &serde_json::Value, json: &str, toon: &str) {
    let o200k = tiktoken_rs::o200k_base().unwrap();
    let cl100k = tiktoken_rs::cl100k_base().unwrap();
    for (encoding, tokenizer) in [("o200k_base", &o200k), ("cl100k_base", &cl100k)] {
        assert_eq!(
            row["tokens"][encoding]["json"],
            tokenizer.encode_ordinary(json).len()
        );
        assert_eq!(
            row["tokens"][encoding]["toon"],
            tokenizer.encode_ordinary(toon).len()
        );
    }
}

#[test]
fn manual_comparison_preserves_original_json_sequence_framing() {
    let _lock = fixture_executable_test_lock();
    let directory = manual_review_directory();
    let executable = fake(
        directory.path(),
        "encoder",
        r#"
if [ "$1" = --version ]; then printf 'fake-1\n'; exit 0; fi
if [ "$ROLE" = jq ]; then
    sequence=false
    for argument in "$@"; do
        if [ "$argument" = --seq ]; then sequence=true; fi
    done
    if [ "$sequence" = true ]; then printf '\0361\n'; else printf '1\n'; fi
    exit 0
fi
format=
previous=
sequence=false
for argument in "$@"; do
    if [ "$FORBID_SEQ" = true ] && [ "$argument" = --seq ]; then exit 92; fi
    if [ "$previous" = output ]; then format="$argument"; previous=; fi
    if [ "$argument" = -o ] || [ "$argument" = --output-format ]; then previous=output; fi
    case "$argument" in
        -o=*) format="${argument#-o=}";;
        --output-format=*) format="${argument#*=}";;
        -o?*) format="${argument#-o}";;
    esac
    if [ "$argument" = --seq ]; then sequence=true; fi
done
case "$format" in
json|json-seq)
    if [ "$sequence" = true ] || [ "$format" = json-seq ]; then printf '\0361\n'; else printf '1\n'; fi
    ;;
toon)
    if [ "$sequence" = true ]; then printf '\0361\n'; else printf '1\n'; fi
    ;;
toon-seq) printf '\0361\n';;
*) exit 91;;
esac
"#,
    );
    let case: CompatibilityCase = serde_json::from_value(serde_json::json!({
        "schema_version": 1,
        "id": "manual.test-original-sequence",
        "title": "Original JSON sequence adapter",
        "classification": "jq-target",
        "capabilities": ["manual.comparison"],
        "status": "mvp",
        "fixture": {"format": "none", "inline": ""},
        "query": ".",
        "invocation_mode": "null-input",
        "expected": {"contract": "result-sequence", "baseline": "required"},
        "adapters": {
            "jq": {"supported": true, "args": ["--seq"], "env": {"ROLE": "jq"}},
            "tq": {"supported": true, "args": ["--seq"], "env": {"ROLE": "tq"}}
        }
    }))
    .unwrap();
    let mut explicit_toon_sequence = case.clone();
    explicit_toon_sequence.id = "manual.test-original-toon-sequence".to_owned();
    explicit_toon_sequence.fixture.format = FixtureFormat::Json;
    explicit_toon_sequence.fixture.inline = Some("1\n".to_owned());
    explicit_toon_sequence.invocation_mode = InvocationMode::Stdin;
    explicit_toon_sequence.adapters.jq.args.clear();
    let tq_adapter = &mut explicit_toon_sequence.adapters.tq;
    tq_adapter.args = vec!["-o".into(), "toon-sequence".into()];
    tq_adapter.env.insert("FORBID_SEQ".into(), "true".into());
    let report = tq_test_support::compatibility::compare_manual(
        &CompatibilityCatalog {
            cases: vec![case, explicit_toon_sequence],
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
            ..Default::default()
        },
        directory.path(),
        Duration::from_secs(2),
    )
    .unwrap();
    let row = &report["cases"][0];
    assert_eq!(row["toon_sequence"], true);
    assert_eq!(row["toon_contract_match"], true);
    assert!(row.get("tokens").is_none());
    assert_eq!(report["summary"]["size_samples"], 0);
    let explicit_row = report["cases"]
        .as_array()
        .expect("comparison rows")
        .iter()
        .find(|row| row["id"] == "manual.test-original-toon-sequence")
        .expect("explicit sequence comparison row");
    assert_eq!(explicit_row["toon_sequence"], true);
    assert_eq!(explicit_row["toon_contract_match"], true);
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

fn comparison_fixtures() -> [ComparisonFixture; 9] {
    [
        (
            "equivalent",
            "{\"b\":2,\"a\":1.0}\n",
            "{\"a\":1, \"b\":2}\n",
            "a: 1\nb: 2\n",
            "0",
            "0",
            "match",
            true,
        ),
        (
            "ordered",
            "[1,2]\n",
            "[2,1]\n",
            "[2]: 2,1\n",
            "0",
            "0",
            "failure",
            false,
        ),
        (
            "unicode",
            "\"é😀\"\n",
            "\"é😀\"\n",
            "é😀\n",
            "0",
            "0",
            "match",
            true,
        ),
        ("bad-toon", "1\n", "1\n", "2\n", "0", "0", "match", false),
        (
            "partial-error",
            "1\n",
            "1\n",
            "1\n",
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
            "1\n",
            "0",
            "0",
            "failure",
            false,
        ),
        (
            "changed-number",
            "1\n",
            "2\n",
            "2\n",
            "0",
            "0",
            "failure",
            false,
        ),
        (
            "changed-order",
            "1\n2\n",
            "2\n1\n",
            "2\n1\n",
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
        environment: BTreeMap::new(),
    })
    .expect("fake process")
}

#[test]
fn controlled_value_and_error_outputs_are_normalized() {
    let _lock = fixture_executable_test_lock();
    let directory = tempfile::tempdir().expect("temporary directory");
    let value = invoke(
        fake(directory.path(), "value", "printf '{\"ok\":true}\\n'"),
        Duration::from_secs(5),
    );
    assert_eq!(
        value.status,
        ProcessStatus::Exited,
        "value outcome: {value:?}"
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
        Duration::from_secs(5),
    );
    assert_eq!(
        normalize_raw(tq_test_support::compatibility::ToolKind::Jq, &error).error_class,
        Some(ErrorClass::RuntimeTypePath)
    );
}

#[test]
fn controlled_timeout_signal_and_malformed_output_are_distinct() {
    let _lock = fixture_executable_test_lock();
    let directory = tempfile::tempdir().expect("temporary directory");
    let timeout = invoke(
        fake(directory.path(), "timeout", "while :; do :; done"),
        Duration::from_millis(30),
    );
    assert_eq!(timeout.status, ProcessStatus::TimedOut);

    let signal = invoke(
        fake(directory.path(), "signal", "kill -TERM $$"),
        Duration::from_secs(5),
    );
    assert_eq!(signal.status, ProcessStatus::Signaled);

    let malformed = invoke(
        fake(directory.path(), "malformed", "printf 'not-json'"),
        Duration::from_secs(5),
    );
    assert!(normalize_jq(&malformed).is_err());
}
