//! Embedded VM resource and cancellation contracts for composed user filters.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

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
fn recursive_constructor_composition_stays_bounded_by_managed_stacks() {
    let values = evaluate_with_limits(
        "def descend: if . == 0 then {n: .} else {n: ., next: ((. - 1) | descend)} end; descend",
        "3",
        VmLimits {
            call_stack: 32,
            steps: 256,
            ..VmLimits::default()
        },
    )
    .expect("recursive constructor composition evaluates");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!({
            "n": 3,
            "next": {"n": 2, "next": {"n": 1, "next": {"n": 0}}}
        }))
        .unwrap()]
    );
}

#[test]
fn recursive_constructor_reports_call_stack_exhaustion_precisely() {
    let error = evaluate_with_limits(
        "def descend: if . == 0 then {n: .} else {n: ., next: ((. - 1) | descend)} end; descend",
        "100",
        VmLimits {
            call_stack: 8,
            steps: 10_000,
            ..VmLimits::default()
        },
    )
    .expect_err("deep recursive constructor must exhaust the managed call stack");
    assert!(matches!(
        error,
        VmError::Resource {
            resource: "call-stack"
        }
    ));
}

#[test]
fn recursive_constructor_reports_vm_step_exhaustion_with_generous_stacks() {
    let error = evaluate_with_limits(
        "def descend: if . == 0 then {n: .} else {n: ., next: ((. - 1) | descend)} end; descend",
        "100",
        VmLimits {
            call_stack: 128,
            path_stack: 128,
            fork_stack: 1024,
            steps: 32,
            ..VmLimits::default()
        },
    )
    .expect_err("deep recursive constructor must exhaust the VM step budget");
    assert!(matches!(
        error,
        VmError::Resource {
            resource: "vm-steps"
        }
    ));
}

#[test]
fn fold_composition_preserves_value_and_filter_parameters_under_limits() {
    let values = evaluate_with_limits(
        "def scale($factor; f): reduce .[] as $x (0; . + (($x * $factor) | f)); scale(1; sqrt)",
        "[1,4,9]",
        VmLimits {
            call_stack: 8,
            steps: 256,
            ..VmLimits::default()
        },
    )
    .expect("fold callback composition evaluates");
    assert_eq!(values, [Value::from_json(serde_json::json!(6)).unwrap()]);
}

#[test]
fn cancellation_is_observed_between_composed_callback_results() {
    let query = "def twice(f): f, f; twice(.)";
    let plan = plan(query);
    let cancellation = Arc::new(AtomicBool::new(false));
    let mut vm = Vm::new(
        &plan,
        Value::from_json(serde_json::json!(7)).unwrap(),
        VmLimits::default(),
    )
    .with_cancellation(Arc::clone(&cancellation));

    assert_eq!(
        vm.next_result().expect("first callback result"),
        Some(Value::from_json(serde_json::json!(7)).unwrap())
    );
    cancellation.store(true, Ordering::Relaxed);
    assert_eq!(vm.next_result(), Err(VmError::Interrupted));
}

#[test]
fn first_composed_callback_skips_late_error_and_effect() {
    let query = r#"def first_of(f): first(f); first_of((1, (debug("late") | error("late"))))"#;
    let plan = plan(query);
    let mut vm = Vm::new(&plan, Value::Null, VmLimits::default());
    assert_eq!(
        vm.next_result().expect("first callback result"),
        Some(Value::from_json(serde_json::json!(1)).unwrap())
    );
    assert_eq!(vm.take_effects(), [] as [u8; 0]);
    assert_eq!(vm.next_result().expect("first completes"), None);
}

#[test]
fn limit_composed_recursive_generator_without_collecting_the_tail() {
    let values = evaluate_with_limits(
        "def many: recurse(. + 1); limit(3; many)",
        "0",
        VmLimits {
            call_stack: 8,
            steps: 256,
            ..VmLimits::default()
        },
    )
    .expect("bounded recursive generator evaluates");
    assert_eq!(
        values,
        [
            Value::from_json(serde_json::json!(0)).unwrap(),
            Value::from_json(serde_json::json!(1)).unwrap(),
            Value::from_json(serde_json::json!(2)).unwrap(),
        ]
    );
}

#[test]
fn composed_catches_preserve_typed_errors_and_capture_independent_branches() {
    let values = evaluate_with_limits(
        r#"def recover($payload): try error($payload) catch .; [recover(42), recover({"tag":"object"}), recover(null)]"#,
        "null",
        VmLimits::default(),
    )
    .expect("typed errors remain catchable values");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!([42, {"tag": "object"}, null])).unwrap()]
    );

    let branches = evaluate_with_limits(
        r"def branch($prefix): [$prefix, ($prefix + 1)]; [branch(10), branch(20)]",
        "null",
        VmLimits::default(),
    )
    .expect("captured branches evaluate independently");
    assert_eq!(
        branches,
        [Value::from_json(serde_json::json!([[10, 11], [20, 21]])).unwrap()]
    );
}
