//! Real-CLI compatibility runner coverage for native output framing.

#![cfg(unix)]

use std::path::Path;

use tq_test_support::compatibility::{
    CompatibilityCase, CompatibilityCatalog, ErrorClass, ExecutableConfig, ObservationState,
    ProcessStatus, ToolKind, encode_hex, run_campaign,
};
use tq_test_support::corpus::ArtifactIdentity;

fn case(id: &str, format: &str, input: &str, query: &str, args: &[String]) -> CompatibilityCase {
    serde_json::from_value(serde_json::json!({
        "schema_version": 1,
        "id": id,
        "title": "runner output framing",
        "classification": "common",
        "capabilities": ["runner.output-mode"],
        "status": "mvp",
        "fixture": {"format": format, "inline": input},
        "query": query,
        "adapters": {"tq": {"supported": true, "args": args}},
        "invocation_mode": "stdin",
        "expected": {"contract": "result-sequence", "baseline": "not-applicable"}
    }))
    .expect("runner regression case")
}

fn run(case: CompatibilityCase) -> tq_test_support::compatibility::CompatibilityReport {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    run_campaign(
        &CompatibilityCatalog {
            cases: vec![case],
            identity: ArtifactIdentity {
                path: "runner-test".to_owned(),
                bytes: 0,
                sha256: String::new(),
            },
        },
        tq_test_support::compatibility::CampaignProfile::Full,
        &ExecutableConfig {
            tq: Some(env!("CARGO_BIN_EXE_tq").into()),
            ..ExecutableConfig::default()
        },
        &root,
        std::time::Duration::from_secs(2),
    )
    .expect("run compatibility campaign")
}

fn tq_observation(
    report: &tq_test_support::compatibility::CompatibilityReport,
) -> &tq_test_support::compatibility::ToolObservation {
    report.cases[0]
        .observations
        .iter()
        .find(|observation| observation.tool == ToolKind::Tq)
        .expect("tq observation")
}

#[test]
fn real_runner_preserves_document_input_for_toon_output_modes() {
    for (format, input) in [("yaml", "a: 1\n"), ("toon", "a: 1\n")] {
        for (mode, extra) in [
            ("default", Vec::new()),
            ("toon", vec!["-o", "toon"]),
            ("toon-seq", vec!["-o", "toon-seq"]),
        ] {
            let mut args = vec!["--input-format".to_owned(), format.to_owned()];
            args.extend(extra.into_iter().map(str::to_owned));
            let report = run(case(
                &format!("runner.real.{format}.{mode}"),
                format,
                input,
                ".",
                &args,
            ));
            let observation = tq_observation(&report);
            assert_eq!(observation.state, ObservationState::Executed);
            assert_eq!(observation.exit_code, Some(0), "{format} {mode}");
            assert_eq!(observation.results, [serde_json::json!({"a": 1})]);
        }
    }
}

#[test]
fn real_runner_keeps_json_lines_output_unframed_for_sequence_input() {
    let report = run(case(
        "runner.real.json-lines-sequence-input",
        "json",
        "\x1e1\n\x1e2\n",
        ".",
        &[
            "--seq".to_owned(),
            "--input-format".to_owned(),
            "json".to_owned(),
            "-o".to_owned(),
            "jsonl".to_owned(),
        ],
    ));
    let observation = tq_observation(&report);
    assert_eq!(observation.state, ObservationState::Executed);
    assert_eq!(observation.exit_code, Some(0));
    assert_eq!(
        observation.results,
        [serde_json::json!(1), serde_json::json!(2)]
    );
    assert_eq!(observation.stdout_hex, Some(encode_hex(b"1\n2\n")));
}

#[test]
fn real_runner_normalizes_unframed_toon_scalar_and_empty_object() {
    for (id, input, expected, stdout) in [
        (
            "runner.real.unframed-scalar",
            "1\n",
            vec![serde_json::json!(1)],
            b"1".as_slice(),
        ),
        (
            "runner.real.unframed-empty-object",
            "{}\n",
            vec![serde_json::json!({})],
            b"".as_slice(),
        ),
    ] {
        let report = run(case(
            id,
            "json",
            input,
            ".",
            &[
                "--input-format".to_owned(),
                "json".to_owned(),
                "-o".to_owned(),
                "toon".to_owned(),
                "--unframed".to_owned(),
            ],
        ));
        let observation = tq_observation(&report);
        assert_eq!(observation.state, ObservationState::Executed, "{id}");
        assert_eq!(observation.exit_code, Some(0), "{id}");
        assert_eq!(observation.results, expected, "{id}");
        assert_eq!(observation.stdout_hex, Some(encode_hex(stdout)), "{id}");
    }
}

