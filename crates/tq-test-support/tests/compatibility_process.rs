//! Compatibility subprocess isolation tests.

use std::{collections::BTreeMap, path::PathBuf, time::Duration};

use tq_test_support::compatibility::{
    Invocation, OutputStream, ProcessStatus, run_process, run_process_with_environment,
    run_process_with_environment_bounded,
};

#[test]
fn environment_overrides_apply_only_to_the_requested_child() {
    let variable = "TQ_COMPAT_MANUAL_COLOR_TEST";
    let parent_value = std::env::var_os(variable);
    let invocation = Invocation {
        executable: PathBuf::from("/bin/sh"),
        args: vec![
            "-c".to_owned(),
            format!("printf '%s' \"${{{variable}-unset}}\""),
        ],
        stdin: Vec::new(),
        timeout: Duration::from_secs(2),
        current_dir: None,
        environment: BTreeMap::from([(variable.to_owned(), "base".to_owned())]),
    };
    let before = run_process(&invocation).expect("original environment");
    let changed = run_process_with_environment(
        &invocation,
        &BTreeMap::from([(variable.to_owned(), "1;31".to_owned())]),
    )
    .expect("overridden environment");
    let after = run_process(&invocation).expect("unchanged environment");
    assert_eq!(before.stdout, b"base");
    assert_eq!(changed.stdout, b"1;31");
    assert_eq!(before.stdout, after.stdout);
    assert_eq!(std::env::var_os(variable), parent_value);
}

#[test]
fn stdout_stderr_stdin_and_exit_status_are_captured_separately() {
    let outcome = run_process(&Invocation {
        executable: PathBuf::from("/bin/sh"),
        args: vec![
            "-c".to_owned(),
            "read value; printf 'out:%s' \"$value\"; printf 'diagnostic' >&2; exit 7".to_owned(),
        ],
        stdin: b"input\n".to_vec(),
        timeout: Duration::from_secs(2),
        current_dir: None,
        environment: BTreeMap::new(),
    })
    .expect("process observation");

    assert_eq!(outcome.status, ProcessStatus::Exited);
    assert_eq!(outcome.exit_code, Some(7));
    assert_eq!(outcome.stdout, b"out:input");
    assert_eq!(outcome.stderr, b"diagnostic");
}

#[test]
fn runaway_process_is_killed_and_classified_timeout() {
    let outcome = run_process(&Invocation {
        executable: PathBuf::from("/bin/sh"),
        args: vec!["-c".to_owned(), "while :; do :; done".to_owned()],
        stdin: Vec::new(),
        timeout: Duration::from_millis(30),
        current_dir: None,
        environment: BTreeMap::new(),
    })
    .expect("timeout observation");

    assert_eq!(outcome.status, ProcessStatus::TimedOut);
    assert!(outcome.wall_time_micros < 1_000_000);
}

#[test]
fn recorded_command_redacts_structured_argument_values() {
    let outcome = run_process(&Invocation {
        executable: PathBuf::from("/usr/bin/true"),
        args: vec![
            "--argjson".to_owned(),
            "token".to_owned(),
            "secret-value".to_owned(),
            ".".to_owned(),
        ],
        stdin: Vec::new(),
        timeout: Duration::from_secs(1),
        current_dir: None,
        environment: BTreeMap::new(),
    })
    .expect("process observation");

    assert_eq!(
        outcome.recorded_command,
        ["/usr/bin/true", "--argjson", "token", "<redacted>", "."]
    );
}

#[test]
fn bounded_capture_terminates_continuous_stdout_and_records_the_limit() {
    let outcome = run_process_with_environment_bounded(
        &Invocation {
            executable: PathBuf::from("/bin/sh"),
            args: vec!["-c".to_owned(), "while :; do printf x; done".to_owned()],
            stdin: Vec::new(),
            timeout: Duration::from_secs(5),
            current_dir: None,
            environment: BTreeMap::new(),
        },
        &BTreeMap::new(),
        4096,
    )
    .expect("bounded process observation");

    let limit = outcome.output_limit.expect("stdout limit");
    assert_eq!(limit.stream, OutputStream::Stdout);
    assert_eq!(limit.limit, 4096);
    assert_eq!(limit.observed_bytes, 4097);
    assert_eq!(outcome.outcome.stdout.len(), 4096);
    assert!(outcome.outcome.wall_time_micros < 2_000_000);
}

#[test]
fn bounded_capture_terminates_continuous_stderr_and_preserves_stdout() {
    let outcome = run_process_with_environment_bounded(
        &Invocation {
            executable: PathBuf::from("/bin/sh"),
            args: vec![
                "-c".to_owned(),
                "printf ok; while :; do printf x >&2; done".to_owned(),
            ],
            stdin: Vec::new(),
            timeout: Duration::from_secs(5),
            current_dir: None,
            environment: BTreeMap::new(),
        },
        &BTreeMap::new(),
        4096,
    )
    .expect("bounded process observation");

    let limit = outcome.output_limit.expect("stderr limit");
    assert_eq!(limit.stream, OutputStream::Stderr);
    assert_eq!(limit.limit, 4096);
    assert_eq!(limit.observed_bytes, 4097);
    assert_eq!(outcome.outcome.stdout, b"ok");
    assert_eq!(outcome.outcome.stderr.len(), 4096);
}

#[test]
fn bounded_capture_marks_output_that_exceeds_the_limit_before_exit() {
    let outcome = run_process_with_environment_bounded(
        &Invocation {
            executable: PathBuf::from("/bin/sh"),
            args: vec![
                "-c".to_owned(),
                "i=0; while [ \"$i\" -lt 5000 ]; do printf x; i=$((i + 1)); done".to_owned(),
            ],
            stdin: Vec::new(),
            timeout: Duration::from_secs(2),
            current_dir: None,
            environment: BTreeMap::new(),
        },
        &BTreeMap::new(),
        4096,
    )
    .expect("bounded process observation");

    let limit = outcome.output_limit.expect("stdout limit");
    assert_eq!(limit.stream, OutputStream::Stdout);
    assert_eq!(outcome.outcome.stdout.len(), 4096);
}

#[test]
fn bounded_capture_preserves_complete_legacy_sized_output() {
    let outcome = run_process_with_environment_bounded(
        &Invocation {
            executable: PathBuf::from("/bin/sh"),
            args: vec![
                "-c".to_owned(),
                "printf out; printf err >&2; exit 7".to_owned(),
            ],
            stdin: Vec::new(),
            timeout: Duration::from_secs(2),
            current_dir: None,
            environment: BTreeMap::new(),
        },
        &BTreeMap::new(),
        4096,
    )
    .expect("bounded process observation");

    assert_eq!(outcome.output_limit, None);
    assert_eq!(outcome.outcome.status, ProcessStatus::Exited);
    assert_eq!(outcome.outcome.exit_code, Some(7));
    assert_eq!(outcome.outcome.stdout, b"out");
    assert_eq!(outcome.outcome.stderr, b"err");
}
