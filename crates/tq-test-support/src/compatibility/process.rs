//! Timeout-safe subprocess execution and capture.

use std::{
    collections::BTreeMap,
    io::{self, Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    thread,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(unix)]
use std::os::fd::AsFd;

#[cfg(unix)]
use nix::{
    errno::Errno,
    poll::{PollFd, PollFlags, PollTimeout, poll},
    sys::signal::{Signal, killpg},
    unistd::Pid,
};

#[cfg(any(target_os = "macos", target_os = "linux"))]
use rustix::process::{Pid as RustixPid, WaitId, WaitIdOptions, waitid};

#[cfg(unix)]
const CAPTURE_DRAIN_TIMEOUT: Duration = Duration::from_millis(250);
const CAPTURE_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Complete isolated subprocess request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Invocation {
    /// Executable path.
    pub executable: PathBuf,
    /// Exact argument vector.
    pub args: Vec<String>,
    /// Bytes written to stdin before it is closed.
    pub stdin: Vec<u8>,
    /// Maximum wall time.
    pub timeout: Duration,
    /// Optional working directory.
    pub current_dir: Option<PathBuf>,
    /// Explicit environment entries added to the child process.
    pub environment: BTreeMap<String, String>,
}

/// Classified process completion.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProcessStatus {
    /// Process exited normally, including nonzero exit codes.
    Exited,
    /// Process exceeded its wall-time limit and was killed.
    TimedOut,
    /// Process terminated from a signal.
    Signaled,
}

/// Stream whose bounded capture limit was reached.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputStream {
    /// Standard output.
    Stdout,
    /// Standard error.
    Stderr,
}

/// Provenance for a bounded output capture.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OutputLimit {
    /// Stream that exceeded the configured limit.
    pub stream: OutputStream,
    /// Maximum number of retained bytes for each stream.
    pub limit: u64,
    /// Bytes observed before capture stopped, including the first rejected byte.
    pub observed_bytes: u64,
}

/// Captured process observation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessOutcome {
    /// Completion classification.
    pub status: ProcessStatus,
    /// Numeric exit code for normal exits.
    pub exit_code: Option<i32>,
    /// Platform signal number when available.
    pub signal: Option<i32>,
    /// Captured stdout bytes. When a new-session descendant retains the pipe,
    /// this is the bytes available before the bounded capture drain stops.
    pub stdout: Vec<u8>,
    /// Captured stderr bytes. When a new-session descendant retains the pipe,
    /// this is the bytes available before the bounded capture drain stops.
    pub stderr: Vec<u8>,
    /// Observed wall time in microseconds.
    pub wall_time_micros: u128,
    /// Secret-redacted executable and argument vector.
    pub recorded_command: Vec<String>,
}

/// Process observation with an explicit bounded-output result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundedProcessOutcome {
    /// The process result and retained output prefix.
    pub outcome: ProcessOutcome,
    /// Set when stdout or stderr exceeded the configured byte limit.
    pub output_limit: Option<OutputLimit>,
}

