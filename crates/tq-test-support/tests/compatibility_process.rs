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
fn concurrent_children_do_not_inherit_other_invocations_capture_pipes() {
    const CALLERS: usize = 12;
    const ROUNDS: usize = 32;
    // Snapshot once before these callers create any capture endpoints. Refreshing
    // per invocation would incorrectly allow another active caller's pipes.
    let baseline = parent_pipe_identities();
    let ready = std::sync::Barrier::new(CALLERS);
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..CALLERS)
            .map(|_| {
                scope.spawn(|| {
                    let mut outcomes = Vec::with_capacity(ROUNDS);
                    for round in 0..ROUNDS {
                        ready.wait();
                        let invocation = capture_pipe_probe();
                        let outcome = if round % 2 == 0 {
                            run_process(&invocation)
                        } else {
                            run_process_with_environment_bounded(
                                &invocation,
                                &BTreeMap::new(),
                                4096,
                            )
                            .map(|result| result.outcome)
                        };
                        // Assert only after all rounds so a failure cannot strand
                        // the remaining callers at the next barrier.
                        outcomes.push(outcome);
                    }
                    outcomes
                })
            })
            .collect();
        for worker in workers {
            for (round, outcome) in worker
                .join()
                .expect("concurrent caller")
                .into_iter()
                .enumerate()
            {
                let outcome = outcome.expect("concurrent process observation");
                assert_eq!(outcome.status, ProcessStatus::Exited);
                assert_eq!(outcome.exit_code, Some(0));
                assert_eq!(outcome.stderr, [] as [u8; 0]);
                let observed = observed_pipe_identities(&outcome.stdout);
                let unexpected: Vec<_> = observed.difference(&baseline).collect();
                assert!(
                    unexpected.is_empty(),
                    "round {round}: inherited new capture pipe identities: {unexpected:?}"
                );
            }
        }
    });
}

#[cfg(target_os = "linux")]
const DESCRIPTOR_DIRECTORY: &str = "/proc/self/fd";
#[cfg(target_os = "macos")]
const DESCRIPTOR_DIRECTORY: &str = "/dev/fd";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PipeIdentity {
    descriptor: i32,
    device: u64,
    inode: u64,
}

fn pipe_identity(descriptor: i32, metadata: &std::fs::Metadata) -> PipeIdentity {
    use std::os::unix::fs::MetadataExt as _;

    PipeIdentity {
        descriptor,
        device: metadata.dev(),
        inode: metadata.ino(),
    }
}

fn descriptor_metadata(path: impl AsRef<std::path::Path>) -> std::io::Result<std::fs::Metadata> {
    let metadata = std::fs::metadata(&path)?;
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::FileTypeExt as _;

        if metadata.file_type().is_fifo() {
            // macOS stat(/dev/fd/N) describes the fdescfs entry, not the pipe.
            // Opening this pseudo-path duplicates the endpoint without waiting
            // for a peer; fstat on that owned duplicate supplies its real identity.
            let duplicate = match std::fs::File::open(&path) {
                Ok(file) => file,
                Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                    std::fs::OpenOptions::new().write(true).open(&path)?
                }
                Err(error) => return Err(error),
            };
            return duplicate.metadata();
        }
    }
    Ok(metadata)
}

fn parent_pipe_identities() -> std::collections::BTreeSet<PipeIdentity> {
    use std::os::unix::fs::FileTypeExt as _;

    // Cargo jobserver pipes and other caller-owned descriptors may legitimately
    // survive exec. Snapshot identity, not just fd numbers: reuse must not allow
    // a newly created capture endpoint to masquerade as an existing pipe.
    std::fs::read_dir(DESCRIPTOR_DIRECTORY)
        .expect("parent descriptor directory")
        .collect::<Result<Vec<_>, _>>()
        .expect("parent descriptor entries")
        .into_iter()
        .filter_map(|entry| {
            let descriptor: i32 = entry.file_name().to_str()?.parse().ok()?;
            if descriptor <= 2 {
                return None;
            }
            match descriptor_metadata(entry.path()) {
                Ok(metadata) if metadata.file_type().is_fifo() => {
                    Some(pipe_identity(descriptor, &metadata))
                }
                Ok(_) => None,
                // Concurrent tests may close a descriptor after readdir.
                Err(error)
                    if error.kind() == std::io::ErrorKind::NotFound
                        || error.raw_os_error() == Some(nix::errno::Errno::EBADF as i32) =>
                {
                    None
                }
                Err(error) => panic!("parent descriptor {descriptor}: {error}"),
            }
        })
        .collect()
}

