#![allow(missing_docs)]

use std::{
    fmt::Write as _,
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

fn module_root() -> std::path::PathBuf {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "tq-resolver-metadata-{}-{suffix}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn imported_deep_constant_metadata_on_one_mebibyte_stack() {
    bounded(
        "imported_deep_constant_metadata_on_one_mebibyte_stack",
        || {
            let root = module_root();
            for (name, open, close) in [("arrays", "[", "]"), ("objects", "{child:", "}")] {
                let payload = format!("{}0{}", open.repeat(1024), close.repeat(1024));
                let module = format!("module {{payload: {payload}}}; def f: 0;");
                assert!(module.len() < ResolveOptions::default().module_bytes);
                std::fs::write(root.join(format!("{name}.jq")), module).unwrap();
                let resolved = resolve(
                    parse(&format!("import \"{name}\" as m; m::f")).unwrap(),
                    &ResolveOptions {
                        module_roots: vec![root.clone()],
                        ..ResolveOptions::default()
                    },
                )
                .expect("nested constant metadata resolves");
                let Value::Object(metadata) = &resolved.modules()[0].metadata else {
                    panic!("object metadata")
                };
                let mut leaf = metadata.get("payload").unwrap();
                for _ in 0..1024 {
                    leaf = match leaf {
                        Value::Array(values) => {
                            assert_eq!(values.len(), 1);
                            &values[0]
                        }
                        Value::Object(values) => {
                            assert_eq!(values.len(), 1);
                            values.get("child").unwrap()
                        }
                        _ => panic!("constant container shape preserved"),
                    };
                }
                assert_eq!(leaf, &Value::from_json(serde_json::json!(0)).unwrap());
            }
            std::fs::remove_dir_all(root).unwrap();
        },
    );
}

#[test]
fn imported_deep_metadata_sequence_on_one_mebibyte_stack() {
    bounded(
        "imported_deep_metadata_sequence_on_one_mebibyte_stack",
        || {
            let root = module_root();
            let mut sequence = "empty".to_owned();
            for value in (0..1024).rev() {
                sequence = format!("{value},({sequence})");
            }
            std::fs::write(
                root.join("sequence.jq"),
                format!("module {{payload: [{sequence}]}}; def f: 0;"),
            )
            .unwrap();
            let resolved = resolve(
                parse(r#"import "sequence" as m; m::f"#).unwrap(),
                &ResolveOptions {
                    module_roots: vec![root.clone()],
                    ..ResolveOptions::default()
                },
            )
            .expect("constant comma metadata resolves");
            let Value::Object(metadata) = &resolved.modules()[0].metadata else {
                panic!("object metadata")
            };
            let Value::Array(values) = metadata.get("payload").unwrap() else {
                panic!("array payload")
            };
            assert_eq!(values.len(), 1024);
            for (index, value) in values.iter().enumerate() {
                assert_eq!(value, &Value::from_json(serde_json::json!(index)).unwrap());
            }
            drop(resolved);
            std::fs::remove_dir_all(root).unwrap();
        },
    );
}

#[test]
fn imported_definition_and_include_chain_on_one_mebibyte_stack() {
    bounded(
        "imported_definition_and_include_chain_on_one_mebibyte_stack",
        || {
            let root = module_root();
            std::fs::write(root.join("leaf.jq"), "def leaf: 0;").unwrap();
            let mut module = String::new();
            for index in 0..512 {
                writeln!(
                    module,
                    "def f{index}: 0; include \"leaf\" {{search: [\"$ORIGIN\"], tag: {index}}};"
                )
                .unwrap();
            }
            std::fs::write(root.join("chain.jq"), module).unwrap();
            let resolved = resolve(
                parse(r#"import "chain" as m; m::leaf"#).unwrap(),
                &ResolveOptions {
                    module_roots: vec![root.clone()],
                    ..ResolveOptions::default()
                },
            )
            .expect("long definition/include chain resolves");
            let info = resolved
                .modules()
                .iter()
                .find(|module| module.name == "chain")
                .unwrap();
            let Value::Object(metadata) = &info.metadata else {
                panic!("object metadata")
            };
            let Value::Array(definitions) = metadata.get("defs").unwrap() else {
                panic!("definition metadata")
            };
            let Value::Array(dependencies) = metadata.get("deps").unwrap() else {
                panic!("dependency metadata")
            };
            assert_eq!(definitions.len(), 512);
            assert_eq!(dependencies.len(), 512);
            for (index, definition) in definitions.iter().enumerate() {
                assert_eq!(definition, &Value::string(format!("f{index}/0")));
            }
            for dependency in dependencies.iter() {
                let Value::Object(fields) = dependency else {
                    panic!("dependency entry")
                };
                assert_eq!(fields.get("relpath"), Some(&Value::string("leaf")));
                assert_eq!(fields.get("is_data"), Some(&Value::Bool(false)));
                assert_eq!(
                    fields.get("search"),
                    Some(&Value::array(vec![Value::string("$ORIGIN")]))
                );
            }
            drop(resolved);
            std::fs::remove_dir_all(root).unwrap();
        },
    );
}

#[test]
fn imported_definition_and_import_chain_on_one_mebibyte_stack() {
    bounded(
        "imported_definition_and_import_chain_on_one_mebibyte_stack",
        || {
            let root = module_root();
            std::fs::write(root.join("leaf.jq"), "def leaf: 0;").unwrap();
            std::fs::write(root.join("data.json"), "[1,2]").unwrap();
            let mut module = String::new();
            for index in 0..512 {
                let (path, alias) = if index % 2 == 0 {
                    ("leaf", format!("ns{index}"))
                } else {
                    ("data", format!("$data{index}"))
                };
                writeln!(
                    module,
                    "def f{index}: 0; import \"{path}\" as {alias} {{search: [\"$ORIGIN\"]}};"
                )
                .unwrap();
            }
            std::fs::write(root.join("imports.jq"), module).unwrap();
            let resolved = resolve(
                parse(r#"import "imports" as m; m::f511"#).unwrap(),
                &ResolveOptions {
                    module_roots: vec![root.clone()],
                    ..ResolveOptions::default()
                },
            )
            .expect("long definition/filter/data import chain resolves");
            let info = resolved
                .modules()
                .iter()
                .find(|module| module.name == "imports")
                .unwrap();
            let Value::Object(metadata) = &info.metadata else {
                panic!("object metadata")
            };
            let Value::Array(definitions) = metadata.get("defs").unwrap() else {
                panic!("definition metadata")
            };
            let Value::Array(dependencies) = metadata.get("deps").unwrap() else {
                panic!("dependency metadata")
            };
            assert_eq!(definitions.len(), 512);
            assert_eq!(dependencies.len(), 512);
            for (index, dependency) in dependencies.iter().enumerate() {
                let Value::Object(fields) = dependency else {
                    panic!("dependency entry")
                };
                let is_data = index % 2 != 0;
                assert_eq!(fields.get("is_data"), Some(&Value::Bool(is_data)));
                assert_eq!(
                    fields.get("relpath"),
                    Some(&Value::string(if is_data { "data" } else { "leaf" }))
                );
                assert_eq!(
                    fields.get("as"),
                    (!is_data)
                        .then(|| Value::string(format!("ns{index}")))
                        .as_ref()
                );
                assert_eq!(
                    fields.get("search"),
                    Some(&Value::array(vec![Value::string("$ORIGIN")]))
                );
            }
            drop(resolved);
            std::fs::remove_dir_all(root).unwrap();
        },
    );
}

#[test]
fn imported_deep_metadata_and_content_diagnostics_on_one_mebibyte_stack() {
    bounded(
        "imported_deep_metadata_and_content_diagnostics_on_one_mebibyte_stack",
        || {
            let root = module_root();
            let payload = format!("{}$missing{}", "[".repeat(1024), "]".repeat(1024));
            let module = format!("module {payload}; def f: 0;");
            std::fs::write(root.join("invalid.jq"), &module).unwrap();
            let options = ResolveOptions {
                module_roots: vec![root.clone()],
                ..ResolveOptions::default()
            };
            let error = resolve(parse(r#"import "invalid" as m; ."#).unwrap(), &options)
                .expect_err("nonconstant nested module metadata returns its original diagnostic");
            assert_eq!(error.code, "TQ-MODULE-METADATA-001");
            assert_eq!(
                error.message,
                "module metadata must be a constant expression"
            );
            assert_eq!(error.labels[0].span.source, SourceId::new(2));
            assert_eq!(error.labels[0].span.start, 7);
            assert_eq!(
                usize::try_from(error.labels[0].span.end).unwrap(),
                7 + payload.len()
            );

            let query = format!("include \"invalid\" {{payload: {payload}}}; .");
            let error = resolve(parse(&query).unwrap(), &options).expect_err(
                "nonconstant directive metadata is diagnosed before loading the module",
            );
            assert_eq!(error.code, "TQ-MODULE-METADATA-001");
            assert_eq!(
                error.message,
                "module metadata must be a constant expression"
            );
            assert_eq!(error.labels[0].span.source, SourceId::new(0));
            assert_eq!(error.labels[0].span.start, 0);
            assert_eq!(
                usize::try_from(error.labels[0].span.end).unwrap(),
                query.len()
            );

            let mut module = String::new();
            for index in 0..1024 {
                writeln!(module, "def f{index}: 0;").unwrap();
            }
            let terminal = module.len();
            module.push('1');
            std::fs::write(root.join("content.jq"), module).unwrap();
            let error = resolve(parse(r#"import "content" as m; ."#).unwrap(), &options)
            .expect_err("invalid terminal module content retains its diagnostic after peeling definitions");
            assert_eq!(error.code, "TQ-MODULE-CONTENT-001");
            assert_eq!(
                error.message,
                "module files may contain metadata, imports, includes, and definitions"
            );
            assert_eq!(error.labels[0].span.source, SourceId::new(2));
            assert_eq!(
                usize::try_from(error.labels[0].span.start).unwrap(),
                terminal
            );
            assert_eq!(
                usize::try_from(error.labels[0].span.end).unwrap(),
                terminal + 1
            );
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
