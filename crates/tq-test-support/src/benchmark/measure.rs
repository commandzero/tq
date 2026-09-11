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

/// Source of an authoritative peak RSS value.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RssProvenance {
    /// GNU `/usr/bin/time -v` on Linux.
    GnuTimeV,
    /// BSD `/usr/bin/time -l` on macOS.
    BsdTimeL,
}

impl RssProvenance {
    /// Stable report label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::GnuTimeV => "gnu-time-v",
            Self::BsdTimeL => "bsd-time-l",
        }
    }
}

/// Evidence that the local host can collect the required RSS metrics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RssPreflight {
    /// Format emitted by the authoritative `/usr/bin/time` implementation.
    pub provenance: RssProvenance,
}

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
    /// Explicit source of the authoritative peak RSS value.
    pub rss_provenance: RssProvenance,
    /// Peak resident bytes observed by the process-group sampler, when any.
    pub process_group_peak_rss_bytes: Option<u64>,
    /// Total stdout bytes observed, including bytes beyond capture limit.
    pub output_bytes: u64,
    /// Preserved stdout file for an unsuccessful invocation.
    pub stdout_path: Option<PathBuf>,
    /// Preserved stderr file for an unsuccessful invocation.
    pub stderr_path: Option<PathBuf>,
    /// Whether process-group sampling observed a positive RSS value.
    pub process_group_rss_observed: bool,
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
    /// The selected authoritative time implementation did not emit a usable RSS value.
    #[error("authoritative peak RSS unavailable: {0}")]
    RssUnavailable(String),
    /// The process-group sampler could not support a configured RSS limit.
    #[error("process-group RSS inspection unavailable: {0}")]
    RssSamplerUnavailable(String),
}

/// Verifies the complete RSS collection path before a campaign allocates or
/// replays a corpus.
///
/// The probe allocates and touches 64 MiB, keeps the child resident long enough
/// for the process-group sampler to observe it, and requires a positive
/// authoritative `/usr/bin/time` value. A caller must abandon the campaign
/// when this fails.
///
/// # Errors
///
/// Returns an infrastructure error when the authoritative time output or
/// process-group sampler cannot provide the required RSS evidence.
pub fn preflight_rss() -> Result<RssPreflight, MeasureError> {
    #[cfg(unix)]
    {
        let measured = measure_process(&BenchmarkInvocation {
            executable: PathBuf::from("python3"),
            args: vec![
                "-c".to_owned(),
                "import time; b=bytearray(64*1024*1024); b[::4096]=b'\\x01'*(len(b)//4096); time.sleep(0.4)"
                    .to_owned(),
            ],
            stdin: Vec::new(),
            current_dir: None,
            timeout: Duration::from_secs(2),
            output_limit: 1024,
            rss_limit: None,
            retain_output: false,
        })?;
        if measured.status != MeasuredStatus::Exited || measured.exit_code != Some(0) {
            return Err(MeasureError::RssUnavailable(format!(
                "preflight child did not exit successfully (status {:?}, exit {:?})",
                measured.status, measured.exit_code
            )));
        }
        if measured.peak_rss_bytes.is_none_or(|bytes| bytes == 0) {
            return Err(MeasureError::RssUnavailable(
                "preflight child produced no positive peak RSS".to_owned(),
            ));
        }
        if !measured.process_group_rss_observed {
            return Err(MeasureError::RssSamplerUnavailable(
                "preflight child was not observable through its process group".to_owned(),
            ));
        }
        Ok(RssPreflight {
            provenance: measured.rss_provenance,
        })
    }
    #[cfg(not(unix))]
    {
        Err(MeasureError::RssUnavailable(
            "no supported authoritative time implementation".to_owned(),
        ))
    }
}

