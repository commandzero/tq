//! Independently specified semantics for formerly stale shared-catalog contracts.

use std::{collections::BTreeMap, path::Path, time::Duration};

use serde_json::{Value, json};
use tq_cli::{ExitStatus, RunError, parse_args, run_with_io};

use tq_test_support::compatibility::{
    BaselinePolicy, CompatibilityCase, ContractKind, ErrorClass, ProcessOutcome, ProcessStatus,
    ToolKind, classify_process, load_catalog, normalize_jq, toon_values_match,
};

fn case(id: &str) -> CompatibilityCase {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    load_catalog(&root.join("tests/compatibility/cases"))
        .unwrap()
        .cases
        .into_iter()
        .find(|case| case.id == id)
        .unwrap_or_else(|| panic!("missing historical case {id}"))
}

fn execute(query: &str, input: &[u8], input_format: &str, output_format: &str) -> ProcessOutcome {
    let mut args = vec!["-i", input_format, "-o", output_format];
    if output_format == "json" {
        args.push("-c");
    }
    args.push(query);
    let command = parse_args(args).unwrap();
    let mut input = input;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let result = run_with_io(command, &mut input, &mut stdout, &mut stderr);
    let status = match result {
        Ok(status) => status,
        Err(error) => {
            if !matches!(error, RunError::ReportedRuntime(_)) {
                stderr.extend_from_slice(format!("tq: {error}\n").as_bytes());
            }
            error.status()
        }
    };
    ProcessOutcome {
        status: ProcessStatus::Exited,
        exit_code: Some(i32::from(status.code())),
        signal: None,
        stdout,
        stderr,
        wall_time_micros: 0,
        recorded_command: Vec::new(),
    }
}

fn assert_raised_prefix(
    id: &str,
    query: &str,
    values: &[Value],
    json_stdout: &[u8],
    toon_stdout: &[u8],
    caught_stdout: &[u8],
) {
    let case = case(id);
    assert_eq!(case.query, query);
    assert_eq!(case.fixture.inline.as_deref(), Some("null"));
    for input_format in ["json", "yaml", "toon"] {
        for output_format in ["json", "toon"] {
            let outcome = execute(&case.query, b"null", input_format, output_format);
            assert_eq!(
                outcome.exit_code,
                Some(i32::from(ExitStatus::Runtime.code()))
            );
            assert_eq!(outcome.stderr, b"tq: runtime error: boom\n");
            assert_eq!(
                classify_process(ToolKind::Tq, &outcome),
                Some(ErrorClass::RuntimeTypePath)
            );

            if output_format == "json" {
                assert_eq!(outcome.stdout, json_stdout);
                assert_eq!(normalize_jq(&outcome).unwrap().results, values);
            } else {
                assert_eq!(outcome.stdout, toon_stdout);
                assert!(toon_values_match(&outcome.stdout, values));
            }
        }
        let caught = execute(
            &format!("try ({query}) catch ."),
            b"null",
            input_format,
            "json",
        );
        assert_eq!(caught.exit_code, Some(0));
        assert_eq!(caught.stdout, caught_stdout);
        assert_eq!(caught.stderr, [] as [u8; 0]);
    }
    assert_eq!(case.expected.contract, ContractKind::Error);
    assert_eq!(
        case.expected.error_class.as_deref(),
        Some("runtime-type-path")
    );
    assert_eq!(case.expected.baseline, BaselinePolicy::Required);
    assert!(case.adapters.jq.supported && case.adapters.tq.supported);
}

#[test]
fn invalid_lvalue_catalog_requires_a_catchable_runtime_path_error() {
    let case = case("update.invalid-lvalue");
    assert_eq!(case.query, "(1 + 2) = 3");
    assert_eq!(case.fixture.inline.as_deref(), Some("null"));
    for input_format in ["json", "yaml", "toon"] {
        for output_format in ["json", "toon"] {
            let outcome = execute(&case.query, b"null", input_format, output_format);
            assert_eq!(outcome.exit_code, Some(5));
            assert_eq!(outcome.stdout, [] as [u8; 0]);
            assert_eq!(
                outcome.stderr,
                b"tq: runtime error: assignment left side is not a path\n"
            );
            assert_eq!(
                classify_process(ToolKind::Tq, &outcome),
                Some(ErrorClass::RuntimeTypePath)
            );
        }
        let caught = execute(
            &format!("try ({}) catch .", case.query),
            b"null",
            input_format,
            "json",
        );
        assert_eq!(caught.exit_code, Some(0));
        assert_eq!(caught.stdout, b"\"assignment left side is not a path\"\n");
        assert_eq!(caught.stderr, [] as [u8; 0]);
    }
    assert_eq!(case.expected.contract, ContractKind::Error);
    assert_eq!(
        case.expected.error_class.as_deref(),
        Some("runtime-type-path")
    );
    assert_eq!(case.expected.baseline, BaselinePolicy::Required);
    assert!(case.adapters.jq.supported && case.adapters.tq.supported);
    assert!(
        case.capabilities
            .iter()
            .any(|tag| tag == "error.runtime-type")
    );
    assert!(
        !case
            .capabilities
            .iter()
            .any(|tag| tag == "error.query-compile")
    );
}