/// Stable harness process failures.
#[derive(Debug, Error)]
pub enum ProcessError {
    /// Subprocess capture requires macOS/Linux non-consuming child observation.
    #[error("subprocess capture is unsupported on this platform")]
    Unsupported,
    /// Process creation or lifecycle I/O failed.
    #[error("subprocess I/O failed: {0}")]
    Io(#[from] io::Error),
    /// A capture worker panicked.
    #[error("subprocess capture worker panicked")]
    CaptureWorker,
    /// A Unix capture pipe remained open after the bounded drain interval.
    #[error("subprocess capture drain exceeded the bounded interval")]
    CaptureTimeout,
}

const fn capture_platform_supported() -> bool {
    cfg!(any(target_os = "macos", target_os = "linux"))
}

/// Runs one subprocess with isolated streams and a hard wall-time limit.
///
/// # Errors
///
/// Returns an I/O, capture-worker, or bounded-capture-drain error. Nonzero
/// exits, signals, and timeouts are successful observations rather than harness
/// errors. A natural exit with an escaped descendant that keeps a capture pipe
/// open is reported as a bounded-capture-drain error instead of a partial
/// successful observation.
/// On macOS and Linux, ordinary descendants are placed in a dedicated process
/// group so inherited capture pipes are closed during cleanup; descendants that
/// create a new session are outside that cleanup boundary.
/// Other platforms return [`ProcessError::Unsupported`] before spawning.
pub fn run_process(invocation: &Invocation) -> Result<ProcessOutcome, ProcessError> {
    run_process_with_environment(invocation, &BTreeMap::new())
}

/// Runs a subprocess with environment overrides confined to that child.
///
/// # Errors
///
/// Returns the same process and capture errors as [`run_process`].
/// On platforms other than macOS and Linux it returns
/// [`ProcessError::Unsupported`] before spawning.
pub fn run_process_with_environment(
    invocation: &Invocation,
    environment: &BTreeMap<String, String>,
) -> Result<ProcessOutcome, ProcessError> {
    if !capture_platform_supported() {
        return Err(ProcessError::Unsupported);
    }

    let started = Instant::now();
    let SpawnedProcess {
        mut child,
        mut stdin,
        mut stdout,
        mut stderr,
    } = spawn_process(invocation, environment)?;
    let mut child_reaped = false;
    let stdin_bytes = invocation.stdin.clone();
    let stop_capture = Arc::new(AtomicBool::new(false));
    let _stop_guard = CaptureStopGuard::new(Arc::clone(&stop_capture));

    let stdin_stop = Arc::clone(&stop_capture);
    let (stdin_worker, stdin_done) = spawn_capture(move || {
        let result = write_all_until_stopped(&mut stdin, &stdin_bytes, &stdin_stop);
        drop(stdin);
        result
    });
    let stdout_stop = Arc::clone(&stop_capture);
    let (stdout_worker, stdout_done) = spawn_capture(move || read_all(&mut stdout, &stdout_stop));
    let stderr_stop = Arc::clone(&stop_capture);
    let (stderr_worker, stderr_done) = spawn_capture(move || read_all(&mut stderr, &stderr_stop));

    let lifecycle_result = wait_for_child(&mut child, invocation.timeout);
    let (exit, timed_out) = match lifecycle_result {
        Ok(result) => {
            child_reaped = true;
            result
        }
        Err(error) => {
            stop_capture.store(true, Ordering::Release);
            let capture_error = finish_captures(
                stdin_worker,
                &stdin_done,
                stdout_worker,
                &stdout_done,
                stderr_worker,
                &stderr_done,
                &stop_capture,
            )
            .err();
            return Err(process_error_after_spawn(
                combine_process_errors(ProcessError::Io(error), capture_error),
                &mut child,
                &mut child_reaped,
                &stop_capture,
            ));
        }
    };

    let (((), stdin_stopped), (stdout, stdout_stopped), (stderr, stderr_stopped)) =
        match finish_captures(
            stdin_worker,
            &stdin_done,
            stdout_worker,
            &stdout_done,
            stderr_worker,
            &stderr_done,
            &stop_capture,
        ) {
            Ok(result) => result,
            Err(error) => {
                return Err(process_error_after_spawn(
                    error,
                    &mut child,
                    &mut child_reaped,
                    &stop_capture,
                ));
            }
        };
    if (stdin_stopped || stdout_stopped || stderr_stopped) && !timed_out {
        return Err(process_error_after_spawn(
            ProcessError::CaptureTimeout,
            &mut child,
            &mut child_reaped,
            &stop_capture,
        ));
    }
    let signal = signal(exit);
    let status = if timed_out {
        ProcessStatus::TimedOut
    } else if signal.is_some() {
        ProcessStatus::Signaled
    } else {
        ProcessStatus::Exited
    };

    Ok(ProcessOutcome {
        status,
        exit_code: exit.code(),
        signal,
        stdout,
        stderr,
        wall_time_micros: started.elapsed().as_micros(),
        recorded_command: redact(invocation),
    })
}

fn wait_for_child(
    child: &mut std::process::Child,
    timeout: Duration,
) -> io::Result<(std::process::ExitStatus, bool)> {
    let started = Instant::now();
    loop {
        if child_has_exited(child)? {
            return Ok((reap_exited_child(child)?, false));
        }
        if started.elapsed() >= timeout {
            return Ok((terminate_child(child)?, true));
        }
        thread::sleep(Duration::from_millis(1));
    }
}

fn wait_for_bounded_child(
    child: &mut std::process::Child,
    started: Instant,
    timeout: Duration,
    stdout_limited: &AtomicBool,
    stderr_limited: &AtomicBool,
) -> io::Result<(std::process::ExitStatus, bool)> {
    loop {
        if stdout_limited.load(Ordering::Acquire) || stderr_limited.load(Ordering::Acquire) {
            return terminate_child(child).map(|exit| (exit, false));
        }
        match child_has_exited(child) {
            Ok(true) => return reap_exited_child(child).map(|exit| (exit, false)),
            Ok(false) => {}
            Err(error) => return Err(error),
        }
        if started.elapsed() >= timeout {
            return terminate_child(child).map(|exit| (exit, true));
        }
        thread::sleep(Duration::from_millis(1));
    }
}

fn reap_exited_child(child: &mut std::process::Child) -> io::Result<std::process::ExitStatus> {
    reap_exited_child_with_cleanup(child, cleanup_process_group)
}

fn reap_exited_child_with_cleanup<F>(
    child: &mut std::process::Child,
    cleanup: F,
) -> io::Result<std::process::ExitStatus>
where
    F: FnOnce(&mut std::process::Child) -> io::Result<()>,
{
    let cleanup_error = cleanup(child).err();
    let exit = child.wait();
    preserve_cleanup_error(cleanup_error, exit)
}

fn preserve_cleanup_error<T>(
    cleanup_error: Option<io::Error>,
    result: io::Result<T>,
) -> io::Result<T> {
    match (cleanup_error, result) {
        (None, result) => result,
        (Some(error), Ok(_)) => Err(error),
        (Some(error), Err(reap_error)) => {
            let kind = error.kind();
            let message = format!("{error}; owned child reap failed: {reap_error}");
            Err(io::Error::new(kind, message))
        }
    }
}

fn merge_cleanup_errors(first: Option<io::Error>, second: Option<io::Error>) -> Option<io::Error> {
    match (first, second) {
        (None, error) | (error, None) => error,
        (Some(first), Some(second)) => {
            let kind = first.kind();
            Some(io::Error::new(
                kind,
                format!("{first}; additional cleanup error: {second}"),
            ))
        }
    }
}

fn spawn_pipe_error(child: &mut std::process::Child, pipe: &str) -> ProcessError {
    let stop_capture = AtomicBool::new(false);
    let mut child_reaped = false;
    process_error_after_spawn(
        ProcessError::Io(io::Error::other(format!("piped {pipe} was not created"))),
        child,
        &mut child_reaped,
        &stop_capture,
    )
}

fn process_error_after_spawn(
    primary: ProcessError,
    child: &mut std::process::Child,
    child_reaped: &mut bool,
    stop_capture: &AtomicBool,
) -> ProcessError {
    process_error_after_spawn_with_cleanup(
        primary,
        child,
        child_reaped,
        stop_capture,
        cleanup_process_group,
    )
}

fn process_error_after_spawn_with_cleanup<F>(
    primary: ProcessError,
    child: &mut std::process::Child,
    child_reaped: &mut bool,
    stop_capture: &AtomicBool,
    cleanup: F,
) -> ProcessError
where
    F: Fn(&mut std::process::Child) -> io::Result<()> + Copy,
{
    stop_capture.store(true, Ordering::Release);
    match cleanup_owned_child_after_error_with(child, child_reaped, cleanup) {
        Ok(()) => primary,
        Err(cleanup_error) => ProcessError::Io(io::Error::new(
            cleanup_error.kind(),
            format!("{primary}; owned child cleanup failed: {cleanup_error}"),
        )),
    }
}

/// Cleans up a child still owned by a failing process path.
///
/// A `Child` that has already been reaped must never be signalled again: its
/// PID may already belong to an unrelated process. The non-consuming waitid
/// probe lets natural exits clean their process group before the exact wait4;
/// an ECHILD result means the child was reaped elsewhere and is therefore
/// treated as complete. All other observation failures still attempt bounded
/// cleanup, with both errors retained for the caller.
fn cleanup_owned_child_after_error_with<F>(
    child: &mut std::process::Child,
    child_reaped: &mut bool,
    cleanup: F,
) -> io::Result<()>
where
    F: Fn(&mut std::process::Child) -> io::Result<()> + Copy,
{
    if *child_reaped {
        return Ok(());
    }

    match child_has_exited(child) {
        Ok(true) => {
            // Descendants may still hold capture pipes after the direct child
            // became a zombie, so clean the group before the exact wait4.
            let cleanup_error = cleanup(child).err();
            match child.wait() {
                Ok(_) => {
                    *child_reaped = true;
                    cleanup_error.map_or(Ok(()), Err)
                }
                Err(error) if is_child_reaped_error(&error) => {
                    *child_reaped = true;
                    merge_cleanup_errors(cleanup_error, None).map_or(Ok(()), Err)
                }
                Err(wait_error) => {
                    let fallback = cleanup_live_child(child, child_reaped, cleanup);
                    let errors = merge_cleanup_errors(cleanup_error, Some(wait_error));
                    merge_cleanup_errors(errors, fallback.err()).map_or(Ok(()), Err)
                }
            }
        }
        Ok(false) => cleanup_live_child(child, child_reaped, cleanup),
        Err(error) if is_child_reaped_error(&error) => {
            *child_reaped = true;
            Ok(())
        }
        Err(observation_error) => {
            let cleanup_result = cleanup_live_child(child, child_reaped, cleanup);
            merge_cleanup_errors(Some(observation_error), cleanup_result.err()).map_or(Ok(()), Err)
        }
    }
}

fn cleanup_live_child<F>(
    child: &mut std::process::Child,
    child_reaped: &mut bool,
    cleanup: F,
) -> io::Result<()>
where
    F: Fn(&mut std::process::Child) -> io::Result<()> + Copy,
{
    let mut errors = cleanup(child).err();
    let mut should_kill = true;
    match child.try_wait() {
        Ok(Some(_)) => {
            *child_reaped = true;
            should_kill = false;
        }
        Ok(None) => {}
        Err(error) if is_child_reaped_error(&error) => {
            *child_reaped = true;
            should_kill = false;
        }
        Err(error) => errors = merge_cleanup_errors(errors, Some(error)),
    }

    if should_kill {
        match child.kill() {
            Ok(()) => {}
            // Group cleanup can race the direct-child fallback. The bounded
            // try_wait loop below remains responsible for reaping it.
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => errors = merge_cleanup_errors(errors, Some(error)),
        }
    }

    if !*child_reaped {
        match reap_child_after_termination(child) {
            Ok(_) => *child_reaped = true,
            Err(error) if is_child_reaped_error(&error) => *child_reaped = true,
            Err(error) => errors = merge_cleanup_errors(errors, Some(error)),
        }
    }

    errors.map_or(Ok(()), Err)
}

#[cfg(unix)]
fn is_child_reaped_error(error: &io::Error) -> bool {
    error.raw_os_error() == Some(Errno::ECHILD as i32)
}

#[cfg(not(unix))]
fn is_child_reaped_error(_error: &io::Error) -> bool {
    false
}

fn child_has_exited(child: &mut std::process::Child) -> io::Result<bool> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let pid = RustixPid::from_raw(
            i32::try_from(child.id()).map_err(|_| io::Error::other("child PID exceeds i32"))?,
        )
        .ok_or_else(|| io::Error::other("child PID is invalid"))?;
        loop {
            match waitid(
                WaitId::Pid(pid),
                WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
            ) {
                Ok(status) => return Ok(status.is_some()),
                Err(rustix::io::Errno::INTR) => {}
                Err(error) => return Err(error.into()),
            }
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        // A consuming probe would release the process-group leader before
        // cleanup, permitting its ID to be reused by an unrelated process.
        let _ = child;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "non-consuming child observation is unavailable on this platform",
        ))
    }
}

