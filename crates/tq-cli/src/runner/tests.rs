use std::{
    fs,
    io::{self, Cursor, Read, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::sync_channel,
    },
    thread,
    time::Duration,
};

use super::{
    automatic::prune_object_fields_in_place,
    query::{AmbientModuleRoots, module_roots_with_ambient},
    record_document_observations, run_with_io,
    test_file::{
        RunTestCapture, RunTestCaptureSnapshot, compile_diagnostic_matches, parse_run_test_cases,
        run_test_output_matches, run_tests_self_checks,
    },
    transcode::map_transcode_error,
};
use crate::{CapabilityPolicy, Command, ExecutionOverride, ExitStatus, parse_args};
use tq_core::{PathComponent, Value, VmObservations, parse_bytes};

struct NoRead;

impl Read for NoRead {
    fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
        panic!("compile failure must not read input")
    }
}

fn execute(
    arguments: &[&str],
    input: &[u8],
) -> (Result<ExitStatus, super::RunError>, Vec<u8>, Vec<u8>) {
    let command = parse_args(arguments.iter().copied()).unwrap();
    let mut stdin = input;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let status = run_with_io(command, &mut stdin, &mut stdout, &mut stderr);
    (status, stdout, stderr)
}

fn assert_single_runtime_diagnostic(stderr: &[u8]) {
    let stderr = std::str::from_utf8(stderr).expect("runtime diagnostic is UTF-8");
    assert!(stderr.ends_with('\n'));
    let mut lines = stderr.lines();
    let diagnostic = lines.next().expect("runtime diagnostic is non-empty");
    assert!(lines.next().is_none(), "expected one diagnostic line");
    assert!(diagnostic.starts_with("tq: runtime error:"));
}

fn observed_exit_status(status: &Result<ExitStatus, super::RunError>) -> ExitStatus {
    status
        .as_ref()
        .map_or_else(super::RunError::status, |status| *status)
}

fn observation_options(report: bool, stream: bool) -> super::RunOptions {
    let command = parse_args(["."]).expect("baseline options parse");
    let Command::Run(mut options) = command else {
        panic!("expected run command")
    };
    options.report_file = report.then(|| std::path::PathBuf::from("unit-test-report.json"));
    options.stream = stream;
    *options
}

#[test]
fn recursive_schema_resource_errors_keep_cli_resource_classification() {
    let writer = tq_toon::TranscodeError::Writer(tq_toon::WriterError::Schema {
        resource: "field count",
        limit: 1,
    });
    assert!(matches!(
        map_transcode_error(writer),
        super::RunError::Resource("fields")
    ));

    let spool =
        tq_toon::TranscodeError::Spool(tq_toon::SpoolError::Writer(tq_toon::WriterError::Schema {
            resource: "retained bytes",
            limit: 1,
        }));
    assert!(matches!(
        map_transcode_error(spool),
        super::RunError::Resource("preparation-memory")
    ));
}

#[test]
fn document_observations_are_retained_only_for_reports() {
    let item = VmObservations {
        value_stack_high_water: 2,
        call_stack_high_water: 3,
        path_stack_high_water: 4,
        fork_stack_high_water: 5,
        steps: 6,
        results: 7,
    };
    let mut observations = Vec::new();
    record_document_observations(&observation_options(false, false), &mut observations, item);
    assert_eq!(observations, [] as [tq_core::VmObservations; 0]);

    let mut observations = Vec::new();
    record_document_observations(&observation_options(true, false), &mut observations, item);
    record_document_observations(
        &observation_options(true, false),
        &mut observations,
        VmObservations {
            steps: 8,
            results: 9,
            ..VmObservations::default()
        },
    );
    assert_eq!(
        observations,
        vec![
            item,
            VmObservations {
                steps: 8,
                results: 9,
                ..VmObservations::default()
            }
        ]
    );
}

#[test]
fn streamed_report_observations_merge_work_and_high_water_fields() {
    let mut observations = vec![VmObservations {
        value_stack_high_water: 2,
        call_stack_high_water: 3,
        path_stack_high_water: 4,
        fork_stack_high_water: 5,
        steps: 6,
        results: 7,
    }];
    record_document_observations(
        &observation_options(true, true),
        &mut observations,
        VmObservations {
            value_stack_high_water: 8,
            call_stack_high_water: 1,
            path_stack_high_water: 9,
            fork_stack_high_water: 0,
            steps: 10,
            results: 11,
        },
    );
    assert_eq!(
        observations,
        vec![VmObservations {
            value_stack_high_water: 8,
            call_stack_high_water: 3,
            path_stack_high_water: 9,
            fork_stack_high_water: 5,
            steps: 16,
            results: 18,
        }]
    );
}

fn execute_with_override(
    arguments: &[&str],
    input: &[u8],
    execution_override: ExecutionOverride,
) -> (Result<ExitStatus, super::RunError>, Vec<u8>, Vec<u8>) {
    let mut command = parse_args(arguments.iter().copied()).unwrap();
    let Command::Run(options) = &mut command else {
        panic!("expected run command")
    };
    options.execution_override = execution_override;
    let mut stdin = input;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let status = run_with_io(command, &mut stdin, &mut stdout, &mut stderr);
    (status, stdout, stderr)
}

#[test]
fn run_test_matcher_requires_typed_capture() {
    let expected = vec!["null".to_owned()];
    assert!(!run_test_output_matches(b"null\n", &expected, None));
}

#[test]
fn run_test_matcher_rejects_overflowed_capture() {
    let expected = vec!["null".to_owned()];
    let capture = RunTestCaptureSnapshot {
        values: vec![Value::Null],
        overflowed: true,
    };
    assert!(!run_test_output_matches(
        b"null\n",
        &expected,
        Some(&capture)
    ));
}

#[test]
fn run_test_capture_enforces_result_count_bound() {
    let mut capture = RunTestCapture::new(0);
    capture.record(&Value::Null);
    capture.record(&Value::Null);
    let capture = capture.snapshot();
    assert!(capture.overflowed);
    assert_eq!(capture.values, vec![Value::Null]);
}

#[test]
fn run_test_matcher_accepts_a_captured_compact_value() {
    let expected = vec!["null".to_owned()];
    let capture = RunTestCaptureSnapshot {
        values: vec![Value::Null],
        overflowed: false,
    };
    assert!(run_test_output_matches(
        b"null\n",
        &expected,
        Some(&capture)
    ));
}

#[test]
fn object_capture_pruning_is_in_place_and_keeps_required_order() {
    let mut properties = tq_core::Object::new();
    properties.insert("mag".into(), Value::string("3"));
    properties.insert("unused".into(), Value::string("drop"));
    let mut object = tq_core::Object::new();
    object.insert("id".into(), Value::string("feature"));
    object.insert("properties".into(), Value::object(properties));
    object.insert("unused".into(), Value::string("drop"));
    let mut value = Value::object(object);
    let root_ptr = match &value {
        Value::Object(values) => Arc::as_ptr(values),
        _ => unreachable!("test value is an object"),
    };
    let paths = vec![
        vec![
            PathComponent::Key("properties".into()),
            PathComponent::Key("mag".into()),
        ],
        vec![PathComponent::Key("id".into())],
    ];

    prune_object_fields_in_place(&mut value, &paths);

    let Value::Object(values) = &value else {
        unreachable!("pruned value is an object")
    };
    assert_eq!(Arc::as_ptr(values), root_ptr);
    assert_eq!(
        values.keys().map(AsRef::as_ref).collect::<Vec<&str>>(),
        ["id", "properties"]
    );
    let Value::Object(properties) = values.get("properties").expect("properties retained") else {
        unreachable!("properties remains an object")
    };
    assert_eq!(
        properties.keys().map(AsRef::as_ref).collect::<Vec<&str>>(),
        ["mag"]
    );

    let shared = value.clone();
    let mut shared_copy = value;
    prune_object_fields_in_place(&mut shared_copy, &[vec![PathComponent::Key("id".into())]]);
    let Value::Object(values) = &shared_copy else {
        unreachable!("shared value is an object")
    };
    assert!(values.contains_key("properties"));
    assert!(shared.shares_node_with(&shared_copy));
}

#[test]
fn object_capture_pruning_keeps_sibling_paths_separate() {
    let mut left = tq_core::Object::new();
    left.insert("x".into(), Value::string("left-x"));
    left.insert("y".into(), Value::string("left-y"));
    let mut right = tq_core::Object::new();
    right.insert("x".into(), Value::string("right-x"));
    right.insert("y".into(), Value::string("right-y"));
    let mut object = tq_core::Object::new();
    object.insert("a".into(), Value::object(left));
    object.insert("b".into(), Value::object(right));
    let mut value = Value::object(object);

    prune_object_fields_in_place(
        &mut value,
        &[
            vec![
                PathComponent::Key("a".into()),
                PathComponent::Key("x".into()),
            ],
            vec![
                PathComponent::Key("b".into()),
                PathComponent::Key("y".into()),
            ],
        ],
    );

    let Value::Object(values) = &value else {
        unreachable!("pruned value is an object")
    };
    let Value::Object(left) = values.get("a").expect("left branch retained") else {
        unreachable!("left branch remains an object")
    };
    let Value::Object(right) = values.get("b").expect("right branch retained") else {
        unreachable!("right branch remains an object")
    };
    assert_eq!(left.keys().map(AsRef::as_ref).collect::<Vec<&str>>(), ["x"]);
    assert_eq!(
        right.keys().map(AsRef::as_ref).collect::<Vec<&str>>(),
        ["y"]
    );

    let mut complete_left = tq_core::Object::new();
    complete_left.insert("x".into(), Value::string("left-x"));
    complete_left.insert("y".into(), Value::string("left-y"));
    let mut complete_root = tq_core::Object::new();
    complete_root.insert("a".into(), Value::object(complete_left));
    complete_root.insert("b".into(), Value::string("remove"));
    let mut complete = Value::object(complete_root);
    prune_object_fields_in_place(
        &mut complete,
        &[
            vec![PathComponent::Key("a".into())],
            vec![
                PathComponent::Key("a".into()),
                PathComponent::Key("x".into()),
            ],
        ],
    );
    let Value::Object(values) = complete else {
        unreachable!("terminal-path value is an object")
    };
    let Value::Object(left) = values.get("a").expect("terminal branch retained") else {
        unreachable!("terminal branch remains an object")
    };
    assert_eq!(
        left.keys().map(AsRef::as_ref).collect::<Vec<&str>>(),
        ["x", "y"]
    );
}

#[test]
fn user_function_executes_inside_map_before_cli_output() {
    let (status, output, error) = execute(
        &[
            "--input-format",
            "json",
            "--output-format",
            "json",
            "--compact-output",
            "def f: . + 1; map(f)",
        ],
        b"[1,2]\n",
    );
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(output, b"[2,3]\n");
    assert_eq!(error, [] as [u8; 0]);
}

