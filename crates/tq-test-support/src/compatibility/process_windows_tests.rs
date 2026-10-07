//! Focused native Windows process tests; run with `process::windows::tests`.

use super::*;
use std::{
    io::{Read as _, Write as _},
    path::PathBuf,
    sync::{Arc, Barrier, Mutex},
};

use process_wrap::tokio::CommandWrapper;

fn powershell(script: &str) -> Invocation {
    let executable = std::env::var_os("SystemRoot")
        .map_or_else(|| PathBuf::from("C:/Windows"), PathBuf::from)
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    Invocation {
        executable,
        args: vec![
            "-NoLogo".into(),
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-Command".into(),
            script.into(),
        ],
        stdin: Vec::new(),
        timeout: Duration::from_secs(10),
        current_dir: None,
        environment: BTreeMap::new(),
    }
}

#[test]
fn exact_binary_streams_and_nonzero_exit() {
    let mut invocation = powershell(
        "$i=[Console]::OpenStandardInput(); $o=[Console]::OpenStandardOutput(); $i.CopyTo($o); $o.Flush(); $e=[Console]::OpenStandardError(); $b=[byte[]](0,255,13,10,128); $e.Write($b,0,$b.Length); $e.Flush(); exit 7",
    );
    invocation.stdin = (0..=u8::MAX).cycle().take(256 * 1024).collect();
    let result = run(&invocation, &BTreeMap::new(), None).expect("binary capture");
    assert_eq!(result.outcome.stdout, invocation.stdin);
    assert_eq!(result.outcome.stderr, [0, 255, 13, 10, 128]);
    assert_eq!(result.outcome.exit_code, Some(7));
    assert_eq!(result.outcome.status, ProcessStatus::Exited);
    assert_eq!(result.outcome.signal, None);
}

#[test]
fn delayed_stdin_reader_receives_every_byte_before_eof() {
    for length in [0, 1, CHUNK - 1, CHUNK, CHUNK + 1, CHUNK * 3 + 17] {
        let mut invocation = powershell(
            "Start-Sleep -Milliseconds 150; $i=[Console]::OpenStandardInput(); $o=[Console]::OpenStandardOutput(); $i.CopyTo($o); $o.Flush()",
        );
        invocation.stdin = (0..=u8::MAX).cycle().take(length).collect();
        let outcome = run(&invocation, &BTreeMap::new(), None).expect("delayed stdin capture");
        assert_eq!(outcome.outcome.exit_code, Some(0));
        assert_eq!(
            outcome.outcome.stdout, invocation.stdin,
            "stdin length {length}"
        );
    }
}

#[test]
fn environment_and_working_directory_are_child_local() {
    let directory = tempfile::tempdir().expect("directory");
    let mut invocation = powershell(
        "[Console]::Write($env:TQ_CAPTURE_TEST); [Console]::Error.Write((Get-Location).Path)",
    );
    invocation
        .environment
        .insert("TQ_CAPTURE_TEST".into(), "base".into());
    invocation.current_dir = Some(directory.path().to_owned());
    let parent = std::env::var_os("TQ_CAPTURE_TEST");
    let result = run(
        &invocation,
        &BTreeMap::from([("TQ_CAPTURE_TEST".into(), "override".into())]),
        None,
    )
    .expect("capture");
    assert_eq!(result.outcome.stdout, b"override");
    assert_eq!(
        PathBuf::from(String::from_utf8(result.outcome.stderr).expect("path")),
        directory.path()
    );
    assert_eq!(std::env::var_os("TQ_CAPTURE_TEST"), parent);
}

#[test]
fn early_exit_does_not_stick_large_stdin_write() {
    let mut invocation = powershell("exit 0");
    invocation.stdin = vec![42; 8 * 1024 * 1024];
    let result = run(&invocation, &BTreeMap::new(), None).expect("early exit");
    assert_eq!(result.outcome.exit_code, Some(0));
}

#[test]
fn deadline_cancels_unread_stdin() {
    let mut invocation = powershell("Start-Sleep -Seconds 60");
    invocation.stdin = vec![42; 8 * 1024 * 1024];
    invocation.timeout = Duration::from_millis(500);
    let started = Instant::now();
    let result = run(&invocation, &BTreeMap::new(), None).expect("timeout");
    assert_eq!(result.outcome.status, ProcessStatus::TimedOut);
    assert!(started.elapsed() < Duration::from_secs(3));
}