fn capture_pipe_probe() -> Invocation {
    Invocation {
        executable: PathBuf::from("/usr/bin/perl"),
        args: vec![
            "-MFcntl=:mode".to_owned(),
            "-e".to_owned(),
            "opendir(my $directory, $ARGV[0]) or die $!;
             my @fds = grep { /^\\d+$/ && $_ > 2 } readdir($directory);
             closedir($directory);
             for my $fd (@fds) {
                 if (open(my $fh, '<&=' . $fd)) {
                     my @s = stat($fh);
                     print qq($fd $s[0] $s[1]\\n) if S_ISFIFO($s[2]);
                 }
             }"
            .to_owned(),
            DESCRIPTOR_DIRECTORY.to_owned(),
        ],
        stdin: Vec::new(),
        timeout: Duration::from_secs(2),
        current_dir: None,
        environment: BTreeMap::new(),
    }
}

fn observed_pipe_identities(stdout: &[u8]) -> std::collections::BTreeSet<PipeIdentity> {
    std::str::from_utf8(stdout)
        .expect("pipe identity output is UTF-8")
        .lines()
        .map(|line| {
            let mut fields = line.split_whitespace();
            let identity = PipeIdentity {
                descriptor: fields
                    .next()
                    .expect("descriptor")
                    .parse()
                    .expect("fd number"),
                device: fields
                    .next()
                    .expect("device")
                    .parse()
                    .expect("device number"),
                inode: fields.next().expect("inode").parse().expect("inode number"),
            };
            assert_eq!(fields.next(), None, "unexpected pipe identity field");
            identity
        })
        .collect()
}

#[test]
fn capture_pipe_probe_distinguishes_preexisting_pipes_and_reused_descriptors() {
    use std::os::fd::AsRawFd as _;

    const FIXTURE_ENV: &str = "TQ_CAPTURE_PIPE_IDENTITY_FIXTURE";
    if std::env::var_os(FIXTURE_ENV).is_none() {
        // Create deliberate inheritable pipes only in an isolated test process,
        // so they cannot contaminate other tests' concurrent baseline snapshots.
        let outcome = run_process(&Invocation {
            executable: std::env::current_exe().expect("integration test executable"),
            args: vec![
                "--exact".to_owned(),
                "capture_pipe_probe_distinguishes_preexisting_pipes_and_reused_descriptors"
                    .to_owned(),
                "--nocapture".to_owned(),
            ],
            stdin: Vec::new(),
            timeout: Duration::from_secs(2),
            current_dir: None,
            environment: BTreeMap::from([(FIXTURE_ENV.to_owned(), "1".to_owned())]),
        })
        .expect("isolated pipe identity fixture");
        assert_eq!(outcome.status, ProcessStatus::Exited);
        assert_eq!(outcome.exit_code, Some(0), "{outcome:?}");
        return;
    }

    let identity = |fd| {
        let metadata = descriptor_metadata(format!("{DESCRIPTOR_DIRECTORY}/{fd}"))
            .expect("owned pipe metadata");
        pipe_identity(fd, &metadata)
    };
    let (mut existing_read, existing_write) = nix::unistd::pipe().expect("inherited pipe");
    let existing = std::collections::BTreeSet::from([
        identity(existing_read.as_raw_fd()),
        identity(existing_write.as_raw_fd()),
    ]);
    let baseline = parent_pipe_identities();
    assert!(existing.is_subset(&baseline));
    let (new_read, new_write) = nix::unistd::pipe().expect("new inheritable pipe");
    let new = std::collections::BTreeSet::from([
        identity(new_read.as_raw_fd()),
        identity(new_write.as_raw_fd()),
    ]);
    let observe = || {
        let outcome = run_process(&capture_pipe_probe()).expect("pipe identity probe");
        assert_eq!(outcome.status, ProcessStatus::Exited);
        assert_eq!(outcome.exit_code, Some(0));
        assert!(outcome.stderr.is_empty(), "{outcome:?}");
        observed_pipe_identities(&outcome.stdout)
    };
    let observed = observe();
    assert!(
        existing.is_subset(&observed),
        "pre-existing pipes must really survive exec"
    );
    assert_eq!(
        observed
            .difference(&baseline)
            .copied()
            .collect::<std::collections::BTreeSet<_>>(),
        new
    );

    // Replacing an allowed fd with a different endpoint must still be detected.
    rustix::io::dup2(&new_read, &mut existing_read).expect("reuse baseline descriptor");
    let reused = identity(existing_read.as_raw_fd());
    assert!(!baseline.contains(&reused));
    assert!(observe().difference(&baseline).any(|pipe| *pipe == reused));
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