#[test]
fn historical_lookaround_case_requires_real_positive_and_negative_matches() {
    let case = case("regex.unsupported-lookaround");
    assert_eq!(case.query, r#"test("(?=a)")"#);
    assert_eq!(case.fixture.inline.as_deref(), Some(r#""a""#));
    for (input, expected, stdout) in [
        (br#""a""#.as_slice(), true, b"true\n".as_slice()),
        (br#""b""#.as_slice(), false, b"false\n".as_slice()),
        (br#""ba""#.as_slice(), true, b"true\n".as_slice()),
    ] {
        for input_format in ["json", "yaml", "toon"] {
            for output_format in ["json", "toon"] {
                let outcome = execute(&case.query, input, input_format, output_format);
                assert_eq!(outcome.exit_code, Some(0));
                assert_eq!(outcome.stdout, stdout);
                assert_eq!(outcome.stderr, [] as [u8; 0]);
                assert_eq!(classify_process(ToolKind::Tq, &outcome), None);
                if output_format == "json" {
                    assert_eq!(normalize_jq(&outcome).unwrap().results, [json!(expected)]);
                } else {
                    assert!(toon_values_match(&outcome.stdout, &[json!(expected)]));
                }
            }
        }
    }
    assert_eq!(case.expected.contract, ContractKind::ResultSequence);
    assert_eq!(case.expected.error_class, None);
    assert_eq!(case.expected.baseline, BaselinePolicy::Required);
    assert!(case.adapters.jq.supported && case.adapters.tq.supported);
    assert!(
        case.capabilities
            .iter()
            .any(|tag| tag == "regex.lookaround")
    );
    assert!(
        !case
            .capabilities
            .iter()
            .any(|tag| tag == "regex.unsupported")
    );
    assert!(
        case.adapters
            .tq
            .note
            .as_deref()
            .unwrap()
            .contains("historical ID")
    );
}

#[test]
fn interpolation_catalog_keeps_the_completed_string_before_the_raised_error() {
    assert_raised_prefix(
        "interpolation.partial-error",
        r#""before=\(1,error("boom"),2)""#,
        &[json!("before=1")],
        b"\"before=1\"\n",
        b"before=1\n",
        b"\"before=1\"\n\"boom\"\n",
    );
}

#[test]
fn foreach_catalog_keeps_ordered_prefix_and_exact_raised_error() {
    assert_raised_prefix(
        "fold.foreach.partial-error",
        "foreach (1,2,error(\"boom\")) as $x (0; . + $x; .)",
        &[json!(1), json!(3)],
        b"1\n3\n",
        b"1\n3\n",
        b"1\n3\n\"boom\"\n",
    );
}

#[test]
#[ignore = "requires the reviewed matched-platform jq 1.8.2 reference"]
fn jq_reference_independently_verifies_the_corrected_catalog_contracts() {
    use tq_test_support::compatibility::{
        ExecutableConfig, Invocation, discover_tool, manual_host_target, run_process,
        validate_manual_reference,
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let executable = std::env::var_os("TQ_JQ").map_or_else(
        || {
            root.join(format!(
                "target/reference-build/jq/jq{}",
                std::env::consts::EXE_SUFFIX
            ))
        },
        |path| root.join(path),
    );
    let reference = discover_tool(
        ToolKind::Jq,
        &ExecutableConfig {
            jq: Some(executable),
            ..Default::default()
        },
        &root,
    )
    .unwrap()
    .unwrap();
    let pin = tq_test_support::fixture_data::read(
        &root.join("tests/compatibility/reviews/jq-manual/reference-pin.toon"),
    )
    .unwrap();
    validate_manual_reference(&pin, &reference, &manual_host_target()).unwrap();
    for (id, input, stdout, stderr, code) in [
        (
            "fold.foreach.partial-error",
            b"null".as_slice(),
            b"1\n3\n".as_slice(),
            b"jq: error (at <stdin>:0): boom\n".as_slice(),
            5,
        ),
        (
            "interpolation.partial-error",
            b"null".as_slice(),
            b"\"before=1\"\n".as_slice(),
            b"jq: error (at <stdin>:0): boom\n".as_slice(),
            5,
        ),
        (
            "regex.unsupported-lookaround",
            br#""a""#.as_slice(),
            b"true\n".as_slice(),
            b"".as_slice(),
            0,
        ),
        (
            "regex.unsupported-lookaround",
            br#""b""#.as_slice(),
            b"false\n".as_slice(),
            b"".as_slice(),
            0,
        ),
        (
            "regex.unsupported-lookaround",
            br#""ba""#.as_slice(),
            b"true\n".as_slice(),
            b"".as_slice(),
            0,
        ),
        (
            "update.invalid-lvalue",
            b"null".as_slice(),
            b"".as_slice(),
            b"jq: error (at <stdin>:0): Invalid path expression with result 3\n".as_slice(),
            5,
        ),
    ] {
        let case = case(id);
        let outcome = run_process(&Invocation {
            executable: reference.path.clone(),
            args: vec!["-c".into(), case.query],
            stdin: input.to_vec(),
            timeout: Duration::from_secs(3),
            current_dir: Some(root.clone()),
            environment: BTreeMap::new(),
        })
        .unwrap();
        assert_eq!(outcome.status, ProcessStatus::Exited, "{id}");
        assert_eq!(outcome.exit_code, Some(code), "{id}");
        assert_eq!(outcome.stdout, stdout, "{id}");
        assert_eq!(outcome.stderr, stderr, "{id}");
        assert_eq!(
            classify_process(ToolKind::Jq, &outcome),
            if code == 5 {
                Some(ErrorClass::RuntimeTypePath)
            } else {
                None
            },
            "{id}"
        );
    }
}
