//! Failure-injection coverage for the isolated benchmark worker.
//!
//! These tests are ignored because they deliberately kill a coordinator or
//! worker. The parent test process owns the temporary evidence directory and
//! uses the worker's documented request protocol so it can inspect the
//! target's descendant capture after the helper coordinator is gone.

#![cfg(any(target_os = "macos", target_os = "linux"))]
#![allow(missing_docs)]

use std::{
    fs,
    os::unix::process::CommandExt as _,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use nix::{
    sys::signal::{Signal, kill},
    unistd::{Pid, getppid},
};
use tq_test_support::benchmark::{BenchmarkInvocation, measure_process_worker};
use wait_timeout::ChildExt as _;

const PROBE: &str = env!("CARGO_BIN_EXE_tq-bench-probe");
const WORKER: &str = env!("CARGO_BIN_EXE_tq-bench-worker");
const HELPER_DIR: &str = "TQ_WORKER_LIFECYCLE_DIR";
const HELPER: &str = "TQ_WORKER_LIFECYCLE_HELPER";
const DESCENDANT_MILLIS: u64 = 30_000;
const HELPER_TIMEOUT: Duration = Duration::from_secs(10);
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(5);

#[test]
#[ignore = "kills a helper coordinator; run explicitly on native hosts"]
fn coordinator_loss_cleans_worker_target_and_descendant() {
    run_loss_test("coordinator-loss");
}

#[test]
#[ignore = "kills a worker; run explicitly on native hosts"]
fn worker_loss_cleans_target_and_descendant() {
    run_loss_test("worker-loss");
}

#[test]
#[ignore = "helper entry point for the coordinator-loss integration test"]
fn coordinator_loss_helper() {
    if std::env::var_os(HELPER).is_none() {
        return;
    }
    helper_main();
}

#[test]
#[ignore = "helper entry point for the worker-loss integration test"]
fn worker_loss_helper() {
    if std::env::var_os(HELPER).is_none() {
        return;
    }
    helper_main();
}

fn run_loss_test(mode: &str) {
    let directory = tempfile::tempdir().expect("lifecycle evidence directory");
    let mut cleanup = LifecycleCleanup {
        helper: Some(spawn_helper(mode, directory.path())),
        worker_pid: None,
        target_pid: None,
        descendant_pid: None,
    };
    let helper = cleanup.helper.as_mut().expect("helper was spawned");
    let helper_id = helper.id();
    let worker_pid = wait_for_pid(&directory.path().join("worker.pid"));
    cleanup.worker_pid = Some(worker_pid);
    let target_pid = wait_for_pid(&directory.path().join("target.pid"));
    cleanup.target_pid = Some(target_pid);
    let descendant_pid = wait_for_pid(&directory.path().join("stdout"));
    cleanup.descendant_pid = Some(descendant_pid);
    assert!(
        pid_is_alive(target_pid),
        "target helper exited before injection"
    );
    assert!(
        pid_is_alive(descendant_pid),
        "descendant exited before injection"
    );

    let worker_group = Pid::from_raw(worker_pid);
    if mode == "coordinator-loss" {
        kill(
            Pid::from_raw(helper_id.try_into().expect("helper PID fits i32")),
            Signal::SIGKILL,
        )
        .expect("kill helper coordinator");
    } else {
        kill(worker_group, Signal::SIGKILL).expect("kill worker process");
    }

    let Some(status) = helper
        .wait_timeout(HELPER_TIMEOUT)
        .expect("wait for lifecycle helper")
    else {
        terminate_pid(Some(worker_pid));
        let _ = helper.kill();
        let _ = helper.wait();
        panic!("{mode} helper did not exit within {HELPER_TIMEOUT:?}");
    };
    if mode == "coordinator-loss" {
        assert!(
            !status.success() || status.code().is_none(),
            "coordinator helper unexpectedly survived SIGKILL: {status:?}"
        );
    } else {
        assert!(
            !status.success(),
            "worker loss helper unexpectedly succeeded"
        );
    }

    assert!(
        !directory.path().join("unexpected-success").exists(),
        "worker loss was reported as a successful measurement"
    );
    wait_until_gone(worker_pid);
    wait_until_gone(target_pid);
    wait_until_gone(descendant_pid);
    cleanup.helper = None;
    cleanup.worker_pid = None;
    cleanup.target_pid = None;
    cleanup.descendant_pid = None;
    // Keep the directory alive until all checks have completed, so a stale
    // worker cannot hide a still-open capture behind temporary-file cleanup.
    drop(directory);
}

struct LifecycleCleanup {
    helper: Option<Child>,
    worker_pid: Option<i32>,
    target_pid: Option<i32>,
    descendant_pid: Option<i32>,
}

impl Drop for LifecycleCleanup {
    fn drop(&mut self) {
        if let Some(mut helper) = self.helper.take() {
            let _ = helper.kill();
            let _ = helper.wait();
        }
        terminate_pid(self.worker_pid);
        terminate_pid(self.target_pid);
        if let Some(descendant_pid) = self.descendant_pid {
            let _ = kill(Pid::from_raw(descendant_pid), Signal::SIGKILL);
        }
    }
}

fn spawn_helper(mode: &str, directory: &Path) -> std::process::Child {
    let test_binary = std::env::current_exe().expect("locate lifecycle test binary");
    let helper_name = match mode {
        "coordinator-loss" => "coordinator_loss_helper",
        "worker-loss" => "worker_loss_helper",
        other => panic!("unknown lifecycle mode {other:?}"),
    };
    let mut command = Command::new(test_binary);
    command
        .args(["--ignored", "--exact", helper_name, "--nocapture"])
        .env(HELPER, "1")
        .env(HELPER_DIR, directory)
        .env("TQ_BENCH_WORKER", WORKER)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .process_group(0);
    command.spawn().expect("spawn lifecycle helper")
}

fn helper_main() {
    let directory =
        PathBuf::from(std::env::var_os(HELPER_DIR).expect("lifecycle helper evidence directory"));
    let target = std::env::current_exe().expect("locate target helper binary");
    let invocation = BenchmarkInvocation {
        cancellation: None,
        executable: target,
        args: vec![
            "--ignored".to_owned(),
            "--exact".to_owned(),
            "target_helper".to_owned(),
            "--nocapture".to_owned(),
        ],
        stdin: Vec::new(),
        current_dir: None,
        timeout: HELPER_TIMEOUT,
        output_limit: 1024,
        rss_limit: None,
        retain_output: false,
    };
    if measure_process_worker(&invocation).is_ok() {
        fs::write(directory.join("unexpected-success"), b"1").expect("record success");
    }
    // A coordinator that reaches this point only did so because the target or
    // worker exited without the injected failure. Keep the helper failing so
    // the parent cannot mistake a lost worker for a successful measurement.
    std::process::exit(1);
}

#[test]
#[ignore = "target entry point for worker lifecycle injection"]
fn target_helper() {
    if std::env::var_os(HELPER).is_none() {
        return;
    }
    let directory =
        PathBuf::from(std::env::var_os(HELPER_DIR).expect("target helper evidence directory"));
    fs::write(directory.join("target.pid"), std::process::id().to_string())
        .expect("record target PID");
    let worker_pid = getppid();
    fs::write(
        directory.join("worker.pid"),
        worker_pid.as_raw().to_string(),
    )
    .expect("record worker PID");
    let mut child = Command::new(PROBE)
        .args(["descendant-child", &DESCENDANT_MILLIS.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn target descendant");
    fs::write(directory.join("stdout"), child.id().to_string()).expect("record descendant PID");
    // Do not help the measurement layer clean up: injected failures must
    // terminate this target and its descendant through the real lifecycle.
    child
        .wait()
        .expect("wait for descendant without self-cleanup");
}

fn wait_for_pid(path: &Path) -> i32 {
    let deadline = Instant::now() + HELPER_TIMEOUT;
    loop {
        if let Ok(value) = fs::read_to_string(path)
            && let Ok(pid) = value.trim().parse::<i32>()
        {
            return pid;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for PID artifact {}",
            path.display()
        );
        thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(target_os = "linux")]
fn pid_is_alive(pid: i32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|stat| stat.split_whitespace().nth(2)?.chars().next())
        .is_some_and(|state| state != 'Z')
}

#[cfg(target_os = "macos")]
fn pid_is_alive(pid: i32) -> bool {
    use libproc::{bsd_info::BSDInfo, proc_pid::pidinfo};
    pidinfo::<BSDInfo>(pid, 1).is_ok_and(|info| info.pbi_status != nix::libc::SZOMB)
}

fn wait_until_gone(pid: i32) {
    let deadline = Instant::now() + CLEANUP_TIMEOUT;
    while pid_is_alive(pid) {
        assert!(
            Instant::now() < deadline,
            "process {pid} remained alive after cleanup"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn terminate_pid(pid: Option<i32>) {
    if let Some(pid) = pid {
        let _ = kill(Pid::from_raw(pid), Signal::SIGKILL);
    }
}
