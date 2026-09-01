//! End-to-end harness boundaries using controlled fake executables.

#![cfg(unix)]

use std::{collections::BTreeMap, fs, os::unix::fs::PermissionsExt, path::Path, time::Duration};

use tq_test_support::compatibility::{
    ErrorClass, Invocation, ProcessStatus, normalize_jq, normalize_raw, run_process,
};

fn fake(directory: &Path, name: &str, body: &str) -> std::path::PathBuf {
    let path = directory.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write fake executable");
    let mut permissions = fs::metadata(&path).expect("fake metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("make fake executable");
    path
}

fn invoke(
    executable: std::path::PathBuf,
    timeout: Duration,
) -> tq_test_support::compatibility::ProcessOutcome {
    run_process(&Invocation {
        executable,
        args: Vec::new(),
        stdin: b"fixture".to_vec(),
        timeout,
        current_dir: None,
        environment: BTreeMap::new(),
    })
    .expect("fake process")
}

#[test]
fn controlled_value_and_error_outputs_are_normalized() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let value = invoke(
        fake(directory.path(), "value", "printf '{\"ok\":true}\\n'"),
        Duration::from_secs(1),
    );
    assert_eq!(
        normalize_jq(&value).unwrap().results,
        [serde_json::json!({"ok": true})]
    );

    let error = invoke(
        fake(
            directory.path(),
            "error",
            "printf 'cannot index number with string' >&2; exit 5",
        ),
        Duration::from_secs(1),
    );
    assert_eq!(
        normalize_raw(tq_test_support::compatibility::ToolKind::Jq, &error).error_class,
        Some(ErrorClass::RuntimeTypePath)
    );
}

#[test]
fn controlled_timeout_signal_and_malformed_output_are_distinct() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let timeout = invoke(
        fake(directory.path(), "timeout", "while :; do :; done"),
        Duration::from_millis(30),
    );
    assert_eq!(timeout.status, ProcessStatus::TimedOut);

    let signal = invoke(
        fake(directory.path(), "signal", "kill -TERM $$"),
        Duration::from_secs(1),
    );
    assert_eq!(signal.status, ProcessStatus::Signaled);

    let malformed = invoke(
        fake(directory.path(), "malformed", "printf 'not-json'"),
        Duration::from_secs(1),
    );
    assert!(normalize_jq(&malformed).is_err());
}
