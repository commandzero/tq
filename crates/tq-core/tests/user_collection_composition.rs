//! Public VM regressions for callback-driven collection built-ins.
//!
//! Expected values were checked against the pinned jq 1.8.1 reference
//! executable with bounded two-second oracle probes.  These tests deliberately
//! use named definitions plus captured value/filter parameters so the managed
//! callback path is exercised once the collection slice is admitted.

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

fn default_limits() -> VmLimits {
    VmLimits {
        steps: 512,
        ..VmLimits::default()
    }
}

#[test]
fn group_by_captured_filters_preserve_zero_multiple_and_stable_ties() {
    let input = r#"[{"k":1,"id":"first"},{"k":1,"id":"last"}]"#;
    let zero_result = evaluate_with_limits(
        "def grouped(f): group_by(f); grouped(empty)",
        input,
        default_limits(),
    )
    .expect("group_by zero-result key filter evaluates");
    assert_eq!(
        zero_result,
        [Value::from_json(serde_json::json!([
            [{"k": 1, "id": "first"}, {"k": 1, "id": "last"}]
        ]))
        .unwrap()]
    );

    let multiple_result = evaluate_with_limits(
        "def grouped(f): group_by(f); grouped(.k, (.k + 1))",
        input,
        default_limits(),
    )
    .expect("group_by multiple-result key filter evaluates");
    assert_eq!(multiple_result, zero_result);

    let ordered = evaluate_with_limits(
        "def grouped(f): group_by(f); grouped(.k)",
        r#"[{"k":2,"id":"first"},{"k":1,"id":"middle"},{"k":2,"id":"last"}]"#,
        default_limits(),
    )
    .expect("groups sort by key while preserving ties");
    assert_eq!(
        ordered,
        [Value::from_json(serde_json::json!([
            [{"k":1,"id":"middle"}],
            [{"k":2,"id":"first"},{"k":2,"id":"last"}]
        ]))
        .unwrap()]
    );
}

#[test]
fn keyed_extrema_preserve_ties_and_report_late_key_errors() {
    let distinct = evaluate_with_limits(
        "def extrema(f): [min_by(f), max_by(f)]; extrema(.k)",
        r#"[{"k":3},{"k":1},{"k":2}]"#,
        default_limits(),
    )
    .expect("extrema select opposite ends of unequal keys");
    assert_eq!(
        distinct,
        [Value::from_json(serde_json::json!([{"k":1},{"k":3}])).unwrap()]
    );
    let tied = evaluate_with_limits(
        "def extrema(f): [min_by(f), max_by(f)]; extrema(.k)",
        r#"[{"k":1,"id":"first"},{"k":1,"id":"last"}]"#,
        default_limits(),
    )
    .expect("min_by/max_by tie query evaluates");
    assert_eq!(
        tied,
        [Value::from_json(serde_json::json!([
            {"k": 1, "id": "first"},
            {"k": 1, "id": "last"}
        ]))
        .unwrap()]
    );

    for operation in ["min_by", "max_by"] {
        let query = format!(
            "def extrema(f): {operation}(f); extrema(if .id == \"last\" then error(\"late\") else .k end)"
        );
        let late = evaluate_with_limits(
            &query,
            r#"[{"k":1,"id":"first"},{"k":1,"id":"last"}]"#,
            default_limits(),
        )
        .expect_err("a later key error must not be suppressed");
        assert!(late.to_string().contains("late"));
    }
}

#[test]
fn any_all_short_circuit_before_later_errors_and_handle_empty_filters() {
    let any_value = evaluate_with_limits(
        "def first_true(g; f): any(g; f); first_true((1, error(\"late\")); . == 1)",
        "null",
        default_limits(),
    )
    .expect("any short-circuits after a decisive true result");
    assert_eq!(any_value, [Value::Bool(true)]);

    let all_value = evaluate_with_limits(
        "def first_false(g; f): all(g; f); first_false((0, error(\"late\")); . == 1)",
        "null",
        default_limits(),
    )
    .expect("all short-circuits after a decisive false result");
    assert_eq!(all_value, [Value::Bool(false)]);

    let empty = evaluate_with_limits(
        "def predicates(g; f): [any(g; f), all(g; f)]; predicates(empty; .)",
        "null",
        default_limits(),
    )
    .expect("empty predicate generator evaluates");
    assert_eq!(
        empty,
        [Value::from_json(serde_json::json!([false, true])).unwrap()]
    );
}

#[test]
fn any_all_captured_generators_preserve_multi_result_predicates() {
    let values = evaluate_with_limits(
        "def predicates(g; f): [any(g; f), all(g; f)]; predicates((0, 1); (. == 1, . == 0))",
        "null",
        default_limits(),
    )
    .expect("any/all multi-result generator evaluates");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!([true, false])).unwrap()]
    );
}

