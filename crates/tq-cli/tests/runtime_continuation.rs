//! Process-level diagnostics for recoverable document-root runtime failures.

use std::{
    io::{Cursor, Write},
    process::{Child, Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

use tq_cli::{ExitStatus, RunError, parse_args, run_with_io};
use tq_core::VmError;

fn run_with_deadline(args: &[&str], input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tq");
    child
        .stdin
        .take()
        .expect("stdin pipe")
        .write_all(input)
        .expect("write input");
    wait_with_deadline(child, Duration::from_secs(2))
}

fn wait_with_deadline(mut child: Child, timeout: Duration) -> Output {
    let deadline = Instant::now() + timeout;
    loop {
        if child.try_wait().expect("poll tq").is_some() {
            return child.wait_with_output().expect("collect tq output");
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("tq did not terminate before the deadline");
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn recoverable_runtime_and_debug_diagnostics_keep_root_order() {
    let output = run_with_deadline(
        &[
            "-ijson",
            "-ojson",
            "-c",
            "if . == 1 then error(\"first\") elif . == 2 then debug(\"middle\") else error(\"last\") end",
        ],
        b"1\n2\n3\n",
    );

    assert_eq!(output.status.code(), Some(5));
    assert_eq!(output.stdout, b"2\n");
    assert_eq!(
        output.stderr,
        b"tq: runtime error: first\n[\"DEBUG:\",\"middle\"]\ntq: runtime error: last\n"
    );
}

#[test]
fn recoverable_runtime_diagnostic_precedes_a_later_parse_diagnostic() {
    let output = run_with_deadline(&["-ijson", "-ojson", "-c", ". + 1"], b"\"x\"\n{\n");

    assert_eq!(output.status.code(), Some(5));
    assert_eq!(output.stdout, [] as [u8; 0]);
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 diagnostics");
    let lines = stderr.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 2, "unexpected diagnostics: {stderr}");
    assert!(lines[0].starts_with("tq: runtime error:"));
    assert!(
        lines[1].starts_with("tq: Json input rejected: invalid JSON"),
        "unexpected input diagnostic: {stderr}"
    );
}

#[test]
fn embedded_uncaught_capability_denial_stops_without_eager_diagnostic() {
    let command = parse_args(["-ijson", "-ojson", "-c", "if . == null then env else . end"])
        .expect("ambient root-boundary arguments parse");
    let mut input = Cursor::new(b"null\n1\n");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let error = run_with_io(command, &mut input, &mut stdout, &mut stderr)
        .expect_err("embedded capability denial must remain an error");

    assert!(matches!(
        &error,
        RunError::Runtime(VmError::CapabilityDenied { .. })
    ));
    assert_eq!(error.status(), ExitStatus::Runtime);
    assert_eq!(stdout, b"");
    assert_eq!(stderr, b"");
    // A second root of `1` would emit `1` through the `else .` branch. Empty
    // stdout therefore proves evaluation stopped at the first denial; the
    // buffered reader may still have prefetched later input bytes.
}
