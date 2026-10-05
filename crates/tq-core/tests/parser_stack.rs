#![allow(missing_docs)]

use std::{
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use tq_core::{SourceId, parse, parse_bytes, parse_with_startup};

const CHILD_ENV: &str = "TQ_PARSER_STACK_CHILD";

#[test]
fn grouped_right_comma_parses_on_one_mebibyte_stack() {
    bounded_parse("grouped_right_comma_parses_on_one_mebibyte_stack", || {
        let mut query = "0".to_owned();
        for _ in 0..256 {
            query = format!("1,({query})");
        }
        parse(&query).expect("nested grouped right comma query parses");
    });
}

#[test]
fn nested_containers_and_arguments_parse_on_one_mebibyte_stack() {
    bounded_parse(
        "nested_containers_and_arguments_parse_on_one_mebibyte_stack",
        || {
            for wrap in ["array", "object", "call", "index", "key"] {
                let mut query = "0".to_owned();
                for _ in 0..256 {
                    query = match wrap {
                        "array" => format!("[{query}]"),
                        "object" => format!("{{a: {query}}}"),
                        "call" => format!("map({query})"),
                        "index" => format!(".[{query}]"),
                        "key" => format!("{{({query}): 0}}"),
                        _ => unreachable!(),
                    };
                }
                parse(&query).expect("nested container query parses");
            }
        },
    );
}

#[test]
fn nested_control_bodies_parse_on_one_mebibyte_stack() {
    bounded_parse("nested_control_bodies_parse_on_one_mebibyte_stack", || {
        for wrap in ["if", "reduce", "foreach", "def", "try", "label", "bind"] {
            let mut query = "0".to_owned();
            for _ in 0..256 {
                query = match wrap {
                    "if" => format!("if true then {query} else 0 end"),
                    "reduce" => format!("reduce .[] as $x (0; {query})"),
                    "foreach" => format!("foreach .[] as $x (0; .; {query})"),
                    "def" => format!("def f: {query}; f"),
                    "try" => format!("try ({query}) catch 0"),
                    "label" => format!("label $x | {query}"),
                    "bind" => format!("0 as $x | {query}"),
                    _ => unreachable!(),
                };
            }
            parse(&query).expect("nested control query parses");
        }
    });
}

#[test]
fn nested_interpolation_parses_on_one_mebibyte_stack() {
    bounded_parse("nested_interpolation_parses_on_one_mebibyte_stack", || {
        for formatted in [false, true] {
            let mut query = "0".to_owned();
            for _ in 0..256 {
                query = format!("{}\"\\({query})\"", if formatted { "@json " } else { "" });
            }
            parse(&query).expect("nested interpolation query parses");
        }
    });
}

#[test]
fn binding_pattern_limit_is_preserved_on_one_mebibyte_stack() {
    bounded_parse(
        "binding_pattern_limit_is_preserved_on_one_mebibyte_stack",
        || {
            let accepted = format!("0 as {}$x{} | .", "[".repeat(255), "]".repeat(255));
            parse(&accepted).expect("255 nested containers retain the admitted variable depth");
            let rejected = format!("0 as {}$x{} | .", "[".repeat(256), "]".repeat(256));
            let error = parse(&rejected).expect_err("existing pattern limit remains enforced");
            assert_eq!(error.code, "TQ-RESOURCE-BIND-PATTERN-001");
        },
    );
}

#[test]
fn named_and_startup_sources_retain_nested_diagnostic_context() {
    let bytes = parse_bytes("named.jq", b"(.a, .b)[0]?").expect("named query parses");
    assert_eq!(bytes.source().name(), "named.jq");
    assert_eq!(
        bytes.hir(),
        "optional(access(comma(access(., field:a), access(., field:b)), index:0))"
    );
    let error = parse_with_startup(
        "query.jq",
        b".",
        "startup.jq",
        b"def f: if true then try (1 + ) end;\n",
    )
    .expect_err("malformed startup query reports its own source");
    assert!(
        error
            .labels
            .iter()
            .all(|label| label.span.source == SourceId::new(1))
    );
    assert!(
        error
            .labels
            .iter()
            .any(|label| label.message == "try expression")
    );
    assert!(
        error
            .labels
            .iter()
            .any(|label| label.message == "unterminated if expression")
    );
    let error = parse_bytes("numeric.jq", b"(1e999999999999999999999)")
        .expect_err("numeric range limit remains enforced");
    assert_eq!(error.code, "TQ-NUMBER-RANGE-001");
}

#[test]
fn nested_named_and_startup_queries_parse_on_one_mebibyte_stack() {
    bounded_parse(
        "nested_named_and_startup_queries_parse_on_one_mebibyte_stack",
        || {
            let mut body = "$__loc__".to_owned();
            for _ in 0..256 {
                body = format!("1,({body})");
            }
            let named =
                parse_bytes("named.jq", body.as_bytes()).expect("nested named query parses");
            assert_eq!(named.source().name(), "named.jq");
            drop(named);

            let startup = format!("def f: {body};\n");
            let parsed = parse_with_startup("query.jq", b"f", "startup.jq", startup.as_bytes())
                .expect("nested startup definition parses");
            assert_eq!(parsed.source().name(), "query.jq");
            let retained = parsed
                .source_by_id(SourceId::new(1))
                .expect("startup source retained");
            assert_eq!(retained.name(), "startup.jq");
            assert_eq!(retained.text(), startup);
        },
    );
}

#[test]
fn nested_startup_errors_retain_context_on_one_mebibyte_stack() {
    bounded_parse(
        "nested_startup_errors_retain_context_on_one_mebibyte_stack",
        || {
            let mut body = "try (1 + )".to_owned();
            for _ in 0..256 {
                body = format!("if true then {body} end");
            }
            let startup = format!("def f: {body};");
            let error = parse_with_startup("query.jq", b".", "startup.jq", startup.as_bytes())
                .expect_err("malformed nested startup query returns a diagnostic");
            assert_eq!(error.code, "TQ-PARSE-EXPRESSION-001");
            let primary = error
                .labels
                .iter()
                .find(|label| label.primary)
                .expect("primary source span");
            assert_eq!(
                usize::try_from(primary.span.start).expect("source offset fits usize"),
                startup.find(')').expect("malformed operand")
            );
            assert!(
                error
                    .labels
                    .iter()
                    .all(|label| label.span.source == SourceId::new(1))
            );
            assert_eq!(
                error
                    .labels
                    .iter()
                    .filter(|label| label.message == "unterminated if expression")
                    .count(),
                256
            );
            assert_eq!(
                error
                    .labels
                    .iter()
                    .filter(|label| label.message == "try expression")
                    .count(),
                1
            );
        },
    );
}

#[test]
fn completed_deep_subtrees_return_diagnostics_on_one_mebibyte_stack() {
    bounded_parse(
        "completed_deep_subtrees_return_diagnostics_on_one_mebibyte_stack",
        || {
            let depth = 16_384;
            let complete = format!("{}0{}", "[".repeat(depth), "]".repeat(depth));
            for (query, message, offset) in [
                (
                    format!("{complete})"),
                    "expected end of query",
                    complete.len(),
                ),
                (
                    format!("[{complete}"),
                    "expected ']' after array constructor",
                    complete.len() + 1,
                ),
                (
                    format!("{complete} + )"),
                    "expected filter expression",
                    complete.len() + 3,
                ),
            ] {
                let error = parse_bytes("deep.jq", query.as_bytes())
                    .expect_err("malformed query returns a diagnostic, not a stack overflow");
                assert_eq!(error.message, message);
                let primary = error
                    .labels
                    .iter()
                    .find(|label| label.primary)
                    .expect("primary diagnostic span");
                assert_eq!(primary.span.source, SourceId::new(0));
                assert_eq!(usize::try_from(primary.span.start).unwrap(), offset);
            }
        },
    );
}

#[test]
fn pending_deep_subtrees_return_original_diagnostics_on_one_mebibyte_stack() {
    bounded_parse(
        "pending_deep_subtrees_return_original_diagnostics_on_one_mebibyte_stack",
        || {
            let deep = format!("{}0{}", "[".repeat(16_384), "]".repeat(16_384));
            for (prefix, suffix) in [
                ("(", " + )"),
                ("if ", " )"),
                ("if true then ", " )"),
                ("if true then 0 else ", " )"),
                ("f(", "; )"),
                ("f(", "]"),
                ("{a: ", ", b: )}"),
                ("{a: ", "]"),
                ("{(", "): )}"),
                ("{(", ")}"),
                ("def f: ", "]"),
                ("def f: ", "; )"),
                ("reduce ", "]"),
                ("reduce .[] as $x (", "]"),
                ("reduce .[] as $x (0; ", "]"),
                ("foreach .[] as $x (0; .; ", "]"),
                ("try ", " catch )"),
                (".[", ": )]"),
                (".[:", ") ]"),
                ("\"\\(", " + )\""),
                ("\"\\(", ")\\())\""),
                ("include \"m\" ", "]"),
                ("module ", "; )"),
                ("", " as )"),
                ("", ".)"),
            ] {
                let small = format!("{prefix}[0]{suffix}");
                let baseline = parse_bytes("diagnostic.jq", small.as_bytes())
                    .expect_err("small malformed query");
                let query = format!("{prefix}{deep}{suffix}");
                let error = parse_bytes("diagnostic.jq", query.as_bytes())
                    .expect_err("deep malformed query returns diagnostic");
                assert_eq!(error.code, baseline.code, "{prefix}...{suffix}");
                assert_eq!(error.message, baseline.message, "{prefix}...{suffix}");
                assert_eq!(
                    error.labels.len(),
                    baseline.labels.len(),
                    "{prefix}...{suffix}"
                );
                let growth = u64::try_from(deep.len() - 3).unwrap();
                let boundary = u64::try_from(prefix.len() + 3).unwrap();
                for (actual, expected) in error.labels.iter().zip(&baseline.labels) {
                    let shift = |offset: u64| {
                        if offset >= boundary {
                            offset + growth
                        } else {
                            offset
                        }
                    };
                    assert_eq!(actual.span.source, expected.span.source);
                    assert_eq!(actual.span.start, shift(expected.span.start));
                    assert_eq!(actual.span.end, shift(expected.span.end));
                    assert_eq!(actual.primary, expected.primary);
                    if !actual.primary {
                        assert_eq!(actual.message, expected.message);
                    }
                }
            }
        },
    );
}

#[test]
fn completed_query_forms_dispose_deep_children_on_one_mebibyte_stack() {
    bounded_parse(
        "completed_query_forms_dispose_deep_children_on_one_mebibyte_stack",
        || {
            let deep = format!("{}0{}", "[".repeat(16_384), "]".repeat(16_384));
            for (prefix, suffix) in [
                ("", "?"),
                ("", ".a"),
                ("1 | ", ""),
                ("1, ", ""),
                ("-", ""),
                ("", " + 1"),
                (". = ", ""),
                ("if true then ", " else 0 end"),
                ("0 as $x | ", ""),
                ("0 as $x ?// $y | ", ""),
                ("label $x | ", ""),
                ("try ", " catch 0"),
                ("try 0 catch ", ""),
                ("f(", ")"),
                ("{a: ", "}"),
                ("{(", "): 0}"),
                ("def f: ", "; f"),
                ("include \"m\" ", "; ."),
                ("import \"m\" as m ", "; ."),
                ("module ", "; ."),
                (".[", "]"),
                (".[", ":]"),
                (".[:", "]"),
                ("\"\\(", ")\""),
                ("@json \"\\(", ")\""),
                ("reduce ", " as $x (0; .)"),
                ("reduce .[] as $x (", "; .)"),
                ("reduce .[] as $x (0; ", ")"),
                ("foreach .[] as $x (0; .; ", ")"),
            ] {
                let query = format!("{prefix}{deep}{suffix})");
                let error = parse(&query).expect_err(
                    "trailing token rejects a complete query and disposes every deep child",
                );
                assert_eq!(error.code, "TQ-PARSE-UNEXPECTED-001", "{prefix}...{suffix}");
                assert_eq!(
                    error.message, "expected end of query",
                    "{prefix}...{suffix}"
                );
                let primary = error
                    .labels
                    .iter()
                    .find(|label| label.primary)
                    .expect("trailing token diagnostic span");
                assert_eq!(
                    usize::try_from(primary.span.start).unwrap(),
                    query.len() - 1
                );
            }
        },
    );
}

#[test]
fn startup_failure_disposes_completed_query_on_one_mebibyte_stack() {
    bounded_parse(
        "startup_failure_disposes_completed_query_on_one_mebibyte_stack",
        || {
            let query = format!("{}0{}", "[".repeat(16_384), "]".repeat(16_384));
            for startup in ["(", "\"", "0", "def f: [0]; 0"] {
                let error = parse_with_startup(
                    "query.jq",
                    query.as_bytes(),
                    "startup.jq",
                    startup.as_bytes(),
                )
                .expect_err(
                    "startup failure returns a diagnostic while disposing the completed query",
                );
                assert_eq!(error.labels[0].span.source, SourceId::new(1));
            }
        },
    );
}

fn bounded_parse(name: &str, run: impl FnOnce() + Send + 'static) {
    if std::env::var_os(CHILD_ENV).is_some() {
        thread::Builder::new()
            .name("parser-one-mebibyte".to_owned())
            .stack_size(1024 * 1024)
            .spawn(run)
            .expect("parser thread starts")
            .join()
            .expect("parser thread succeeds");
        return;
    }
    let mut child = Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", name, "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, "1")
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("isolated parser child starts");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.try_wait().expect("parser child status") {
            assert!(status.success(), "parser child failed: {status}");
            return;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("parser child exceeded ten-second deadline");
        }
        thread::sleep(Duration::from_millis(25));
    }
}
