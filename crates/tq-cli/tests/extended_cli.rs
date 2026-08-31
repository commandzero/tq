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
    assert_eq!(output.stdout, [] as [u8; 0]);
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
fn input_line_number_is_input_context_while_ambient_platform_data_remains_denied() {
    let line_number = tq(
        &[
            "--input-format",
            "json",
            "--output-format",
            "json",
            "-c",
            "input_line_number",
        ],
        b"1\n",
    );
    assert_eq!(
        line_number.code,
        0,
        "{}",
        String::from_utf8_lossy(&line_number.stderr)
    );
    assert_eq!(line_number.stdout, b"1\n");
    assert_eq!(line_number.stderr, [] as [u8; 0]);

    for query in ["input_filename", "now"] {
        let denied = tq(&["-ijson", "-ojson", "-c", query], b"1\n");
        assert_eq!(denied.code, 5, "{query}");
        assert_eq!(denied.stdout, [] as [u8; 0], "{query}");
        assert!(
            String::from_utf8_lossy(&denied.stderr).contains("capability policy"),
            "{query}: {}",
            String::from_utf8_lossy(&denied.stderr)
        );
    }
}

#[test]
fn regex_and_date_failures_keep_distinct_cli_experiences() {
    let unsupported = tq(
        &["--output-format", "json", "-c", r#"test("(?=a)")"#],
        b"\"a\"\n",
    );
    assert_eq!(unsupported.code, 2);
    assert_eq!(unsupported.stdout, [] as [u8; 0]);
    assert!(String::from_utf8_lossy(&unsupported.stderr).contains("not supported"));

    let range = tq(
        &["--output-format", "json", "-c", "todateiso8601"],
        b"253402300800\n",
    );
    assert_eq!(range.code, 5);
    assert_eq!(range.stdout, [] as [u8; 0]);
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
    assert_eq!(output.stderr, [] as [u8; 0]);

    let output = tq(&["-n", "--raw-output0", r#""a","b""#], b"");
    assert_eq!(output.code, 0);
    assert_eq!(output.stdout, b"a\0b\0");
    assert_eq!(output.stderr, [] as [u8; 0]);

    let output = tq(&["-n", "--raw-output0", r#""\u0000""#], b"");
    assert_eq!(output.code, 5);
    assert_eq!(output.stdout, [] as [u8; 0]);
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
    assert_eq!(output.stderr, [] as [u8; 0]);

    let output = tq(
        &["--output-format", "json", "-n", "--tab", r"{a:{b:1}}"],
        b"",
    );
    assert_eq!(output.code, 0);
    assert_eq!(output.stdout, b"{\n\t\"a\": {\n\t\t\"b\": 1\n\t}\n}\n");
    assert_eq!(output.stderr, [] as [u8; 0]);

    let output = tq(&["--output-format", "json", "-nCM", "1"], b"");
    assert_eq!(output.code, 0);
    assert_eq!(output.stdout, b"1\n");
    assert_eq!(output.stderr, [] as [u8; 0]);
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
    assert_eq!(output.stderr, [] as [u8; 0]);

    let output = tq(&["--output-format", "toon", "-a", "."], b"null");
    assert_eq!(output.code, 2);
    assert_eq!(output.stdout, [] as [u8; 0]);
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
    assert_eq!(output.stderr, [] as [u8; 0]);

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
    assert_eq!(output.stderr, [] as [u8; 0]);
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
    assert_eq!(output.stderr, [] as [u8; 0]);
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
fn inputs_consumes_one_shared_ordered_source_cursor() {
    let all = tq(&["-ijson", "-ojsonl", "[., inputs]"], b"1\n2\n3\n");
    assert_eq!(all.code, 0, "{}", String::from_utf8_lossy(&all.stderr));
    assert_eq!(all.stdout, b"[1,2,3]\n");

    let partial = tq(
        &["-ijson", "-ojsonl", "[., limit(1; inputs)]"],
        b"1\n2\n3\n",
    );
    assert_eq!(
        partial.code,
        0,
        "{}",
        String::from_utf8_lossy(&partial.stderr)
    );
    assert_eq!(partial.stdout, b"[1,2]\n[3]\n");

    let directory = tempdir().unwrap();
    let first = directory.path().join("first.json");
    let second = directory.path().join("second.json");
    fs::write(&first, "1\n").unwrap();
    fs::write(&second, "2\n3\n").unwrap();
    let files = tq(
        &[
            "-ojsonl",
            "[., inputs]",
            first.to_str().unwrap(),
            second.to_str().unwrap(),
        ],
        b"",
    );
    assert_eq!(files.code, 0, "{}", String::from_utf8_lossy(&files.stderr));
    assert_eq!(files.stdout, b"[1,2,3]\n");

    let metadata = tq(
        &[
            "--allow-platform",
            "-ojsonl",
            "[input_filename, (inputs | input_filename)]",
            first.to_str().unwrap(),
            second.to_str().unwrap(),
        ],
        b"",
    );
    assert_eq!(
        metadata.code,
        0,
        "{}",
        String::from_utf8_lossy(&metadata.stderr)
    );
    let filenames: serde_json::Value = serde_json::from_slice(&metadata.stdout).unwrap();
    assert_eq!(filenames[0], first.display().to_string());
    assert_eq!(filenames[1], second.display().to_string());

    let malformed = tq(&["-ijson", "-ojsonl", "[., limit(0; inputs)]"], b"1\n{\n");
    assert_ne!(malformed.code, 0);
    assert_eq!(malformed.stdout, b"[1]\n");
    assert!(
        String::from_utf8_lossy(&malformed.stderr).contains("input"),
        "{}",
        String::from_utf8_lossy(&malformed.stderr)
    );

    let byte_limited = tq(
        &[
            "-ijsonl",
            "-ojsonl",
            "--max-input-bytes",
            "3",
            "[., limit(0; inputs)]",
        ],
        b"1\n2\n",
    );
    assert_ne!(byte_limited.code, 0);
    assert_eq!(byte_limited.stdout, b"[1]\n");
    assert!(
        String::from_utf8_lossy(&byte_limited.stderr).contains("resource limit"),
        "{}",
        String::from_utf8_lossy(&byte_limited.stderr)
    );
}

#[test]
fn proxy_on_error_inputs_preserve_loaded_source_metadata() {
    let directory = tempdir().unwrap();
    let first = directory.path().join("first.json");
    let second = directory.path().join("second.json");
    fs::write(&first, "1\n").unwrap();
    fs::write(&second, "2\n").unwrap();

    let output = tq(
        &[
            "--proxy-on-error",
            "--allow-platform",
            "-ojsonl",
            "[input_filename, (inputs | input_filename)]",
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
    let filenames: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(filenames[0], first.display().to_string());
    assert_eq!(filenames[1], second.display().to_string());
}

#[test]
fn proxy_on_error_preserves_content_detected_automatic_event_plan() {
    let directory = tempdir().unwrap();
    let records = directory.path().join("records.data");
    fs::write(&records, "{\"values\":[1,2,3]}\n").unwrap();

    let output = tq(
        &[
            "-x",
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

    let stdin = tq(
        &["-x", "-ojsonl", "--explain-json", ".values[] | numbers"],
        b"{\"values\":[4,5]}\n",
    );
    assert_eq!(stdin.code, 0, "{}", String::from_utf8_lossy(&stdin.stderr));
    assert_eq!(stdin.stdout, b"4\n5\n");
    let explain: serde_json::Value = serde_json::from_slice(&stdin.stderr).unwrap();
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
    assert_eq!(output.stdout, [] as [u8; 0]);
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("line-bytes"));
    assert!(error.contains("line 1"));
    assert!(!error.contains("secret"));
}

#[test]
fn json_lines_token_limits_apply_to_document_event_and_subtree_plans() {
    let document = tq(
        &["-ijsonl", "-ojsonl", "--max-token-bytes", "3", ".id"],
        b"{\"id\":12345}\n",
    );
    assert_eq!(document.code, 5);
    assert_eq!(document.stdout, [] as [u8; 0]);
    assert!(String::from_utf8_lossy(&document.stderr).contains("token-bytes"));

    let streamed = tq(
        &[
            "--stream",
            "--stream-errors",
            "-ijsonl",
            "-ojsonl",
            "--max-token-bytes",
            "3",
            ".",
        ],
        b"12345\n",
    );
    assert_eq!(streamed.code, 5);
    assert_eq!(streamed.stdout, [] as [u8; 0]);
    assert!(String::from_utf8_lossy(&streamed.stderr).contains("token-bytes"));

    let event = tq(
        &[
            "-ijsonl",
            "-ojsonl",
            "--max-token-bytes",
            "3",
            ".v[] | numbers",
        ],
        b"{\"v\":[12345]}\n",
    );
    assert_eq!(event.code, 5);
    assert_eq!(event.stdout, [] as [u8; 0]);
    assert!(String::from_utf8_lossy(&event.stderr).contains("token-bytes"));

    let subtree = tq(
        &[
            "-ijsonl",
            "-ojsonl",
            "--max-token-bytes",
            "3",
            ".i[] | select(.a) | .v",
        ],
        b"{\"i\":[{\"a\":true,\"v\":12345}]}\n",
    );
    assert_eq!(subtree.code, 5);
    assert_eq!(subtree.stdout, [] as [u8; 0]);
    assert!(String::from_utf8_lossy(&subtree.stderr).contains("token-bytes"));
}

#[test]
fn json_lines_automatic_subtree_resets_between_records() {
    let output = tq(
        &[
            "-ijsonl",
            "-ojsonl",
            ".items[] | select(.active) | .id",
        ],
        b"{\"items\":[{\"id\":1,\"active\":true},{\"id\":2,\"active\":false}]}\n{\"items\":[{\"id\":3,\"active\":true}]}\n",
    );
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"1\n3\n");
    assert_eq!(output.stderr, [] as [u8; 0]);
}

#[test]
fn proxy_on_error_preserves_rejected_stdin_and_valid_transformations() {
    let invalid = b"{\"unfinished\":\xff\n";
    let proxied = tq(&["-ex", "-ijson"], invalid);
    assert_eq!(proxied.code, 0);
    assert_eq!(proxied.stdout, invalid);
    assert_eq!(proxied.stderr, [] as [u8; 0]);

    let invalid_auto = b"\xffunrecognized\n";
    let auto = tq(&["-x"], invalid_auto);
    assert_eq!(auto.code, 0);
    assert_eq!(auto.stdout, invalid_auto);
    assert_eq!(auto.stderr, [] as [u8; 0]);

    let transformed = tq(&["-x", "-ijson", "-ojsonl", ".value"], b"{\"value\":7}\n");
    assert_eq!(transformed.code, 0);
    assert_eq!(transformed.stdout, b"7\n");
    assert_eq!(transformed.stderr, [] as [u8; 0]);
}

#[test]
fn proxy_on_error_is_source_atomic_for_late_json_lines_failures() {
    let input = b"{\"id\":1}\nnot-json\n{\"id\":3}\n";
    let output = tq(&["-x", "-ijsonl", "-ojsonl", ".id"], input);
    assert_eq!(output.code, 0);
    assert_eq!(output.stdout, input);
    assert_eq!(output.stderr, [] as [u8; 0]);

    let streamed = tq(&["-x", "--stream", "-ijsonl"], input);
    assert_eq!(streamed.code, 0);
    assert_eq!(streamed.stdout, input);
    assert_eq!(streamed.stderr, [] as [u8; 0]);

    let slurped = tq(&["-x", "-s", "-ijsonl"], input);
    assert_eq!(slurped.code, 0);
    assert_eq!(slurped.stdout, input);
    assert_eq!(slurped.stderr, [] as [u8; 0]);
}

#[test]
fn proxy_on_error_preserves_file_order_and_only_proxies_rejected_sources() {
    let directory = tempdir().unwrap();
    let first = directory.path().join("first.json");
    let rejected = directory.path().join("rejected.json");
    let last = directory.path().join("last.json");
    fs::write(&first, "{\"id\":1}\n").unwrap();
    fs::write(&rejected, "not-json\n").unwrap();
    fs::write(&last, "{\"id\":3}\n").unwrap();

    let output = tq(
        &[
            "-x",
            "-ijson",
            "-ojsonl",
            ".id",
            first.to_str().unwrap(),
            rejected.to_str().unwrap(),
            last.to_str().unwrap(),
        ],
        b"",
    );
    assert_eq!(output.code, 0);
    assert_eq!(output.stdout, b"1\nnot-json\n3\n");
    assert_eq!(output.stderr, [] as [u8; 0]);
}

#[test]
fn proxy_on_error_preserves_unframed_toon_cardinality() {
    let directory = tempdir().unwrap();
    let valid = directory.path().join("valid.json");
    let rejected = directory.path().join("rejected.json");
    let also_rejected = directory.path().join("also-rejected.json");
    fs::write(&valid, "{\"id\":1}\n").unwrap();
    fs::write(&rejected, "not-json\n").unwrap();
    fs::write(&also_rejected, "still-not-json\n").unwrap();

    let mixed = tq(
        &[
            "-x",
            "-ijson",
            "--unframed",
            ".",
            valid.to_str().unwrap(),
            rejected.to_str().unwrap(),
        ],
        b"",
    );
    assert_eq!(mixed.code, 5);
    assert_eq!(mixed.stdout, [] as [u8; 0]);
    assert!(String::from_utf8_lossy(&mixed.stderr).contains("multiple"));

    let proxied = tq(
        &[
            "-x",
            "-ijson",
            "--unframed",
            ".",
            rejected.to_str().unwrap(),
            also_rejected.to_str().unwrap(),
        ],
        b"",
    );
    assert_eq!(proxied.code, 5);
    assert_eq!(proxied.stdout, b"not-json\n");
    assert!(String::from_utf8_lossy(&proxied.stderr).contains("multiple"));
}

#[test]
fn proxy_on_error_does_not_mask_resource_or_runtime_errors() {
    let resource = tq(&["-x", "-ijson", "--max-input-bytes", "3"], b"null");
    assert_eq!(resource.code, 5);
    assert_eq!(resource.stdout, [] as [u8; 0]);
    assert!(String::from_utf8_lossy(&resource.stderr).contains("input-bytes"));

    let output_limit = tq(&["-x", "-ijson", "--max-output-bytes", "3"], b"invalid");
    assert_eq!(output_limit.code, 5);
    assert_eq!(output_limit.stdout, [] as [u8; 0]);
    assert!(String::from_utf8_lossy(&output_limit.stderr).contains("output-bytes"));

    let runtime = tq(&["-x", "-ijson", "error(\"boom\")"], b"null\n");
    assert_ne!(runtime.code, 0);
    assert_eq!(runtime.stdout, [] as [u8; 0]);
    assert!(String::from_utf8_lossy(&runtime.stderr).contains("boom"));
}

#[test]
fn generated_help_and_build_configuration_are_stdout_only() {
    for arguments in [&["--help"][..], &["--build-configuration"][..]] {
        let output = tq(arguments, b"");
        assert_eq!(output.code, 0);
        assert_ne!(output.stdout, [] as [u8; 0]);
        assert_eq!(output.stderr, [] as [u8; 0]);
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
    assert_eq!(output.stderr, [] as [u8; 0]);
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
    assert_eq!(output.stdout, [] as [u8; 0]);
    assert!(String::from_utf8_lossy(&output.stderr).contains("UTF-8"));

    let output = tq(&["-n", "--argjson", "value", "1 2", "$value"], b"");
    assert_eq!(output.code, 5);
    assert_eq!(output.stdout, [] as [u8; 0]);
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

#[test]
fn sort_by_accepts_a_comma_generator_as_one_argument() {
    let output = tq(
        &[
            "--input-format",
            "json",
            "--output-format",
            "json",
            "-c",
            "sort_by(.a,.b)",
        ],
        br#"[{"a":1,"b":2},{"a":1,"b":1}]
"#,
    );
    assert_eq!(output.code, 0);
    assert_eq!(
        output.stdout,
        br#"[{"a":1,"b":1},{"a":1,"b":2}]
"#
    );
    assert!(output.stderr.is_empty());
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
    assert_eq!(output.stderr, [] as [u8; 0]);
}
