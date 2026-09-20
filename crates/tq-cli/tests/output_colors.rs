//! Cross-format black-box contracts for tq output presentation.

use std::io::Write;
use std::process::{Command, Output, Stdio};

use tq_cli::{CapabilityPolicy, CliError, parse_args_with_policy, run_with_io};

const DEFAULT_COLORS: &str = "0;39:0;94:0;94:0;35:0;32:0;90:0;90:0;36";
const CUSTOM_EIGHT: &str = "31:32:33:34:35:36:37:90";
const CUSTOM_SEVEN: &str = "31:32:33:34:35:36:37";
const ENVIRONMENT_DENIED_HELPER: &str = "TQ_OUTPUT_COLORS_ENVIRONMENT_DENIED_HELPER";

fn run(args: &[&str], input: &[u8], colors: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tq"));
    command
        .args(args)
        .env("NO_COLOR", "")
        .env_remove("HOME")
        .env_remove("JQ_LIBRARY_PATH");
    if let Some(colors) = colors {
        command.env("JQ_COLORS", colors);
    } else {
        command.env_remove("JQ_COLORS");
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tq color process");
    child
        .stdin
        .take()
        .expect("tq color stdin")
        .write_all(input)
        .expect("write tq color input");
    child.wait_with_output().expect("collect tq color output")
}

fn run_mode(args: &[&str], input: &[u8], colored: bool, colors: Option<&str>) -> Output {
    let flag = if colored { "-C" } else { "-M" };
    let mut all_args = Vec::with_capacity(args.len() + 1);
    all_args.push(flag);
    all_args.extend_from_slice(args);
    run(&all_args, input, colors)
}

fn strip_sgr(bytes: &[u8]) -> Vec<u8> {
    let mut stripped = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == 0x1b && bytes.get(index + 1) == Some(&b'[') {
            let mut end = index + 2;
            while end < bytes.len() && !(0x40..=0x7e).contains(&bytes[end]) {
                end += 1;
            }
            if bytes.get(end) == Some(&b'm') {
                index = end + 1;
                continue;
            }
        }
        stripped.push(bytes[index]);
        index += 1;
    }
    stripped
}

fn assert_colored_strips_to_plain(args: &[&str], input: &[u8], colors: Option<&str>) {
    let colored = run_mode(args, input, true, colors);
    let plain = run_mode(args, input, false, colors);
    assert!(
        colored.status.success(),
        "colored {args:?}: {}",
        String::from_utf8_lossy(&colored.stderr)
    );
    assert!(
        plain.status.success(),
        "plain {args:?}: {}",
        String::from_utf8_lossy(&plain.stderr)
    );
    assert!(
        colored.stdout.contains(&0x1b),
        "no color emitted for {args:?}"
    );
    assert_eq!(
        colored.status, plain.status,
        "colored/plain status mismatch"
    );
    assert_eq!(
        strip_sgr(&colored.stdout),
        plain.stdout,
        "generated SGR changed serialized bytes for {args:?}"
    );
}

#[test]
fn default_palette_uses_tq_slots_for_json_scalars_and_keys() {
    let output = run_mode(
        &["-i", "json", "-o", "json", "-c", "."],
        br#"{"key":{"number":1.2300,"true":true,"false":false,"null":null,"string":"x"}}"#,
        true,
        None,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output
            .stdout
            .windows(b"\x1b[0;39mnull".len())
            .any(|window| { window == b"\x1b[0;39mnull" })
    );
    assert!(
        output
            .stdout
            .windows(b"\x1b[0;94mtrue".len())
            .any(|window| { window == b"\x1b[0;94mtrue" })
    );
    assert!(
        output
            .stdout
            .windows(b"\x1b[0;94mfalse".len())
            .any(|window| { window == b"\x1b[0;94mfalse" })
    );
    assert!(
        output
            .stdout
            .windows(b"\x1b[0;35m1.2300".len())
            .any(|window| { window == b"\x1b[0;35m1.2300" })
    );
    assert!(
        output
            .stdout
            .windows(b"\x1b[0;32mx".len())
            .any(|window| window == b"\x1b[0;32mx")
    );
    assert!(
        output
            .stdout
            .windows(b"\x1b[0;36mkey".len())
            .any(|window| window == b"\x1b[0;36mkey")
    );
    assert_eq!(strip_sgr(&output.stdout), b"{\"key\":{\"number\":1.2300,\"true\":true,\"false\":false,\"null\":null,\"string\":\"x\"}}\n");
}

