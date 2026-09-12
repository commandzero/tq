//! Managed-regex coverage through user definitions and captured arguments.

use tq_core::{ResolveOptions, Value, Vm, VmError, VmLimits, analyze, parse, resolve};

fn evaluate(query: &str, input: &str) -> Vec<Value> {
    evaluate_with_limits(query, input, VmLimits::default()).expect("query evaluates")
}

fn evaluate_with_limits(query: &str, input: &str, limits: VmLimits) -> Result<Vec<Value>, VmError> {
    let input = Value::from_json(serde_json::from_str(input).expect("valid JSON")).unwrap();
    let resolved = resolve(
        parse(query).expect("query parses"),
        &ResolveOptions::default(),
    )
    .expect("query resolves");
    let plan = analyze(resolved)
        .compile()
        .expect("query compiles")
        .document_plan();
    let mut vm = Vm::new(&plan, input, limits);
    let mut values = Vec::new();
    while let Some(value) = vm.next_result()? {
        values.push(value);
    }
    Ok(values)
}

fn json_value(value: serde_json::Value) -> Value {
    Value::from_json(value).expect("valid JSON value")
}

#[test]
fn called_defs_cover_test_match_and_capture_value_arguments() {
    let values = evaluate(
        r#"
        def t(pattern; flags): test(pattern; flags);
        def m(pattern; flags): [match(pattern; flags)];
        def c(pattern; flags): capture(pattern; flags);
        [t("foo"; "i"), m("foo"; "g"), c("(?<x>foo)"; "i")]
        "#,
        r#""FOO foo""#,
    );
    assert_eq!(
        values,
        [json_value(serde_json::json!([
            true,
            [{
                "offset": 4,
                "length": 3,
                "string": "foo",
                "captures": [],
            }],
            { "x": "FOO" },
        ]))]
    );
}

#[test]
fn called_defs_cover_scan_split_and_splits_value_arguments() {
    let values = evaluate(
        r#"
        def sc(pattern; flags): [scan(pattern; flags)];
        def sp(pattern; flags): split(pattern; flags);
        def sps(pattern; flags): [splits(pattern; flags)];
        [sc("(.)"; "g"), sp(","; null), sps("a"; "g")]
        "#,
        r#""a,b""#,
    );
    assert_eq!(
        values,
        [json_value(serde_json::json!([
            [["a"], [","], ["b"]],
            ["a", "b"],
            ["", ",b"],
        ]))]
    );
}

#[test]
fn replacement_filter_scope_zip_and_empty_branches_match_jq() {
    let values = evaluate(
        r#"
        def sb(pattern; flags; replacement): sub(pattern; replacement; flags);
        def gs($prefix; pattern; flags; replacement):
          gsub(pattern; ($prefix + replacement); flags);
        [
          sb("(?<d>[0-9])"; "g";
            (if .d == "1" then "a", "b" else "x" end)),
          gs("q"; "(?<d>[0-9])"; "g";
            (if .d == "1" then "a", "b" else "x" end))
        ]
        "#,
        r#""12""#,
    );
    assert_eq!(
        values,
        [json_value(serde_json::json!(["ax", "b", "qaqx", "qb"]))]
    );

    let empty_values = evaluate(
        r#"
        def empty_sub: sub("a"; empty);
        def empty_gsub: gsub("a"; empty);
        [empty_sub, empty_gsub]
        "#,
        r#""a""#,
    );
    assert_eq!(empty_values, [json_value(serde_json::json!(["a", "a"]))]);
}

#[test]
fn regex_pattern_and_flag_generators_keep_cartesian_order_in_defs() {
    let values = evaluate(
        r#"
        def t(pattern; flags): test(pattern; flags);
        [t(("a", "b"); ("g", "i"))]
        "#,
        r#""A""#,
    );
    assert_eq!(
        values,
        [json_value(serde_json::json!([false, false, true, false]))]
    );
}

