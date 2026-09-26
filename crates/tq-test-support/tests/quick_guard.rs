//! End-to-end quick guard behavior with real sleeping processes.

use std::{
    env, fs,
    os::unix::process::CommandExt as _,
    process::Command,
    thread,
    time::{Duration, Instant},
};

fn quick() -> &'static str {
    env!("CARGO_BIN_EXE_tq-quick")
}

fn run_quick(seconds: &str, program: &str, args: &[&str]) -> (std::process::Output, Duration) {
    let started = Instant::now();
    let output = Command::new(quick())
        .args(["--budget-seconds", seconds, "--", program])
        .args(args)
        .output()
        .expect("start quick guard");
    (output, started.elapsed())
}

#[test]
fn blocked_preparation_expires_without_waiting_for_the_child() {
    let (output, elapsed) = run_quick("1", "/bin/sh", &["-c", "sleep 10"]);
    assert_eq!(output.status.code(), Some(124));
    assert!(elapsed < Duration::from_secs(3), "elapsed: {elapsed:?}");
}

#[test]
fn benchmark_tool_discovery_is_bounded_before_options_run() {
    use std::os::unix::fs::PermissionsExt as _;

    let temp = tempfile::tempdir().unwrap();
    let slow_tool = temp.path().join("slow-jq");
    fs::write(&slow_tool, "#!/bin/sh\nexec sleep 10\n").unwrap();
    fs::set_permissions(&slow_tool, fs::Permissions::from_mode(0o755)).unwrap();
    let started = Instant::now();
    let output = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
        .args([
            "run",
            "--suite",
            "smoke",
            "--profile",
            "quick",
            "--case",
            "benchmark.startup",
            "--adapter",
            "jq-json",
            "--campaign-budget-seconds",
            "1",
            "--case-budget-seconds",
            "1",
        ])
        .arg("--output")
        .arg(temp.path().join("report.json"))
        .arg("--cache-root")
        .arg(temp.path().join("cache"))
        .env("TQ_JQ", &slow_tool)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(124));
    assert!(started.elapsed() < Duration::from_secs(3));
}

#[test]
fn effective_last_budget_controls_guard_after_duplicate_options() {
    use std::os::unix::fs::PermissionsExt as _;
    let temp = tempfile::tempdir().unwrap();
    let slow_tool = temp.path().join("jq");
    fs::write(&slow_tool, "#!/bin/sh\nsleep 2\nexit 0\n").unwrap();
    fs::set_permissions(&slow_tool, fs::Permissions::from_mode(0o755)).unwrap();
    let started = Instant::now();
    let output = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
        .args([
            "run",
            "--suite",
            "smoke",
            "--profile",
            "standard",
            "--profile",
            "quick",
            "--case",
            "benchmark.unknown",
            "--campaign-budget-seconds",
            "1",
            "--campaign-budget-seconds",
            "10",
        ])
        .arg("--output")
        .arg(temp.path().join("report.json"))
        .env("TQ_JQ", &slow_tool)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(started.elapsed() >= Duration::from_secs(2));
}

#[test]
fn overridden_quick_profile_is_not_supervised_as_quick() {
    use std::os::unix::fs::PermissionsExt as _;
    let temp = tempfile::tempdir().unwrap();
    let slow_tool = temp.path().join("jq");
    fs::write(&slow_tool, "#!/bin/sh\nsleep 2\nexit 0\n").unwrap();
    fs::set_permissions(&slow_tool, fs::Permissions::from_mode(0o755)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
        .args([
            "run",
            "--suite",
            "smoke",
            "--profile",
            "quick",
            "--profile",
            "standard",
            "--sampling",
            "catalog",
            "--max-samples",
            "1",
            "--campaign-budget-seconds",
            "1",
            "--case",
            "benchmark.unknown",
        ])
        .arg("--output")
        .arg(temp.path().join("report.json"))
        .env("TQ_JQ", &slow_tool)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
}
#[test]
fn normal_exit_code_and_stdio_are_preserved() {
    let (output, elapsed) = run_quick("1", "/bin/sh", &["-c", "printf 'ready\\n'; exit 7"]);
    assert_eq!(output.status.code(), Some(7));
    assert_eq!(output.stdout, b"ready\n");
    assert!(elapsed < Duration::from_secs(2), "elapsed: {elapsed:?}");
}

#[test]
fn nested_stage_cannot_reset_the_shared_budget() {
    let (output, elapsed) = run_quick(
        "1",
        quick(),
        &["--budget-seconds", "50", "--", "/bin/sh", "-c", "sleep 10"],
    );
    assert_eq!(output.status.code(), Some(124));
    assert!(elapsed < Duration::from_secs(3), "elapsed: {elapsed:?}");
}

#[test]
fn cooperative_exit_at_inherited_deadline_is_still_incomplete() {
    let started = Instant::now();
    let output = Command::new(quick())
        .args(["--budget-seconds", "1", "--"])
        .arg(env::current_exe().unwrap())
        .args(["--exact", "cooperative_deadline_helper", "--nocapture"])
        .env("TQ_QUICK_TEST_COOPERATIVE_EXIT", "1")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(124));
    assert!(started.elapsed() < Duration::from_secs(3));
}

