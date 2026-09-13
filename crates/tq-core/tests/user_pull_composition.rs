//! Public VM coverage for pull-driven and stream-consuming built-ins.

use std::fmt::Write;
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
fn pull_consumers_preserve_captured_values_and_filter_arguments() {
    let values = evaluate_with_limits(
        "def pick($items; f): [($items | first), ($items | last), ($items | nth(1; f)), ($items | [skip(1; f)])]; pick([1,2,3]; .[])",
        "null",
        VmLimits {
            steps: 256,
            ..VmLimits::default()
        },
    )
    .expect("captured pull consumers evaluate");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!([1, 3, 2, [2, 3]])).unwrap()]
    );
}

#[test]
fn pull_consumers_preserve_empty_and_multiple_count_arguments() {
    let values = evaluate_with_limits(
        "def pick($items; f): [(($items | first)), (($items | last)), ($items | nth((0,1); f)), ($items | [skip((1,2); f)])]; pick([10,20,30]; .[])",
        "null",
        VmLimits {
            steps: 256,
            ..VmLimits::default()
        },
    )
    .expect("multiple count arguments evaluate");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!([10, 30, 10, 20, [20, 30, 30]])).unwrap()]
    );

    let empty = evaluate_with_limits(
        "def empty_pull: [first(empty), last(empty), nth(5; empty)]; empty_pull",
        "null",
        VmLimits::default(),
    )
    .expect("empty pull consumers evaluate");
    assert_eq!(empty, [Value::array(Vec::new())]);
}

#[test]
fn pull_consumers_stop_before_late_errors_and_bound_work() {
    let values = evaluate_with_limits(
        "def pulls: [first(1, error(\"late\")), nth(0; 1, error(\"late\"))]; pulls",
        "null",
        VmLimits::default(),
    )
    .expect("first and nth stop before a later error");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!([1, 1])).unwrap()]
    );

    let error = evaluate_with_limits(
        "def take(f): last(f); take(1, error(\"late\"))",
        "null",
        VmLimits::default(),
    )
    .expect_err("last must inspect the later error");
    assert!(matches!(
        error,
        VmError::Raised {
            value: Value::String(message),
            ..
        } if message.contains("late")
    ));

    let first = evaluate_with_limits(
        "def take(f): first(f); take(range(0; 1000000))",
        "null",
        VmLimits {
            steps: 64,
            ..VmLimits::default()
        },
    )
    .expect("first must terminate without consuming the tail");
    assert_eq!(first, [Value::from_json(serde_json::json!(0)).unwrap()]);

    let error = evaluate_with_limits(
        "def take(f): last(f); take(range(0; 1000000))",
        "null",
        VmLimits {
            steps: 64,
            ..VmLimits::default()
        },
    )
    .expect_err("last must observe the bounded work limit");
    assert!(matches!(
        error,
        VmError::Resource {
            resource: "vm-steps"
        }
    ));
}

#[test]
fn pull_consumer_arities_and_fractional_counts_match_jq() {
    let direct = evaluate_with_limits(
        "def direct: [first, last, nth(-1), nth(5)]; direct",
        "[0,1,2]",
        VmLimits::default(),
    )
    .expect("direct pull arities evaluate");
    assert_eq!(
        direct,
        [Value::from_json(serde_json::json!([0, 2, 2, null])).unwrap()]
    );

    let fractional = evaluate_with_limits(
        "def fractional: [nth(0.5; range(4)), [skip(0.5; range(4))]]; fractional",
        "null",
        VmLimits::default(),
    )
    .expect("fractional pull counts evaluate");
    assert_eq!(
        fractional,
        [Value::from_json(serde_json::json!([0, [0, 1, 2, 3]])).unwrap()]
    );
}

#[test]
fn pull_consumers_preserve_selected_input_origins() {
    let cases = [
        ("[1,2] | .[1] as $p | last | ($p = 9)", "[1,2]"),
        ("[1,2] | .[1] as $p | nth(1) | ($p = 9)", "[1,2]"),
        ("[1,2] | .[1] as $p | last(.[]) | ($p = 9)", "[1,2]"),
        ("1 | . as $p | last(.) | ($p = 9)", "null"),
        ("1 | . as $p | nth(0; .) | ($p = 9)", "null"),
    ];
    for (query, input) in cases {
        assert_eq!(
            evaluate_with_limits(query, input, VmLimits::default())
                .expect("pull preserves selected origin"),
            [Value::from_json(serde_json::json!(9)).unwrap()],
            "query: {query}",
        );
    }
}

