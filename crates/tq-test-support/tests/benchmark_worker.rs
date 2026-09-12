//! Public worker isolation checks. These are lifecycle tests, not calibration.
#![cfg(any(target_os = "macos", target_os = "linux"))]

use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{Read as _, Write as _},
    os::unix::{
        ffi::{OsStrExt as _, OsStringExt as _},
        fs::PermissionsExt as _,
        process::CommandExt as _,
    },
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::Duration,
};
use tq_test_support::benchmark::{
    BenchmarkInvocation, MeasureError, MeasuredOutcome, MeasuredStatus, measure_process_worker,
};
use wait_timeout::ChildExt as _;

const PROBE: &str = env!("CARGO_BIN_EXE_tq-bench-probe");
const WORKER: &str = env!("CARGO_BIN_EXE_tq-bench-worker");
const WORKER_HELPER_ENV: &str = "TQ_TEST_WORKER_HELPER";
const COORDINATOR_ENV: &str = "TQ_TEST_WORKER_COORDINATOR";

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

#[derive(Debug, serde::Deserialize)]
struct FaultMeasurement {
    outcome: Option<MeasuredOutcome>,
    error: Option<String>,
    stdout_path: Option<PathBuf>,
    stderr_path: Option<PathBuf>,
    #[serde(skip)]
    worker_pid: Option<i32>,
}

