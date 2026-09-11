//! Embedded VM admission tests for composed scalar and generator built-ins.

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
fn scalar_value_and_filter_arguments_keep_cartesian_order() {
    let direct = evaluate_with_limits(
        "def direct: [pow((2,3);(2,3))]; direct",
        "null",
        VmLimits {
            steps: 256,
            ..VmLimits::default()
        },
    )
    .expect("direct scalar Cartesian composition evaluates");
    assert_eq!(
        direct,
        [Value::from_json(serde_json::json!([4, 9, 8, 27])).unwrap()]
    );

    let values = evaluate_with_limits(
        "def powers($base; f): [$base, pow($base; f)]; [powers((2,3); (2,3))]",
        "null",
        VmLimits {
            steps: 256,
            ..VmLimits::default()
        },
    )
    .expect("scalar value/filter composition evaluates");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!([[2, 4, 8], [3, 9, 27]])).unwrap()]
    );
}

#[test]
fn composed_range_preserves_decimal_first_values() {
    let values = evaluate_with_limits(
        "def decimal: [range(1.00; 3.00; 1.00) | tojson]; decimal",
        "null",
        VmLimits {
            steps: 128,
            ..VmLimits::default()
        },
    )
    .expect("decimal range composition evaluates");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!(["1.00", "2"])).unwrap()]
    );
}

#[test]
fn composed_range_handles_zero_and_nan_steps_without_hanging() {
    let values = evaluate_with_limits(
        "def edge_steps: [[limit(2; range(0;3;0))], [limit(2; range(0;3;nan))]]; edge_steps",
        "null",
        VmLimits {
            steps: 128,
            ..VmLimits::default()
        },
    )
    .expect("bounded zero and NaN ranges evaluate");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!([[], []])).unwrap()]
    );
}

#[test]
fn composed_range_no_progress_at_large_decimal_step_stays_bounded() {
    let values = evaluate_with_limits(
        "def no_progress: [limit(3; range(1e20;1e21;1) | tojson)]; no_progress",
        "null",
        VmLimits {
            steps: 128,
            ..VmLimits::default()
        },
    )
    .expect("bounded large-decimal range evaluates");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!(["1E+20", "1e+20", "1e+20"])).unwrap()]
    );
}

#[test]
fn composed_range_nan_start_is_observable_as_nan() {
    let values = evaluate_with_limits(
        "def nan_start: [limit(2; range(nan;3) | isnan)]; nan_start",
        "null",
        VmLimits {
            steps: 128,
            ..VmLimits::default()
        },
    )
    .expect("bounded NaN-start range evaluates");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!([true, true])).unwrap()]
    );
}

#[test]
fn combinations_with_generator_arguments_accumulate_width_three() {
    let values = evaluate_with_limits(
        "def combos: [combinations(1,2)]; combos",
        "[1,2]",
        VmLimits {
            steps: 256,
            ..VmLimits::default()
        },
    )
    .expect("width-three combinations evaluate");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!([
            [1, 1, 1],
            [1, 1, 2],
            [1, 2, 1],
            [1, 2, 2],
            [2, 1, 1],
            [2, 1, 2],
            [2, 2, 1],
            [2, 2, 2]
        ]))
        .unwrap()]
    );
}

#[test]
fn combinations_preserve_empty_counts_and_object_iterables() {
    let empty = evaluate_with_limits(
        "def empty_count: [combinations(empty)]; empty_count",
        "null",
        VmLimits::default(),
    )
    .expect("empty combination counts evaluate");
    assert_eq!(empty, [Value::from_json(serde_json::json!([[]])).unwrap()]);

    let objects = evaluate_with_limits(
        "def object_values: [combinations(2)]; object_values",
        "{\"a\":1,\"b\":2}",
        VmLimits::default(),
    )
    .expect("object combinations evaluate");
    assert_eq!(
        objects,
        [Value::from_json(serde_json::json!([[1, 1], [1, 2], [2, 1], [2, 2]])).unwrap()]
    );
}

#[test]
fn first_composed_combination_stops_before_the_remaining_generator() {
    let values = evaluate_with_limits(
        "def first_combo: first(combinations(1,2)); first_combo",
        "[1,2]",
        VmLimits {
            steps: 128,
            ..VmLimits::default()
        },
    )
    .expect("first combination evaluates");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!([1, 1, 1])).unwrap()]
    );
}

#[test]
fn generated_values_can_become_assignment_inputs() {
    let values = evaluate_with_limits(
        "def generated: [range(0;2) | (. as $p | ($p = 9))]; generated",
        "null",
        VmLimits {
            steps: 256,
            ..VmLimits::default()
        },
    )
    .expect("generated values retain an assignment identity");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!([9, 9])).unwrap()]
    );
}

#[test]
fn composed_bsearch_preserves_argument_generator_order() {
    let values = evaluate_with_limits(
        "def search: [bsearch((2,4))]; search",
        "[1,3,5]",
        VmLimits {
            steps: 256,
            ..VmLimits::default()
        },
    )
    .expect("bsearch arguments compose through the managed evaluator");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!([-2, -3])).unwrap()]
    );
}

#[test]
fn tostream_composition_closes_nested_empty_containers_in_order() {
    let values = evaluate_with_limits(
        "def id: .; def stream: [{a: [], b: {}, c: [[],{}]} | tostream]; id | stream",
        "null",
        VmLimits {
            steps: 256,
            ..VmLimits::default()
        },
    )
    .expect("nested empty stream composition evaluates");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!([
            [["a"], []],
            [["b"], {}],
            [["c", 0], []],
            [["c", 1], {}],
            [["c", 1]],
            [["c"]]
        ]))
        .unwrap()]
    );
}
