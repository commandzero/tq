//! Public VM date built-in overload witnesses.

use tq_core::{ResolveOptions, Value, Vm, VmLimits, analyze, parse, resolve};

fn evaluate(query: &str, input: &str) -> Vec<Value> {
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
    let mut vm = Vm::new(&plan, input, VmLimits::default());
    let mut values = Vec::new();
    while let Some(value) = vm.next_result().expect("query evaluates") {
        values.push(value);
    }
    values
}

fn evaluate_result(query: &str, input: &str) -> Result<Vec<Value>, tq_core::VmError> {
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
    let mut vm = Vm::new(&plan, input, VmLimits::default());
    let mut values = Vec::new();
    while let Some(value) = vm.next_result()? {
        values.push(value);
    }
    Ok(values)
}

#[test]
fn todate_formats_epoch_seconds_as_a_utc_string() {
    assert_eq!(
        evaluate("todate", "0"),
        [Value::string("1970-01-01T00:00:00Z")]
    );
}

#[test]
fn fromdateiso8601_parses_a_utc_string() {
    assert_eq!(
        evaluate("fromdateiso8601", "\"1970-01-01T00:00:00Z\""),
        [Value::Number(tq_core::Number::from_f64(0.0).unwrap())]
    );
}

#[test]
fn localtime_is_denied_without_the_platform_capability() {
    let error = evaluate_result("localtime", "0").expect_err("platform access is denied");
    assert!(error.to_string().contains("platform"));
}
