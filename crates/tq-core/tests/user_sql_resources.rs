//! Public VM resource coverage for pull-driven SQL built-ins.

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

fn evaluate_until_error(query: &str, input: &str) -> (Vec<Value>, VmError) {
    let input = Value::from_json(serde_json::from_str(input).expect("valid JSON input"))
        .expect("JSON converts to a tq value");
    let plan = plan(query);
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

fn rows_json(count: usize) -> String {
    serde_json::to_string(
        &(0..count)
            .map(|id| serde_json::json!({ "id": id }))
            .collect::<Vec<_>>(),
    )
    .expect("rows serialize")
}

#[test]
fn in_short_circuits_infinite_right_stream_and_empty_right_is_false() {
    let values = evaluate_with_limits(
        r"def contains(f; g): IN(f; g); contains(1; repeat(1))",
        "null",
        VmLimits {
            steps: 128,
            ..VmLimits::default()
        },
    )
    .expect("membership itself stops after a match without consuming the infinite tail");
    assert_eq!(values, [Value::Bool(true)]);

    let values = evaluate_with_limits(
        r#"def contains(f; g): IN(f; g); contains(error("left"); empty)"#,
        "null",
        VmLimits::default(),
    )
    .expect("empty right stream avoids evaluating the left filter");
    assert_eq!(values, [Value::Bool(false)]);
}

#[test]
fn index_enforces_materialization_quota_without_flat_width_cap() {
    let error = evaluate_with_limits(
        r"def make(f; g): INDEX(f; g); make(.[]; .id)",
        r#"[{"id":"a"},{"id":"b"}]"#,
        VmLimits {
            output_bytes: 5,
            ..VmLimits::default()
        },
    )
    .expect_err("retained index keys must respect output quota");
    assert!(matches!(
        error,
        VmError::Resource {
            resource: "output-bytes"
        }
    ));

    let values = evaluate_with_limits(
        r"def make(f; g): INDEX(f; g); make(.[]; .id)",
        &rows_json(5_000),
        VmLimits::default(),
    )
    .expect("a shallow index may exceed the temporary value-stack width");
    assert_eq!(values.len(), 1);
    match &values[0] {
        Value::Object(index) => assert_eq!(index.len(), 5_000),
        value => panic!("INDEX returned {value:?}, expected object"),
    }
}

#[test]
fn join_three_streams_early_but_join_two_buffers_before_error() {
    let values = evaluate_with_limits(
        r#"def j3($idx; f; g): JOIN($idx; f; g); first(j3({"a":1}; ("a", error("late")); .))"#,
        "null",
        VmLimits::default(),
    )
    .expect("JOIN/3 emits its first pair before a later source error");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!(["a", 1])).unwrap()]
    );

    let (values, error) = evaluate_until_error(
        r#"def j2($idx; f): JOIN($idx; f); j2({"a":1}; if .id == "b" then error("late") else .id end)"#,
        r#"[{"id":"a"},{"id":"b"}]"#,
    );
    assert!(values.is_empty(), "JOIN/2 buffers before emitting");
    assert!(error.to_string().contains("late"));
}

#[test]
fn join_three_observes_cancellation_after_first_streamed_result() {
    let cancellation = Arc::new(AtomicBool::new(false));
    let query = plan(r#"def j3($idx; f; g): JOIN($idx; f; g); j3({"a":1}; ("a", recurse(.)); .)"#);
    let mut vm = Vm::new(&query, Value::Null, VmLimits::default())
        .with_cancellation(Arc::clone(&cancellation));
    assert_eq!(
        vm.next_result().expect("first JOIN/3 result evaluates"),
        Some(Value::from_json(serde_json::json!(["a", 1])).unwrap())
    );
    cancellation.store(true, Ordering::Relaxed);
    assert!(matches!(vm.next_result(), Err(VmError::Interrupted)));
}