struct SpawnedProcess {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    stdout: std::process::ChildStdout,
    stderr: std::process::ChildStderr,
}

fn spawn_process(
    invocation: &Invocation,
    environment: &BTreeMap<String, String>,
) -> Result<SpawnedProcess, ProcessError> {
    let mut command = Command::new(&invocation.executable);
    command
        .args(&invocation.args)
        .envs(&invocation.environment)
        .envs(environment)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        command.process_group(0);
    }
    if let Some(current_dir) = &invocation.current_dir {
        command.current_dir(current_dir);
    }
    let mut child = command.spawn()?;
    let Some(stdin) = child.stdin.take() else {
        return Err(spawn_pipe_error(&mut child, "stdin"));
    };
    let Some(stdout) = child.stdout.take() else {
        return Err(spawn_pipe_error(&mut child, "stdout"));
    };
    let Some(stderr) = child.stderr.take() else {
        return Err(spawn_pipe_error(&mut child, "stderr"));
    };
    Ok(SpawnedProcess {
        child,
        stdin,
        stdout,
        stderr,
    })
}

/// Runs a subprocess while retaining at most `output_limit` bytes from each
/// output stream. The child is killed and reaped as soon as either stream
/// exceeds the limit. The returned metadata makes that resource outcome
/// explicit; output is never silently truncated and reported as complete.
///
/// # Errors
///
/// Returns an I/O, capture-worker, or bounded-capture-drain error. Nonzero
/// exits, signals, timeouts, and output-limit terminations are successful
/// observations. A natural exit with an escaped descendant that keeps a
/// capture pipe open is reported as a bounded-capture-drain error instead of a
/// partial successful observation.
/// On macOS and Linux, ordinary descendants are placed in a dedicated process
/// group so inherited capture pipes are closed during cleanup; descendants that
/// create a new session are outside that cleanup boundary.
/// Other platforms return [`ProcessError::Unsupported`] before spawning.
pub fn run_process_with_environment_bounded(
    invocation: &Invocation,
    environment: &BTreeMap<String, String>,
    output_limit: u64,
) -> Result<BoundedProcessOutcome, ProcessError> {
    if !capture_platform_supported() {
        return Err(ProcessError::Unsupported);
    }

    let started = Instant::now();
    let SpawnedProcess {
        mut child,
        stdin,
        stdout,
        stderr,
    } = spawn_process(invocation, environment)?;
    let mut child_reaped = false;
    let workers = spawn_bounded_capture_workers(
        stdin,
        stdout,
        stderr,
        invocation.stdin.clone(),
        output_limit,
    );
    let stop_capture = Arc::clone(&workers.stop_capture);
    let _stop_guard = CaptureStopGuard::new(Arc::clone(&stop_capture));

    let lifecycle_result = wait_for_bounded_child(
        &mut child,
        started,
        invocation.timeout,
        &workers.stdout_limited,
        &workers.stderr_limited,
    );
    let (exit, timed_out) = match lifecycle_result {
        Ok(exit) => {
            child_reaped = true;
            exit
        }
        Err(error) => {
            stop_capture.store(true, Ordering::Release);
            let capture_error = workers.finish().err();
            return Err(process_error_after_spawn(
                combine_process_errors(ProcessError::Io(error), capture_error),
                &mut child,
                &mut child_reaped,
                &stop_capture,
            ));
        }
    };

    let (((), _stdin_stopped), (stdout, _stdout_stopped), (stderr, _stderr_stopped)) =
        finish_bounded_captures(workers, &mut child, &mut child_reaped, timed_out)?;
    let output_limit = stdout.limit.or(stderr.limit);
    let signal = signal(exit);
    let status = if timed_out {
        ProcessStatus::TimedOut
    } else if signal.is_some() {
        ProcessStatus::Signaled
    } else {
        ProcessStatus::Exited
    };

    Ok(BoundedProcessOutcome {
        outcome: ProcessOutcome {
            status,
            exit_code: exit.code(),
            signal,
            stdout: stdout.bytes,
            stderr: stderr.bytes,
            wall_time_micros: started.elapsed().as_micros(),
            recorded_command: redact(invocation),
        },
        output_limit,
    })
}

