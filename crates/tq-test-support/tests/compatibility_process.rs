//! Compatibility subprocess isolation tests.

#![cfg(any(target_os = "macos", target_os = "linux"))]

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
fn early_exit_treats_closed_stdin_as_completed() {
    let outcome = run_process(&Invocation {
        executable: PathBuf::from("/bin/sh"),
        args: vec!["-c".to_owned(), "exit 0".to_owned()],
        stdin: vec![b'x'; 1024 * 1024],
        timeout: Duration::from_secs(2),
        current_dir: None,
        environment: BTreeMap::new(),
    })
    .expect("early child exit with large stdin");

    assert_eq!(outcome.status, ProcessStatus::Exited);
    assert_eq!(outcome.exit_code, Some(0));
    assert_eq!(outcome.stdout, [] as [u8; 0]);
    assert_eq!(outcome.stderr, [] as [u8; 0]);
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

#[cfg(unix)]
#[test]
fn bounded_capture_cleans_descendant_holding_pipes_after_child_exit() {
    let outcome = run_process_with_environment_bounded(
        &Invocation {
            executable: PathBuf::from("/bin/sh"),
            args: vec![
                "-c".to_owned(),
                "printf before; sleep 2 & exit 0".to_owned(),
            ],
            stdin: Vec::new(),
            timeout: Duration::from_secs(5),
            current_dir: None,
            environment: BTreeMap::new(),
        },
        &BTreeMap::new(),
        4096,
    )
    .expect("descendant process observation");

    assert_eq!(outcome.output_limit, None);
    assert_eq!(outcome.outcome.status, ProcessStatus::Exited);
    assert_eq!(outcome.outcome.exit_code, Some(0));
    assert_eq!(outcome.outcome.stdout, b"before");
    assert!(
        outcome.outcome.wall_time_micros < 1_000_000,
        "capture drain waited for descendant: {} micros",
        outcome.outcome.wall_time_micros
    );
}

#[cfg(unix)]
#[test]
fn bounded_capture_kills_descendant_holding_pipes_after_timeout() {
    let outcome = run_process_with_environment_bounded(
        &Invocation {
            executable: PathBuf::from("/bin/sh"),
            args: vec![
                "-c".to_owned(),
                "printf before; sleep 2 & while :; do :; done".to_owned(),
            ],
            stdin: Vec::new(),
            timeout: Duration::from_millis(100),
            current_dir: None,
            environment: BTreeMap::new(),
        },
        &BTreeMap::new(),
        4096,
    )
    .expect("timed-out descendant observation");

    assert_eq!(outcome.output_limit, None);
    assert_eq!(outcome.outcome.status, ProcessStatus::TimedOut);
    assert_eq!(outcome.outcome.stdout, b"before");
    assert!(
        outcome.outcome.wall_time_micros < 1_000_000,
        "capture drain waited for descendant after timeout: {} micros",
        outcome.outcome.wall_time_micros
    );
}

#[cfg(unix)]
#[test]
fn unbounded_capture_cleans_descendant_holding_pipes_after_child_exit() {
    let outcome = run_process(&Invocation {
        executable: PathBuf::from("/bin/sh"),
        args: vec![
            "-c".to_owned(),
            "printf before; sleep 2 & exit 0".to_owned(),
        ],
        stdin: Vec::new(),
        timeout: Duration::from_secs(5),
        current_dir: None,
        environment: BTreeMap::new(),
    })
    .expect("unbounded descendant process observation");

    assert_eq!(outcome.status, ProcessStatus::Exited);
    assert_eq!(outcome.exit_code, Some(0));
    assert_eq!(outcome.stdout, b"before");
    assert!(
        outcome.wall_time_micros < 1_000_000,
        "unbounded capture drain waited for descendant: {} micros",
        outcome.wall_time_micros
    );
}

#[cfg(unix)]
#[test]
fn bounded_capture_returns_when_new_session_descendant_holds_pipes() {
    let started = std::time::Instant::now();
    let outcome = run_process_with_environment_bounded(
        &Invocation {
            executable: PathBuf::from("/usr/bin/perl"),
            args: vec![
                "-MPOSIX".to_owned(),
                "-e".to_owned(),
                format!(
                    "sysread(STDIN, my $discard, 4096); {}",
                    escaped_descendant_script("print 'before'; exit 0")
                ),
            ],
            stdin: vec![b'x'; 1024 * 1024],
            timeout: Duration::from_secs(2),
            current_dir: None,
            environment: BTreeMap::new(),
        },
        &BTreeMap::new(),
        4096,
    )
    .expect_err("escaped descendant must not produce a silently partial observation");

    assert_eq!(
        outcome.to_string(),
        "subprocess capture drain exceeded the bounded interval"
    );
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "capture drain waited for escaped descendant: {:?}",
        started.elapsed()
    );
}

