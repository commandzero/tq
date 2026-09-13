//! Public behavior tests for bounded composition of user-defined filters.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

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

fn evaluate_events(query: &str, input: &str) -> Vec<Result<Value, String>> {
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
    let mut events = Vec::new();
    loop {
        match vm.next_result() {
            Ok(Some(value)) => events.push(Ok(value)),
            Ok(None) => break,
            Err(error) => {
                events.push(Err(error.to_string()));
                break;
            }
        }
    }
    events
}

#[test]
fn user_filter_can_construct_an_object() {
    assert_eq!(
        evaluate("def f: {tool}; f", "null").expect("object construction evaluates"),
        [Value::from_json(serde_json::json!({"tool": null})).unwrap()]
    );
}

#[test]
fn user_filter_can_construct_a_slice() {
    assert_eq!(
        evaluate("def f: .[0:1]; f", "[1,2]").expect("slice evaluates"),
        [Value::from_json(serde_json::json!([1])).unwrap()]
    );
}

#[test]
fn object_fields_preserve_generator_cartesian_order() {
    assert_eq!(
        evaluate("def f: {a: (1,2), b: (3,4)}; f", "null").expect("object generators evaluate"),
        [
            Value::from_json(serde_json::json!({"a": 1, "b": 3})).unwrap(),
            Value::from_json(serde_json::json!({"a": 1, "b": 4})).unwrap(),
            Value::from_json(serde_json::json!({"a": 2, "b": 3})).unwrap(),
            Value::from_json(serde_json::json!({"a": 2, "b": 4})).unwrap(),
        ]
    );
}

#[test]
fn computed_keys_and_lexical_input_are_preserved() {
    let input = r#"{"key":"name","value":7}"#;
    let direct = evaluate("{(.key): .value}", input).expect("direct object evaluates");
    let composed =
        evaluate("def f: {(.key): .value}; f", input).expect("computed object key evaluates");
    assert_eq!(direct, composed,);
    assert_eq!(
        composed,
        [Value::from_json(serde_json::json!({"name": 7})).unwrap()]
    );
}

#[test]
fn object_constructors_preserve_value_parameters_and_nested_captures() {
    assert_eq!(
        evaluate("def f($key): {($key): .}; f(\"name\")", "null")
            .expect("value parameter evaluates"),
        [Value::from_json(serde_json::json!({"name": null})).unwrap()]
    );
    assert_eq!(
        evaluate(
            "def outer($value): def inner: {value: $value}; inner; outer(7)",
            "null"
        )
        .expect("nested capture evaluates"),
        [Value::from_json(serde_json::json!({"value": 7})).unwrap()]
    );
}

#[test]
fn empty_object_key_or_value_branches_do_not_emit_or_evaluate_later_fields() {
    assert_eq!(
        evaluate("def f: {(empty): error(\"unreached\")}; f", "null")
            .expect("empty key branch evaluates"),
        [] as [tq_core::Value; 0]
    );
    assert_eq!(
        evaluate("def f: {a: empty}; f", "null").expect("empty value branch evaluates"),
        [] as [tq_core::Value; 0]
    );
}

#[test]
fn object_partial_output_precedes_a_later_error() {
    let events = evaluate_events("def f: {a: (1, error(\"late\")), b: 2}; f", "null");
    assert_eq!(events.len(), 2);
    assert_eq!(
        events[0],
        Ok(Value::from_json(serde_json::json!({"a": 1, "b": 2})).unwrap())
    );
    assert!(
        events[1]
            .as_ref()
            .is_err_and(|error| error.contains("late"))
    );
}

#[test]
fn first_stops_object_work_before_late_error_branches() {
    assert_eq!(
        evaluate("first({a: (1, error(\"late\")), b: 2})", "null").expect("first object evaluates"),
        [Value::from_json(serde_json::json!({"a": 1, "b": 2})).unwrap()]
    );
}

#[test]
fn slice_bounds_use_original_input_and_preserve_bound_generators() {
    assert_eq!(
        evaluate("def f: .a[.b:.c]; f", r#"{"a":[10,20,30],"b":1,"c":2}"#)
            .expect("captured slice evaluates"),
        [Value::from_json(serde_json::json!([20])).unwrap()]
    );
    assert_eq!(
        evaluate("def f: .[(0,1):(2,3)]; f", "[0,1,2]").expect("slice generators evaluate"),
        [
            Value::from_json(serde_json::json!([0, 1])).unwrap(),
            Value::from_json(serde_json::json!([0, 1, 2])).unwrap(),
            Value::from_json(serde_json::json!([1])).unwrap(),
            Value::from_json(serde_json::json!([1, 2])).unwrap(),
        ]
    );
}

#[test]
fn first_stops_slice_bound_work_before_a_late_error() {
    assert_eq!(
        evaluate("first(.[(0,error(\"late\")):2])", "[0,1,2]").expect("first slice evaluates"),
        [Value::from_json(serde_json::json!([0, 1])).unwrap()]
    );
}

#[test]
fn object_composition_honors_the_vm_step_limit() {
    let resolved = resolve(
        parse("def f: {a: (1,2), b: (3,4)}; f").expect("query parses"),
        &ResolveOptions::default(),
    )
    .expect("query resolves");
    let plan = analyze(resolved)
        .compile()
        .expect("query compiles")
        .document_plan();
    let mut vm = Vm::new(
        &plan,
        Value::Null,
        VmLimits {
            steps: 4,
            ..VmLimits::default()
        },
    );
    assert!(matches!(
        vm.next_result(),
        Err(VmError::Resource {
            resource: "vm-steps"
        })
    ));
}

#[test]
fn object_composition_honors_cancellation_between_generator_results() {
    let resolved = resolve(
        parse("def f: {a: (1,2), b: (3,4)}; f").expect("query parses"),
        &ResolveOptions::default(),
    )
    .expect("query resolves");
    let plan = analyze(resolved)
        .compile()
        .expect("query compiles")
        .document_plan();
    let cancellation = Arc::new(AtomicBool::new(false));
    let mut vm = Vm::new(&plan, Value::Null, VmLimits::default())
        .with_cancellation(Arc::clone(&cancellation));
    assert_eq!(
        vm.next_result().expect("first object result"),
        Some(Value::from_json(serde_json::json!({"a": 1, "b": 3})).unwrap())
    );
    cancellation.store(true, Ordering::Relaxed);
    assert_eq!(vm.next_result(), Err(VmError::Interrupted));
}

#[test]
fn slices_follow_jq_fractional_null_and_nonfinite_bounds() {
    let cases = [
        ("null | .[0:2]", "null", "null"),
        (".[0.5:2.5]", "[0,1,2,3]", "[0,1,2]"),
        ("def f: .[-2.5:-0.5]; f", "[0,1,2,3]", "[1,2,3]"),
        (".[1e100:infinite]", "[0,1,2,3]", "[]"),
        (".[nan:2]", "[0,1,2,3]", "[0,1]"),
        (".[0:nan]", "[0,1,2,3]", "[0,1,2,3]"),
        ("def f: .[(\"-0.0\"|tonumber):2]; f", "[0,1,2,3]", "[0,1]"),
        (".[0:(\"-0.0\"|tonumber)]", "[0,1,2,3]", "[]"),
    ];
    for (query, input, expected) in cases {
        assert_eq!(
            evaluate(query, input).expect("slice boundary evaluates"),
            [Value::from_json(serde_json::from_str(expected).unwrap()).unwrap()],
            "{query}"
        );
    }
}