#[test]
fn json5_input_runs_explicitly_and_by_extension_without_weakening_json() {
    let (status, output, error) = execute(
        &[
            "--input-format",
            "json5",
            "--output-format",
            "json",
            "--compact-output",
            "--explain-json",
            ".value",
        ],
        b"{value: 'ok',}",
    );
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(output, b"\"ok\"\n");
    let explanation: serde_json::Value = serde_json::from_slice(&error).unwrap();
    assert_eq!(explanation["execution"]["plan"], "document");

    let directory = tempfile::tempdir().unwrap();
    let json5 = directory.path().join("first.JSON5");
    let json = directory.path().join("second.json");
    let permissive_json = directory.path().join("strict.json");
    fs::write(&json5, b"{value: 1,}").unwrap();
    fs::write(&json, br#"{"value":2}"#).unwrap();
    fs::write(&permissive_json, b"{value: 3,}").unwrap();

    let paths = [json5.to_str().unwrap(), json.to_str().unwrap()];
    let (status, output, error) = execute(
        &["--output-format", "jsonl", ".value", paths[0], paths[1]],
        b"",
    );
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(output, b"1\n2\n");
    assert_eq!(error, [] as [u8; 0]);

    let strict_path = permissive_json.to_str().unwrap();
    let (status, output, _) = execute(&[".", strict_path], b"");
    assert_eq!(status.unwrap_err().status(), ExitStatus::Input);
    assert_eq!(output, [] as [u8; 0]);

    let (status, output, error) = execute(
        &[
            "--input-format",
            "json5",
            "--output-format",
            "json",
            "--compact-output",
            ".value",
            strict_path,
        ],
        b"",
    );
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(output, b"3\n");
    assert_eq!(error, [] as [u8; 0]);
}

#[test]
fn json5_extension_rejects_stream_mode_before_opening_input() {
    let directory = tempfile::tempdir().unwrap();
    let missing = directory.path().join("missing.json5");
    let command = parse_args(["--stream", ".", missing.to_str().unwrap()]).unwrap();
    let mut input = NoRead;
    let mut output = Vec::new();
    let mut error = Vec::new();

    let failure = run_with_io(command, &mut input, &mut output, &mut error).unwrap_err();

    assert_eq!(failure.status(), ExitStatus::Unsupported);
    assert!(failure.to_string().contains("JSON5"));
    assert!(failure.to_string().contains("document-at-a-time"));
    assert_eq!(output, [] as [u8; 0]);
}

#[test]
fn non_strict_toon_stream_projects_last_write_wins_documents() {
    let (status, output, error) = execute(
        &[
            "--stream",
            "--non-strict",
            "--output-format",
            "json",
            "--compact-output",
            ".",
        ],
        b"a: 1\na: 2",
    );
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(output, b"[[\"a\"],2]\n[[\"a\"]]\n");
    assert_eq!(error, [] as [u8; 0]);
}

#[test]
fn non_strict_keyed_toon_and_toon_sequence_streams_project_staged_documents() {
    for (options, input) in [
        (
            &[
                "--stream",
                "--non-strict",
                "--input-format",
                "toon",
                "--output-format",
                "json",
                "--compact-output",
                ".",
            ][..],
            b"m[2:]{v}:\n  a: 1\n  a: 2".as_slice(),
        ),
        (
            &[
                "--stream",
                "--non-strict",
                "--toon-sequence-input",
                "--output-format",
                "json",
                "--compact-output",
                ".",
            ][..],
            b"\x1em[2:]{v}:\n  a: 1\n  a: 2\n".as_slice(),
        ),
    ] {
        let (status, output, error) = execute(options, input);
        assert_eq!(status.unwrap(), ExitStatus::Success);
        assert_eq!(
            output,
            b"[[\"m\",\"a\",\"v\"],2]\n[[\"m\",\"a\",\"v\"]]\n[[\"m\",\"a\"]]\n[[\"m\"]]\n"
        );
        assert_eq!(error, [] as [u8; 0]);
    }
}

#[test]
fn unknown_filter_fails_before_input() {
    let command = parse_args([
        "--input-format",
        "json",
        "--output-format",
        "json",
        "def f: .; .[0:1] | unknown_filter",
    ])
    .unwrap();
    let mut input = NoRead;
    let mut output = Vec::new();
    let mut error = Vec::new();
    let failure = run_with_io(command, &mut input, &mut output, &mut error).unwrap_err();
    assert_eq!(failure.status(), ExitStatus::Compile);
    assert!(failure.to_string().contains("TQ-RESOLVE-BUILTIN-001"));
    assert!(
        failure
            .to_string()
            .contains("unknown filter unknown_filter/0")
    );
    assert_eq!(output, [] as [u8; 0]);
}

#[test]
fn executable_user_function_closure_runs_after_slice() {
    let (status, output, error) = execute(
        &[
            "--input-format",
            "json",
            "--output-format",
            "json",
            "--compact-output",
            "def f: .; .[0:1] | f",
        ],
        b"[1,2]\n",
    );
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(output, b"[1]\n");
    assert_eq!(error, [] as [u8; 0]);
}

#[test]
fn generator_bound_report_input_preserves_cli_assignment_path() {
    let (status, output, error) = execute(
        &[
            "--input-format",
            "json",
            "--output-format",
            "json",
            "--compact-output",
            r". as $approvals
                | input as $report
                | $approvals
                | map(
                    . as $approval
                    | ($report.cases[] | select(.id == $approval.case_id)) as $case
                    | .evidence = {x: 1}
                  )",
        ],
        b"[{\"case_id\":\"x\"}]\n{\"cases\":[{\"id\":\"x\"}]}\n",
    );
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(output, b"[{\"case_id\":\"x\",\"evidence\":{\"x\":1}}]\n");
    assert_eq!(error, [] as [u8; 0]);
}

#[test]
fn generator_bound_report_input_preserves_cli_slice_and_filter_bindings() {
    let (status, output, error) = execute(
        &[
            "--input-format",
            "json",
            "--output-format",
            "json",
            "--compact-output",
            r". as $approvals
                   | input as $report
                   | def find(predicate):
                       $report.cases[0:1][] | select(predicate)
                     ;
                   $approvals
                   | map(
                       . as $approval
                       | find(.id == $approval.case_id) as $case
                       | .evidence = $case.value
                     )",
        ],
        b"[{\"case_id\":\"x\"}]\n{\"cases\":[{\"id\":\"x\",\"value\":1}]}\n",
    );
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(output, b"[{\"case_id\":\"x\",\"evidence\":1}]\n");
    assert_eq!(error, [] as [u8; 0]);
}

#[test]
fn internal_override_forces_document_plan_without_changing_output() {
    let arguments = ["--output-format", "toon-seq", "--explain-json", ".[]"];
    let (_, automatic_output, automatic_explain) = execute(&arguments, b"[1,2]");

    let mut command = parse_args(arguments).unwrap();
    let Command::Run(options) = &mut command else {
        panic!("expected run command")
    };
    options.execution_override = ExecutionOverride::Document;
    let mut input = &b"[1,2]"[..];
    let mut forced_output = Vec::new();
    let mut forced_explain = Vec::new();
    assert_eq!(
        run_with_io(command, &mut input, &mut forced_output, &mut forced_explain).unwrap(),
        ExitStatus::Success
    );

    assert_eq!(forced_output, automatic_output);
    let automatic: serde_json::Value = serde_json::from_slice(&automatic_explain).unwrap();
    let forced: serde_json::Value = serde_json::from_slice(&forced_explain).unwrap();
    assert_eq!(automatic["execution"]["plan"], "subtree");
    assert_eq!(forced["execution"]["plan"], "document");
    assert_eq!(
        forced["execution"]["stream_rejection"],
        "forced document override for differential run"
    );
}

#[test]
fn identity_json_transcode_rejects_a_late_duplicate_without_partial_output() {
    let (status, output, explain) = execute(
        &["--input-format", "json", "--explain-json", "."],
        br#"{"b":1,"a":2,"b":3}"#,
    );
    assert_eq!(status.unwrap_err().status(), ExitStatus::Input);
    assert_eq!(output, [] as [u8; 0]);
    let explain: serde_json::Value = serde_json::from_slice(&explain).unwrap();
    assert_eq!(explain["execution"]["plan"], "transcode");
    assert_eq!(explain["execution"]["duplicate_policy"], "reject");
    assert_eq!(explain["execution"]["commitment_mode"], "direct-values");
}

#[test]
fn transcode_streams_ordered_files_and_keeps_unframed_output_atomic() {
    let directory = tempfile::tempdir().unwrap();
    let first = directory.path().join("first.json");
    let second = directory.path().join("second.json");
    fs::write(&first, b"1").unwrap();
    fs::write(&second, b"2").unwrap();
    let command = parse_args([
        "--output-format",
        "toon-seq",
        ".",
        first.to_str().unwrap(),
        second.to_str().unwrap(),
    ])
    .unwrap();
    let mut input = &[][..];
    let mut output = Vec::new();
    let mut error = Vec::new();
    assert_eq!(
        run_with_io(command, &mut input, &mut output, &mut error).unwrap(),
        ExitStatus::Success
    );
    assert_eq!(output, b"\x1e1\n\x1e2\n");
    assert_eq!(error, [] as [u8; 0]);

    let (status, output, error) = execute(&["--input-format", "json", "--unframed", "."], b"1 2");
    assert!(matches!(status, Err(super::RunError::Cardinality(_))));
    assert_eq!(output, [] as [u8; 0]);
    assert_eq!(error, [] as [u8; 0]);
}

#[test]
fn auto_detected_identity_json_uses_transcode_and_reports_observations() {
    let directory = tempfile::tempdir().unwrap();
    let report = directory.path().join("transcode.json");
    let command = parse_args([
        "--explain-json",
        "--report-file",
        report.to_str().unwrap(),
        ".",
    ])
    .unwrap();
    let mut input = br"[1,2,3]".as_slice();
    let mut output = Vec::new();
    let mut error = Vec::new();
    assert_eq!(
        run_with_io(command, &mut input, &mut output, &mut error).unwrap(),
        ExitStatus::Success
    );
    let explain: serde_json::Value = serde_json::from_slice(&error).unwrap();
    assert_eq!(explain["execution"]["plan"], "transcode");
    let report: serde_json::Value = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
    assert_eq!(report["execution"]["plan"], "transcode");
    assert_eq!(report["execution"]["array_preparations"], 1);
    assert_eq!(report["execution"]["resource_outcome"], "success");
    assert!(
        report["execution"]["preparation_high_water_bytes"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert_eq!(report["execution"]["spool_bytes_written"], 0);
    assert_eq!(report["execution"]["input_stage_bytes_written"], 0);
    assert_eq!(report["execution"]["input_stage_bytes_replayed"], 0);
    assert_eq!(report["execution"]["selected_input_format"], "json");
    assert_eq!(report["execution"]["input_format_source"], "detected");
    assert_eq!(report["execution"]["detection"][0]["identity"], "<stdin>");
    assert!(report["execution"]["detection"][0]["lookahead_bytes"].is_u64());
    assert!(report["execution"]["detection"][0]["commitment_bytes"].is_u64());
}

#[test]
fn transcode_reports_resource_failure_and_output_aware_fallbacks() {
    let directory = tempfile::tempdir().unwrap();
    let report = directory.path().join("limited.json");
    let command = parse_args([
        "--input-format",
        "json",
        "--prepare-memory-bytes",
        "0",
        "--max-spool-bytes",
        "1",
        "--report-file",
        report.to_str().unwrap(),
        ".",
    ])
    .unwrap();
    let mut input = br#"["too large"]"#.as_slice();
    let mut output = Vec::new();
    let mut error = Vec::new();
    assert_eq!(
        run_with_io(command, &mut input, &mut output, &mut error)
            .unwrap_err()
            .status(),
        ExitStatus::Resource
    );
    let report: serde_json::Value = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
    assert_eq!(report["execution"]["resource_outcome"], "resource-limit");

    let command = parse_args([
        "--input-format",
        "json",
        "--output-format",
        "json",
        "--max-output-bytes",
        "1",
        ".",
    ])
    .unwrap();
    let mut input = b"10".as_slice();
    let mut output = Vec::new();
    let mut error = Vec::new();
    assert_eq!(
        run_with_io(command, &mut input, &mut output, &mut error)
            .unwrap_err()
            .status(),
        ExitStatus::Resource
    );
    assert_eq!(output, [] as [u8; 0]);

    for arguments in [
        ["--input-format", "json", "--output-format", "json", "."],
        [
            "--input-format",
            "json",
            "--sort-keys",
            "--explain-json",
            ".",
        ],
    ] {
        let (status, _, explain) = execute(&arguments, br#"{"x":1}"#);
        assert_eq!(status.unwrap(), ExitStatus::Success);
        if arguments.contains(&"--explain-json") {
            let explain: serde_json::Value = serde_json::from_slice(&explain).unwrap();
            assert_eq!(explain["execution"]["plan"], "document");
            assert!(explain["execution"]["transcode_rejection"].is_string());
        }
    }
}

#[test]
fn ambient_builtins_are_denied_by_default_and_admitted_explicitly() {
    let (denied, output, error) = execute(&["--output-format", "json", "-c", "env"], b"null\n");
    assert!(denied.is_err());
    assert_eq!(output, [] as [u8; 0]);
    let error = String::from_utf8(error).expect("UTF-8 stderr");
    assert_eq!(error, "", "run_with_io returns errors to its caller");

    for query in ["now", "input_filename"] {
        let (denied, output, error) = execute(&["--output-format", "json", "-c", query], b"null\n");
        let denied = denied.expect_err("platform access should be denied by default");
        assert_eq!(denied.status(), ExitStatus::Runtime);
        assert!(denied.to_string().contains("capability policy"));
        assert_eq!(output, [] as [u8; 0]);
        assert_eq!(error, [] as [u8; 0]);
    }

    let (allowed, output, error) = execute(
        &[
            "--allow-environment",
            "--output-format",
            "json",
            "-c",
            "env | type",
        ],
        b"null\n",
    );
    assert_eq!(allowed.unwrap(), ExitStatus::Success);
    assert_eq!(output, b"\"object\"\n");
    assert_eq!(error, [] as [u8; 0]);

    let (allowed_variable, output, error) = execute(
        &[
            "--allow-environment",
            "--output-format",
            "json",
            "-c",
            "$ENV | type",
        ],
        b"null\n",
    );
    assert_eq!(allowed_variable.unwrap(), ExitStatus::Success);
    assert_eq!(output, b"\"object\"\n");
    assert_eq!(error, [] as [u8; 0]);

    let (platform, output, error) = execute(
        &[
            "--allow-platform",
            "--output-format",
            "json",
            "-c",
            "[input_filename, input_line_number, (now | type)]",
        ],
        b"null\n",
    );
    assert_eq!(platform.unwrap(), ExitStatus::Success);
    assert_eq!(output, b"[\"<stdin>\",1,\"number\"]\n");
    assert_eq!(error, [] as [u8; 0]);
}

#[test]
fn jq_special_variables_follow_cli_source_and_capability_contracts() {
    let (denied, output, error) = execute(&["--output-format", "json", "-c", "$ENV"], b"null\n");
    let denied = denied.expect_err("environment access should be denied by default");
    assert_eq!(denied.status(), ExitStatus::Runtime);
    assert!(denied.to_string().contains("capability policy"));
    assert!(!denied.to_string().contains("__tq_"));
    assert_eq!(output, [] as [u8; 0]);
    assert_eq!(error, [] as [u8; 0]);

    let (allowed, output, error) = execute(
        &[
            "--allow-environment",
            "--arg",
            "ENV",
            "replacement",
            "--output-format",
            "json",
            "-c",
            "$ENV | type",
        ],
        b"null\n",
    );
    assert_eq!(allowed.unwrap(), ExitStatus::Success);
    assert_eq!(output, b"\"object\"\n");
    assert_eq!(error, [] as [u8; 0]);

    let (location, output, error) = execute(
        &[
            "--arg",
            "__loc__",
            "replacement",
            "--output-format",
            "json",
            "-c",
            "$__loc__",
        ],
        b"null\n",
    );
    assert_eq!(location.unwrap(), ExitStatus::Success);
    assert_eq!(output, b"{\"file\":\"<top-level>\",\"line\":1}\n");
    assert_eq!(error, [] as [u8; 0]);

    let directory = tempfile::tempdir().unwrap();
    let filter = directory.path().join("location.jq");
    fs::write(&filter, b"\n$__loc__\n").unwrap();
    let (location, output, error) = execute(
        &[
            "--null-input",
            "--output-format",
            "json",
            "-c",
            "--from-file",
            filter.to_str().unwrap(),
        ],
        b"",
    );
    assert_eq!(location.unwrap(), ExitStatus::Success);
    let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(value["file"], filter.display().to_string());
    assert_eq!(value["line"], 2);
    assert_eq!(error, [] as [u8; 0]);

    let module = directory.path().join("location.jq");
    fs::write(&module, b"def module_location:\n  $__loc__;\n").unwrap();
    let (location, output, error) = execute(
        &[
            "--null-input",
            "--library-path",
            directory.path().to_str().unwrap(),
            "--output-format",
            "json",
            "-c",
            "include \"location\"; module_location",
        ],
        b"",
    );
    assert_eq!(location.unwrap(), ExitStatus::Success);
    let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(
        value["file"],
        module.canonicalize().unwrap().display().to_string()
    );
    assert_eq!(value["line"], 2);
    assert_eq!(error, [] as [u8; 0]);

    let (stable, output, error) = execute(
        &[
            "--allow-environment",
            "--input-format",
            "json",
            "--output-format",
            "json",
            "-c",
            "[$__loc__, ($ENV | type)]",
        ],
        b"null null",
    );
    assert_eq!(stable.unwrap(), ExitStatus::Success);
    assert_eq!(
            output,
            b"[{\"file\":\"<top-level>\",\"line\":1},\"object\"]\n[{\"file\":\"<top-level>\",\"line\":1},\"object\"]\n"
        );
    assert_eq!(error, [] as [u8; 0]);
}

#[test]
fn jq_special_variables_match_automatic_and_document_plans() {
    let arguments = [
        "--allow-environment",
        "--input-format",
        "json",
        "--output-format",
        "json",
        "-c",
        ".items[] | [$__loc__, ($ENV | type)]",
    ];
    let input = br#"{"items":[null,null]}"#;
    let automatic = execute_with_override(&arguments, input, ExecutionOverride::Automatic);
    let document = execute_with_override(&arguments, input, ExecutionOverride::Document);
    assert_eq!(automatic.0.unwrap(), ExitStatus::Success);
    assert_eq!(document.0.unwrap(), ExitStatus::Success);
    assert_eq!(automatic.1, document.1);
    assert_eq!(
            automatic.1,
            b"[{\"file\":\"<top-level>\",\"line\":1},\"object\"]\n[{\"file\":\"<top-level>\",\"line\":1},\"object\"]\n"
        );
    assert_eq!(automatic.2, [] as [u8; 0]);
    assert_eq!(document.2, [] as [u8; 0]);

    let denied_arguments = [
        "--input-format",
        "json",
        "--output-format",
        "json",
        "-c",
        ".items[] | $ENV",
    ];
    let automatic = execute_with_override(&denied_arguments, input, ExecutionOverride::Automatic);
    let document = execute_with_override(&denied_arguments, input, ExecutionOverride::Document);
    assert_eq!(
        automatic.0.unwrap_err().status(),
        document.0.unwrap_err().status()
    );
    assert_eq!(automatic.1, document.1);
    assert_eq!(automatic.2, [] as [u8; 0]);
    assert_eq!(document.2, [] as [u8; 0]);
}

#[test]
fn external_arguments_cannot_supply_internal_ambient_values() {
    let (status, output, error) = execute(
        &[
            "--argjson",
            "__tq_ambient_environment",
            r#"{"FORGED":"value"}"#,
            "--output-format",
            "json",
            "-c",
            "$ENV",
        ],
        b"null\n",
    );
    let failure = status.expect_err("internal argument names must be rejected");
    assert_ne!(failure.status(), ExitStatus::Success);
    assert_eq!(output, [] as [u8; 0]);
    assert_eq!(error, [] as [u8; 0]);
}

#[test]
fn environment_policy_denial_precedes_input_for_env_variable() {
    let mut command = parse_args(["--allow-environment", "$ENV"]).unwrap();
    let Command::Run(options) = &mut command else {
        panic!("expected run command")
    };
    options.capability_policy = CapabilityPolicy {
        environment: false,
        ..CapabilityPolicy::default()
    };
    let mut input = NoRead;
    let mut output = Vec::new();
    let mut error = Vec::new();
    let failure = run_with_io(command, &mut input, &mut output, &mut error).unwrap_err();
    assert_eq!(failure.status(), ExitStatus::Usage);
    assert!(
        failure
            .to_string()
            .contains("environment access is disabled")
    );
    assert_eq!(output, [] as [u8; 0]);
    assert_eq!(error, [] as [u8; 0]);
}

#[test]
fn embedded_run_tests_keeps_ambient_policy_confined() {
    let command = Command::RunTests(None);
    let mut input = b"$ENV.TQ_EMBEDDED_RUN_TEST_SENTINEL\nnull\n\"present\"\n\n".as_slice();
    let mut output = Vec::new();
    let mut error = Vec::new();
    let status = run_with_io(command, &mut input, &mut output, &mut error).unwrap();
    assert_eq!(status, ExitStatus::FalseOrNull);
    assert!(String::from_utf8_lossy(&output).contains("0 of 1 tests passed"));
    assert!(String::from_utf8_lossy(&output).contains("Expected \"present\""));
    assert_eq!(error, [] as [u8; 0]);
}

#[test]
fn denied_filesystem_cannot_use_implicit_module_or_startup_roots() {
    for policy in [
        crate::CapabilityPolicy {
            filesystem: false,
            ..crate::CapabilityPolicy::default()
        },
        crate::CapabilityPolicy {
            filesystem: false,
            environment: false,
            terminal: false,
            platform: false,
        },
    ] {
        let mut command = parse_args([
            "-n",
            "--output-format",
            "json",
            r#"import "basic" as b; b::value"#,
        ])
        .expect("parse implicit module policy query");
        let Command::Run(options) = &mut command else {
            panic!("expected run command")
        };
        options.capability_policy = policy;
        let mut input = Cursor::new(Vec::<u8>::new());
        let mut output = Vec::new();
        let mut error = Vec::new();
        let result = run_with_io(command, &mut input, &mut output, &mut error)
            .expect_err("implicit module roots must be denied");
        assert!(result.to_string().contains("TQ-MODULE-ROOT-001"));
        assert_eq!(output, [] as [u8; 0]);
        assert_eq!(error, [] as [u8; 0]);
    }
}

#[test]
fn denied_environment_cannot_supply_ambient_module_roots_but_explicit_l_remains() {
    let home = tempfile::tempdir().expect("ambient home creates");
    fs::create_dir(home.path().join(".jq")).expect("ambient home module root creates");
    let library = tempfile::tempdir().expect("ambient library creates");
    let ambient = AmbientModuleRoots {
        home: Some(home.path().to_path_buf()),
        library_path: Some(library.path().as_os_str().to_owned()),
    };

    let mut denied_command = parse_args(["-n", "."]).expect("denied module roots parse");
    let Command::Run(denied_options) = &mut denied_command else {
        panic!("expected run command")
    };
    assert!(!denied_options.allow_environment);
    let denied_roots = module_roots_with_ambient(denied_options, "<command-line>", Some(ambient));
    assert!(!denied_roots.contains(&home.path().join(".jq")));
    assert!(!denied_roots.contains(&library.path().to_path_buf()));

    let mut enabled_command = parse_args(["-n", "."]).expect("enabled module roots parse");
    let Command::Run(enabled_options) = &mut enabled_command else {
        panic!("expected run command")
    };
    enabled_options.allow_environment = true;
    let enabled_roots = module_roots_with_ambient(
        enabled_options,
        "<command-line>",
        Some(AmbientModuleRoots {
            home: Some(home.path().to_path_buf()),
            library_path: Some(library.path().as_os_str().to_owned()),
        }),
    );
    assert!(enabled_roots.contains(&home.path().join(".jq")));
    assert!(enabled_roots.contains(&library.path().to_path_buf()));

    let explicit = tempfile::tempdir().expect("explicit module root creates");
    fs::write(explicit.path().join("answer.jq"), "def answer: 42;")
        .expect("explicit module writes");
    let explicit_path = explicit.path().to_path_buf();
    let mut explicit_command = parse_args([
        "-L",
        explicit_path.to_str().expect("explicit root is UTF-8"),
        "-n",
        ".",
    ])
    .expect("explicit module roots parse");
    let Command::Run(explicit_options) = &mut explicit_command else {
        panic!("expected run command")
    };
    explicit_options.capability_policy = CapabilityPolicy {
        environment: false,
        ..CapabilityPolicy::default()
    };
    let explicit_roots = module_roots_with_ambient(
        explicit_options,
        "<command-line>",
        Some(AmbientModuleRoots::default()),
    );
    assert_eq!(explicit_roots, vec![explicit_path.clone()]);

    let mut explicit_run = parse_args([
        "-L",
        explicit_path.to_str().expect("explicit root is UTF-8"),
        "-n",
        "--output-format",
        "json",
        "-c",
        "include \"answer\"; answer",
    ])
    .expect("explicit module run parses");
    let Command::Run(explicit_options) = &mut explicit_run else {
        panic!("expected run command")
    };
    explicit_options.capability_policy = CapabilityPolicy {
        environment: false,
        ..CapabilityPolicy::default()
    };
    let mut input = &[][..];
    let mut output = Vec::new();
    let mut error = Vec::new();
    assert_eq!(
        run_with_io(explicit_run, &mut input, &mut output, &mut error).unwrap(),
        ExitStatus::Success
    );
    assert_eq!(output, b"42\n");
    assert_eq!(error, [] as [u8; 0]);
}

#[derive(Default)]
struct FlushWriter {
    bytes: Vec<u8>,
    flush_points: Vec<usize>,
}

impl Write for FlushWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush_points.push(self.bytes.len());
        Ok(())
    }
}

struct FailingFlushWriter {
    bytes: Vec<u8>,
}

impl Write for FailingFlushWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("injected flush failure"))
    }
}

