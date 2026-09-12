//! Public VM resource coverage for scalar helpers that materialize values.

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

fn array_input(length: usize) -> String {
    serde_json::to_string(&(0..length).collect::<Vec<_>>()).expect("array serializes")
}

#[test]
fn scalar_copy_helpers_charge_each_retained_item() {
    let input = array_input(32);
    for query in [
        "def collect: to_entries; collect",
        "def collect: sort; collect",
        "def collect: unique; collect",
        "def collect: add; collect",
    ] {
        let error = evaluate_with_limits(
            query,
            &input,
            VmLimits {
                steps: 8,
                ..VmLimits::default()
            },
        )
        .expect_err("scalar materialization must observe the VM step budget");
        assert!(
            matches!(
                error,
                VmError::Resource {
                    resource: "vm-steps"
                }
            ),
            "{query}"
        );
    }

    let small = evaluate_with_limits(
        "def collect: sort; collect",
        "[3,1,2]",
        VmLimits {
            steps: 64,
            ..VmLimits::default()
        },
    )
    .expect("small scalar materialization remains supported");
    assert_eq!(
        small,
        [Value::from_json(serde_json::json!([1, 2, 3])).unwrap()]
    );
}

#[test]
fn scalar_string_and_flatten_work_charge_is_bounded() {
    let error = evaluate_with_limits(
        "def collect: explode; collect",
        &serde_json::to_string(&"a".repeat(32)).unwrap(),
        VmLimits {
            steps: 8,
            ..VmLimits::default()
        },
    )
    .expect_err("explode must charge each output item");
    assert!(matches!(
        error,
        VmError::Resource {
            resource: "vm-steps"
        }
    ));

    let error = evaluate_with_limits(
        "def collect: flatten; collect",
        "[[[[0]]]]",
        VmLimits {
            steps: 256,
            value_stack: 3,
            ..VmLimits::default()
        },
    )
    .expect_err("flatten must bound its explicit work stack");
    assert!(matches!(
        error,
        VmError::Resource {
            resource: "value-stack"
        }
    ));

    for query in [
        "def collect: flatten(-1); collect",
        "def collect: flatten(-9223372036854775808); collect",
    ] {
        let error = evaluate_with_limits(query, "[[0]]", VmLimits::default())
            .expect_err("negative flatten depths must remain invalid");
        assert!(matches!(error, VmError::Runtime { message } if message.contains("negative")));
    }
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "the scalar materialization matrix keeps its quota cases together"
)]
fn scalar_materialization_rejects_oversized_results_before_copying() {
    let input = array_input(16);
    for query in [
        "def collect: keys; collect",
        "def collect: to_entries; collect",
        "def collect: reverse; collect",
        "def collect: from_entries; collect",
    ] {
        let source = if query.contains("from_entries") {
            r#"[{"key":"a","value":1},{"key":"b","value":2},{"key":"c","value":3},{"key":"d","value":4},{"key":"e","value":5},{"key":"f","value":6}]"#
        } else {
            &input
        };
        let error = evaluate_with_limits(
            query,
            source,
            VmLimits {
                output_bytes: 4,
                steps: 256,
                ..VmLimits::default()
            },
        )
        .expect_err("scalar materialization must reject oversized work");
        assert!(
            matches!(
                error,
                VmError::Resource {
                    resource: "output-bytes"
                }
            ),
            "{query}"
        );
    }

    let duplicates = evaluate_with_limits(
        "def collect: from_entries; collect",
        r#"[{"key":"a","value":1},{"key":"a","value":2},{"key":"a","value":3},{"key":"a","value":4},{"key":"a","value":5},{"key":"a","value":6}]"#,
        VmLimits {
            output_bytes: 4,
            steps: 256,
            ..VmLimits::default()
        },
    )
    .expect("duplicate from_entries keys retain one bounded object entry");
    assert_eq!(
        duplicates,
        [Value::from_json(serde_json::json!({"a": 6})).unwrap()]
    );

    let unique = evaluate_with_limits(
        "def collect: unique; collect",
        "[1,1,1,1,1,1]",
        VmLimits {
            output_bytes: 3,
            steps: 256,
            ..VmLimits::default()
        },
    )
    .expect("duplicate unique values retain one bounded array entry");
    assert_eq!(unique, [Value::from_json(serde_json::json!([1])).unwrap()]);

    let error = evaluate_with_limits(
        "def collect: add; collect",
        "[\"aaaa\",\"bbbb\"]",
        VmLimits {
            output_bytes: 7,
            steps: 256,
            ..VmLimits::default()
        },
    )
    .expect_err("string concatenation must observe the output quota");
    assert!(matches!(
        error,
        VmError::Resource {
            resource: "output-bytes"
        }
    ));

    let array_add = evaluate_with_limits(
        "def collect: add; collect",
        "[[1,2],[3,4]]",
        VmLimits {
            output_bytes: 6,
            steps: 256,
            ..VmLimits::default()
        },
    )
    .expect("small array concatenation remains supported");
    assert_eq!(
        array_add,
        [Value::from_json(serde_json::json!([1, 2, 3, 4])).unwrap()]
    );

    let duplicate_object_add = evaluate_with_limits(
        "def collect: add; collect",
        "[{\"a\":1},{\"a\":2}]",
        VmLimits {
            output_bytes: 3,
            steps: 256,
            ..VmLimits::default()
        },
    )
    .expect("duplicate object keys retain one bounded entry");
    assert_eq!(
        duplicate_object_add,
        [Value::from_json(serde_json::json!({"a": 2})).unwrap()]
    );

    assert!(
        evaluate_with_limits(
            "def collect: keys; collect",
            &array_input(5_001),
            VmLimits {
                steps: 6_000,
                ..VmLimits::default()
            },
        )
        .is_ok()
    );
}
