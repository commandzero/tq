//! Public API coverage for jq-compatible grammar precedence and identifiers.

use std::{
    fs,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use tq_core::{ResolveOptions, Value, Vm, VmError, VmLimits, analyze, parse, parse_bytes, resolve};

fn evaluate(query: &str, input: &str) -> Vec<Value> {
    evaluate_result(query, input).expect("query evaluates")
}

fn evaluate_result(query: &str, input: &str) -> Result<Vec<Value>, VmError> {
    evaluate_result_with_limits(query, input, VmLimits::default())
}

fn evaluate_result_with_limits(
    query: &str,
    input: &str,
    limits: VmLimits,
) -> Result<Vec<Value>, VmError> {
    evaluate_result_with_name("<query>", query, input, limits)
}

fn evaluate_result_with_name(
    name: &str,
    query: &str,
    input: &str,
    limits: VmLimits,
) -> Result<Vec<Value>, VmError> {
    let input = Value::from_json(serde_json::from_str(input).expect("valid JSON")).unwrap();
    let resolved = resolve(
        parse_bytes(name, query.as_bytes()).expect("query parses"),
        &ResolveOptions::default(),
    )
    .expect("query resolves");
    let plan = analyze(resolved)
        .compile()
        .expect("query compiles")
        .document_plan();
    let mut vm = Vm::new(&plan, input, limits);
    let mut results = Vec::new();
    while let Some(value) = vm.next_result()? {
        results.push(value);
    }
    Ok(results)
}

#[test]
fn comma_index_generators_emit_selected_elements_in_order() {
    assert_eq!(
        evaluate(".[4,2]", r#"["a","b","c","d","e"]"#),
        [Value::string("e"), Value::string("c")]
    );
}

#[test]
fn quoted_field_identifiers_select_keys_with_special_characters() {
    assert_eq!(
        evaluate(r#"."foo$""#, r#"{"foo$":42}"#),
        [Value::from_json(serde_json::json!(42)).unwrap()]
    );
}

#[test]
fn unquoted_colon_qualified_field_is_rejected_as_invalid_syntax() {
    let error = parse(".foo::bar").expect_err("unquoted qualified field must be rejected");
    assert_eq!(error.code, "TQ-PARSE-FIELD-001");
}

#[test]
fn unknown_qualified_filters_report_a_resolver_namespace_error() {
    let error = resolve(
        parse("missing::filter").expect("qualified name parses"),
        &ResolveOptions::default(),
    )
    .expect_err("unknown qualified filter must fail resolution");
    assert_eq!(error.code, "TQ-RESOLVE-BUILTIN-001");
}

#[test]
fn comma_binds_before_pipe_for_boolean_filters() {
    assert_eq!(
        evaluate("[true, false | not]", "null"),
        [Value::array([Value::Bool(false), Value::Bool(true)])]
    );
}

#[test]
fn comma_pipe_precedence_feeds_each_upstream_result() {
    assert_eq!(
        evaluate(".a, .b | .c", r#"{"a":{"c":1},"b":{"c":2}}"#,),
        [
            Value::from_json(serde_json::json!(1)).unwrap(),
            Value::from_json(serde_json::json!(2)).unwrap(),
        ]
    );
}

#[test]
fn quoted_empty_field_identifiers_remain_accessible() {
    assert_eq!(
        evaluate(r#".foo."""#, r#"{"foo":{"":7}}"#),
        [Value::from_json(serde_json::json!(7)).unwrap()]
    );
}

#[test]
fn quoted_namespace_separator_is_a_literal_field_name() {
    assert_eq!(
        evaluate(r#"."foo::bar""#, r#"{"foo::bar":11}"#),
        [Value::from_json(serde_json::json!(11)).unwrap()]
    );
}

#[test]
fn dynamic_modulemeta_observes_cancellation_before_loading_files() {
    let mut options = ResolveOptions::default();
    options
        .module_roots
        .push(std::env::current_dir().expect("current directory canonicalizes"));
    let resolved = resolve(parse("modulemeta").expect("modulemeta parses"), &options)
        .expect("modulemeta resolves");
    let plan = analyze(resolved)
        .compile()
        .expect("modulemeta compiles")
        .document_plan();
    let cancellation = Arc::new(AtomicBool::new(true));
    let mut vm = Vm::new(&plan, Value::string("cancelled"), VmLimits::default())
        .with_cancellation(cancellation);
    assert!(matches!(vm.next_result(), Err(VmError::Interrupted)));
}

#[test]
fn try_catch_keeps_catch_filter_pipe_and_generator_behavior() {
    assert_eq!(
        evaluate(r#"try error("x") catch "caught", 3"#, "null"),
        [
            Value::string("caught"),
            Value::from_json(serde_json::json!(3)).unwrap()
        ]
    );
    assert_eq!(
        evaluate(r#"try (1,error("x")) catch . | ."#, "null"),
        [
            Value::from_json(serde_json::json!(1)).unwrap(),
            Value::string("x")
        ]
    );
}

#[test]
fn try_catch_does_not_capture_trailing_comma_or_pipe() {
    assert_eq!(
        evaluate("try 1 catch 2,3", "null"),
        [
            Value::from_json(serde_json::json!(1)).unwrap(),
            Value::from_json(serde_json::json!(3)).unwrap()
        ]
    );
    assert_eq!(
        evaluate("try 1 catch 2 | .+1", "null"),
        [Value::from_json(serde_json::json!(2)).unwrap()]
    );
    let error = evaluate_result(r#"try error("a"), error("b")"#, "null")
        .expect_err("the second error is outside try");
    assert!(matches!(
        error,
        VmError::Raised {
            value: Value::String(message),
            ..
        } if message.contains('b')
    ));
}

#[test]
fn try_catch_preserves_typed_error_values_and_empty_results() {
    assert_eq!(
        evaluate("try error(42) catch .", "null"),
        [Value::from_json(serde_json::json!(42)).unwrap()]
    );
    assert_eq!(
        evaluate("try error({value: 42}) catch .", "null"),
        [Value::from_json(serde_json::json!({"value": 42})).unwrap()]
    );
    assert_eq!(
        evaluate("try error catch .", r#"{"value":42}"#),
        [Value::from_json(serde_json::json!({"value": 42})).unwrap()]
    );
    assert_eq!(evaluate("try error(null) catch .", "null"), [Value::Null]);
    assert_eq!(
        evaluate("try error(false) catch .", "null"),
        [Value::Bool(false)]
    );
    assert_eq!(
        evaluate("try error(42) catch . + 1", "null"),
        [Value::from_json(serde_json::json!(43)).unwrap()]
    );
    assert_eq!(
        evaluate(
            "try (try error(42) catch error({inner: .})) catch .",
            "null",
        ),
        [Value::from_json(serde_json::json!({"inner": 42})).unwrap()]
    );
    assert_eq!(
        evaluate("try empty catch .", "null"),
        [] as [tq_core::Value; 0]
    );
    assert_eq!(evaluate("error(42)?", "null"), [] as [tq_core::Value; 0]);
    assert_eq!(evaluate("error(null)?", "null"), [] as [tq_core::Value; 0]);
    assert!(matches!(
        evaluate_result(".foo? | error(\"later\")", r#"{"foo": 1}"#),
        Err(VmError::Raised {
            value: Value::String(message),
            ..
        }) if message.as_ref() == "later"
    ));
}

#[test]
fn try_does_not_convert_resource_errors_into_values() {
    let error = evaluate_result_with_limits(
        "try last(range(0; 1000000)) catch .",
        "null",
        VmLimits {
            steps: 16,
            ..VmLimits::default()
        },
    )
    .expect_err("resource exhaustion must remain an execution error");
    assert!(matches!(
        error,
        VmError::Resource {
            resource: "vm-steps"
        }
    ));
}

#[test]
fn try_does_not_convert_cancellation_into_a_value() {
    let plan = analyze(
        resolve(
            parse("try range(0; 100) catch .").expect("query parses"),
            &ResolveOptions::default(),
        )
        .expect("query resolves"),
    )
    .compile()
    .expect("query compiles")
    .document_plan();
    let cancellation = Arc::new(AtomicBool::new(true));
    let mut vm = Vm::new(&plan, Value::Null, VmLimits::default())
        .with_cancellation(Arc::clone(&cancellation));
    assert!(matches!(vm.next_result(), Err(VmError::Interrupted)));
}

#[test]
fn try_and_optional_preserve_label_break_control_flow() {
    assert_eq!(
        evaluate("label $out | try (1, break $out, 2) catch . , 3", "null"),
        [
            Value::from_json(serde_json::json!(1)).unwrap(),
            Value::from_json(serde_json::json!({"__jq": 0})).unwrap(),
            Value::from_json(serde_json::json!(3)).unwrap(),
        ]
    );
    assert_eq!(
        evaluate("label $out | (break $out)? , 1", "null"),
        [Value::from_json(serde_json::json!(1)).unwrap()]
    );
    assert_eq!(
        evaluate("try (label $out | 1, break $out, 2) catch .", "null"),
        [Value::from_json(serde_json::json!(1)).unwrap()]
    );
}

#[test]
fn conditionals_preserve_optional_else_and_condition_cardinality() {
    assert_eq!(
        evaluate(
            "[if false then 1 end, if false then 1 elif false then 2 end]",
            r#"{"original":true}"#,
        ),
        [Value::array([
            Value::from_json(serde_json::json!({"original": true})).unwrap(),
            Value::from_json(serde_json::json!({"original": true})).unwrap(),
        ])]
    );
    assert_eq!(
        evaluate(
            "if (false, null, 0, \"\", [], {}) then \"yes\" else \"no\" end",
            "null",
        ),
        [
            Value::string("no"),
            Value::string("no"),
            Value::string("yes"),
            Value::string("yes"),
            Value::string("yes"),
            Value::string("yes"),
        ]
    );
    assert_eq!(
        evaluate("if empty then 1 else 2 end", "null"),
        [] as [tq_core::Value; 0]
    );
    assert_eq!(
        evaluate("[{}, error(empty)]", "null"),
        [Value::array([
            Value::from_json(serde_json::json!({})).unwrap()
        ])]
    );
}

#[test]
fn paths_pick_and_deletion_preserve_recursive_identity_and_order() {
    assert_eq!(
        evaluate("[paths(scalars)]", r#"{"a":[1,2],"b":3}"#),
        [Value::from_json(serde_json::json!([["a", 0], ["a", 1], ["b"]])).unwrap()]
    );
    assert_eq!(
        evaluate(
            "[path(.. | select(type == \"number\"))]",
            r#"{"a":[1,2],"b":3}"#,
        ),
        [Value::from_json(serde_json::json!([["a", 0], ["a", 1], ["b"]])).unwrap()]
    );
    assert_eq!(
        evaluate("pick(.a, .b.c, .x)", r#"{"a":1,"b":{"c":2,"d":3},"e":4}"#,),
        [Value::from_json(serde_json::json!({"a":1,"b":{"c":2},"x":null})).unwrap()]
    );
    assert_eq!(
        evaluate("[path(.[2], .[0], .[0])]", "[1,2,3,4]"),
        [Value::from_json(serde_json::json!([[2], [0], [0]])).unwrap()]
    );
    assert_eq!(
        evaluate("pick(.[2])", "[1,2,3,4]"),
        [Value::from_json(serde_json::json!([null, null, 3])).unwrap()]
    );
    assert_eq!(
        evaluate("pick(.[2], .[0])", "[1,2,3,4]"),
        [Value::from_json(serde_json::json!([1, null, 3])).unwrap()]
    );
    assert_eq!(
        evaluate("pick(.[2], .[0], .[0])", "[1,2,3,4]"),
        [Value::from_json(serde_json::json!([1, null, 3])).unwrap()]
    );
    assert_eq!(
        evaluate("del(.foo)", r#"{"foo":42,"bar":9001,"baz":42}"#),
        [Value::from_json(serde_json::json!({"bar":9001,"baz":42})).unwrap()]
    );
    assert_eq!(
        evaluate("del(.[1,2])", r#"["foo","bar","baz"]"#),
        [Value::from_json(serde_json::json!(["foo"])).unwrap()]
    );
    assert_eq!(
        evaluate("delpaths([[\"a\",\"b\"]])", r#"{"a":{"b":1},"x":{"y":2}}"#),
        [Value::from_json(serde_json::json!({"a":{},"x":{"y":2}})).unwrap()]
    );
}

#[test]
fn path_mutators_preserve_generator_scope_duplicates_and_negative_indices() {
    assert_eq!(
        evaluate("[paths(true, true)]", "[1]"),
        [Value::from_json(serde_json::json!([[0], [0]])).unwrap()]
    );
    assert_eq!(
        evaluate("del(.[1], .[1])", "[0,1,2,3]"),
        [Value::from_json(serde_json::json!([0, 2, 3])).unwrap()]
    );
    assert_eq!(
        evaluate("delpaths([[1], [1]])", "[0,1,2,3]"),
        [Value::from_json(serde_json::json!([0, 2, 3])).unwrap()]
    );
    assert_eq!(
        evaluate("path(.a[.b])", r#"{"a":[10,20],"b":1}"#),
        [Value::from_json(serde_json::json!(["a", 1])).unwrap()]
    );
    assert_eq!(
        evaluate("del(.[-1])", "[0,1,2]"),
        [Value::from_json(serde_json::json!([0, 1])).unwrap()]
    );
    assert!(matches!(
        evaluate_result_with_limits(
            "delpaths([[\"a\",\"b\",\"c\"]])",
            r#"{"a":{"b":{"c":1}}}"#,
            VmLimits {
                path_stack: 2,
                ..VmLimits::default()
            },
        ),
        Err(VmError::Resource {
            resource: "path-stack"
        })
    ));
    assert!(matches!(
        evaluate_result_with_limits(
            "[paths]",
            "[[[0]]]",
            VmLimits {
                path_stack: 2,
                ..VmLimits::default()
            },
        ),
        Err(VmError::Resource {
            resource: "path-stack"
        })
    ));
}

#[test]
fn assignments_pair_plain_rhs_and_keep_update_empty_semantics() {
    assert_eq!(
        evaluate("(.a,.b) = (1,2)", r#"{"a":0,"b":0}"#),
        [
            Value::from_json(serde_json::json!({"a": 1, "b": 1})).unwrap(),
            Value::from_json(serde_json::json!({"a": 2, "b": 2})).unwrap(),
        ]
    );
    assert_eq!(
        evaluate(".a |= range(2)", r#"{"a":9}"#),
        [Value::from_json(serde_json::json!({"a": 0})).unwrap()]
    );
    assert_eq!(
        evaluate(".a += range(2)", r#"{"a":0}"#),
        [
            Value::from_json(serde_json::json!({"a": 0})).unwrap(),
            Value::from_json(serde_json::json!({"a": 1})).unwrap(),
        ]
    );
    assert_eq!(
        evaluate(".a |= empty", r#"{"a":9,"b":2}"#),
        [Value::from_json(serde_json::json!({"b": 2})).unwrap()]
    );
    assert_eq!(
        evaluate("(.a,.b) = range(3)", "null"),
        [
            Value::from_json(serde_json::json!({"a": 0, "b": 0})).unwrap(),
            Value::from_json(serde_json::json!({"a": 1, "b": 1})).unwrap(),
            Value::from_json(serde_json::json!({"a": 2, "b": 2})).unwrap(),
        ]
    );
    assert_eq!(
        evaluate("(.a,.b) |= range(3)", "null"),
        [Value::from_json(serde_json::json!({"a": 0, "b": 0})).unwrap()]
    );
    assert_eq!(
        evaluate(
            ".posts[].comments |= . + [\"this is great\"]",
            r#"{"posts":[{"title":"First","author":"stedolan","comments":[]},{"title":"Second","author":"other","comments":["existing"]}]}"#,
        ),
        [Value::from_json(serde_json::json!({
            "posts": [
                {"title": "First", "author": "stedolan", "comments": ["this is great"]},
                {"title": "Second", "author": "other", "comments": ["existing", "this is great"]}
            ]
        }))
        .unwrap()]
    );
    assert_eq!(
        evaluate(
            "(.posts[] | select(.author == \"stedolan\") | .comments) |= . + [\"terrible.\"]",
            r#"{"posts":[{"title":"First","author":"stedolan","comments":[]},{"title":"Second","author":"other","comments":[]}] }"#,
        ),
        [Value::from_json(serde_json::json!({
            "posts": [
                {"title": "First", "author": "stedolan", "comments": ["terrible."]},
                {"title": "Second", "author": "other", "comments": []}
            ]
        }))
        .unwrap()]
    );
    assert_eq!(
        evaluate(".a,.b=0", "null"),
        [
            Value::Null,
            Value::from_json(serde_json::json!({"b": 0})).unwrap()
        ]
    );
    assert_eq!(
        evaluate("(.a,.b)=0", "null"),
        [Value::from_json(serde_json::json!({"a": 0, "b": 0})).unwrap()]
    );
    assert_eq!(evaluate(".[] |= empty", "[1,2,3]"), [Value::array([])]);
    assert_eq!(
        evaluate("(.[] | select(. > 1)) |= empty", "[1,2,3]"),
        [Value::from_json(serde_json::json!([1])).unwrap()]
    );
    assert_eq!(
        evaluate("(.[] | select(. > 1)) += 1", "[1,2,3]"),
        [Value::from_json(serde_json::json!([1, 3, 4])).unwrap()]
    );
    assert_eq!(
        evaluate("{a:{b:{c:1}}} | (.a.b|=3), .", "null"),
        [
            Value::from_json(serde_json::json!({"a":{"b":3}})).unwrap(),
            Value::from_json(serde_json::json!({"a":{"b":{"c":1}}})).unwrap(),
        ]
    );
    assert_eq!(
        evaluate(".a.b = 5", r#"{"z":1,"a":{"b":2,"c":3},"tail":4}"#,),
        [Value::from_json(serde_json::json!({
            "z": 1,
            "a": {"b": 5, "c": 3},
            "tail": 4
        }))
        .unwrap()]
    );
}

#[test]
fn object_values_accept_unparenthesized_pipes() {
    assert_eq!(
        evaluate("{a: .foo | .bar}", r#"{"foo":{"bar":9}}"#),
        [Value::from_json(serde_json::json!({"a": 9})).unwrap()]
    );
}

#[test]
fn nested_array_and_object_patterns_bind_values() {
    assert_eq!(
        evaluate(". as [$a, {c: $c}] | $a + $c", r#"[2, {"c": 4}]"#),
        [Value::from_json(serde_json::json!(6)).unwrap()]
    );
}

#[test]
fn manual_nested_pattern_example_preserves_order_and_cardinality() {
    assert_eq!(
        evaluate(
            ". as [$a, $b, {c: $c}] | $a + $b + $c",
            r#"[2, 3, {"c": 4, "d": 5}]"#,
        ),
        [Value::from_json(serde_json::json!(9)).unwrap()]
    );
}

#[test]
fn variable_object_shorthand_uses_lexical_binding() {
    assert_eq!(
        evaluate(".foo as $foo | {$foo}", r#"{"foo":42}"#),
        [Value::from_json(serde_json::json!({"foo": 42})).unwrap()]
    );
}

#[test]
fn variable_object_explicit_key_uses_key_value() {
    assert_eq!(
        evaluate(
            r#""f o o" as $foo | "b a r" as $bar | {$foo, $bar:$foo}"#,
            "null",
        ),
        [Value::from_json(serde_json::json!({
            "foo": "f o o",
            "b a r": "f o o"
        }))
        .unwrap()],
    );
}

#[test]
fn location_magic_expands_to_source_file_and_line() {
    let error = evaluate_result_with_name(
        "<top-level>",
        r#"error("\($__loc__)")"#,
        "null",
        VmLimits::default(),
    )
    .expect_err("location interpolation raises the expanded location");
    assert!(
        matches!(
            &error,
            VmError::Raised { value, .. }
            if *value == Value::string(r#"{"file":"<top-level>","line":1}"#)
        ),
        "actual error: {error:?}"
    );
}

#[test]
fn location_magic_counts_multiline_query_lines() {
    let error = evaluate_result_with_name(
        "<top-level>",
        "\nerror(\"\\($__loc__)\")",
        "null",
        VmLimits::default(),
    )
    .expect_err("location interpolation raises the expanded location");
    assert!(matches!(
        error,
        VmError::Raised { value, .. }
            if value == Value::string(r#"{"file":"<top-level>","line":2}"#)
    ));
}

#[test]
fn data_import_accepts_variable_namespace_alias() {
    parse(r#"import "data" as $data; $data::data"#)
        .expect("JSON data imports use a variable namespace alias");
}

#[test]
fn data_import_loads_json_into_the_variable_namespace() {
    let root = std::env::temp_dir().join(format!("tq-manual-data-module-{}", std::process::id()));
    fs::create_dir_all(&root).expect("data module test directory creates");
    fs::write(root.join("data.json"), br#"{"name":"manual","count":3}"#)
        .expect("data module writes");
    let resolved = resolve(
        parse(r#"import "data" as $data; $data::data"#).expect("data import parses"),
        &ResolveOptions {
            module_roots: vec![root.clone()],
            ..ResolveOptions::default()
        },
    )
    .expect("data module resolves");
    let plan = analyze(resolved)
        .compile()
        .expect("data module compiles")
        .document_plan();
    let mut vm = Vm::new(&plan, Value::Null, VmLimits::default());
    assert_eq!(
        vm.next_result().expect("data module evaluates"),
        Some(
            Value::from_json(serde_json::json!([{
                "name": "manual",
                "count": 3
            }]))
            .unwrap()
        )
    );
    fs::remove_dir_all(root).expect("data module test directory removes");
}

#[test]
fn missing_pattern_members_bind_null() {
    assert_eq!(
        evaluate(". as [$a, $b] | [$a, $b]", "[1]"),
        [Value::array([
            Value::from_json(serde_json::json!(1)).unwrap(),
            Value::Null
        ])]
    );
}

#[test]
fn nested_binding_shadows_only_its_lexical_body() {
    assert_eq!(
        evaluate("1 as $x | ((2 as $x | [$x]), [$x])", "null"),
        [
            Value::array([Value::from_json(serde_json::json!(2)).unwrap()]),
            Value::array([Value::from_json(serde_json::json!(1)).unwrap()]),
        ]
    );
}

#[test]
fn invalid_pattern_input_is_a_runtime_error() {
    let error = evaluate_result(". as [$value] | $value", "1")
        .expect_err("array destructuring a scalar must fail");
    assert!(
        matches!(error, VmError::Runtime { message } if message.contains("array destructuring"))
    );
}

#[test]
fn wide_patterns_consume_the_configured_vm_work_budget() {
    let names = (0..32)
        .map(|index| format!("$value{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let query = format!(". as [{names}] | $value31");
    let error = evaluate_result_with_limits(
        &query,
        "null",
        VmLimits {
            steps: 6,
            ..VmLimits::default()
        },
    )
    .expect_err("wide pattern should consume the configured work budget");
    assert!(matches!(
        error,
        VmError::Resource {
            resource: "vm-steps"
        }
    ));
}

#[test]
fn deeply_nested_patterns_are_rejected_before_parser_stack_exhaustion() {
    let mut pattern = "$value".to_owned();
    for _ in 0..300 {
        pattern = format!("[{pattern}]");
    }
    let error = parse(&format!(". as {pattern} | null"))
        .expect_err("excessively nested patterns must be bounded");
    assert_eq!(error.code, "TQ-RESOURCE-BIND-PATTERN-001");
}

#[test]
fn missing_object_pattern_fields_bind_null() {
    assert_eq!(
        evaluate(
            ". as {$present, $missing} | {$present, $missing}",
            r#"{"present":7}"#
        ),
        [Value::from_json(serde_json::json!({"present": 7, "missing": null})).unwrap()]
    );
}

#[test]
fn missing_nested_pattern_members_bind_null_recursively() {
    assert_eq!(
        evaluate(". as {outer: {inner: $value}} | $value", "{}"),
        [Value::Null]
    );
}

#[test]
fn destructuring_alternatives_select_the_matching_pattern() {
    assert_eq!(
        evaluate(". as [$value] ?// {value: $value} | $value", "[1]"),
        [Value::from_json(serde_json::json!(1)).unwrap()]
    );
    assert_eq!(
        evaluate(
            ". as [$value] ?// {value: $value} | $value",
            "{\"value\": 2}"
        ),
        [Value::from_json(serde_json::json!(2)).unwrap()]
    );
}

#[test]
fn destructuring_alternatives_match_nested_shapes_and_default_missing_variables() {
    assert_eq!(
        evaluate(
            ".[] as {$a, $b, c: {$d, $e}} ?// {$a, $b, c: [{$d, $e}]} | {$a, $b, $d, $e}",
            r#"[{"a":1,"b":2,"c":{"d":3,"e":4}},{"a":1,"b":2,"c":[{"d":3,"e":4}]}]"#,
        ),
        [
            Value::from_json(serde_json::json!({"a":1,"b":2,"d":3,"e":4})).unwrap(),
            Value::from_json(serde_json::json!({"a":1,"b":2,"d":3,"e":4})).unwrap(),
        ]
    );
    assert_eq!(
        evaluate(
            ".[] as {$a, $b, c: {$d}} ?// {$a, $b, c: [{$e}]} | {$a, $b, $d, $e}",
            r#"[{"a":1,"b":2,"c":{"d":3,"e":4}},{"a":1,"b":2,"c":[{"d":3,"e":4}]}]"#,
        ),
        [
            Value::from_json(serde_json::json!({"a":1,"b":2,"d":3,"e":null})).unwrap(),
            Value::from_json(serde_json::json!({"a":1,"b":2,"d":null,"e":4})).unwrap(),
        ]
    );
}

#[test]
fn destructuring_alternatives_retry_after_body_error() {
    assert_eq!(
        evaluate(
            ".[] as [$a] ?// [$b] | if $a != null then error(\"err: \\($a)\") else {$a,$b} end",
            "[[3]]",
        ),
        [Value::from_json(serde_json::json!({"a": null, "b": 3})).unwrap()]
    );
    let error = evaluate_result(
        ". as [$value] ?// {value: $value} | error(\"later body\")",
        "{\"value\": 2}",
    )
    .expect_err("the final alternative body error must be preserved");
    assert!(matches!(
        error,
        VmError::Raised {
            value: Value::String(message),
            ..
        } if message.contains("later body")
    ));
    let error = evaluate_result(". as [$value] ?// {value: $value} | $value", "true")
        .expect_err("all destructuring alternatives should report the final pattern error");
    assert!(
        matches!(error, VmError::Runtime { message } if message.contains("object destructuring"))
    );
}

#[test]
fn destructuring_alternatives_propagate_resource_and_cancellation_errors() {
    let error = evaluate_result_with_limits(
        ". as [$value] ?// {value: $value} | (0, range(0; 100))",
        "[1]",
        VmLimits {
            steps: 12,
            ..VmLimits::default()
        },
    )
    .expect_err("alternative retry must not turn a VM limit into a pattern mismatch");
    assert!(matches!(
        error,
        VmError::Resource {
            resource: "vm-steps"
        }
    ));

    let plan = analyze(
        resolve(
            parse(". as [$value] ?// {value: $value} | range(0; 100)").expect("query parses"),
            &ResolveOptions::default(),
        )
        .expect("query resolves"),
    )
    .compile()
    .expect("query compiles")
    .document_plan();
    let cancellation = Arc::new(AtomicBool::new(false));
    let mut vm = Vm::new(
        &plan,
        Value::array([Value::from_json(serde_json::json!(1)).unwrap()]),
        VmLimits::default(),
    )
    .with_cancellation(Arc::clone(&cancellation));
    assert_eq!(
        vm.next_result().expect("first result evaluates"),
        Some(Value::from_json(serde_json::json!(0)).unwrap())
    );
    cancellation.store(true, Ordering::Relaxed);
    assert!(matches!(vm.next_result(), Err(VmError::Interrupted)));
}

#[test]
fn destructuring_alternatives_preserve_manual_nested_resource_examples() {
    let input = r#"{"resources":[{"id":1,"kind":"widget","events":{"action":"create","user_id":1,"ts":13}},{"id":2,"kind":"widget","events":[{"action":"create","user_id":1,"ts":14},{"action":"destroy","user_id":1,"ts":15}]}]}"#;
    assert_eq!(
        evaluate(
            ".resources[] as {$id, $kind, events: {$user_id, $ts}} ?// {$id, $kind, events: [{$user_id, $ts}]} | {$user_id, $kind, $id, $ts}",
            input,
        ),
        [
            Value::from_json(serde_json::json!({"user_id":1,"kind":"widget","id":1,"ts":13}))
                .unwrap(),
            Value::from_json(serde_json::json!({"user_id":1,"kind":"widget","id":2,"ts":14}))
                .unwrap(),
        ]
    );
    assert_eq!(
        evaluate(
            ".resources[] as {$id, $kind, events: {$user_id, $ts}} ?// {$id, $kind, events: [{$first_user_id, $first_ts}]} | {$user_id, $first_user_id, $kind, $id, $ts, $first_ts}",
            input,
        ),
        [
            Value::from_json(serde_json::json!({"user_id":1,"first_user_id":null,"kind":"widget","id":1,"ts":13,"first_ts":null})).unwrap(),
            Value::from_json(serde_json::json!({"user_id":null,"first_user_id":null,"kind":"widget","id":2,"ts":null,"first_ts":null})).unwrap(),
        ]
    );
}

#[test]
fn filter_arguments_capture_their_calling_environment() {
    assert_eq!(
        evaluate(
            "def addvalue(f): . + [f]; map(addvalue(.[0]))",
            "[[1,2],[10,20]]",
        ),
        [Value::array([
            Value::array([
                Value::from_json(serde_json::json!(1)).unwrap(),
                Value::from_json(serde_json::json!(2)).unwrap(),
                Value::from_json(serde_json::json!(1)).unwrap(),
            ]),
            Value::array([
                Value::from_json(serde_json::json!(10)).unwrap(),
                Value::from_json(serde_json::json!(20)).unwrap(),
                Value::from_json(serde_json::json!(10)).unwrap(),
            ]),
        ])]
    );
    assert_eq!(
        evaluate(
            "def addvalue(f): f as $x | map(. + $x); addvalue(.[0])",
            "[[1,2],[10,20]]",
        ),
        [Value::array([
            Value::array([
                Value::from_json(serde_json::json!(1)).unwrap(),
                Value::from_json(serde_json::json!(2)).unwrap(),
                Value::from_json(serde_json::json!(1)).unwrap(),
                Value::from_json(serde_json::json!(2)).unwrap(),
            ]),
            Value::array([
                Value::from_json(serde_json::json!(10)).unwrap(),
                Value::from_json(serde_json::json!(20)).unwrap(),
                Value::from_json(serde_json::json!(1)).unwrap(),
                Value::from_json(serde_json::json!(2)).unwrap(),
            ]),
        ])]
    );
    assert_eq!(
        evaluate("def foo(f): f|f; 5|foo(.*2)", "null"),
        [Value::from_json(serde_json::json!(20)).unwrap()]
    );
    assert_eq!(
        evaluate(
            "def addvalue(f): f as $f | map(. + $f); addvalue(.[])",
            "[1,2]"
        ),
        [
            Value::array([
                Value::from_json(serde_json::json!(2)).unwrap(),
                Value::from_json(serde_json::json!(3)).unwrap(),
            ]),
            Value::array([
                Value::from_json(serde_json::json!(3)).unwrap(),
                Value::from_json(serde_json::json!(4)).unwrap(),
            ]),
        ]
    );
}

#[test]
fn binding_closures_preserve_post_author_lookup_scope() {
    let input = r#"{"posts":[{"title":"First post","author":"anon"},{"title":"A well-written article","author":"person1"}],"realnames":{"anon":"Anonymous Coward","person1":"Person McPherson"}}"#;
    assert_eq!(
        evaluate(
            ".realnames as $names | .posts[] | {title, author: $names[.author]}",
            input,
        ),
        [
            Value::from_json(serde_json::json!({"title":"First post","author":"Anonymous Coward"}))
                .unwrap(),
            Value::from_json(
                serde_json::json!({"title":"A well-written article","author":"Person McPherson"})
            )
            .unwrap(),
        ]
    );
    let error = parse("(.realnames as $names | .posts[]) | {title, author: $names[.author]}")
        .expect("query parses");
    let error = resolve(error, &ResolveOptions::default())
        .expect_err("bindings outside their lexical body must be rejected");
    assert_eq!(error.code, "TQ-RESOLVE-VARIABLE-001");
}

#[test]
fn reduce_and_foreach_preserve_destructured_accumulator_scope() {
    assert_eq!(
        evaluate(
            "reduce .[] as [$i, $j] (0; . + $i * $j)",
            "[[1,2],[3,4],[5,6]]"
        ),
        [Value::from_json(serde_json::json!(44)).unwrap()]
    );
    assert_eq!(
        evaluate(
            "reduce .[] as {$x, $y} (null; .x += $x | .y += [$y])",
            "[{\"x\":\"a\",\"y\":1},{\"x\":\"b\",\"y\":2},{\"x\":\"c\",\"y\":3}]",
        ),
        [Value::from_json(serde_json::json!({"x":"abc","y":[1,2,3]})).unwrap()]
    );
    assert_eq!(
        evaluate("foreach .[] as $item (0; . + $item)", "[1,2,3,4,5]"),
        [
            Value::from_json(serde_json::json!(1)).unwrap(),
            Value::from_json(serde_json::json!(3)).unwrap(),
            Value::from_json(serde_json::json!(6)).unwrap(),
            Value::from_json(serde_json::json!(10)).unwrap(),
            Value::from_json(serde_json::json!(15)).unwrap(),
        ]
    );
    assert_eq!(
        evaluate(
            "foreach .[] as $item (0; . + $item; [$item, . * 2])",
            "[1,2,3,4,5]",
        ),
        [
            Value::from_json(serde_json::json!([1, 2])).unwrap(),
            Value::from_json(serde_json::json!([2, 6])).unwrap(),
            Value::from_json(serde_json::json!([3, 12])).unwrap(),
            Value::from_json(serde_json::json!([4, 20])).unwrap(),
            Value::from_json(serde_json::json!([5, 30])).unwrap(),
        ]
    );
    assert_eq!(
        evaluate(
            "foreach .[] as $item (0; . + 1; {index: ., $item})",
            "[\"foo\",\"bar\",\"baz\"]",
        ),
        [
            Value::from_json(serde_json::json!({"index":1,"item":"foo"})).unwrap(),
            Value::from_json(serde_json::json!({"index":2,"item":"bar"})).unwrap(),
            Value::from_json(serde_json::json!({"index":3,"item":"baz"})).unwrap(),
        ]
    );
}

#[test]
fn reduce_and_foreach_preserve_generator_cardinality() {
    assert_eq!(
        evaluate("reduce .[] as $x (0; empty)", "[1,2]"),
        [Value::Null]
    );
    assert_eq!(
        evaluate("foreach .[] as $x (0; empty)", "[1,2]"),
        [] as [tq_core::Value; 0]
    );
    assert_eq!(
        evaluate("reduce .[] as $x (0; (. + $x), (. + $x + 100))", "[1,2]",),
        [Value::from_json(serde_json::json!(203)).unwrap()]
    );
    assert_eq!(
        evaluate("foreach .[] as $x (0; (. + $x), (. + $x + 100))", "[1,2]",),
        [
            Value::from_json(serde_json::json!(1)).unwrap(),
            Value::from_json(serde_json::json!(101)).unwrap(),
            Value::from_json(serde_json::json!(103)).unwrap(),
            Value::from_json(serde_json::json!(203)).unwrap(),
        ]
    );
    assert_eq!(
        evaluate("reduce empty as $x (0,10; . + $x)", "[1,2]"),
        [
            Value::from_json(serde_json::json!(0)).unwrap(),
            Value::from_json(serde_json::json!(10)).unwrap(),
        ]
    );
    assert_eq!(
        evaluate("foreach empty as $x (0,10; . + $x)", "[1,2]"),
        [] as [tq_core::Value; 0]
    );
    assert_eq!(
        evaluate("reduce .[] as $x (empty; . + $x)", "[1,2]"),
        [] as [tq_core::Value; 0]
    );
    assert_eq!(
        evaluate("foreach .[] as $x (empty; . + $x)", "[1,2]"),
        [] as [tq_core::Value; 0]
    );
    assert_eq!(
        evaluate("reduce .[] as $x (empty,0; . + $x)", "[1,2]"),
        [Value::from_json(serde_json::json!(3)).unwrap()]
    );
    assert_eq!(
        evaluate("foreach .[] as $x (empty,0; . + $x)", "[1,2]"),
        [
            Value::from_json(serde_json::json!(1)).unwrap(),
            Value::from_json(serde_json::json!(3)).unwrap(),
        ]
    );
    for query in [
        "reduce .[] as $x (0,error(\"later\"); . + $x)",
        "foreach .[] as $x (0,error(\"later\"); . + $x)",
    ] {
        assert!(matches!(
            evaluate_result(query, "[1,2]"),
            Err(VmError::Raised {
                value: Value::String(message),
                ..
            }) if message.contains("later")
        ));
    }
}

#[test]
fn pull_consumers_match_manual_generator_examples() {
    assert_eq!(
        evaluate("def take(f): first(f); take(1, 2)", "null"),
        [Value::from_json(serde_json::json!(1)).unwrap()]
    );
    assert_eq!(
        evaluate("def take(f): last(f); take(1, 2)", "null"),
        [Value::from_json(serde_json::json!(2)).unwrap()]
    );
    assert_eq!(
        evaluate("def take(f): isempty(f); take(empty)", "null"),
        [Value::Bool(true)]
    );
    assert_eq!(
        evaluate("def take(f): first(f); take(empty)", "null"),
        [] as [tq_core::Value; 0]
    );
    assert_eq!(
        evaluate("def take(f): last(f); take(empty)", "null"),
        [] as [tq_core::Value; 0]
    );
    assert_eq!(
        evaluate("[first(range(.)), last(range(.)), nth(5; range(.))]", "10",),
        [Value::array([
            Value::from_json(serde_json::json!(0)).unwrap(),
            Value::from_json(serde_json::json!(9)).unwrap(),
            Value::from_json(serde_json::json!(5)).unwrap(),
        ])]
    );
    assert_eq!(
        evaluate("[first(empty), last(empty), nth(5; empty)]", "null"),
        [Value::array([])],
    );
    assert_eq!(
        evaluate("[range(.)] | [first, last, nth(5)]", "10"),
        [Value::array([
            Value::from_json(serde_json::json!(0)).unwrap(),
            Value::from_json(serde_json::json!(9)).unwrap(),
            Value::from_json(serde_json::json!(5)).unwrap(),
        ])]
    );
    assert_eq!(
        evaluate("[skip(3; .[])]", "[0,1,2,3,4,5,6,7,8,9]"),
        [Value::array(
            (3..10)
                .map(|value| Value::from_json(serde_json::json!(value)).unwrap())
                .collect::<Vec<_>>(),
        )],
    );
    assert_eq!(evaluate("isempty(empty)", "null"), [Value::Bool(true)]);
    assert_eq!(evaluate("isempty(.[])", "[]"), [Value::Bool(true)]);
    assert_eq!(evaluate("isempty(.[])", "[1,2,3]"), [Value::Bool(false)]);
}

#[test]
fn pull_consumers_stop_before_later_errors_and_bound_work() {
    assert_eq!(
        evaluate(
            "[first(1, error(\"later\")), nth(0; 1, error(\"later\")), isempty(1, error(\"later\"))]",
            "null",
        ),
        [Value::array([
            Value::from_json(serde_json::json!(1)).unwrap(),
            Value::from_json(serde_json::json!(1)).unwrap(),
            Value::Bool(false),
        ])]
    );
    assert!(matches!(
        evaluate_result("last(1, error(\"later\"))", "null"),
        Err(VmError::Raised {
            value: Value::String(message),
            ..
        }) if message.contains("later")
    ));
    assert_eq!(
        evaluate_result_with_limits(
            "first(range(0; 1000000))",
            "null",
            VmLimits {
                steps: 16,
                ..VmLimits::default()
            },
        )
        .expect("first must terminate without consuming the tail"),
        [Value::from_json(serde_json::json!(0)).unwrap()]
    );
    assert!(matches!(
        evaluate_result_with_limits(
            "last(range(0; 1000000))",
            "null",
            VmLimits {
                steps: 16,
                ..VmLimits::default()
            },
        ),
        Err(VmError::Resource {
            resource: "vm-steps"
        })
    ));
    assert_eq!(
        evaluate_result_with_limits(
            "first(nth((0, range(0; 1000000)); range(0; 10)))",
            "null",
            VmLimits {
                steps: 32,
                ..VmLimits::default()
            },
        )
        .expect("first must stop before the count generator tail"),
        [Value::from_json(serde_json::json!(0)).unwrap()]
    );
}

#[test]
fn pull_consumer_arities_preserve_direct_index_and_empty_behavior() {
    assert_eq!(
        evaluate("[first, last, nth(0)]", "null"),
        [Value::array([Value::Null, Value::Null, Value::Null])]
    );
    assert_eq!(
        evaluate("[first, last, nth(0)]", "[]"),
        [Value::array([Value::Null, Value::Null, Value::Null])]
    );
    assert_eq!(
        evaluate("[first, last, nth(-1), nth(5)]", "[0,1,2]"),
        [Value::array([
            Value::from_json(serde_json::json!(0)).unwrap(),
            Value::from_json(serde_json::json!(2)).unwrap(),
            Value::from_json(serde_json::json!(2)).unwrap(),
            Value::Null,
        ])]
    );
    for query in ["first", "last", "nth(0)"] {
        let error = evaluate_result(query, "{}").expect_err("object index must fail");
        assert!(matches!(error, VmError::Runtime { message } if message.contains("index")));
    }
    assert_eq!(evaluate("first(empty)", "null"), [] as [tq_core::Value; 0]);
    assert_eq!(evaluate("last(empty)", "null"), [] as [tq_core::Value; 0]);
    assert_eq!(evaluate("nth(0; empty)", "null"), [] as [tq_core::Value; 0]);
    assert!(matches!(
        evaluate_result("nth(-1; .[])", "[1]"),
        Err(VmError::Runtime { message }) if message.contains("non-negative")
    ));
}

#[test]
fn pull_consumer_count_generators_preserve_multiplicity() {
    assert_eq!(
        evaluate("[nth((0,1); (10,20))]", "null"),
        [Value::array([
            Value::from_json(serde_json::json!(10)).unwrap(),
            Value::from_json(serde_json::json!(20)).unwrap(),
        ])]
    );
    assert_eq!(
        evaluate("[skip((1,2); .[])]", "[1,2,3]"),
        [Value::array([
            Value::from_json(serde_json::json!(2)).unwrap(),
            Value::from_json(serde_json::json!(3)).unwrap(),
            Value::from_json(serde_json::json!(3)).unwrap(),
        ])]
    );
}

#[test]
fn pull_consumer_counts_truncate_fractional_indices_like_jq() {
    assert_eq!(
        evaluate("nth(0.5; range(4))", "null"),
        [Value::from_json(serde_json::json!(0)).unwrap()]
    );
    assert_eq!(
        evaluate("[skip(0.5; range(4))]", "null"),
        [Value::array(
            (0..4)
                .map(|value| Value::from_json(serde_json::json!(value)).unwrap())
                .collect::<Vec<_>>(),
        )]
    );
    assert_eq!(
        evaluate("nth(0.5)", "[10,20]"),
        [Value::from_json(serde_json::json!(10)).unwrap()]
    );
}
