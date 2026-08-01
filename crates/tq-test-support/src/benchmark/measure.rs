//! Per-process wall, latency, CPU, memory, and byte measurement.

use std::{
    io::{self, Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use wait_timeout::ChildExt;

/// Exact command measurement request.
#[derive(Clone, Debug)]
pub struct BenchmarkInvocation {
    /// Executable path.
    pub executable: PathBuf,
    /// Argument vector.
    pub args: Vec<String>,
    /// Bytes supplied to stdin.
    pub stdin: Vec<u8>,
    /// Working directory.
    pub current_dir: Option<PathBuf>,
    /// Wall timeout.
    pub timeout: Duration,
    /// Captured stdout limit.
    pub output_limit: u64,
}

/// First-class process measurement outcome.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MeasuredOutcome {
    /// Completion class.
    pub status: MeasuredStatus,
    /// Exit code for normal completion.
    pub exit_code: Option<i32>,
    /// Signal number on Unix.
    pub signal: Option<i32>,
    /// Total wall duration.
    pub wall_time_micros: u128,
    /// Time until the first stdout byte, or unavailable/no output.
    pub first_result_micros: Option<u128>,
    /// User CPU time.
    pub user_cpu_micros: Option<u128>,
    /// System CPU time.
    pub system_cpu_micros: Option<u128>,
    /// Peak process resident bytes.
    pub peak_rss_bytes: Option<u64>,
    /// Total stdout bytes observed, including bytes beyond capture limit.
    pub output_bytes: u64,
    /// Captured stdout prefix.
    pub stdout: Vec<u8>,
    /// Captured stderr prefix.
    pub stderr: Vec<u8>,
}

/// Process measurement status.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MeasuredStatus {
    /// Normal process exit.
    Exited,
    /// Wall timeout and forced termination.
    Timeout,
    /// Signal termination.
    Signaled,
    /// Captured output crossed its configured limit.
    OutputLimit,
}

/// Benchmark process lifecycle failure.
#[derive(Debug, Error)]
pub enum MeasureError {
    /// Spawn, pipe, or wait failure.
    #[error("benchmark process I/O failed: {0}")]
    Io(#[from] io::Error),
    /// Capture thread panicked.
    #[error("benchmark capture worker panicked")]
    CaptureWorker,
}

/// Measures one fresh process invocation.
///
/// On Unix, `/usr/bin/time` writes CPU/RSS metrics to a separate temporary
/// file while stdout and stderr remain the tool's original streams. Fields
/// unsupported by the local implementation remain `None`.
///
/// # Errors
///
/// Returns spawn, wait, pipe, or worker failures. Nonzero exit, timeout,
/// signal, and output-limit outcomes remain successful measurements.
#[allow(
    clippy::too_many_lines,
    reason = "the measurement lifecycle is kept linear so every pipe and process is reaped"
)]
pub fn measure_process(invocation: &BenchmarkInvocation) -> Result<MeasuredOutcome, MeasureError> {
    let started = Instant::now();
    let resource_file = tempfile::NamedTempFile::new()?;
    let mut command = measured_command(invocation, resource_file.path());
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(directory) = &invocation.current_dir {
        command.current_dir(directory);
    }
    configure_process_group(&mut command);
    let mut child = command.spawn()?;
    let process_id = child.id();
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("stdin pipe"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("stdout pipe"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("stderr pipe"))?;

    let stdin_bytes = invocation.stdin.clone();
    let stdin_worker = thread::spawn(move || {
        let result = stdin.write_all(&stdin_bytes);
        drop(stdin);
        result
    });
    let output_exceeded = Arc::new(AtomicBool::new(false));
    let output_bytes = Arc::new(AtomicU64::new(0));
    let first_result = Arc::new(Mutex::new(None));
    let stdout_worker = {
        let exceeded = Arc::clone(&output_exceeded);
        let byte_count = Arc::clone(&output_bytes);
        let first = Arc::clone(&first_result);
        let output_limit = invocation.output_limit;
        thread::spawn(move || {
            read_stream(
                stdout,
                output_limit,
                &exceeded,
                &byte_count,
                Some((&first, started)),
            )
        })
    };
    let stderr_worker = thread::spawn(move || {
        let exceeded = AtomicBool::new(false);
        let bytes = AtomicU64::new(0);
        read_stream(stderr, 1024 * 1024, &exceeded, &bytes, None)
    });

    let mut forced = None;
    let exit = loop {
        let remaining = invocation.timeout.saturating_sub(started.elapsed());
        let interval = remaining.min(Duration::from_millis(2));
        if let Some(status) = child.wait_timeout(interval)? {
            break status;
        }
        if output_exceeded.load(Ordering::Relaxed) {
            forced = Some(MeasuredStatus::OutputLimit);
        } else if started.elapsed() >= invocation.timeout {
            forced = Some(MeasuredStatus::Timeout);
        }
        if forced.is_some() {
            terminate_process_group(process_id, &mut child)?;
            break child.wait()?;
        }
    };

    let _stdin_result = stdin_worker
        .join()
        .map_err(|_| MeasureError::CaptureWorker)?;
    let stdout = stdout_worker
        .join()
        .map_err(|_| MeasureError::CaptureWorker)??;
    let stderr = stderr_worker
        .join()
        .map_err(|_| MeasureError::CaptureWorker)??;
    let resources = std::fs::read_to_string(resource_file.path()).unwrap_or_default();
    let inferred_signal = infer_signal(exit, &resources);
    let status = forced.unwrap_or_else(|| {
        if inferred_signal.is_some() {
            MeasuredStatus::Signaled
        } else {
            MeasuredStatus::Exited
        }
    });
    let first_result_micros = *first_result
        .lock()
        .map_err(|_| io::Error::other("first-result mutex poisoned"))?;
    Ok(MeasuredOutcome {
        status,
        exit_code: exit.code(),
        signal: inferred_signal,
        wall_time_micros: started.elapsed().as_micros(),
        first_result_micros,
        user_cpu_micros: resource_seconds(&resources, "user"),
        system_cpu_micros: resource_seconds(&resources, "sys"),
        peak_rss_bytes: resource_rss(&resources),
        output_bytes: output_bytes.load(Ordering::Relaxed),
        stdout,
        stderr,
    })
}

fn measured_command(invocation: &BenchmarkInvocation, resource_path: &std::path::Path) -> Command {
    #[cfg(unix)]
    {
        let mut command = Command::new("/usr/bin/time");
        command.arg("-p");
        #[cfg(target_os = "macos")]
        command.arg("-l");
        #[cfg(all(unix, not(target_os = "macos")))]
        command.arg("-v");
        command
            .arg("-o")
            .arg(resource_path)
            .arg(&invocation.executable)
            .args(&invocation.args);
        command
    }
    #[cfg(not(unix))]
    {
        let _ = resource_path;
        let mut command = Command::new(&invocation.executable);
        command.args(&invocation.args);
        command
    }
}

#[cfg(unix)]
fn configure_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt as _;
    command.process_group(0);
}

