//! Public regex built-in behavior at the parser, resolver, and VM seam.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

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

#[test]
fn regex_accepts_array_pattern_flags_and_null_flags() {
    assert_eq!(
        evaluate(r#"match(["foo", "g"])"#, r#""foo foo""#),
        evaluate(r#"match("foo"; "g")"#, r#""foo foo""#)
    );
    assert_eq!(
        evaluate(r#"test(["FOO", "i"])"#, r#""foo""#),
        [Value::Bool(true)]
    );
    assert_eq!(
        evaluate(r#"split(","; null)"#, r#""a,b,c""#),
        [Value::array([
            Value::string("a"),
            Value::string("b"),
            Value::string("c"),
        ])]
    );
    assert_eq!(
        evaluate(r#"split("a+")"#, r#""aab""#),
        [Value::array([Value::string("aab")])]
    );
    assert_eq!(
        evaluate(r#"split("a+"; null)"#, r#""aab""#),
        [Value::array([Value::string(""), Value::string("b")])]
    );
    assert_eq!(
        evaluate(r#"[splits("a+")]"#, r#""aab""#),
        [Value::array([Value::string(""), Value::string("b")])]
    );
    let array_error = evaluate_with_limits(r#"split(["a", "g"])"#, r#""a""#, VmLimits::default())
        .expect_err("split does not accept array shorthand");
    assert!(matches!(array_error, VmError::Runtime { message } if message.contains("array")));
    let error = evaluate_with_limits(r#"sub(["p", "g"]; "x")"#, r#""pp""#, VmLimits::default())
        .expect_err("sub does not accept the array shorthand");
    assert!(matches!(error, VmError::Runtime { message } if message.contains("array")));
    for query in [r#"scan(["a", "g"])"#, r#"splits(["a", "g"])"#] {
        let error = evaluate_with_limits(query, r#""aAa""#, VmLimits::default())
            .expect_err("scan and splits do not accept the array shorthand");
        assert!(matches!(error, VmError::Runtime { message } if message.contains("array")));
    }
}

#[test]
fn regex_scan_single_capture_keeps_capture_arrays() {
    assert_eq!(
        evaluate(r#"[scan("(.)")]"#, r#""ab""#),
        [Value::array([
            Value::array([Value::string("a")]),
            Value::array([Value::string("b")]),
        ])]
    );
}

#[test]
fn regex_two_argument_capture_and_scan_follow_jq_overloads() {
    assert_eq!(
        evaluate(r#"capture("(?<x>foo)"; "i")"#, r#""FOO""#)[0].to_string(),
        r#"{"x":"FOO"}"#
    );
    assert_eq!(
        evaluate(r#"[scan("a"; "g")]"#, r#""aAa""#)[0].to_string(),
        r#"["a","a"]"#
    );
}

#[test]
fn regex_supports_scoped_case_and_extended_flags() {
    assert_eq!(
        evaluate(r#"test("foo (?i:bar)"; "x")"#, r#""fooBAR""#),
        [Value::Bool(true)]
    );
    assert_eq!(
        evaluate(r#"test("foo (?i:bar)"; "x")"#, r#""foo bar""#),
        [Value::Bool(false)]
    );
}

#[test]
fn regex_line_flags_match_pinned_jq_probe() {
    assert_eq!(
        evaluate(
            r#"[test("^b"), test("^b"; "s"), test("a.b"), test("a.b"; "s"), test("a.b"; "m"), test("^b"; "m"), test("^b"; "p")]"#,
            r#""a\nb""#,
        ),
        [Value::array([
            Value::Bool(false),
            Value::Bool(false),
            Value::Bool(false),
            Value::Bool(false),
            Value::Bool(true),
            Value::Bool(false),
            Value::Bool(false),
        ])]
    );
}

#[test]
fn regex_reports_unicode_offsets_and_respects_empty_match_suppression() {
    assert_eq!(
        evaluate(r#"[match("a"; "g")]"#, r#""éaé""#),
        [Value::array([Value::object(indexmap::IndexMap::from([
            (
                "offset".into(),
                Value::from_json(serde_json::json!(1)).unwrap(),
            ),
            (
                "length".into(),
                Value::from_json(serde_json::json!(1)).unwrap(),
            ),
            ("string".into(), Value::string("a")),
            ("captures".into(), Value::array(Vec::new())),
        ]))])]
    );
    assert_eq!(evaluate(r#"[match(""; "g")]"#, r#""a""#).len(), 1);
    assert_eq!(
        evaluate(r#"[match(""; "gn")]"#, r#""a""#),
        [Value::array(Vec::new())]
    );
    assert_eq!(
        evaluate(r#"[gsub(""; "X")]"#, r#""a""#),
        [Value::array([Value::string("XaX")])]
    );
    assert_eq!(
        evaluate(r#"[gsub(""; "X"; "n")]"#, r#""a""#),
        [Value::array([Value::string("a")])]
    );
    assert_eq!(
        evaluate(r#"split(""; "n")"#, r#""ab""#),
        [Value::array([Value::string("ab")])]
    );
    assert_eq!(
        evaluate(r#"split("")"#, r#""ab""#),
        [Value::array([Value::string("a"), Value::string("b")])]
    );
}

#[test]
fn regex_unmatched_capture_preserves_jq_compact_field_order() {
    assert_eq!(
        evaluate(r#"[match("(?<x>a)?b")]"#, r#""b""#)[0].to_string(),
        r#"[{"offset":0,"length":1,"string":"b","captures":[{"offset":-1,"string":null,"length":0,"name":"x"}]}]"#
    );
}

#[test]
fn regex_replacement_generators_preserve_branch_cardinality() {
    assert_eq!(
        evaluate(
            r#"[gsub("(?<d>[0-9])"; (if .d == "1" then "a" else "x", "y" end))]"#,
            r#""12""#,
        ),
        [Value::array([Value::string("ax"), Value::string("y")])]
    );
    assert_eq!(
        evaluate(
            r#"[gsub("(?<d>[0-9])"; (if .d == "1" then empty else "x", "y" end))]"#,
            r#""12""#,
        ),
        [Value::array([Value::string("x"), Value::string("y")])]
    );
    assert_eq!(
        evaluate(r#"[gsub("p"; ("a", "b"))]"#, r#""pp""#),
        [Value::array([Value::string("aa"), Value::string("bb")])]
    );
    assert_eq!(
        evaluate(r#"[sub("p"; ("a", "b"))]"#, r#""pp""#),
        [Value::array([Value::string("ap"), Value::string("bp")])]
    );
    assert_eq!(
        evaluate(r#"[gsub("(?<x>[a-z])"; (.x, "z"))]"#, r#""pq""#,),
        [Value::array([Value::string("pq"), Value::string("zz")])]
    );
    assert_eq!(
        evaluate(r#"[sub("(?<x>a)?b"; .x)]"#, r#""b""#),
        [Value::array([Value::string("")])]
    );
    assert_eq!(
        evaluate(
            r#"[gsub("(?<d>[0-9])"; (if .d == "1" then "a", "b" else "x" end))]"#,
            r#""12""#,
        ),
        [Value::array([Value::string("ax"), Value::string("b")])]
    );
}

#[test]
fn empty_replacement_stream_preserves_the_original_input() {
    assert_eq!(
        evaluate(r#"[sub("a"; empty), gsub("a"; empty)]"#, r#""a""#),
        [Value::array([Value::string("a"), Value::string("a")])]
    );
}

#[test]
fn longest_match_remains_an_explicit_safe_engine_limitation() {
    let error = evaluate_with_limits(r#"match("a|ab"; "l")"#, r#""ab""#, VmLimits::default())
        .expect_err("longest-match mode is not available in fancy-regex");
    assert!(matches!(error, VmError::Unsupported { operation } if operation.contains("flag 'l'")));
}

#[test]
fn malformed_patterns_and_flags_are_runtime_regex_errors() {
    let pattern = evaluate_with_limits(r#"test("[")"#, r#""x""#, VmLimits::default())
        .expect_err("malformed regex patterns are runtime errors");
    assert!(matches!(pattern, VmError::Runtime { message } if message.contains("regex pattern")));

    let flag = evaluate_with_limits(r#"test("x"; "z")"#, r#""x""#, VmLimits::default())
        .expect_err("malformed regex flags are runtime errors");
    assert!(matches!(flag, VmError::Runtime { message } if message.contains("regex flag")));
}

#[test]
fn managed_regex_runtime_errors_flow_through_user_catch() {
    assert_eq!(
        evaluate(r#"def call: test("["); try call catch "caught""#, r#""x""#),
        [Value::string("caught")]
    );
    assert_eq!(
        evaluate(
            r#"def call: test("x"; "z"); try call catch "caught""#,
            r#""x""#,
        ),
        [Value::string("caught")]
    );
    assert_eq!(
        evaluate(r#"def call: test("x"); try call catch "caught""#, r"[1]"),
        [Value::string("caught")]
    );
}

#[test]
fn managed_splits_is_pull_driven_and_stops_after_first_result() {
    let input = serde_json::to_string(&"a,".repeat(1_000)).expect("valid JSON string");
    let result = evaluate_with_limits(
        r#"first(splits("a"))"#,
        &input,
        VmLimits {
            steps: 32,
            ..VmLimits::default()
        },
    )
    .expect("first split result should stop before the tail");
    assert_eq!(result, [Value::string("")]);
}

#[test]
fn regex_backtracking_limit_is_an_observed_resource_boundary() {
    let error = evaluate_with_limits(
        r#"test("(?:(a|aa)+)\\1")"#,
        r#""aa""#,
        VmLimits {
            regex_backtrack_limit: 1,
            ..VmLimits::default()
        },
    )
    .expect_err("hostile backtracking must exhaust the configured budget");
    assert_eq!(
        error,
        VmError::Resource {
            resource: "regex-backtrack"
        }
    );
}

#[test]
fn regex_hostile_input_hits_a_small_nonzero_backtracking_budget() {
    let error = evaluate_with_limits(
        r#"test("^(?:(a|aa)+)\\1$")"#,
        r#""aaaaaaaaaaaaaaaaaaaab""#,
        VmLimits {
            regex_backtrack_limit: 100,
            ..VmLimits::default()
        },
    )
    .expect_err("hostile input must exhaust a small nonzero backtracking budget");
    assert_eq!(
        error,
        VmError::Resource {
            resource: "regex-backtrack"
        }
    );
}

#[test]
fn regex_substitution_vectors_have_explicit_resource_bounds() {
    let matches = evaluate_with_limits(
        r#"gsub("p"; "x")"#,
        r#""pp""#,
        VmLimits {
            regex_match_limit: 1,
            ..VmLimits::default()
        },
    )
    .expect_err("retained regex matches must be bounded");
    assert_eq!(
        matches,
        VmError::Resource {
            resource: "regex-match-count"
        }
    );

    let replacements = evaluate_with_limits(
        r#"gsub("p"; ("x", "y"))"#,
        r#""p""#,
        VmLimits {
            regex_replacement_limit: 1,
            ..VmLimits::default()
        },
    )
    .expect_err("replacement branches must be bounded");
    assert_eq!(
        replacements,
        VmError::Resource {
            resource: "regex-replacement-count"
        }
    );
}

#[test]
fn regex_result_vectors_obey_match_limits_across_all_collectors() {
    for query in [
        r#"[match("p"; "g")]"#,
        r#"[capture("p"; "g")]"#,
        r#"[scan("p"; "g")]"#,
        r#"[splits("p"; "g")]"#,
    ] {
        let error = evaluate_with_limits(
            query,
            r#""pp""#,
            VmLimits {
                regex_match_limit: 1,
                ..VmLimits::default()
            },
        )
        .expect_err("regex collectors must bound retained matches and pieces");
        assert_eq!(
            error,
            VmError::Resource {
                resource: "regex-match-count"
            }
        );
    }
}

#[test]
fn regex_rejects_pre_cancelled_matching_operation() {
    let plan = analyze(
        resolve(
            parse(r#"test("(?:(a|aa)+)\\1")"#).unwrap(),
            &ResolveOptions::default(),
        )
        .unwrap(),
    )
    .compile()
    .unwrap()
    .document_plan();
    let cancellation = Arc::new(AtomicBool::new(true));
    let mut vm = Vm::new(&plan, Value::string("aa"), VmLimits::default())
        .with_cancellation(Arc::clone(&cancellation));
    assert_eq!(vm.next_result(), Err(VmError::Interrupted));
    cancellation.store(false, Ordering::Relaxed);
}

#[test]
fn regex_cancellation_is_checked_between_result_operations() {
    let plan = analyze(
        resolve(
            parse(r#"test("a"), test("b")"#).unwrap(),
            &ResolveOptions::default(),
        )
        .unwrap(),
    )
    .compile()
    .unwrap()
    .document_plan();
    let cancellation = Arc::new(AtomicBool::new(false));
    let mut vm = Vm::new(&plan, Value::string("a"), VmLimits::default())
        .with_cancellation(Arc::clone(&cancellation));
    let mut values = Vec::new();
    let result = vm.for_each_result(|value| {
        values.push(value);
        cancellation.store(true, Ordering::Relaxed);
        true
    });
    assert_eq!(result, Err(VmError::Interrupted));
    assert_eq!(values, [Value::Bool(true)]);
}
