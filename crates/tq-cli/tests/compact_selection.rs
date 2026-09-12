//! Process-level contracts for jq-shaped output selection.

use std::io::Write;
use std::process::{Command, Stdio};

struct Outcome {
    code: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn tq(arguments: &[&str], stdin: &[u8]) -> Outcome {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tq"));
    command.args(arguments);
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

fn tq_without_stdin(arguments: &[&str]) -> Outcome {
    let output = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(arguments)
        .output()
        .expect("run tq");
    Outcome {
        code: output.status.code().expect("ordinary exit"),
        stdout: output.stdout,
        stderr: output.stderr,
    }
}

#[test]
fn output_selection_matrix_preserves_toon_and_selects_json_compactness() {
    let input = b"{\"a\":1}\n";

    let default = tq(&["."], input);
    assert_eq!(
        default.code,
        0,
        "{}",
        String::from_utf8_lossy(&default.stderr)
    );
    assert_eq!(default.stdout, b"a: 1\n");

    let sequence = tq(&["--seq", "."], b"\x1e{\"a\":1}\n");
    assert_eq!(sequence.code, 0);
    assert_eq!(sequence.stdout, b"\x1ea: 1\n");

    let multiple_default = tq(&["."], b"1 2");
    assert_eq!(multiple_default.code, 0);
    assert_eq!(multiple_default.stdout, b"1\n2\n");

    let multiple_sequence = tq(&["--seq", "."], b"\x1e1\n\x1e2\n");
    assert_eq!(multiple_sequence.code, 0);
    assert_eq!(multiple_sequence.stdout, b"\x1e1\n\x1e2\n");

    let pretty = tq(&["-o", "json", "."], input);
    assert_eq!(
        pretty.code,
        0,
        "{}",
        String::from_utf8_lossy(&pretty.stderr)
    );
    assert_eq!(pretty.stdout, b"{\n  \"a\": 1\n}\n");

    for arguments in [
        &["-c", "."][..],
        &["--compact-output", "."][..],
        &["-cr", "."][..],
        &["-o", "json", "-c", "."][..],
        &["-c", "-o", "json", "."][..],
    ] {
        let compact = tq(arguments, input);
        assert_eq!(
            compact.code,
            0,
            "{arguments:?}: {}",
            String::from_utf8_lossy(&compact.stderr)
        );
        assert_eq!(compact.stdout, b"{\"a\":1}\n", "{arguments:?}");
    }

    let pretty_after_compact = tq(&["-c", "--pretty-output", "."], input);
    assert_eq!(pretty_after_compact.code, 0);
    assert_eq!(pretty_after_compact.stdout, b"{\n  \"a\": 1\n}\n");

    let compact_after_pretty = tq(&["--pretty-output", "-c", "."], input);
    assert_eq!(compact_after_pretty.code, 0);
    assert_eq!(compact_after_pretty.stdout, b"{\"a\":1}\n");

    let raw = tq(&["-cr", "-n", "\"x\""], b"");
    assert_eq!(raw.code, 0);
    assert_eq!(raw.stdout, b"x\n");

    let joined = tq(&["-cj", "-n", "\"a\",\"b\""], b"");
    assert_eq!(joined.code, 0);
    assert_eq!(joined.stdout, b"ab");

    let sorted_ascii = tq(&["-cSa", "-n", r#"{z:"µ",a:1}"#], b"");
    assert_eq!(sorted_ascii.code, 0);
    assert_eq!(sorted_ascii.stdout, b"{\"a\":1,\"z\":\"\\u00b5\"}\n");

    let json_lines = tq(&["-c", "-o", "jsonl", "."], input);
    assert_eq!(json_lines.code, 0);
    assert_eq!(json_lines.stdout, b"{\"a\":1}\n");
}

#[test]
fn explicit_toon_and_compact_output_conflict_before_input_consumption() {
    for arguments in [
        &[
            "-o",
            "toon",
            "-c",
            ".",
            "missing-input-that-must-not-be-opened",
        ][..],
        &[
            "-c",
            "-o",
            "toon",
            ".",
            "missing-input-that-must-not-be-opened",
        ][..],
        &[
            "-c",
            "-o",
            "toon",
            "--pretty-output",
            ".",
            "missing-input-that-must-not-be-opened",
        ][..],
        &[
            "-o",
            "toon",
            "-c",
            "--pretty-output",
            ".",
            "missing-input-that-must-not-be-opened",
        ][..],
        &[
            "-c",
            "--delimiter",
            "pipe",
            ".",
            "missing-input-that-must-not-be-opened",
        ][..],
    ] {
        let result = tq_without_stdin(arguments);
        assert_eq!(result.code, 2, "{arguments:?}");
        assert!(result.stdout.is_empty(), "{arguments:?}");
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(stderr.contains("incompatible"), "{arguments:?}: {stderr}");
        assert!(!stderr.contains("missing-input-that-must-not-be-opened"));
    }
}

#[test]
fn json_lines_rejects_incompatible_controls_before_opening_input() {
    for arguments in [
        &[
            "-o",
            "jsonl",
            "--pretty-output",
            ".",
            "missing-input-that-must-not-be-opened",
        ][..],
        &[
            "-o",
            "jsonl",
            "--indent",
            "4",
            ".",
            "missing-input-that-must-not-be-opened",
        ][..],
        &[
            "-o",
            "jsonl",
            "--tab",
            ".",
            "missing-input-that-must-not-be-opened",
        ][..],
        &[
            "-o",
            "jsonl",
            "-r",
            ".",
            "missing-input-that-must-not-be-opened",
        ][..],
        &[
            "-o",
            "jsonl",
            "-j",
            ".",
            "missing-input-that-must-not-be-opened",
        ][..],
        &[
            "-o",
            "jsonl",
            "-C",
            ".",
            "missing-input-that-must-not-be-opened",
        ][..],
    ] {
        let result = tq_without_stdin(arguments);
        assert_eq!(result.code, 2, "{arguments:?}");
        assert!(result.stdout.is_empty(), "{arguments:?}");
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(stderr.contains("incompatible"), "{arguments:?}: {stderr}");
        assert!(!stderr.contains("missing-input-that-must-not-be-opened"));
    }
}

#[test]
fn default_toon_preserves_jq_result_cardinality_and_prior_outputs() {
    for (query, input, expected, code) in [
        (".[] | select(. % 2 == 0)", "[1,2,3,4]", "2\n4\n", 0),
        ("empty", "null", "", 0),
        ("select(false)", "null", "", 0),
        ("1, error(\"stop\")", "null", "1\n", 5),
        (".[]", "[\"a\",\"b\"]", "a\nb\n", 0),
        (".[]", "[{\"a\":1},{\"b\":2}]", "a: 1\nb: 2\n", 0),
    ] {
        let result = tq(&[query], input.as_bytes());
        assert_eq!(
            result.code,
            code,
            "{query}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(result.stdout, expected.as_bytes(), "{query}");
    }
    assert_eq!(tq(&["-e", "empty"], b"null").code, 4);
    for query in ["empty", "1,2"] {
        let result = tq(&["--unframed", query], b"null");
        assert_ne!(result.code, 0);
        assert!(result.stdout.is_empty());
    }
}