#[cfg(not(unix))]
fn configure_process_group(_command: &mut Command) {}

#[cfg(unix)]
fn terminate_process_group(process_id: u32, child: &mut std::process::Child) -> io::Result<()> {
    use nix::{sys::signal, unistd::Pid};
    let pid = i32::try_from(process_id).map_err(|_| io::Error::other("PID overflow"))?;
    match signal::killpg(Pid::from_raw(pid), signal::Signal::SIGKILL) {
        Ok(()) | Err(nix::errno::Errno::ESRCH) => Ok(()),
        Err(error) => {
            child.kill()?;
            Err(io::Error::other(error))
        }
    }
}

#[cfg(not(unix))]
fn terminate_process_group(_process_id: u32, child: &mut std::process::Child) -> io::Result<()> {
    child.kill()
}

fn read_stream(
    mut reader: impl Read,
    limit: u64,
    exceeded: &AtomicBool,
    total: &AtomicU64,
    first: Option<(&Mutex<Option<u128>>, Instant)>,
) -> io::Result<Vec<u8>> {
    let capacity = usize::try_from(limit.min(1024 * 1024)).unwrap_or(1024 * 1024);
    let mut captured = Vec::with_capacity(capacity);
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        if total.fetch_add(read as u64, Ordering::Relaxed) == 0
            && let Some((slot, started)) = first
        {
            *slot
                .lock()
                .map_err(|_| io::Error::other("first-result mutex poisoned"))? =
                Some(started.elapsed().as_micros());
        }
        let remaining =
            usize::try_from(limit.saturating_sub(captured.len() as u64)).unwrap_or(usize::MAX);
        captured.extend_from_slice(&buffer[..read.min(remaining)]);
        if total.load(Ordering::Relaxed) > limit {
            exceeded.store(true, Ordering::Relaxed);
        }
    }
    Ok(captured)
}

fn resource_seconds(report: &str, label: &str) -> Option<u128> {
    report.lines().find_map(|line| {
        let line = line.trim();
        let seconds = if let Some(value) = line.strip_prefix(&format!("{label} ")) {
            value.trim()
        } else if let Some(value) = line.strip_suffix(label) {
            value.split_whitespace().last()?
        } else {
            return None;
        };
        parse_seconds_micros(seconds)
    })
}

fn parse_seconds_micros(value: &str) -> Option<u128> {
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    let seconds = whole.parse::<u128>().ok()?;
    let digits = fraction.as_bytes();
    if !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let kept = &fraction[..fraction.len().min(6)];
    let fractional = if kept.is_empty() {
        0
    } else {
        kept.parse::<u128>().ok()? * 10_u128.pow(u32::try_from(6 - kept.len()).ok()?)
    };
    seconds.checked_mul(1_000_000)?.checked_add(fractional)
}

fn resource_rss(report: &str) -> Option<u64> {
    for line in report.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_suffix("maximum resident set size") {
            return value.trim().parse().ok();
        }
        if let Some((_, value)) = trimmed.split_once("Maximum resident set size (kbytes):") {
            return value.trim().parse::<u64>().ok()?.checked_mul(1024);
        }
    }
    None
}

#[cfg(unix)]
fn infer_signal(status: std::process::ExitStatus, report: &str) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt as _;
    status.signal().or_else(|| {
        report.lines().find_map(|line| {
            line.trim()
                .strip_prefix("Command terminated by signal ")?
                .parse()
                .ok()
        })
    })
}

#[cfg(not(unix))]
fn infer_signal(_status: std::process::ExitStatus, _report: &str) -> Option<i32> {
    None
}
