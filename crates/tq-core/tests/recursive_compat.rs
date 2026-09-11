//! Public VM checks for jq's bounded recursive utility filters.

use tq_core::{ResolveOptions, Value, Vm, VmLimits, analyze, parse, resolve};

fn evaluate(query: &str, input: &str) -> Vec<Value> {
    evaluate_with_limits(query, input, VmLimits::default()).expect("query evaluates")
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
fn while_and_until_update_the_current_value() {
    assert_eq!(
        evaluate("while(. < 3; . + 1)", "1")
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["1", "2"]
    );
    assert_eq!(
        evaluate("until(. >= 3; . + 1)", "1")
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["3"]
    );
    assert!(evaluate("while(false; . + 1)", "1").is_empty());
    assert_eq!(evaluate("until(true; . + 1)", "1")[0].to_string(), "1");
}

#[test]
fn recursive_utilities_preserve_empty_and_multiple_update_cardinality() {
    assert_eq!(evaluate("while(true; empty)", "1")[0].to_string(), "1");
    assert_eq!(
        evaluate_with_limits("limit(4; while(. < 2; ., . + 1))", "1", VmLimits::default())
            .expect("bounded multiple update")
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["1", "1", "1", "1"]
    );
}

#[test]
fn recursive_conditions_preserve_empty_and_multi_result_predicates() {
    assert!(evaluate("while(empty; . + 1)", "0").is_empty());
    assert!(evaluate("until(empty; . + 1)", "0").is_empty());
    assert_eq!(
        evaluate("limit(6; while((. < 2, false); . + 1))", "0")
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["0", "1"]
    );
    assert_eq!(
        evaluate(
            "until((true, false); if . < 2 then . + 1 else empty end)",
            "0",
        )
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>(),
        ["0", "1", "2"]
    );
    assert_eq!(
        evaluate("limit(5; repeat((., . + 1)))", "0")
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["0", "1", "0", "1", "0"]
    );
}

#[test]
fn repeat_uses_the_original_input_and_pull_limits_stop_it() {
    assert_eq!(evaluate("first(repeat(. * 2))", "1")[0].to_string(), "2");
    assert_eq!(
        evaluate("limit(3; repeat(. * 2))", "1")
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["2", "2", "2"]
    );
}

#[test]
fn walk_supports_postorder_object_key_rewrites_in_document_execution() {
    assert_eq!(
        evaluate(
            "walk(if type == \"object\" then with_entries(.key |= sub(\"^_+\";\"\")) else . end)",
            "[{\"_a\":{\"__b\":2}}]",
        ),
        [serde_json::from_str::<Value>(r#"[{"a":{"b":2}}]"#).unwrap()]
    );
}

#[test]
fn walk_matches_managed_cardinality_and_empty_child_behavior() {
    assert_eq!(
        evaluate("walk((., .))", "[1]")
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["[1,1]", "[1,1]"]
    );
    assert_eq!(
        evaluate(
            "walk(if type == \"number\" then empty else . end)",
            "[1,{\"a\":2},\"x\"]",
        )
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>(),
        ["[{},\"x\"]"]
    );
}

#[test]
fn walk_stops_before_later_siblings_after_a_child_error() {
    let error = evaluate_with_limits(
        "walk(if . == 1 then error(\"first\") elif . == 2 then error(\"later\") else . end)",
        "[1,2]",
        VmLimits::default(),
    )
    .expect_err("the first child error should terminate the fallback walk");
    assert!(error.to_string().contains("first"));
}

#[test]
fn recursive_utilities_obey_a_tight_vm_budget() {
    let values = evaluate_with_limits(
        "limit(32; recurse(. + 1))",
        "0",
        VmLimits {
            call_stack: 8,
            ..VmLimits::default()
        },
    )
    .expect("iterative recurse does not consume native call stack");
    assert_eq!(values.len(), 32);
    assert_eq!(values[31].to_string(), "31");

    let tail = evaluate_with_limits(
        "last(limit(10000; recurse(. + 1)))",
        "0",
        VmLimits {
            path_stack: 4,
            steps: 100_000,
            ..VmLimits::default()
        },
    )
    .expect("a scalar recurse chain does not consume path stack per value");
    assert_eq!(tail[0].to_string(), "9999");

    let error = evaluate_with_limits(
        "first(repeat(. + 1))",
        "0",
        VmLimits {
            steps: 8,
            ..VmLimits::default()
        },
    )
    .expect("first result is available before the repeat tail");
    assert_eq!(error[0].to_string(), "1");

    let error = evaluate_with_limits(
        "while(true; .)",
        "0",
        VmLimits {
            steps: 32,
            ..VmLimits::default()
        },
    )
    .expect_err("unbounded recursive output must remain budgeted");
    assert!(matches!(
        error,
        tq_core::VmError::Resource {
            resource: "vm-steps"
        }
    ));
}