#[test]
fn managed_iteration_preserves_order_errors_and_early_stop() {
    let object = evaluate_with_limits("[.[]]", r#"{"first":1,"second":2}"#, VmLimits::default())
        .expect("object iteration evaluates");
    assert_eq!(
        object,
        [Value::from_json(serde_json::json!([1, 2])).unwrap()]
    );

    let caught = evaluate_with_limits(
        "def children: .[]; try children catch \"caught\"",
        "1",
        VmLimits::default(),
    )
    .expect("scalar iteration error is catchable");
    assert_eq!(caught, [Value::string("caught")]);

    let mut source = String::from("{");
    for index in 0..1_000 {
        if index > 0 {
            source.push(',');
        }
        let _ = write!(source, "\"field{index}\":{index}");
    }
    source.push('}');
    let first = evaluate_with_limits(
        "def take(f): first(f); take(.[]) ",
        &source,
        VmLimits {
            steps: 64,
            ..VmLimits::default()
        },
    )
    .expect("first object child stops before the remaining fields");
    assert_eq!(first, [Value::from_json(serde_json::json!(0)).unwrap()]);

    let alias = evaluate_with_limits(
        "def pick(f): first(f); {a:1,b:2} | .a as $p | pick(.[]) | ($p = 9)",
        "null",
        VmLimits::default(),
    )
    .expect("called object iteration preserves child origin");
    assert_eq!(alias, [Value::from_json(serde_json::json!(9)).unwrap()]);

    let deep = evaluate_with_limits(
        "first(.[] | .[])",
        "[[1]]",
        VmLimits {
            path_stack: 1,
            ..VmLimits::default()
        },
    )
    .expect_err("deep child origin must obey the path-stack bound");
    assert_eq!(
        deep,
        VmError::Resource {
            resource: "path-stack"
        }
    );
}

#[test]
fn fromstream_uses_captured_value_and_filter_arguments() {
    let values = evaluate_with_limits(
        "def reconstruct($events; f): [fromstream($events[] | f)]; reconstruct([[[\"a\"],1],[[\"a\"]],[[\"b\"],2],[[\"b\"]]]; .)",
        "null",
        VmLimits {
            steps: 256,
            ..VmLimits::default()
        },
    )
    .expect("captured fromstream evaluates");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!([{"a": 1}, {"b": 2}])).unwrap()]
    );

    let first = evaluate_with_limits(
        "def take(f): first(f); take(fromstream((([1] | tostream), error(\"late\"))))",
        "null",
        VmLimits::default(),
    )
    .expect("first fromstream result stops before the later error");
    assert_eq!(first, [Value::from_json(serde_json::json!([1])).unwrap()]);
}

#[test]
fn fromstream_drops_incomplete_root_at_end_of_input() {
    let incomplete = evaluate_with_limits(
        "def reconstruct(f): [fromstream(f)]; reconstruct([[0],1])",
        "null",
        VmLimits::default(),
    )
    .expect("incomplete fromstream input evaluates");
    assert_eq!(incomplete, [Value::array(Vec::new())]);

    let closed = evaluate_with_limits(
        "def reconstruct(f): [fromstream(f)]; reconstruct(([[0],1],[[0]]))",
        "null",
        VmLimits::default(),
    )
    .expect("closed fromstream input evaluates");
    assert_eq!(
        closed,
        [Value::from_json(serde_json::json!([[1]])).unwrap()]
    );
}

#[test]
fn truncate_stream_uses_captured_value_and_filter_arguments() {
    let values = evaluate_with_limits(
        "def trunc($count; f): $count | [truncate_stream(f)]; trunc(1; ([[0],\"a\"],[[1,0],\"b\"],[[1,0]],[[1]]))",
        "null",
        VmLimits {
            steps: 256,
            ..VmLimits::default()
        },
    )
    .expect("captured truncate_stream evaluates");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!([[[0], "b"], [[0]]])).unwrap()]
    );

    let rebuilt = evaluate_with_limits(
        "def trunc($count; f): $count | fromstream(truncate_stream(f)); trunc(1; ([[0],\"a\"],[[1,0],\"b\"],[[1,0]],[[1]]))",
        "null",
        VmLimits::default(),
    )
    .expect("truncated stream can be consumed by fromstream");
    assert_eq!(
        rebuilt,
        [Value::from_json(serde_json::json!(["b"])).unwrap()]
    );
}

