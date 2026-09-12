//! Automatic JSON capture pushdown must retain DOM semantics at capture paths.

use std::{ffi::OsString, io::Cursor};

use tq_cli::{Command, ExecutionOverride, ExitStatus, parse_args, run_with_io};
use tq_core::{AutomaticPlan, Compiled, Plan, ResolveOptions, analyze, parse, resolve};

#[derive(Debug, Eq, PartialEq)]
struct Observation {
    status: Result<ExitStatus, ExitStatus>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn execute<I, S>(arguments: I, input: &[u8], execution_override: ExecutionOverride) -> Observation
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut command = parse_args(arguments).expect("capture-pushdown arguments parse");
    let Command::Run(options) = &mut command else {
        panic!("capture-pushdown arguments must produce a run command");
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

fn assert_capture_plan<M>(plan: &Plan<Compiled, M>) {
    assert!(plan.automatic_projection().is_none());
    assert!(plan.automatic_capture_paths().is_some());
}

fn assert_capture_proof(query: &str) {
    let automatic = analyze(
        resolve(
            parse(query).expect("capture-pushdown query parses"),
            &ResolveOptions::default(),
        )
        .expect("capture-pushdown query resolves"),
    )
    .compile()
    .expect("capture-pushdown query compiles")
    .automatic_plan()
    .expect("capture-pushdown query admits an automatic plan");

    match automatic {
        AutomaticPlan::Events(plan) => assert_capture_plan(&plan),
        AutomaticPlan::Subtree(plan) => assert_capture_plan(&plan),
        AutomaticPlan::HybridBlocking(plan) => assert_capture_plan(&plan),
        other => panic!("expected a capture-admitted automatic plan, got {other:?}"),
    }
}

fn assert_same(
    arguments: &[&str],
    input: &[u8],
    expected_status: Result<ExitStatus, ExitStatus>,
    expected_stdout: &[u8],
) {
    let automatic = execute(arguments, input, ExecutionOverride::Automatic);
    let document = execute(arguments, input, ExecutionOverride::Document);

    assert_eq!(automatic, document, "automatic capture diverged from DOM");
    assert_eq!(automatic.status, expected_status);
    assert_eq!(automatic.stdout, expected_stdout);
}

fn json_arguments(query: &str) -> [&str; 4] {
    ["-ijson", "-ojson", "-c", query]
}

#[test]
fn array_ancestor_preserves_missing_null_scalar_wrong_kind_and_empty() {
    let query = ".groups[] | select(.child.id >= 0) | .id";
    assert_capture_proof(query);

    let cases = [
        (
            br#"{"groups":[{"id":1,"child":{"id":2}},{"id":3,"child":{"id":4}}]}"#.as_slice(),
            Ok(ExitStatus::Success),
            b"1\n3\n".as_slice(),
        ),
        (br"{}".as_slice(), Ok(ExitStatus::Runtime), b"".as_slice()),
        (
            br#"{"groups":null}"#.as_slice(),
            Ok(ExitStatus::Runtime),
            b"".as_slice(),
        ),
        (
            br#"{"groups":7}"#.as_slice(),
            Ok(ExitStatus::Runtime),
            b"".as_slice(),
        ),
        (
            br#"{"groups":"wrong"}"#.as_slice(),
            Ok(ExitStatus::Runtime),
            b"".as_slice(),
        ),
        (
            br#"{"groups":[]}"#.as_slice(),
            Ok(ExitStatus::Success),
            b"".as_slice(),
        ),
        (
            br#"{"groups":[{}]}"#.as_slice(),
            Ok(ExitStatus::Success),
            b"".as_slice(),
        ),
        (
            br#"{"groups":[{"id":1,"child":null}]}"#.as_slice(),
            Ok(ExitStatus::Success),
            b"".as_slice(),
        ),
        (
            br#"{"groups":[{"id":1,"child":0}]}"#.as_slice(),
            Ok(ExitStatus::Runtime),
            b"".as_slice(),
        ),
    ];

    for (input, status, stdout) in cases {
        assert_same(&json_arguments(query), input, status, stdout);
    }
}

#[test]
fn nested_array_ancestors_match_dom_for_missing_null_scalar_and_empty() {
    let query = ".groups[].items[].id";
    let cases = [
        (
            br#"{"groups":[{"items":[{"id":1},{"id":2}]}]}"#.as_slice(),
            Ok(ExitStatus::Success),
            b"1\n2\n".as_slice(),
        ),
        (
            br#"{"groups":[{}]}"#.as_slice(),
            Ok(ExitStatus::Runtime),
            b"".as_slice(),
        ),
        (
            br#"{"groups":[{"items":null}]}"#.as_slice(),
            Ok(ExitStatus::Runtime),
            b"".as_slice(),
        ),
        (
            br#"{"groups":[{"items":7}]}"#.as_slice(),
            Ok(ExitStatus::Runtime),
            b"".as_slice(),
        ),
        (
            br#"{"groups":[{"items":[]}]}"#.as_slice(),
            Ok(ExitStatus::Success),
            b"".as_slice(),
        ),
    ];

    for (input, status, stdout) in cases {
        assert_same(&json_arguments(query), input, status, stdout);
    }
}

#[test]
fn empty_and_all_discarded_objects_keep_result_cardinality() {
    let empty_object = execute(
        json_arguments(".groups[] | {}"),
        br#"{"groups":[{}, {"id":1}, {"unused":{"x":2}}]}"#,
        ExecutionOverride::Automatic,
    );
    let empty_object_dom = execute(
        json_arguments(".groups[] | {}"),
        br#"{"groups":[{}, {"id":1}, {"unused":{"x":2}}]}"#,
        ExecutionOverride::Document,
    );
    assert_eq!(empty_object, empty_object_dom);
    assert_eq!(empty_object.status, Ok(ExitStatus::Success));
    assert_eq!(empty_object.stdout, b"{}\n{}\n{}\n");
    assert!(empty_object.stderr.is_empty());

    assert_same(
        &json_arguments(".groups[] | select(false)"),
        br#"{"groups":[{}, {"id":1}, {"unused":{"x":2}}]}"#,
        Ok(ExitStatus::Success),
        b"",
    );
}

#[test]
fn capture_empty_shells_preserve_missing_value_cardinality() {
    let query = ".groups[] | select(true) | .id";
    assert_capture_proof(query);
    assert_same(
        &json_arguments(query),
        br#"{"groups":[{}, {"unused":1}, {"unused":{"x":2}}]}"#,
        Ok(ExitStatus::Success),
        b"null\nnull\nnull\n",
    );
}

#[test]
fn duplicate_root_and_nested_ancestors_replace_old_captures() {
    let query = ".groups[] | select(.child.id >= 0) | .id";
    assert_capture_proof(query);

    assert_same(
        &json_arguments(query),
        br#"{"groups":[{"id":1,"child":{"id":2}},{"id":2,"child":{"id":3}}]}"#,
        Ok(ExitStatus::Success),
        b"1\n2\n",
    );
    assert_same(
        &json_arguments(query),
        br#"{"groups":[{"id":1,"child":{"id":2},"child":{"id":3}}]}"#,
        Ok(ExitStatus::Success),
        b"1\n",
    );
    assert_same(
        &json_arguments(query),
        br#"{"groups":[{"id":1,"child":{"id":2},"child":0}]}"#,
        Ok(ExitStatus::Runtime),
        b"",
    );
    assert_same(
        &json_arguments(query),
        br#"{"groups":[{"id":1,"child":{"id":2}}],"groups":[{"id":3,"child":{"id":4}}]}"#,
        Ok(ExitStatus::Success),
        b"3\n",
    );
}

#[test]
fn duplicate_nested_ancestor_preserves_prior_item_and_empty_replacement() {
    let query = ".groups[] | select(.id >= 0) | .child.id";
    assert_capture_proof(query);
    assert_same(
        &json_arguments(query),
        br#"{"groups":[{"id":1,"child":{"id":2}},{"id":2,"child":{"id":3},"child":{"unused":9}}]}"#,
        Ok(ExitStatus::Success),
        b"2\nnull\n",
    );
}

#[test]
fn array_child_at_object_key_matches_dom_shape_error() {
    let query = ".groups[] | select(.child.id >= 0) | .id";
    assert_capture_proof(query);
    assert_same(
        &json_arguments(query),
        br#"{"groups":[{"id":1,"child":[2,3]}]}"#,
        Ok(ExitStatus::Runtime),
        b"",
    );
}

#[test]
fn sibling_capture_paths_preserve_order_and_shape() {
    let query = ".groups[] | select(.left.value < .right.value) | .id";
    assert_capture_proof(query);
    let input = br#"{"groups":[{"id":1,"left":{"value":1},"right":{"value":2},"unused":9},{"id":2,"left":{"value":3},"right":{"value":4},"unused":10}]}"#;
    assert_same(
        &json_arguments(query),
        input,
        Ok(ExitStatus::Success),
        b"1\n2\n",
    );
}

#[test]
fn terminal_whole_child_capture_preserves_nested_values() {
    let query = ".groups[] | select(.payload.keep == true) | .payload";
    assert_capture_proof(query);
    let input = br#"{"groups":[{"payload":{"keep":true,"nested":{"x":[1,2]}}},{"payload":{"keep":false,"nested":{"x":[3]}}},{"payload":{"keep":true}}]}"#;
    assert_same(
        &json_arguments(query),
        input,
        Ok(ExitStatus::Success),
        br#"{"keep":true,"nested":{"x":[1,2]}}
{"keep":true}
"#,
    );
}

#[test]
fn malformed_discarded_values_and_limits_fail_before_capture_output() {
    let query = ".g[] | select(.c.i >= 0) | .i";
    assert_capture_proof(query);
    let malformed = json_arguments(query);
    assert_same(
        &malformed,
        br#"{"g":[{"i":1,"c":{"i":2},"x":1e}]}"#,
        Err(ExitStatus::Input),
        b"",
    );

    let depth = ["-ijson", "-ojson", "-c", "--max-depth", "4", query];
    assert_same(
        &depth,
        br#"{"g":[{"i":1,"c":{"i":2},"x":{"y":{"z":true}}}]}"#,
        Err(ExitStatus::Resource),
        b"",
    );

    let token = ["-ijson", "-ojson", "-c", "--max-token-bytes", "3", query];
    assert_same(
        &token,
        br#"{"g":[{"i":1,"c":{"i":2},"x":12345}]}"#,
        Err(ExitStatus::Resource),
        b"",
    );
}

#[test]
fn native_toon_capture_contract_is_unchanged() {
    let arguments = ["-itoon", "-ojson", "-c", ".items[] | .a"];
    let input = b"items[2]{a}:\n  1\n  2\n";
    assert_same(&arguments, input, Ok(ExitStatus::Success), b"1\n2\n");
}
