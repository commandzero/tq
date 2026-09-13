//! Public origin-token regressions for bounded path reconstruction.

use tq_core::{ResolveOptions, Value, Vm, VmError, VmLimits, analyze, parse, resolve};

fn evaluate_with_limits(query: &str, input: &str, limits: VmLimits) -> Result<Vec<Value>, VmError> {
    let input = Value::from_json(serde_json::from_str(input).expect("valid JSON input"))
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
    let mut vm = Vm::new(&plan, input, limits);
    let mut values = Vec::new();
    while let Some(value) = vm.next_result()? {
        values.push(value);
    }
    Ok(values)
}

#[test]
fn deep_selected_alias_assignment_observes_path_stack() {
    let query = r"(.a | .b | .c | .d | .e) = 2";
    let input = r#"{"a":{"b":{"c":{"d":{"e":0}}}}}"#;
    let expected = [Value::from_json(serde_json::json!({
        "a": { "b": { "c": { "d": { "e": 2 } } } }
    }))
    .unwrap()];
    assert_eq!(
        evaluate_with_limits(query, input, VmLimits::default()).expect("pinned jq result"),
        expected
    );

    let error = evaluate_with_limits(
        query,
        input,
        VmLimits {
            path_stack: 2,
            ..VmLimits::default()
        },
    )
    .expect_err("deep selected aliases must not grow beyond path_stack");
    assert!(matches!(
        error,
        VmError::Resource {
            resource: "path-stack"
        }
    ));
}

#[test]
fn getpath_object_child_keeps_assignment_origin() {
    let values = evaluate_with_limits(
        r#"def update: (getpath(["a"]) | .x) = 2; update"#,
        r#"{"a":{"x":1}}"#,
        VmLimits::default(),
    )
    .expect("pinned jq getpath child assignment");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!({ "a": { "x": 2 } })).unwrap()]
    );
}

#[test]
fn getpath_scalar_child_keeps_assignment_origin() {
    let values = evaluate_with_limits(
        r#"def update: (getpath(["a","x"]) | .) = 2; update"#,
        r#"{"a":{"x":1}}"#,
        VmLimits::default(),
    )
    .expect("pinned jq scalar child assignment");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!({ "a": { "x": 2 } })).unwrap()]
    );
}

#[test]
fn origin_child_growth_is_bounded_during_deep_reconstruction() {
    let query = r"(.a | .b | .c | .d | .e | .f) = 3";
    let input = r#"{"a":{"b":{"c":{"d":{"e":{"f":0}}}}}}"#;
    let error = evaluate_with_limits(
        query,
        input,
        VmLimits {
            path_stack: 3,
            steps: 128,
            ..VmLimits::default()
        },
    )
    .expect_err("origin path growth must be bounded before reconstruction");
    assert!(matches!(
        error,
        VmError::Resource {
            resource: "path-stack"
        }
    ));
}

#[test]
fn selector_only_origin_growth_is_bounded_before_output() {
    let error = evaluate_with_limits(
        r"def select_deep: .a | .b | .c | .d | .e; select_deep",
        r#"{"a":{"b":{"c":{"d":{"e":1}}}}}"#,
        VmLimits {
            path_stack: 2,
            ..VmLimits::default()
        },
    )
    .expect_err("selector-only origin paths must remain bounded");
    assert!(matches!(
        error,
        VmError::Resource {
            resource: "path-stack"
        }
    ));
}

#[test]
fn called_path_builtins_normalize_numeric_indices_like_jq() {
    let gets = evaluate_with_limits(
        r"def get($path): getpath($path); [get([-1]), get([1.5]), get([-5]), get([nan]), get([infinite])]",
        "[10,20,30]",
        VmLimits::default(),
    )
    .expect("getpath numeric probes evaluate");
    assert_eq!(
        gets,
        [Value::from_json(serde_json::json!([30, 20, null, null, null])).unwrap()]
    );

    let sets = evaluate_with_limits(
        r"def set($path): setpath($path; 9); [set([-1]), set([1.5])]",
        "[10,20,30]",
        VmLimits::default(),
    )
    .expect("setpath numeric probes evaluate");
    assert_eq!(
        sets,
        [Value::from_json(serde_json::json!([[10, 20, 9], [10, 9, 30]])).unwrap()]
    );

    let deletes = evaluate_with_limits(
        r"def drop($paths): delpaths($paths); [drop([[-1]]), drop([[-5]]), drop([[-1],[0]])]",
        "[10,20,30]",
        VmLimits::default(),
    )
    .expect("delpaths numeric probes evaluate");
    assert_eq!(
        deletes,
        [Value::from_json(serde_json::json!([[10, 20], [10, 20, 30], [20]])).unwrap()]
    );
}

#[test]
fn called_path_builtins_keep_missing_and_invalid_component_distinctions() {
    let get_error = evaluate_with_limits(
        r#"def get($path): getpath($path); try get([-5,true]) catch "caught""#,
        "[1]",
        VmLimits::default(),
    )
    .expect("getpath component error is catchable");
    assert_eq!(get_error, [Value::string("caught")]);

    let set_error = evaluate_with_limits(
        r#"def set($path): setpath($path; 9); [try set([nan]) catch "nan", try set([infinite]) catch "infinite"]"#,
        "[1]",
        VmLimits::default(),
    )
    .expect("setpath nonfinite errors are catchable");
    assert_eq!(
        set_error,
        [Value::from_json(serde_json::json!(["nan", "infinite"])).unwrap()]
    );
}

#[test]
fn nonassignable_getpath_reads_are_catchable_runtime_errors() {
    let values = evaluate_with_limits(
        r#"[try ((getpath([-5])) = 9) catch "negative", try ((getpath([nan])) = 9) catch "nan"]"#,
        "[1]",
        VmLimits::default(),
    )
    .expect("nonassignable getpath updates remain catchable");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!(["negative", "nan"])).unwrap()]
    );
}

#[test]
fn getpath_nonfinite_index_checks_target_type_before_returning_null() {
    let values = evaluate_with_limits(
        r#"def probe: try getpath([nan]) catch "caught"; [(null|probe),([1]|probe),({"a":1}|probe),("x"|probe),(true|probe),(1|probe)]"#,
        "null",
        VmLimits::default(),
    )
    .expect("nonfinite getpath target checks are catchable");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!([
            null, null, "caught", "caught", "caught", "caught"
        ]))
        .unwrap()]
    );
}