#[test]
fn truncate_stream_uses_jq_count_slice_semantics_per_event() {
    let values = evaluate_with_limits(
        "[null,-2,-1,0,0.5,1,1.5,2,\"x\",[],{}] | map(. as $n | {count:$n,result:([try ($n|truncate_stream(([[0,1],\"v\"]))) catch {error:.}])})",
        "null",
        VmLimits::default(),
    )
    .expect("truncate count matrix evaluates");
    assert_eq!(
        values,
        [Value::from_json(serde_json::json!([
            {"count": null, "result": [[[0, 1], "v"]]},
            {"count": -2, "result": [[[0, 1], "v"]]},
            {"count": -1, "result": [[[1], "v"]]},
            {"count": 0, "result": [[[0, 1], "v"]]},
            {"count": 0.5, "result": [[[0, 1], "v"]]},
            {"count": 1, "result": [[[1], "v"]]},
            {"count": 1.5, "result": [[[1], "v"]]},
            {"count": 2, "result": []},
            {"count": "x", "result": []},
            {"count": [], "result": []},
            {"count": {}, "result": []},
        ]))
        .unwrap()]
    );

    let error = evaluate_with_limits(
        "try (false | truncate_stream(([[0,1],\"v\"]))) catch \"caught\"",
        "null",
        VmLimits::default(),
    )
    .expect("boolean count error is catchable");
    assert_eq!(error, [Value::string("caught")]);

    let extra = evaluate_with_limits(
        "0 | truncate_stream(([[true],\"v\",\"extra\"]))",
        "null",
        VmLimits::default(),
    )
    .expect("truncate preserves non-component paths and extra fields");
    assert_eq!(
        extra,
        [Value::from_json(serde_json::json!([[true], "v", "extra"])).unwrap()]
    );

    let unusual_paths = evaluate_with_limits(
        "[{count:-1,event:null},{count:null,event:null},{count:-1,event:[]},{count:null,event:[]},{count:-1,event:[null]},{count:-1,event:[null,1]},{count:-1,event:[[],1]},{count:-1,event:[3,1]},{count:-1,event:[{\"a\":1},1]}] | map(. as $case | {count:$case.count,event:$case.event,result:[try ($case.count | truncate_stream($case.event)) catch \"caught\"]})",
        "null",
        VmLimits::default(),
    )
    .expect("truncate preserves jq's non-array path semantics");
    assert_eq!(
        unusual_paths,
        [Value::from_json(serde_json::json!([
            {"count": -1, "event": null, "result": [[null]]},
            {"count": null, "event": null, "result": [[null]]},
            {"count": -1, "event": [], "result": [[null]]},
            {"count": null, "event": [], "result": [[null]]},
            {"count": -1, "event": [null], "result": [[null]]},
            {"count": -1, "event": [null, 1], "result": [[null, 1]]},
            {"count": -1, "event": [[], 1], "result": [[[], 1]]},
            {"count": -1, "event": [3, 1], "result": ["caught"]},
            {"count": -1, "event": [{"a": 1}, 1], "result": ["caught"]},
        ]))
        .unwrap()]
    );
}

#[test]
fn pull_consumers_observe_cancellation_between_results() {
    let query = "def twice(f): first(f), first(f); twice((1,2,3))";
    let compiled = plan(query);
    let cancellation = Arc::new(AtomicBool::new(false));
    let mut vm = Vm::new(
        &compiled,
        Value::Null,
        VmLimits {
            steps: 128,
            ..VmLimits::default()
        },
    )
    .with_cancellation(Arc::clone(&cancellation));
    assert_eq!(
        vm.next_result().expect("first pull result"),
        Some(Value::from_json(serde_json::json!(1)).unwrap())
    );
    cancellation.store(true, Ordering::Relaxed);
    assert_eq!(vm.next_result(), Err(VmError::Interrupted));
}