#[test]
fn real_runner_preserves_unframed_empty_error_contract() {
    let report = run(case(
        "runner.real.unframed-empty-error",
        "json",
        "null\n",
        "error(\"x\")",
        &[
            "--input-format".to_owned(),
            "json".to_owned(),
            "-o".to_owned(),
            "toon".to_owned(),
            "--unframed".to_owned(),
        ],
    ));
    let observation = tq_observation(&report);
    assert_eq!(observation.state, ObservationState::Executed);
    assert_eq!(observation.results, [] as [serde_json::Value; 0]);
    assert_eq!(observation.exit_code, Some(5));
    assert_eq!(observation.error_class, Some(ErrorClass::RuntimeTypePath));
    assert_eq!(observation.stdout_hex, Some(encode_hex(b"")));
}

#[test]
fn real_runner_rejects_successful_unframed_zero_results() {
    let report = run(case(
        "runner.real.unframed-zero-results",
        "json",
        "null\n",
        "empty",
        &[
            "--input-format".to_owned(),
            "json".to_owned(),
            "-o".to_owned(),
            "toon".to_owned(),
            "--unframed".to_owned(),
        ],
    ));
    assert_eq!(
        tq_observation(&report).state,
        ObservationState::HarnessError
    );
}

#[test]
fn real_runner_uses_tq_json_companion_for_default_toon_multivalues() {
    let report = run(case(
        "runner.real.default-multiple",
        "json",
        "0\n",
        "1, null, false",
        &["--input-format".to_owned(), "json".to_owned()],
    ));
    let observation = tq_observation(&report);
    assert_eq!(observation.state, ObservationState::Executed);
    assert_eq!(
        observation.results,
        [
            serde_json::json!(1),
            serde_json::Value::Null,
            serde_json::json!(false),
        ]
    );
    assert_eq!(
        observation.stdout_hex,
        Some(encode_hex(b"1\nnull\nfalse\n"))
    );
}

#[test]
fn real_runner_uses_tq_json_companion_for_multiline_toon_results() {
    let report = run(case(
        "runner.real.default-multiline",
        "toon",
        "items[2]{name,meta}:\n  Ada,x\n  Bob,y\n",
        ".items[]",
        &["--input-format".to_owned(), "toon".to_owned()],
    ));
    let observation = tq_observation(&report);
    assert_eq!(observation.state, ObservationState::Executed);
    assert_eq!(
        observation.results,
        [
            serde_json::json!({"name": "Ada", "meta": "x"}),
            serde_json::json!({"name": "Bob", "meta": "y"}),
        ]
    );
    assert_eq!(
        observation.stdout_hex,
        Some(encode_hex(b"name: Ada\nmeta: x\nname: Bob\nmeta: y\n"))
    );
}

#[test]
fn real_runner_preserves_nonzero_exit_status_with_toon_companion() {
    let report = run(case(
        "runner.real.exit-status",
        "json",
        "null\n",
        "false",
        &[
            "--input-format".to_owned(),
            "json".to_owned(),
            "-e".to_owned(),
        ],
    ));
    let observation = tq_observation(&report);
    assert_eq!(observation.state, ObservationState::Executed);
    assert_eq!(observation.results, [serde_json::json!(false)]);
    assert_eq!(observation.process_status, Some(ProcessStatus::Exited));
    assert_eq!(observation.exit_code, Some(1));
    assert_eq!(observation.error_class, Some(ErrorClass::RuntimeTypePath));
}

#[test]
fn real_runner_preserves_partial_results_and_error_status_with_toon_companion() {
    let report = run(case(
        "runner.real.partial-error",
        "json",
        "null\n",
        "1,2,error(\"x\")",
        &["--input-format".to_owned(), "json".to_owned()],
    ));
    let observation = tq_observation(&report);
    assert_eq!(observation.state, ObservationState::Executed);
    assert_eq!(
        observation.results,
        [serde_json::json!(1), serde_json::json!(2)]
    );
    assert_eq!(observation.process_status, Some(ProcessStatus::Exited));
    assert_eq!(observation.exit_code, Some(5));
    assert_eq!(observation.error_class, Some(ErrorClass::RuntimeTypePath));
}