/// Measures one fresh process invocation.
///
/// On Unix, `/usr/bin/time` writes timing metrics and authoritative RSS data to
/// a separate temporary file. Tool stdout and stderr are spooled to files, not
/// held in memory or passed to the invoking terminal. Successful output files
/// are deleted. Failed output files are retained for diagnosis. A measurement
/// without a positive, recognized RSS value is an infrastructure error.
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
    measure_process_with_probe(invocation, process_group_rss)
}

#[allow(
    clippy::too_many_lines,
    reason = "keep process and worker cleanup together"
)]
fn measure_process_with_probe(
    invocation: &BenchmarkInvocation,
    probe: fn(u32) -> io::Result<Option<u64>>,
) -> Result<MeasuredOutcome, MeasureError> {
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
    let sampled_observed = Arc::new(AtomicBool::new(false));
    match probe(process_id) {
        Ok(Some(bytes)) => {
            sampled_observed.store(true, Ordering::Relaxed);
            sampled_peak.fetch_max(bytes, Ordering::Relaxed);
        }
        Ok(None) => {}
        Err(error) => {
            emergency_terminate_process_group(process_id, &mut child);
            return Err(MeasureError::RssSamplerUnavailable(error.to_string()));
        }
    }
    let sampler_failed = Arc::new(AtomicBool::new(false));
    let rss_sampler = spawn_rss_sampler(
        process_id,
        Arc::clone(&rss_sampler_stop),
        Arc::clone(&sampled_peak),
        Arc::clone(&sampled_observed),
        Arc::clone(&sampler_failed),
        probe,
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
        if sampler_failed.load(Ordering::Acquire) {
            emergency_terminate_process_group(process_id, &mut child);
            rss_sampler_stop.store(true, Ordering::Relaxed);
            let result = rss_sampler.join();
            let _ = stdin_worker.join();
            return Err(match result {
                Ok(Err(error)) => MeasureError::RssSamplerUnavailable(error.to_string()),
                _ => MeasureError::CaptureWorker,
            });
        }
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
            if let Err(error) = terminate_process_group(process_id, &mut child) {
                emergency_terminate_process_group(process_id, &mut child);
                rss_sampler_stop.store(true, Ordering::Relaxed);
                let _ = rss_sampler.join();
                let _ = stdin_worker.join();
                return Err(MeasureError::Io(error));
            }
            break child.wait()?;
        }
    };
    rss_sampler_stop.store(true, Ordering::Relaxed);
    let sampler_result = rss_sampler.join();
    let process_group_peak_rss_bytes = match sampled_peak.load(Ordering::Relaxed) {
        0 => None,
        value => Some(value),
    };
    let process_group_rss_observed = sampled_observed.load(Ordering::Relaxed);

    let _stdin_result = stdin_worker
        .join()
        .map_err(|_| MeasureError::CaptureWorker)?;
    sampler_result
        .map_err(|_| MeasureError::CaptureWorker)?
        .map_err(|error| MeasureError::RssSamplerUnavailable(error.to_string()))?;
    let output_bytes = stdout_file.as_file().metadata()?.len();
    if output_bytes > 0 && first_result_micros.is_none() {
        first_result_micros = Some(started.elapsed().as_micros());
    }
    let resources = std::fs::read_to_string(resource_file.path()).unwrap_or_default();
    let (peak_rss_bytes, rss_provenance) = resource_rss(&resources).ok_or_else(|| {
        MeasureError::RssUnavailable(format!(
            "/usr/bin/time did not emit a positive {} value",
            expected_rss_provenance().label()
        ))
    })?;
    let inferred_signal = infer_signal(exit, &resources);
    let status = if output_bytes > invocation.output_limit {
        MeasuredStatus::OutputLimit
    } else if invocation
        .rss_limit
        .is_some_and(|limit| peak_rss_bytes > limit)
    {
        MeasuredStatus::RssLimit
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
        peak_rss_bytes: Some(peak_rss_bytes),
        rss_provenance,
        process_group_peak_rss_bytes,
        output_bytes,
        stdout_path,
        stderr_path,
        process_group_rss_observed,
    })
}

