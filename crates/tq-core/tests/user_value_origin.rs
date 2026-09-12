//! Runtime value-origin behavior for managed assignments.

use tq_core::{ResolveOptions, Value, Vm, VmError, VmLimits, analyze, parse, resolve};

fn evaluate(query: &str, input: &str) -> Result<Vec<Value>, VmError> {
    let input = Value::from_json(serde_json::from_str(input).expect("valid JSON"))
        .expect("JSON converts to a tq value");
    let resolved = resolve(
        parse(query).expect("query parses"),
        &ResolveOptions::default(),
    )
    .expect("query resolves");
    let plan = analyze(resolved)
        .compile()
        .expect("query compiles")
        .document_plan();
    let mut vm = Vm::new(&plan, input, VmLimits::default());
    let mut values = Vec::new();
    while let Some(value) = vm.next_result()? {
        values.push(value);
    }
    Ok(values)
}

fn assert_assignment_error(query: &str, input: &str) {
    match evaluate(query, input) {
        Err(VmError::Runtime { message }) => {
            assert!(
                message.contains("assignment left side"),
                "unexpected runtime error: {message}"
            );
        }
        other => panic!("expected assignment path error, got {other:?}"),
    }
}

#[test]
fn identity_and_literal_root_aliases_have_distinct_origins() {
    assert_eq!(
        evaluate("1 | . as $p | ($p = 2)", "null").expect("identity stays anchored"),
        [Value::from_json(serde_json::json!(2)).unwrap()]
    );
    assert_assignment_error("1 | 1 as $p | ($p = 2)", "null");
}

#[test]
fn computed_values_do_not_inherit_input_origin() {
    assert_assignment_error("1 | (. + 0) as $p | ($p = 2)", "null");
}

#[test]
fn constructed_value_can_become_assignment_input_after_capture() {
    assert_eq!(
        evaluate("{} as $p | $p | ($p.x = 2)", "null")
            .expect("captured constructed root can flow through identity"),
        [Value::from_json(serde_json::json!({"x": 2})).unwrap()]
    );
}

#[test]
fn destructured_child_origin_tracks_its_member_path() {
    assert_eq!(
        evaluate(". as {a: $p} | .a | ($p.x = 2)", r#"{"a":{"b":1}}"#)
            .expect("destructured child remains anchored"),
        [Value::from_json(serde_json::json!({"b":1,"x":2})).unwrap()]
    );
}

#[test]
fn changing_input_invalidates_a_root_alias() {
    assert_assignment_error(". as $p | .a | ($p.x = 2)", r#"{"a":{"b":1}}"#);
}

#[test]
fn alias_chains_preserve_runtime_identity() {
    assert_eq!(
        evaluate(". as $p | ($p as $q | $q.x = 2)", r#"{"x":1}"#)
            .expect("nested root aliases preserve identity"),
        [Value::from_json(serde_json::json!({"x":2})).unwrap()]
    );
}

#[test]
fn filter_pipe_and_try_wrapper_preserve_identity() {
    assert_eq!(
        evaluate(
            "def pass(f): f; ((pass(.) | .) as $p | $p.x) = 2",
            r#"{"x":1}"#,
        )
        .expect("filter pipe preserves identity"),
        [Value::from_json(serde_json::json!({"x":2})).unwrap()]
    );
    assert_eq!(
        evaluate("((try . catch .) as $p | $p.x) = 2", r#"{"x":1}"#)
            .expect("try wrapper preserves identity"),
        [Value::from_json(serde_json::json!({"x":2})).unwrap()]
    );
}

#[test]
fn conditional_identity_branch_preserves_input_origin() {
    assert_eq!(
        evaluate(
            "((if .a then . else 7 end) as $p | $p.x) = 2",
            r#"{"a":true,"x":1}"#,
        )
        .expect("identity conditional branch preserves origin"),
        [Value::from_json(serde_json::json!({"a":true,"x":2})).unwrap()]
    );
}

#[test]
fn index_operands_keep_the_caller_origin() {
    assert_eq!(
        evaluate("1 | . as $p | [10,20][($p = 0)]", "null")
            .expect("index assignment keeps the caller origin"),
        [Value::from_json(serde_json::json!(10)).unwrap()]
    );
    assert_eq!(
        evaluate("[10,20] | . as $p | .[($p | .[0] = 0 | 1)]", "null",)
            .expect("nested index filter keeps the caller origin"),
        [Value::from_json(serde_json::json!(20)).unwrap()]
    );
}

#[test]
fn value_parameter_preserves_outer_root_origin() {
    assert_eq!(
        evaluate("def setx($p): $p.x = 2; . as $p | setx($p)", r#"{"x":1}"#)
            .expect("root alias survives a value-parameter user call"),
        [Value::from_json(serde_json::json!({"x": 2})).unwrap()]
    );
}

#[test]
fn no_op_trimming_preserves_captured_input_origin() {
    for filter in [
        "trim",
        "ltrim",
        "rtrim",
        "ltrimstr(\"x\")",
        "rtrimstr(\"x\")",
        "trimstr(\"x\")",
    ] {
        for input in [r#""abc""#, r#""a b""#] {
            let query = format!("def apply(f): f; . as $p | apply({filter}) | ($p = \"ok\")");
            assert_eq!(
                evaluate(&query, input).expect("no-op trimming retains the input origin"),
                [Value::from_json(serde_json::json!("ok")).unwrap()],
                "filter: {filter}, input: {input}",
            );
        }
    }
}

#[test]
fn trimming_transform_branches_do_not_preserve_captured_input_origin() {
    for (filter, input) in [
        ("trim", " abc "),
        ("ltrim", " abc"),
        ("rtrim", "abc "),
        ("ltrimstr(\"a\")", "abc"),
        ("rtrimstr(\"c\")", "abc"),
        ("trimstr(\"a\")", "abc"),
        // An empty affix still selects jq's slicing branch. Equal bytes do
        // not imply the same runtime path identity.
        ("ltrimstr(\"\")", "abc"),
        ("rtrimstr(\"\")", "abc"),
        ("trimstr(\"\")", "abc"),
    ] {
        let query = format!("def apply(f): f; . as $p | apply({filter}) | ($p = \"ok\")");
        assert_assignment_error(&query, &serde_json::to_string(input).unwrap());
    }
}