struct OpenReader {
    entered: Arc<AtomicBool>,
    release: Arc<AtomicBool>,
}

impl Read for OpenReader {
    fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
        self.entered.store(true, Ordering::Release);
        while !self.release.load(Ordering::Acquire) {
            thread::sleep(Duration::from_millis(1));
        }
        Ok(0)
    }
}

struct CountingReader {
    input: Cursor<Vec<u8>>,
    reads: usize,
}

impl Read for CountingReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.reads = self.reads.saturating_add(1);
        self.input.read(buffer)
    }
}

#[test]
fn identity_uses_lf_terminated_toon_and_keeps_stderr_clean() {
    let (status, stdout, stderr) = execute(&["."], b"name: Ada");
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(stdout, b"name: Ada\n");
    assert_eq!(stderr, [] as [u8; 0]);
}

#[test]
fn transcode_flushes_only_after_the_sequence_record_is_complete() {
    let command = parse_args(["--output-format", "toon-seq", "."]).unwrap();
    let mut input = br#"{"name":"Ada"}"#.as_slice();
    let mut output = FlushWriter::default();
    let mut error = Vec::new();
    assert_eq!(
        run_with_io(command, &mut input, &mut output, &mut error).unwrap(),
        ExitStatus::Success
    );
    assert_eq!(output.bytes, b"\x1ename: Ada\n");
    assert_eq!(output.flush_points.first(), Some(&output.bytes.len()));
    assert_eq!(error, [] as [u8; 0]);
}

