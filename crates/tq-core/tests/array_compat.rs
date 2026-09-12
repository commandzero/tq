//! Public VM checks for jq array combinations, transpose, and binary search.

use tq_core::{ResolveOptions, Value, Vm, VmLimits, analyze, parse, resolve};

fn evaluate(query: &str, input: &str) -> Vec<Value> {
    evaluate_result(query, input).expect("query evaluates")
}

fn evaluate_result(query: &str, input: &str) -> Result<Vec<Value>, tq_core::VmError> {
    evaluate_with_limits(query, input, VmLimits::default())
}

fn evaluate_with_limits(
    query: &str,
    input: &str,
    limits: VmLimits,
) -> Result<Vec<Value>, tq_core::VmError> {
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
    let mut vm = Vm::new(&plan, input, limits);
    let mut values = Vec::new();
    loop {
        match vm.next_result()? {
            Some(value) => values.push(value),
            None => return Ok(values),
        }
    }
}

#[test]
fn combinations_cover_empty_and_cardinality_forms() {
    assert_eq!(
        evaluate("combinations", "[[1,2],[\"a\",\"b\"]]")
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["[1,\"a\"]", "[1,\"b\"]", "[2,\"a\"]", "[2,\"b\"]"]
    );
    assert_eq!(evaluate("combinations", "[]")[0].to_string(), "[]");
    assert_eq!(
        evaluate("combinations(2)", "[1,2]")
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["[1,1]", "[1,2]", "[2,1]", "[2,2]"]
    );
    assert_eq!(evaluate("combinations(0)", "[1,2]")[0].to_string(), "[]");
    assert_eq!(evaluate("combinations(-1)", "[1,2]")[0].to_string(), "[]");
    assert_eq!(
        evaluate("combinations", "[[1],[]]"),
        [] as [tq_core::Value; 0]
    );
    assert_eq!(
        evaluate("combinations((1,2))", "[1,2]")
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        [
            "[1,1,1]", "[1,1,2]", "[1,2,1]", "[1,2,2]", "[2,1,1]", "[2,1,2]", "[2,2,1]", "[2,2,2]",
        ]
    );
    assert!(evaluate_result("combinations", "[1,2]").is_err());
}

#[test]
fn transpose_pads_ragged_rows_and_preserves_empty_matrix() {
    assert_eq!(
        evaluate("transpose", "[[1],[2,3]]")[0].to_string(),
        "[[1,2],[null,3]]"
    );
    assert_eq!(evaluate("transpose", "[]")[0].to_string(), "[]");
    assert_eq!(
        evaluate("transpose", "[[1,2],[3]]")[0].to_string(),
        "[[1,3],[2,null]]"
    );
    let error = evaluate_with_limits(
        "transpose",
        "[[],[]]",
        VmLimits {
            steps: 2,
            ..VmLimits::default()
        },
    )
    .expect_err("zero-width rows still consume scan work");
    assert!(matches!(
        error,
        tq_core::VmError::Resource {
            resource: "vm-steps"
        }
    ));
}

#[test]
fn bsearch_returns_matches_and_negative_insertion_positions() {
    for (needle, expected) in [("1", "0"), ("2", "-2"), ("4", "-3"), ("6", "-4")] {
        assert_eq!(
            evaluate(&format!("bsearch({needle})"), "[1,3,5]")[0].to_string(),
            expected,
            "needle {needle}"
        );
    }
    assert_eq!(evaluate("bsearch(2)", "[]")[0].to_string(), "-1");
    assert_eq!(evaluate("bsearch(2)", "[1,2,2,2,3]")[0].to_string(), "2");
}

#[test]
fn array_searches_reject_wrong_shapes_and_charge_work() {
    assert!(evaluate_result("transpose", "[1]").is_err());
    assert!(evaluate_result("combinations", "[1]").is_err());
    assert!(evaluate_result("bsearch(1)", "\"not-an-array\"").is_err());
    let error = evaluate_with_limits(
        "combinations(1e100)",
        "[1]",
        VmLimits {
            steps: 32,
            ..VmLimits::default()
        },
    )
    .expect_err("huge dimension count must fail before allocation");
    assert!(matches!(
        error,
        tq_core::VmError::Resource {
            resource: "fork-stack"
        }
    ));
    assert_eq!(
        evaluate("limit(1; [range(0;20) | [0,1]] | combinations)", "null")[0].to_string(),
        "[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]"
    );
    let wide = evaluate_with_limits(
        "combinations(2048)",
        "[0]",
        VmLimits {
            fork_stack: 4096,
            ..VmLimits::default()
        },
    )
    .expect("wide product uses an iterative odometer, not native recursion");
    let Value::Array(wide) = &wide[0] else {
        panic!("wide combinations result must be an array");
    };
    assert_eq!(wide.len(), 2048);
    let input = serde_json::to_string(&(0..256).collect::<Vec<_>>()).expect("array JSON");
    let input: Value = serde_json::from_str(&input).expect("array value");
    let plan = analyze(
        resolve(
            parse("bsearch(1000)").expect("query parses"),
            &ResolveOptions::default(),
        )
        .expect("query resolves"),
    )
    .compile()
    .expect("query compiles")
    .document_plan();
    let mut vm = Vm::new(
        &plan,
        input,
        VmLimits {
            steps: 3,
            ..VmLimits::default()
        },
    );
    assert!(matches!(
        vm.next_result(),
        Err(tq_core::VmError::Resource {
            resource: "vm-steps"
        })
    ));
}
