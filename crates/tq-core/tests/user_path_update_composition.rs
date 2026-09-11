//! Public behavior tests for managed path selection and assignments.

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

#[test]
fn user_update_assignments_preserve_selected_paths_and_original_input_bounds() {
    assert_eq!(
        evaluate("def f: .a |= . + 1; f", r#"{"a":1,"b":2}"#).expect("field update evaluates"),
        [Value::from_json(serde_json::json!({"a": 2, "b": 2})).unwrap()]
    );
    assert_eq!(
        evaluate("def f: .[] |= . + 10; f", "[1,2,3]").expect("array update evaluates"),
        [Value::from_json(serde_json::json!([11, 12, 13])).unwrap()]
    );
    assert_eq!(
        evaluate("def f: (.[] | select(. > 1)) |= . + 10; f", "[1,2,3]",)
            .expect("selected update evaluates"),
        [Value::from_json(serde_json::json!([1, 12, 13])).unwrap()]
    );
    assert_eq!(
        evaluate("def f: .a[.b] |= . + 1; f", r#"{"a":[10,20],"b":1}"#,)
            .expect("dynamic index update evaluates"),
        [Value::from_json(serde_json::json!({"a": [10, 21], "b": 1})).unwrap()]
    );
}

#[test]
fn user_updates_defer_array_deletions_and_reuse_prior_replacements() {
    for (body, input, expected) in [
        (".[] |= empty", "[1,2,3]", "[]"),
        ("(.[] | select(. > 1)) |= empty", "[1,2,3]", "[1]"),
        (
            ".[] |= if . == 2 then . + 10 else empty end",
            "[1,2,3]",
            "[12]",
        ),
        ("(.a,.a) |= . + 1", r#"{"a":1}"#, r#"{"a":3}"#),
        ("(.[1:3],.[2:4]) |= empty", "[0,1,2,3,4]", "[0,4]"),
        (".a |= (. as $p | ($p = 9))", r#"{"a":1}"#, r#"{"a":9}"#),
        (". as $p | . |= ($p.a = 9)", r#"{"a":1}"#, r#"{"a":9}"#),
    ] {
        for query in [body.to_owned(), format!("def update: {body}; update")] {
            let expected: Value = serde_json::from_str(expected).unwrap();
            assert_eq!(evaluate(&query, input).unwrap(), [expected], "{query}");
        }
    }
}

#[test]
fn generator_binding_preserves_the_outer_assignment_path() {
    assert_eq!(
        evaluate(
            r#". as $approvals
            | {cases: [{id: "x"}]} as $report
            | $approvals
            | map(
                . as $approval
                | ($report.cases[] | select(.id == $approval.case_id)) as $case
                | .evidence = {x: 1}
              )"#,
            r#"[{"case_id":"x"}]"#,
        )
        .expect("generator-bound metadata must not discard the approval path"),
        [Value::from_json(serde_json::json!([{"case_id": "x", "evidence": {"x": 1}}])).unwrap()]
    );
}

#[test]
fn generator_binding_handles_matching_nonmatching_and_multiple_cases() {
    let query = r#". as $approvals
        | {cases: [{id: "x", value: 1}, {id: "x", value: 3}]} as $report
        | $approvals
        | map(
            . as $approval
            | ($report.cases[] | select(.id == $approval.case_id)) as $case
            | .evidence = ($case.value | . as $same | $same)
          )"#;
    assert_eq!(
        evaluate(query, r#"[{"case_id":"x"},{"case_id":"missing"}]"#)
            .expect("matching cases update their original approvals"),
        [Value::from_json(serde_json::json!([
            {"case_id": "x", "evidence": 1},
            {"case_id": "x", "evidence": 3}
        ]))
        .unwrap()]
    );
}

#[test]
fn generator_binding_allows_foreign_slice_and_filter_parameter_sources() {
    let query = r#". as $approvals
        | {cases: [{id: "x", value: 1}]} as $report
        | $approvals
        | map(
            . as $approval
            | ($report.cases[0:1] | .[] | select(.id == $approval.case_id)) as $case
            | .evidence = $case.value
          )"#;
    assert_eq!(
        evaluate(query, r#"[{"case_id":"x"}]"#)
            .expect("foreign slice sources must remain ordinary values"),
        [Value::from_json(serde_json::json!([{"case_id": "x", "evidence": 1}])).unwrap()]
    );

    let query = r#". as $approvals
        | {cases: [{id: "x", value: 1}]} as $report
        | def find(predicate):
            $report.cases[0:1][] | select(predicate)
          ;
        $approvals
        | map(
            . as $approval
            | find(.id == $approval.case_id) as $case
            | .evidence = $case.value
          )"#;
    assert_eq!(
        evaluate(query, r#"[{"case_id":"x"}]"#)
            .expect("filter-parameter sources must retain value-binding context"),
        [Value::from_json(serde_json::json!([{"case_id": "x", "evidence": 1}])).unwrap()]
    );
}

#[test]
fn generator_binding_keeps_ordinary_value_bodies_out_of_path_mode() {
    assert_eq!(
        evaluate(
            r#". as $approvals
            | {cases: [{id: "x"}]} as $report
            | $approvals
            | map(($report.cases[]) as $case | .case_id)"#,
            r#"[{"case_id":"x"}]"#,
        )
        .expect("ordinary value binding must not force its body into path mode"),
        [Value::from_json(serde_json::json!(["x"])).unwrap()]
    );
}

#[test]
fn generator_bound_foreign_root_remains_invalid_as_assignment_target() {
    let error = evaluate(
        r#". as $root
            | {cases: [{id: "x"}]} as $report
            | ($report.cases[0].id = 2)"#,
        r#"{"root": 1}"#,
    )
    .expect_err("a foreign constructed root cannot become an assignment target");
    assert!(
        matches!(&error, VmError::Runtime { message } if message.contains("assignment left side")),
        "unexpected error: {error}"
    );
}

#[test]
fn generator_bound_path_context_does_not_swallow_nested_path_errors() {
    for query in [
        r". as $root
            | {foreign: {x: 1}} as $outer
            | (. as $value | ($outer.foreign.x = 2)) as $result
            | $root",
        r". as $root
            | {foreign: {x: 1}} as $outer
            | path($outer.foreign.x) as $result
            | $root",
    ] {
        let error = evaluate(query, r#"{"root": 1}"#)
            .expect_err("nested foreign-root path errors must remain visible");
        assert!(
            matches!(&error, VmError::Runtime { message } if message.contains("assignment left side")),
            "unexpected error: {error}"
        );
    }
}

#[test]
fn user_plain_assignment_keeps_rhs_branches_and_missing_paths() {
    assert_eq!(
        evaluate("def f: (.a,.b) = (0,1,2); f", "null").expect("plain assignment evaluates"),
        [
            Value::from_json(serde_json::json!({"a": 0, "b": 0})).unwrap(),
            Value::from_json(serde_json::json!({"a": 1, "b": 1})).unwrap(),
            Value::from_json(serde_json::json!({"a": 2, "b": 2})).unwrap(),
        ]
    );
    assert_eq!(
        evaluate("def f: .missing = 1; f", "{}").expect("missing path creates a field"),
        [Value::from_json(serde_json::json!({"missing": 1})).unwrap()]
    );
    assert_eq!(
        evaluate("def f: .a |= first((0,1,2)); f", r#"{"a":3}"#).expect("first update evaluates"),
        [Value::from_json(serde_json::json!({"a": 0})).unwrap()]
    );
    assert_eq!(
        evaluate("def f: .a |= empty; f", r#"{"a":3,"b":2}"#).expect("empty update evaluates"),
        [Value::from_json(serde_json::json!({"b": 2})).unwrap()]
    );
}

#[test]
fn user_update_supports_negative_indices_and_preserves_first_result() {
    assert_eq!(
        evaluate("def f: .[-1] |= (0,1,2); f", "[0,1,2]").expect("negative update evaluates"),
        [Value::from_json(serde_json::json!([0, 1, 0])).unwrap()]
    );
}

#[test]
fn user_filter_calls_can_be_paths_on_the_left_hand_side() {
    assert_eq!(
        evaluate("def p: .a; p |= . + 1", r#"{"a":1}"#)
            .expect("path-returning user filter updates"),
        [Value::from_json(serde_json::json!({"a": 2})).unwrap()]
    );
    assert_eq!(
        evaluate("def apply(p): p |= . + 1; apply(.a)", r#"{"a":1}"#)
            .expect("parameterized path filter updates"),
        [Value::from_json(serde_json::json!({"a": 2})).unwrap()]
    );
    assert_eq!(
        evaluate("def p($key): .[$key]; p(\"a\") |= . + 1", r#"{"a":1}"#)
            .expect("value-parameter path filter updates"),
        [Value::from_json(serde_json::json!({"a": 2})).unwrap()]
    );
    assert_eq!(
        evaluate(
            "def p: if .a == 1 then .a else .b end; p |= . + 1",
            r#"{"a":1,"b":2}"#,
        )
        .expect("conditional path filter updates"),
        [Value::from_json(serde_json::json!({"a": 2, "b": 2})).unwrap()]
    );
    assert_eq!(
        evaluate(
            "def f: ((if .a then . else 7 end) as $p | $p.x) = 2; f",
            r#"{"a":true,"x":1}"#,
        )
        .expect("conditional root path provenance"),
        [Value::from_json(serde_json::json!({"a": true, "x": 2})).unwrap()]
    );
    assert_eq!(
        evaluate(
            "def f: ((if .a then . else 7 end) as $p | .x) = 2; f",
            r#"{"a":false,"x":1}"#,
        )
        .expect("ordinary conditional bind branch is unused by the path body"),
        [Value::from_json(serde_json::json!({"a": false, "x": 2})).unwrap()]
    );
    assert_eq!(
        evaluate("def f: ((.,7) as $p | .x) = 2; f", r#"{"x":1}"#,)
            .expect("ordinary comma bind branch is unused by the path body"),
        [Value::from_json(serde_json::json!({"x": 2})).unwrap()]
    );
    assert_eq!(
        evaluate("def p($v): $v.x; p(.) = 2", r#"{"x":1}"#)
            .expect("root provenance through value parameter"),
        [Value::from_json(serde_json::json!({"x": 2})).unwrap()]
    );
    assert_eq!(
        evaluate(
            "def pass(f): f; ((pass(.) | .) as $p | $p.x) = 2",
            r#"{"x":1}"#,
        )
        .expect("filter parameter preserves path provenance through a pipe"),
        [Value::from_json(serde_json::json!({"x": 2})).unwrap()]
    );
    assert_eq!(
        evaluate("def setx($p): $p.x = 2; . as $p | setx($p)", r#"{"x":1}"#,)
            .expect("root alias survives a value-parameter user call"),
        [Value::from_json(serde_json::json!({"x": 2})).unwrap()]
    );
    assert_eq!(
        evaluate(". as $p | ($p as $q | $q.x = 2)", r#"{"x":1}"#,)
            .expect("nested root aliases preserve path identity"),
        [Value::from_json(serde_json::json!({"x": 2})).unwrap()]
    );
    assert_eq!(
        evaluate("((try . catch .) as $p | $p.x) = 2", r#"{"x":1}"#,)
            .expect("try preserves a successful root path"),
        [Value::from_json(serde_json::json!({"x": 2})).unwrap()]
    );
    assert_eq!(
        evaluate("((. as $p ?// $q | .x) = 2)", r#"{"x":1}"#,)
            .expect("destructuring alternative keeps the path body"),
        [Value::from_json(serde_json::json!({"x": 2})).unwrap()]
    );
    assert!(evaluate(
        "try ((.,7) as $p | $p.x) = 2 catch .",
        r#"{"x":1}"#,
    )
    .expect("mixed path/value branch reports a caught selector error")
    .iter()
    .any(|value| matches!(value, Value::String(message) if message.contains("assignment left side"))));
}

#[test]
fn user_slice_paths_update_the_selected_array_range() {
    assert_eq!(
        evaluate(".[1:2] = [8,9]", "[0,1,2]").expect("direct slice assignment"),
        [Value::from_json(serde_json::json!([0, 8, 9, 2])).unwrap()]
    );
    assert_eq!(
        evaluate("def f: .[1:2] = [8,9]; f", "[0,1,2]").expect("user slice assignment"),
        [Value::from_json(serde_json::json!([0, 8, 9, 2])).unwrap()]
    );
    assert_eq!(
        evaluate(".[1:2] |= . + [7]", "[0,1,2]").expect("slice update assignment"),
        [Value::from_json(serde_json::json!([0, 1, 7, 2])).unwrap()]
    );
    assert_eq!(
        evaluate(".[1:1] = [8]", "[0,1,2]").expect("empty slice insertion"),
        [Value::from_json(serde_json::json!([0, 8, 1, 2])).unwrap()]
    );
    assert_eq!(
        evaluate(
            "def f: .a[.b:.c] = [8]; f",
            r#"{"a":[10,20,30],"b":1,"c":2}"#,
        )
        .expect("slice bounds use the original input"),
        [Value::from_json(serde_json::json!({
            "a": [10, 8, 30],
            "b": 1,
            "c": 2
        }))
        .unwrap()]
    );
    assert_eq!(
        evaluate("def f: (.[] as $x | .[0]) = 9; f", "[1,2]")
            .expect("value-producing bind on the path side"),
        [Value::from_json(serde_json::json!([9, 2])).unwrap()]
    );
    assert_eq!(
        evaluate("def f: ((.[] | . + 1) as $x | .[0]) = 9; f", "[1,2]")
            .expect("ordinary filter bind on the path side"),
        [Value::from_json(serde_json::json!([9, 2])).unwrap()]
    );
    assert_eq!(
        evaluate("(. as $p | $p.a) = 2", r#"{"a":1}"#).expect("root alias can extend a path"),
        [Value::from_json(serde_json::json!({"a": 2})).unwrap()]
    );
    let literal_array_error = evaluate("try ([\"a\"] as $p | $p = 2) catch .", "null")
        .expect("literal array alias rejection");
    assert!(matches!(
        literal_array_error.as_slice(),
        [Value::String(message)] if message.contains("assignment left side is not a path")
    ));
    let shadowed_alias_error = evaluate(
        "try ((. as $p | 7 as $p | $p.a) = 3) catch .",
        r#"{"a":1,"b":2}"#,
    )
    .expect("shadowed alias rejection");
    assert!(matches!(
        shadowed_alias_error.as_slice(),
        [Value::String(message)] if message.contains("assignment left side is not a path")
    ));
    let non_root_alias_error = evaluate("try ((.a as $p | $p.b) = 2) catch .", r#"{"a":{"b":1}}"#)
        .expect("non-root alias rejection");
    assert!(matches!(
        non_root_alias_error.as_slice(),
        [Value::String(message)] if message.contains("assignment left side is not a path")
    ));
}
