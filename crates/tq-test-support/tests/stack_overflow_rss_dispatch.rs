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
