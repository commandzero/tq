//! Automatic JSON input must preserve jq's root boundaries before optimization.

use std::{
    io::{Cursor, Write},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use tq_cli::{Command as TqCommand, ExecutionOverride, ExitStatus, parse_args, run_with_io};
use tq_core::{PlanKind, ResolveOptions, analyze, parse, resolve};

#[derive(Debug, Eq, PartialEq)]
struct Observation {
    status: Result<ExitStatus, ExitStatus>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn execute(arguments: &[&str], input: &[u8], execution_override: ExecutionOverride) -> Observation {
    let command = parse_args(arguments).expect("root-boundary arguments parse");
    let mut command = command;
    let TqCommand::Run(options) = &mut command else {
        panic!("root-boundary arguments must produce a run command");
    };
    options.execution_override = execution_override;

    let mut input = Cursor::new(input);
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let status =
        run_with_io(command, &mut input, &mut stdout, &mut stderr).map_err(|error| error.status());
    Observation {
        status,
        stdout,
        stderr,
    }
}

fn process(arguments: &[&str], input: &[u8]) -> Observation {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tq");
    child
        .stdin
        .take()
        .expect("stdin pipe")
        .write_all(input)
        .expect("write input");
    let output = wait_with_deadline(child, Duration::from_secs(2));
    let code = output.status.code().expect("ordinary exit");
    let status = if code == i32::from(ExitStatus::Success.code()) {
        Ok(ExitStatus::Success)
    } else {
        Err(match code {
            5 => ExitStatus::Input,
            other => panic!("unexpected root-boundary process status {other}"),
        })
    };
    Observation {
        status,
        stdout: output.stdout,
        stderr: output.stderr,
    }
}

fn wait_with_deadline(mut child: std::process::Child, timeout: Duration) -> std::process::Output {
    let deadline = Instant::now() + timeout;
    loop {
        if child.try_wait().expect("poll tq").is_some() {
            return child.wait_with_output().expect("collect tq output");
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("tq did not terminate before the deadline");
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn assert_subtree_proof(query: &str) {
    let analyzed = analyze(
        resolve(
            parse(query).expect("query parses"),
            &ResolveOptions::default(),
        )
        .expect("query resolves"),
    );
    assert_eq!(analyzed.analysis().selected_plan, PlanKind::Subtree);
    assert!(analyzed.analysis().stream_proof.is_some());
}

#[test]
fn explicit_json_automatic_execution_preserves_each_whitespace_root() {
    let arguments = ["-ijson", "-ojson", "-c", ".features[].id"];
    let input = br#"{"features":[{"id":1}]} {"features":[{"id":2}]}"#;
    let automatic = execute(&arguments, input, ExecutionOverride::Automatic);
    let document = execute(&arguments, input, ExecutionOverride::Document);

    assert_eq!(document.status, Ok(ExitStatus::Success));
    assert_eq!(document.stdout, b"1\n2\n");
    assert_eq!(automatic, document);
}

#[test]
fn automatic_detection_preserves_each_whitespace_root() {
    let arguments = ["-ojson", "-c", ".features[].id"];
    let input = br#"{"features":[{"id":1}]} {"features":[{"id":2}]}"#;
    let automatic = execute(&arguments, input, ExecutionOverride::Automatic);
    let document = execute(&arguments, input, ExecutionOverride::Document);

    assert_eq!(document.status, Ok(ExitStatus::Success));
    assert_eq!(document.stdout, b"1\n2\n");
    assert_eq!(automatic, document);
}

#[test]
fn duplicate_object_keys_use_the_last_value_before_vm_evaluation() {
    let arguments = ["-ijson", "-ojson", "-c", ".features[].id"];
    let input = br#"{"features":[{"id":1}],"features":[]}"#;
    let automatic = execute(&arguments, input, ExecutionOverride::Automatic);
    let document = execute(&arguments, input, ExecutionOverride::Document);

    assert_eq!(document.status, Ok(ExitStatus::Success));
    assert_eq!(document.stdout, [] as [u8; 0]);
    assert_eq!(automatic, document);
}

#[test]
fn selected_prefixes_reset_between_validated_roots() {
    let arguments = ["-ijson", "-ojson", "-c", ".features[].id"];
    let input = br#"{"features":[{"id":1},{"id":2}]} {"features":[{"id":3}]}"#;
    let automatic = execute(&arguments, input, ExecutionOverride::Automatic);
    let document = execute(&arguments, input, ExecutionOverride::Document);

    assert_eq!(document.status, Ok(ExitStatus::Success));
    assert_eq!(document.stdout, b"1\n2\n3\n");
    assert_eq!(automatic, document);
}

#[test]
fn selected_prefix_keeps_missing_empty_object_and_null_iteration_semantics() {
    let arguments = ["-ijson", "-ojson", "-c", ".features[].id"];
    for (input, expected_status, expected_stdout) in [
        (br"{}".as_slice(), Ok(ExitStatus::Runtime), b"".as_slice()),
        (
            br#"{"other":1}"#.as_slice(),
            Ok(ExitStatus::Runtime),
            b"".as_slice(),
        ),
        (
            br#"{"features":{}}"#.as_slice(),
            Ok(ExitStatus::Success),
            b"".as_slice(),
        ),
        (
            br#"{"features":null}"#.as_slice(),
            Ok(ExitStatus::Runtime),
            b"".as_slice(),
        ),
    ] {
        let automatic = execute(&arguments, input, ExecutionOverride::Automatic);
        let document = execute(&arguments, input, ExecutionOverride::Document);

        assert_eq!(document.status, expected_status);
        assert_eq!(document.stdout, expected_stdout);
        assert_eq!(automatic, document);
    }
}

#[test]
fn automatic_subtree_preserves_siblings_in_projected_objects() {
    let arguments = ["-ijson", "-ojson", "-c", ".features[] | {a,b}"];
    let input = br#"{"features":[{"a":{"nested":[1,2]},"b":"first"},{"a":0,"b":{"keep":true}}]}"#;
    let automatic = execute(&arguments, input, ExecutionOverride::Automatic);
    let document = execute(&arguments, input, ExecutionOverride::Document);

    assert_eq!(document.status, Ok(ExitStatus::Success));
    assert_eq!(
        document.stdout,
        b"{\"a\":{\"nested\":[1,2]},\"b\":\"first\"}\n{\"a\":0,\"b\":{\"keep\":true}}\n"
    );
    assert_eq!(automatic, document);
}

#[test]
fn automatic_preserves_sibling_after_duplicate_nested_container() {
    let arguments = ["-ijson", "-ojson", "-c", ".features[] | {a,b}"];
    let input = br#"{"features":[{"a":{"old":1},"b":2,"a":{"new":[3,4]}}]}"#;
    let automatic = execute(&arguments, input, ExecutionOverride::Automatic);
    let document = execute(&arguments, input, ExecutionOverride::Document);

    assert_eq!(document.status, Ok(ExitStatus::Success));
    assert_eq!(document.stdout, b"{\"a\":{\"new\":[3,4]},\"b\":2}\n");
    assert_eq!(automatic, document);
}

#[test]
fn automatic_subtree_replaces_duplicate_nested_a_types() {
    let query = ".features[].a.b";
    assert_subtree_proof(query);
    let arguments = ["-ijson", "-ojson", "-c", query];
    for (input, expected_status, expected_stdout) in [
        (
            br#"{"features":[{"a":{"b":1},"a":2}]}"#.as_slice(),
            Ok(ExitStatus::Runtime),
            b"".as_slice(),
        ),
        (
            br#"{"features":[{"a":{"b":2},"a":[]}]}"#.as_slice(),
            Ok(ExitStatus::Runtime),
            b"".as_slice(),
        ),
        (
            br#"{"features":[{"a":{"b":3},"a":{"b":4}}]}"#.as_slice(),
            Ok(ExitStatus::Success),
            b"4\n".as_slice(),
        ),
    ] {
        let automatic = execute(&arguments, input, ExecutionOverride::Automatic);
        let document = execute(&arguments, input, ExecutionOverride::Document);

        assert_eq!(document.status, expected_status);
        assert_eq!(document.stdout, expected_stdout);
        assert_eq!(automatic, document);
    }
}

#[test]
fn automatic_subtree_preserves_nested_array_indexing() {
    let query = ".features[].a[0][1]";
    assert_subtree_proof(query);
    let arguments = ["-ijson", "-ojson", "-c", query];
    let input = br#"{"features":[{"a":[[10,11],[12]]},{"a":[[20,21],[22]]}]}"#;
    let automatic = execute(&arguments, input, ExecutionOverride::Automatic);
    let document = execute(&arguments, input, ExecutionOverride::Document);

    assert_eq!(document.status, Ok(ExitStatus::Success));
    assert_eq!(document.stdout, b"11\n21\n");
    assert_eq!(automatic, document);
}

#[test]
fn automatic_subtree_recovers_after_wrong_nested_type_in_an_earlier_root() {
    let query = ".features[].a.b";
    assert_subtree_proof(query);
    let arguments = ["-ijson", "-ojson", "-c", query];
    let input = br#"{"features":[{"a":0}]} {"features":[{"a":{"b":7}}]}"#;
    let automatic = execute(&arguments, input, ExecutionOverride::Automatic);
    let document = execute(&arguments, input, ExecutionOverride::Document);

    assert_eq!(document.status, Ok(ExitStatus::Success));
    assert_eq!(document.stdout, b"7\n");
    assert_eq!(automatic, document);
}

#[test]
fn automatic_subtree_keeps_first_object_key_order_with_nested_replacement() {
    let query = ".features[].nested.value";
    assert_subtree_proof(query);
    let arguments = ["-ijson", "-ojson", "-c", query];
    let input = br#"{"features":{"a":{"nested":{"value":1}},"b":{"nested":{"value":2}},"a":{"nested":{"value":3}}}}"#;
    let automatic = execute(&arguments, input, ExecutionOverride::Automatic);
    let document = execute(&arguments, input, ExecutionOverride::Document);

    assert_eq!(document.status, Ok(ExitStatus::Success));
    assert_eq!(document.stdout, b"3\n2\n");
    assert_eq!(automatic, document);
}

#[test]
fn automatic_json_preserves_scalar_and_array_root_type_errors() {
    let arguments = ["-ijson", "-ojson", "-c", ".features[].id"];
    for input in [br"0".as_slice(), br"[]".as_slice(), br"[1]".as_slice()] {
        let automatic = execute(&arguments, input, ExecutionOverride::Automatic);
        let document = execute(&arguments, input, ExecutionOverride::Document);

        assert_eq!(document.status, Ok(ExitStatus::Runtime));
        assert_eq!(document.stdout, [] as [u8; 0]);
        assert_eq!(automatic, document);
    }
}

#[test]
fn automatic_json_preserves_wrong_type_at_an_array_index_ancestor() {
    let arguments = ["-ijson", "-ojson", "-c", ".items[1].features[].id"];
    let input = br#"{"items":[{"features":[{"id":1}]},7]}"#;
    let automatic = execute(&arguments, input, ExecutionOverride::Automatic);
    let document = execute(&arguments, input, ExecutionOverride::Document);

    assert_eq!(document.status, Ok(ExitStatus::Runtime));
    assert_eq!(document.stdout, [] as [u8; 0]);
    assert_eq!(automatic, document);
}

#[test]
fn duplicate_object_iteration_keys_keep_last_value_in_first_key_order() {
    let arguments = ["-ojson", "-c", ".features[].id"];
    let input = br#"{"features":{"a":{"id":1},"b":{"id":2},"a":{"id":3}}}"#;
    let automatic = execute(&arguments, input, ExecutionOverride::Automatic);
    let document = execute(&arguments, input, ExecutionOverride::Document);

    assert_eq!(document.status, Ok(ExitStatus::Success));
    assert_eq!(document.stdout, b"3\n2\n");
    assert_eq!(automatic, document);
}

#[test]
fn nested_duplicate_key_replaced_by_empty_object_has_no_old_value_effect() {
    let arguments = ["-ijson", "-ojson", "-c", ".features[].item | debug"];
    let input = br#"{"features":[{"item":{"id":1},"item":{}}]}"#;
    let automatic = execute(&arguments, input, ExecutionOverride::Automatic);
    let document = execute(&arguments, input, ExecutionOverride::Document);

    assert_eq!(document.status, Ok(ExitStatus::Success));
    assert_eq!(document.stdout, b"{}\n");
    assert_eq!(document.stderr, b"[\"DEBUG:\",{}]\n");
    assert!(!document.stderr.windows(4).any(|window| window == b"\"id\""));
    assert_eq!(automatic, document);
}

#[test]
fn duplicate_ancestor_removes_selected_descendant_instead_of_reusing_old_value() {
    let arguments = ["-ijson", "-ojson", "-c", ".outer.features[].id"];
    let input = br#"{"outer":{"features":[{"id":1}]},"outer":{}}"#;
    let automatic = execute(&arguments, input, ExecutionOverride::Automatic);
    let document = execute(&arguments, input, ExecutionOverride::Document);

    assert_eq!(document.status, Ok(ExitStatus::Runtime));
    assert_eq!(document.stdout, [] as [u8; 0]);
    assert_eq!(automatic, document);
}

#[test]
fn hybrid_sort_resets_blocking_state_at_each_json_root() {
    let arguments = ["-ijson", "-ojson", "-c", "[.items[].value] | sort"];
    let input = br#"{"items":[{"value":2},{"value":1}]} {"items":[{"value":4},{"value":3}]}"#;
    let automatic = execute(&arguments, input, ExecutionOverride::Automatic);
    let document = execute(&arguments, input, ExecutionOverride::Document);

    assert_eq!(document.status, Ok(ExitStatus::Success));
    assert_eq!(document.stdout, b"[1,2]\n[3,4]\n");
    assert_eq!(automatic, document);
}

#[test]
fn prior_valid_root_is_published_before_a_later_malformed_root() {
    let arguments = ["-ijson", "-ojson", "-c", ".features[].id"];
    let input = br#"{"features":[{"id":1}]} {"features":[{"id":2}],"discarded":[1,]}"#;
    let automatic = execute(&arguments, input, ExecutionOverride::Automatic);
    let document = execute(&arguments, input, ExecutionOverride::Document);

    assert_eq!(document.status, Err(ExitStatus::Input));
    assert_eq!(document.stdout, b"1\n");
    assert_eq!(automatic, document);
}

#[test]
fn malformed_root_does_not_publish_partial_values_or_debug_effects() {
    let input = br#"{"features":[{"id":1},{"id":2}],"discarded":[1,]}"#;
    let output = process(&["-ijson", "-ojson", "-c", ".features[].id | debug"], input);

    assert_eq!(output.status, Err(ExitStatus::Input));
    assert!(output.stdout.is_empty(), "partial values were published");
    assert!(!String::from_utf8_lossy(&output.stderr).contains("DEBUG"));
}

#[test]
fn malformed_root_is_validated_before_vm_evaluation() {
    let input = br#"{"features":[{"id":1},{"id":2}],"discarded":[1,]}"#;
    let output = process(
        &[
            "-ijson",
            "-ojson",
            "-c",
            ".features[].id | error(\"must-not-run\")",
        ],
        input,
    );

    assert_eq!(output.status, Err(ExitStatus::Input));
    assert!(output.stdout.is_empty(), "partial values were published");
    assert!(!String::from_utf8_lossy(&output.stderr).contains("must-not-run"));
}

#[test]
fn runtime_error_precedes_a_late_parse_error() {
    let input = br#"{"ok":1} {"bad":[1,]}"#;
    let output = process(&["-ijson", "-ojson", "-c", "error(\"vm-marker\")"], input);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let lines = stderr.lines().collect::<Vec<_>>();

    assert_eq!(output.status, Err(ExitStatus::Input));
    let runtime = lines
        .iter()
        .position(|line| line.contains("vm-marker"))
        .expect("runtime error diagnostic");
    let parse = lines
        .iter()
        .position(|line| line.contains("input rejected") && !line.contains("vm-marker"))
        .expect("late parse diagnostic");
    assert!(runtime < parse, "diagnostics were reordered: {stderr}");
}

#[test]
fn automatic_runtime_error_stops_only_the_current_root() {
    let arguments = [
        "-ijson",
        "-ojson",
        "-c",
        ".features[].id | if . == 1 then error(\"bad\") else debug end",
    ];
    let input = br#"{"features":[{"id":0},{"id":1},{"id":2}]} {"features":[{"id":3}]}"#;
    let document = execute(&arguments, input, ExecutionOverride::Document);
    let automatic = execute(&arguments, input, ExecutionOverride::Automatic);
    assert_eq!(document.stdout, b"0\n3\n");
    assert_eq!(automatic, document);
}
