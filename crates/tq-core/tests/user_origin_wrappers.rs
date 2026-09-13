//! Public VM regressions for input origins through wrapper and destructuring paths.

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
fn false_conditional_wrapper_keeps_current_path_writable() {
    assert_eq!(
        evaluate(
            ". as $p | (if false then (7 as $unused | .) else . end) | .x = 2",
            r#"{"x":1}"#,
        )
        .expect("false conditional evaluates"),
        [Value::from_json(serde_json::json!({"x": 2})).unwrap()]
    );
}

#[test]
fn mixed_comma_current_value_remains_writable() {
    assert_eq!(
        evaluate("((., 7) as $p | .x) = 2", r#"{"x":1}"#).expect("current-value branch evaluates"),
        [Value::from_json(serde_json::json!({"x": 2})).unwrap()]
    );
}

#[test]
fn mixed_comma_alias_value_rejects_nonroot_assignment() {
    assert_assignment_error("((., 7) as $p | $p.x) = 2", r#"{"x":1}"#);
}

#[test]
fn array_destructure_selected_child_preserves_member_origin() {
    assert_eq!(
        evaluate(". as [$p] | .[0] | ($p.x = 2)", r#"[{"b":1}]"#,)
            .expect("selected array child evaluates"),
        [Value::from_json(serde_json::json!({"b": 1, "x": 2})).unwrap()]
    );
}

#[test]
fn array_destructure_root_alias_without_selection_rejects_assignment() {
    assert_assignment_error(". as [$p] | ($p.x = 2)", r#"[{"b":1}]"#);
}
