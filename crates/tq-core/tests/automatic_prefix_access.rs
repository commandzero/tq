//! Regression coverage for one-step automatic prefix access programs.

use std::collections::BTreeMap;

use tq_core::{
    AutomaticPlan, Compiled, Document, Number, Object, PathComponent, Plan, ResolveOptions, Value,
    Vm, VmError, VmLimits, analyze, parse, resolve,
};

fn automatic_plan(query: &str) -> AutomaticPlan {
    analyze(
        resolve(
            parse(query).expect("query parses"),
            &ResolveOptions::default(),
        )
        .expect("query resolves"),
    )
    .compile()
    .expect("query compiles")
    .automatic_plan()
    .expect("query admits an automatic plan")
}

fn document_plan(query: &str) -> Plan<Compiled, Document> {
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

fn drain(mut vm: Vm) -> Result<Vec<Value>, VmError> {
    let mut values = Vec::new();
    while let Some(value) = vm.next_result()? {
        values.push(value);
    }
    Ok(values)
}

fn prefix_for<M>(
    plan: &Plan<Compiled, M>,
    component: usize,
    input: Value,
) -> Result<Vec<Value>, VmError> {
    drain(Vm::new_automatic_prefix_access(
        plan,
        component,
        input,
        VmLimits::default(),
        BTreeMap::new(),
    ))
}

fn prefix(plan: &AutomaticPlan, component: usize, input: Value) -> Result<Vec<Value>, VmError> {
    match plan {
        AutomaticPlan::Events(plan) => prefix_for(plan, component, input),
        AutomaticPlan::Subtree(plan) => prefix_for(plan, component, input),
        AutomaticPlan::HybridBlocking(plan) => prefix_for(plan, component, input),
        other => panic!("expected automatic stream plan, got {other:?}"),
    }
}

fn prefix_path(plan: &AutomaticPlan) -> Option<&[PathComponent]> {
    match plan {
        AutomaticPlan::Events(plan) => plan.automatic_prefix(),
        AutomaticPlan::Subtree(plan) => plan.automatic_prefix(),
        AutomaticPlan::HybridBlocking(plan) => plan.automatic_prefix(),
        _ => None,
    }
}

fn document_results(query: &str, input: Value) -> Result<Vec<Value>, VmError> {
    drain(Vm::new(&document_plan(query), input, VmLimits::default()))
}

fn object(entries: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    let mut values = Object::new();
    for (key, value) in entries {
        values.insert(key.into(), value);
    }
    Value::object(values)
}

fn number(literal: &str) -> Value {
    Value::Number(Number::parse(literal).expect("number literal parses"))
}

#[test]
fn each_static_component_matches_document_vm_on_wrong_ancestor_shapes() {
    let plan = automatic_plan(".outer.inner[]");
    assert_eq!(
        prefix_path(&plan),
        Some(
            &[
                PathComponent::Key("outer".into()),
                PathComponent::Key("inner".into()),
            ][..]
        )
    );

    let inputs = [
        Value::Bool(true),
        Value::array(vec![Value::Null]),
        object([("outer", Value::Bool(true))]),
        object([("outer", Value::array(vec![]))]),
        object([("outer", object([]))]),
        object([(
            "outer",
            object([("inner", Value::array(vec![number("1")]))]),
        )]),
    ];
    for input in inputs {
        let first_expected = document_results(".outer", input.clone());
        let first_actual = prefix(&plan, 0, input);
        assert_eq!(first_actual, first_expected, "outer access diverged");

        if let Ok(values) = first_expected {
            for value in values {
                assert_eq!(
                    prefix(&plan, 1, value.clone()),
                    document_results(".inner", value),
                    "inner access diverged",
                );
            }
        }
    }
}

#[test]
fn missing_and_null_ancestors_follow_document_access() {
    let plan = automatic_plan(".outer.inner[]");
    for input in [object([]), Value::Null] {
        let first = prefix(&plan, 0, input).expect("missing/null outer access succeeds");
        assert_eq!(first, [Value::Null]);
        let second = prefix(&plan, 1, first[0].clone()).expect("null inner access succeeds");
        assert_eq!(second, [Value::Null]);
    }
}

#[test]
fn huge_nonnegative_index_is_one_step_and_does_not_need_dense_storage() {
    let index = 1_000_000_000usize;
    let plan = automatic_plan(&format!(".[{index}][]"));
    assert_eq!(prefix_path(&plan), Some(&[PathComponent::Index(index)][..]));
    let input = Value::array(vec![Value::Null]);
    let actual = prefix(&plan, 0, input.clone());
    let expected = document_results(&format!(".[{index}]"), input);
    assert_eq!(actual, expected);
    assert_eq!(actual.expect("large index access succeeds"), [Value::Null]);
}

#[test]
fn hybrid_separates_shared_base_producer_from_collection_suffix() {
    let query = "[.features[].id] | sort | length | . + 1";
    let plan = automatic_plan(query);
    let AutomaticPlan::HybridBlocking(plan) = &plan else {
        panic!("query should use the hybrid plan");
    };
    let input = object([(
        "features",
        Value::array(vec![
            object([("id", number("3"))]),
            object([("id", number("1"))]),
        ]),
    )]);

    let selected = prefix_for(plan, 0, input.clone()).expect("prefix access succeeds");
    assert_eq!(selected.len(), 1);
    let base_values = drain(Vm::new_automatic_base(
        plan,
        selected.into_iter().next().expect("selected value"),
        VmLimits::default(),
        BTreeMap::new(),
    ))
    .expect("shared base producer succeeds");
    assert_eq!(base_values, [number("3"), number("1")]);

    let suffix = drain(Vm::new_hybrid_suffix(
        plan,
        Value::array(base_values),
        VmLimits::default(),
        BTreeMap::new(),
    ))
    .expect("collection suffix evaluates the complete producer result");
    assert_eq!(
        suffix,
        document_results(query, input).expect("document succeeds")
    );
    assert_eq!(suffix, [number("3")]);
}