fn spawn_rss_sampler(
    process_group: u32,
    stop: Arc<AtomicBool>,
    maximum: Arc<AtomicU64>,
    observed: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
    probe: fn(u32) -> io::Result<Option<u64>>,
) -> thread::JoinHandle<io::Result<()>> {
    thread::spawn(move || {
        loop {
            match probe(process_group) {
                Ok(Some(bytes)) => {
                    observed.store(true, Ordering::Relaxed);
                    maximum.fetch_max(bytes, Ordering::Relaxed);
                }
                Ok(None) => {}
                Err(error) => {
                    failed.store(true, Ordering::Release);
                    return Err(error);
                }
            }
            if stop.load(Ordering::Relaxed) {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(25));
        }
    })
}

#[cfg(unix)]
fn process_group_rss(process_group: u32) -> io::Result<Option<u64>> {
    // An exited group can disappear before a successful scan. That is distinct
    // from a failed inspection; authoritative time RSS is still required.
    let output = Command::new("ps").args(["-axo", "pgid=,rss="]).output()?;
    parse_process_group_rss(process_group, &output)
}

#[cfg(unix)]
fn parse_process_group_rss(
    process_group: u32,
    output: &std::process::Output,
) -> io::Result<Option<u64>> {
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "ps failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let text = std::str::from_utf8(&output.stdout).map_err(io::Error::other)?;
    let mut kibibytes = 0_u64;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let mut fields = line.split_whitespace();
        let group = fields.next().and_then(|value| value.parse::<u32>().ok());
        let rss = fields.next().and_then(|value| value.parse::<u64>().ok());
        let (Some(group), Some(rss)) = (group, rss) else {
            return Err(io::Error::other("invalid ps PGID/RSS output"));
        };
        if group == process_group {
            kibibytes = kibibytes.saturating_add(rss);
        }
    }
    Ok((kibibytes > 0).then(|| kibibytes.saturating_mul(1024)))
}

#[cfg(not(unix))]
fn process_group_rss(_process_group: u32) -> io::Result<Option<u64>> {
    Err(io::Error::other("process-group inspection unsupported"))
}

fn measured_command(invocation: &BenchmarkInvocation, resource_path: &std::path::Path) -> Command {
    #[cfg(unix)]
    {
        let mut command = Command::new("/usr/bin/time");
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
    use nix::{errno::Errno, sys::signal, unistd::Pid};
    let supervisor = i32::try_from(process_id).map_err(|_| io::Error::other("PID overflow"))?;
    for _ in 0..8 {
        let members = process_group_members(process_id)?;
        let targets = members
            .into_iter()
            .filter(|pid| *pid != supervisor)
            .collect::<Vec<_>>();
        if !targets.is_empty() {
            for pid in targets {
                match signal::kill(Pid::from_raw(pid), signal::Signal::SIGKILL) {
                    Ok(()) | Err(Errno::ESRCH) => {}
                    Err(error) => return Err(io::Error::other(error.to_string())),
                }
            }
            return Ok(());
        }
        if child.try_wait()?.is_some() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(1));
    }
    Err(io::Error::other(
        "measured process group has no killable child; refusing to kill /usr/bin/time",
    ))
}

