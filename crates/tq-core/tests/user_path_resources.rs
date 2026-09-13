//! Public VM resource and cancellation contracts for path updates.

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
fn slice_assignment_requires_an_array_direct_and_through_def() {
    for query in [
        ".[1:2] = 9",
        ".[1:2] = null",
        ".[1:2] = {replacement: 9}",
        "def replace: .[1:2] = 9; replace",
        "def replace: .[1:2] = null; replace",
        "def replace: .[1:2] = {replacement: 9}; replace",
    ] {
        let error = evaluate_with_limits(query, "[0,1,2]", VmLimits::default())
            .expect_err("non-array slice assignment must fail");
        assert!(matches!(
            error,
            VmError::Runtime { message }
                if message.as_ref() == "A slice of an array can only be assigned another array"
        ));
    }
}

fn medium_array_input() -> String {
    serde_json::to_string(&(0..200).collect::<Vec<_>>()).expect("medium array serializes")
}

fn assert_bounded_path_work(query: &str) {
    let input = medium_array_input();
    let error = evaluate_with_limits(
        query,
        &input,
        VmLimits {
            steps: 64,
            ..VmLimits::default()
        },
    )
    .expect_err("medium path reconstruction must observe per-element charging");
    assert!(matches!(
        error,
        VmError::Resource {
            resource: "vm-steps"
        }
    ));
}

#[test]
fn slice_reconstruction_is_bounded_by_vm_steps() {
    assert_bounded_path_work(".[50:150] = [9]");
    assert!(
        evaluate_with_limits(
            ".[1:2] = [9]",
            "[0,1,2,3]",
            VmLimits {
                steps: 64,
                ..VmLimits::default()
            },
        )
        .is_ok()
    );
}

#[test]
fn slice_old_value_copy_is_bounded_by_vm_steps() {
    let error = evaluate_with_limits(
        ".[1:199] |= empty",
        &medium_array_input(),
        VmLimits {
            steps: 64,
            ..VmLimits::default()
        },
    )
    .expect_err("reading a large old slice must observe per-element charging");
    assert!(matches!(
        error,
        VmError::Resource {
            resource: "vm-steps"
        }
    ));
    assert_eq!(
        evaluate_with_limits(
            ".[1:3] |= empty",
            "[0,1,2,3]",
            VmLimits {
                steps: 64,
                ..VmLimits::default()
            },
        )
        .expect("small old slice remains within the budget"),
        [Value::from_json(serde_json::json!([0, 3])).unwrap()]
    );
}

#[test]
fn delete_reconstruction_is_bounded_by_vm_steps() {
    assert_bounded_path_work("del(.[100])");
    assert!(
        evaluate_with_limits(
            "del(.[1])",
            "[0,1,2,3]",
            VmLimits {
                steps: 64,
                ..VmLimits::default()
            },
        )
        .is_ok()
    );
}

#[test]
fn plain_assignment_copy_is_bounded_by_vm_steps() {
    assert_bounded_path_work(".[100] = 9");
    assert!(
        evaluate_with_limits(
            ".[1] = 9",
            "[0,1,2,3]",
            VmLimits {
                steps: 64,
                ..VmLimits::default()
            },
        )
        .is_ok()
    );
}

#[test]
fn path_depth_is_bounded_before_nested_update() {
    let error = evaluate_with_limits(
        ".a.b.c = 1",
        "{}",
        VmLimits {
            path_stack: 2,
            ..VmLimits::default()
        },
    )
    .expect_err("nested path must observe the managed path depth");
    assert!(matches!(
        error,
        VmError::Resource {
            resource: "path-stack"
        }
    ));
}

#[test]
fn deferred_deletions_are_not_limited_by_pending_fork_width() {
    let values = evaluate_with_limits(
        "def update: (range(0;5000) as $unused | .a) |= empty; update",
        r#"{"a":1,"b":2}"#,
        VmLimits::default(),
    )
    .expect("many sequential deletion targets are not simultaneous pending forks");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!({"b": 2})).unwrap()]
    );
}

#[test]
fn deferred_deletion_work_remains_bounded() {
    let error = evaluate_with_limits(
        "def update: .[] |= empty; update",
        &medium_array_input(),
        VmLimits {
            steps: 64,
            ..VmLimits::default()
        },
    )
    .expect_err("deferred deletion still charges VM work");
    assert!(matches!(
        error,
        VmError::Resource {
            resource: "vm-steps"
        }
    ));
}

#[test]
fn cancellation_is_observed_before_path_work() {
    let cancellation = Arc::new(AtomicBool::new(true));
    let query = ".a[1:2] = [9]";
    let plan = plan(query);
    let input = Value::from_json(serde_json::json!({"a": [0, 1, 2]})).unwrap();
    let mut vm =
        Vm::new(&plan, input, VmLimits::default()).with_cancellation(Arc::clone(&cancellation));
    assert_eq!(vm.next_result(), Err(VmError::Interrupted));
}
