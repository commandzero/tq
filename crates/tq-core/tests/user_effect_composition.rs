//! Public VM coverage for composed input providers and effect delivery.

use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use tq_core::{
    InputCursor, InputValue, ResolveOptions, Value, Vm, VmError, VmLimits, analyze, parse, resolve,
};

fn number(value: i64) -> Value {
    Value::from_json(serde_json::json!(value)).expect("number value")
}

#[test]
fn composed_user_filter_consumes_provider_inputs_once_in_order() {
    let plan = analyze(
        resolve(
            parse("def consume: [., inputs]; consume").expect("input composition parses"),
            &ResolveOptions::default(),
        )
        .expect("input composition resolves"),
    )
    .compile()
    .expect("input composition compiles")
    .document_plan();

    let calls = Arc::new(AtomicUsize::new(0));
    let calls_in_provider = Arc::clone(&calls);
    let mut values = vec![
        InputValue {
            value: number(2),
            identity: Arc::from("provider.json"),
            line_number: 2,
        },
        InputValue {
            value: number(3),
            identity: Arc::from("provider.json"),
            line_number: 3,
        },
    ]
    .into_iter();
    let cursor = InputCursor::from_provider(move || {
        calls_in_provider.fetch_add(1, Ordering::Relaxed);
        Ok(values.next())
    });
    let mut vm = Vm::new(&plan, number(1), VmLimits::default()).with_input_cursor(cursor.clone());

    assert_eq!(
        vm.next_result().expect("input composition evaluates"),
        Some(Value::array([number(1), number(2), number(3)])),
    );
    assert_eq!(vm.next_result().expect("input composition completes"), None);
    assert_eq!(calls.load(Ordering::Relaxed), 3);
    assert_eq!(
        cursor.current_context(),
        Some(InputValue {
            value: number(3),
            identity: Arc::from("provider.json"),
            line_number: 3,
        }),
    );
}

#[test]
fn early_composed_input_consumer_does_not_overconsume_provider() {
    let plan = analyze(
        resolve(
            parse("def take_one: first(inputs); take_one").expect("input composition parses"),
            &ResolveOptions::default(),
        )
        .expect("input composition resolves"),
    )
    .compile()
    .expect("input composition compiles")
    .document_plan();

    let calls = Arc::new(AtomicUsize::new(0));
    let calls_in_provider = Arc::clone(&calls);
    let mut values = vec![
        InputValue {
            value: number(2),
            identity: Arc::from("provider.json"),
            line_number: 2,
        },
        InputValue {
            value: number(3),
            identity: Arc::from("provider.json"),
            line_number: 3,
        },
    ]
    .into_iter();
    let cursor = InputCursor::from_provider(move || {
        calls_in_provider.fetch_add(1, Ordering::Relaxed);
        Ok(values.next())
    });
    let mut vm = Vm::new(&plan, Value::Null, VmLimits::default()).with_input_cursor(cursor.clone());

    assert_eq!(
        vm.next_result().expect("early input composition evaluates"),
        Some(number(2)),
    );
    assert_eq!(
        vm.next_result().expect("early input composition completes"),
        None
    );
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert_eq!(
        cursor.current_context(),
        Some(InputValue {
            value: number(2),
            identity: Arc::from("provider.json"),
            line_number: 2,
        }),
    );
}

