use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use indexmap::IndexMap;

use super::{
    StableSortPipeline,
    ordered::{PARALLEL_REDUCTION_THRESHOLD, PARALLEL_SORT_THRESHOLD, extrema, sort_by_cached_key},
    stable_sort_values,
};
use crate::{
    InputCursor, InputValue, ResolveOptions, Value, Vm, VmLimits, VmObservations, analyze,
    bytecode::Operation, parse, resolve, vm::EffectState,
};

fn run(query: &str, input: &str) -> Vec<Result<Value, String>> {
    run_with_variables(query, input, BTreeMap::new())
}

fn run_with_variables(
    query: &str,
    input: &str,
    variables: BTreeMap<Arc<str>, Value>,
) -> Vec<Result<Value, String>> {
    let query = parse(query).unwrap();
    let query = resolve(
        query,
        &ResolveOptions {
            variables: variables.keys().cloned().collect::<BTreeSet<_>>(),
            ..ResolveOptions::default()
        },
    )
    .unwrap();
    let plan = analyze(query).compile().unwrap().document_plan();
    let input: Value = serde_json::from_str(input).unwrap();
    let mut vm = Vm::new_with_variables(&plan, input, VmLimits::default(), variables);
    let mut values = Vec::new();
    loop {
        match vm.next_result() {
            Ok(Some(value)) => values.push(Ok(value)),
            Ok(None) => break,
            Err(error) => {
                values.push(Err(error.to_string()));
                break;
            }
        }
    }
    values
}

fn vm_with_input_context(query: &str, identity: &str, line_number: u64) -> Vm {
    let query = parse(query).unwrap();
    let query = resolve(query, &ResolveOptions::default()).unwrap();
    let plan = analyze(query).compile().unwrap().document_plan();
    let cursor = InputCursor::from_input_values(vec![InputValue {
        value: serde_json::from_str("null").unwrap(),
        identity: Arc::from(identity),
        line_number,
    }]);
    let input = cursor.next_value().unwrap().unwrap();

    Vm::new(&plan, input, VmLimits::default()).with_input_cursor(cursor)
}

fn first_error_with_limits(query: &str, input: &str, limits: VmLimits) -> crate::VmError {
    let plan = analyze(resolve(parse(query).unwrap(), &ResolveOptions::default()).unwrap())
        .compile()
        .unwrap()
        .document_plan();
    let input: Value = serde_json::from_str(input).unwrap();
    Vm::new(&plan, input, limits)
        .next_result()
        .expect_err("query should fail before producing a result")
}

fn json(values: Vec<Result<Value, String>>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| value.map_or_else(|error| format!("error:{error}"), |value| value.to_string()))
        .collect()
}

#[test]
fn lexical_labels_prune_owned_work_and_preserve_outer_work() {
    assert_eq!(
        json(run("label $out | 1, break $out, 2", "null")),
        vec!["1"]
    );
    assert_eq!(
        json(run(
            "label $x | (1, (label $x | 2, break $x, 3), 4)",
            "null"
        )),
        vec!["1", "2", "4"]
    );
    assert_eq!(
        json(run("label $a | (label $b | 1, break $a, 2), 3", "null")),
        vec!["1"]
    );
}