#[test]
fn bounded_stdout_and_stderr_report_first_rejected_byte() {
    for (stream, console) in [
        (OutputStream::Stdout, "OpenStandardOutput"),
        (OutputStream::Stderr, "OpenStandardError"),
    ] {
        let invocation = powershell(&format!(
            "$s=[Console]::{console}(); $b=New-Object byte[] 8192; while($true){{$s.Write($b,0,$b.Length);$s.Flush()}}"
        ));
        let result = run(&invocation, &BTreeMap::new(), Some(4096)).expect("bounded capture");
        assert_eq!(
            result.output_limit,
            Some(OutputLimit {
                stream,
                limit: 4096,
                observed_bytes: 4097
            })
        );
        let retained = if stream == OutputStream::Stdout {
            result.outcome.stdout
        } else {
            result.outcome.stderr
        };
        assert_eq!(retained, vec![0; 4096]);
        assert_ne!(result.outcome.status, ProcessStatus::TimedOut);
    }
}

#[test]
fn exact_limit_is_not_overflow_and_zero_limit_detects_output() {
    let invocation = powershell("[Console]::Write('abcd')");
    let result = run(&invocation, &BTreeMap::new(), Some(4)).expect("exact limit");
    assert_eq!(result.output_limit, None);
    assert_eq!(result.outcome.stdout, b"abcd");
    let result = run(&invocation, &BTreeMap::new(), Some(0)).expect("zero limit");
    assert_eq!(
        result.output_limit,
        Some(OutputLimit {
            stream: OutputStream::Stdout,
            limit: 0,
            observed_bytes: 1
        })
    );
    assert_eq!(result.outcome.stdout, [] as [u8; 0]);
}

#[test]
fn descendants_holding_pipes_are_killed_after_root_exit_and_timeout() {
    // UseShellExecute=false and no redirection causes the descendant to inherit
    // the root's standard handles. Its later marker detects leaked descendants.
    for (timeout, limit) in [(false, None), (false, Some(4096)), (true, Some(4096))] {
        let directory = tempfile::tempdir().expect("directory");
        let marker = directory.path().join("leaked");
        let marker_literal = marker.to_string_lossy().replace('\'', "''");
        let descendant =
            format!("Start-Sleep -Seconds 4; [IO.File]::WriteAllText('{marker_literal}','leaked')");
        let script = format!(
            "$s=New-Object Diagnostics.ProcessStartInfo; $s.FileName=(Get-Process -Id $PID).Path; $s.UseShellExecute=$false; $s.Arguments=\"-NoProfile -NonInteractive -Command {descendant}\"; $p=[Diagnostics.Process]::Start($s); [Console]::Write('before'); {}",
            if timeout {
                "Start-Sleep -Seconds 60"
            } else {
                "exit 0"
            },
        );
        let mut invocation = powershell(&script);
        if timeout {
            invocation.timeout = Duration::from_secs(2);
        }
        let result = run(&invocation, &BTreeMap::new(), limit).expect("descendant cleanup");
        assert_eq!(result.outcome.stdout, b"before");
        assert_eq!(
            result.outcome.status,
            if timeout {
                ProcessStatus::TimedOut
            } else {
                ProcessStatus::Exited
            }
        );
        std::thread::sleep(Duration::from_secs(5));
        assert!(!marker.exists(), "descendant survived Job cleanup");
    }
}

#[test]
fn public_dispatch_supports_windows_and_async_callers() {
    let invocation = powershell("[Console]::Write('ok')");
    let outcome = super::super::run_process(&invocation).expect("public Windows dispatch");
    assert_eq!(outcome.stdout, b"ok");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("caller runtime");
    runtime.block_on(async {
        let outcome =
            super::super::run_process_with_environment_bounded(&invocation, &BTreeMap::new(), 2)
                .expect("nested caller");
        assert_eq!(outcome.outcome.stdout, b"ok");
        assert_eq!(outcome.output_limit, None);
    });
}

#[test]
fn spawn_errors_are_not_reported_as_unsupported() {
    let mut invocation = powershell("");
    invocation.executable = PathBuf::from("C:/definitely-not-an-executable.exe");
    assert!(matches!(
        super::super::run_process(&invocation),
        Err(ProcessError::Io(_))
    ));
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime")
}

