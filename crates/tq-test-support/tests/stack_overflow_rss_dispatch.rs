//! Process-level checks for the Stack Overflow RSS helper modes.

#![allow(missing_docs)]

use std::process::{Command, Output};

fn run_hidden_mode(mode: &str) -> (Output, tempfile::TempDir) {
    let directory = tempfile::tempdir().expect("temporary RSS dispatch directory");
    let missing_scenarios = directory.path().join("missing-scenarios");
    let report = directory.path().join("must-not-publish.json");
    let output = Command::new(env!("CARGO_BIN_EXE_tq-stack-overflow"))
        .args([mode, "--scenario-dir"])
        .arg(&missing_scenarios)
        .arg("--report-dir")
        .arg(&report)
        .output()
        .expect("spawn Stack Overflow RSS helper");
    assert!(!missing_scenarios.exists());
    assert!(!report.exists());
    (output, directory)
}

#[test]
fn internal_rss_control_exits_before_campaign_setup() {
    let (output, _directory) = run_hidden_mode("--internal-rss-control");
    assert!(
        output.status.success(),
        "RSS control failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, [] as [u8; 0]);
    assert_eq!(output.stderr, [] as [u8; 0]);
}

#[test]
fn internal_rss_probe_exits_before_campaign_setup() {
    let (output, _directory) = run_hidden_mode("--internal-rss-probe");
    assert!(
        output.status.success(),
        "RSS probe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, [] as [u8; 0]);
    assert_eq!(output.stderr, [] as [u8; 0]);
}

#[cfg(unix)]
#[test]
fn quick_stack_campaign_checkpoints_completed_rows_and_reaps_cancelled_child() {
    use nix::{errno::Errno, sys::signal::kill, unistd::Pid};
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        time::{Duration, Instant},
    };

    let directory = tempfile::tempdir().expect("temporary campaign directory");
    for name in ["jq", "yq"] {
        let path = directory.path().join(name);
        fs::write(&path, format!("#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then printf 'fake-{name} 1.0\\n'; exit 0; fi\nprintf 'null\\n'\n"))
            .expect("fake adapter");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("executable");
    }
    let tq = directory.path().join("slow-tq");
    fs::write(&tq, "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then printf 'fake-tq 1.0\\n'; exit 0; fi\nprintf '%s' \"$$\" > \"$BENCH_CHILD_PID\"\nexec sleep 10\n")
        .expect("slow adapter");
    fs::set_permissions(&tq, fs::Permissions::from_mode(0o755)).expect("executable");
    let checkpoint = directory.path().join("stack-quick.json");
    let pid_path = directory.path().join("child.pid");
    let started = Instant::now();
    let output = Command::new(env!("CARGO_BIN_EXE_tq-quick"))
        .args(["--budget-seconds", "3", "--"])
        .arg(env!("CARGO_BIN_EXE_tq-stack-overflow"))
        .args(["run", "--profile", "quick", "--output"])
        .arg(&checkpoint)
        .env("TQ_JQ", directory.path().join("jq"))
        .env("TQ_YQ", directory.path().join("yq"))
        .env("TQ_BIN", &tq)
        .env("BENCH_CHILD_PID", &pid_path)
        .output()
        .expect("bounded Stack run");
    assert!(
        started.elapsed() < Duration::from_secs(7),
        "quick Stack run exceeded deadline"
    );
    assert!(
        !output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(&checkpoint).expect("checkpoint")).expect("valid JSON");
    assert_eq!(report["final_status"], "incomplete");
    assert_eq!(report["execution"]["complete"], false);
    let rows = report["cases"].as_array().expect("checkpointed rows");
    assert!(
        rows.len() >= 2 && rows.len() < 150,
        "expected partial rows: {}",
        rows.len()
    );
    assert_eq!(rows[0]["adapter_id"], "jq-json");
    assert_eq!(rows[1]["adapter_id"], "yq-json");
    assert!(String::from_utf8_lossy(&output.stdout).contains("not run"));
    assert!(String::from_utf8_lossy(&output.stderr).contains(checkpoint.to_str().unwrap()));
    let pid: i32 = fs::read_to_string(pid_path)
        .expect("slow child ran")
        .parse()
        .expect("PID");
    assert_eq!(kill(Pid::from_raw(pid), None), Err(Errno::ESRCH));
}
