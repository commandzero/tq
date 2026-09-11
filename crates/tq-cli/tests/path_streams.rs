//! CLI path generators preserve filters, cardinality, and earlier output.

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
    .expect("path query arguments parse");
    let mut input = Cursor::new(input.as_bytes());
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let status = run_with_io(command, &mut input, &mut stdout, &mut stderr);
    (status, stdout, stderr)
}

#[test]
fn paths_filter_is_applied_in_the_cli_execution_path() {
    let (status, stdout, stderr) = execute("paths(. == 2)", "[1,2]\n");
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(stdout, b"[1]\n");
    assert!(stderr.is_empty());
}

#[test]
fn paths_filter_keeps_prior_results_before_a_later_error() {
    let (status, stdout, _stderr) = execute(
        "paths(if . == 2 then error(\"later\") else true end)",
        "[1,2]\n",
    );
    assert_eq!(status.unwrap_err().status(), ExitStatus::Runtime);
    assert_eq!(stdout, b"[0]\n");
}

#[test]
fn path_keeps_prior_paths_before_a_later_error() {
    let (status, stdout, _stderr) = execute("path(.[0],error(\"later\"))", "[1]\n");
    assert_eq!(status.unwrap_err().status(), ExitStatus::Runtime);
    assert_eq!(stdout, b"[0]\n");
}

#[test]
fn delpaths_keeps_prior_documents_before_a_later_error() {
    let (status, stdout, _stderr) = execute("delpaths([[0]],error(\"later\"))", "[1,2]\n");
    assert_eq!(status.unwrap_err().status(), ExitStatus::Runtime);
    assert_eq!(stdout, b"[2]\n");
}
