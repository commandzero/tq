//! Observable behavior checks for the benchmark subprocess probe.

#![allow(missing_docs)]

use std::{
    io::Write as _,
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

const PROBE: &str = env!("CARGO_BIN_EXE_tq-bench-probe");

fn run(arguments: &[&str], input: &[u8]) -> Output {
    let mut child = Command::new(PROBE)
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn benchmark probe");
    child
        .stdin
        .take()
        .expect("probe stdin")
        .write_all(input)
        .expect("write probe stdin");
    child.wait_with_output().expect("wait for benchmark probe")
}

#[test]
fn cargo_discovers_the_probe_binary() {
    assert!(std::path::Path::new(PROBE).is_file(), "probe path: {PROBE}");
}

#[test]
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn default_native_measurement_needs_no_path_tools_but_requested_inspection_fails() {
    let output = Command::new(PROBE)
        .arg("measure-noop")
        .env("PATH", "")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = Command::new(PROBE)
        .arg("measure-limit")
        .env("PATH", "")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("RSS inspection unavailable"));
}

#[test]
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn uninstrumented_rss_limit_needs_no_path_tools() {
    let output = Command::new(PROBE)
        .arg("measure-uninstrumented-noop")
        .env("PATH", "")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn native_preflight_succeeds_before_corpus_access_without_sampling_tools() {
    let directory = tempfile::tempdir().unwrap();
    let missing = directory.path().join("absent-manifest.json");
    let report = directory.path().join("must-not-publish.json");
    let output = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
        .args(["run", "--profile", "standard", "--manifest"])
        .arg(&missing)
        .arg("--output")
        .arg(&report)
        .env("PATH", "")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("RSS preflight passed"), "{error}");
    assert!(!report.exists());
}

#[test]
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn public_preflight_only_skips_tools_corpus_and_report_work() {
    let directory = tempfile::tempdir().unwrap();
    let missing = directory.path().join("absent-manifest.json");
    let report = directory.path().join("must-not-publish.json");
    let output = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
        .args(["--preflight-only", "--profile", "standard", "--manifest"])
        .arg(&missing)
        .arg("--output")
        .arg(&report)
        .env("PATH", "")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("RSS preflight passed"));
    assert!(!report.exists());
}

#[test]
fn benchmark_help_documents_public_preflight_only() {
    let output = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("--preflight-only"));
}

#[test]
#[cfg(unix)]
fn stalled_requested_sampler_is_bounded_and_propagates_failure() {
    use std::os::unix::fs::PermissionsExt as _;
    let directory = tempfile::tempdir().unwrap();
    let ps = directory.path().join("ps");
    std::fs::write(&ps, "#!/bin/sh\nexec /bin/sleep 30\n").unwrap();
    std::fs::set_permissions(&ps, std::fs::Permissions::from_mode(0o755)).unwrap();
    let started = Instant::now();
    let output = Command::new(PROBE)
        .arg("measure-limit")
        .env("PATH", directory.path())
        .output()
        .unwrap();
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("ps inspection timed out"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[cfg(unix)]
fn missing_requested_group_observation_fails_while_child_is_live() {
    let directory = tempfile::tempdir().unwrap();
    let ps = directory.path().join("ps");
    std::os::unix::fs::symlink(PROBE, &ps).unwrap();
    let output = Command::new(PROBE)
        .arg("measure-sleep-limit")
        .env("PATH", directory.path())
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "missing enforcement must not pass"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("live child"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[cfg(unix)]
fn child_exiting_before_inspection_still_has_native_limit_accounting() {
    let directory = tempfile::tempdir().unwrap();
    let ps = directory.path().join("ps");
    std::os::unix::fs::symlink(PROBE, &ps).unwrap();
    let output = Command::new(PROBE)
        .arg("measure-limit")
        .env("PATH", directory.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn no_op_and_released_allocation_exit_successfully() {
    assert!(run(&["noop"], &[]).status.success());
    assert!(run(&["allocate", "1048576"], &[]).status.success());
    assert!(run(&["allocate-burst", "1048576"], &[]).status.success());
    assert!(
        run(&["waited-allocation-child", "1048576"], &[])
            .status
            .success()
    );
    assert!(
        run(&["allocate-threads", "262144", "2"], &[])
            .status
            .success()
    );
}

#[test]
fn known_duration_sleeps_for_the_requested_interval() {
    let started = Instant::now();
    let output = run(&["sleep", "30"], &[]);
    let elapsed = started.elapsed();
    assert!(output.status.success());
    assert!(elapsed >= Duration::from_millis(20), "elapsed: {elapsed:?}");
    assert!(elapsed < Duration::from_secs(2), "elapsed: {elapsed:?}");
}

#[test]
fn literal_arguments_are_printed_without_shell_interpretation() {
    let output = run(
        &["literal-args", "with spaces", "* [glob] $HOME", "quote'\""],
        &[],
    );
    assert!(output.status.success());
    assert_eq!(output.stdout, b"with spaces\n* [glob] $HOME\nquote'\"\n");
}

#[test]
fn stdin_echo_preserves_bytes_and_eof() {
    let input = b"line one\n\0line two\xff";
    let output = run(&["stdin-echo"], input);
    assert!(output.status.success());
    assert_eq!(output.stdout, input);
    assert_eq!(output.stderr, [] as [u8; 0]);
}

#[test]
fn nonzero_exit_is_distinct_from_success() {
    let output = run(&["nonzero", "23"], &[]);
    assert_eq!(output.status.code(), Some(23));
    assert!(!output.status.success());
}

#[test]
fn blocked_input_does_not_consume_a_large_stdin_payload() {
    let mut child = Command::new(PROBE)
        .args(["blocked-input", "40"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn blocked-input probe");
    let stdin = child.stdin.take().expect("blocked probe stdin");
    let writer = thread::spawn(move || {
        let mut stdin = stdin;
        let _ = stdin.write_all(&vec![b'x'; 256 * 1024]);
    });
    let output = child
        .wait_with_output()
        .expect("wait for blocked-input probe");
    writer.join().expect("join blocked-input writer");
    assert!(output.status.success());
    assert_eq!(output.stdout, [] as [u8; 0]);
}

#[test]
fn output_flood_writes_the_requested_byte_count() {
    let output = run(&["flood-output", "131072"], &[]);
    assert!(output.status.success());
    assert_eq!(output.stdout.len(), 131_072);
    assert!(output.stdout.iter().all(|byte| *byte == b'x'));
}

#[cfg(unix)]
#[test]
fn self_signal_terminates_the_probe_by_signal() {
    use std::os::unix::process::ExitStatusExt as _;

    let output = run(&["self-signal", "term"], &[]);
    assert_eq!(output.status.signal(), Some(15));
}

#[cfg(unix)]
#[test]
fn descendant_fixture_exposes_a_live_child_for_cleanup_tests() {
    use nix::{
        sys::signal::{Signal, kill},
        unistd::Pid,
    };

    let output = run(&["descendant", "5000"], &[]);
    assert!(output.status.success());
    let pid = String::from_utf8(output.stdout)
        .expect("descendant pid is utf8")
        .trim()
        .parse::<i32>()
        .expect("descendant pid is numeric");
    let pid = Pid::from_raw(pid);

    // The fixture is intentionally detached from the parent's wait.  Kill it
    // here so the test suite never leaves a long-lived child behind.
    let _ = kill(pid, Signal::SIGKILL);
    for _ in 0..100 {
        if kill(pid, None).is_err() {
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("descendant process {pid} was not cleaned up");
}