#[test]
fn cooperative_deadline_helper() {
    if env::var_os("TQ_QUICK_TEST_COOPERATIVE_EXIT").is_none() {
        return;
    }
    while tq_test_support::benchmark::quick::remaining_work_budget()
        .is_some_and(|remaining| !remaining.is_zero())
    {
        thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn guard_rejects_a_budget_above_the_quick_ceiling() {
    let (output, elapsed) = run_quick("51", "/bin/sh", &["-c", "exit 0"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(elapsed < Duration::from_secs(1));
}

// This test body runs only inside a separately guarded copy of this test
// executable. Its child intentionally creates its own process group, just like
// the benchmark's native worker; killing the outer group alone is insufficient.
#[test]
fn detached_group_helper() {
    let Ok(pid_file) = env::var("TQ_QUICK_TEST_DESCENDANT_PID") else {
        return;
    };
    let remaining = tq_test_support::benchmark::quick::remaining_work_budget()
        .expect("helper inherited the absolute work deadline");
    assert!(remaining <= Duration::from_secs(10));
    let mut sleep = Command::new("sleep");
    sleep.arg("10").process_group(0);
    let mut child = sleep.spawn().expect("spawn dedicated-group descendant");
    fs::write(pid_file, child.id().to_string()).expect("publish descendant pid");
    child.wait().expect("reap dedicated-group descendant");
}

#[test]
fn descendant_in_a_separate_process_group_is_cleaned_up() {
    let temp = tempfile::tempdir().unwrap();
    let pid_file = temp.path().join("descendant.pid");
    let started = Instant::now();
    let output = Command::new(quick())
        .args(["--budget-seconds", "1", "--"])
        .arg(env::current_exe().unwrap())
        .args(["--exact", "detached_group_helper", "--nocapture"])
        .env("TQ_QUICK_TEST_DESCENDANT_PID", &pid_file)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(124));
    assert!(started.elapsed() < Duration::from_secs(3));
    let pid: u32 = fs::read_to_string(&pid_file)
        .expect("helper ran")
        .parse()
        .unwrap();
    assert!(
        !process_is_running(pid),
        "dedicated-group descendant {pid} survived cleanup"
    );
}

#[test]
fn interrupt_and_terminate_cancel_the_separately_grouped_coordinator() {
    for (signal, expected) in [
        (nix::sys::signal::Signal::SIGINT, 130),
        (nix::sys::signal::Signal::SIGTERM, 143),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let pid_file = temp.path().join("descendant.pid");
        let mut guard = Command::new(quick())
            .args(["--budget-seconds", "10", "--"])
            .arg(env::current_exe().unwrap())
            .args(["--exact", "detached_group_helper", "--nocapture"])
            .env("TQ_QUICK_TEST_DESCENDANT_PID", &pid_file)
            .spawn()
            .unwrap();
        let started = Instant::now();
        while !pid_file.exists() {
            assert!(
                started.elapsed() < Duration::from_secs(3),
                "helper never spawned"
            );
            thread::sleep(Duration::from_millis(10));
        }
        nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(i32::try_from(guard.id()).unwrap()),
            signal,
        )
        .unwrap();
        let status = guard.wait().unwrap();
        assert_eq!(status.code(), Some(expected));
        assert!(started.elapsed() < Duration::from_secs(3));
        let pid: u32 = fs::read_to_string(&pid_file).unwrap().parse().unwrap();
        assert!(!process_is_running(pid), "orphaned descendant {pid}");
    }
}

#[test]
fn blocked_stdout_never_publishes_a_staged_completed_report() {
    let temp = tempfile::tempdir().unwrap();
    let report = temp.path().join("report.json");
    let mut command = Command::new(quick());
    command
        .args(["--budget-seconds", "1", "--"])
        .arg(env::current_exe().unwrap())
        .args(["--exact", "blocked_report_helper", "--nocapture"])
        .env("TQ_QUICK_REPORT_PATH", &report)
        .env("TQ_QUICK_TEST_REPORT_PATH", &report)
        .stdout(std::process::Stdio::piped());
    let started = Instant::now();
    let mut guard = command.spawn().unwrap();
    let status = guard.wait().unwrap();
    assert_eq!(status.code(), Some(124));
    assert!(started.elapsed() < Duration::from_secs(3));
    assert_eq!(fs::read_to_string(&report).unwrap(), "incomplete");
}

#[test]
fn signalled_coordinator_cannot_publish_a_staged_completed_report() {
    let temp = tempfile::tempdir().unwrap();
    let report = temp.path().join("report.json");
    let output = Command::new(quick())
        .args(["--budget-seconds", "2", "--", "/bin/sh", "-c"])
        .arg(
            "printf incomplete > \"$TQ_QUICK_REPORT_PATH\"; \
             printf complete > \"$TQ_QUICK_COMPLETION_STAGE\"; kill -KILL $$",
        )
        .env("TQ_QUICK_REPORT_PATH", &report)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(137));
    assert_eq!(fs::read_to_string(report).unwrap(), "incomplete");
}

#[test]
fn blocked_report_helper() {
    use std::io::Write as _;
    let Ok(report) = env::var("TQ_QUICK_TEST_REPORT_PATH") else {
        return;
    };
    fs::write(&report, "incomplete").unwrap();
    let stage = tq_test_support::benchmark::quick::completion_stage().unwrap();
    fs::write(stage, "complete").unwrap();
    let mut stdout = std::io::stdout().lock();
    loop {
        stdout.write_all(&[b'x'; 8192]).unwrap();
    }
}

fn process_is_running(pid: u32) -> bool {
    #[cfg(target_os = "linux")]
    {
        let Ok(stat) = fs::read_to_string(format!("/proc/{pid}/stat")) else {
            return false;
        };
        stat.rsplit_once(") ")
            .and_then(|(_, rest)| rest.split_whitespace().next())
            != Some("Z")
    }
    #[cfg(target_os = "macos")]
    {
        use libproc::{bsd_info::BSDInfo, proc_pid::pidinfo};
        let Ok(pid) = i32::try_from(pid) else {
            return false;
        };
        pidinfo::<BSDInfo>(pid, 1).is_ok_and(|info| {
            info.pbi_pid == pid.cast_unsigned() && info.pbi_status != nix::libc::SZOMB
        })
    }
}
