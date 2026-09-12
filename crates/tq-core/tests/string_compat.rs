//! Public jq-compatible string behavior at the parser, resolver, and VM seam.

use tq_core::{ResolveOptions, Value, Vm, VmError, VmLimits, analyze, parse, resolve};

fn evaluate(query: &str, input: &str) -> Vec<Value> {
    evaluate_with_limits(query, input, VmLimits::default()).expect("query evaluates")
}

fn evaluate_with_limits(query: &str, input: &str, limits: VmLimits) -> Result<Vec<Value>, VmError> {
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
    while let Some(value) = vm.next_result()? {
        values.push(value);
    }
    Ok(values)
}

#[test]
fn ascii_case_filters_change_ascii_only() {
    assert_eq!(
        evaluate("ascii_upcase", r#""AbÇ-ä-123""#),
        [Value::string("ABÇ-ä-123")]
    );
    assert_eq!(
        evaluate("ascii_downcase", r#""AbÇ-ä-123""#),
        [Value::string("abÇ-ä-123")]
    );
}

#[test]
fn trim_prefix_suffix_and_affix_filters_match_jq() {
    assert_eq!(
        evaluate("[trim, ltrim, rtrim]", r#""  ab  ""#)[0].to_string(),
        r#"["ab","ab  ","  ab"]"#
    );
    assert_eq!(
        evaluate("trim", r#""\u00a0\u0085x\u0085\u00a0""#),
        [Value::string("x")]
    );
    assert_eq!(evaluate("trim", r#""  \tAbÇ\n  ""#), [Value::string("AbÇ")]);
    assert_eq!(
        evaluate(r#"ltrimstr("ab")"#, r#""ababc""#),
        [Value::string("abc")]
    );
    assert_eq!(
        evaluate(r#"rtrimstr("ab")"#, r#""cabab""#),
        [Value::string("cab")]
    );
    assert_eq!(
        evaluate(r#"startswith("ab")"#, r#""abc""#),
        [Value::Bool(true)]
    );
    assert_eq!(
        evaluate(r#"endswith("bc")"#, r#""abc""#),
        [Value::Bool(true)]
    );
    assert_eq!(
        evaluate("ltrim", r#""  \tAb\n  ""#),
        [Value::string("Ab\n  ")]
    );
    assert_eq!(
        evaluate("rtrim", r#""  \tAb\n  ""#),
        [Value::string("  \tAb")]
    );
    assert_eq!(
        evaluate(r#"trimstr("ab")"#, r#""ababcab""#),
        [Value::string("abc")]
    );
}

#[test]
fn string_division_splits_by_string_or_unicode_scalar() {
    assert_eq!(
        evaluate(r#". / "/""#, r#""a/b/""#)[0].to_string(),
        r#"["a","b",""]"#
    );
    assert_eq!(
        evaluate(r#". / """#, r#""é😊""#)[0].to_string(),
        r#"["é","😊"]"#
    );
}

#[test]
fn join_and_toboolean_follow_jq_scalar_rules() {
    assert_eq!(
        evaluate(r#"join("|")"#, r#"[true,false,1,2.5,"x",null]"#),
        [Value::string("true|false|1|2.5|x|")]
    );
    assert_eq!(evaluate("join(3)", "[]"), [Value::string("")]);
    assert_eq!(evaluate("join(3)", "[1]"), [Value::string("1")]);
    assert_eq!(
        evaluate("join(\", \")", r#"{"a":1,"b":2}"#),
        [Value::string("1, 2")]
    );
    assert_eq!(evaluate("toboolean", r#""true""#), [Value::Bool(true)]);
    assert_eq!(evaluate("toboolean", "false"), [Value::Bool(false)]);
    assert_eq!(evaluate("toboolean", "true"), [Value::Bool(true)]);
    assert!(evaluate_with_limits("toboolean", r#""TRUE""#, VmLimits::default()).is_err());
}

#[test]
fn utf8_length_and_uri_decode_match_jq() {
    assert!(evaluate_with_limits("@urid", r#""%FF""#, VmLimits::default()).is_err());
    assert_eq!(
        evaluate("utf8bytelength", r#""é""#),
        [Value::Number("2".parse().unwrap())]
    );
    assert_eq!(
        evaluate("@urid", r#""a%2Fb%20%E2%82%AC""#),
        [Value::string("a/b €")]
    );
    assert_eq!(evaluate("@urid", "true"), [Value::string("true")]);
    assert_eq!(evaluate("@urid", "[1,\"x\"]"), [Value::string("[1,\"x\"]")]);
    assert!(evaluate_with_limits("@urid", r#""bad%ZZ""#, VmLimits::default()).is_err());
}

#[test]
fn string_operations_reject_wrong_types_and_bound_join_output() {
    assert!(evaluate_with_limits(". / 1", r#""a""#, VmLimits::default()).is_err());
    assert!(evaluate_with_limits("startswith(1)", r#""a""#, VmLimits::default()).is_err());
    assert!(evaluate_with_limits("join(\",\")", "[{}]", VmLimits::default()).is_err());
    let limits = VmLimits {
        output_bytes: 2,
        ..VmLimits::default()
    };
    assert!(evaluate_with_limits("join(\",\")", r#"["a","b"]"#, limits).is_err());
}
