//! Public VM coverage for scalar path built-ins and captured arguments.

use tq_core::{
    Document, Plan, ResolveOptions, Value, Vm, VmError, VmLimits, analyze, parse, resolve,
};

type DocumentPlan = Plan<tq_core::Compiled, Document>;

fn plan(query: &str) -> DocumentPlan {
    analyze(
        resolve(
            parse(query).expect("query parses"),
            &ResolveOptions::default(),
        )
        .expect("query resolves"),
    )
    .compile()
    .expect("query compiles")
    .document_plan()
}

fn evaluate_with_limits(query: &str, input: &str, limits: VmLimits) -> Result<Vec<Value>, VmError> {
    let input = Value::from_json(serde_json::from_str(input).expect("valid JSON input"))
        .expect("JSON converts to a tq value");
    let plan = plan(query);
    let mut vm = Vm::new(&plan, input, limits);
    let mut values = Vec::new();
    while let Some(value) = vm.next_result()? {
        values.push(value);
    }
    Ok(values)
}

#[test]
fn getpath_and_setpath_work_through_called_defs_and_captured_values() {
    let selected = evaluate_with_limits(
        "def get($path): getpath($path); get([\"a\",1])",
        r#"{"a":[10,20]}"#,
        VmLimits::default(),
    )
    .expect("getpath in a def evaluates");
    assert_eq!(selected, [Value::from_json(serde_json::json!(20)).unwrap()]);

    let missing = evaluate_with_limits(
        "def get: getpath([\"missing\",0]); get",
        "{}",
        VmLimits::default(),
    )
    .expect("missing getpath components produce null");
    assert_eq!(missing, [Value::Null]);

    let updated = evaluate_with_limits(
        "def set($path; f): setpath($path; f); set([\"a\"]; .a + 1)",
        r#"{"a":1}"#,
        VmLimits::default(),
    )
    .expect("captured setpath arguments evaluate");
    assert_eq!(
        updated,
        [Value::from_json(serde_json::json!({"a": 2})).unwrap()]
    );
}

#[test]
fn setpath_preserves_jq_value_major_cartesian_order() {
    let values = evaluate_with_limits(
        "def set: [setpath(( [\"a\"], [\"b\"] ); (2,3))]; set",
        "{}",
        VmLimits::default(),
    )
    .expect("multiple path and value results evaluate");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!([
            {"a": 2},
            {"b": 2},
            {"a": 3},
            {"b": 3}
        ]))
        .unwrap()]
    );
}

#[test]
fn delpaths_preserves_order_and_missing_path_behavior() {
    let values = evaluate_with_limits(
        "def remove($paths): delpaths($paths); remove([[\"a\",0],[\"a\",1],[\"missing\"]])",
        r#"{"a":[0,1,2]}"#,
        VmLimits::default(),
    )
    .expect("delpaths evaluates a captured path list");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!({"a": [2]})).unwrap()]
    );

    let missing = evaluate_with_limits(
        "def remove: delpaths([[\"missing\",0]]); remove",
        r#"{"a":1}"#,
        VmLimits::default(),
    )
    .expect("missing delpaths are no-ops");
    assert_eq!(
        missing,
        [Value::from_json(serde_json::json!({"a": 1})).unwrap()]
    );
}

#[test]
fn del_accepts_a_called_path_filter() {
    let values = evaluate_with_limits(
        "def remove: .a[] | select(. == 1); del(remove)",
        r#"{"a":[0,1,2],"b":3}"#,
        VmLimits::default(),
    )
    .expect("del evaluates a captured path filter");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!({"a": [0, 2], "b": 3})).unwrap()]
    );
}

#[test]
fn path_accepts_a_called_path_filter() {
    let values = evaluate_with_limits(
        "def choose: .a[] | select(. == 20); path(choose)",
        r#"{"a":[10,20,30]}"#,
        VmLimits::default(),
    )
    .expect("path evaluates a captured path filter");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!(["a", 1])).unwrap()]
    );
}

#[test]
fn paths_accepts_a_called_predicate_filter() {
    let values = evaluate_with_limits(
        "def choose: select(type == \"number\" and . > 0); [paths(choose)]",
        r#"{"a":[0,1],"b":2}"#,
        VmLimits::default(),
    )
    .expect("paths evaluates a captured predicate filter");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!([["a", 1], ["b"]])).unwrap()]
    );
}

