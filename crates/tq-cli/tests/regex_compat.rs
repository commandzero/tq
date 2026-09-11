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
fn scan_cli_keeps_single_capture_arrays() {
    let (status, stdout, stderr) = execute(r#"[scan("(.)")]"#, r#""ab""#);
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(stdout, b"[[\"a\"],[\"b\"]]\n");
    assert!(stderr.is_empty());
}
