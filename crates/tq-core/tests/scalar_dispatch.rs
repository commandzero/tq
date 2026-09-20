//! Synchronous scalar dispatch preserves managed execution contracts.

use std::sync::{Arc, atomic::AtomicBool};
use tq_core::{ResolveOptions, Value, Vm, VmError, VmLimits, analyze, parse, resolve};

fn evaluate(query: &str, input: &str, steps: u64, cancelled: bool) -> Result<Vec<Value>, VmError> {
    let plan = analyze(resolve(parse(query).unwrap(), &ResolveOptions::default()).unwrap())
        .compile()
        .unwrap()
        .document_plan();
    let input = Value::from_json(serde_json::from_str(input).unwrap()).unwrap();
    let mut vm = Vm::new(
        &plan,
        input,
        VmLimits {
            steps,
            ..VmLimits::default()
        },
    )
    .with_cancellation(Arc::new(AtomicBool::new(cancelled)));
    let mut values = Vec::new();
    vm.for_each_result(|value| {
        values.push(value);
        true
    })?;
    Ok(values)
}

#[test]
fn zero_argument_scalar_keeps_task_and_operation_charges() {
    assert_eq!(
        evaluate("length", "[1,2]", 3, false).unwrap(),
        [Value::from_json(serde_json::json!(2)).unwrap()]
    );
    assert!(matches!(
        evaluate("length", "[1,2]", 2, false),
        Err(VmError::Resource {
            resource: "vm-steps"
        })
    ));
    assert!(matches!(
        evaluate("try length catch 0", "[1,2]", 3, false),
        Err(VmError::Resource {
            resource: "vm-steps"
        })
    ));
    assert!(matches!(
        evaluate("length", "[1,2]", 100, true),
        Err(VmError::Interrupted)
    ));
}

#[test]
fn scalar_dispatch_preserves_filtering_order_and_catch() {
    let values = evaluate(
        ".[] | (numbers, try length catch 99)",
        "[7,true,8]",
        1000,
        false,
    )
    .unwrap();
    let expected = [7, 7, 99, 8, 8]
        .into_iter()
        .map(|n| Value::from_json(serde_json::json!(n)).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(values, expected);
}

#[test]
fn scalar_dispatch_preserves_identity_and_fresh_origins() {
    for query in [
        ". as $p | numbers | ($p = 2)",
        ". as $p | tonumber | ($p = 2)",
    ] {
        assert!(evaluate(query, "1", 1000, false).is_ok(), "{query}");
    }
    assert!(evaluate(". as $p | trim | ($p = 2)", "\"abc\"", 1000, false).is_ok());
    for (query, input) in [
        ("length as $p | ($p = 2)", "1"),
        (". as $p | trim | ($p = 2)", "\" abc \""),
    ] {
        assert!(
            matches!(
                evaluate(query, input.trim(), 1000, false),
                Err(VmError::Runtime { .. })
            ),
            "{query}"
        );
    }
}