fn spawn_capture<T, F>(capture: F) -> (thread::JoinHandle<io::Result<T>>, Receiver<()>)
where
    F: FnOnce() -> io::Result<T> + Send + 'static,
    T: Send + 'static,
{
    let (done_sender, done_receiver) = mpsc::channel();
    let worker = thread::spawn(move || {
        let result = capture();
        let _ = done_sender.send(());
        result
    });
    (worker, done_receiver)
}

/// Stops poll-based capture workers if an owning process path returns early.
///
/// The workers are intentionally joined on the normal path, but an error from
/// any later lifecycle step can otherwise detach a worker that is still
/// waiting on a pipe retained by a descendant. Setting the shared flag during
/// unwinding lets that worker leave its bounded poll loop.
struct CaptureStopGuard {
    stop_capture: Arc<AtomicBool>,
}

impl CaptureStopGuard {
    fn new(stop_capture: Arc<AtomicBool>) -> Self {
        Self { stop_capture }
    }
}

impl Drop for CaptureStopGuard {
    fn drop(&mut self) {
        self.stop_capture.store(true, Ordering::Release);
    }
}

fn finish_capture<T>(
    worker: thread::JoinHandle<io::Result<T>>,
    done: &Receiver<()>,
    stop_capture: &AtomicBool,
) -> Result<(T, bool), ProcessError>
where
    T: Send + 'static,
{
    let mut stopped = false;
    #[cfg(unix)]
    if done
        .recv_timeout(CAPTURE_DRAIN_TIMEOUT)
        .is_err_and(|error| matches!(error, mpsc::RecvTimeoutError::Timeout))
    {
        // A child that creates a new session can retain the capture pipe after
        // the process-group cleanup. Pollable Unix readers observe this stop
        // flag and return their retained prefix instead of joining forever.
        stop_capture.store(true, Ordering::Release);
        stopped = true;
    }
    #[cfg(not(unix))]
    {
        let _ = done.recv();
    }
    let result = worker
        .join()
        .map_err(|_| ProcessError::CaptureWorker)?
        .map_err(ProcessError::Io)?;
    Ok((result, stopped))
}

type FinishedCaptures<A, B, C> = ((A, bool), (B, bool), (C, bool));

fn finish_captures<A, B, C>(
    stdin_worker: thread::JoinHandle<io::Result<A>>,
    stdin_done: &Receiver<()>,
    stdout_worker: thread::JoinHandle<io::Result<B>>,
    stdout_done: &Receiver<()>,
    stderr_worker: thread::JoinHandle<io::Result<C>>,
    stderr_done: &Receiver<()>,
    stop_capture: &AtomicBool,
) -> Result<FinishedCaptures<A, B, C>, ProcessError>
where
    A: Send + 'static,
    B: Send + 'static,
    C: Send + 'static,
{
    let stdin_result = finish_capture(stdin_worker, stdin_done, stop_capture);
    if stdin_result.is_err() {
        stop_capture.store(true, Ordering::Release);
    }
    let stdout_result = finish_capture(stdout_worker, stdout_done, stop_capture);
    if stdout_result.is_err() {
        stop_capture.store(true, Ordering::Release);
    }
    let stderr_result = finish_capture(stderr_worker, stderr_done, stop_capture);

    match (stdin_result, stdout_result, stderr_result) {
        (Ok(stdin), Ok(stdout), Ok(stderr)) => Ok((stdin, stdout, stderr)),
        (Err(error), _, _) | (Ok(_), Err(error), _) | (Ok(_), Ok(_), Err(error)) => Err(error),
    }
}

fn combine_process_errors(primary: ProcessError, additional: Option<ProcessError>) -> ProcessError {
    match additional {
        None => primary,
        Some(additional) => ProcessError::Io(io::Error::other(format!(
            "{primary}; additional capture cleanup error: {additional}"
        ))),
    }
}