#[test]
fn lightweight_json_transcode_matches_document_scalar_rendering() {
    for input in [
        br"1e2".as_slice(),
        br#"[1e2,"true",false,null,9007199254740993]"#.as_slice(),
        br#"{"plain":"null","number":1e2,"bool":true,"nil":null,"array":[1e2,"true",false,null]}"#
            .as_slice(),
    ] {
        let arguments = ["-ijson", "-otoon", "."];
        let automatic = execute_with_override(&arguments, input, ExecutionOverride::Automatic);
        let document = execute_with_override(&arguments, input, ExecutionOverride::Document);
        assert_eq!(automatic.0.unwrap(), ExitStatus::Success);
        assert_eq!(document.0.unwrap(), ExitStatus::Success);
        assert_eq!(automatic.1, document.1);
        assert_eq!(automatic.2, [] as [u8; 0]);
        assert_eq!(document.2, [] as [u8; 0]);
    }
}

#[test]
fn json_transcode_buffers_source_reads() {
    let mut json = Vec::with_capacity(256 * 1024 + 2);
    json.push(b'"');
    json.extend(std::iter::repeat_n(b'a', 256 * 1024));
    json.push(b'"');
    let mut input = CountingReader {
        input: Cursor::new(json),
        reads: 0,
    };
    let command = parse_args(["--input-format", "json", "."]).unwrap();
    let mut output = Vec::new();
    let mut error = Vec::new();
    assert_eq!(
        run_with_io(command, &mut input, &mut output, &mut error).unwrap(),
        ExitStatus::Success
    );
    assert!(input.reads < 16, "source was read {} times", input.reads);
    assert_eq!(error, [] as [u8; 0]);
}

#[test]
fn json_event_stream_buffers_source_reads() {
    let mut json = Vec::with_capacity(256 * 1024 + 2);
    json.push(b'"');
    json.extend(std::iter::repeat_n(b'a', 256 * 1024));
    json.push(b'"');
    let mut input = CountingReader {
        input: Cursor::new(json),
        reads: 0,
    };
    let command = parse_args([
        "--stream",
        "--input-format",
        "json",
        "--output-format",
        "json",
        ".",
    ])
    .unwrap();
    let mut output = Vec::new();
    let mut error = Vec::new();
    assert_eq!(
        run_with_io(command, &mut input, &mut output, &mut error).unwrap(),
        ExitStatus::Success
    );
    assert!(input.reads < 16, "source was read {} times", input.reads);
    assert_eq!(error, [] as [u8; 0]);
}

#[test]
fn automatic_json_buffers_source_reads() {
    let item = br#"{"value":1,"discarded":[1,2,3]}"#;
    let mut json = Vec::with_capacity(item.len() * 4_000 + 16);
    json.extend_from_slice(br#"{"items":["#);
    for index in 0..4_000 {
        if index != 0 {
            json.push(b',');
        }
        json.extend_from_slice(item);
    }
    json.extend_from_slice(b"]}");
    let mut input = CountingReader {
        input: Cursor::new(json),
        reads: 0,
    };
    let command = parse_args([
        "--input-format",
        "json",
        "--output-format",
        "json",
        ".items[].value",
    ])
    .unwrap();
    let mut output = Vec::new();
    let mut error = Vec::new();
    assert_eq!(
        run_with_io(command, &mut input, &mut output, &mut error).unwrap(),
        ExitStatus::Success
    );
    assert!(input.reads < 16, "source was read {} times", input.reads);
    assert_eq!(error, [] as [u8; 0]);
}

#[test]
fn inputs_json_buffers_source_reads() {
    let mut json = Vec::with_capacity(16 * 1024);
    for _ in 0..4_000 {
        json.extend_from_slice(b"1\n");
    }
    let mut input = CountingReader {
        input: Cursor::new(json),
        reads: 0,
    };
    let command = parse_args([
        "--input-format",
        "json",
        "--output-format",
        "json",
        "[., inputs] | length",
    ])
    .unwrap();
    let mut output = Vec::new();
    let mut error = Vec::new();
    assert_eq!(
        run_with_io(command, &mut input, &mut output, &mut error).unwrap(),
        ExitStatus::Success
    );
    assert_eq!(output, b"4000\n");
    assert!(input.reads < 16, "source was read {} times", input.reads);
    assert_eq!(error, [] as [u8; 0]);
}

#[test]
fn explicit_module_roots_import_include_metadata_and_reject_cycles() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("sample.jq"),
        "module {kind:\"test\"}; def twice($x): $x * 2;",
    )
    .unwrap();
    fs::write(root.path().join("included.jq"), "def answer: 42;").unwrap();
    fs::write(root.path().join("a.jq"), "include \"b\"; def a: 1;").unwrap();
    fs::write(root.path().join("b.jq"), "include \"a\"; def b: 2;").unwrap();
    fs::write(root.path().join("c.jq"), "include \"d\"; def c: 3;").unwrap();
    fs::write(root.path().join("d.jq"), "def d: 4;").unwrap();
    fs::write(root.path().join("large.jq"), "def large: 123456;").unwrap();
    let root = root.path().to_string_lossy().into_owned();

    let arguments = [
        "-L",
        root.as_str(),
        "-n",
        "--output-format",
        "json",
        "-c",
        "import \"sample\" as s; include \"included\"; s::twice(3), answer, (\"sample\" | modulemeta)",
    ];
    let (status, stdout, stderr) = execute(&arguments, b"");
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(
        stdout,
        b"6\n42\n{\"kind\":\"test\",\"deps\":[],\"defs\":[\"twice/1\"]}\n"
    );
    assert_eq!(stderr, [] as [u8; 0]);

    let cycle = ["-L", root.as_str(), "-n", "include \"a\"; ."];
    let (status, stdout, _) = execute(&cycle, b"must not be consumed");
    let error = status.unwrap_err().to_string();
    assert!(error.contains("cyclic module import"));
    assert!(error.contains("a.jq") && error.contains("b.jq"));
    assert_eq!(stdout, [] as [u8; 0]);

    let escape = ["-L", root.as_str(), "-n", "include \"../outside\"; ."];
    let (status, _, _) = execute(&escape, b"");
    assert!(
        status
            .unwrap_err()
            .to_string()
            .contains("escapes configured roots")
    );

    #[cfg(unix)]
    {
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join("outside.jq");
        fs::write(&target, "def escaped: 1;").unwrap();
        std::os::unix::fs::symlink(target, root.as_str().to_owned() + "/linked.jq").unwrap();
        let linked = ["-L", root.as_str(), "-n", "include \"linked\"; ."];
        let (status, _, _) = execute(&linked, b"");
        assert!(
            status
                .unwrap_err()
                .to_string()
                .contains("resolves outside root")
        );
    }

    let count_limited = [
        "-L",
        root.as_str(),
        "--max-depth",
        "1",
        "-n",
        "include \"c\"; .",
    ];
    let (status, _, _) = execute(&count_limited, b"");
    assert!(
        status
            .unwrap_err()
            .to_string()
            .contains("module count limit")
    );

    let bytes_limited = [
        "-L",
        root.as_str(),
        "--max-input-bytes",
        "8",
        "-n",
        "include \"large\"; .",
    ];
    let (status, _, _) = execute(&bytes_limited, b"");
    assert!(status.unwrap_err().to_string().contains("byte limit"));

    let explain = [
        "-L",
        root.as_str(),
        "-n",
        "--explain-json",
        "import \"sample\" as s; s::twice(1)",
    ];
    let (status, _, stderr) = execute(&explain, b"");
    assert_eq!(status.unwrap(), ExitStatus::Success);
    let report: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
    assert_eq!(report["modules"].as_array().unwrap().len(), 1);
    assert_eq!(report["modules"][0]["sha256"].as_str().unwrap().len(), 64);
}