#[test]
fn called_regex_errors_are_catchable_for_pattern_flag_and_input() {
    let values = evaluate(
        r#"
        def pattern_error($pattern): test($pattern);
        def flag_error($flags): test("x"; $flags);
        [try pattern_error("[") catch "pattern", try flag_error("z") catch "flag"]
        "#,
        r#""x""#,
    );
    assert_eq!(values, [json_value(serde_json::json!(["pattern", "flag"]))]);

    let input_error = evaluate(
        r#"def pattern_error($pattern): test($pattern); try pattern_error("x") catch "input""#,
        "null",
    );
    assert_eq!(input_error, [Value::string("input")]);
}

#[test]
fn replacement_branch_and_output_limits_propagate_through_called_defs() {
    let branch_error = evaluate_with_limits(
        r#"
        def expand($pattern; replacement): gsub($pattern; replacement);
        expand("p"; ("x", "y"))
        "#,
        r#""p""#,
        VmLimits {
            regex_replacement_limit: 1,
            ..VmLimits::default()
        },
    )
    .expect_err("replacement branches must retain their configured bound");
    assert_eq!(
        branch_error,
        VmError::Resource {
            resource: "regex-replacement-count"
        }
    );

    let output_error = evaluate_with_limits(
        r#"
        def expand($pattern; replacement): gsub($pattern; replacement);
        expand("p"; "xxxxxxxx")
        "#,
        r#""p""#,
        VmLimits {
            output_bytes: 4,
            ..VmLimits::default()
        },
    )
    .expect_err("replacement output must retain its configured bound");
    assert_eq!(
        output_error,
        VmError::Resource {
            resource: "output-bytes"
        }
    );
}

#[test]
fn replacement_stream_completes_before_first_sub_result_and_types_are_catchable() {
    let late = evaluate_with_limits(
        r#"
        def replace(pattern; replacement): sub(pattern; replacement);
        first(replace("a"; ("x", error("late"))))
        "#,
        r#""a""#,
        VmLimits::default(),
    )
    .expect_err("sub must finish its replacement stream before yielding output");
    assert!(matches!(late, VmError::Raised { message, .. } if message.contains("late")));

    let wrong_type = evaluate(
        r#"
        def replace(pattern; replacement): sub(pattern; replacement);
        try replace("a"; 1) catch "caught"
        "#,
        r#""a""#,
    );
    assert_eq!(wrong_type, [Value::string("caught")]);
}

#[test]
fn called_first_splits_is_pull_driven_and_split_copy_is_bounded() {
    let large_input = format!("{}z{}", "a".repeat(256), "a".repeat(256));
    let input = serde_json::to_string(&large_input).expect("valid JSON input");
    let discard_error = evaluate_with_limits(
        r#"
        def discard($pattern): splits($pattern) | empty;
        discard("z")
        "#,
        &input,
        VmLimits {
            output_bytes: 16,
            ..VmLimits::default()
        },
    )
    .expect_err("discarded split pieces still require bounded materialization");
    assert_eq!(
        discard_error,
        VmError::Resource {
            resource: "output-bytes"
        }
    );

    let adequate = evaluate_with_limits(
        r#"
        def discard($pattern): splits($pattern) | empty;
        discard("z")
        "#,
        &input,
        VmLimits {
            output_bytes: 1024,
            ..VmLimits::default()
        },
    )
    .expect("adequate split quota permits discarded pieces");
    assert!(adequate.is_empty());

    let tail_input = format!("a{}", "a".repeat(1_000));
    let tail_input = serde_json::to_string(&tail_input).expect("valid JSON input");
    let first = evaluate_with_limits(
        r#"
        def first_piece($pattern): first(splits($pattern));
        first_piece("a")
        "#,
        &tail_input,
        VmLimits {
            steps: 32,
            output_bytes: 4,
            ..VmLimits::default()
        },
    )
    .expect("first split result must not copy the unobserved tail");
    assert_eq!(first, [Value::string("")]);
}