#[test]
fn custom_palette_fixtures_have_the_declared_jq_arity() {
    assert_eq!(CUSTOM_EIGHT.split(':').count(), 8);
    assert_eq!(CUSTOM_SEVEN.split(':').count(), 7);
}

#[test]
fn custom_eight_slot_palette_colors_quote_contexts_and_content_separately() {
    let input = br#"{"array":["inside"],"object":{"key":"inside"},"root":"outside"}"#;
    let output = run_mode(
        &["-i", "json", "-o", "json", "-c", "."],
        input,
        true,
        Some(CUSTOM_EIGHT),
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output
            .stdout
            .windows(b"\x1b[37m{\x1b[0m".len())
            .any(|window| { window == b"\x1b[37m{\x1b[0m" })
    );
    assert!(
        output
            .stdout
            .windows(b"\x1b[37m\"\x1b[0m\x1b[90marray\x1b[0m".len())
            .any(|window| { window == b"\x1b[37m\"\x1b[0m\x1b[90marray\x1b[0m" })
    );
    assert!(
        output
            .stdout
            .windows(b"\x1b[36m[\x1b[0m\x1b[36m\"\x1b[0m\x1b[35minside\x1b[0m".len())
            .any(|window| { window == b"\x1b[36m[\x1b[0m\x1b[36m\"\x1b[0m\x1b[35minside\x1b[0m" })
    );
    assert!(
        output
            .stdout
            .windows(b"\x1b[37m\"\x1b[0m\x1b[35moutside\x1b[0m".len())
            .any(|window| { window == b"\x1b[37m\"\x1b[0m\x1b[35moutside\x1b[0m" })
    );
    assert_eq!(
        strip_sgr(&output.stdout),
        [input.as_slice(), b"\n"].concat()
    );
}

#[test]
fn custom_seven_slot_palette_keeps_number_to_key_fallback() {
    let output = run_mode(
        &["-i", "json", "-o", "json", "-c", "."],
        br#"{"key":"value"}"#,
        true,
        Some(CUSTOM_SEVEN),
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output
            .stdout
            .windows(b"\x1b[34mkey".len())
            .any(|window| window == b"\x1b[34mkey")
    );
    assert!(
        output
            .stdout
            .windows(b"\x1b[35mvalue".len())
            .any(|window| window == b"\x1b[35mvalue")
    );
    assert_eq!(strip_sgr(&output.stdout), b"{\"key\":\"value\"}\n");
}

#[test]
fn every_native_output_format_strips_to_its_plain_bytes() {
    let cases = [
        (
            vec!["-i", "json", "-o", "json", "-c", "."],
            br#"{"name":"Ada","value":42}"#.as_slice(),
        ),
        (
            vec!["-i", "json", "-o", "yaml", "."],
            br#"{"name":"Ada","value":42}"#.as_slice(),
        ),
        (
            vec!["-i", "json", "-o", "toon", "."],
            br#"{"name":"Ada","value":42}"#.as_slice(),
        ),
        (
            vec!["-i", "jsonl", "-o", "jsonl", "-c", "."],
            b"{\"id\":1}\n{\"id\":2}\n".as_slice(),
        ),
        (
            vec!["-i", "json-seq", "-o", "json-seq", "-c", "."],
            b"\x1e{\"id\":1}\n\x1e{\"id\":2}\n".as_slice(),
        ),
        (
            vec!["-i", "json", "-o", "toon-seq", ".[]"],
            br#"[{"id":1},{"id":2}]"#.as_slice(),
        ),
        (
            vec!["-i", "json", "-o", "csv", ".[]"],
            br#"[{"name":"Ada","text":"42","number":42,"active":true,"missing":null}]"#.as_slice(),
        ),
        (
            vec!["-i", "json", "-o", "tsv", ".[]"],
            br#"[{"name":"Ada","text":"42","number":42,"active":true,"missing":null}]"#.as_slice(),
        ),
    ];
    for (args, input) in cases {
        assert_colored_strips_to_plain(&args, input, Some(DEFAULT_COLORS));
    }
}