#[test]
fn label_break_observes_try_function_and_fold_boundaries() {
    assert_eq!(
        json(run(
            "label $out | try (1, break $out, 2) catch \"caught\", 3",
            "null"
        )),
        vec!["1", r#""caught""#, "3"]
    );
    assert_eq!(
        json(run(
            "try (label $out | 1, break $out, 2) catch \"caught\"",
            "null"
        )),
        vec!["1"]
    );
    assert_eq!(
        json(run("label $x | try break $x catch .", "null")),
        vec![r#"{"__jq":0}"#]
    );
    assert_eq!(
        json(run("label $x | def f: 1, break $x, 2; f, 3", "null")),
        vec!["1"]
    );
    assert_eq!(
        json(run(
            "label $out | foreach .[] as $item (null; $item; if . == false then break $out else . end)",
            "[1,2,false,3,null]"
        )),
        vec!["1", "2"]
    );
}

#[test]
fn recursive_builtins_match_jq_order_and_cardinality() {
    assert_eq!(
        json(run("[recurse(.[]?)]", r#"{"a":[1]}"#)),
        vec![r#"[{"a":[1]},[1],1]"#]
    );
    assert_eq!(
        json(run("recurse", r#"{"a":[1]}"#)),
        vec![r#"{"a":[1]}"#, "[1]", "1"]
    );
    assert_eq!(json(run("recurse(.+1; . < 4)", "2")), vec!["2", "3"]);
    assert_eq!(
        json(run("recurse(if . < 2 then .+1, .+10 else empty end)", "0")),
        vec!["0", "1", "2", "11", "10"]
    );
}

#[test]
fn walk_is_post_order_and_preserves_jq_callback_cardinality() {
    assert_eq!(
        json(run(
            "walk(if type == \"number\" then . + 1 else . end)",
            r#"{"a":[1,2]}"#
        )),
        vec![r#"{"a":[2,3]}"#]
    );
    assert_eq!(json(run("walk(., .)", "[1]")), vec!["[1,1]", "[1,1]"]);
    assert_eq!(
        json(run(
            "walk(if type == \"number\" then ., .+10 else . end)",
            r#"{"a":1,"b":2}"#
        )),
        vec![r#"{"a":1,"b":2}"#]
    );
}

#[test]
fn recursive_control_flow_stops_and_obeys_limits_and_cancellation() {
    assert_eq!(
        json(run("limit(1; recurse(error(\"late\")))", "null")),
        vec!["null"]
    );
    assert_eq!(
        json(run(
            "limit(1; label $out | 1, error(\"late\"), break $out)",
            "null"
        )),
        vec!["1"]
    );

    for (limits, resource_name) in [
        (
            VmLimits {
                steps: 1,
                ..VmLimits::default()
            },
            "vm-steps",
        ),
        (
            VmLimits {
                call_stack: 1,
                ..VmLimits::default()
            },
            "call-stack",
        ),
    ] {
        let error = first_error_with_limits("[label $out | 1, 2]", "null", limits);
        assert_eq!(
            error,
            crate::VmError::Resource {
                resource: resource_name
            }
        );
    }

    for query in ["[recurse(.[]?)]", "walk(.)"] {
        let error = first_error_with_limits(
            query,
            "[[[0]]]",
            VmLimits {
                path_stack: 2,
                ..VmLimits::default()
            },
        );
        assert_eq!(
            error,
            crate::VmError::Resource {
                resource: "path-stack"
            }
        );
    }

    let plan = analyze(
        resolve(
            parse("label $out | recurse(.[]?)").unwrap(),
            &ResolveOptions::default(),
        )
        .unwrap(),
    )
    .compile()
    .unwrap()
    .document_plan();
    let cancellation = Arc::new(AtomicBool::new(true));
    let mut vm = Vm::new(
        &plan,
        serde_json::from_str("[[[0]]]").unwrap(),
        VmLimits::default(),
    )
    .with_cancellation(cancellation);
    assert_eq!(vm.next_result().unwrap_err(), crate::VmError::Interrupted);
}

#[test]
fn default_recurse_pulls_wide_children_on_demand() {
    let plan = analyze(
        resolve(
            parse("limit(2; recurse)").unwrap(),
            &ResolveOptions::default(),
        )
        .unwrap(),
    )
    .compile()
    .unwrap()
    .document_plan();
    let source = format!(
        "[{}]",
        (0..128)
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    let input: Value = serde_json::from_str(&source).unwrap();
    let mut vm = Vm::new(
        &plan,
        input.clone(),
        VmLimits {
            fork_stack: 8,
            ..VmLimits::default()
        },
    );

    assert_eq!(vm.next_result().unwrap(), Some(input));
    assert_eq!(
        vm.next_result().unwrap(),
        Some(serde_json::from_str("0").unwrap())
    );
    assert_eq!(vm.next_result().unwrap(), None);
}

#[test]
fn regex_builtins_match_jq_unicode_captures_scans_splits_and_substitution() {
    assert_eq!(
        json(run(r#"match("a"; "g")"#, r#""éaéa""#)),
        vec![
            r#"{"offset":1,"length":1,"string":"a","captures":[]}"#,
            r#"{"offset":3,"length":1,"string":"a","captures":[]}"#,
        ]
    );
    assert_eq!(
        json(run(r#"capture("(?<x>a)(?<y>z)?")"#, r#""a""#)),
        vec![r#"{"x":"a","y":null}"#]
    );
    assert_eq!(
        json(run(r#"scan("([a-z]+)([0-9]+)")"#, r#""ab12cd34""#)),
        vec![r#"["ab","12"]"#, r#"["cd","34"]"#]
    );
    assert_eq!(
        json(run(
            r#"sub("(?<x>[a-z]+)(?<n>[0-9]+)"; "\(.n)-\(.x)")"#,
            r#""abc123""#,
        )),
        vec![r#""123-abc""#]
    );
    assert_eq!(json(run(r#"test("(?=a)")"#, r#""a""#)), vec!["true"]);
}

#[test]
fn regex_pattern_input_and_compiled_program_limits_are_resources() {
    for (query, limits, expected) in [
        (
            r#"test("ab")"#,
            VmLimits {
                regex_pattern_bytes: 1,
                ..VmLimits::default()
            },
            "regex-pattern-bytes",
        ),
        (
            r#"test("ab")"#,
            VmLimits {
                regex_input_bytes: 1,
                ..VmLimits::default()
            },
            "regex-input-bytes",
        ),
        (
            r#"test("(?:[A-Za-z0-9_]{1,100}){100}")"#,
            VmLimits {
                regex_compiled_bytes: 1,
                ..VmLimits::default()
            },
            "regex-compiled-bytes",
        ),
    ] {
        let error = first_error_with_limits(query, r#""ab""#, limits);
        assert_eq!(error, crate::VmError::Resource { resource: expected });
    }
}

#[test]
fn utc_date_builtins_round_trip_with_stable_arrays() {
    assert_eq!(
        json(run("fromdateiso8601", r#""2015-03-05T23:51:47Z""#)),
        vec!["1425599507"]
    );
    assert_eq!(
        json(run("gmtime", "1425599507")),
        vec!["[2015,2,5,23,51,47,4,63]"]
    );
    assert_eq!(
        json(run(
            r#"gmtime | strftime("%Y-%m-%dT%H:%M:%SZ")"#,
            "1425599507",
        )),
        vec![r#""2015-03-05T23:51:47Z""#]
    );
    assert!(matches!(
        first_error_with_limits("todateiso8601", "253402300800", VmLimits::default()),
        crate::VmError::NumericRange { .. }
    ));
}

#[test]
fn regex_date_platform_release_host_contract_covers_utc_boundaries() {
    for timestamp in ["0000-01-01T00:00:00Z", "9999-12-30T22:00:00Z"] {
        let input = serde_json::to_string(timestamp).unwrap();
        assert_eq!(
            json(run("fromdateiso8601 | todateiso8601", &input)),
            [input]
        );
    }
}

#[test]
fn ambient_builtins_require_reserved_policy_admission_without_echoing_values() {
    assert!(json(run("env", "null"))[0].contains("capability policy"));
    assert!(json(run("input_filename", "null"))[0].contains("capability policy"));
    let mut variables = BTreeMap::new();
    variables.insert(
        Arc::from(super::AMBIENT_ENVIRONMENT),
        Value::object(IndexMap::from([(
            Arc::from("SECRET"),
            Value::string("redacted"),
        )])),
    );
    variables.insert(Arc::from(super::AMBIENT_PLATFORM), Value::Bool(true));
    variables.insert(
        Arc::from(super::INPUT_FILENAME),
        Value::string("fixture.json"),
    );
    assert_eq!(
        json(run_with_variables("env | type", "null", variables.clone())),
        vec![r#""object""#]
    );
    assert_eq!(
        json(run_with_variables("input_filename", "null", variables)),
        vec![r#""fixture.json""#]
    );
}

#[test]
fn env_variable_uses_the_ambient_snapshot_and_policy_error() {
    let denied = json(run("$ENV", "null"));
    assert!(
        denied[0].contains("$ENV requires environment access permitted by capability policy"),
        "{denied:?}"
    );
    assert!(!denied[0].contains("__tq_ambient_environment"));

    let mut variables = BTreeMap::new();
    variables.insert(
        Arc::from(super::AMBIENT_ENVIRONMENT),
        Value::object(IndexMap::from([(
            Arc::from("KNOWN_SENTINEL"),
            Value::string("present"),
        )])),
    );
    assert_eq!(
        json(run_with_variables("$ENV.KNOWN_SENTINEL", "null", variables)),
        vec![r#""present""#]
    );
}

#[test]
fn env_variable_is_consistent_across_all_evaluator_routes() {
    let variables = BTreeMap::from([(
        Arc::from(super::AMBIENT_ENVIRONMENT),
        Value::object(IndexMap::from([(
            Arc::from("KNOWN_SENTINEL"),
            Value::string("present"),
        )])),
    )]);
    let compile = |query: &str| {
        let query = resolve(
            parse(query).unwrap(),
            &ResolveOptions {
                variables: variables.keys().cloned().collect(),
                ..ResolveOptions::default()
            },
        )
        .unwrap();
        analyze(query).compile().unwrap()
    };

    let kernel = compile("$ENV");
    let kernel_bytecode = kernel.bytecode();
    assert!(matches!(
        kernel_bytecode.instructions()[kernel_bytecode.root() as usize].operation,
        Operation::Variable(_)
    ));

    let evaluate_fallback = |variables: &BTreeMap<Arc<str>, Value>| {
        let stop = AtomicBool::new(false);
        let effects = Arc::new(EffectState::new_for_test());
        let evaluator = super::Evaluator {
            bytecode: kernel_bytecode,
            limits: VmLimits::default(),
            observations: Cell::new(VmObservations::default()),
            cancellation: None,
            stop: &stop,
            input_cursor: None,
            effects,
        };
        let mut outcomes = Vec::new();
        evaluator.emit_node(
            kernel_bytecode.root(),
            &Value::Null,
            variables,
            0,
            &mut |outcome| {
                outcomes.push(outcome);
                true
            },
        );
        outcomes
    };

    assert_eq!(
        evaluate_fallback(&variables),
        vec![Ok(Value::object(IndexMap::from([(
            Arc::from("KNOWN_SENTINEL"),
            Value::string("present"),
        )])))]
    );
    assert!(matches!(
        evaluate_fallback(&BTreeMap::new()).as_slice(),
        [Err(crate::VmError::CapabilityDenied { message })]
            if message.contains("$ENV requires environment access permitted by capability policy")
    ));

    let managed = compile("[$ENV.KNOWN_SENTINEL]");
    assert!(managed.bytecode().managed_tree_execution());
    assert!(!matches!(
        managed.bytecode().instructions()[managed.bytecode().root() as usize].operation,
        Operation::Variable(_)
    ));

    assert_eq!(
        json(run_with_variables("$ENV", "null", variables.clone())),
        [r#"{"KNOWN_SENTINEL":"present"}"#]
    );
    assert_eq!(
        json(run_with_variables(
            "[$ENV.KNOWN_SENTINEL]",
            "null",
            variables.clone()
        )),
        [r#"["present"]"#]
    );
    assert_eq!(
        json(run_with_variables(
            "$ENV.KNOWN_SENTINEL | explode | length",
            "null",
            variables
        )),
        ["7"]
    );

    for query in [
        "$ENV",
        "[$ENV.KNOWN_SENTINEL]",
        "$ENV.KNOWN_SENTINEL | explode | length",
    ] {
        let denied = json(run(query, "null"));
        assert!(
            denied[0].contains("capability policy"),
            "{query}: {denied:?}"
        );
    }
}

#[test]
fn special_variables_preserve_location_and_scope_semantics() {
    assert_eq!(
        json(run(
            "def location:\n  $__loc__;\n[$__loc__, location]",
            "null"
        )),
        vec![r#"[{"file":"<query>","line":3},{"file":"<query>","line":2}]"#]
    );
    assert_eq!(json(run("1 as $ENV | $ENV", "null")), vec!["1"]);
    assert_eq!(
        json(run(r#""line=\($__loc__.line)""#, "null")),
        vec![r#""line=1""#]
    );

    let variables = BTreeMap::from([
        (Arc::from("ENV"), Value::string("external")),
        (Arc::from("__loc__"), Value::string("external")),
    ]);
    let values = json(run_with_variables(
        "[$ENV | type, $__loc__.file]",
        "null",
        variables,
    ));
    assert!(values[0].contains("capability policy"), "{values:?}");
}

#[test]
fn input_line_number_uses_supplied_context_without_platform_admission() {
    let mut variables = BTreeMap::new();
    variables.insert(Arc::from(super::AMBIENT_PLATFORM), Value::Bool(false));
    variables.insert(
        Arc::from(super::INPUT_LINE_NUMBER),
        serde_json::from_str("7").unwrap(),
    );

    assert_eq!(
        json(run_with_variables("input_line_number", "null", variables)),
        ["7"]
    );
}

#[test]
fn input_line_number_prefers_active_cursor_context_without_platform_admission() {
    let mut vm = vm_with_input_context("input_line_number", "fixture.json", 9);

    assert_eq!(vm.next_result().unwrap().unwrap().to_string(), "9");
}

#[test]
fn input_filename_requires_platform_admission_with_active_cursor() {
    let mut vm = vm_with_input_context("input_filename", "fixture.json", 9);

    let error = vm.next_result().unwrap_err().to_string();
    assert!(error.contains("capability policy"));
    assert!(!error.contains("fixture.json"));
}

#[test]
fn input_line_number_without_context_reports_metadata_unavailable() {
    let error = json(run("input_line_number", "null"));

    assert!(error[0].contains("metadata is unavailable"));
    assert!(!error[0].contains("capability policy"));
}

#[test]
fn wave_one_navigation_and_generators_compose() {
    assert_eq!(json(run(".a.b", r#"{"a":{"b":2}}"#)), ["2"]);
    assert_eq!(json(run(".[-1]", "[1,2,3]")), ["3"]);
    assert_eq!(json(run(".[1:3]", "[0,1,2,3]")), ["[1,2]"]);
    assert_eq!(json(run(".[] | .x", r#"[{"x":1},{"x":2}]"#)), ["1", "2"]);
    assert_eq!(json(run(".a, .missing?", r#"{"a":1}"#)), ["1", "null"]);
    assert_eq!(run("empty", "null"), Vec::<Result<Value, String>>::new());
}

#[test]
fn wave_two_construction_control_and_operators_compose() {
    assert_eq!(json(run("[.[] | . * 2]", "[1,2,3]")), ["[2,4,6]"]);
    assert_eq!(
        json(run(
            "{name, ok: (.age >= 18)}",
            r#"{"name":"Ada","age":30}"#
        )),
        [r#"{"name":"Ada","ok":true}"#]
    );
    assert_eq!(
        json(run("if . then \"yes\" else \"no\" end", "0")),
        [r#""yes""#]
    );
    assert_eq!(json(run("null // false // 7", "null")), ["7"]);
    assert_eq!(json(run(r#""a" + "b""#, "null")), [r#""ab""#]);
    assert_eq!(
        json(run(r#"{"a":1}+{"a":2,"b":3}"#, "null")),
        [r#"{"a":2,"b":3}"#]
    );
}

#[test]
fn object_multiplication_recursively_merges_and_preserves_key_order() {
    assert_eq!(
        json(run(
            r#"{"a":{"b":1},"replace":{"left":true},"keep":0} * {"a":{"c":2},"replace":4,"new":3}"#,
            "null"
        )),
        [r#"{"a":{"b":1,"c":2},"replace":4,"keep":0,"new":3}"#]
    );
}

#[test]
fn object_multiplication_uses_the_right_value_for_non_object_conflicts() {
    assert_eq!(
        json(run(
            r#"[{"a":1} * {"a":{"nested":true}}, {"a":{"nested":true}} * {"a":1}, 6 * 7]"#,
            "null"
        )),
        [r#"[{"a":{"nested":true}},{"a":1},42]"#]
    );
}

#[test]
fn multiplication_rejects_mixed_object_and_number_operands() {
    assert_eq!(
        json(run(r#"{"a":1} * 2"#, "null")),
        ["error:runtime error: cannot multiply object and number"]
    );
}

#[test]
fn multiplication_update_recursively_merges_objects() {
    assert_eq!(
        json(run(
            r#".settings *= {"display":{"theme":"dark"}}"#,
            r#"{"settings":{"display":{"density":"compact"},"cache":true}}"#
        )),
        [r#"{"settings":{"display":{"density":"compact","theme":"dark"},"cache":true}}"#]
    );
}

#[test]
fn wave_three_variables_builtins_errors_and_updates_compose() {
    let variables = BTreeMap::from([(
        Arc::from("n"),
        Value::from_json(serde_json::json!(3)).unwrap(),
    )]);
    assert_eq!(json(run_with_variables("$n + 1", "null", variables)), ["4"]);
    assert_eq!(
        json(run(
            "map(.x) | sort | unique",
            r#"[{"x":2},{"x":1},{"x":2}]"#
        )),
        ["[1,2]"]
    );
    assert_eq!(json(run("type, length", r"[1,2]")), [r#""array""#, "2"]);
    assert_eq!(json(run("tostring | length", "1e4096")), ["7"]);
    assert_eq!(json(run(r#""\(.)""#, "1e4096")), [r#""1E+4096""#]);
    assert_eq!(
        json(run("try error(\"boom\") catch .", "null")),
        [r#""boom""#]
    );
    let error_after = json(run("1, error(\"later\")", "null"));
    assert_eq!(error_after[0], "1");
    assert!(error_after[1].starts_with("error:runtime error: later"));
    assert_eq!(json(run(".a.b = 2", "{}")), [r#"{"a":{"b":2}}"#]);
    assert_eq!(
        json(run("(.a, .b) += 10", r#"{"a":1,"b":2,"c":[3]}"#)),
        [r#"{"a":11,"b":12,"c":[3]}"#]
    );
}

#[test]
fn collection_and_scalar_utility_builtins_match_jq() {
    assert_eq!(
        json(run("to_entries", r#"{"z":1,"a":2}"#)),
        [r#"[{"key":"z","value":1},{"key":"a","value":2}]"#]
    );
    assert_eq!(
        json(run("with_entries(.value += 1)", r#"{"a":1,"b":2}"#)),
        [r#"{"a":2,"b":3}"#]
    );
    assert_eq!(
        json(run(
            "group_by(.k)",
            r#"[{"k":2,"v":"a"},{"k":1},{"k":2,"v":"b"}]"#
        )),
        [r#"[[{"k":1}],[{"k":2,"v":"a"},{"k":2,"v":"b"}]]"#]
    );
    assert_eq!(
        json(run(
            "[min_by(.k), max_by(.k)]",
            r#"[{"k":2},{"k":1},{"k":3}]"#
        )),
        [r#"[{"k":1},{"k":3}]"#]
    );
    assert_eq!(json(run("[limit(2; range(0; 5))]", "null")), ["[0,1]"]);
    assert_eq!(
        json(run("[limit((1,2); range(0; 4))]", "null")),
        ["[0,0,1]"]
    );
    assert_eq!(json(run("[limit(empty; error(\"late\"))]", "null")), ["[]"]);
    assert_eq!(
        json(run("[any(.[]; . > 2), all(.[]; . < 4)]", "[1,2,3]")),
        ["[true,true]"]
    );
    assert_eq!(
        json(run(
            r#"[ltrimstr("pre"), ascii_downcase, explode, (explode | implode)]"#,
            r#""preABCé""#,
        )),
        [r#"["ABCé","preabcé",[112,114,101,65,66,67,233],"preABCé"]"#]
    );
    assert_eq!(json(run("[floor, ceil, fabs]", "-1.5")), ["[-2,-1,1.5]"]);
}

#[test]
fn path_and_stream_builtins_match_jq_order_and_creation() {
    assert_eq!(
        json(run("[paths]", r#"{"a":[10],"z":{}}"#)),
        [r#"[["a"],["a",0],["z"]]"#]
    );
    assert_eq!(
        json(run("path(.a[0]), path(.missing)", r#"{"a":[10]}"#)),
        [r#"["a",0]"#, r#"["missing"]"#]
    );
    assert_eq!(
        json(run(
            r#"[getpath(["a",0]), getpath(["x"]), setpath(["b",1]; 7)]"#,
            r#"{"a":[10]}"#,
        )),
        [r#"[10,null,{"a":[10],"b":[null,7]}]"#]
    );
    assert_eq!(
        json(run(r#"setpath((["a"],["b"]); (1,2))"#, "{}")),
        [r#"{"a":1}"#, r#"{"b":1}"#, r#"{"a":2}"#, r#"{"b":2}"#]
    );
    assert_eq!(
        json(run("tostream", r#"{"a":[1],"z":{}}"#)),
        [
            r#"[["a",0],1]"#,
            r#"[["a",0]]"#,
            r#"[["z"],{}]"#,
            r#"[["z"]]"#,
        ]
    );
    assert_eq!(
        json(run(
            r#"truncate_stream([[0],"a"],[[1,0],"b"],[[1,0]],[[1]])"#,
            "1",
        )),
        ["[[0],\"b\"]", "[[0]]"]
    );
    assert_eq!(
        json(run(
            r#"fromstream(1|truncate_stream([[0],"a"],[[1,0],"b"],[[1,0]],[[1]]))"#,
            "null",
        )),
        [r#"["b"]"#]
    );
    assert_eq!(
        json(run("fromstream((1, [], {}, [1,[2]]) | tostream)", "null",)),
        ["1", "[]", "{}", "[1,[2]]"]
    );
    assert_eq!(
        json(run("fromstream(([1],[2]) | tostream)", "null")),
        ["[1]", "[2]"]
    );
    assert_eq!(
        json(run("fromstream(([[0],1],[[0]],[[1],2],[[1]]))", "null",)),
        ["[1]", "[null,2]"]
    );
    assert_eq!(
        json(run("fromstream(([[0,0],1],[[0,0]]))", "null")),
        [] as [String; 0]
    );
    let prior_then_error = json(run("fromstream((([[],1], error(\"later\"))))", "null"));
    assert_eq!(prior_then_error[0], "1");
    assert!(prior_then_error[1].contains("later"));
    assert_eq!(
        json(run(
            "first(fromstream((([1] | tostream), error(\"later\"))))",
            "null",
        )),
        ["[1]"]
    );
    assert_eq!(
        json(run(
            ". as $dot|fromstream($dot|tostream)|.==$dot",
            "[0,[1,{\"a\":1},{\"b\":2}]]",
        )),
        ["true"]
    );
    assert_eq!(
        run("fromstream([[0],1])", "null"),
        [] as [Result<Value, String>; 0]
    );
}

#[test]
fn keyed_sort_uses_every_value_from_its_filter() {
    assert_eq!(
        json(run(
            "sort_by(.a,.b)",
            r#"[{"a":1,"b":2},{"a":1,"b":1},{"a":0,"b":9}]"#,
        )),
        [r#"[{"a":0,"b":9},{"a":1,"b":1},{"a":1,"b":2}]"#]
    );
    assert_eq!(
        json(run(
            "unique_by(.a,.b)",
            r#"[{"a":1,"b":2,"id":"first"},{"a":1,"b":1},{"a":1,"b":2,"id":"last"}]"#,
        )),
        [r#"[{"a":1,"b":1},{"a":1,"b":2,"id":"first"}]"#]
    );
    assert_eq!(json(run("sort_by(empty)", "[3,1,2]")), ["[3,1,2]"]);
    assert_eq!(json(run("sort_by(.)", "[3,1,2]")), ["[1,2,3]"]);
    assert!(
        json(run(r#"sort_by(.a, error("bad"))"#, r#"[{"a":1}]"#,))[0]
            .starts_with("error:runtime error: bad")
    );
}

#[test]
fn group_by_preserves_equal_key_order() {
    assert_eq!(
        json(run(
            "group_by(.k)",
            r#"[{"k":1,"v":"first"},{"k":1,"v":"second"},{"k":1,"v":"third"}]"#
        )),
        [r#"[[{"k":1,"v":"first"},{"k":1,"v":"second"},{"k":1,"v":"third"}]]"#]
    );
}

#[test]
fn json_text_conversion_matches_jq() {
    assert_eq!(
        json(run("tojson", r#"{"z":9007199254740993,"a":"é"}"#)),
        [r#""{\"z\":9007199254740993,\"a\":\"é\"}""#]
    );
    assert_eq!(
        json(run("fromjson", r#""{\"z\":9007199254740993,\"a\":\"é\"}""#)),
        [r#"{"z":9007199254740993,"a":"é"}"#]
    );
    assert_eq!(
        json(run("fromjson", r#""{\"b\":1,\"a\":2,\"b\":3}""#)),
        [r#"{"b":3,"a":2}"#]
    );
    assert_eq!(json(run("fromjson", r#""NaN""#)), ["null"]);
    assert_eq!(
        json(run("fromjson", r#""Infinity""#)),
        ["1.7976931348623157e+308"]
    );
    assert_eq!(json(run("fromjson", r#""+01.200""#)), ["1.200"]);
    assert_eq!(
        json(run(
            r#"["NaN","Infinity","sNaN","+01.200","1."] | map(tonumber)"#,
            "null",
        )),
        ["[null,1.7976931348623157e+308,null,1.200,1]"]
    );
    assert_eq!(
        json(run(
            r#"[" 1 ","[1]","null","1 2","0x10"] | map(try tonumber catch "bad")"#,
            "null",
        )),
        [r#"["bad","bad","bad","bad","bad"]"#]
    );
    assert!(json(run("fromjson", r#""not json""#))[0].starts_with("error:runtime error:"));
}

#[test]
fn fromjson_applies_json_depth_and_token_limits() {
    let nested = r#""[[1]]""#;
    let error = first_error_with_limits(
        "fromjson",
        nested,
        VmLimits {
            json_depth: 1,
            ..VmLimits::default()
        },
    );
    assert_eq!(error, crate::VmError::Resource { resource: "depth" });

    let error = first_error_with_limits(
        "fromjson",
        r#""12345""#,
        VmLimits {
            json_token_bytes: 4,
            ..VmLimits::default()
        },
    );
    assert_eq!(
        error,
        crate::VmError::Resource {
            resource: "token-bytes"
        }
    );
}

#[test]
fn with_entries_accepts_jq_key_aliases() {
    assert_eq!(
        json(run(
            "with_entries(if .key == \"a\" then {Key: \"x\", Value: .value} else {name: \"y\", value: .value} end)",
            r#"{"a":1,"b":2}"#,
        )),
        [r#"{"x":1,"y":2}"#]
    );
}

#[test]
fn inputs_pull_from_one_shared_cursor() {
    let plan = analyze(
        resolve(
            parse("[., limit(1; inputs)]").unwrap(),
            &ResolveOptions::default(),
        )
        .unwrap(),
    )
    .compile()
    .unwrap()
    .document_plan();
    let cursor = InputCursor::new(vec![
        serde_json::from_str("2").unwrap(),
        serde_json::from_str("3").unwrap(),
    ]);
    let mut first = Vm::new(
        &plan,
        serde_json::from_str("1").unwrap(),
        VmLimits::default(),
    )
    .with_input_cursor(cursor.clone());
    assert_eq!(first.next_result().unwrap().unwrap().to_string(), "[1,2]");
    assert_eq!(cursor.next_value().unwrap().unwrap().to_string(), "3");
    assert!(cursor.next_value().unwrap().is_none());
}

#[test]
fn parallel_collections_preserve_order_and_extrema_ties() {
    let count = PARALLEL_SORT_THRESHOLD + 1;
    let original = (0..count)
        .map(|index| Value::string(format!("{index:05}")))
        .collect::<Vec<_>>();

    let mut values = original.iter().rev().cloned().collect::<Vec<_>>();
    stable_sort_values(&mut values);
    assert_eq!(values, original);

    let mut keyed = original
        .iter()
        .cloned()
        .map(|value| (Value::Null, value))
        .collect::<Vec<_>>();
    sort_by_cached_key(&mut keyed);
    assert!(keyed.iter().map(|(_, value)| value).eq(original.iter()));

    let first_equal = Value::object(IndexMap::from([
        (Arc::from("a"), Value::Null),
        (Arc::from("b"), Value::Null),
    ]));
    let last_equal = Value::object(IndexMap::from([
        (Arc::from("b"), Value::Null),
        (Arc::from("a"), Value::Null),
    ]));
    let mut equal_values = vec![first_equal.clone(); PARALLEL_REDUCTION_THRESHOLD];
    equal_values.push(last_equal.clone());
    let equal_values = Value::array(equal_values);
    let minimum = extrema(&equal_values, false).pop().unwrap().unwrap();
    let maximum = extrema(&equal_values, true).pop().unwrap().unwrap();
    assert!(minimum.shares_node_with(&first_equal));
    assert!(maximum.shares_node_with(&last_equal));
}

#[test]
fn recursive_descent_is_depth_first_and_streams_scalars_in_object_order() {
    assert_eq!(
        json(run("..", r#"{"z":1,"a":[2,{"b":3}]}"#)),
        [
            r#"{"z":1,"a":[2,{"b":3}]}"#,
            "1",
            r#"[2,{"b":3}]"#,
            "2",
            r#"{"b":3}"#,
            "3",
        ]
    );
    assert_eq!(
        json(run(".. | scalars", r#"{"z":1,"a":[2,{"b":3}]}"#)),
        ["1", "2", "3"]
    );
    assert_eq!(json(run("..", "42")), ["42"]);
}

#[test]
fn user_functions_preserve_generator_arguments_scope_and_managed_recursion() {
    assert_eq!(
        json(run("def pair($x; $y): $x, $y; pair((1,2); (3,4))", "null")),
        ["1", "3", "1", "4", "2", "3", "2", "4"]
    );
    assert_eq!(
        json(run("def pair($x; $y): $x, $y; pair(1,2; 3,4)", "null")),
        ["1", "3", "1", "4", "2", "3", "2", "4"]
    );
    assert_eq!(json(run("def twice(f): f | f; twice(. + 1)", "1")), ["3"]);
    assert_eq!(
        json(run("def wrapped(f): [f, (not)]; wrapped((1,2))", "null")),
        ["[1,2,true]"]
    );
    assert_eq!(
        json(run("def fallback: empty // 9; fallback", "null")),
        ["9"]
    );
    assert_eq!(
        json(run(
            "1 as $x | def captured: $x; 2 as $x | captured",
            "null"
        )),
        ["1"]
    );
    assert_eq!(
        json(run(
            "def down($n): if $n == 0 then 0 else $n, down($n - 1) end; down(3)",
            "null"
        )),
        ["3", "2", "1", "0"]
    );

    let plan = analyze(
        resolve(
            parse("def forever: forever; forever").unwrap(),
            &ResolveOptions::default(),
        )
        .unwrap(),
    )
    .compile()
    .unwrap()
    .document_plan();
    let mut limited = Vm::new(
        &plan,
        Value::Null,
        VmLimits {
            call_stack: 8,
            ..VmLimits::default()
        },
    );
    assert!(matches!(
        limited.next_result(),
        Err(crate::VmError::Resource {
            resource: "call-stack"
        })
    ));
}

#[test]
fn user_functions_execute_inside_callback_builtins() {
    let cases = [
        ("def f: . + 1; map(f)", "[1,2]", "[2,3]"),
        ("def f: ., . + 10; map(f)", "[1,2]", "[1,11,2,12]"),
        (
            "def f: . + 1; map_values(f)",
            r#"{"a":1,"b":2}"#,
            r#"{"a":2,"b":3}"#,
        ),
        ("def f: empty; map_values(f)", r#"{"a":1}"#, "{}"),
        ("def f: ., . + 10; map_values(f)", "[1,2]", "[1,2]"),
        (
            "def f: if . == 1 then empty else . end; map_values(f)",
            "[1,2]",
            "[2]",
        ),
        ("def f: true, true; [.[] | select(f)]", "[1,2]", "[1,1,2,2]"),
        (
            r#"def f: "a", "missing"; [has(f)]"#,
            r#"{"a":1}"#,
            "[true,false]",
        ),
        ("def f: [1,2]; 1 | in(f)", "null", "true"),
        (
            "def f: .k; sort_by(f)",
            r#"[{"k":2,"v":"a"},{"k":1,"v":"b"},{"k":2,"v":"c"}]"#,
            r#"[{"k":1,"v":"b"},{"k":2,"v":"a"},{"k":2,"v":"c"}]"#,
        ),
        (
            "def f: .k; unique_by(f)",
            r#"[{"k":2,"v":"a"},{"k":1,"v":"b"},{"k":2,"v":"c"}]"#,
            r#"[{"k":1,"v":"b"},{"k":2,"v":"a"}]"#,
        ),
        ("def f: empty; sort_by(f)", "[3,1,2]", "[3,1,2]"),
        ("def f: ., -.; sort_by(f)", "[3,1,2]", "[1,2,3]"),
        ("1 as $x | def f: . + $x; map(f)", "[1,2]", "[2,3]"),
        ("def f($x): . + $x; map(f(2))", "[1,2]", "[3,4]"),
        ("def apply(f): map(f); apply(. + 1)", "[1,2]", "[2,3]"),
    ];
    for (query, input, expected) in cases {
        assert_eq!(json(run(query, input)), [expected], "query: {query}");
    }
}

#[test]
fn trusted_compilation_accepts_slice_user_function_closure() {
    let compiled = analyze(
        resolve(
            parse("def f: .; .[0:1] | f").unwrap(),
            &ResolveOptions::default(),
        )
        .unwrap(),
    )
    .compile()
    .expect("slice and user call share the managed execution path");
    assert!(compiled.bytecode().managed_tree_execution());
}

#[test]
fn managed_admission_covers_every_ported_callback_builtin() {
    for query in [
        "def f: .; map(f)",
        "def f: .; map_values(f)",
        "def f: true; select(f)",
        r#"def f: "a"; has(f)"#,
        "def f: [1]; 1 | in(f)",
        "def f: .; sort_by(f)",
        "def f: .; unique_by(f)",
    ] {
        let compiled =
            analyze(resolve(parse(query).unwrap(), &ResolveOptions::default()).unwrap()).compile();
        assert!(
            compiled.is_ok(),
            "query should use managed execution: {query}"
        );
    }
}

#[test]
fn callback_user_functions_obey_errors_limits_cancellation_and_early_stop() {
    assert_eq!(
        json(run(
            r#"def f: if . == 2 then error("bad") else . end; map(f)"#,
            "[1,2,3]"
        )),
        ["error:runtime error: bad"]
    );

    assert!(matches!(
        first_error_with_limits(
            "def f: ., .; map(f)",
            "[1,2,3]",
            VmLimits {
                fork_stack: 2,
                ..VmLimits::default()
            }
        ),
        crate::VmError::Resource {
            resource: "fork-stack"
        }
    ));
    assert!(matches!(
        first_error_with_limits(
            "def f: . + 1; map(f)",
            "[1,2,3]",
            VmLimits {
                steps: 2,
                ..VmLimits::default()
            }
        ),
        crate::VmError::Resource {
            resource: "vm-steps"
        }
    ));

    let plan = analyze(
        resolve(
            parse("def down($n): if $n == 0 then 0 else $n, down($n - 1) end; map(down(3))")
                .unwrap(),
            &ResolveOptions::default(),
        )
        .unwrap(),
    )
    .compile()
    .unwrap()
    .document_plan();
    let mut vm = Vm::new(
        &plan,
        serde_json::from_str("[null]").unwrap(),
        VmLimits {
            call_stack: 16,
            ..VmLimits::default()
        },
    );
    assert_eq!(
        vm.next_result().unwrap(),
        Some(Value::array([3, 2, 1, 0].map(|value| {
            Value::Number(crate::Number::parse(&value.to_string()).unwrap())
        })))
    );
    assert!(vm.observations().call_stack_high_water <= 16);
    assert!(vm.observations().fork_stack_high_water <= VmLimits::default().fork_stack);

    let cancellation = Arc::new(AtomicBool::new(true));
    let mut cancelled = Vm::new(&plan, Value::array([Value::Null]), VmLimits::default())
        .with_cancellation(cancellation);
    assert!(matches!(
        cancelled.next_result(),
        Err(crate::VmError::Interrupted)
    ));

    let plan = analyze(
        resolve(
            parse("def f: true, true; .[] | select(f)").unwrap(),
            &ResolveOptions::default(),
        )
        .unwrap(),
    )
    .compile()
    .unwrap()
    .document_plan();
    let mut early = Vm::new(
        &plan,
        Value::array([
            Value::Number(crate::Number::parse("1").unwrap()),
            Value::Number(crate::Number::parse("2").unwrap()),
        ]),
        VmLimits::default(),
    );
    assert!(early.next_result().unwrap().is_some());
    drop(early);
}

#[test]
fn interpolation_matches_jq_conversion_generator_order_nesting_and_errors() {
    assert_eq!(
        json(run(
            r#""v=\(null),\(false),\(1),\("x"),\([1]),\({a:1})""#,
            "null",
        )),
        [r#""v=null,false,1,x,[1],{\"a\":1}""#]
    );
    assert_eq!(
        json(run(r#""x=\(1,2);y=\("a","b")""#, "null")),
        [
            r#""x=1;y=a""#,
            r#""x=2;y=a""#,
            r#""x=1;y=b""#,
            r#""x=2;y=b""#
        ]
    );
    assert_eq!(
        json(run(r#""\(if true then "x\(1,2)" else "z" end)""#, "null")),
        [r#""x1""#, r#""x2""#]
    );
    assert_eq!(json(run(r#""sum=\(1 + 2)""#, "null")), [r#""sum=3""#]);
    assert_eq!(
        json(run(r#""x=\((1,2) + 1);y=\(("a","b") + "!")""#, "null",)),
        [
            r#""x=2;y=a!""#,
            r#""x=3;y=a!""#,
            r#""x=2;y=b!""#,
            r#""x=3;y=b!""#,
        ]
    );
    let partial = json(run(r#""before=\(1,error("boom"),2)""#, "null"));
    assert_eq!(partial[0], r#""before=1""#);
    assert!(partial[1].starts_with("error:runtime error: boom"));
    assert_eq!(
        run(r#""\(empty)""#, "null"),
        Vec::<Result<Value, String>>::new()
    );
}

#[test]
fn format_filters_match_jq_across_generator_and_fallback_paths() {
    assert_eq!(json(run("@base64", r#""hello""#)), [r#""aGVsbG8=""#]);
    assert_eq!(
        json(run("@base64 | .[0:]", r#""hello""#)),
        [r#""aGVsbG8=""#]
    );
    assert_eq!(
        json(run(
            r#"@uri "https://x.test?q=\(.q, .alt)""#,
            r#"{"q":"a b","alt":"<&"}"#,
        )),
        [
            r#""https://x.test?q=a%20b""#,
            r#""https://x.test?q=%3C%26""#,
        ]
    );
    assert_eq!(
        run(r#"@uri "x=\(empty)""#, "null"),
        [] as [Result<Value, String>; 0]
    );
    assert_eq!(
        json(run("@csv, @tsv, @sh", r#"["a b",1,null,true]"#)),
        [
            r#""\"a b\",1,,true""#,
            r#""a b\t1\t\ttrue""#,
            r#""'a b' 1 null true""#,
        ]
    );
    assert!(json(run("@base64d", r#""not base64""#))[0].starts_with("error:runtime error:"));

    let plan = analyze(resolve(parse("@uri").unwrap(), &ResolveOptions::default()).unwrap())
        .compile()
        .unwrap()
        .document_plan();
    let mut limited = Vm::new(
        &plan,
        Value::string(" "),
        VmLimits {
            output_bytes: 2,
            ..VmLimits::default()
        },
    );
    assert!(matches!(
        limited.next_result(),
        Err(crate::VmError::Resource {
            resource: "output-bytes"
        })
    ));
}

#[test]
fn recursive_cursor_obeys_depth_work_and_cancellation_after_partial_output() {
    let plan = analyze(resolve(parse("..").unwrap(), &ResolveOptions::default()).unwrap())
        .compile()
        .unwrap()
        .document_plan();
    let mut input = Value::Null;
    for _ in 0..128 {
        input = Value::array([input]);
    }
    let mut depth_limited = Vm::new(
        &plan,
        input.clone(),
        VmLimits {
            path_stack: 16,
            ..VmLimits::default()
        },
    );
    for _ in 0..16 {
        assert!(depth_limited.next_result().unwrap().is_some());
    }
    assert!(matches!(
        depth_limited.next_result(),
        Err(crate::VmError::Resource {
            resource: "path-stack"
        })
    ));

    let mut work_limited = Vm::new(
        &plan,
        input.clone(),
        VmLimits {
            steps: 2,
            ..VmLimits::default()
        },
    );
    assert!(work_limited.next_result().unwrap().is_some());
    assert!(matches!(
        work_limited.next_result(),
        Err(crate::VmError::Resource {
            resource: "vm-steps"
        })
    ));

    let cancellation = Arc::new(AtomicBool::new(false));
    let mut cancelled =
        Vm::new(&plan, input, VmLimits::default()).with_cancellation(Arc::clone(&cancellation));
    assert!(cancelled.next_result().unwrap().is_some());
    cancellation.store(true, Ordering::Relaxed);
    assert!(matches!(
        cancelled.next_result(),
        Err(crate::VmError::Interrupted)
    ));
}

#[test]
fn interpolation_generator_explosion_is_bounded_by_managed_forks() {
    let plan = analyze(
        resolve(
            parse(r#""\(1,2,3)-\("a","b")""#).unwrap(),
            &ResolveOptions::default(),
        )
        .unwrap(),
    )
    .compile()
    .unwrap()
    .document_plan();
    let mut limited = Vm::new(
        &plan,
        Value::Null,
        VmLimits {
            fork_stack: 1,
            ..VmLimits::default()
        },
    );
    assert!(matches!(
        limited.next_result(),
        Err(crate::VmError::Resource {
            resource: "fork-stack"
        })
    ));

    let fallback_plan = analyze(
        resolve(
            parse(r#""\(range(0;3) + 0)""#).unwrap(),
            &ResolveOptions::default(),
        )
        .unwrap(),
    )
    .compile()
    .unwrap()
    .document_plan();
    let mut fallback_limited = Vm::new(
        &fallback_plan,
        Value::Null,
        VmLimits {
            fork_stack: 1,
            ..VmLimits::default()
        },
    );
    assert!(matches!(
        fallback_limited.next_result(),
        Err(crate::VmError::Resource {
            resource: "fork-stack"
        })
    ));

    let output_plan = analyze(
        resolve(
            parse(r#""prefix=\(.)""#).unwrap(),
            &ResolveOptions::default(),
        )
        .unwrap(),
    )
    .compile()
    .unwrap()
    .document_plan();
    let mut output_limited = Vm::new(
        &output_plan,
        Value::string("value"),
        VmLimits {
            output_bytes: 8,
            ..VmLimits::default()
        },
    );
    assert!(matches!(
        output_limited.next_result(),
        Err(crate::VmError::Resource {
            resource: "output-bytes"
        })
    ));

    let wide = Value::array(vec![Value::string("0123456789"); 100_000]);
    let mut serialization_limited = Vm::new(
        &output_plan,
        wide,
        VmLimits {
            output_bytes: 16,
            ..VmLimits::default()
        },
    );
    assert!(matches!(
        serialization_limited.next_result(),
        Err(crate::VmError::Resource {
            resource: "output-bytes"
        })
    ));
}

#[test]
fn stable_sort_pipeline_bounds_runs_and_merges_deterministically() {
    let mut pipeline = StableSortPipeline::new(2, 256, 2, 512);
    for value in [3, 1, 2, 1, 3, 2] {
        pipeline
            .push(serde_json::from_str(&value.to_string()).unwrap(), 32)
            .unwrap();
    }
    let (values, observations) = pipeline.finish().unwrap();
    assert_eq!(
        values,
        [1, 1, 2, 2, 3, 3]
            .map(|value| serde_json::from_str(&value.to_string()).unwrap())
            .to_vec()
    );
    assert_eq!(observations.batches, 3);
    assert!(observations.in_flight_batches <= 2);
    assert!(observations.in_flight_bytes <= 512);
}

#[test]
fn stable_sort_pipeline_cancels_with_workers_active() {
    let cancellation = Arc::new(AtomicBool::new(false));
    let mut pipeline =
        StableSortPipeline::new(1, 64, 2, 128).with_cancellation(Arc::clone(&cancellation));
    pipeline
        .push(serde_json::from_str("2").unwrap(), 32)
        .unwrap();
    cancellation.store(true, Ordering::Relaxed);
    assert_eq!(pipeline.finish().unwrap_err(), "interrupted");
}

#[test]
fn stable_sort_pipeline_preserves_equal_object_order_across_runs() {
    let first: Value = serde_json::from_str(r#"{"a":1,"b":2}"#).unwrap();
    let second: Value = serde_json::from_str(r#"{"b":2,"a":1}"#).unwrap();
    assert_eq!(first.cmp(&second), std::cmp::Ordering::Equal);
    assert_ne!(first.to_string(), second.to_string());

    let run = || {
        let mut pipeline = StableSortPipeline::new(1, 256, 2, 512);
        pipeline.push(first.clone(), 64).unwrap();
        pipeline.push(second.clone(), 64).unwrap();
        pipeline
            .finish()
            .unwrap()
            .0
            .into_iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
    };
    let expected = vec![first.to_string(), second.to_string()];
    assert_eq!(run(), expected);
    assert_eq!(run(), expected);
}