fn descendant_invocation(directory: &std::path::Path) -> Invocation {
    let mut invocation = powershell(
        "$s=New-Object Diagnostics.ProcessStartInfo; $s.FileName=(Get-Process -Id $PID).Path; $s.UseShellExecute=$false; $s.Arguments='-NoProfile -NonInteractive -Command Start-Sleep -Seconds 60'; $p=[Diagnostics.Process]::Start($s); [IO.File]::WriteAllText($env:TQ_CAPTURE_READY,\"$PID,$($p.Id)\"); [Console]::Write('ready'); Start-Sleep -Seconds 60",
    );
    invocation.environment.insert(
        "TQ_CAPTURE_READY".into(),
        directory.join("ready").to_string_lossy().into_owned(),
    );
    invocation
}

async fn ready_processes(invocation: &Invocation) -> Vec<u32> {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Ok(contents) =
                std::fs::read_to_string(&invocation.environment["TQ_CAPTURE_READY"])
                && let Ok(ids) = contents
                    .split(',')
                    .map(str::parse::<u32>)
                    .collect::<Result<Vec<_>, _>>()
                && ids.len() == 2
            {
                return ids;
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    })
    .await
    .expect("root and descendant readiness")
}

fn assert_processes_exit(ids: &[u32]) {
    let ids = ids.iter().map(u32::to_string).collect::<Vec<_>>().join(",");
    let invocation = powershell(&format!(
        "foreach($id in @({ids})){{try{{$p=[Diagnostics.Process]::GetProcessById($id)}}catch [ArgumentException]{{continue}}; if(-not $p.WaitForExit(3000)){{[Console]::Error.Write('surviving process '+$id); exit 1}}}}; exit 0"
    ));
    let outcome = run(&invocation, &BTreeMap::new(), Some(4096)).expect("process liveness check");
    assert_eq!(
        outcome.outcome.exit_code,
        Some(0),
        "{}",
        String::from_utf8_lossy(&outcome.outcome.stderr)
    );
}

#[test]
fn capture_observation_error_closes_job_and_kills_descendants() {
    let directory = tempfile::tempdir().expect("directory");
    let invocation = descendant_invocation(directory.path());
    let ids = runtime().block_on(async {
        let (child, pipes) = spawn_capture(&invocation, &BTreeMap::new())
            .await
            .expect("spawn");
        let ids = ready_processes(&invocation).await;
        let child = Box::new(FailingLifecycle {
            child,
            failure: LifecycleFailure::Observe,
        });
        let error = capture_child(&invocation, None, child, pipes, Instant::now())
            .await
            .expect_err("forced observation error");
        assert!(error.to_string().contains("forced child observation error"));
        ids
    });
    assert_processes_exit(&ids);
}

#[derive(Debug)]
enum LifecycleFailure {
    Observe,
    Terminate,
}

#[derive(Debug)]
struct FailingLifecycle {
    child: Box<dyn ChildWrapper>,
    failure: LifecycleFailure,
}

impl ChildWrapper for FailingLifecycle {
    fn inner(&self) -> &dyn ChildWrapper {
        self.child.as_ref()
    }
    fn inner_mut(&mut self) -> &mut dyn ChildWrapper {
        self.child.as_mut()
    }
    fn into_inner(self: Box<Self>) -> Box<dyn ChildWrapper> {
        self.child
    }
    fn try_wait(&mut self) -> io::Result<Option<std::process::ExitStatus>> {
        if matches!(self.failure, LifecycleFailure::Observe) {
            Err(io::Error::other("forced child observation error"))
        } else {
            self.child.try_wait()
        }
    }
    fn start_kill(&mut self) -> io::Result<()> {
        if matches!(self.failure, LifecycleFailure::Terminate) {
            Err(io::Error::other("forced Job termination error"))
        } else {
            self.child.start_kill()
        }
    }
}

