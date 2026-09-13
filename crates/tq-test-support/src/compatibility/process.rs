//! Timeout-safe subprocess execution and capture.

use std::{
    collections::BTreeMap,
    io::{self, Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
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
    /// Exact stdout bytes.
    pub stdout: Vec<u8>,
    /// Exact stderr bytes.
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
    run_process_with_environment(invocation, &BTreeMap::new())
}

/// Runs a subprocess with environment overrides confined to that child.
///
/// # Errors
///
/// Returns the same process and capture errors as [`run_process`].
pub fn run_process_with_environment(
    invocation: &Invocation,
    environment: &BTreeMap<String, String>,
) -> Result<ProcessOutcome, ProcessError> {
    let started = Instant::now();
    let SpawnedProcess {
        mut child,
        mut stdin,
        mut stdout,
        mut stderr,
    } = spawn_process(invocation, environment)?;
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
    if let Some(current_dir) = &invocation.current_dir {
        command.current_dir(current_dir);
    }
    let mut child = command.spawn()?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("piped stdin was not created"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("piped stdout was not created"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("piped stderr was not created"))?;
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
/// Returns an I/O or capture-worker error. Nonzero exits, signals, timeouts,
/// and output-limit terminations are successful observations.
pub fn run_process_with_environment_bounded(
    invocation: &Invocation,
    environment: &BTreeMap<String, String>,
    output_limit: u64,
) -> Result<BoundedProcessOutcome, ProcessError> {
    let started = Instant::now();
    let SpawnedProcess {
        mut child,
        mut stdin,
        mut stdout,
        mut stderr,
    } = spawn_process(invocation, environment)?;
    let stdin_bytes = invocation.stdin.clone();
    let stdout_limited = Arc::new(AtomicBool::new(false));
    let stderr_limited = Arc::new(AtomicBool::new(false));

    let stdin_worker = thread::spawn(move || {
        let result = stdin.write_all(&stdin_bytes);
        drop(stdin);
        result
    });
    let stdout_flag = Arc::clone(&stdout_limited);
    let stdout_worker = thread::spawn(move || {
        read_limited(
            &mut stdout,
            OutputStream::Stdout,
            output_limit,
            &stdout_flag,
        )
    });
    let stderr_flag = Arc::clone(&stderr_limited);
    let stderr_worker = thread::spawn(move || {
        read_limited(
            &mut stderr,
            OutputStream::Stderr,
            output_limit,
            &stderr_flag,
        )
    });

    let mut timed_out = false;
    let exit = loop {
        if stdout_limited.load(Ordering::Acquire) || stderr_limited.load(Ordering::Acquire) {
            break terminate_child(&mut child)?;
        }
        if let Some(exit) = child.try_wait()? {
            break exit;
        }
        if started.elapsed() >= invocation.timeout {
            timed_out = true;
            break terminate_child(&mut child)?;
        }
        thread::sleep(Duration::from_millis(1));
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

fn read_all(reader: &mut impl Read) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    Ok(bytes)
}

struct BoundedRead {
    bytes: Vec<u8>,
    limit: Option<OutputLimit>,
}

fn read_limited(
    reader: &mut impl Read,
    stream: OutputStream,
    output_limit: u64,
    limit_reached: &AtomicBool,
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
        let read = read_retry(reader, &mut buffer[..read_len])?;
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

fn terminate_child(child: &mut std::process::Child) -> io::Result<std::process::ExitStatus> {
    if child.try_wait()?.is_none()
        && let Err(error) = child.kill()
        && child.try_wait()?.is_none()
    {
        return Err(error);
    }
    child.wait()
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
