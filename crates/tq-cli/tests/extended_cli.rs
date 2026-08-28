//! Executable jq-shaped CLI baselines: argv, input sources, output bytes,
//! diagnostics, and exit status are all observable.

use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
};

use tempfile::tempdir;

struct Outcome {
    code: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn tq(arguments: &[&str], stdin: &[u8]) -> Outcome {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tq"));
    command.args(arguments);
    run_tq(command, stdin)
}

fn tq_with_environment(arguments: &[&str], stdin: &[u8], key: &str, value: &str) -> Outcome {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tq"));
    command.args(arguments).env(key, value);
    run_tq(command, stdin)
}

fn run_tq(mut command: Command, stdin: &[u8]) -> Outcome {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tq");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin)
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait for tq");
    Outcome {
        code: output.status.code().expect("ordinary exit"),
        stdout: output.stdout,
        stderr: output.stderr,
    }
}

#[test]
fn denied_ambient_access_redacts_diagnostics_and_report_observations() {
    use tq_test_support::compatibility::{
        ErrorClass, FixtureFormat, ObservationState, ProcessStatus, ToolKind, ToolObservation,
        encode_hex,
    };

    const SENTINEL: &str = "tq-redaction-sentinel-never-report-this";
    let output = tq_with_environment(
        &["--output-format", "json", "-c", "env"],
        b"null\n",
        "TQ_REDACTION_SENTINEL",
        SENTINEL,
    );
    assert_eq!(output.code, 5);
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("capability policy"));
    assert!(
        !output
            .stderr
            .windows(SENTINEL.len())
            .any(|bytes| bytes == SENTINEL.as_bytes())
    );

    let observation = ToolObservation {
        tool: ToolKind::Tq,
        input_format: Some(FixtureFormat::Json),
        state: ObservationState::Executed,
        results: Vec::new(),
        stdout_hex: Some(encode_hex(&output.stdout)),
        raw_stdout_hex: None,
        stderr_hex: Some(encode_hex(&output.stderr)),
        process_status: Some(ProcessStatus::Exited),
        exit_code: Some(output.code),
        error_class: Some(ErrorClass::RuntimePolicy),
        wall_time_micros: Some(0),
        note: None,
    };
    let report_bytes = serde_json::to_vec(&observation).expect("compatibility observation JSON");
    assert!(
        !report_bytes
            .windows(SENTINEL.len())
            .any(|bytes| bytes == SENTINEL.as_bytes())
    );
}