#[test]
fn paths_excludes_root_for_called_and_true_filters() {
    let called = evaluate_with_limits("def p: paths; [p]", r#"{"a":1}"#, VmLimits::default())
        .expect("called paths emits descendants only");
    assert_eq!(
        called,
        [Value::from_json(serde_json::json!([["a"]])).unwrap()]
    );

    let explicit =
        evaluate_with_limits("def p: paths(true); [p]", r#"{"a":1}"#, VmLimits::default())
            .expect("paths(true) emits descendants only");
    assert_eq!(
        explicit,
        [Value::from_json(serde_json::json!([["a"]])).unwrap()]
    );

    let scalar = evaluate_with_limits("def p: paths; [p]", "1", VmLimits::default())
        .expect("paths on a scalar has no descendants");
    assert_eq!(scalar, [Value::from_json(serde_json::json!([])).unwrap()]);
}

#[test]
fn paths_depth_limit_allows_one_component_and_rejects_deeper() {
    let shallow = evaluate_with_limits(
        "def p: paths; [p]",
        r#"{"a":1}"#,
        VmLimits {
            path_stack: 1,
            ..VmLimits::default()
        },
    )
    .expect("one-component paths fit path_stack=1");
    assert_eq!(
        shallow,
        [Value::from_json(serde_json::json!([["a"]])).unwrap()]
    );

    let deep = evaluate_with_limits(
        "def p: paths; [p]",
        r#"{"a":{"b":1}}"#,
        VmLimits {
            path_stack: 1,
            ..VmLimits::default()
        },
    )
    .expect_err("deeper paths exceed path_stack=1");
    assert!(matches!(
        deep,
        VmError::Resource {
            resource: "path-stack"
        }
    ));
}

#[test]
fn constructed_path_results_are_not_assignment_aliases() {
    for body in ["path(.a)", "paths", "paths(true)", "pick(.a)", "del(.a)"] {
        let own_input = format!("def p: {body}; p | . as $p | ($p = 9)");
        let values = evaluate_with_limits(&own_input, r#"{"a":1}"#, VmLimits::default())
            .expect("a binding of the constructed current input remains assignable");
        assert_eq!(
            values,
            [Value::from_json(serde_json::json!(9)).unwrap()],
            "{body}"
        );

        let original_input = format!("def p: {body}; p as $p | try ($p = 9) catch \"caught\"");
        let values = evaluate_with_limits(&original_input, r#"{"a":1}"#, VmLimits::default())
            .expect("constructed values do not alias the original input");
        assert_eq!(
            values,
            [Value::from_json(serde_json::json!("caught")).unwrap()],
            "{body}",
        );
    }
}

#[test]
fn caught_late_path_collection_errors_do_not_emit_stale_results() {
    let picked = evaluate_with_limits(
        "def p(f): pick(f); [try p((.a, error(\"late\"))) catch \"caught\", 1]",
        r#"{"a":1}"#,
        VmLimits::default(),
    )
    .expect("pick callback errors remain catchable");
    assert_eq!(
        picked,
        [Value::from_json(serde_json::json!(["caught", 1])).unwrap()]
    );

    let deleted = evaluate_with_limits(
        "def p(f): del(f); [try p((.a, error(\"late\"))) catch \"caught\", 1]",
        r#"{"a":1}"#,
        VmLimits::default(),
    )
    .expect("del callback errors remain catchable");
    assert_eq!(
        deleted,
        [Value::from_json(serde_json::json!(["caught", 1])).unwrap()]
    );
}

#[test]
fn pick_accepts_a_called_path_filter() {
    let values = evaluate_with_limits(
        "def choose: .a[] | select(. == 20); pick(choose)",
        r#"{"a":[10,20,30]}"#,
        VmLimits::default(),
    )
    .expect("pick evaluates a captured path filter");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!({"a": [null, 20]})).unwrap()]
    );
}

#[test]
fn path_builtins_preserve_empty_and_late_error_cardinality() {
    let empty = evaluate_with_limits(
        "def e: [getpath(empty), setpath(empty; 1), delpaths(empty)]; e",
        "{}",
        VmLimits::default(),
    )
    .expect("empty path generators emit no calls");
    assert_eq!(empty, [Value::from_json(serde_json::json!([])).unwrap()]);

    let first = evaluate_with_limits(
        "def first_good: first(getpath(([\"a\"], error(\"late\")))); first_good",
        r#"{"a":1}"#,
        VmLimits::default(),
    )
    .expect("first stops before a late path error");
    assert_eq!(first, [Value::from_json(serde_json::json!(1)).unwrap()]);
}

#[test]
fn path_builtins_reject_invalid_paths_and_enforce_depth() {
    let type_error = evaluate_with_limits("def bad: getpath(1); bad", "{}", VmLimits::default())
        .expect_err("getpath requires an array path");
    assert!(matches!(type_error, VmError::Runtime { .. }));

    let type_error = evaluate_with_limits("def bad: delpaths([1]); bad", "{}", VmLimits::default())
        .expect_err("delpaths requires an array of array paths");
    assert!(matches!(type_error, VmError::Runtime { .. }));

    let scalar_error = evaluate_with_limits(
        "def bad: setpath([\"a\"]; 9); bad",
        "1",
        VmLimits::default(),
    )
    .expect_err("setpath must reject a scalar ancestor");
    assert!(matches!(scalar_error, VmError::Runtime { .. }));

    let error = evaluate_with_limits(
        "def get: getpath([\"a\",\"b\",\"c\"]); get",
        r#"{"a":{"b":{"c":1}}}"#,
        VmLimits {
            path_stack: 2,
            ..VmLimits::default()
        },
    )
    .expect_err("path depth must be charged before traversal");
    assert!(matches!(
        error,
        VmError::Resource {
            resource: "path-stack"
        }
    ));
}

#[test]
fn path_reconstruction_observes_tight_work_limits() {
    let error = evaluate_with_limits(
        "def set: setpath([\"a\",\"b\"]; 1); set",
        r#"{"a":{"b":0,"sibling":[0,1,2,3,4]}}"#,
        VmLimits {
            steps: 8,
            ..VmLimits::default()
        },
    )
    .expect_err("setpath copies ancestors under the VM work budget");
    assert!(matches!(
        error,
        VmError::Resource {
            resource: "vm-steps"
        }
    ));
}
