//! Timeout-safe subprocess execution and capture.

use std::{
    io::{self, Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use thiserror::Error;
use wait_timeout::ChildExt;

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
}

/// Classified process completion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessStatus {
    /// Process exited normally, including nonzero exit codes.
    Exited,
    /// Process exceeded its wall-time limit and was killed.
    TimedOut,
    /// Process terminated from a signal.
    Signaled,
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
    /// Exact stdout bytes.
    pub stdout: Vec<u8>,
    /// Exact stderr bytes.
    pub stderr: Vec<u8>,
    /// Observed wall time in microseconds.
    pub wall_time_micros: u128,
    /// Secret-redacted executable and argument vector.
    pub recorded_command: Vec<String>,
}

/// Stable harness process failures.
#[derive(Debug, Error)]
pub enum ProcessError {
    /// Process creation or lifecycle I/O failed.
    #[error("subprocess I/O failed: {0}")]
    Io(#[from] io::Error),
    /// A capture worker panicked.
    #[error("subprocess capture worker panicked")]
    CaptureWorker,
}

/// Runs one subprocess with isolated streams and a hard wall-time limit.
///
/// # Errors
///
/// Returns an I/O or capture-worker error. Nonzero exits, signals, and timeouts
/// are successful observations rather than harness errors.
pub fn run_process(invocation: &Invocation) -> Result<ProcessOutcome, ProcessError> {
    let started = Instant::now();
    let mut command = Command::new(&invocation.executable);
    command
        .args(&invocation.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(current_dir) = &invocation.current_dir {
        command.current_dir(current_dir);
    }
    let mut child = command.spawn()?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("piped stdin was not created"))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("piped stdout was not created"))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("piped stderr was not created"))?;
    let stdin_bytes = invocation.stdin.clone();

    let stdin_worker = thread::spawn(move || {
        let result = stdin.write_all(&stdin_bytes);
        drop(stdin);
        result
    });
    let stdout_worker = thread::spawn(move || read_all(&mut stdout));
    let stderr_worker = thread::spawn(move || read_all(&mut stderr));

    let waited = child.wait_timeout(invocation.timeout)?;
    let timed_out = waited.is_none();
    let exit = if let Some(exit) = waited {
        exit
    } else {
        child.kill()?;
        child.wait()?
    };

    let _ = stdin_worker
        .join()
        .map_err(|_| ProcessError::CaptureWorker)?;
    let stdout = stdout_worker
        .join()
        .map_err(|_| ProcessError::CaptureWorker)??;
    let stderr = stderr_worker
        .join()
        .map_err(|_| ProcessError::CaptureWorker)??;
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

fn read_all(reader: &mut impl Read) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    Ok(bytes)
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
