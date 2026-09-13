//! Public VM coverage for recursive utilities through called filters.

use std::sync::{Arc, atomic::AtomicBool};

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
fn walk_called_filter_captures_value_and_rebuilds_nested_input() {
    let values = evaluate_with_limits(
        r#"def walkf($inc; f): walk(if type == "number" then f else . end); def apply($inc): walkf($inc; . + $inc); apply(1)"#,
        r#"[1,{"a":2}]"#,
        VmLimits::default(),
    )
    .expect("called walk filter evaluates");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!([2, {"a": 3}])).unwrap()]
    );
}

#[test]
fn while_called_filter_first_skips_late_error() {
    let values = evaluate_with_limits(
        r#"def first_while($seed; $limit; f): $seed | first(while(. < $limit; f)); first_while(0; 3; (. + 1, error("late")))"#,
        "null",
        VmLimits::default(),
    )
    .expect("first while result avoids the late callback branch");
    assert_eq!(values, [Value::Number("0".parse().unwrap())]);

    let values = evaluate_with_limits(
        r#"def whilef($seed; $limit; $step; f): $seed | [limit(3; while(. < $limit; f))]; def outer($step): whilef(0; 3; $step; (. + $step, error("late"))); outer(1)"#,
        "null",
        VmLimits::default(),
    )
    .expect("bounded while callback evaluates");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!([0, 1, 2])).unwrap()]
    );
}

#[test]
fn until_called_filter_preserves_empty_callback_branches_and_capture() {
    let values = evaluate_with_limits(
        r"def untilf($seed; $stop; f): $seed | until(. >= $stop; f); def outer($step): untilf(0; 3; (empty, . + $step)); outer(1)",
        "null",
        VmLimits::default(),
    )
    .expect("until callback evaluates");
    assert_eq!(values, [Value::Number("3".parse().unwrap())]);
}

#[test]
fn repeat_called_filter_preserves_multiple_callback_results() {
    let values = evaluate_with_limits(
        r"def repeatf($seed; f): $seed | limit(5; repeat(f)); def outer($step): repeatf(1; (., . + $step)); outer(1)",
        "null",
        VmLimits::default(),
    )
    .expect("repeat callback evaluates");
    assert_eq!(
        values,
        [
            Value::Number("1".parse().unwrap()),
            Value::Number("2".parse().unwrap()),
            Value::Number("1".parse().unwrap()),
            Value::Number("2".parse().unwrap()),
            Value::Number("1".parse().unwrap()),
        ]
    );
}

#[test]
fn recurse_called_filter_stops_at_empty_branch() {
    let values = evaluate_with_limits(
        r"def recursef($seed; f): $seed | [recurse(f)]; recursef(1; if . < 3 then . + 1 else empty end)",
        "null",
        VmLimits::default(),
    )
    .expect("recurse callback evaluates");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!([1, 2, 3])).unwrap()]
    );
}

#[test]
fn called_recursive_filters_honor_step_limits_and_cancellation() {
    let error = evaluate_with_limits(
        r"def loop(f): while(true; f); loop(.)",
        "0",
        VmLimits {
            steps: 32,
            ..VmLimits::default()
        },
    )
    .expect_err("unbounded called recursion must remain budgeted");
    assert!(matches!(
        error,
        VmError::Resource {
            resource: "vm-steps"
        }
    ));

    let cancellation = Arc::new(AtomicBool::new(false));
    let input = Value::Number("0".parse().unwrap());
    let query = plan(r"def loop(f): while(true; f); loop(.)");
    let mut vm =
        Vm::new(&query, input, VmLimits::default()).with_cancellation(Arc::clone(&cancellation));
    assert_eq!(
        vm.next_result()
            .expect("first recursive result is available"),
        Some(Value::Number("0".parse().unwrap()))
    );
    cancellation.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(matches!(vm.next_result(), Err(VmError::Interrupted)));
}
