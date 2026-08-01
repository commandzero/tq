//! Compatibility subprocess isolation tests.

use std::{path::PathBuf, time::Duration};

use tq_test_support::compatibility::{Invocation, ProcessStatus, run_process};

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
    })
    .expect("process observation");

    assert_eq!(
        outcome.recorded_command,
        ["/usr/bin/true", "--argjson", "token", "<redacted>", "."]
    );
}