#[test]
fn json_override_raw_join_null_and_exit_status_are_wired() {
    let (status, stdout, _) = execute(
        &[
            "--input-format",
            "json",
            "--output-format",
            "json",
            "-c",
            ".",
        ],
        br#"{"n":9007199254740993}"#,
    );
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(
        stdout,
        br#"{"n":9007199254740993}
"#
    );

    let (status, stdout, _) = execute(&["-n", "-j", "\"x\""], b"ignored");
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(stdout, b"x");

    let (status, _, _) = execute(&["-n", "--output-format", "json", "-e", "empty"], b"");
    assert_eq!(status.unwrap(), ExitStatus::NoResult);
}

#[test]
fn external_values_compile_and_execute() {
    let (status, stdout, _) = execute(&["-n", "--argjson", "n", "42", "$n"], b"");
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(stdout, b"42\n");
}

#[test]
fn unbuffered_flushes_after_each_complete_result() {
    let command = parse_args(["--output-format", "json", "-nc", "--unbuffered", "1, 2"]).unwrap();
    let mut input = &b""[..];
    let mut output = FlushWriter::default();
    let mut error = Vec::new();
    assert_eq!(
        run_with_io(command, &mut input, &mut output, &mut error).unwrap(),
        ExitStatus::Success
    );
    assert_eq!(output.bytes, b"1\n2\n");
    assert_eq!(output.flush_points, [2, 4, 4]);
    assert_eq!(error, [] as [u8; 0]);
}

#[test]
fn unbuffered_effect_flush_failure_cancels_before_open_input_read() {
    let command = parse_args(["-n", "--unbuffered", "-c", "debug, inputs"]).unwrap();
    let entered = Arc::new(AtomicBool::new(false));
    let release = Arc::new(AtomicBool::new(false));
    let reader = OpenReader {
        entered: Arc::clone(&entered),
        release: Arc::clone(&release),
    };
    let (done_sender, done_receiver) = sync_channel(1);
    let worker = thread::spawn(move || {
        let mut stdin = reader;
        let mut stdout = Vec::new();
        let mut stderr = FailingFlushWriter { bytes: Vec::new() };
        let failed = run_with_io(command, &mut stdin, &mut stdout, &mut stderr).is_err();
        done_sender
            .send(failed)
            .expect("test receiver remains alive");
    });

    let Ok(failed) = done_receiver.recv_timeout(Duration::from_secs(2)) else {
        release.store(true, Ordering::Release);
        let _ = worker.join();
        panic!("flush failure did not cancel the open-input worker")
    };
    assert!(failed);
    assert!(
        !entered.load(Ordering::Acquire),
        "worker read open input before flush failure was acknowledged"
    );
    release.store(true, Ordering::Release);
    worker.join().expect("flush-failure worker should exit");
}

#[test]
fn help_version_and_compatibility_write_only_stdout() {
    for command in [Command::Help, Command::Version, Command::Compatibility] {
        let mut input = &b""[..];
        let mut output = Vec::new();
        let mut error = Vec::new();
        assert_eq!(
            run_with_io(command, &mut input, &mut output, &mut error).unwrap(),
            ExitStatus::Success
        );
        assert_eq!(error, [] as [u8; 0]);
    }
}

#[test]
fn prior_framed_results_survive_a_later_runtime_error() {
    let (status, stdout, stderr) = execute(
        &["-n", "--output-format", "toon-seq", "1, error(\"later\")"],
        b"",
    );
    assert!(matches!(status, Err(super::RunError::Runtime(_))));
    assert_eq!(stdout, b"\x1e1\n");
    assert_eq!(stderr, [] as [u8; 0]);
}

#[test]
fn document_roots_continue_after_recoverable_runtime_errors() {
    let (status, stdout, stderr) = execute_with_override(
        &["-ijson", "-ojson", "-c", ". + 1"],
        b"1\n\"x\"\n2\n",
        ExecutionOverride::Document,
    );
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(stdout, b"2\n3\n");
    assert_single_runtime_diagnostic(&stderr);
}

#[test]
fn raw_input_lines_continue_after_recoverable_runtime_errors() {
    let (status, stdout, stderr) = execute_with_override(
        &[
            "-R",
            "-ojson",
            "-c",
            "if . == \"x\" then error(\"bad\") else . end",
        ],
        b"a\nx\nb\n",
        ExecutionOverride::Document,
    );
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(stdout, b"\"a\"\n\"b\"\n");
    assert_eq!(stderr, b"tq: runtime error: bad\n");
}

#[test]
fn document_runtime_diagnostics_preserve_effect_order_and_final_status() {
    let (status, stdout, stderr) = execute_with_override(
            [
                "-ijson",
                "-ojson",
                "-c",
                "if . == 1 then error(\"first\") elif . == 2 then debug(\"middle\") else error(\"last\") end",
            ]
            .as_slice(),
            b"1\n2\n3\n",
            ExecutionOverride::Document,
        );
    assert_eq!(status.unwrap(), ExitStatus::Runtime);
    assert_eq!(stdout, b"2\n");
    assert_eq!(
        stderr,
        b"tq: runtime error: first\n[\"DEBUG:\",\"middle\"]\ntq: runtime error: last\n"
    );
}

#[test]
fn document_runtime_diagnostic_precedes_a_later_parse_error() {
    let (status, stdout, stderr) = execute_with_override(
        &["-ijson", "-ojson", "-c", ". + 1"],
        b"\"x\"\n{\n",
        ExecutionOverride::Document,
    );
    let error = status.unwrap_err();
    assert_eq!(error.status(), ExitStatus::Input);
    assert!(matches!(error, super::RunError::Input(_)));
    assert_eq!(stdout, [] as [u8; 0]);
    let stderr = String::from_utf8(stderr).unwrap();
    assert_eq!(stderr.matches("runtime error:").count(), 1);
    assert!(stderr.starts_with("tq: runtime error:"));
    assert_eq!(stderr.lines().count(), 1);
}

#[test]
fn single_document_invocations_keep_embedded_runtime_errors() {
    let (status, stdout, stderr) = execute_with_override(
        &["-n", "-ojson", "-c", "error(\"single\")"],
        b"",
        ExecutionOverride::Document,
    );
    let error = status.unwrap_err();
    assert_eq!(error.status(), ExitStatus::Runtime);
    assert!(matches!(error, super::RunError::Runtime(_)));
    assert_eq!(stdout, [] as [u8; 0]);
    assert_eq!(stderr, [] as [u8; 0]);

    let (status, stdout, stderr) = execute_with_override(
        &["--slurp", "-ijson", "-ojson", "-c", ". + 1"],
        b"1\n2\n",
        ExecutionOverride::Document,
    );
    let error = status.unwrap_err();
    assert_eq!(error.status(), ExitStatus::Runtime);
    assert!(matches!(error, super::RunError::Runtime(_)));
    assert_eq!(stdout, [] as [u8; 0]);
    assert_eq!(stderr, [] as [u8; 0]);

    let (status, stdout, stderr) = execute_with_override(
        &["-R", "--slurp", "-ojson", "-c", "error(\"single\")"],
        b"a\nb\n",
        ExecutionOverride::Document,
    );
    let error = status.unwrap_err();
    assert_eq!(error.status(), ExitStatus::Runtime);
    assert!(matches!(error, super::RunError::Runtime(_)));
    assert_eq!(stdout, [] as [u8; 0]);
    assert_eq!(stderr, [] as [u8; 0]);
}

#[test]
fn document_empty_after_runtime_error_obeys_exit_status_mode() {
    let arguments = [
        "-ijson",
        "-ojson",
        "-c",
        "if type == \"string\" then . + 1 else empty end",
    ];
    let without_e = execute_with_override(&arguments, b"\"x\"\n2\n", ExecutionOverride::Document);
    assert_eq!(without_e.0.unwrap(), ExitStatus::Success);
    assert_eq!(without_e.1, [] as [u8; 0]);
    assert_single_runtime_diagnostic(&without_e.2);

    let with_e_arguments = [
        "-ijson",
        "-ojson",
        "-c",
        "-e",
        "if type == \"string\" then . + 1 else empty end",
    ];
    let with_e = execute_with_override(
        &with_e_arguments,
        b"\"x\"\n2\n",
        ExecutionOverride::Document,
    );
    assert_eq!(with_e.0.unwrap(), ExitStatus::NoResult);
    assert_eq!(with_e.1, [] as [u8; 0]);
    assert_single_runtime_diagnostic(&with_e.2);
}

