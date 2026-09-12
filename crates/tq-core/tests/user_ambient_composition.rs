//! Public VM coverage for capability-gated ambient scalar built-ins.

use std::{collections::BTreeMap, sync::Arc};

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

fn evaluate(
    query: &str,
    input: Value,
    variables: BTreeMap<Arc<str>, Value>,
) -> Result<Vec<Value>, VmError> {
    evaluate_with_limits(query, input, variables, VmLimits::default())
}

fn evaluate_with_limits(
    query: &str,
    input: Value,
    variables: BTreeMap<Arc<str>, Value>,
    limits: VmLimits,
) -> Result<Vec<Value>, VmError> {
    let plan = plan(query);
    let mut vm = Vm::new_with_variables(&plan, input, limits, variables);
    let mut values = Vec::new();
    while let Some(value) = vm.next_result()? {
        values.push(value);
    }
    Ok(values)
}

fn platform_variables() -> BTreeMap<Arc<str>, Value> {
    BTreeMap::from([
        (Arc::from("__tq_ambient_platform"), Value::Bool(true)),
        (
            Arc::from("__tq_ambient_environment"),
            Value::object(indexmap::IndexMap::from([(
                Arc::from("TQ_TEST_SECRET"),
                Value::string("redacted"),
            )])),
        ),
    ])
}

#[test]
fn ambient_builtins_are_denied_without_reserved_capabilities() {
    for query in [
        "def read: env; read",
        "def read: now; read",
        "def read: localtime; read",
        "def read: 0 | strflocaltime(\"%Y\"); read",
    ] {
        let error = evaluate(query, Value::Null, BTreeMap::new())
            .expect_err("ambient access must be denied by default");
        assert!(matches!(error, VmError::CapabilityDenied { .. }));
        assert!(error.to_string().contains("capability"));
    }
}

#[test]
fn denied_capabilities_remain_catchable_and_optional() {
    let caught = evaluate("try env catch .", Value::Null, BTreeMap::new())
        .expect("capability denial is catchable");
    assert_eq!(
        caught,
        [Value::string(
            "env requires environment access permitted by capability policy",
        )]
    );

    let suppressed = evaluate("env?", Value::Null, BTreeMap::new())
        .expect("optional capability denial is suppressible");
    assert_eq!(suppressed, [] as [tq_core::Value; 0]);
}

#[test]
fn explicitly_admitted_environment_and_platform_builtins_work_in_defs() {
    let variables = platform_variables();
    let keys = evaluate("def read: env | keys; read", Value::Null, variables.clone())
        .expect("admitted env evaluates");
    assert_eq!(
        keys,
        [Value::from_json(serde_json::json!(["TQ_TEST_SECRET"])).unwrap()]
    );

    let localtime_length = evaluate(
        "def read: localtime | length; read",
        Value::from_json(serde_json::json!(0)).unwrap(),
        variables.clone(),
    )
    .expect("admitted localtime evaluates");
    assert_eq!(
        localtime_length,
        [Value::from_json(serde_json::json!(8)).unwrap()]
    );

    let now_type = evaluate("def read: now | type; read", Value::Null, variables.clone())
        .expect("admitted now evaluates");
    assert_eq!(now_type, [Value::string("number")]);

    let format_lengths = evaluate(
        "def format($f): 0 | strflocaltime($f) | length; [format((\"%Y\",\"%m\"))]",
        Value::Null,
        variables,
    )
    .expect("captured strflocaltime formats evaluate");
    assert_eq!(
        format_lengths,
        [Value::from_json(serde_json::json!([4, 2])).unwrap()]
    );
}

#[test]
fn ambient_output_and_work_limits_remain_active() {
    let variables = platform_variables();
    let error = evaluate_with_limits(
        "def local: 0 | strflocaltime(\"%Y\"); local",
        Value::Null,
        variables.clone(),
        VmLimits {
            output_bytes: 1,
            ..VmLimits::default()
        },
    )
    .expect_err("formatted ambient output observes the byte quota");
    assert!(matches!(error, VmError::Resource { .. }));

    let error = evaluate_with_limits(
        "def clock: now; clock",
        Value::Null,
        variables,
        VmLimits {
            steps: 1,
            ..VmLimits::default()
        },
    )
    .expect_err("ambient calls observe the VM work budget");
    assert!(matches!(
        error,
        VmError::Resource {
            resource: "vm-steps"
        }
    ));
}