fn read_all<R: CaptureReader>(reader: &mut R, stop_capture: &AtomicBool) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut buffer = [0; 8192];
    loop {
        let read = reader.read_with_stop(&mut buffer, stop_capture)?;
        if read == 0 {
            return Ok(bytes);
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
}

struct BoundedRead {
    bytes: Vec<u8>,
    limit: Option<OutputLimit>,
}

struct BoundedCaptureWorkers {
    stop_capture: Arc<AtomicBool>,
    stdout_limited: Arc<AtomicBool>,
    stderr_limited: Arc<AtomicBool>,
    stdin_worker: thread::JoinHandle<io::Result<()>>,
    stdin_done: Receiver<()>,
    stdout_worker: thread::JoinHandle<io::Result<BoundedRead>>,
    stdout_done: Receiver<()>,
    stderr_worker: thread::JoinHandle<io::Result<BoundedRead>>,
    stderr_done: Receiver<()>,
}

impl BoundedCaptureWorkers {
    fn finish(self) -> Result<FinishedCaptures<(), BoundedRead, BoundedRead>, ProcessError> {
        let Self {
            stop_capture,
            stdin_worker,
            stdin_done,
            stdout_worker,
            stdout_done,
            stderr_worker,
            stderr_done,
            ..
        } = self;
        finish_captures(
            stdin_worker,
            &stdin_done,
            stdout_worker,
            &stdout_done,
            stderr_worker,
            &stderr_done,
            &stop_capture,
        )
    }
}

fn spawn_bounded_capture_workers(
    mut stdin: std::process::ChildStdin,
    mut stdout: std::process::ChildStdout,
    mut stderr: std::process::ChildStderr,
    stdin_bytes: Vec<u8>,
    output_limit: u64,
) -> BoundedCaptureWorkers {
    let stdout_limited = Arc::new(AtomicBool::new(false));
    let stderr_limited = Arc::new(AtomicBool::new(false));
    let stop_capture = Arc::new(AtomicBool::new(false));

    let stdin_stop = Arc::clone(&stop_capture);
    let (stdin_worker, stdin_done) = spawn_capture(move || {
        let result = write_all_until_stopped(&mut stdin, &stdin_bytes, &stdin_stop);
        drop(stdin);
        result
    });
    let stdout_flag = Arc::clone(&stdout_limited);
    let stdout_stop = Arc::clone(&stop_capture);
    let (stdout_worker, stdout_done) = spawn_capture(move || {
        read_limited(
            &mut stdout,
            OutputStream::Stdout,
            output_limit,
            &stdout_flag,
            &stdout_stop,
        )
    });
    let stderr_flag = Arc::clone(&stderr_limited);
    let stderr_stop = Arc::clone(&stop_capture);
    let (stderr_worker, stderr_done) = spawn_capture(move || {
        read_limited(
            &mut stderr,
            OutputStream::Stderr,
            output_limit,
            &stderr_flag,
            &stderr_stop,
        )
    });

    BoundedCaptureWorkers {
        stop_capture,
        stdout_limited,
        stderr_limited,
        stdin_worker,
        stdin_done,
        stdout_worker,
        stdout_done,
        stderr_worker,
        stderr_done,
    }
}

fn finish_bounded_captures(
    workers: BoundedCaptureWorkers,
    child: &mut std::process::Child,
    child_reaped: &mut bool,
    timed_out: bool,
) -> Result<FinishedCaptures<(), BoundedRead, BoundedRead>, ProcessError> {
    let stop_capture = Arc::clone(&workers.stop_capture);
    let captures = match workers.finish() {
        Ok(result) => result,
        Err(error) => {
            return Err(process_error_after_spawn(
                error,
                child,
                child_reaped,
                &stop_capture,
            ));
        }
    };
    let (((), stdin_stopped), (stdout, stdout_stopped), (stderr, stderr_stopped)) = captures;
    let output_limit = stdout.limit.or(stderr.limit);
    if (stdin_stopped || stdout_stopped || stderr_stopped) && !timed_out && output_limit.is_none() {
        return Err(process_error_after_spawn(
            ProcessError::CaptureTimeout,
            child,
            child_reaped,
            &stop_capture,
        ));
    }
    Ok((
        ((), stdin_stopped),
        (stdout, stdout_stopped),
        (stderr, stderr_stopped),
    ))
}

fn read_limited<R: CaptureReader>(
    reader: &mut R,
    stream: OutputStream,
    output_limit: u64,
    limit_reached: &AtomicBool,
    stop_capture: &AtomicBool,
) -> io::Result<BoundedRead> {
    const READ_CHUNK: usize = 8192;
    let limit = usize::try_from(output_limit).unwrap_or(usize::MAX);
    let mut bytes = Vec::with_capacity(limit.min(READ_CHUNK));
    let mut buffer = [0; READ_CHUNK];
    loop {
        let remaining = limit.saturating_sub(bytes.len());
        let read_len = remaining
            .min(READ_CHUNK)
            .saturating_add(usize::from(remaining < READ_CHUNK));
        let read = reader.read_with_stop(&mut buffer[..read_len], stop_capture)?;
        if read == 0 {
            return Ok(BoundedRead { bytes, limit: None });
        }
        if read > remaining {
            bytes.extend_from_slice(&buffer[..remaining]);
            limit_reached.store(true, Ordering::Release);
            return Ok(BoundedRead {
                bytes,
                limit: Some(OutputLimit {
                    stream,
                    limit: output_limit,
                    observed_bytes: output_limit
                        .saturating_add(u64::try_from(read - remaining).unwrap_or(u64::MAX)),
                }),
            });
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
}

trait CaptureReader: Read {
    fn read_with_stop(&mut self, buffer: &mut [u8], stop_capture: &AtomicBool)
    -> io::Result<usize>;
}

#[cfg(unix)]
impl<R: Read + AsFd> CaptureReader for R {
    fn read_with_stop(
        &mut self,
        buffer: &mut [u8],
        stop_capture: &AtomicBool,
    ) -> io::Result<usize> {
        read_until_stopped(self, buffer, stop_capture)
    }
}

#[cfg(not(unix))]
impl<R: Read> CaptureReader for R {
    fn read_with_stop(
        &mut self,
        buffer: &mut [u8],
        _stop_capture: &AtomicBool,
    ) -> io::Result<usize> {
        read_retry(self, buffer)
    }
}

#[cfg(unix)]
fn read_until_stopped<R: Read + AsFd>(
    reader: &mut R,
    buffer: &mut [u8],
    stop_capture: &AtomicBool,
) -> io::Result<usize> {
    if !poll_until_ready(reader.as_fd(), PollFlags::POLLIN, stop_capture)? {
        return Ok(0);
    }
    read_retry(reader, buffer)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn write_all_until_stopped<W: Write + AsFd>(
    writer: &mut W,
    bytes: &[u8],
    stop_capture: &AtomicBool,
) -> io::Result<()> {
    use rustix::fs::{OFlags, fcntl_getfl, fcntl_setfl};

    // Readiness guarantees some capacity, not enough for the whole chunk.
    // Keep the owned pipe nonblocking so a partial drain cannot strand this
    // worker inside write after the child or an escaped descendant stops reading.
    let flags = fcntl_getfl(&*writer)?;
    fcntl_setfl(&*writer, flags | OFlags::NONBLOCK)?;
    let mut written = 0;
    while written < bytes.len() {
        if !poll_until_ready(writer.as_fd(), PollFlags::POLLOUT, stop_capture)? {
            return Ok(());
        }
        let end = written.saturating_add(8192).min(bytes.len());
        match writer.write(&bytes[written..end]) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "stdin pipe closed",
                ));
            }
            Ok(count) => written += count,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock
                ) => {}
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => return Ok(()),
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

#[cfg(unix)]
fn poll_until_ready(
    fd: std::os::fd::BorrowedFd<'_>,
    events: PollFlags,
    stop_capture: &AtomicBool,
) -> io::Result<bool> {
    loop {
        if stop_capture.load(Ordering::Acquire) {
            return Ok(false);
        }
        let mut poll_fd = [PollFd::new(fd, events)];
        match poll(
            &mut poll_fd,
            PollTimeout::try_from(CAPTURE_POLL_INTERVAL).expect("fixed capture poll interval fits"),
        ) {
            Ok(0) | Err(Errno::EINTR) => {}
            Ok(_) => {
                let observed = poll_fd[0]
                    .revents()
                    .ok_or_else(|| io::Error::other("capture poll returned unknown events"))?;
                if observed.intersects(events | PollFlags::POLLHUP | PollFlags::POLLERR) {
                    return Ok(true);
                }
                if observed.contains(PollFlags::POLLNVAL) {
                    return Err(io::Error::other("capture pipe became invalid"));
                }
            }
            Err(error) => return Err(error.into()),
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn write_all_until_stopped<W: Write>(
    writer: &mut W,
    bytes: &[u8],
    _stop_capture: &AtomicBool,
) -> io::Result<()> {
    match writer.write_all(bytes) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        Err(error) => Err(error),
    }
}

fn terminate_child(child: &mut std::process::Child) -> io::Result<std::process::ExitStatus> {
    #[cfg(unix)]
    {
        terminate_child_with_cleanup(child, cleanup_process_group)
    }
    #[cfg(not(unix))]
    {
        if child.try_wait()?.is_none()
            && let Err(error) = child.kill()
            && child.try_wait()?.is_none()
        {
            return Err(error);
        }
        child.wait()
    }
}

const CHILD_REAP_TIMEOUT: Duration = Duration::from_secs(1);

#[cfg(unix)]
fn terminate_child_with_cleanup<F>(
    child: &mut std::process::Child,
    cleanup: F,
) -> io::Result<std::process::ExitStatus>
where
    F: Fn(&mut std::process::Child) -> io::Result<()> + Copy,
{
    let (child_is_exited, state_error) = match child_has_exited(child) {
        Ok(exited) => (exited, None),
        Err(error) if is_child_reaped_error(&error) => {
            // Do not signal after another owner reaped this child: its PID
            // may already belong to an unrelated process.
            return Err(error);
        }
        Err(error) => (false, Some(error)),
    };
    let cleanup_error = cleanup(child).err();
    let kill_error = if child_is_exited {
        None
    } else {
        match child.kill() {
            Ok(()) => None,
            // Group cleanup can kill the child between the status probe and
            // this direct-child fallback. The bounded reap below still owns
            // wait4.
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => Some(error),
        }
    };
    let exit = reap_child_after_termination(child);
    let cleanup_error = merge_cleanup_errors(cleanup_error, state_error);
    let cleanup_error = merge_cleanup_errors(cleanup_error, kill_error);
    preserve_cleanup_error(cleanup_error, exit)
}

fn reap_child_after_termination(
    child: &mut std::process::Child,
) -> io::Result<std::process::ExitStatus> {
    let started = Instant::now();
    loop {
        if let Some(exit) = child.try_wait()? {
            return Ok(exit);
        }
        if started.elapsed() >= CHILD_REAP_TIMEOUT {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "owned child did not exit after termination",
            ));
        }
        thread::sleep(CAPTURE_POLL_INTERVAL);
    }
}

#[cfg(target_os = "macos")]
fn signal_error(pid: Pid, error: Errno) -> io::Error {
    io::Error::other(format!(
        "killpg(SIGKILL, pgid={}) failed: {error}",
        pid.as_raw()
    ))
}

/// Kill the dedicated child process group so ordinary descendants release the
/// inherited capture pipes before the reader workers are joined. A descendant
/// that creates a new session is outside this cleanup boundary.
fn cleanup_process_group(child: &mut std::process::Child) -> io::Result<()> {
    #[cfg(unix)]
    {
        let pid = Pid::from_raw(
            i32::try_from(child.id()).map_err(|_| io::Error::other("child PID exceeds i32"))?,
        );
        match killpg(pid, Signal::SIGKILL) {
            Ok(()) | Err(Errno::ESRCH) => Ok(()),
            #[cfg(target_os = "macos")]
            Err(Errno::EPERM) => macos_cleanup_after_eperm(child, pid),
            Err(error) => Err(io::Error::other(format!(
                "killpg(SIGKILL, pgid={}) failed: {error}",
                pid.as_raw()
            ))),
        }
    }
    #[cfg(not(unix))]
    {
        let _ = child;
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn macos_cleanup_after_eperm(child: &mut std::process::Child, group: Pid) -> io::Result<()> {
    // Output-limit cleanup can race the child's transition to zombie. Wait a
    // short bounded interval for that observation; a still-live child keeps
    // the permission failure explicit instead of being treated as clean.
    let deadline = Instant::now() + Duration::from_millis(50);
    loop {
        if child_has_exited(child)? {
            return macos_ensure_group_empty(group);
        }
        if Instant::now() >= deadline {
            return Err(signal_error(group, Errno::EPERM));
        }
        thread::sleep(Duration::from_millis(1));
    }
}

#[cfg(target_os = "macos")]
fn macos_ensure_group_empty(group: Pid) -> io::Result<()> {
    let members = macos_live_group_members(group)?;
    if members.is_empty() {
        Ok(())
    } else {
        Err(macos_live_members_error(group, &members))
    }
}

#[cfg(target_os = "macos")]
fn macos_live_group_members(group: Pid) -> io::Result<Vec<u32>> {
    use libproc::{
        bsd_info::BSDInfo,
        proc_pid::pidinfo,
        processes::{ProcFilter, pids_by_type},
    };

    let group = u32::try_from(group.as_raw())
        .map_err(|_| io::Error::other("process-group identifier is negative"))?;
    let pids = pids_by_type(ProcFilter::ByProgramGroup { pgrpid: group }).map_err(|error| {
        io::Error::other(format!(
            "libproc process-group listing for pgid {group} failed: {error}"
        ))
    })?;
    let mut live = Vec::new();
    for pid in pids {
        // The leader is known to be exited from waitid and is allowed to
        // remain a zombie until the exact wait4 below.
        if pid == group {
            continue;
        }
        let pid_i32 = i32::try_from(pid).map_err(|_| {
            io::Error::other(format!("libproc returned out-of-range process PID {pid}"))
        })?;
        let info = pidinfo::<BSDInfo>(pid_i32, 1).map_err(|error| {
            io::Error::other(format!(
                "libproc BSD info for process {pid} in pgid {group} failed: {error}"
            ))
        })?;
        // A PID can exit or change groups between listpids and pidinfo. Ignore
        // entries that no longer match; lookup failures remain errors so a
        // permission failure cannot be mistaken for an empty group.
        if info.pbi_pid == pid && info.pbi_pgid == group && info.pbi_status != nix::libc::SZOMB {
            live.push(pid);
        }
    }
    Ok(live)
}

#[cfg(target_os = "macos")]
fn macos_live_members_error(pid: Pid, members: &[u32]) -> io::Error {
    io::Error::other(format!(
        "process-group cleanup for pgid {} left live members after EPERM: {members:?}",
        pid.as_raw()
    ))
}

fn read_retry(reader: &mut impl Read, buffer: &mut [u8]) -> io::Result<usize> {
    loop {
        match reader.read(buffer) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            result => return result,
        }
    }
}

fn redact(invocation: &Invocation) -> Vec<String> {
    let mut recorded = vec![invocation.executable.display().to_string()];
    let mut index = 0;
    while index < invocation.args.len() {
        let argument = &invocation.args[index];
        recorded.push(argument.clone());
        if matches!(argument.as_str(), "--arg" | "--argjson" | "--argtoon") {
            if let Some(name) = invocation.args.get(index + 1) {
                recorded.push(name.clone());
            }
            if invocation.args.get(index + 2).is_some() {
                recorded.push("<redacted>".to_owned());
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    recorded
}

#[cfg(unix)]
fn signal(status: std::process::ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

#[cfg(not(unix))]
fn signal(_status: std::process::ExitStatus) -> Option<i32> {
    None
}

#[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
mod tests {
    use super::*;

    #[test]
    fn capture_stop_guard_releases_held_open_reader_after_early_error() {
        use std::os::unix::net::UnixStream;

        let (mut reader, writer) = UnixStream::pair().expect("capture pipe pair");
        let stop_capture = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop_capture);
        let (worker, done, result) = {
            let _stop_guard = CaptureStopGuard::new(Arc::clone(&stop_capture));
            let (worker, done) = spawn_capture(move || read_all(&mut reader, &worker_stop));
            (
                worker,
                done,
                Err::<(), ProcessError>(ProcessError::CaptureWorker),
            )
        };

        assert!(matches!(result, Err(ProcessError::CaptureWorker)));
        assert!(stop_capture.load(Ordering::Acquire));
        done.recv_timeout(CAPTURE_DRAIN_TIMEOUT)
            .expect("capture worker stopped after early return");
        assert_eq!(
            worker
                .join()
                .expect("capture worker thread")
                .expect("capture reader result"),
            Vec::<u8>::new()
        );
        drop(writer);
    }

    fn assert_child_was_reaped(child: &std::process::Child) {
        let pid = RustixPid::from_raw(
            i32::try_from(child.id()).expect("cleanup test child PID exceeds i32"),
        )
        .expect("cleanup test child PID is invalid");
        match waitid(
            WaitId::Pid(pid),
            WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
        ) {
            Err(rustix::io::Errno::CHILD) => {}
            Ok(status) => panic!("cleanup test child remained waitable: {status:?}"),
            Err(error) => panic!("unexpected cleanup test child wait error: {error}"),
        }
    }

    #[test]
    fn cleanup_error_still_reaps_owned_child() {
        let mut child = Command::new("/bin/sleep")
            .arg("30")
            .spawn()
            .expect("spawn cleanup test child");
        let result = terminate_child_with_cleanup(&mut child, |_| {
            Err(io::Error::other("synthetic process-group cleanup failure"))
        });

        assert_child_was_reaped(&child);
        assert_eq!(
            result
                .expect_err("cleanup failure must remain observable")
                .to_string(),
            "synthetic process-group cleanup failure"
        );
    }

    #[test]
    fn cleanup_error_still_waits_for_exited_child() {
        let mut child = Command::new("/usr/bin/true")
            .spawn()
            .expect("spawn exited cleanup test child");
        let result = reap_exited_child_with_cleanup(&mut child, |_| {
            Err(io::Error::other("synthetic process-group cleanup failure"))
        });

        assert_child_was_reaped(&child);
        assert_eq!(
            result
                .expect_err("cleanup failure must remain observable")
                .to_string(),
            "synthetic process-group cleanup failure"
        );
    }

    #[test]
    fn post_spawn_error_terminates_and_reaps_live_child() {
        let mut child = Command::new("/bin/sleep")
            .arg("30")
            .spawn()
            .expect("spawn live cleanup test child");
        let stop_capture = AtomicBool::new(false);
        let mut child_reaped = false;
        let error = process_error_after_spawn(
            ProcessError::CaptureWorker,
            &mut child,
            &mut child_reaped,
            &stop_capture,
        );

        assert!(matches!(error, ProcessError::CaptureWorker));
        assert!(stop_capture.load(Ordering::Acquire));
        assert!(child_reaped);
        assert_child_was_reaped(&child);
    }

    #[test]
    fn post_spawn_error_preserves_primary_and_cleanup_errors() {
        let mut child = Command::new("/bin/sleep")
            .arg("30")
            .spawn()
            .expect("spawn synthetic cleanup test child");
        let stop_capture = AtomicBool::new(false);
        let mut child_reaped = false;
        let error = process_error_after_spawn_with_cleanup(
            ProcessError::CaptureWorker,
            &mut child,
            &mut child_reaped,
            &stop_capture,
            |_| Err(io::Error::other("synthetic owned-child cleanup failure")),
        );
        let message = error.to_string();

        assert!(message.contains("capture worker panicked"));
        assert!(message.contains("synthetic owned-child cleanup failure"));
        assert!(stop_capture.load(Ordering::Acquire));
        assert!(child_reaped);
        assert_child_was_reaped(&child);
    }

    #[test]
    fn post_spawn_error_does_not_signal_already_reaped_child() {
        let mut child = Command::new("/usr/bin/true")
            .spawn()
            .expect("spawn already-reaped cleanup test child");
        child.wait().expect("reap cleanup test child");
        let cleanup_called = AtomicBool::new(false);
        let mut child_reaped = false;
        let result = cleanup_owned_child_after_error_with(
            &mut child,
            &mut child_reaped,
            |_: &mut std::process::Child| {
                cleanup_called.store(true, Ordering::Release);
                Ok(())
            },
        );

        assert!(result.is_ok());
        assert!(child_reaped);
        assert!(!cleanup_called.load(Ordering::Acquire));
        assert_child_was_reaped(&child);
    }

    #[test]
    fn natural_exit_cleanup_runs_before_reap() {
        let mut child = Command::new("/usr/bin/true")
            .spawn()
            .expect("spawn natural-exit cleanup test child");
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            if child_has_exited(&mut child).expect("observe natural-exit cleanup child") {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "natural-exit cleanup test child did not exit"
            );
            thread::sleep(Duration::from_millis(1));
        }
        let cleanup_called = AtomicBool::new(false);
        let mut child_reaped = false;
        let result = cleanup_owned_child_after_error_with(
            &mut child,
            &mut child_reaped,
            |child: &mut std::process::Child| {
                let pid = RustixPid::from_raw(
                    i32::try_from(child.id()).expect("natural-exit child PID exceeds i32"),
                )
                .expect("natural-exit child PID is invalid");
                assert!(matches!(
                    waitid(
                        WaitId::Pid(pid),
                        WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
                    ),
                    Ok(Some(_))
                ));
                cleanup_called.store(true, Ordering::Release);
                Ok(())
            },
        );

        assert!(result.is_ok());
        assert!(cleanup_called.load(Ordering::Acquire));
        assert!(child_reaped);
        assert_child_was_reaped(&child);
    }

    #[test]
    fn capture_failure_joins_all_remaining_workers() {
        use std::os::unix::net::UnixStream;

        let (mut stdout_reader, stdout_writer) = UnixStream::pair().expect("stdout pipe pair");
        let (mut stderr_reader, stderr_writer) = UnixStream::pair().expect("stderr pipe pair");
        let stop_capture = Arc::new(AtomicBool::new(false));
        let stdout_stopped = Arc::new(AtomicBool::new(false));
        let stderr_stopped = Arc::new(AtomicBool::new(false));
        let stdout_marker = Arc::clone(&stdout_stopped);
        let stderr_marker = Arc::clone(&stderr_stopped);
        let (stdin_worker, stdin_done) = spawn_capture(|| {
            Err::<(), io::Error>(io::Error::other("synthetic stdin capture failure"))
        });
        let stdout_stop = Arc::clone(&stop_capture);
        let (stdout_worker, stdout_done) = spawn_capture(move || {
            let result = read_all(&mut stdout_reader, &stdout_stop);
            stdout_marker.store(true, Ordering::Release);
            result
        });
        let stderr_stop = Arc::clone(&stop_capture);
        let (stderr_worker, stderr_done) = spawn_capture(move || {
            let result = read_all(&mut stderr_reader, &stderr_stop);
            stderr_marker.store(true, Ordering::Release);
            result
        });

        let result = finish_captures(
            stdin_worker,
            &stdin_done,
            stdout_worker,
            &stdout_done,
            stderr_worker,
            &stderr_done,
            &stop_capture,
        );

        assert!(matches!(result, Err(ProcessError::Io(_))));
        assert!(stdout_stopped.load(Ordering::Acquire));
        assert!(stderr_stopped.load(Ordering::Acquire));
        drop(stdout_writer);
        drop(stderr_writer);
    }
}

#[cfg(all(test, not(any(target_os = "macos", target_os = "linux"))))]
mod unsupported_platform_tests {
    use super::*;

    #[test]
    fn capture_rejects_unsupported_platform_before_spawn() {
        let invocation = Invocation {
            executable: PathBuf::from("definitely-not-an-executable"),
            args: Vec::new(),
            stdin: Vec::new(),
            timeout: Duration::from_secs(1),
            current_dir: None,
            environment: BTreeMap::new(),
        };
        assert!(matches!(
            run_process(&invocation),
            Err(ProcessError::Unsupported)
        ));
        assert!(matches!(
            run_process_with_environment(&invocation, &BTreeMap::new()),
            Err(ProcessError::Unsupported)
        ));
        assert!(matches!(
            run_process_with_environment_bounded(&invocation, &BTreeMap::new(), 1),
            Err(ProcessError::Unsupported)
        ));
    }
}