#[test]
fn result_and_output_limits_preserve_complete_prior_frames() {
    let (status, stdout, _) = execute(
        &[
            "-n",
            "--output-format",
            "toon-seq",
            "--max-results",
            "1",
            "1, 2",
        ],
        b"",
    );
    assert!(matches!(
        status,
        Err(super::RunError::Resource("result-count"))
    ));
    assert_eq!(stdout, b"\x1e1\n");

    let (status, stdout, _) = execute(
        &[
            "-n",
            "--output-format",
            "toon-seq",
            "--max-output-bytes",
            "3",
            "1, 2",
        ],
        b"",
    );
    assert_eq!(status.unwrap_err().status(), ExitStatus::Resource);
    assert_eq!(stdout, b"\x1e1\n");

    let (status, stdout, _) = execute(
        &[
            "-n",
            "--output-format",
            "toon-seq",
            "--max-results",
            "1",
            "foreach (1,2) as $x (0; . + $x; .)",
        ],
        b"",
    );
    assert!(matches!(
        status,
        Err(super::RunError::Resource("result-count"))
    ));
    assert_eq!(stdout, b"\x1e1\n");

    let (status, stdout, _) = execute(
        &[
            "-n",
            "--output-format",
            "toon-seq",
            "--max-output-bytes",
            "3",
            "foreach (1,2) as $x (0; . + $x; .)",
        ],
        b"",
    );
    assert_eq!(status.unwrap_err().status(), ExitStatus::Resource);
    assert_eq!(stdout, b"\x1e1\n");

    let (status, stdout, _) = execute(
        &["--output-format", "toon-seq", "--max-results", "2", ".."],
        b"[1,2]",
    );
    assert!(matches!(
        status,
        Err(super::RunError::Resource("result-count"))
    ));
    assert_eq!(stdout, b"\x1e[2]: 1,2\n\x1e1\n");

    let (status, stdout, _) = execute(
        &[
            "-n",
            "--output-format",
            "toon-seq",
            "--max-output-bytes",
            "5",
            r#"1, "abcdef=\(.)""#,
        ],
        b"",
    );
    assert_eq!(status.unwrap_err().status(), ExitStatus::Resource);
    assert_eq!(stdout, b"\x1e1\n");
}

#[test]
fn input_limits_fail_with_the_resource_exit_category() {
    let (status, stdout, _) = execute(
        &["--input-format", "json", "--max-input-bytes", "3", "."],
        b"null",
    );
    assert_eq!(status.unwrap_err().status(), ExitStatus::Resource);
    assert_eq!(stdout, [] as [u8; 0]);
}

#[test]
fn explain_json_publishes_plan_detection_and_limits() {
    let (status, stdout, stderr) = execute(
        &[
            "--input-format",
            "json",
            "--stream",
            "--output-format",
            "json",
            "--explain-json",
            ".",
        ],
        b"[1]",
    );
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_ne!(stdout, [] as [u8; 0]);
    let report: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
    assert_eq!(report["execution"]["plan"], "events");
    assert_eq!(report["execution"]["input_detection"], "override:json");
    assert!(report["execution"]["limits"]["input_bytes"].is_u64());
}

#[test]
fn automatic_subtree_projection_and_selection_preserve_order_and_missing_values() {
    let input = br#"{"features":[{"id":"a","properties":{"mag":1}},{"id":"b","properties":{"mag":3}},{"id":"c","properties":{}}]}"#;
    let (status, stdout, stderr) = execute(
        &[
            "--input-format",
            "json",
            "--output-format",
            "json",
            "-c",
            ".features[].properties.mag",
        ],
        input,
    );
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(stdout, b"1\n3\nnull\n");
    assert_eq!(stderr, [] as [u8; 0]);

    let (status, stdout, stderr) = execute(
        &[
            "--input-format",
            "json",
            "--output-format",
            "json",
            "-c",
            ".features[] | select(.properties.mag >= 2) | .id",
        ],
        input,
    );
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(stdout, b"\"b\"\n");
    assert_eq!(stderr, [] as [u8; 0]);
}

#[test]
fn automatic_event_plan_filters_scalars_without_retaining_containers() {
    let (status, stdout, stderr) = execute(
        &[
            "--input-format",
            "json",
            "--output-format",
            "json",
            "-c",
            ".items[] | numbers",
        ],
        br#"{"items":[1,{"x":2},null,3]}"#,
    );
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(stdout, b"1\n3\n");
    assert_eq!(stderr, [] as [u8; 0]);
}

#[test]
fn automatic_event_plan_uses_auto_detected_json() {
    let (status, stdout, stderr) = execute(
        &[
            "--output-format",
            "json",
            "-c",
            "--explain-json",
            ".items[] | numbers",
        ],
        br#"{"items":[1,{"x":2},null,3]}"#,
    );
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(stdout, b"1\n3\n");
    let explain: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
    assert_eq!(explain["execution"]["plan"], "events");
    assert_eq!(
        explain["execution"]["retained_working_set"],
        "decoder frames, current path, and one scalar event value"
    );
}

#[test]
fn automatic_event_plan_uses_auto_detected_json_file() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.json");
    std::fs::write(&source, br#"{"items":[1,{"x":2},null,3]}"#).unwrap();
    let command = parse_args([
        "--output-format",
        "json",
        "-c",
        "--explain-json",
        ".items[] | numbers",
        source.to_str().unwrap(),
    ])
    .unwrap();
    let mut input = &[][..];
    let mut output = Vec::new();
    let mut error = Vec::new();
    assert_eq!(
        run_with_io(command, &mut input, &mut output, &mut error).unwrap(),
        ExitStatus::Success
    );
    assert_eq!(output, b"1\n3\n");
    let explain: serde_json::Value = serde_json::from_slice(&error).unwrap();
    assert_eq!(explain["execution"]["plan"], "events");
}

#[test]
fn auto_detected_yaml_keeps_document_fallback() {
    let (status, stdout, stderr) = execute(
        &[
            "--output-format",
            "json",
            "-c",
            "--explain-json",
            ".[] | numbers",
        ],
        b"- 1\n- value: 2\n- 3\n",
    );
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(stdout, b"1\n3\n");
    let explain: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
    assert_eq!(explain["execution"]["plan"], "document");
}

#[test]
fn automatic_projection_uses_auto_detected_toon_decoder_events() {
    let (status, stdout, stderr) = execute(
        &["--output-format", "json", "-c", ".items[].x"],
        b"items[2]:\n  - x: 1\n  - x: 2\n",
    );
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(stdout, b"1\n2\n");
    assert_eq!(stderr, [] as [u8; 0]);
}

#[test]
fn automatic_plans_preserve_prior_frames_before_hostile_type_errors() {
    let (status, stdout, stderr) = execute(
        &[
            "--input-format",
            "json",
            "--output-format",
            "json",
            "-c",
            ".items[] | .x",
        ],
        br#"{"items":[{"x":1},2,{"x":3}]}"#,
    );
    assert_eq!(status.unwrap(), ExitStatus::Runtime);
    assert_eq!(stdout, b"1\n");
    assert_eq!(
        stderr,
        b"tq: runtime error: field access cannot be applied to number\n"
    );
}

#[test]
fn automatic_subtree_limit_fails_before_the_first_partial_result() {
    let (status, stdout, stderr) = execute(
        &[
            "--input-format",
            "json",
            "--output-format",
            "json",
            "-c",
            "--prepare-memory-bytes",
            "32",
            ".items[] | .x",
        ],
        br#"{"items":[{"x":"a value deliberately larger than the capture limit"}]}"#,
    );
    assert!(matches!(
        status,
        Err(super::RunError::Resource("subtree-bytes"))
    ));
    assert_eq!(stdout, [] as [u8; 0]);
    assert_eq!(stderr, [] as [u8; 0]);
}