#[test]
fn termination_error_preserves_error_and_kill_on_close_cleanup() {
    let directory = tempfile::tempdir().expect("directory");
    let mut invocation = descendant_invocation(directory.path());
    invocation.timeout = Duration::ZERO;
    let ids = runtime().block_on(async {
        let (child, pipes) = spawn_capture(&invocation, &BTreeMap::new())
            .await
            .expect("spawn");
        let ids = ready_processes(&invocation).await;
        let error = capture_child(
            &invocation,
            None,
            Box::new(FailingLifecycle {
                child,
                failure: LifecycleFailure::Terminate,
            }),
            pipes,
            Instant::now(),
        )
        .await
        .expect_err("forced termination failure");
        assert!(error.to_string().contains("forced Job termination error"));
        ids
    });
    assert_processes_exit(&ids);
}

#[test]
fn cancelling_capture_future_closes_job_without_capture_workers() {
    let directory = tempfile::tempdir().expect("directory");
    let invocation = descendant_invocation(directory.path());
    let ids = runtime().block_on(async {
        let pending = capture(&invocation, &invocation.environment, None);
        tokio::pin!(pending);
        tokio::select! {
            result = &mut pending => panic!("capture finished before cancellation: {result:?}"),
            ids = ready_processes(&invocation) => ids,
        }
    });
    assert_processes_exit(&ids);
}

#[derive(Debug)]
struct FailSpawnHook {
    pid: Arc<Mutex<Option<u32>>>,
    after_assignment: bool,
}

impl CommandWrapper for FailSpawnHook {
    fn post_spawn(
        &mut self,
        _command: &mut Command,
        child: &mut tokio::process::Child,
        _core: &CommandWrap,
    ) -> io::Result<()> {
        if self.after_assignment {
            return Ok(());
        }
        *self.pid.lock().expect("PID mutex") = child.id();
        Err(io::Error::other("forced pre-assignment spawn-hook error"))
    }

    fn wrap_child(
        &mut self,
        child: Box<dyn ChildWrapper>,
        _core: &CommandWrap,
    ) -> io::Result<Box<dyn ChildWrapper>> {
        *self.pid.lock().expect("PID mutex") = child.id();
        Err(io::Error::other("forced post-assignment spawn-hook error"))
    }
}

#[test]
fn spawn_hook_failures_terminate_suspended_and_assigned_roots() {
    for after_assignment in [false, true] {
        let pid = Arc::new(Mutex::new(None));
        runtime().block_on(async {
            let invocation = powershell("Start-Sleep -Seconds 60");
            let (_pipes, files) = CapturePipes::new().await.expect("named stdio pipes");
            let mut command = isolated_command(&invocation, &BTreeMap::new(), files);
            command.wrap(FailSpawnHook {
                pid: Arc::clone(&pid),
                after_assignment,
            });
            let error = command.spawn().expect_err("forced spawn-hook failure");
            assert!(error.to_string().contains("spawn-hook error"));
        });
        assert_processes_exit(&[pid.lock().expect("PID mutex").expect("spawned root PID")]);
    }
}

#[test]
fn concurrent_captures_keep_streams_limits_and_deadlines_isolated() {
    const WORKERS: usize = 6;
    let barrier = Arc::new(Barrier::new(WORKERS));
    let started = Instant::now();
    std::thread::scope(|scope| {
        let workers = (0..WORKERS).map(|worker| {
            let barrier = Arc::clone(&barrier);
            scope.spawn(move || {
                barrier.wait();
                for round in 0..3 {
                    let token = format!("worker-{worker}-round-{round}");
                    let mut invocation = powershell("$i=[Console]::OpenStandardInput(); $o=[Console]::OpenStandardOutput(); $i.CopyTo($o); [Console]::Error.Write($env:TQ_CAPTURE_TOKEN)");
                    invocation.stdin = token.as_bytes().repeat(1024);
                    invocation.environment.insert("TQ_CAPTURE_TOKEN".into(), token.clone());
                    let outcome = run(&invocation, &BTreeMap::new(), Some(64 * 1024)).expect("concurrent capture");
                    assert_eq!(outcome.outcome.stdout, invocation.stdin);
                    assert_eq!(outcome.outcome.stderr, token.as_bytes());
                    assert_eq!(outcome.output_limit, None);
                    let mut blocked = powershell("Start-Sleep -Seconds 60");
                    blocked.stdin = vec![42; 1024 * 1024];
                    blocked.timeout = Duration::from_millis(250);
                    assert_eq!(run(&blocked, &BTreeMap::new(), Some(1024)).expect("concurrent deadline").outcome.status, ProcessStatus::TimedOut);
                    let flood = powershell("$o=[Console]::OpenStandardOutput(); $b=New-Object byte[] 8192; while($true){$o.Write($b,0,$b.Length)}");
                    let outcome = run(&flood, &BTreeMap::new(), Some(1024)).expect("concurrent limit");
                    assert_eq!(outcome.outcome.stdout.len(), 1024);
                    assert_eq!(outcome.output_limit.expect("output limit").observed_bytes, 1025);
                }
            })
        }).collect::<Vec<_>>();
        for worker in workers {
            worker.join().expect("concurrent capture worker");
        }
    });
    assert!(
        started.elapsed() < Duration::from_secs(45),
        "capture workers failed to finish promptly"
    );
}