#[test]
fn regex_and_date_failures_keep_distinct_cli_experiences() {
    let unsupported = tq(
        &["--output-format", "json", "-c", r#"test("(?=a)")"#],
        b"\"a\"\n",
    );
    assert_eq!(unsupported.code, 2);
    assert!(unsupported.stdout.is_empty());
    assert!(String::from_utf8_lossy(&unsupported.stderr).contains("not supported"));

    let range = tq(
        &["--output-format", "json", "-c", "todateiso8601"],
        b"253402300800\n",
    );
    assert_eq!(range.code, 5);
    assert!(range.stdout.is_empty());
    assert!(String::from_utf8_lossy(&range.stderr).contains("numeric range error"));
}

#[test]
fn short_clusters_sort_ascii_indent_and_raw_zero_have_stable_bytes() {
    let output = tq(&["--output-format", "json", "-nacS", r#"{z:"µ",a:1}"#], b"");
    assert_eq!(output.code, 0);
    assert_eq!(
        output.stdout,
        br#"{"a":1,"z":"\u00b5"}
"#
    );
    assert!(output.stderr.is_empty());

    let output = tq(&["-n", "--raw-output0", r#""a","b""#], b"");
    assert_eq!(output.code, 0);
    assert_eq!(output.stdout, b"a\0b\0");
    assert!(output.stderr.is_empty());

    let output = tq(&["-n", "--raw-output0", r#""\u0000""#], b"");
    assert_eq!(output.code, 5);
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("NUL"));
}

#[test]
fn indentation_and_monochrome_controls_have_stable_output_bytes() {
    let output = tq(
        &[
            "--output-format",
            "json",
            "-n",
            "--indent",
            "4",
            r"{a:{b:1}}",
        ],
        b"",
    );
    assert_eq!(output.code, 0);
    assert_eq!(
        output.stdout,
        b"{\n    \"a\": {\n        \"b\": 1\n    }\n}\n"
    );
    assert!(output.stderr.is_empty());

    let output = tq(
        &["--output-format", "json", "-n", "--tab", r"{a:{b:1}}"],
        b"",
    );
    assert_eq!(output.code, 0);
    assert_eq!(output.stdout, b"{\n\t\"a\": {\n\t\t\"b\": 1\n\t}\n}\n");
    assert!(output.stderr.is_empty());

    let output = tq(&["--output-format", "json", "-nCM", "1"], b"");
    assert_eq!(output.code, 0);
    assert_eq!(output.stdout, b"1\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn named_positional_and_file_arguments_are_bounded_and_ordered() {
    let directory = tempdir().unwrap();
    let raw = directory.path().join("raw.txt");
    let json = directory.path().join("values.json");
    fs::write(&raw, "hello\n").unwrap();
    fs::write(&json, "1\n2\n").unwrap();

    let output = tq(
        &[
            "--output-format",
            "json",
            "-nc",
            "--arg",
            "name",
            "Ada",
            "--rawfile",
            "raw",
            raw.to_str().unwrap(),
            "--slurpfile",
            "values",
            json.to_str().unwrap(),
            "[$name,$ARGS.named.name,$raw,$values]",
        ],
        b"",
    );
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"[\"Ada\",\"Ada\",\"hello\\n\",[1,2]]\n");

    let output = tq(
        &[
            "--output-format",
            "json",
            "-nc",
            "--args",
            "$ARGS.positional",
            "x",
            "2",
        ],
        b"",
    );
    assert_eq!(output.code, 0);
    assert_eq!(output.stdout, b"[\"x\",\"2\"]\n");

    let output = tq(
        &[
            "--output-format",
            "json",
            "-nc",
            "--jsonargs",
            "$ARGS.positional",
            "1",
            "{\"x\":2}",
        ],
        b"",
    );
    assert_eq!(output.code, 0);
    assert_eq!(output.stdout, b"[1,{\"x\":2}]\n");
}

#[test]
fn yaml_multi_file_and_invalid_combinations_preserve_channels_and_status() {
    let directory = tempdir().unwrap();
    let first = directory.path().join("first.json");
    let second = directory.path().join("second.json");
    fs::write(&first, "{\"n\":1}").unwrap();
    fs::write(&second, "{\"n\":2}").unwrap();

    let output = tq(
        &[
            "--input-format",
            "json",
            "--output-format",
            "yaml",
            ".n",
            first.to_str().unwrap(),
            second.to_str().unwrap(),
        ],
        b"",
    );
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"1\n---\n2\n");
    assert!(output.stderr.is_empty());

    let output = tq(&["--output-format", "toon", "-a", "."], b"null");
    assert_eq!(output.code, 2);
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("incompatible"));
}

#[test]
fn yaml_extension_selects_yaml_input_and_short_formats_emit_block_yaml() {
    let directory = tempdir().unwrap();
    let source = directory.path().join("source.yaml");
    fs::write(
        &source,
        "name: Ada\nmetadata:\n  active: true\nitems:\n  - one\n  - two\n",
    )
    .unwrap();

    let output = tq(&["-oyaml", ".", source.to_str().unwrap()], b"");
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout,
        b"name: Ada\nmetadata:\n  active: true\nitems:\n  - one\n  - two\n"
    );
    assert!(output.stderr.is_empty());

    let output = tq(
        &[
            "-iyaml",
            "-o",
            "json",
            "-c",
            ".name",
            source.to_str().unwrap(),
        ],
        b"",
    );
    assert_eq!(output.code, 0);
    assert_eq!(output.stdout, b"\"Ada\"\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn json_lines_aliases_preserve_records_blank_lines_and_final_eof() {
    let output = tq(
        &["-indjson", "-ojsonl", "."],
        b"{\"n\":9007199254740993}\n\ntrue\n[1,2]",
    );
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"{\"n\":9007199254740993}\ntrue\n[1,2]\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn json_lines_extension_override_and_mixed_files_are_ordered() {
    let directory = tempdir().unwrap();
    let records = directory.path().join("records.JSONL");
    let yaml = directory.path().join("last.yaml");
    fs::write(&records, "{\"id\":1}\n{\"id\":2}\n").unwrap();
    fs::write(&yaml, "id: 3\n").unwrap();

    let output = tq(
        &[
            "-ojsonl",
            ".id",
            records.to_str().unwrap(),
            yaml.to_str().unwrap(),
        ],
        b"",
    );
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"1\n2\n3\n");

    let pretty_json = directory.path().join("pretty.jsonl");
    fs::write(&pretty_json, "{\n  \"id\": 4\n}\n").unwrap();
    let inferred = tq(&["-ojsonl", ".", pretty_json.to_str().unwrap()], b"");
    assert_ne!(inferred.code, 0);
    assert!(String::from_utf8_lossy(&inferred.stderr).contains("pretty.jsonl:1"));

    let overridden = tq(
        &["-ijson", "-ojsonl", ".id", pretty_json.to_str().unwrap()],
        b"",
    );
    assert_eq!(overridden.code, 0);
    assert_eq!(overridden.stdout, b"4\n");
}

#[test]
fn ndjson_extension_enables_automatic_event_plan() {
    let directory = tempdir().unwrap();
    let records = directory.path().join("records.ndjson");
    fs::write(&records, "{\"values\":[1,2]}\n{\"values\":[3]}\n").unwrap();

    let output = tq(
        &[
            "-ojsonl",
            "--explain-json",
            ".values[] | numbers",
            records.to_str().unwrap(),
        ],
        b"",
    );
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"1\n2\n3\n");
    let explain: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(explain["execution"]["plan"], "events");
}

#[test]
fn json_lines_stream_resets_roots_and_late_errors_keep_prior_output() {
    let streamed = tq(
        &["--stream", "-ijsonl", "-ojsonl", "."],
        b"{\"a\":1}\n{\"b\":2}\n",
    );
    assert_eq!(
        streamed.code,
        0,
        "{}",
        String::from_utf8_lossy(&streamed.stderr)
    );
    assert_eq!(
        streamed.stdout,
        b"[[\"a\"],1]\n[[\"a\"]]\n[[\"b\"],2]\n[[\"b\"]]\n"
    );

    let failed = tq(
        &["-ijsonl", "-ojsonl", ".id"],
        b"{\"id\":1}\nnot-json\n{\"id\":3}\n",
    );
    assert_ne!(failed.code, 0);
    assert_eq!(failed.stdout, b"1\n");
    let error = String::from_utf8_lossy(&failed.stderr);
    assert!(error.contains("<stdin>:2"), "{error}");
    assert!(!error.contains("not-json"));
}

#[test]
fn json_lines_line_limit_is_a_resource_error_without_record_disclosure() {
    let output = tq(
        &["-ijsonl", "-ojsonl", "--max-line-bytes", "8", "."],
        b"{\"secret\":123}\n",
    );
    assert_eq!(output.code, 5);
    assert!(output.stdout.is_empty());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("line-bytes"));
    assert!(error.contains("line 1"));
    assert!(!error.contains("secret"));
}

#[test]
fn generated_help_and_build_configuration_are_stdout_only() {
    for arguments in [&["--help"][..], &["--build-configuration"][..]] {
        let output = tq(arguments, b"");
        assert_eq!(output.code, 0);
        assert!(!output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
    let help = tq(&["--help"], b"");
    let help = String::from_utf8(help.stdout).unwrap();
    for option in ["--raw-output0", "--sort-keys", "--slurpfile", "--jsonargs"] {
        assert!(help.contains(option), "missing {option}");
    }
}

#[test]
fn argument_free_invocation_uses_identity_filter_over_stdin() {
    let output = tq(&[], b"name: Ada\nactive: true\n");
    assert_eq!(output.code, 0);
    assert_eq!(output.stdout, b"\x1ename: Ada\nactive: true\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn color_binary_encoding_and_argument_file_limits_are_classified() {
    let output = tq(&["--output-format", "json", "-nC", "1"], b"");
    assert_eq!(output.code, 0);
    assert_eq!(output.stdout, b"\x1b[36m1\x1b[0m\n");

    let output = tq(&["--output-format", "json", "-nbc", "1"], b"");
    assert_eq!(output.code, 0);
    assert_eq!(output.stdout, b"1\n");

    let output = tq(&["-R", "."], &[0xff]);
    assert_eq!(output.code, 5);
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("UTF-8"));

    let output = tq(&["-n", "--argjson", "value", "1 2", "$value"], b"");
    assert_eq!(output.code, 5);
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("exactly one"));

    let directory = tempdir().unwrap();
    let secret = directory.path().join("secret.txt");
    fs::write(&secret, "do-not-leak").unwrap();
    let output = tq(
        &[
            "-n",
            "--max-input-bytes",
            "2",
            "--rawfile",
            "value",
            secret.to_str().unwrap(),
            "$value",
        ],
        b"",
    );
    assert_eq!(output.code, 5);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("resource limit"));
    assert!(stderr.contains(secret.to_str().unwrap()));
    assert!(!stderr.contains("do-not-leak"));
}

#[cfg(unix)]
#[test]
fn ordered_files_preserve_a_complete_frame_before_a_closed_downstream_pipe() {
    use std::io::Read;

    let directory = tempdir().unwrap();
    let first = directory.path().join("first.json");
    let second = directory.path().join("second.json");
    fs::write(&first, "{\"n\":1}").unwrap();
    fs::write(&second, "{\"n\":2}").unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args([
            "--input-format",
            "json",
            "--output-format",
            "json",
            "-c",
            ".n, range(0; 100000000)",
            first.to_str().unwrap(),
            second.to_str().unwrap(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tq");
    let mut stdout = child.stdout.take().unwrap();
    let mut complete_first_frame = [0_u8; 2];
    stdout.read_exact(&mut complete_first_frame).unwrap();
    assert_eq!(&complete_first_frame, b"1\n");
    drop(stdout);
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
}
