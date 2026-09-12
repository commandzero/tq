#![allow(missing_docs)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde_json::{Value, json};

fn write_report(path: &Path, final_status: &str) {
    write_report_for_host(path, final_status, "test", "test");
}

fn write_report_for_host(path: &Path, final_status: &str, os: &str, architecture: &str) {
    let report = json!({
        "schema_version": 1,
        "campaign_id": "test-campaign",
        "profile": "rapid",
        "environment": {
            "collected_at": "2026-01-01T00:00:00Z",
            "os": os,
            "kernel": null,
            "architecture": architecture,
            "logical_cpus": 1,
            "physical_cpus": null,
            "cpu_model": null,
            "memory_bytes": null,
            "filesystem": null,
            "power_settings": null,
            "compiler_profile": "test",
            "machine_identity": "test"
        },
        "corpus": [],
        "tools": [],
        "cases": [],
        "comparability": {"comparable": true, "reasons": []},
        "regression_gate": {
            "evaluated": false,
            "thresholds": null,
            "failures": []
        },
        "final_status": final_status
    });
    fs::write(
        path,
        serde_json::to_vec_pretty(&report).expect("serialize report"),
    )
    .expect("write report");
}

#[test]
fn render_only_succeeds_for_historical_failed_reports() {
    let directory = tempfile::tempdir().expect("temporary directory");
    for status in ["observed-failures", "regression"] {
        let report = directory.path().join(format!("{status}.json"));
        let markdown = directory.path().join(status);
        write_report(&report, status);
        fs::create_dir(&markdown).expect("create markdown directory");

        let output = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
            .args([
                "--render-only",
                report.to_str().expect("report path"),
                "--markdown-dir",
                markdown.to_str().expect("markdown path"),
            ])
            .output()
            .expect("run render-only benchmark command");

        assert!(
            output.status.success(),
            "render-only failed for {status}: stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        let rendered = String::from_utf8_lossy(&output.stdout);
        let status_name = match status {
            "observed-failures" => "ObservedFailures",
            "regression" => "Regression",
            _ => unreachable!("test status should be known"),
        };
        assert!(rendered.contains(status_name));
    }
}

#[test]
fn render_only_accepts_multiple_host_reports_without_overwriting_host_sections() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let linux_report = directory.path().join("linux.json");
    let macos_report = directory.path().join("macos.json");
    let markdown = directory.path().join("markdown");
    fs::create_dir(&markdown).expect("create markdown directory");
    write_report_for_host(&linux_report, "passed", "linux", "x86_64");
    write_report_for_host(&macos_report, "passed", "macos", "aarch64");

    let output = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
        .args([
            "--render-only",
            linux_report.to_str().expect("linux report path"),
            "--render-only",
            macos_report.to_str().expect("macos report path"),
            "--markdown-dir",
            markdown.to_str().expect("markdown path"),
        ])
        .output()
        .expect("run multi-host render-only benchmark command");

    assert!(
        output.status.success(),
        "multi-host render-only failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let index = fs::read_to_string(markdown.join("index.md")).expect("rendered index");
    assert_eq!(index.matches("### Host: linux / x86_64 / rapid").count(), 1);
    assert_eq!(
        index.matches("### Host: macos / aarch64 / rapid").count(),
        1
    );
}

#[cfg(unix)]
#[test]
fn failed_run_writes_report_returns_failure_and_reports_row_progress() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let jq = fake_tool(directory.path(), "jq", "null");
    let yq = fake_tool(directory.path(), "yq", "null");
    let tq = fake_tool(directory.path(), "tq", "1");
    let report = directory.path().join("run.json");

    let output = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
        .args([
            "run",
            "--profile",
            "smoke",
            "--max-samples",
            "1",
            "--case",
            "benchmark.startup",
            "--output",
            report.to_str().expect("report path"),
        ])
        .env("TQ_JQ", jq)
        .env("TQ_YQ", yq)
        .env("TQ_BIN", tq)
        .output()
        .expect("run benchmark command");

    assert_eq!(output.status.code(), Some(1));
    assert!(report.is_file(), "failed run should preserve its report");
    let report: Value =
        serde_json::from_slice(&fs::read(report).expect("read report")).expect("parse report");
    assert_eq!(report["final_status"], "observed-failures");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "workload=benchmark.startup dataset=startup adapter=tq-json outcome=Incorrect"
        )
    );
    assert!(
        stderr.contains("workload=benchmark.startup dataset=startup adapter=yq-json outcome=Timed")
    );
}

#[cfg(unix)]
#[test]
fn missing_campaign_executable_override_aborts_before_writing_a_report() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let report = directory.path().join("preflight.json");

    let output = Command::new(env!("CARGO_BIN_EXE_tq-bench"))
        .args([
            "run",
            "--profile",
            "smoke",
            "--output",
            report.to_str().expect("report path"),
        ])
        // Discovery also searches repository-local reference builds, so an
        // empty PATH alone does not make the executable unavailable.
        .env("TQ_JQ", directory.path().join("missing-jq"))
        .output()
        .expect("run benchmark preflight");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        !report.exists(),
        "failed executable discovery must not write a report"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("configured Jq executable is not usable:"),
        "unexpected executable-discovery stderr: {stderr}"
    );
}

#[cfg(unix)]
fn fake_tool(directory: &Path, name: &str, output: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let path = directory.join(name);
    let script = format!(
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n  printf 'fake-{name} 1.0\\n'\n  exit 0\nfi\nprintf '{output}\\n'\n"
    );
    fs::write(&path, script).expect("write fake executable");
    let mut permissions = fs::metadata(&path).expect("fake metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("make fake executable");
    path
}