#[test]
fn capture_works_inside_non_breakaway_outer_job() {
    let helper = format!(
        "{}::nested_job_helper",
        module_path!().split_once("::").expect("crate prefix").1
    );
    let invocation = Invocation {
        executable: std::env::current_exe().expect("test executable"),
        args: vec![
            "--exact".into(),
            helper,
            "--ignored".into(),
            "--nocapture".into(),
        ],
        stdin: Vec::new(),
        timeout: Duration::from_secs(20),
        current_dir: None,
        environment: BTreeMap::new(),
    };
    // The outer Job has neither BREAKAWAY_OK nor SILENT_BREAKAWAY_OK. The helper
    // and its descendants must remain in it while creating the inner capture Job.
    let outcome = run(&invocation, &BTreeMap::new(), Some(64 * 1024)).expect("nested Job capture");
    assert_eq!(
        outcome.outcome.exit_code,
        Some(0),
        "{}",
        String::from_utf8_lossy(&outcome.outcome.stdout)
    );
    assert!(String::from_utf8_lossy(&outcome.outcome.stdout).contains("1 passed"));
}

#[test]
#[ignore = "launched in a non-breakaway outer Job by capture_works_inside_non_breakaway_outer_job"]
fn nested_job_helper() {
    exact_binary_streams_and_nonzero_exit();
    capture_observation_error_closes_job_and_kills_descendants();
    deadline_cancels_unread_stdin();
}

#[test]
fn read_errors_preserve_io_error_instead_of_becoming_eof() {
    let mut stream = CapturedStream::new(OutputStream::Stderr, None);
    let error = stream
        .accept_read(
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "forced capture read error",
            )),
            &[],
        )
        .expect_err("read failure");
    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    assert!(!stream.eof);
    stream
        .accept_read(Err(io::Error::from(io::ErrorKind::BrokenPipe)), &[])
        .expect("closed pipe");
    assert!(stream.eof);
}

