//! Per-process wall, latency, CPU, memory, and byte measurement.

use std::{
    io::{self, Write},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        Arc,
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
    /// Maximum stdout bytes written before the harness stops the process.
    pub output_limit: u64,
    /// Optional peak resident-memory limit enforced when the host exposes RSS.
    pub rss_limit: Option<u64>,
    /// Retain output files after a successful invocation for semantic checks.
    pub retain_output: bool,
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
    /// Preserved stdout file for an unsuccessful invocation.
    pub stdout_path: Option<PathBuf>,
    /// Preserved stderr file for an unsuccessful invocation.
    pub stderr_path: Option<PathBuf>,
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
    /// The process group crossed its configured resident-memory limit.
    RssLimit,
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
/// On Unix, `/usr/bin/time` writes timing metrics and, where supported, RSS data
/// to a separate temporary file. Tool stdout and stderr are spooled to files,
/// not held in memory or passed to the invoking terminal. Successful output
/// files are deleted.
/// Failed output files are retained for diagnosis. Fields unsupported by the
/// local implementation remain `None`.
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
    let stdout_file = tempfile::NamedTempFile::new()?;
    let stderr_file = tempfile::NamedTempFile::new()?;
    let mut command = measured_command(invocation, resource_file.path());
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::from(stdout_file.reopen()?))
        .stderr(Stdio::from(stderr_file.reopen()?));
    if let Some(directory) = &invocation.current_dir {
        command.current_dir(directory);
    }
    configure_process_group(&mut command);
    let mut child = command.spawn()?;
    let process_id = child.id();
    let rss_sampler_stop = Arc::new(AtomicBool::new(false));
    let sampled_peak = Arc::new(AtomicU64::new(0));
    let rss_sampler = spawn_rss_sampler(
        process_id,
        Arc::clone(&rss_sampler_stop),
        Arc::clone(&sampled_peak),
    );
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("stdin pipe"))?;
    let stdin_bytes = invocation.stdin.clone();
    let stdin_worker = thread::spawn(move || {
        let result = stdin.write_all(&stdin_bytes);
        drop(stdin);
        result
    });
    let mut first_result_micros = None;

    let mut forced = None;
    let exit = loop {
        let remaining = invocation.timeout.saturating_sub(started.elapsed());
        let interval = remaining.min(Duration::from_millis(2));
        if let Some(status) = child.wait_timeout(interval)? {
            break status;
        }
        let output_bytes = stdout_file.as_file().metadata()?.len();
        if output_bytes > 0 && first_result_micros.is_none() {
            first_result_micros = Some(started.elapsed().as_micros());
        }
        if output_bytes > invocation.output_limit {
            forced = Some(MeasuredStatus::OutputLimit);
        } else if invocation
            .rss_limit
            .is_some_and(|limit| sampled_peak.load(Ordering::Relaxed) > limit)
        {
            forced = Some(MeasuredStatus::RssLimit);
        } else if started.elapsed() >= invocation.timeout {
            forced = Some(MeasuredStatus::Timeout);
        }
        if forced.is_some() {
            terminate_process_group(process_id, &mut child)?;
            break child.wait()?;
        }
    };
    rss_sampler_stop.store(true, Ordering::Relaxed);
    rss_sampler
        .join()
        .map_err(|_| MeasureError::CaptureWorker)?;
    let sampled_peak_rss = match sampled_peak.load(Ordering::Relaxed) {
        0 => None,
        value => Some(value),
    };

    let _stdin_result = stdin_worker
        .join()
        .map_err(|_| MeasureError::CaptureWorker)?;
    let output_bytes = stdout_file.as_file().metadata()?.len();
    if output_bytes > 0 && first_result_micros.is_none() {
        first_result_micros = Some(started.elapsed().as_micros());
    }
    let resources = std::fs::read_to_string(resource_file.path()).unwrap_or_default();
    let inferred_signal = infer_signal(exit, &resources);
    let status = if output_bytes > invocation.output_limit {
        MeasuredStatus::OutputLimit
    } else {
        forced.unwrap_or_else(|| {
            if inferred_signal.is_some() {
                MeasuredStatus::Signaled
            } else {
                MeasuredStatus::Exited
            }
        })
    };
    let preserve_output =
        invocation.retain_output || status != MeasuredStatus::Exited || exit.code() != Some(0);
    let stdout_path = if preserve_output {
        Some(stdout_file.keep().map_err(|error| error.error)?.1)
    } else {
        None
    };
    let stderr_path = if preserve_output {
        Some(stderr_file.keep().map_err(|error| error.error)?.1)
    } else {
        None
    };
    Ok(MeasuredOutcome {
        status,
        exit_code: exit.code(),
        signal: inferred_signal,
        wall_time_micros: started.elapsed().as_micros(),
        first_result_micros,
        user_cpu_micros: resource_seconds(&resources, "user"),
        system_cpu_micros: resource_seconds(&resources, "sys"),
        peak_rss_bytes: maximum_available(resource_rss(&resources), sampled_peak_rss),
        output_bytes,
        stdout_path,
        stderr_path,
    })
}

fn maximum_available(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn spawn_rss_sampler(
    process_group: u32,
    stop: Arc<AtomicBool>,
    maximum: Arc<AtomicU64>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        while !stop.load(Ordering::Relaxed) {
            if let Some(bytes) = process_group_rss(process_group) {
                maximum.fetch_max(bytes, Ordering::Relaxed);
            }
            thread::sleep(Duration::from_millis(25));
        }
        if let Some(bytes) = process_group_rss(process_group) {
            maximum.fetch_max(bytes, Ordering::Relaxed);
        }
    })
}

#[cfg(target_os = "macos")]
fn process_group_rss(process_group: u32) -> Option<u64> {
    let output = Command::new("ps")
        .args(["-axo", "pgid=,rss="])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let kibibytes = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let group = fields.next()?.parse::<u32>().ok()?;
            let rss = fields.next()?.parse::<u64>().ok()?;
            (group == process_group).then_some(rss)
        })
        .sum::<u64>();
    (kibibytes > 0).then(|| kibibytes.saturating_mul(1024))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn process_group_rss(process_group: u32) -> Option<u64> {
    let output = Command::new("ps")
        .args(["-o", "rss=", "-g", &process_group.to_string()])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let kibibytes = String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .filter_map(|value| value.parse::<u64>().ok())
        .sum::<u64>();
    (kibibytes > 0).then(|| kibibytes.saturating_mul(1024))
}

#[cfg(not(unix))]
fn process_group_rss(_process_group: u32) -> Option<u64> {
    None
}

fn measured_command(invocation: &BenchmarkInvocation, resource_path: &std::path::Path) -> Command {
    #[cfg(unix)]
    {
        let mut command = Command::new("/usr/bin/time");
        command.arg("-p");
        // Keep macOS on `-p`: `time -l` invokes restricted sysctl calls and can
        // make the wrapper fail even when the measured command succeeds.
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
        Err(_) => child.kill(),
    }
}

#[cfg(not(unix))]
fn terminate_process_group(_process_id: u32, child: &mut std::process::Child) -> io::Result<()> {
    child.kill()
}

fn resource_seconds(report: &str, label: &str) -> Option<u128> {
    report.lines().find_map(|line| {
        let line = line.trim();
        let prefix = format!("{label} ");
        let seconds = line.strip_prefix(&prefix).map(str::trim).or_else(|| {
            line.strip_suffix(label)
                .and_then(|value| value.split_whitespace().last())
        })?;
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