#[test]
fn automatic_explain_and_report_publish_proof_and_retention_high_water() {
    let directory = tempfile::tempdir().unwrap();
    let report_path = directory.path().join("automatic.json");
    let command = parse_args([
        "--input-format",
        "json",
        "--output-format",
        "json",
        "-c",
        "--explain-json",
        "--report-file",
        report_path.to_str().unwrap(),
        ".items[] | .x",
    ])
    .unwrap();
    let mut input = br#"{"items":[{"x":1},{"x":2}]}"#.as_slice();
    let mut output = Vec::new();
    let mut error = Vec::new();
    assert_eq!(
        run_with_io(command, &mut input, &mut output, &mut error).unwrap(),
        ExitStatus::Success
    );
    let explain: serde_json::Value = serde_json::from_slice(&error).unwrap();
    assert_eq!(explain["execution"]["plan"], "subtree");
    assert_eq!(
        explain["execution"]["proof"]["required_path_prefix"][0]["value"],
        "items"
    );
    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(report_path).unwrap()).unwrap();
    assert_eq!(report["execution"]["plan"], "subtree");
    assert!(
        report["execution"]["retention_high_water"]["bytes"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert_eq!(
        report["execution"]["retention_high_water"]["completed_subtrees"],
        2
    );
}

#[test]
fn automatic_and_document_plans_agree_across_nested_boundary_shapes() {
    let inputs = [
        br#"{"items":[]}"#.as_slice(),
        br#"{"items":[null,false,0,"",[],{}]}"#.as_slice(),
        br#"{"items":[{"x":1},{"x":[2,3]},{"nested":{"x":4}}]}"#.as_slice(),
        br#"{"items":{"first":{"x":1},"second":{"x":2}}}"#.as_slice(),
    ];
    for input in inputs {
        let automatic = execute(
            &[
                "--input-format",
                "json",
                "--output-format",
                "json",
                "-c",
                ".items[] | .x?",
            ],
            input,
        );
        let document = execute(
            &[
                "--input-format",
                "yaml",
                "--output-format",
                "json",
                "-c",
                ".items[] | .x?",
            ],
            input,
        );
        assert_eq!(automatic.0.unwrap(), document.0.unwrap());
        assert_eq!(automatic.1, document.1);
        assert_eq!(automatic.2, [] as [u8; 0]);
        assert_eq!(document.2, [] as [u8; 0]);
    }
}

#[test]
fn non_event_input_reports_deterministic_pre_input_fallback() {
    let (status, stdout, stderr) = execute(
        &["--input-format", "yaml", "--explain-json", ".items[] | .x"],
        br#"{"items":[{"x":1}]}"#,
    );
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_ne!(stdout, [] as [u8; 0]);
    let explain: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
    assert_eq!(explain["execution"]["plan"], "document");
    assert_eq!(
        explain["execution"]["stream_rejection"],
        "the selected input or CLI mode does not expose automatic decoder events"
    );
}

#[test]
fn json_lines_records_are_consistent_across_execution_plans() {
    let input = br#"{"items":[{"x":1},{"x":2}],"values":[1,2]}
{"items":[{"x":3}],"values":[3]}
"#;
    let cases = [
        (
            ".",
            "document",
            "{\"items\":[{\"x\":1},{\"x\":2}],\"values\":[1,2]}\n{\"items\":[{\"x\":3}],\"values\":[3]}\n",
        ),
        (".values[] | numbers", "events", "1\n2\n3\n"),
        (".items[] | .x", "subtree", "1\n2\n3\n"),
        (".items | sort | .[].x", "blocking-document", "1\n2\n3\n"),
    ];
    for (query, expected_plan, expected_output) in cases {
        let (status, output, explain) =
            execute(&["-ijsonl", "-ojsonl", "--explain-json", query], input);
        assert_eq!(status.unwrap(), ExitStatus::Success, "{query}");
        assert_eq!(output, expected_output.as_bytes(), "{query}");
        let explain: serde_json::Value = serde_json::from_slice(&explain).unwrap();
        assert_eq!(explain["execution"]["plan"], expected_plan, "{query}");
    }

    let (status, output, explain) = execute(
        &[
            "-ijsonl",
            "-ojsonl",
            "--slurp",
            "--explain-json",
            "map(.items[]) | map(.x)",
        ],
        input,
    );
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(output, b"[1,2,3]\n");
    let explain: serde_json::Value = serde_json::from_slice(&explain).unwrap();
    assert_eq!(explain["execution"]["plan"], "whole-input");
}

#[test]
fn hybrid_sort_matches_document_and_preserves_json_lines_boundaries() {
    let query = "[.items[].value] | sort";
    let input = br#"{"items":[{"value":3},{"value":1},{"value":2}]}"#;
    let hybrid = execute(&["-ijson", "-ojson", "-c", "--explain-json", query], input);
    let document = execute(&["-iyaml", "-ojson", "-c", query], input);
    assert_eq!(hybrid.0.unwrap(), ExitStatus::Success);
    assert_eq!(document.0.unwrap(), ExitStatus::Success);
    assert_eq!(hybrid.1, document.1);
    let explain: serde_json::Value = serde_json::from_slice(&hybrid.2).unwrap();
    assert_eq!(explain["execution"]["plan"], "hybrid-streaming-blocking");
    assert_eq!(
        explain["execution"]["hybrid_proof"]["preparation"],
        "stable-sort-runs"
    );

    let json_lines = br#"{"items":[{"value":2},{"value":1}]}
{"items":[{"value":4},{"value":3}]}
"#;
    let (status, output, _) = execute(&["-ijsonl", "-ojsonl", query], json_lines);
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(output, b"[1,2]\n[3,4]\n");

    let (status, output, _) = execute(
        &["-ijson", "-ojson", "-c", query],
        br#"{"items":[{"value":2},{"value":1},"#,
    );
    assert_eq!(status.unwrap_err().status(), ExitStatus::Input);
    assert_eq!(output, [] as [u8; 0]);

    let (status, output, _) = execute(
        &[
            "-ijson",
            "-ojson",
            "-c",
            "--hybrid-in-flight-bytes",
            "1",
            query,
        ],
        input,
    );
    assert_eq!(status.unwrap_err().status(), ExitStatus::Resource);
    assert_eq!(output, [] as [u8; 0]);

    let (status, output, _) = execute(
        &["-ijson", "-ojson", "-c", query],
        br#"{"items":[{"value":1,"value":2},{}]}"#,
    );
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(output, b"[null,2]\n");
}

#[test]
fn hybrid_and_forced_document_agree_on_projected_sort_edge_cases() {
    let arguments = [
        "-ijson",
        "-ojson",
        "-c",
        "--hybrid-batch-values",
        "1",
        "--decode-batch-values",
        "1",
        "[.items[].value] | sort",
    ];
    for input in [
            br#"{"items":[]}"#.as_slice(),
            br#"{"items":[{"value":null},{"value":false},{"value":1},{"value":"x"},{"value":[]},{"value":{}}]}"#.as_slice(),
            br#"{"items":[{"value":{"a":1,"b":2}},{"value":{"b":2,"a":1}}]}"#.as_slice(),
            br#"{"items":[{"discarded":{"x":1,"x":2},"value":3},{"value":1,"value":2},{}]}"#.as_slice(),
        ] {
            let hybrid = execute_with_override(&arguments, input, ExecutionOverride::Automatic);
            let document =
                execute_with_override(&arguments, input, ExecutionOverride::Document);
            assert_eq!(hybrid.0.unwrap(), document.0.unwrap());
            assert_eq!(hybrid.1, document.1);
            assert_eq!(hybrid.2, [] as [u8; 0]);
            assert_eq!(document.2, [] as [u8; 0]);
        }

    let input = br#"{"items":{"value":1}}"#;
    let hybrid = execute_with_override(&arguments, input, ExecutionOverride::Automatic);
    let document = execute_with_override(&arguments, input, ExecutionOverride::Document);
    assert_eq!(
        observed_exit_status(&hybrid.0),
        observed_exit_status(&document.0)
    );
    assert_eq!(hybrid.1, document.1);
}

#[test]
fn hybrid_vm_step_limit_is_shared_across_document_suffixes() {
    let arguments = [
        "-ijsonl",
        "-ojsonl",
        "--max-vm-steps",
        "1",
        "[.items[].value] | sort",
    ];
    let input = br#"{"items":[{"value":2},{"value":1}]}
{"items":[{"value":4},{"value":3}]}
"#;
    let hybrid = execute_with_override(&arguments, input, ExecutionOverride::Automatic);

    assert_eq!(hybrid.0.unwrap_err().status(), ExitStatus::Resource);
    assert_eq!(hybrid.1, b"[1,2]\n");
}

#[test]
fn parallel_decode_preserves_cross_batch_stability_and_downstream_errors() {
    let stable_arguments = [
        "-ijson",
        "-ojson",
        "-c",
        "--decode-batch-values",
        "1",
        "--hybrid-batch-values",
        "1",
        "[.items[]] | sort_by(.key)",
    ];
    let input =
        br#"{"items":[{"key":1,"id":"first"},{"key":0,"id":"zero"},{"key":1,"id":"second"}]}"#;
    let automatic = execute_with_override(&stable_arguments, input, ExecutionOverride::Automatic);
    let document = execute_with_override(&stable_arguments, input, ExecutionOverride::Document);
    assert_eq!(automatic.0.unwrap(), document.0.unwrap());
    assert_eq!(automatic.1, document.1);
    assert_eq!(automatic.1, b"[{\"key\":0,\"id\":\"zero\"},{\"key\":1,\"id\":\"first\"},{\"key\":1,\"id\":\"second\"}]\n");

    let fallible_arguments = [
        "-ijson",
        "-ojson",
        "-c",
        "--decode-batch-values",
        "1",
        "[.items[].value] | map(if . == 2 then error(\"boom\") else . end) | sort",
    ];
    let input = br#"{"items":[{"value":3},{"value":2},{"value":1}]}"#;
    let automatic = execute_with_override(&fallible_arguments, input, ExecutionOverride::Automatic);
    let document = execute_with_override(&fallible_arguments, input, ExecutionOverride::Document);
    assert_eq!(
        observed_exit_status(&automatic.0),
        observed_exit_status(&document.0)
    );
    assert_eq!(automatic.1, document.1);
}

#[test]
fn nested_static_prefix_explains_root_transactional_serialization() {
    let (status, output, explain) = execute(
        &[
            "-ijson",
            "-ojson",
            "-c",
            "--explain-json",
            "--decode-batch-values",
            "1",
            "[.root.features[].value] | sort",
        ],
        br#"{"root":{"features":[{"value":2},{"value":1}]}}"#,
    );
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(output, b"[1,2]\n");
    let explain: serde_json::Value = serde_json::from_slice(&explain).unwrap();
    assert_eq!(
        explain["execution"]["parallel_selected_decode"]["eligible"],
        false
    );
    assert_eq!(
        explain["execution"]["parallel_selected_decode"]["reason"],
        "root-lifecycle-requires-serial-commit"
    );

    let (status, _, explain) = execute(
        &[
            "-ijson",
            "-ojson",
            "-c",
            "--arg",
            "key",
            "items",
            "--explain-json",
            "[.[$key][]] | sort",
        ],
        br#"{"items":[2,1]}"#,
    );
    assert_eq!(status.unwrap(), ExitStatus::Success);
    let explain: serde_json::Value = serde_json::from_slice(&explain).unwrap();
    assert_eq!(
        explain["execution"]["parallel_selected_decode"]["eligible"],
        false
    );
    assert!(
        explain["execution"]["parallel_selected_decode"]["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("not statically partitionable"))
    );
}

#[test]
fn hybrid_report_and_dead_sort_explain_are_machine_readable() {
    let directory = tempfile::tempdir().unwrap();
    let report_path = directory.path().join("hybrid.json");
    let command = parse_args([
        "-ijson",
        "-ojson",
        "-c",
        "--explain-json",
        "--report-file",
        report_path.to_str().unwrap(),
        "[.items[].value] | sort",
    ])
    .unwrap();
    let mut input = br#"{"items":[{"value":2},{"value":1}]}"#.as_slice();
    let mut output = Vec::new();
    let mut error = Vec::new();
    assert_eq!(
        run_with_io(command, &mut input, &mut output, &mut error).unwrap(),
        ExitStatus::Success
    );
    let explain: serde_json::Value = serde_json::from_slice(&error).unwrap();
    assert_eq!(explain["execution"]["plan"], "hybrid-streaming-blocking");
    assert_eq!(
        explain["execution"]["parallel_selected_decode"]["eligible"],
        false
    );
    assert_eq!(
        explain["execution"]["parallel_selected_decode"]["reason"],
        "root-lifecycle-requires-serial-commit"
    );
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(report_path).unwrap()).unwrap();
    assert_eq!(report["execution"]["plan"], "hybrid-streaming-blocking");
    assert_eq!(
        report["execution"]["retention_high_water"]["root_materialized"],
        false
    );
    assert!(report["execution"]["retention_high_water"]["sort_runs"].is_u64());
    assert!(report["execution"]["retention_high_water"]["worker_count"].is_u64());
    assert_eq!(
        report["execution"]["retention_high_water"]["retained_result_count"],
        2
    );
    assert!(
        report["execution"]["retention_high_water"]["retained_estimated_bytes"]
            .as_u64()
            .is_some_and(|bytes| bytes > 0)
    );
    assert!(
        report["execution"]["retention_high_water"]["decoder_depth"]
            .as_u64()
            .is_some_and(|depth| depth >= 3)
    );
    assert_eq!(
        report["execution"]["retention_high_water"]["blocking_state"],
        "projected-collection-and-blocking-suffix"
    );
    assert_eq!(report["execution"]["resource_outcome"], "success");
    assert_eq!(
        report["execution"]["retention_high_water"]["parallel_decode_active"],
        false
    );

    let (status, _, explain) = execute(
        &[
            "-ijson",
            "-ojson",
            "-c",
            "--explain-json",
            "[.items[].value] | sort | length",
        ],
        br#"{"items":[{"value":2},{"value":1}]}"#,
    );
    assert_eq!(status.unwrap(), ExitStatus::Success);
    let explain: serde_json::Value = serde_json::from_slice(&explain).unwrap();
    assert_eq!(
        explain["execution"]["optimizer_rewrites"][0]["name"],
        "array-sort-before-length"
    );
    assert_eq!(
        explain["execution"]["hybrid_proof"]["preparation"],
        "collect"
    );
}

#[test]
fn hybrid_report_records_resource_failure_outcome() {
    let directory = tempfile::tempdir().unwrap();
    let failed_report_path = directory.path().join("hybrid-failed.json");
    let command = parse_args([
        "-ijson",
        "-ojson",
        "-c",
        "--hybrid-in-flight-bytes",
        "1",
        "--report-file",
        failed_report_path.to_str().unwrap(),
        "[.items[].value] | sort",
    ])
    .unwrap();
    let mut input = br#"{"items":[{"value":2},{"value":1}]}"#.as_slice();
    let mut output = Vec::new();
    let mut error = Vec::new();
    assert_eq!(
        run_with_io(command, &mut input, &mut output, &mut error)
            .unwrap_err()
            .status(),
        ExitStatus::Resource
    );
    assert_eq!(output, [] as [u8; 0]);
    let failed_report: serde_json::Value =
        serde_json::from_slice(&fs::read(failed_report_path).unwrap()).unwrap();
    assert_eq!(
        failed_report["execution"]["resource_outcome"],
        "resource-limit"
    );
    assert!(
        failed_report["execution"]["retention_high_water"]["retained_result_count"]
            .as_u64()
            .is_some_and(|count| count > 0)
    );
}

#[test]
fn run_tests_self_checks_match_jq_state_contract() {
    assert!(run_tests_self_checks());
}

#[test]
fn run_test_compile_expectations_require_all_diagnostic_lines() {
    let (cases, malformed) =
        parse_run_test_cases("%%FAIL\nfoo\nfirst diagnostic\nsecond diagnostic\n\n");
    assert_eq!(malformed, 0);
    assert_eq!(cases.len(), 1);
    assert!(compile_diagnostic_matches(
        "first diagnostic\nsecond diagnostic\n",
        None,
        &cases[0].expected,
        "foo"
    ));
    assert!(!compile_diagnostic_matches(
        "first diagnostic\n",
        None,
        &cases[0].expected,
        "foo"
    ));
    assert!(!compile_diagnostic_matches(
        "prefix first diagnostic\nsecond diagnostic\n",
        None,
        &cases[0].expected,
        "foo"
    ));
}

#[allow(
    clippy::too_many_lines,
    reason = "the jq diagnostic oracle matrix keeps each expected source/caret fixture explicit"
)]
#[test]
fn jq_compile_expectations_render_structured_tq_diagnostics() {
    for (filter, diagnostic, expected) in [
        (
            "$missing",
            "query compilation failed: TQ-RESOLVE-VARIABLE-001: unknown variable $missing",
            "jq: error: $missing is not defined at <top-level>, line 1, column 1:\n    $missing\n    ^^^^^^^^",
        ),
        (
            "1 | foo(1)",
            "query compilation failed: TQ-RESOLVE-BUILTIN-001: unknown filter foo/1",
            "jq: error: foo/1 is not defined at <top-level>, line 1, column 5:\n    1 | foo(1)\n        ^^^",
        ),
        (
            "1 +",
            "query compilation failed: TQ-PARSE-EXPRESSION-001: expected filter expression",
            "jq: error: syntax error, unexpected end of file at <top-level>, line 1, column 3:\n    1 +\n      ^",
        ),
    ] {
        let expected = expected.lines().map(str::to_owned).collect::<Vec<_>>();
        assert!(
            compile_diagnostic_matches(diagnostic, None, &expected, filter),
            "{filter:?}: {expected:?}"
        );
    }
    for (filter, expected) in [
        (
            "]",
            "jq: error: syntax error, unexpected INVALID_CHARACTER, expecting end of file at <top-level>, line 1, column 1:\n    ]\n    ^",
        ),
        (
            ";",
            "jq: error: syntax error, unexpected ';', expecting end of file at <top-level>, line 1, column 1:\n    ;\n    ^",
        ),
        (
            "1 + ]",
            "jq: error: syntax error, unexpected INVALID_CHARACTER at <top-level>, line 1, column 5:\n    1 + ]\n        ^",
        ),
        (
            "1 + ;",
            "jq: error: syntax error, unexpected ';' at <top-level>, line 1, column 5:\n    1 + ;\n        ^",
        ),
        (
            "{x:}",
            "jq: error: syntax error, unexpected '}' at <top-level>, line 1, column 4:\n    {x:}\n       ^",
        ),
    ] {
        let diagnostic = parse_bytes("<query>", filter.as_bytes()).unwrap_err();
        let expected = expected.lines().map(str::to_owned).collect::<Vec<_>>();
        let actual = format!("query compilation failed: {diagnostic}");
        assert!(
            compile_diagnostic_matches(&actual, Some(diagnostic.as_ref()), &expected, filter,),
            "{filter:?}: {expected:?}"
        );
    }
    for (filter, expected) in [
        (
            "[1",
            "jq: error: syntax error, unexpected end of file, expecting '|' or ',' or ']' at <top-level>, line 1, column 2:\n    [1\n     ^",
        ),
        (
            "{x:1",
            "jq: error: syntax error, unexpected end of file, expecting '}' at <top-level>, line 1, column 4:\n    {x:1\n       ^",
        ),
        (
            "(1",
            "jq: error: syntax error, unexpected end of file, expecting '|' or ',' or ')' at <top-level>, line 1, column 2:\n    (1\n     ^",
        ),
        (
            "foo(1",
            "jq: error: syntax error, unexpected end of file, expecting ';' or ')' at <top-level>, line 1, column 5:\n    foo(1\n        ^",
        ),
        (
            "{",
            "jq: error: syntax error, unexpected end of file at <top-level>, line 1, column 1:\n    {\n    ^",
        ),
    ] {
        let diagnostic = parse_bytes("<query>", filter.as_bytes()).unwrap_err();
        let expected = expected.lines().map(str::to_owned).collect::<Vec<_>>();
        let actual = format!("query compilation failed: {diagnostic}");
        assert!(
            compile_diagnostic_matches(&actual, Some(diagnostic.as_ref()), &expected, filter,),
            "{filter:?}: {expected:?}"
        );
    }
    for (filter, expected) in [
        (
            "if true then 1",
            "jq: error: Possibly unterminated 'if' statement at <top-level>, line 1, column 1:\n    if true then 1\n    ^^^^^^^^^^^^^^",
        ),
        (
            "\"é",
            "jq: error: syntax error, unexpected end of file, expecting QQSTRING_TEXT or QQSTRING_INTERP_START or QQSTRING_END at <top-level>, line 1, column 2:\n    \"é\n     ^^",
        ),
    ] {
        let diagnostic = parse_bytes("<query>", filter.as_bytes()).unwrap_err();
        let expected = expected.lines().map(str::to_owned).collect::<Vec<_>>();
        let actual = format!("query compilation failed: {diagnostic}");
        assert!(
            compile_diagnostic_matches(&actual, Some(diagnostic.as_ref()), &expected, filter,),
            "{filter:?}: {expected:?}"
        );
    }
    for (filter, expected) in [
        (
            "if true",
            "jq: error: syntax error, unexpected end of file, expecting then or '|' or ',' at <top-level>, line 1, column 4:\n    if true\n       ^^^^",
        ),
        (
            "if true then",
            "jq: error: Possibly unterminated 'if' statement at <top-level>, line 1, column 1:\n    if true then\n    ^^^^^^^^^^^^",
        ),
        (
            "if true then 1 else",
            "jq: error: Possibly unterminated 'if' statement at <top-level>, line 1, column 1:\n    if true then 1 else\n    ^^^^^^^^^^^^^^^^^^^",
        ),
        (
            "if",
            "jq: error: syntax error, unexpected end of file at <top-level>, line 1, column 1:\n    if\n    ^^",
        ),
        (
            "if true elif",
            "jq: error: syntax error, unexpected elif, expecting then or '|' or ',' at <top-level>, line 1, column 9:\n    if true elif\n            ^^^^",
        ),
        (
            "try",
            "jq: error: syntax error, unexpected end of file at <top-level>, line 1, column 1:\n    try\n    ^^^",
        ),
        (
            "{(1",
            "jq: error: syntax error, unexpected end of file, expecting '|' or ',' or ')' at <top-level>, line 1, column 3:\n    {(1\n      ^",
        ),
        (
            "@",
            "jq: error: syntax error, unexpected INVALID_CHARACTER, expecting end of file at <top-level>, line 1, column 1:\n    @\n    ^",
        ),
        (
            "$",
            "jq: error: syntax error, unexpected end of file, expecting '$' at <top-level>, line 1, column 1:\n    $\n    ^",
        ),
        (
            "def",
            "jq: error: syntax error, unexpected end of file, expecting IDENT at <top-level>, line 1, column 1:\n    def\n    ^^^",
        ),
        (
            "label",
            "jq: error: syntax error, unexpected end of file, expecting BINDING at <top-level>, line 1, column 1:\n    label\n    ^^^^^",
        ),
        (
            "def foo(",
            "jq: error: syntax error, unexpected end of file, expecting IDENT or BINDING at <top-level>, line 1, column 8:\n    def foo(\n           ^",
        ),
        (
            "label $x",
            "jq: error: syntax error, unexpected end of file, expecting '|' at <top-level>, line 1, column 7:\n    label $x\n          ^^",
        ),
        (
            "def 1",
            "jq: error: syntax error, unexpected LITERAL, expecting IDENT at <top-level>, line 1, column 5:\n    def 1\n        ^",
        ),
        (
            "label 1",
            "jq: error: syntax error, unexpected LITERAL, expecting BINDING at <top-level>, line 1, column 7:\n    label 1\n          ^",
        ),
        (
            "def f(1;",
            "jq: error: syntax error, unexpected LITERAL, expecting IDENT or BINDING at <top-level>, line 1, column 7:\n    def f(1;\n          ^",
        ),
        (
            "\"unterminated",
            "jq: error: syntax error, unexpected end of file, expecting QQSTRING_TEXT or QQSTRING_INTERP_START or QQSTRING_END at <top-level>, line 1, column 2:\n    \"unterminated\n     ^^^^^^^^^^^^",
        ),
        (
            "\"\\q\"",
            "jq: error: Invalid escape at line 1, column 4 (while parsing '\"\\q\"') at <top-level>, line 1, column 2:\n    \"\\q\"\n     ^^",
        ),
    ] {
        let diagnostic = parse_bytes("<query>", filter.as_bytes()).unwrap_err();
        let expected = expected.lines().map(str::to_owned).collect::<Vec<_>>();
        let actual = format!("query compilation failed: {diagnostic}");
        assert!(
            compile_diagnostic_matches(&actual, Some(diagnostic.as_ref()), &expected, filter,),
            "{filter:?}: {expected:?}"
        );
    }
    for (filter, expected) in [
        (
            "1 | if true then 1",
            "jq: error: Possibly unterminated 'if' statement at <top-level>, line 1, column 5:\n    1 | if true then 1\n        ^^^^^^^^^^^^^^",
        ),
        (
            "if true then if false then 1 end",
            "jq: error: Possibly unterminated 'if' statement at <top-level>, line 1, column 1:\n    if true then if false then 1 end\n    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^",
        ),
        (
            "1 | @",
            "jq: error: syntax error, unexpected INVALID_CHARACTER at <top-level>, line 1, column 5:\n    1 | @\n        ^",
        ),
        (
            "1 | $",
            "jq: error: syntax error, unexpected end of file, expecting '$' at <top-level>, line 1, column 5:\n    1 | $\n        ^",
        ),
        (
            "1 | def",
            "jq: error: syntax error, unexpected end of file, expecting IDENT at <top-level>, line 1, column 5:\n    1 | def\n        ^^^",
        ),
        (
            "1 | label",
            "jq: error: syntax error, unexpected end of file, expecting BINDING at <top-level>, line 1, column 5:\n    1 | label\n        ^^^^^",
        ),
    ] {
        let diagnostic = parse_bytes("<query>", filter.as_bytes()).unwrap_err();
        let expected = expected.lines().map(str::to_owned).collect::<Vec<_>>();
        let actual = format!("query compilation failed: {diagnostic}");
        assert!(compile_diagnostic_matches(
            &actual,
            Some(diagnostic.as_ref()),
            &expected,
            filter,
        ));
    }
}
