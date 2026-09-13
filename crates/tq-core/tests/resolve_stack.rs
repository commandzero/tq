#![allow(missing_docs)]

use std::{
    fs,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use tq_core::{ResolveOptions, Value, Vm, VmLimits, analyze, parse, resolve};

const LONG_ARRAY_LENGTH: usize = 220;
const TWO_MEBIBYTES: usize = 2 * 1024 * 1024;
const CHILD_ENV: &str = "TQ_RESOLVE_STACK_REGRESSION_CHILD";

fn long_array_query(length: usize) -> String {
    let values = (0..length)
        .map(|index| format!(r#""value-{index}""#))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{values}]")
}

fn long_array_with_location_query(length: usize) -> String {
    let values = (1..length)
        .map(|index| format!(r#""value-{index}""#))
        .collect::<Vec<_>>()
        .join(",");
    format!(r#"["\($__loc__)",{values}]"#)
}

fn evaluate_one(query: &str) -> Value {
    let resolved = resolve(
        parse(query).expect("operator query parses"),
        &ResolveOptions::default(),
    )
    .expect("operator query resolves");
    let plan = analyze(resolved)
        .compile()
        .expect("operator query compiles")
        .document_plan();
    let mut vm = Vm::new(&plan, Value::Null, VmLimits::default());
    let result = vm
        .next_result()
        .expect("operator query evaluates")
        .expect("operator query emits one result");
    assert_eq!(
        vm.next_result().expect("operator query drains"),
        None,
        "operator query must emit exactly one result"
    );
    result
}

#[test]
fn paired_operator_spines_preserve_order_precedence_and_assignment_shape() {
    assert_eq!(
        evaluate_one("1 | . + 2 | . * 3"),
        Value::from_json(serde_json::json!(9)).unwrap()
    );
    assert_eq!(
        evaluate_one("1 + 2 * 3"),
        Value::from_json(serde_json::json!(7)).unwrap()
    );
    assert_eq!(
        evaluate_one("10 - 3 - 2"),
        Value::from_json(serde_json::json!(5)).unwrap()
    );
    assert_eq!(
        evaluate_one("10 - 3 - 2 * 2"),
        Value::from_json(serde_json::json!(3)).unwrap()
    );
    resolve(
        parse("(.a = 1) = 2").expect("nested assignment parses"),
        &ResolveOptions::default(),
    )
    .expect("nested assignment resolves");
}

#[test]
fn long_array_resolves_on_two_megabyte_thread_stack() {
    if std::env::var_os(CHILD_ENV).is_some() {
        let query = long_array_query(LONG_ARRAY_LENGTH);
        let worker = thread::Builder::new()
            .name("resolve-stack-regression".to_owned())
            .stack_size(TWO_MEBIBYTES)
            .spawn(move || {
                resolve(
                    parse(&query).expect("long array parses"),
                    &ResolveOptions::default(),
                )
                .expect("long array resolves");
            })
            .expect("2 MiB resolver thread starts");
        worker.join().expect("2 MiB resolver thread succeeds");
        return;
    }

    let mut child = Command::new(std::env::current_exe().expect("test executable path"))
        .args([
            "--exact",
            "long_array_resolves_on_two_megabyte_thread_stack",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD_ENV, "1")
        .env("RUST_MIN_STACK", TWO_MEBIBYTES.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("bounded resolver child starts");
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait().expect("resolver child status") {
            assert!(status.success(), "resolver child failed: {status}");
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("2 MiB resolver regression exceeded 5-second deadline");
        }
        thread::sleep(Duration::from_millis(25));
    }
}

#[test]
fn import_with_long_definition_body_preserves_module_resolution() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock is after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "tq-resolve-long-include-{}-{suffix}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("module root creates");
    let module = format!(
        "def module_values: {};\n",
        long_array_with_location_query(LONG_ARRAY_LENGTH)
    );
    fs::write(root.join("long_body.jq"), module).expect("long module writes");

    let resolved = resolve(
        parse(r#"import "long_body" as long; long::module_values"#).expect("import query parses"),
        &ResolveOptions {
            module_roots: vec![root.clone()],
            ..ResolveOptions::default()
        },
    )
    .expect("import query resolves");
    let plan = analyze(resolved)
        .compile()
        .expect("import query compiles")
        .document_plan();
    let mut vm = Vm::new(&plan, Value::Null, VmLimits::default());
    let Some(Value::Array(values)) = vm.next_result().expect("import query evaluates") else {
        panic!("import query must emit an array");
    };
    assert_eq!(values.len(), LONG_ARRAY_LENGTH);
    let Value::String(location) = &values[0] else {
        panic!("module location must be a string");
    };
    assert_eq!(
        location.as_ref(),
        format!(
            r#"{{"file":"{}","line":1}}"#,
            fs::canonicalize(root.join("long_body.jq"))
                .expect("module path canonicalizes")
                .display()
        )
    );
    assert_eq!(vm.next_result().expect("import query drains"), None);
    fs::remove_dir_all(root).expect("module root removes");
}