#[test]
fn composed_effect_sink_delivers_debug_and_stderr_without_duplication() {
    let plan = analyze(
        resolve(
            parse(r#"def emit(f): f; emit("debug-value" | debug), emit("stderr-value" | stderr)"#)
                .expect("effect composition parses"),
            &ResolveOptions::default(),
        )
        .expect("effect composition resolves"),
    )
    .compile()
    .expect("effect composition compiles")
    .document_plan();
    let mut vm = Vm::new(&plan, Value::Null, VmLimits::default());
    let sink = vm.effect_sink();

    assert_eq!(
        vm.next_result().expect("debug composition evaluates"),
        Some(Value::string("debug-value")),
    );
    assert_eq!(
        sink.drain(),
        br#"["DEBUG:","debug-value"]
"#
    );
    assert!(sink.drain().is_empty());
    assert_eq!(
        vm.next_result().expect("stderr composition evaluates"),
        Some(Value::string("stderr-value")),
    );
    assert_eq!(sink.drain(), b"stderr-value");
    assert!(sink.drain().is_empty());
    assert_eq!(
        vm.next_result().expect("effect composition completes"),
        None
    );
}

#[test]
fn composed_debug_argument_evaluates_each_callback_value_once() {
    let plan = analyze(
        resolve(
            parse(r#"def emit(f): f; emit(debug(("left", "right")))"#)
                .expect("debug argument composition parses"),
            &ResolveOptions::default(),
        )
        .expect("debug argument composition resolves"),
    )
    .compile()
    .expect("debug argument composition compiles")
    .document_plan();
    let mut vm = Vm::new(&plan, Value::Null, VmLimits::default());
    let sink = vm.effect_sink();

    assert_eq!(
        vm.next_result().expect("debug argument evaluates"),
        Some(Value::Null),
    );
    assert_eq!(
        sink.drain(),
        br#"["DEBUG:","left"]
["DEBUG:","right"]
"#
    );
    assert_eq!(vm.next_result().expect("debug argument completes"), None);
    assert!(sink.drain().is_empty());
}

#[test]
fn composed_debug_streams_effects_before_a_late_callback_error() {
    let plan = analyze(
        resolve(
            parse(r#"def emit(f): f; emit(debug(("first", error("late"))))"#)
                .expect("debug error composition parses"),
            &ResolveOptions::default(),
        )
        .expect("debug error composition resolves"),
    )
    .compile()
    .expect("debug error composition compiles")
    .document_plan();
    let mut vm = Vm::new(&plan, Value::Null, VmLimits::default());
    let sink = vm.effect_sink();

    let error = vm
        .next_result()
        .expect_err("late debug callback error propagates");
    assert!(matches!(
        error,
        VmError::Raised {
            value: Value::String(message),
            ..
        } if message.as_ref() == "late"
    ));
    assert_eq!(
        sink.drain(),
        br#"["DEBUG:","first"]
"#
    );

    let empty_plan = analyze(
        resolve(
            parse("debug(empty)").expect("empty debug parses"),
            &ResolveOptions::default(),
        )
        .expect("empty debug resolves"),
    )
    .compile()
    .expect("empty debug compiles")
    .document_plan();
    let mut empty_vm = Vm::new(&empty_plan, Value::Null, VmLimits::default());
    let empty_sink = empty_vm.effect_sink();
    assert_eq!(
        empty_vm.next_result().expect("empty debug emits input"),
        Some(Value::Null)
    );
    assert!(empty_sink.drain().is_empty());
}

#[test]
fn composed_user_filter_preserves_shared_input_source_metadata() {
    let plan = analyze(
        resolve(
            parse("def inspect: [input, input_filename, input_line_number, inputs, input_filename, input_line_number]; inspect")
                .expect("input metadata composition parses"),
            &ResolveOptions::default(),
        )
        .expect("input metadata composition resolves"),
    )
    .compile()
    .expect("input metadata composition compiles")
    .document_plan();

    let cursor = InputCursor::from_input_values(vec![
        InputValue {
            value: number(2),
            identity: Arc::from("provider.json"),
            line_number: 20,
        },
        InputValue {
            value: number(3),
            identity: Arc::from("provider.json"),
            line_number: 30,
        },
    ]);
    let variables = BTreeMap::from([(Arc::from("__tq_ambient_platform"), Value::Bool(true))]);
    let mut vm = Vm::new_with_variables(&plan, Value::Null, VmLimits::default(), variables)
        .with_input_cursor(cursor.clone());

    assert_eq!(
        vm.next_result().expect("input metadata evaluates"),
        Some(Value::array([
            number(2),
            Value::string("provider.json"),
            number(20),
            number(3),
            Value::string("provider.json"),
            number(30),
        ])),
    );
    assert_eq!(vm.next_result().expect("input metadata completes"), None);
    assert_eq!(
        cursor.current_context(),
        Some(InputValue {
            value: number(3),
            identity: Arc::from("provider.json"),
            line_number: 30,
        }),
    );
}

#[test]
fn input_source_metadata_requires_platform_capability_even_with_cursor_context() {
    for query in [
        "def inspect: input | input_filename; inspect",
        "def inspect: input | input_line_number; inspect",
    ] {
        let plan = analyze(
            resolve(
                parse(query).expect("input capability query parses"),
                &ResolveOptions::default(),
            )
            .expect("input capability query resolves"),
        )
        .compile()
        .expect("input capability query compiles")
        .document_plan();
        let cursor = InputCursor::from_input_values(vec![InputValue {
            value: number(2),
            identity: Arc::from("provider.json"),
            line_number: 20,
        }]);
        let mut vm = Vm::new(&plan, Value::Null, VmLimits::default()).with_input_cursor(cursor);

        let error = vm
            .next_result()
            .expect_err("metadata must be denied without platform capability");
        assert!(
            matches!(error, VmError::CapabilityDenied { ref message } if message.contains("capability")),
            "{query}: {error}"
        );
    }
}

#[test]
fn input_source_metadata_uses_reserved_fallback_without_cursor_context() {
    let plan = analyze(
        resolve(
            parse("def inspect: [input_filename, input_line_number]; inspect")
                .expect("input fallback query parses"),
            &ResolveOptions::default(),
        )
        .expect("input fallback query resolves"),
    )
    .compile()
    .expect("input fallback query compiles")
    .document_plan();
    let variables = BTreeMap::from([
        (Arc::from("__tq_ambient_platform"), Value::Bool(true)),
        (
            Arc::from("__tq_input_filename"),
            Value::string("fallback.json"),
        ),
        (Arc::from("__tq_input_line_number"), number(9)),
    ]);
    let mut vm = Vm::new_with_variables(&plan, Value::Null, VmLimits::default(), variables);

    assert_eq!(
        vm.next_result().expect("input fallback evaluates"),
        Some(Value::array([Value::string("fallback.json"), number(9)])),
    );
    assert_eq!(vm.next_result().expect("input fallback completes"), None);
}

#[test]
fn missing_input_metadata_remains_an_ordinary_runtime_error() {
    let plan = analyze(
        resolve(
            parse("def inspect: input_filename; inspect").expect("missing metadata query parses"),
            &ResolveOptions::default(),
        )
        .expect("missing metadata query resolves"),
    )
    .compile()
    .expect("missing metadata query compiles")
    .document_plan();
    let variables = BTreeMap::from([(Arc::from("__tq_ambient_platform"), Value::Bool(true))]);
    let mut vm = Vm::new_with_variables(&plan, Value::Null, VmLimits::default(), variables);

    let error = vm
        .next_result()
        .expect_err("missing metadata is a runtime error");
    assert!(matches!(
        error,
        VmError::Runtime { ref message } if message.contains("metadata is unavailable")
    ));
}
