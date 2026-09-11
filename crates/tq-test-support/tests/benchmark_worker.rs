//! Public worker isolation checks. These are lifecycle tests, not calibration.
#![cfg(any(target_os = "macos", target_os = "linux"))]

use std::{
    process::{Command, Stdio},
    time::Duration,
};
use tq_test_support::benchmark::{
    BenchmarkInvocation, MeasuredOutcome, MeasuredStatus, measure_process_worker,
};
use wait_timeout::ChildExt as _;

const PROBE: &str = env!("CARGO_BIN_EXE_tq-bench-probe");
const WORKER: &str = env!("CARGO_BIN_EXE_tq-bench-worker");

fn invocation(args: &[&str]) -> BenchmarkInvocation {
    BenchmarkInvocation {
        cancellation: None,
        executable: PROBE.into(),
        args: args.iter().map(|value| (*value).to_owned()).collect(),
        stdin: Vec::new(),
        current_dir: None,
        timeout: Duration::from_secs(2),
        output_limit: 1024,
        rss_limit: None,
        retain_output: true,
    }
}

fn output_and_cleanup(outcome: &MeasuredOutcome) -> Vec<u8> {
    let stdout = outcome.stdout_path.as_ref().unwrap();
    let bytes = std::fs::read(stdout).unwrap();
    std::fs::remove_file(stdout).unwrap();
    std::fs::remove_file(outcome.stderr_path.as_ref().unwrap()).unwrap();
    bytes
}

#[test]
fn worker_preserves_literal_arguments_and_prepared_stdin_bytes() {
    let outcome = measure_process_worker(&invocation(&["args", "a b", "*", "$(false)"]))
        .expect("literal worker invocation");
    assert_eq!(outcome.exit_code, Some(0));
    assert_eq!(output_and_cleanup(&outcome), b"a b\n*\n$(false)\n");

    let mut request = invocation(&["stdin-echo"]);
    request.stdin = b"zero\0byte\r\nlast".to_vec();
    let outcome = measure_process_worker(&request).expect("worker stdin invocation");
    assert_eq!(outcome.exit_code, Some(0));
    assert_eq!(output_and_cleanup(&outcome), request.stdin);
}

#[test]
fn worker_preserves_nonzero_exit_and_target_timeout() {
    let outcome =
        measure_process_worker(&invocation(&["exit", "23"])).expect("nonzero target is an outcome");
    assert_eq!(outcome.status, MeasuredStatus::Exited);
    assert_eq!(outcome.exit_code, Some(23));
    output_and_cleanup(&outcome);

    let mut request = invocation(&["blocked-input", "10000"]);
    request.timeout = Duration::from_millis(30);
    request.stdin = vec![b'x'; 1024 * 1024];
    let started = std::time::Instant::now();
    let outcome = measure_process_worker(&request).expect("timeout target is an outcome");
    assert_eq!(outcome.status, MeasuredStatus::Timeout);
    assert!(started.elapsed() < Duration::from_secs(3));
    assert!(outcome.peak_rss_bytes.unwrap() > 0);
    output_and_cleanup(&outcome);
}

#[test]
fn worker_applies_the_requested_target_directory() {
    let directory = tempfile::tempdir().unwrap();
    let expected = directory.path().canonicalize().unwrap();
    let mut request = invocation(&[]);
    request.executable = "/bin/pwd".into();
    request.current_dir = Some(expected.clone());
    let outcome = measure_process_worker(&request).expect("working-directory target");
    assert_eq!(outcome.exit_code, Some(0));
    assert_eq!(
        String::from_utf8(output_and_cleanup(&outcome))
            .unwrap()
            .trim(),
        expected.to_str().unwrap()
    );
}

fn measure(parent_bytes: usize, input_bytes: usize) -> MeasuredOutcome {
    let stdout = tempfile::NamedTempFile::new().unwrap();
    let stderr = tempfile::NamedTempFile::new().unwrap();
    let mut child = Command::new(PROBE)
        .args([
            "measure-worker-parent",
            &parent_bytes.to_string(),
            &input_bytes.to_string(),
        ])
        .env("TQ_BENCH_WORKER", WORKER)
        .stdin(Stdio::null())
        .stdout(stdout.reopen().unwrap())
        .stderr(stderr.reopen().unwrap())
        .spawn()
        .unwrap();
    let Some(status) = child.wait_timeout(Duration::from_secs(30)).unwrap() else {
        child.kill().unwrap();
        child.wait().unwrap();
        panic!("worker measurement exceeded its lifecycle bound");
    };
    assert!(
        status.success(),
        "{}",
        std::fs::read_to_string(stderr.path()).unwrap()
    );
    serde_json::from_slice(&std::fs::read(stdout.path()).unwrap()).unwrap()
}

#[test]
fn coordinator_memory_does_not_determine_target_peak() {
    let control = measure(0, 0).peak_rss_bytes.unwrap();
    for parent_bytes in [32 * 1024 * 1024, 128 * 1024 * 1024, 0] {
        let peak = measure(parent_bytes, 0).peak_rss_bytes.unwrap();
        assert!(
            peak.abs_diff(control) < 8 * 1024 * 1024,
            "parent={parent_bytes}, control={control}, target={peak}"
        );
    }
}

#[test]
fn large_prepared_stdin_roundtrips_without_a_payload_sized_launch_floor() {
    let bytes = 32 * 1024 * 1024;
    let result = measure(128 * 1024 * 1024, bytes);
    assert_eq!(result.output_bytes, u64::try_from(bytes).unwrap());
    assert!(
        result.peak_rss_bytes.unwrap() < 16 * 1024 * 1024,
        "streaming echo inherited a payload-sized launch floor: {:?}",
        result.peak_rss_bytes
    );
}
