//! Public behavior tests for bounded fold composition in user-defined filters.

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
fn user_reduce_preserves_generator_order_and_update_multiplicity() {
    assert_eq!(
        evaluate(
            "def f: reduce .[] as $x (0; (. + $x), (. + $x + 100)); f",
            "[1,2]",
        )
        .expect("managed reduce evaluates"),
        [Value::from_json(serde_json::json!(203)).unwrap()]
    );
    assert_eq!(
        evaluate("def f: foreach .[] as $x (0; (., . + $x)); f", "[1,2]",)
            .expect("managed foreach evaluates"),
        [
            Value::from_json(serde_json::json!(0)).unwrap(),
            Value::from_json(serde_json::json!(1)).unwrap(),
            Value::from_json(serde_json::json!(1)).unwrap(),
            Value::from_json(serde_json::json!(3)).unwrap(),
        ]
    );
}

#[test]
fn user_folds_keep_initializer_branches_independent() {
    assert_eq!(
        evaluate("def f: reduce .[] as $x (0, 10; . + $x); f", "[1,2]",)
            .expect("initializer branches evaluate"),
        [
            Value::from_json(serde_json::json!(3)).unwrap(),
            Value::from_json(serde_json::json!(13)).unwrap(),
        ]
    );
    assert_eq!(
        evaluate("def f: reduce empty as $x (0; . + $x); f", "[1,2]")
            .expect("empty generator evaluates"),
        [Value::from_json(serde_json::json!(0)).unwrap()]
    );
    assert_eq!(
        evaluate("def f: reduce .[] as $x (0; empty); f", "[1,2]").expect("empty update evaluates"),
        [Value::Null]
    );
}

#[test]
fn user_foreach_extraction_has_its_own_scope_and_cardinality() {
    assert_eq!(
        evaluate(
            "def f: foreach .[] as $item (0; . + $item; {item: $item, total: .}); f",
            "[1,2]",
        )
        .expect("extraction evaluates"),
        [
            Value::from_json(serde_json::json!({"item": 1, "total": 1})).unwrap(),
            Value::from_json(serde_json::json!({"item": 2, "total": 3})).unwrap(),
        ]
    );
    assert_eq!(
        evaluate("def f: foreach .[] as $x (0; . + $x; empty); f", "[1,2]")
            .expect("empty extraction evaluates"),
        [] as [tq_core::Value; 0]
    );
}

#[test]
fn user_fold_errors_preserve_prior_foreach_results_and_catch_scope() {
    let events = evaluate_events(
        "def f: foreach .[] as $x (0; if $x == 2 then error(\"late\") else . + $x end); f",
        "[1,2,3]",
    );
    assert_eq!(
        events,
        [
            Ok(Value::from_json(serde_json::json!(1)).unwrap()),
            Err("runtime error: late".to_owned()),
        ]
    );
    assert_eq!(
        evaluate("def f: reduce .[] as $x (0; if $x == 2 then error(\"late\") else . + $x end); try f catch .", "[1,2]")
            .expect("caught fold error evaluates"),
        [Value::string("late")]
    );
}

#[test]
fn first_stops_a_user_fold_before_late_work_and_cancellation_is_observed() {
    assert_eq!(
        evaluate(
            "def f: foreach .[] as $x (0; if $x == 2 then error(\"late\") else . + $x end); first(f)",
            "[1,2,3]",
        )
        .expect("first stops fold"),
        [Value::from_json(serde_json::json!(1)).unwrap()]
    );

    let resolved = resolve(
        parse("def f: reduce .[] as $x (0; . + $x); f").expect("query parses"),
        &ResolveOptions::default(),
    )
    .expect("query resolves");
    let plan = analyze(resolved)
        .compile()
        .expect("query compiles")
        .document_plan();
    let cancelled = Arc::new(AtomicBool::new(true));
    let mut vm = Vm::new(
        &plan,
        Value::array([1, 2, 3].map(|value| Value::from_json(serde_json::json!(value)).unwrap())),
        VmLimits::default(),
    )
    .with_cancellation(Arc::clone(&cancelled));
    assert!(matches!(vm.next_result(), Err(VmError::Interrupted)));
    cancelled.store(false, Ordering::Relaxed);
}