#[test]
fn held_open_outside_job_clients_do_not_stall_runtime_shutdown() {
    // A separate watchdog process bounds this test even if runtime Drop regresses
    // to waiting forever for a Blocking<ArcFile> task. No harness capture pipes.
    let helper = format!(
        "{}::outside_job_client_helper",
        module_path!().split_once("::").expect("crate prefix").1
    );
    let mut child = std::process::Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", &helper, "--ignored", "--nocapture"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("shutdown watchdog helper");
    let started = Instant::now();
    loop {
        if let Some(exit) = child.try_wait().expect("helper status") {
            assert!(exit.success(), "outside-Job client helper failed: {exit}");
            break;
        }
        if started.elapsed() >= Duration::from_secs(8) {
            child.kill().expect("terminate stuck helper");
            child.wait().expect("reap stuck helper");
            panic!("capture/runtime shutdown blocked on an outside-Job client");
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

#[derive(Clone, Copy)]
enum HeldClientCase {
    Timeout,
    NaturalExit,
    OutputLimit,
    ObservationError,
    Cancellation,
}

fn assert_no_tokio_process_stdio(child: &dyn ChildWrapper) {
    let native = child.try_inner_child().expect("native child");
    assert!(native.stdin.is_none());
    assert!(native.stdout.is_none());
    assert!(native.stderr.is_none());
}

fn capture_with_held_clients(case: HeldClientCase) {
    let runtime = runtime();
    let mut invocation = powershell(if matches!(case, HeldClientCase::NaturalExit) {
        "exit 0"
    } else {
        "Start-Sleep -Seconds 60"
    });
    invocation.stdin = vec![42; 1024 * 1024];
    invocation.timeout = if matches!(case, HeldClientCase::Timeout) {
        Duration::from_millis(150)
    } else {
        Duration::from_secs(5)
    };
    let started = Instant::now();
    let (child, pipes, mut held_clients) = runtime.block_on(async {
        let (pipes, mut files) = CapturePipes::new().await.expect("overlapped servers");
        // These synchronous client duplicates belong to this helper, NOT to the
        // captured Job. Keep its stdin unread and both output writers open even
        // after the captured root dies. The tiny prefix fits the known buffer.
        files.stdout.write_all(b"prefix").expect("retained prefix");
        let held_clients = (
            files.stdin.try_clone().expect("stdin clone"),
            files.stdout.try_clone().expect("stdout clone"),
            files.stderr.try_clone().expect("stderr clone"),
        );
        let mut command = isolated_command(&invocation, &BTreeMap::new(), files);
        let child = command.spawn().expect("spawn root");
        assert_no_tokio_process_stdio(child.as_ref());
        drop(command);
        (child, pipes, held_clients)
    });
    runtime.block_on(async {
        let limit = matches!(case, HeldClientCase::OutputLimit).then_some(3);
        let child: Box<dyn ChildWrapper> = if matches!(case, HeldClientCase::ObservationError) {
            Box::new(FailingLifecycle {
                child,
                failure: LifecycleFailure::Observe,
            })
        } else {
            child
        };
        let pending = capture_child(&invocation, limit, child, pipes, started);
        tokio::pin!(pending);
        if matches!(case, HeldClientCase::Cancellation) {
            tokio::select! {
                result = &mut pending => panic!("capture finished before cancellation: {result:?}"),
                () = tokio::time::sleep(Duration::from_millis(100)) => {}
            }
            return;
        }
        let result = pending.await;
        match case {
            HeldClientCase::Timeout => {
                let outcome = result.expect("deadline observation with outside writers");
                assert_eq!(outcome.outcome.status, ProcessStatus::TimedOut);
                assert_eq!(outcome.outcome.stdout, b"prefix");
            }
            HeldClientCase::NaturalExit => {
                assert!(matches!(result, Err(ProcessError::CaptureTimeout)));
            }
            HeldClientCase::OutputLimit => {
                let outcome = result.expect("output limit with outside writers");
                assert_eq!(outcome.outcome.stdout, b"pre");
                assert_eq!(outcome.output_limit.expect("limit").observed_bytes, 4);
            }
            HeldClientCase::ObservationError => assert!(
                result
                    .expect_err("forced error")
                    .to_string()
                    .contains("forced child observation error")
            ),
            HeldClientCase::Cancellation => unreachable!("cancellation returned above"),
        }
    });
    // Unlike shutdown_timeout, plain Drop waits for every blocking task. It must
    // complete while the unread stdin client and both output clients are LIVE.
    drop(runtime);
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "capture or runtime shutdown exceeded its bound"
    );
    // Disconnect-on-abort must also release the peer, not merely leave a kernel
    // write retained by Mio after the runtime stops. The watchdog bounds these
    // synchronous peer probes if a regression leaves either side still live.
    assert!(
        held_clients.0.read(&mut [0; 1]).is_err(),
        "unread stdin client still connected"
    );
    assert!(
        held_clients.1.write_all(b"x").is_err(),
        "stdout client still connected"
    );
    assert!(
        held_clients.2.write_all(b"x").is_err(),
        "stderr client still connected"
    );
    drop(held_clients);
}

#[test]
#[ignore = "launched with an external watchdog by held_open_outside_job_clients_do_not_stall_runtime_shutdown"]
fn outside_job_client_helper() {
    for case in [
        HeldClientCase::Timeout,
        HeldClientCase::NaturalExit,
        HeldClientCase::OutputLimit,
        HeldClientCase::ObservationError,
        HeldClientCase::Cancellation,
    ] {
        capture_with_held_clients(case);
    }
}

#[test]
fn stream_accumulator_bounds_retention_without_preallocating_limit() {
    let mut stream = CapturedStream::new(OutputStream::Stdout, Some(u64::MAX));
    assert_eq!(stream.bytes.capacity(), 0);
    stream.accept(b"abc");
    assert_eq!(stream.bytes, b"abc");
    assert_eq!(stream.exceeded, None);
}
