//! CLI fallback walk preserves jq cardinality, omission, and error ordering.

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
    .expect("walk query arguments parse");
    let mut input = Cursor::new(input.as_bytes());
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let status = run_with_io(command, &mut input, &mut stdout, &mut stderr);
    (status, stdout, stderr)
}

#[test]
fn walk_cli_collects_array_callback_outputs_without_cartesian_duplication() {
    let (status, stdout, stderr) = execute("walk((., .))", "[1]\n");
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(stdout, b"[1,1]\n[1,1]\n");
    assert!(stderr.is_empty());
}

#[test]
fn walk_cli_drops_empty_array_and_object_children() {
    let (status, stdout, stderr) = execute(
        "walk(if type == \"number\" then empty else . end)",
        "[1,{\"a\":2},\"x\"]\n",
    );
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(stdout, b"[{},\"x\"]\n");
    assert!(stderr.is_empty());
}

#[test]
fn walk_cli_stops_at_the_first_child_error() {
    let (status, stdout, stderr) = execute(
        "walk(if . == 1 then error(\"first\") elif . == 2 then error(\"later\") else . end)",
        "[1,2]\n",
    );
    assert_eq!(status.unwrap(), ExitStatus::Runtime);
    assert!(stdout.is_empty());
    assert_eq!(stderr, b"tq: runtime error: first\n");
}

#[test]
fn optional_repeat_cli_suppresses_the_tail_error_after_first_result() {
    let (status, stdout, stderr) = execute("[repeat(.*2, error)?]", "1\n");
    assert_eq!(status.unwrap(), ExitStatus::Success);
    assert_eq!(stdout, b"[2]\n");
    assert!(stderr.is_empty());
}