#[cfg(unix)]
fn escaped_descendant_script(parent: &str) -> String {
    escaped_descendant_script_with_lifetime(parent, 2)
}

#[cfg(unix)]
fn escaped_descendant_script_with_lifetime(parent: &str, lifetime_seconds: u64) -> String {
    // Wait for the child to escape before the parent can exit or time out.
    // Otherwise group cleanup can win the race and the test proves nothing
    // about descriptors retained outside the owned process group.
    format!(
        "pipe(my $ready_read, my $ready_write) or die $!;
        my $child = fork(); defined($child) or die $!;
        if ($child == 0) {{
            close($ready_read);
            defined(POSIX::setsid()) or die $!;
            syswrite($ready_write, 'x') == 1 or die $!;
            close($ready_write);
            sleep {lifetime_seconds}; exit 0;
        }}
        close($ready_write);
        sysread($ready_read, my $ready, 1) == 1 or die $!;
        close($ready_read);
        {parent}"
    )
}

#[cfg(unix)]
#[test]
fn unbounded_capture_returns_when_new_session_descendant_holds_pipes() {
    let started = std::time::Instant::now();
    let outcome = run_process(&Invocation {
        executable: PathBuf::from("/usr/bin/perl"),
        args: vec![
            "-MPOSIX".to_owned(),
            "-e".to_owned(),
            escaped_descendant_script("print 'before'; exit 0"),
        ],
        stdin: Vec::new(),
        timeout: Duration::from_secs(2),
        current_dir: None,
        environment: BTreeMap::new(),
    })
    .expect_err("escaped descendant must not produce a silently partial observation");

    assert_eq!(
        outcome.to_string(),
        "subprocess capture drain exceeded the bounded interval"
    );
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "capture drain waited for escaped descendant: {:?}",
        started.elapsed()
    );
}

#[cfg(unix)]
#[test]
fn bounded_capture_timeout_keeps_prefix_when_new_session_descendant_holds_pipes() {
    let outcome = run_process_with_environment_bounded(
        &Invocation {
            executable: PathBuf::from("/usr/bin/perl"),
            args: vec![
                "-MPOSIX".to_owned(),
                "-e".to_owned(),
                escaped_descendant_script_with_lifetime(
                    "select STDOUT; $| = 1; print 'before'; while (1) { }",
                    3,
                ),
            ],
            stdin: Vec::new(),
            // Startup, fork, setsid, and the ready handshake must fit this
            // deadline; the margin is test scheduling allowance, not a
            // capture-policy threshold.
            timeout: Duration::from_secs(1),
            current_dir: None,
            environment: BTreeMap::new(),
        },
        &BTreeMap::new(),
        4096,
    )
    .expect("timed-out escaped descendant observation");

    assert_eq!(outcome.output_limit, None);
    assert_eq!(outcome.outcome.status, ProcessStatus::TimedOut);
    assert_eq!(outcome.outcome.stdout, b"before");
    assert!(
        outcome.outcome.wall_time_micros < 2_000_000,
        "capture drain waited for escaped descendant after timeout: {} micros",
        outcome.outcome.wall_time_micros
    );
}