#[test]
fn json_lines_and_json_sequence_reset_styles_at_frame_boundaries() {
    let jsonl_args = ["-i", "jsonl", "-o", "jsonl", "-c", "."];
    let jsonl = run_mode(&jsonl_args, b"{\"id\":1}\n{\"id\":2}\n", true, None);
    assert!(jsonl.status.success());
    assert_eq!(strip_sgr(&jsonl.stdout), b"{\"id\":1}\n{\"id\":2}\n");

    let sequence_args = ["-i", "json-seq", "-o", "json-seq", "-c", "."];
    let sequence = run_mode(
        &sequence_args,
        b"\x1e{\"id\":1}\n\x1e{\"id\":2}\n",
        true,
        None,
    );
    assert!(sequence.status.success());
    assert_eq!(
        strip_sgr(&sequence.stdout),
        b"\x1e{\"id\":1}\n\x1e{\"id\":2}\n"
    );
    assert!(
        sequence
            .stdout
            .windows(b"\x1b[0m\n\x1e".len())
            .any(|window| { window == b"\x1b[0m\n\x1e" })
    );
}

#[test]
fn streamed_event_results_use_every_native_writer_and_alias_with_custom_colors() {
    let input = br#"{"a":[null,true,42,"x"],"b":{"c":"q"}}"#;
    let query = "select(length == 2) | {path: (.[0] | tojson), value: (.[1] | tojson)}";
    for format in [
        "json", "jsonl", "ndjson", "json-seq", "jsonseq", "yaml", "toon", "toon-seq", "csv", "tsv",
    ] {
        let args = ["-i", "json", "--stream", "-o", format, query];
        assert_colored_strips_to_plain(&args, input, Some(CUSTOM_EIGHT));
    }
}

