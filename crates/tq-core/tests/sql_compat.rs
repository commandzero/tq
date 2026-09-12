//! Public VM checks for jq's SQL-style `IN`, `INDEX`, and `JOIN` built-ins.

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
fn in_supports_one_and_two_argument_stream_forms() {
    for (query, input, expected) in [
        (r#"IN(["a","b"][])"#, r#""b""#, "true"),
        (r#"IN(["a","b"][])"#, r#""c""#, "false"),
        (r#"IN(.[]; ["b"][])"#, r#"["a","b"]"#, "true"),
        (r#"IN(.[]; ["c"][])"#, r#"["a","b"]"#, "false"),
    ] {
        assert_eq!(evaluate(query, input)[0].to_string(), expected, "{query}");
    }
    assert_eq!(
        evaluate(r#"IN((1,error("later")))"#, "1")[0].to_string(),
        "true"
    );
    let error = evaluate_with_limits(r#"IN((2,error("later")))"#, "1", VmLimits::default())
        .expect_err("a later source error is observable after no match");
    assert!(error.to_string().contains("later"));
    assert_eq!(
        evaluate("IN(1; (1,2))", "null")
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["true"]
    );
    assert_eq!(evaluate("IN((1,2); empty)", "null"), [Value::Bool(false)]);
    let error = evaluate_with_limits("IN(empty; error(\"late\"))", "null", VmLimits::default())
        .expect_err("the right IN stream is still evaluated when the source is empty");
    assert!(error.to_string().contains("late"));
    assert_eq!(
        evaluate("limit(1; IN(1; (1, recurse(.))))", "null")[0].to_string(),
        "true"
    );
    assert_eq!(evaluate("IN((1, recurse(.)))", "1")[0].to_string(), "true");
}

#[test]
fn index_supports_stream_and_default_input_forms() {
    let input = r#"[{"id":"a","v":1},{"id":"b","v":2}]"#;
    assert_eq!(
        evaluate("INDEX(.[]; .id)", input)[0].to_string(),
        r#"{"a":{"id":"a","v":1},"b":{"id":"b","v":2}}"#
    );
    assert_eq!(
        evaluate("INDEX(.id)", input)[0].to_string(),
        r#"{"a":{"id":"a","v":1},"b":{"id":"b","v":2}}"#
    );
    assert_eq!(
        evaluate(
            "INDEX(.[]; .id)",
            r#"[{"id":"a","v":1},{"id":"a","v":2},{"id":"b","v":3}]"#,
        )[0]
        .to_string(),
        r#"{"a":{"id":"a","v":2},"b":{"id":"b","v":3}}"#
    );
    assert_eq!(evaluate("INDEX(empty; .id)", input)[0].to_string(), "{}");
}

#[test]
fn join_supports_all_overloads_and_join_expressions() {
    let input = r#"[{"id":"a","v":1},{"id":"b","v":2}]"#;
    assert_eq!(
        evaluate("JOIN({\"a\":10}; .id)", r#"[{"id":"a"},{"id":"b"}]"#)[0].to_string(),
        r#"[[{"id":"a"},10],[{"id":"b"},null]]"#
    );
    assert_eq!(
        evaluate("INDEX(.[]; .id) as $idx | JOIN($idx; .[]; .id)", input,)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        [
            r#"[{"id":"a","v":1},{"id":"a","v":1}]"#,
            r#"[{"id":"b","v":2},{"id":"b","v":2}]"#,
        ]
    );
    assert_eq!(
        evaluate(
            "INDEX(.[]; .id) as $idx | JOIN($idx; .[]; .id; {row:.[0],match:.[1]})",
            input,
        )
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>(),
        [
            r#"{"row":{"id":"a","v":1},"match":{"id":"a","v":1}}"#,
            r#"{"row":{"id":"b","v":2},"match":{"id":"b","v":2}}"#,
        ]
    );
    assert_eq!(
        evaluate("JOIN({\"a\":10}; .id; .)", r#"{"id":"a"}"#)[0].to_string(),
        r#"["a",10]"#
    );
    assert_eq!(
        evaluate("JOIN(null; .[]; .id)", r#"[{"id":"a"}]"#)[0].to_string(),
        r#"[{"id":"a"},null]"#
    );
    assert_eq!(
        evaluate("JOIN([10]; .[]; .id)", r#"[{"id":0}]"#)[0].to_string(),
        r#"[{"id":0},10]"#
    );
    assert_eq!(
        evaluate("INDEX(range(0;3); .)", "null")[0].to_string(),
        r#"{"0":0,"1":1,"2":2}"#
    );
    assert_eq!(
        evaluate("INDEX(.[]; (., .+1))", "[1]")[0].to_string(),
        r#"{"1":1,"2":1}"#
    );
    assert_eq!(
        evaluate("JOIN({\"a\":1}; .[]; .id; (.[0], .[1]))", r#"[{"id":"a"}]"#,)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["{\"id\":\"a\"}", "1"]
    );
}

#[test]
fn join_streams_partial_results_and_honors_outer_limits() {
    let input = Value::Null;
    let plan = analyze(
        resolve(
            parse(r#"JOIN({"a":1}; ("a",error("later")); .)"#).expect("query parses"),
            &ResolveOptions::default(),
        )
        .expect("query resolves"),
    )
    .compile()
    .expect("query compiles")
    .document_plan();
    let mut vm = Vm::new(&plan, input, VmLimits::default());
    assert_eq!(
        vm.next_result()
            .expect("first result succeeds")
            .expect("first result exists")
            .to_string(),
        r#"["a",1]"#
    );
    let error = vm
        .next_result()
        .expect_err("later stream error remains observable");
    assert!(error.to_string().contains("later"));
    assert_eq!(
        evaluate("limit(1; JOIN({\"a\":1}; (\"a\", recurse(.)); .))", "null")[0].to_string(),
        r#"["a",1]"#
    );
}

#[test]
fn sql_scans_charge_vm_steps_and_preserve_errors() {
    let input = serde_json::to_string(&(0..256).collect::<Vec<_>>()).expect("array JSON");
    let error = evaluate_with_limits(
        "IN(.[]; 255)",
        &input,
        VmLimits {
            steps: 3,
            ..VmLimits::default()
        },
    )
    .expect_err("membership scan should consume the VM step budget");
    assert!(matches!(
        error,
        tq_core::VmError::Resource {
            resource: "vm-steps"
        }
    ));
    let error = evaluate_with_limits("INDEX(.[]; error(\"later\"))", "[1]", VmLimits::default())
        .expect_err("index key errors remain observable");
    assert!(error.to_string().contains("later"));
}
