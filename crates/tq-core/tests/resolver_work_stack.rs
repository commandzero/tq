#![allow(missing_docs)]

use std::{
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use tq_core::{
    ResolveOptions, SourceId, Value, Vm, VmLimits, analyze, parse, parse_with_startup, resolve,
};

const CHILD_ENV: &str = "TQ_RESOLVER_WORK_STACK_CHILD";

fn bounded(name: &str, run: impl FnOnce() + Send + 'static) {
    if std::env::var_os(CHILD_ENV).is_some() {
        thread::Builder::new()
            .name("resolver-one-mebibyte".to_owned())
            .stack_size(1024 * 1024)
            .spawn(run)
            .expect("resolver thread starts")
            .join()
            .expect("resolver thread succeeds");
        return;
    }
    let mut child = Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", name, "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, "1")
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("isolated resolver child starts");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.try_wait().expect("resolver child status") {
            assert!(status.success(), "resolver child failed: {status}");
            return;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("resolver child exceeded ten-second deadline");
        }
        thread::sleep(Duration::from_millis(25));
    }
}

#[test]
fn right_nested_commas_resolve_and_execute_on_one_mebibyte_stack() {
    bounded(
        "right_nested_commas_resolve_and_execute_on_one_mebibyte_stack",
        || {
            for length in [44, 256] {
                let mut body = length.to_string();
                for value in (1..length).rev() {
                    body = format!("{value},({body})");
                }
                let query = resolve(
                    parse(&format!("[{body}] | sort | length")).unwrap(),
                    &ResolveOptions::default(),
                )
                .expect("right-nested comma query resolves");
                let analyzed = analyze(query);
                assert_eq!(analyzed.analysis().optimizer_rewrites.len(), 1);
                let plan = analyzed.compile().expect("query compiles").document_plan();
                let mut vm = Vm::new(&plan, Value::Null, VmLimits::default());
                assert_eq!(
                    vm.next_result().unwrap(),
                    Some(Value::from_json(serde_json::json!(length)).unwrap())
                );
                assert_eq!(vm.next_result().unwrap(), None);
            }
        },
    );
}

#[test]
fn startup_location_at_depth_1024_preserves_source_on_one_mebibyte_stack() {
    bounded(
        "startup_location_at_depth_1024_preserves_source_on_one_mebibyte_stack",
        || {
            let startup = format!("def f:\n{}$__loc__{};", "[".repeat(1024), "]".repeat(1024));
            let parsed = parse_with_startup("query.jq", b".", "startup.jq", startup.as_bytes())
                .expect("deep startup location substitution succeeds");
            assert_eq!(
                parsed.source_by_id(SourceId::new(1)).unwrap().text(),
                startup
            );
            resolve(parsed, &ResolveOptions::default()).expect("deep startup resolves");
        },
    );
}

#[test]
fn nested_scopes_resolve_on_one_mebibyte_stack() {
    bounded("nested_scopes_resolve_on_one_mebibyte_stack", || {
        for form in [
            "bind",
            "def",
            "label",
            "reduce",
            "foreach",
            "call",
            "if",
            "interpolation",
        ] {
            let mut body = "0".to_owned();
            for _ in 0..256 {
                body = match form {
                    "bind" => format!("0 as $x | ({body}), $x"),
                    "def" => format!("def f($x; g): {body}; f(0; .)"),
                    "label" => format!("label $x | ({body}), break $x"),
                    "reduce" => format!("reduce .[] as $x (0; ({body}) + $x)"),
                    "foreach" => format!("foreach .[] as $x (0; {body}; $x)"),
                    "call" => format!("map({body})"),
                    "if" => format!("if true then {body} else 0 end"),
                    "interpolation" => format!("\"\\({body})\""),
                    _ => unreachable!(),
                };
            }
            let resolved = resolve(
                parse(&body).expect("nested scope parses"),
                &ResolveOptions::default(),
            )
            .expect("nested scope resolves");
            let _ = analyze(resolved);
        }
    });
}

#[test]
fn cached_deep_module_preserves_qualification_on_one_mebibyte_stack() {
    bounded(
        "cached_deep_module_preserves_qualification_on_one_mebibyte_stack",
        || {
            let suffix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir()
                .join(format!("tq-resolver-work-{}-{suffix}", std::process::id()));
            std::fs::create_dir_all(&root).unwrap();
            let module = format!(
                "def location:\n{}$__loc__{};\ndef wrapper: location;",
                "[".repeat(1024),
                "]".repeat(1024)
            );
            std::fs::write(root.join("deep.jq"), module).unwrap();
            let parsed =
                parse(r#"import "deep" as a; import "deep" as b; a::wrapper, b::wrapper"#).unwrap();
            let resolved = resolve(
                parsed,
                &ResolveOptions {
                    module_roots: vec![root.clone()],
                    ..ResolveOptions::default()
                },
            )
            .expect("both fresh and cached deep imports resolve and qualify internal calls");
            assert_eq!(resolved.modules().len(), 1);
            drop(resolved);
            std::fs::remove_dir_all(root).unwrap();
        },
    );
}

#[test]
fn deep_unknown_variable_keeps_diagnostic_span_on_one_mebibyte_stack() {
    bounded(
        "deep_unknown_variable_keeps_diagnostic_span_on_one_mebibyte_stack",
        || {
            let body = format!("{}$missing{}", "[".repeat(1024), "]".repeat(1024));
            let error = resolve(parse(&body).unwrap(), &ResolveOptions::default())
                .expect_err("unknown variable returns a diagnostic");
            assert_eq!(error.code, "TQ-RESOLVE-VARIABLE-001");
            assert_eq!(error.message, "unknown variable $missing");
            assert_eq!(error.labels[0].span.start, 1024);
            assert_eq!(error.labels[0].span.end, 1032);
        },
    );
}