#[test]
fn standalone_quoted_scalars_use_object_style_for_quotes() {
    for format in ["json", "jsonl", "json-seq", "yaml", "toon", "toon-seq"] {
        let args = ["-n", "-o", format, r#""42""#];
        assert_colored_strips_to_plain(&args, b"", Some(CUSTOM_EIGHT));
        let output = run_mode(&args, b"", true, Some(CUSTOM_EIGHT));
        let quotes = if format == "yaml" {
            b"\x1b[37m'\x1b[0m".as_slice()
        } else {
            b"\x1b[37m\"\x1b[0m".as_slice()
        };
        for span in [quotes, b"\x1b[35m42\x1b[0m".as_slice()] {
            assert!(
                output.stdout.windows(span.len()).any(|bytes| bytes == span),
                "wrong root quote/content role for {format}"
            );
        }
    }
}

#[test]
fn late_errors_keep_prior_colored_results_without_changing_plain_bytes() {
    let args = ["-i", "jsonl", "-o", "jsonl", "-c", ".id, error(\"stop\")"];
    let colored = run_mode(&args, b"{\"id\":1}\n", true, None);
    let plain = run_mode(&args, b"{\"id\":1}\n", false, None);
    assert!(!colored.status.success());
    assert_eq!(colored.status, plain.status);
    assert_eq!(strip_sgr(&colored.stdout), plain.stdout);
    assert!(!colored.stdout.is_empty(), "the completed result was lost");
}

#[test]
fn output_limits_count_generated_sgr_bytes() {
    let args = ["-n", "-o", "json", "-c", "--max-output-bytes", "3", "42"];
    let plain = run_mode(&args, b"", false, None);
    assert!(
        plain.status.success(),
        "{}",
        String::from_utf8_lossy(&plain.stderr)
    );
    assert_eq!(plain.stdout, b"42\n");

    let colored = run_mode(&args, b"", true, None);
    assert!(!colored.status.success());
    assert!(String::from_utf8_lossy(&colored.stderr).contains("output"));
    assert!(colored.stdout.len() <= 3);
}

#[test]
fn unframed_multi_result_failure_publishes_no_colored_bytes() {
    let args = ["-n", "-o", "toon", "--unframed", "1,2"];
    let colored = run_mode(&args, b"", true, None);
    let plain = run_mode(&args, b"", false, None);
    assert!(!colored.status.success());
    assert_eq!(colored.status, plain.status);
    assert!(colored.stdout.is_empty());
    assert!(plain.stdout.is_empty());
}

#[test]
fn empty_result_has_no_presentation_bytes() {
    let output = run_mode(&["-n", "-o", "json", "-c", "empty"], b"", true, None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
}

#[test]
fn invalid_or_unset_palette_falls_back_in_json_and_yaml() {
    let cases = [
        (
            ["-i", "json", "-o", "json", "-c", "."].as_slice(),
            br#"{"x":1}"#.as_slice(),
        ),
        (
            ["-i", "json", "-o", "yaml", "."].as_slice(),
            br#"{"x":1}"#.as_slice(),
        ),
    ];
    for (args, input) in cases {
        let unset = run_mode(args, input, true, None);
        let invalid = run_mode(args, input, true, Some("not-a-jq-palette"));
        assert!(unset.status.success(), "unset palette: {unset:?}");
        assert!(invalid.status.success(), "invalid palette: {invalid:?}");
        assert_eq!(invalid.stdout, unset.stdout, "{args:?}");
        assert_eq!(
            strip_sgr(&unset.stdout),
            run_mode(args, input, false, None).stdout
        );
    }
}

#[test]
fn raw_formatted_strings_and_proxy_sources_remain_verbatim() {
    let raw_args = ["-i", "json", "-o", "json", "-r", "@json"];
    let raw_colored = run_mode(&raw_args, br#"{"x":1}"#, true, Some(CUSTOM_EIGHT));
    let raw_plain = run_mode(&raw_args, br#"{"x":1}"#, false, Some(CUSTOM_EIGHT));
    assert!(raw_colored.status.success());
    assert_eq!(raw_colored.stdout, raw_plain.stdout);
    assert_eq!(raw_colored.stdout, b"{\"x\":1}\n");

    let proxy_args = ["-x", "-i", "json", "."];
    let proxy = run_mode(
        proxy_args.as_slice(),
        b"not structured\n",
        true,
        Some(CUSTOM_EIGHT),
    );
    assert!(
        proxy.status.success(),
        "{}",
        String::from_utf8_lossy(&proxy.stderr)
    );
    assert_eq!(proxy.stdout, b"not structured\n");
    assert!(!proxy.stdout.contains(&0x1b));
}

#[test]
fn raw_non_strings_keep_compact_json_and_existing_separators_with_shared_colors() {
    for format in ["json", "yaml", "toon", "toon-seq", "json-seq"] {
        for (flag, separator) in [("-r", "\n"), ("-j", ""), ("--raw-output0", "\0")] {
            let args = [
                "-n",
                "-o",
                format,
                flag,
                r#""raw", {key: [1, true, null, "42"]}, "tail""#,
            ];
            let colored = run_mode(&args, b"", true, Some(CUSTOM_EIGHT));
            let plain = run_mode(&args, b"", false, Some(CUSTOM_EIGHT));
            assert!(
                colored.status.success(),
                "{args:?}: {}",
                String::from_utf8_lossy(&colored.stderr)
            );
            assert!(
                plain.status.success(),
                "{args:?}: {}",
                String::from_utf8_lossy(&plain.stderr)
            );
            let expected =
                format!("raw{separator}{{\"key\":[1,true,null,\"42\"]}}{separator}tail{separator}");
            assert_eq!(
                plain.stdout,
                expected.as_bytes(),
                "raw encoding changed for {args:?}"
            );
            assert_eq!(strip_sgr(&colored.stdout), plain.stdout, "{args:?}");
            assert!(
                colored
                    .stdout
                    .starts_with(format!("raw{separator}").as_bytes())
            );
            assert!(
                colored
                    .stdout
                    .ends_with(format!("tail{separator}").as_bytes())
            );
            for span in [
                b"\x1b[90mkey\x1b[0m".as_slice(),
                b"\x1b[34m1\x1b[0m".as_slice(),
                b"\x1b[33mtrue\x1b[0m".as_slice(),
            ] {
                assert!(
                    colored
                        .stdout
                        .windows(span.len())
                        .any(|bytes| bytes == span),
                    "missing raw fallback color for {args:?}"
                );
            }
        }
    }
}

#[test]
fn denied_terminal_capability_rejects_forced_color_before_input_for_all_formats() {
    let denied = CapabilityPolicy {
        terminal: false,
        ..CapabilityPolicy::default()
    };
    for format in [
        "json", "yaml", "toon", "jsonl", "json-seq", "toon-seq", "csv", "tsv",
    ] {
        assert!(
            matches!(
                parse_args_with_policy(["-C", "-o", format, "."], denied),
                Err(CliError::Incompatible(message)) if message.contains("terminal")
            ),
            "format {format} did not preserve terminal policy"
        );
    }
}

#[test]
fn automatic_pipe_output_is_plain_for_all_formats() {
    let cases: Vec<(Vec<&str>, &[u8])> = vec![
        (vec!["-i", "json", "-o", "json", "-c", "."], br#"{"x":1}"#),
        (vec!["-i", "json", "-o", "yaml", "."], br#"{"x":1}"#),
        (vec!["-i", "json", "-o", "toon", "."], br#"{"x":1}"#),
        (
            vec!["-i", "jsonl", "-o", "jsonl", "-c", "."],
            b"{\"id\":1}\n{\"id\":2}\n",
        ),
        (
            vec!["-i", "json-seq", "-o", "json-seq", "-c", "."],
            b"\x1e{\"id\":1}\n\x1e{\"id\":2}\n",
        ),
        (
            vec!["-i", "json", "-o", "toon-seq", ".[]"],
            br#"[{"id":1},{"id":2}]"#,
        ),
        (
            vec!["-i", "json", "-o", "csv", ".[]"],
            br#"[{"name":"Ada","value":42}]"#,
        ),
        (
            vec!["-i", "json", "-o", "tsv", ".[]"],
            br#"[{"name":"Ada","value":42}]"#,
        ),
    ];
    for (args, input) in cases {
        let output = run(&args, input, Some(CUSTOM_EIGHT));
        let plain = run_mode(&args, input, false, Some(CUSTOM_EIGHT));
        assert!(
            output.status.success(),
            "automatic output {:?}: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.status, plain.status, "automatic status {args:?}");
        assert_eq!(output.stdout, plain.stdout, "automatic bytes {args:?}");
        assert!(!output.stdout.contains(&0x1b), "automatic ANSI {args:?}");
    }
}

#[test]
fn environment_denial_ignores_jq_colors_for_embedded_runs() {
    if std::env::var_os(ENVIRONMENT_DENIED_HELPER).is_some() {
        let denied = CapabilityPolicy {
            environment: false,
            terminal: true,
            ..CapabilityPolicy::default()
        };
        let command = parse_args_with_policy(["-n", "-C", "-o", "json", "1"], denied)
            .expect("parse embedded color command");
        let mut input = std::io::empty();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        run_with_io(command, &mut input, &mut stdout, &mut stderr)
            .expect("run embedded color command");
        assert_eq!(stdout, b"\x1b[0;35m1\x1b[0m\n");
        assert!(stderr.is_empty(), "embedded stderr: {stderr:?}");
        return;
    }

    let output = Command::new(std::env::current_exe().expect("locate color test binary"))
        .args([
            "--exact",
            "environment_denial_ignores_jq_colors_for_embedded_runs",
            "--nocapture",
        ])
        .env(ENVIRONMENT_DENIED_HELPER, "1")
        .env("JQ_COLORS", CUSTOM_EIGHT)
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn environment-denied color helper");
    assert!(
        output.status.success(),
        "helper failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
