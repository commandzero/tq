//! Public VM coverage for composed SQL-style relational built-ins.

use tq_core::{ResolveOptions, Value, Vm, VmError, VmLimits, analyze, parse, resolve};

fn evaluate(query: &str, input: &str) -> Vec<Value> {
    let input: Value = serde_json::from_str(input).expect("valid JSON input");
    let plan = analyze(
        resolve(
            parse(query).expect("query parses"),
            &ResolveOptions::default(),
        )
        .expect("query resolves"),
    )
    .compile()
    .expect("query compiles")
    .document_plan();
    let mut vm = Vm::new(&plan, input, VmLimits::default());
    let mut values = Vec::new();
    while let Some(value) = vm.next_result().expect("query evaluates") {
        values.push(value);
    }
    values
}

fn evaluate_until_error(query: &str, input: &str) -> (Vec<Value>, VmError) {
    let input: Value = serde_json::from_str(input).expect("valid JSON input");
    let plan = analyze(
        resolve(
            parse(query).expect("query parses"),
            &ResolveOptions::default(),
        )
        .expect("query resolves"),
    )
    .compile()
    .expect("query compiles")
    .document_plan();
    let mut vm = Vm::new(&plan, input, VmLimits::default());
    let mut values = Vec::new();
    loop {
        match vm.next_result() {
            Ok(Some(value)) => values.push(value),
            Ok(None) => panic!("query completed without the expected error"),
            Err(error) => return (values, error),
        }
    }
}

fn strings(values: Vec<Value>) -> Vec<String> {
    values.into_iter().map(|value| value.to_string()).collect()
}

#[test]
fn in_one_and_two_argument_forms_cross_called_filter_and_value_boundaries() {
    assert_eq!(
        strings(evaluate(
            r#"def contains(f): IN(f); contains(["a","b"][])"#,
            r#""b""#,
        )),
        ["true"],
    );
    assert_eq!(
        strings(evaluate(
            r#"def contains(f; g): IN(f; g); contains(.[]; ["b"][])"#,
            r#"["a","b"]"#,
        )),
        ["true"],
    );
    assert_eq!(
        strings(evaluate(
            r#"def contains($needle): IN(.[]; $needle); contains("b")"#,
            r#"["a","b"]"#,
        )),
        ["true"],
    );
}

#[test]
fn in_two_stream_form_emits_one_boolean_and_stops_after_any_match() {
    for (arguments, expected) in [
        ("(1,2); (2,3)", "true"),
        ("empty; (2,3)", "false"),
        ("(1,2); empty", "false"),
        ("(1,2); (3,4)", "false"),
        ("(1,2); (3,2,error(\"late\"))", "true"),
    ] {
        for query in [
            format!("IN({arguments})"),
            format!("def contains(left; right): IN(left; right); contains({arguments})"),
        ] {
            assert_eq!(strings(evaluate(&query, "0")), [expected], "{query}");
        }
    }
}