#[test]
fn any_all_zero_and_one_argument_forms_match_jq() {
    let any_zero = evaluate_with_limits("def f: any; f", "[false, true]", default_limits())
        .expect("any/0 evaluates");
    assert_eq!(any_zero, [Value::Bool(true)]);

    let all_zero = evaluate_with_limits("def f: all; f", "[true, true]", default_limits())
        .expect("all/0 evaluates");
    assert_eq!(all_zero, [Value::Bool(true)]);

    let any_one =
        evaluate_with_limits("def f(g): any(g); f(. == 2)", "[1, 2, 3]", default_limits())
            .expect("any/1 evaluates");
    assert_eq!(any_one, [Value::Bool(true)]);

    let all_one = evaluate_with_limits("def f(g): all(g); f(. < 4)", "[1, 2, 3]", default_limits())
        .expect("all/1 evaluates");
    assert_eq!(all_one, [Value::Bool(true)]);
}

#[test]
fn collection_callbacks_honor_step_limits_and_pre_cancellation() {
    let query = "def every(f): all((true, true, true); f); every(.)";
    let input = "null";
    let error = evaluate_with_limits(
        query,
        input,
        VmLimits {
            steps: 2,
            ..VmLimits::default()
        },
    )
    .expect_err("predicate callback must observe a tight VM step limit");
    assert!(matches!(
        error,
        VmError::Resource {
            resource: "vm-steps"
        }
    ));

    let resolved = resolve(
        parse(query).expect("query parses"),
        &ResolveOptions::default(),
    )
    .expect("query resolves");
    let plan = analyze(resolved)
        .compile()
        .expect("query compiles")
        .document_plan();
    let cancellation = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let mut vm = Vm::new(&plan, Value::Null, VmLimits::default())
        .with_cancellation(std::sync::Arc::clone(&cancellation));
    assert_eq!(vm.next_result(), Err(VmError::Interrupted));
}

#[test]
fn grouped_materialization_uses_one_cumulative_quota() {
    let query = "def grouped: group_by(.); grouped | empty";
    let input = "[0,0,0,1,1,1,2,2,2]";
    let error = evaluate_with_limits(
        query,
        input,
        VmLimits {
            output_bytes: 10,
            steps: 256,
            ..VmLimits::default()
        },
    )
    .expect_err("individually small groups must not reset materialization quota");
    assert!(matches!(
        error,
        VmError::Resource {
            resource: "output-bytes"
        }
    ));
    assert_eq!(
        evaluate_with_limits(
            query,
            input,
            VmLimits {
                output_bytes: 64,
                steps: 256,
                ..VmLimits::default()
            },
        )
        .expect("the same grouping fits a sufficient quota"),
        [] as [tq_core::Value; 0]
    );
}

#[test]
fn add_one_captured_generator_preserves_empty_multiple_and_errors() {
    let multiple = evaluate_with_limits(
        "def add_filter(g): add(g); add_filter((1, 2, 3))",
        "null",
        default_limits(),
    )
    .expect("add/1 multiple-result generator evaluates");
    assert_eq!(multiple, [Value::from_json(serde_json::json!(6)).unwrap()]);

    let empty = evaluate_with_limits(
        "def add_filter(g): add(g); add_filter(empty)",
        "null",
        default_limits(),
    )
    .expect("add/1 empty generator evaluates");
    assert_eq!(empty, [Value::Null]);

    let late = evaluate_with_limits(
        "def add_filter(g): add(g); add_filter((1, error(\"late\"), 2))",
        "null",
        default_limits(),
    )
    .expect_err("add/1 must report a later generator error");
    assert!(late.to_string().contains("late"));
}

#[test]
fn with_entries_captured_value_filter_preserves_empty_multiple_and_aliases() {
    let removed = evaluate_with_limits(
        "def rewrite($items; f): $items | with_entries(f); rewrite(.; if .key == \"a\" then empty else {Key: .key, Value: (.value + 1)} end)",
        r#"{"a":1,"b":2}"#,
        default_limits(),
    )
    .expect("with_entries empty callback result evaluates");
    assert_eq!(
        removed,
        [Value::from_json(serde_json::json!({"b": 3})).unwrap()]
    );

    let multiple = evaluate_with_limits(
        "def rewrite($items; f): $items | with_entries(f); rewrite(.; {Key: .key, Value: .value}, {name: (.key + \"2\"), value: .value})",
        r#"{"a":1}"#,
        default_limits(),
    )
    .expect("with_entries multiple callback results evaluate");
    assert_eq!(
        multiple,
        [Value::from_json(serde_json::json!({"a": 1, "a2": 1})).unwrap()]
    );
}
