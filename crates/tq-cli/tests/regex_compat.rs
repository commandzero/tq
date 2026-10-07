//! CLI regex cardinality and overload boundaries.

use std::io::Cursor;

use tq_cli::{ExitStatus, RunError, parse_args, run_with_io};

fn execute(query: &str, input: &str) -> (Result<ExitStatus, RunError>, Vec<u8>, Vec<u8>) {
    let command = parse_args([
        "--input-format",
        "json",
        "--output-format",
        "json",
        "--compact-output",
        query,
    ])
    .expect("regex query arguments parse");
    let mut input = Cursor::new(input.as_bytes());
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let status = run_with_io(command, &mut input, &mut stdout, &mut stderr);
    (status, stdout, stderr)
}

#[test]
fn longest_match_cli_matches_jq_compact_output() {
    let (status, stdout, stderr) = execute(r#"match("a|ab"; "l")"#, r#""ab""#);
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(
        stdout,
        b"{\"offset\":0,\"length\":2,\"string\":\"ab\",\"captures\":[]}\n"
    );
    assert_eq!(stderr, [] as [u8; 0]);
}

#[test]
fn longest_match_cli_accepts_late_literal_with_default_budget() {
    let input = format!("\"{}a{}\"", "b".repeat(900), "b".repeat(99));
    let (status, stdout, stderr) = execute(r#"match("a"; "l")"#, &input);
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(
        stdout,
        b"{\"offset\":900,\"length\":1,\"string\":\"a\",\"captures\":[]}\n"
    );
    assert_eq!(stderr, [] as [u8; 0]);
}

#[test]
fn longest_match_cli_preserves_unicode_byte_ties_across_global_pulls() {
    let (status, stdout, stderr) = execute(
        r#"[match("a|é|aa"; "lg") | [.offset, .string]]"#,
        r#""aéaa""#,
    );
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(stdout, "[[1,\"é\"],[2,\"aa\"]]\n".as_bytes());
    assert_eq!(stderr, [] as [u8; 0]);
}

#[test]
fn scan_cli_keeps_single_capture_arrays() {
    let (status, stdout, stderr) = execute(r#"[scan("(.)")]"#, r#""ab""#);
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(stdout, b"[[\"a\"],[\"b\"]]\n");
    assert_eq!(stderr, [] as [u8; 0]);
}