#[test]
fn index_forms_preserve_duplicate_empty_and_multi_result_key_callbacks() {
    assert_eq!(
        strings(evaluate(
            r"def make(f): INDEX(f); make(.id)",
            r#"[{"id":"a","v":1},{"id":"b","v":2}]"#,
        )),
        [r#"{"a":{"id":"a","v":1},"b":{"id":"b","v":2}}"#],
    );
    assert_eq!(
        strings(evaluate(
            r"def make(f; g): INDEX(f; g); make(.[]; .id)",
            r#"[{"id":"a","v":1},{"id":"b","v":2}]"#,
        )),
        [r#"{"a":{"id":"a","v":1},"b":{"id":"b","v":2}}"#],
    );
    assert_eq!(
        strings(evaluate(
            r"def make(f; g): INDEX(f; g); make(.[]; .id)",
            r#"[{"id":"a","v":1},{"id":"a","v":2},{"id":"b","v":3}]"#,
        )),
        [r#"{"a":{"id":"a","v":2},"b":{"id":"b","v":3}}"#],
    );
    assert_eq!(
        strings(evaluate(
            r"def make(f; g): INDEX(f; g); make(empty; .id)",
            r#"[{"id":"a","v":1}]"#,
        )),
        ["{}"],
    );
    assert_eq!(
        strings(evaluate(
            r#"def make(f; g): INDEX(f; g); make(.[]; (.id, (.id+"!")))"#,
            r#"[{"id":"a","v":1}]"#,
        )),
        [r#"{"a":{"id":"a","v":1},"a!":{"id":"a","v":1}}"#],
    );
    assert_eq!(
        strings(evaluate(
            r#"def make($key): INDEX(.[]; $key); make("id")"#,
            r#"[{"id":"a","v":1},{"id":"b","v":2}]"#,
        )),
        [r#"{"id":{"id":"b","v":2}}"#],
    );
}

#[test]
fn join_two_three_and_four_argument_forms_cross_called_boundaries() {
    assert_eq!(
        strings(evaluate(
            r#"def j($idx; f): JOIN($idx; f); j({"a":10}; .id)"#,
            r#"[{"id":"a"},{"id":"b"}]"#,
        )),
        [r#"[[{"id":"a"},10],[{"id":"b"},null]]"#],
    );
    assert_eq!(
        strings(evaluate(
            r#"def j($idx; f; g): JOIN($idx; f; g); j({"a":10}; .[]; .id)"#,
            r#"[{"id":"a"},{"id":"b"}]"#,
        )),
        [r#"[{"id":"a"},10]"#, r#"[{"id":"b"},null]"#],
    );
    assert_eq!(
        strings(evaluate(
            r#"def j($idx; f; g; h): JOIN($idx; f; g; h); j({"a":10}; .[]; .id; {row:.[0],match:.[1]})"#,
            r#"[{"id":"a"},{"id":"b"}]"#,
        )),
        [
            r#"{"row":{"id":"a"},"match":10}"#,
            r#"{"row":{"id":"b"},"match":null}"#,
        ],
    );
    assert_eq!(
        strings(evaluate(
            r#"def j($idx; f; g; h): JOIN($idx; f; g; h); j({"a":1}; ("a", "b"); .; (.[0], .[1]))"#,
            "null",
        )),
        ["\"a\"", "1", "\"b\"", "null"],
    );
}

#[test]
fn caught_buffered_join_lookup_error_does_not_emit_partial_collection() {
    assert_eq!(
        strings(evaluate(
            r#"def j(idx; key): JOIN(idx; key); [try (["a","b"] | j({"a":1}; if . == "a" then . else 0 end)) catch "caught"]"#,
            "null",
        )),
        [r#"["caught"]"#],
    );
}

#[test]
fn relational_composition_preserves_late_errors_after_partial_results() {
    let (values, error) = evaluate_until_error(
        r#"def j($idx; f; g): JOIN($idx; f; g); j({"a":1}; ("a", error("later")); .)"#,
        "null",
    );
    assert_eq!(strings(values), [r#"["a",1]"#]);
    assert!(error.to_string().contains("later"));

    let (values, error) = evaluate_until_error(
        r#"def contains(f): IN(f); contains((2, error("later")))"#,
        "1",
    );
    assert_eq!(values, [] as [tq_core::Value; 0]);
    assert!(error.to_string().contains("later"));

    let (values, error) = evaluate_until_error(
        r#"def make(f; g): INDEX(f; g); make(.[]; error("later"))"#,
        "[1]",
    );
    assert_eq!(values, [] as [tq_core::Value; 0]);
    assert!(error.to_string().contains("later"));
}

#[test]
fn in_callback_effects_follow_relational_search_order() {
    let plan = analyze(
        resolve(
            parse(r#"0 | IN((debug("left") | (1,2)); (debug("right") | 2))"#)
                .expect("IN effect-order query parses"),
            &ResolveOptions::default(),
        )
        .expect("IN effect-order query resolves"),
    )
    .compile()
    .expect("IN effect-order query compiles")
    .document_plan();
    let mut vm = Vm::new(&plan, Value::Null, VmLimits::default());
    let sink = vm.effect_sink();
    assert_eq!(
        vm.next_result().expect("IN effect-order query evaluates"),
        Some(Value::Bool(true)),
    );
    assert_eq!(
        vm.next_result().expect("IN effect-order query completes"),
        None
    );
    assert_eq!(
        sink.drain(),
        br#"["DEBUG:","right"]
["DEBUG:","left"]
"#
    );

    let plan = analyze(
        resolve(
            parse(r#"0 | IN((debug("left") | (1,2)); (debug("right") | empty))"#)
                .expect("IN empty effect-order query parses"),
            &ResolveOptions::default(),
        )
        .expect("IN empty effect-order query resolves"),
    )
    .compile()
    .expect("IN empty effect-order query compiles")
    .document_plan();
    let mut vm = Vm::new(&plan, Value::Null, VmLimits::default());
    let sink = vm.effect_sink();
    assert_eq!(
        vm.next_result()
            .expect("IN empty effect-order query evaluates"),
        Some(Value::Bool(false)),
    );
    assert_eq!(
        vm.next_result()
            .expect("IN empty effect-order query completes"),
        None
    );
    assert_eq!(
        sink.drain(),
        br#"["DEBUG:","right"]
"#
    );
}

#[test]
fn join_four_argument_callbacks_preserve_index_row_key_join_effect_order() {
    let plan = analyze(
        resolve(
            parse(
                r#"def j($idx; f; g; h): JOIN($idx; f; g; h); j((debug("index") | {"a":10}); (debug("row") | .[]); debug("key") | .id; debug("join") | {row:.[0],match:.[1]})"#,
            )
            .expect("JOIN/4 effect-order query parses"),
            &ResolveOptions::default(),
        )
        .expect("JOIN/4 effect-order query resolves"),
    )
    .compile()
    .expect("JOIN/4 effect-order query compiles")
    .document_plan();
    let mut vm = Vm::new(
        &plan,
        Value::from_json(serde_json::json!([{"id":"a"},{"id":"b"}])).unwrap(),
        VmLimits::default(),
    );
    let sink = vm.effect_sink();
    assert_eq!(
        vm.next_result().expect("JOIN/4 first result evaluates"),
        Some(
            Value::from_json(serde_json::json!({
                "row": {"id": "a"},
                "match": 10,
            }))
            .unwrap()
        ),
    );
    assert_eq!(
        vm.next_result().expect("JOIN/4 second result evaluates"),
        Some(
            Value::from_json(serde_json::json!({
                "row": {"id": "b"},
                "match": null,
            }))
            .unwrap()
        ),
    );
    assert_eq!(vm.next_result().expect("JOIN/4 completes"), None);
    assert_eq!(
        sink.drain(),
        br#"["DEBUG:","index"]
["DEBUG:","row"]
["DEBUG:","key"]
["DEBUG:","join"]
["DEBUG:","key"]
["DEBUG:","join"]
"#
    );
}