#[cfg(unix)]
fn emergency_terminate_process_group(process_id: u32, child: &mut std::process::Child) {
    use nix::{sys::signal, unistd::Pid};
    if let Ok(pid) = i32::try_from(process_id) {
        let _ = signal::killpg(Pid::from_raw(pid), signal::Signal::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(not(unix))]
fn terminate_process_group(_process_id: u32, child: &mut std::process::Child) -> io::Result<()> {
    child.kill()
}

#[cfg(not(unix))]
fn emergency_terminate_process_group(_process_id: u32, child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn resource_seconds(report: &str, label: &str) -> Option<u128> {
    report.lines().find_map(|line| {
        let line = line.trim();
        let verbose_prefix = match label {
            "user" => "User time (seconds):",
            "sys" => "System time (seconds):",
            _ => return None,
        };
        if let Some(value) = line.strip_prefix(verbose_prefix) {
            return parse_seconds_micros(value.trim());
        }
        let mut previous = None;
        for token in line.split_whitespace() {
            if token == label {
                return previous.and_then(parse_seconds_micros);
            }
            previous = Some(token);
        }
        let prefix = format!("{label} ");
        let seconds = line.strip_prefix(&prefix).map(str::trim).or_else(|| {
            line.strip_suffix(label)
                .and_then(|value| value.split_whitespace().last())
        })?;
        parse_seconds_micros(seconds)
    })
}

#[cfg(unix)]
fn process_group_members(process_group: u32) -> io::Result<Vec<i32>> {
    let output = Command::new("ps").args(["-axo", "pid=,pgid="]).output()?;
    if !output.status.success() {
        return Err(io::Error::other("ps failed while inspecting process group"));
    }
    let group = i32::try_from(process_group).map_err(|_| io::Error::other("PGID overflow"))?;
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let process_id = fields.next()?.parse::<i32>().ok()?;
            let process_group_id = fields.next()?.parse::<i32>().ok()?;
            (process_group_id == group).then_some(process_id)
        })
        .collect())
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

fn expected_rss_provenance() -> RssProvenance {
    #[cfg(target_os = "macos")]
    {
        RssProvenance::BsdTimeL
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        RssProvenance::GnuTimeV
    }
    #[cfg(not(unix))]
    {
        RssProvenance::GnuTimeV
    }
}

fn resource_rss(report: &str) -> Option<(u64, RssProvenance)> {
    let expected = expected_rss_provenance();
    for line in report.lines() {
        let trimmed = line.trim();
        if expected == RssProvenance::BsdTimeL
            && let Some(value) = trimmed.strip_suffix("maximum resident set size")
        {
            let bytes = value.trim().parse::<u64>().ok()?;
            return (bytes > 0).then_some((bytes, RssProvenance::BsdTimeL));
        }
        if expected == RssProvenance::GnuTimeV
            && let Some((_, value)) = trimmed.split_once("Maximum resident set size (kbytes):")
        {
            let bytes = value.trim().parse::<u64>().ok()?.checked_mul(1024)?;
            return (bytes > 0).then_some((bytes, RssProvenance::GnuTimeV));
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

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    fn quick_request(limit: u64) -> super::BenchmarkInvocation {
        super::BenchmarkInvocation {
            executable: std::path::PathBuf::from("/bin/sh"),
            args: vec!["-c".to_owned(), "printf done".to_owned()],
            stdin: Vec::new(),
            current_dir: None,
            timeout: std::time::Duration::from_secs(5),
            output_limit: 1024,
            rss_limit: Some(limit),
            retain_output: false,
        }
    }

    #[test]
    #[cfg(unix)]
    fn short_lived_child_with_rss_limit_can_miss_successful_scans() {
        for (limit, expected) in [
            (u64::MAX, super::MeasuredStatus::Exited),
            (1, super::MeasuredStatus::RssLimit),
        ] {
            let outcome = super::measure_process_with_probe(&quick_request(limit), |_| Ok(None))
                .expect("positive authoritative RSS despite no live group in scans");
            assert_eq!(outcome.status, expected);
            assert!(outcome.peak_rss_bytes.unwrap() > 0);
            assert!(!outcome.process_group_rss_observed);
            assert_eq!(outcome.process_group_peak_rss_bytes, None);
            for path in [outcome.stdout_path, outcome.stderr_path]
                .into_iter()
                .flatten()
            {
                std::fs::remove_file(path).unwrap();
            }
        }
    }

    #[test]
    #[cfg(unix)]
    fn ps_failure_propagates_from_measurement() {
        fn failing_ps(group: u32) -> std::io::Result<Option<u64>> {
            let output = std::process::Command::new("/bin/sh")
                .args(["-c", "echo inspection-denied >&2; exit 1"])
                .output()?;
            super::parse_process_group_rss(group, &output)
        }
        let error = super::measure_process_with_probe(&quick_request(u64::MAX), failing_ps)
            .expect_err("actual ps failure must abort");
        assert!(matches!(
            error,
            super::MeasureError::RssSamplerUnavailable(_)
        ));
        assert!(error.to_string().contains("inspection-denied"));
    }

    #[test]
    #[cfg(unix)]
    fn background_ps_failure_aborts_and_reaps_workload() {
        use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
        static SCANS: AtomicUsize = AtomicUsize::new(0);
        static GROUP: AtomicU32 = AtomicU32::new(0);
        fn probe(group: u32) -> std::io::Result<Option<u64>> {
            GROUP.store(group, Ordering::Relaxed);
            if SCANS.fetch_add(1, Ordering::Relaxed) == 0 {
                Ok(None)
            } else {
                Err(std::io::Error::other("inspection lost"))
            }
        }
        let mut request = quick_request(u64::MAX);
        request.args = vec!["-c".to_owned(), "exec /bin/sleep 30".to_owned()];
        let start = std::time::Instant::now();
        let error = super::measure_process_with_probe(&request, probe).unwrap_err();
        assert!(matches!(
            error,
            super::MeasureError::RssSamplerUnavailable(_)
        ));
        assert!(start.elapsed() < std::time::Duration::from_secs(5));
        let group =
            nix::unistd::Pid::from_raw(i32::try_from(GROUP.load(Ordering::Relaxed)).unwrap());
        assert_eq!(
            nix::sys::signal::killpg(group, None),
            Err(nix::errno::Errno::ESRCH)
        );
    }

    #[test]
    #[cfg(unix)]
    fn successful_ps_scan_without_target_is_not_failure() {
        let output = std::process::Command::new("/bin/sh")
            .args(["-c", "printf '42 1024\\n'"])
            .output()
            .unwrap();
        assert_eq!(super::parse_process_group_rss(43, &output).unwrap(), None);
    }

    #[test]
    fn parses_gnu_time_verbose_cpu_labels() {
        let report = "User time (seconds): 1.234567\nSystem time (seconds): 0.000009\n";
        assert_eq!(super::resource_seconds(report, "user"), Some(1_234_567));
        assert_eq!(super::resource_seconds(report, "sys"), Some(9));
    }

    #[test]
    fn parses_bsd_time_l_cpu_labels_on_one_line() {
        let report = "0.15 real 0.00 user 0.00 sys\n";
        assert_eq!(super::resource_seconds(report, "user"), Some(0));
        assert_eq!(super::resource_seconds(report, "sys"), Some(0));
    }

    #[test]
    fn authoritative_rss_requires_expected_format_and_positive_value() {
        let gnu = "Maximum resident set size (kbytes): 64\n";
        let bsd = "  65536  maximum resident set size\n";
        let zero_gnu = "Maximum resident set size (kbytes): 0\n";
        let zero_bsd = "  0  maximum resident set size\n";
        let parsed = super::resource_rss(match super::expected_rss_provenance() {
            super::RssProvenance::GnuTimeV => gnu,
            super::RssProvenance::BsdTimeL => bsd,
        });
        assert!(parsed.is_some());
        assert!(
            super::resource_rss(match super::expected_rss_provenance() {
                super::RssProvenance::GnuTimeV => zero_gnu,
                super::RssProvenance::BsdTimeL => zero_bsd,
            })
            .is_none()
        );
        assert!(
            super::resource_rss(match super::expected_rss_provenance() {
                super::RssProvenance::GnuTimeV => bsd,
                super::RssProvenance::BsdTimeL => gnu,
            })
            .is_none()
        );
    }
}