fn fault_worker(mode: &str) -> (tempfile::TempDir, PathBuf) {
    let helper = match mode {
        "startup-delay" => "worker_fault_helper_startup",
        "request-delay" => "worker_fault_helper_request",
        "reply-delay" => "worker_fault_helper_reply",
        "teardown-delay" => "worker_fault_helper_teardown",
        "empty" => "worker_fault_helper_empty",
        "truncated" => "worker_fault_helper_truncated",
        "oversize" => "worker_fault_helper_oversize",
        "version" => "worker_fault_helper_version",
        "malformed" => "worker_fault_helper_malformed",
        "malformed-descendant" => "worker_fault_helper_malformed_descendant",
        "non-reading" => "worker_fault_helper_non_reading",
        other => panic!("unknown worker fault mode {other}"),
    };
    let directory = tempfile::tempdir().expect("fault worker directory");
    let path = directory.path().join("worker.sh");
    let binary = shell_quote(
        &env::current_exe()
            .expect("current integration test binary")
            .to_string_lossy(),
    );
    let script =
        format!("#!/bin/sh\n{WORKER_HELPER_ENV}=1 exec {binary} --exact {helper} --nocapture\n");
    fs::write(&path, script).expect("write fault worker wrapper");
    let mut permissions = fs::metadata(&path)
        .expect("fault worker wrapper metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("make fault worker executable");
    (directory, path)
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn run_fault_measure(mode: &str, args: &[&str]) -> (FaultMeasurement, Duration) {
    let (_worker_directory, worker) = fault_worker(mode);
    let record = tempfile::NamedTempFile::new().expect("fault measurement record");
    let pid_record = tempfile::NamedTempFile::new().expect("fault worker PID record");
    let started = std::time::Instant::now();
    let mut child = Command::new(env::current_exe().expect("current integration test binary"))
        .args(["--exact", "worker_fault_coordinator", "--nocapture"])
        .env("TQ_BENCH_WORKER", worker)
        .env(COORDINATOR_ENV, record.path())
        .env("TQ_TEST_WORKER_PID_RECORD", pid_record.path())
        .env("TQ_TEST_WORKER_ARGS", args.join("\n"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn worker coordinator");
    let deadline = if mode == "non-reading" {
        Duration::from_secs(8)
    } else {
        Duration::from_secs(3)
    };
    let status = wait_timeout::ChildExt::wait_timeout(&mut child, deadline)
        .expect("wait for worker coordinator")
        .unwrap_or_else(|| {
            child.kill().expect("kill stuck worker coordinator");
            child.wait().expect("reap stuck worker coordinator");
            let diagnostics = child
                .stderr
                .take()
                .map(|mut stderr| {
                    let mut text = String::new();
                    let _ = stderr.read_to_string(&mut text);
                    text
                })
                .unwrap_or_default();
            panic!("worker coordinator exceeded {deadline:?}: {diagnostics}");
        });
    let elapsed = started.elapsed();
    if !status.success() {
        let diagnostics = child
            .stderr
            .take()
            .map(|mut stderr| {
                let mut text = String::new();
                let _ = stderr.read_to_string(&mut text);
                text
            })
            .unwrap_or_default();
        panic!("worker coordinator failed: {status}\n{diagnostics}");
    }
    let mut measurement: FaultMeasurement =
        serde_json::from_slice(&fs::read(record.path()).expect("worker coordinator record"))
            .expect("decode worker coordinator record");
    measurement.worker_pid = fs::read_to_string(pid_record.path())
        .ok()
        .and_then(|value| value.trim().parse().ok());
    (measurement, elapsed)
}

fn cleanup_fault_measurement(measurement: &FaultMeasurement) {
    for path in [&measurement.stdout_path, &measurement.stderr_path]
        .into_iter()
        .flatten()
    {
        let _ = fs::remove_file(path);
    }
}

fn write_fault_record(result: Result<MeasuredOutcome, MeasureError>) {
    let path = PathBuf::from(env::var_os(COORDINATOR_ENV).expect("coordinator record path"));
    let value = match result {
        Ok(outcome) => serde_json::json!({
            "outcome": outcome,
            "error": null,
            "stdout_path": null,
            "stderr_path": null,
        }),
        Err(MeasureError::Collection {
            source,
            stdout_path,
            stderr_path,
            ..
        }) => serde_json::json!({
            "outcome": null,
            "error": source.to_string(),
            "stdout_path": stdout_path,
            "stderr_path": stderr_path,
        }),
        Err(error) => serde_json::json!({
            "outcome": null,
            "error": error.to_string(),
            "stdout_path": null,
            "stderr_path": null,
        }),
    };
    fs::write(
        path,
        serde_json::to_vec(&value).expect("encode worker record"),
    )
    .expect("write worker coordinator record");
}

#[test]
fn worker_fault_coordinator() {
    if env::var_os(COORDINATOR_ENV).is_none() {
        return;
    }
    let args = env::var("TQ_TEST_WORKER_ARGS")
        .expect("worker coordinator arguments")
        .split('\n')
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let mut request = invocation(&[]);
    request.args = args;
    let result = measure_process_worker(&request);
    write_fault_record(result);
}

macro_rules! worker_fault_helper_test {
    ($name:ident, $mode:literal) => {
        #[test]
        fn $name() {
            if env::var_os(WORKER_HELPER_ENV).is_none() {
                return;
            }
            run_fault_helper($mode);
        }
    };
}

worker_fault_helper_test!(worker_fault_helper_startup, "startup-delay");
worker_fault_helper_test!(worker_fault_helper_request, "request-delay");
worker_fault_helper_test!(worker_fault_helper_reply, "reply-delay");
worker_fault_helper_test!(worker_fault_helper_teardown, "teardown-delay");
worker_fault_helper_test!(worker_fault_helper_empty, "empty");
worker_fault_helper_test!(worker_fault_helper_truncated, "truncated");
worker_fault_helper_test!(worker_fault_helper_oversize, "oversize");
worker_fault_helper_test!(worker_fault_helper_version, "version");
worker_fault_helper_test!(worker_fault_helper_malformed, "malformed");
worker_fault_helper_test!(
    worker_fault_helper_malformed_descendant,
    "malformed-descendant"
);
worker_fault_helper_test!(worker_fault_helper_non_reading, "non-reading");

fn run_fault_helper(mode: &str) {
    if mode == "non-reading" {
        if let Some(path) = env::var_os("TQ_TEST_WORKER_PID_RECORD") {
            fs::write(path, std::process::id().to_string()).expect("record non-reading worker PID");
        }
        thread::sleep(Duration::from_secs(30));
        return;
    }
    if mode == "startup-delay" {
        thread::sleep(Duration::from_millis(150));
    }
    let mut request_bytes = Vec::new();
    let mut stdin = std::io::stdin().lock();
    if mode == "request-delay" {
        let mut first_byte = [0; 1];
        stdin
            .read_exact(&mut first_byte)
            .expect("read first worker request byte");
        request_bytes.push(first_byte[0]);
        thread::sleep(Duration::from_millis(150));
    }
    stdin
        .read_to_end(&mut request_bytes)
        .expect("read worker request fixture");
    let request: serde_json::Value =
        serde_json::from_slice(&request_bytes).expect("decode worker request fixture");
    let reply_path = request_path(&request, "reply_path");
    let ready_path = reply_path.with_extension("ready");

    if matches!(
        mode,
        "startup-delay" | "request-delay" | "reply-delay" | "teardown-delay"
    ) {
        run_delayed_proxy(mode, &request);
        return;
    }

    let mut target = None;
    if mode == "malformed-descendant" {
        target = Some(spawn_requested_target(&request));
        thread::sleep(Duration::from_millis(200));
    }
    if mode == "reply-delay" {
        thread::sleep(Duration::from_millis(150));
    }
    let reply = match mode {
        "empty" => Vec::new(),
        "truncated" => b"{".to_vec(),
        "malformed" | "malformed-descendant" => b"not-json".to_vec(),
        "oversize" => vec![b'x'; 64 * 1024 + 1],
        "version" => valid_reply(2),
        "startup-delay" | "request-delay" | "reply-delay" | "teardown-delay" => valid_reply(3),
        other => panic!("unknown fault helper mode {other}"),
    };
    fs::write(&reply_path, reply).expect("write worker reply fixture");
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&ready_path)
        .expect("mark worker reply fixture ready")
        .write_all(b"1")
        .expect("write worker reply marker");
    if matches!(
        mode,
        "startup-delay" | "request-delay" | "reply-delay" | "teardown-delay"
    ) {
        thread::sleep(Duration::from_secs(30));
    }
    if let Some(mut target) = target {
        let _ = target.wait();
    }
}

fn run_delayed_proxy(mode: &str, request: &serde_json::Value) {
    let original_reply = request_path(request, "reply_path");
    let proxy_reply = original_reply.with_file_name(format!(
        ".{}.proxy",
        original_reply
            .file_name()
            .expect("reply file name")
            .to_string_lossy()
    ));
    File::create(&proxy_reply).expect("create proxy reply capture");
    let mut forwarded = request.clone();
    forwarded["reply_path"] = serde_json::Value::Array(
        proxy_reply
            .as_os_str()
            .as_bytes()
            .iter()
            .map(|byte| serde_json::Value::from(u64::from(*byte)))
            .collect(),
    );
    forwarded["coordinator_pid"] = serde_json::Value::from(std::process::id());
    // The proxy is intentionally not a process-group anchor. Keep the real
    // worker's optional group sampler disabled while preserving its native
    // target wall/RSS accounting for this protocol-boundary test.
    forwarded["sample_process_group"] = serde_json::Value::from(false);
    let encoded = serde_json::to_vec(&forwarded).expect("encode proxied worker request");
    let mut worker = Command::new(WORKER)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .expect("spawn proxied real worker");
    if let Some(path) = env::var_os("TQ_TEST_WORKER_PID_RECORD") {
        fs::write(path, worker.id().to_string()).expect("record proxied worker PID");
    }
    let mut stdin = worker.stdin.take().expect("proxied worker stdin");
    stdin
        .write_all(&encoded)
        .and_then(|()| stdin.write_all(b"\n"))
        .expect("write proxied worker request");
    drop(stdin);
    if mode == "reply-delay" {
        wait_for_reply_marker(&proxy_reply);
        thread::sleep(Duration::from_millis(150));
    }
    if mode == "teardown-delay" {
        wait_for_reply_marker(&proxy_reply);
        thread::sleep(Duration::from_millis(150));
    }
    if mode == "startup-delay" || mode == "request-delay" {
        wait_for_reply_marker(&proxy_reply);
    }
    fs::copy(&proxy_reply, &original_reply).expect("copy proxied worker reply");
    fs::remove_file(&proxy_reply).expect("remove proxied reply capture");
    fs::remove_file(proxy_reply.with_extension("ready")).expect("remove proxied reply marker");
    let original_ready = original_reply.with_extension("ready");
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(original_ready)
        .expect("mark proxied worker reply ready")
        .write_all(b"1")
        .expect("write proxied worker reply marker");
    let _ = worker.wait();
}

fn wait_for_reply_marker(reply: &std::path::Path) {
    let marker = reply.with_extension("ready");
    let deadline = std::time::Instant::now() + Duration::from_secs(7);
    while fs::metadata(&marker).is_err() {
        assert!(
            std::time::Instant::now() < deadline,
            "proxied worker did not produce a reply"
        );
        thread::sleep(Duration::from_millis(2));
    }
}

fn spawn_requested_target(request: &serde_json::Value) -> std::process::Child {
    let executable = request_path(request, "executable");
    let args = request["args"]
        .as_array()
        .expect("worker arguments")
        .iter()
        .map(|value| value.as_str().expect("worker argument").to_owned())
        .collect::<Vec<_>>();
    let stdin = File::open(request_path(request, "stdin_path")).expect("open target stdin");
    let stdout =
        File::create(request_path(request, "stdout_path")).expect("open target stdout capture");
    let stderr =
        File::create(request_path(request, "stderr_path")).expect("open target stderr capture");
    let mut command = Command::new(executable);
    command
        .args(args)
        .stdin(Stdio::from(stdin))
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    if let Some(current_dir) = request["current_dir"].as_array() {
        command.current_dir(PathBuf::from(std::ffi::OsString::from_vec(
            current_dir
                .iter()
                .map(|value| u8::try_from(value.as_u64().expect("path byte")).expect("path byte"))
                .collect(),
        )));
    }
    command.spawn().expect("spawn worker target")
}

fn request_path(request: &serde_json::Value, field: &str) -> PathBuf {
    PathBuf::from(std::ffi::OsString::from_vec(
        request[field]
            .as_array()
            .expect("worker path")
            .iter()
            .map(|value| u8::try_from(value.as_u64().expect("path byte")).expect("path byte"))
            .collect(),
    ))
}

fn valid_reply(version: u8) -> Vec<u8> {
    let rss_provenance = if cfg!(target_os = "linux") {
        "linux-wait4"
    } else {
        "darwin-wait4"
    };
    serde_json::to_vec(&serde_json::json!({
        "version": version,
        "result": {
            "outcome": {
                "status": "exited",
                "exit_code": 0,
                "signal": null,
                "wall_time_micros": 20_000,
                "first_result_micros": null,
                "user_cpu_micros": 1,
                "system_cpu_micros": 1,
                "peak_rss_bytes": 4096,
                "rss_provenance": rss_provenance,
                "measurement_protocol": {
                    "timing_method": "test fixture",
                    "input_delivery": "prepared file",
                    "rss_scope": "target process",
                    "exit_poll_interval_micros": 1,
                    "rss_poll_interval_micros": null,
                    "validated_accuracy_micros": null,
                    "worker": null,
                    "isolation_evidence": null,
                },
                "process_group_peak_rss_bytes": null,
                "output_bytes": 0,
                "stdout_path": null,
                "stderr_path": null,
                "process_group_rss_observed": false,
            }
        }
    }))
    .expect("encode valid worker reply fixture")
}

#[test]
fn delayed_worker_startup_and_reply_do_not_change_target_duration() {
    for mode in ["startup-delay", "request-delay", "reply-delay"] {
        let (measurement, elapsed) = run_fault_measure(mode, &["sleep", "20"]);
        let outcome = measurement.outcome.as_ref().unwrap_or_else(|| {
            panic!(
                "delayed worker outcome missing for {mode}: {:?}",
                measurement.error
            )
        });
        assert!(
            elapsed >= Duration::from_millis(100),
            "{mode} fixture did not delay"
        );
        let elapsed_micros = elapsed.as_micros();
        assert!(outcome.wall_time_micros > 0);
        assert!(
            outcome.wall_time_micros + 100_000 < elapsed_micros,
            "{mode} target duration included protocol delay: target={} us, elapsed={elapsed:?}",
            outcome.wall_time_micros
        );
        cleanup_fault_measurement(&measurement);
        wait_until_gone(measurement.worker_pid.expect("proxied worker PID"));
    }
}

#[test]
fn delayed_worker_teardown_is_outside_target_duration() {
    let (measurement, elapsed) = run_fault_measure("teardown-delay", &["sleep", "20"]);
    let outcome = measurement
        .outcome
        .as_ref()
        .expect("teardown worker outcome");
    assert!(
        elapsed < Duration::from_secs(2),
        "worker teardown was not bounded"
    );
    assert!(outcome.wall_time_micros > 0);
    assert!(
        outcome.wall_time_micros + 100_000 < elapsed.as_micros(),
        "worker teardown leaked into target duration: target={} us, elapsed={elapsed:?}",
        outcome.wall_time_micros
    );
    cleanup_fault_measurement(&measurement);
    wait_until_gone(measurement.worker_pid.expect("proxied worker PID"));
}

#[test]
fn malformed_worker_replies_fail_closed_with_retained_diagnostics() {
    for mode in ["empty", "truncated", "oversize", "version", "malformed"] {
        let (measurement, _) = run_fault_measure(mode, &[]);
        assert!(measurement.outcome.is_none(), "{mode} published an outcome");
        let error = measurement.error.as_deref().expect("fault diagnostic");
        assert!(!error.is_empty(), "{mode} lost the diagnostic");
        assert!(
            measurement
                .stdout_path
                .as_ref()
                .is_some_and(|path| path.is_file())
        );
        assert!(
            measurement
                .stderr_path
                .as_ref()
                .is_some_and(|path| path.is_file())
        );
        cleanup_fault_measurement(&measurement);
    }
}

#[test]
fn malformed_reply_cleans_a_target_descendant() {
    let (measurement, _) = run_fault_measure("malformed-descendant", &["descendant", "10000"]);
    assert!(measurement.outcome.is_none());
    let stdout = fs::read(measurement.stdout_path.as_ref().expect("retained stdout"))
        .expect("read descendant diagnostic");
    assert!(
        !stdout.is_empty(),
        "descendant output was empty: {stdout:?}"
    );
    let pid = String::from_utf8(stdout)
        .expect("descendant PID output")
        .trim()
        .parse::<u32>()
        .expect("descendant PID");
    cleanup_fault_measurement(&measurement);
    wait_until_gone(i32::try_from(pid).expect("descendant PID fits i32"));
}

#[test]
fn non_reading_worker_reaches_its_bounded_deadline() {
    let (measurement, elapsed) = run_fault_measure("non-reading", &[]);
    assert!(measurement.outcome.is_none());
    assert!(
        measurement
            .error
            .as_deref()
            .is_some_and(|error| error.contains("bounded deadline"))
    );
    assert!(
        elapsed < Duration::from_secs(8),
        "non-reading worker was unbounded"
    );
    wait_until_gone(
        measurement
            .worker_pid
            .expect("non-reading worker PID was recorded"),
    );
    cleanup_fault_measurement(&measurement);
}

#[cfg(target_os = "linux")]
fn process_is_alive(pid: i32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|stat| stat.split_whitespace().nth(2)?.chars().next())
        .is_some_and(|state| state != 'Z')
}

#[cfg(target_os = "macos")]
fn process_is_alive(pid: i32) -> bool {
    use libproc::{bsd_info::BSDInfo, proc_pid::pidinfo};
    pidinfo::<BSDInfo>(pid, 1).is_ok_and(|info| info.pbi_status != nix::libc::SZOMB)
}

fn wait_until_gone(pid: i32) {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while process_is_alive(pid) {
        assert!(
            std::time::Instant::now() < deadline,
            "process {pid} remained alive after cleanup"
        );
        thread::sleep(Duration::from_millis(10));
    }
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
