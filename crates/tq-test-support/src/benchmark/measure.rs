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
use tempfile::NamedTempFile;
use thiserror::Error;

#[cfg(any(target_os = "macos", target_os = "linux"))]
use super::native_process::{EXIT_POLL, NativeChild};
use super::report::MeasurementProtocol;

/// Identity of the collector sources and dependency lock compiled into this binary.
///
/// Calibration uses this identity to reject evidence from a different collector,
/// even when both collectors use the same platform provenance label.
#[must_use]
pub fn collector_source_sha256() -> String {
    use sha2::{Digest as _, Sha256};
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut hash = Sha256::new();
    for source in [
        include_bytes!("measure.rs").as_slice(),
        include_bytes!("native_process.rs").as_slice(),
        include_bytes!("probe.rs").as_slice(),
        include_bytes!("../bin/tq-bench-probe.rs").as_slice(),
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.lock")).as_slice(),
    ] {
        hash.update(source.len().to_le_bytes());
        hash.update(source);
    }
    hash.finalize()
        .iter()
        .flat_map(|byte| {
            [
                char::from(HEX[usize::from(byte >> 4)]),
                char::from(HEX[usize::from(byte & 15)]),
            ]
        })
        .collect()
}

const CLEANUP_TIMEOUT: Duration = Duration::from_secs(5);
const RSS_SAMPLE_INTERVAL: Duration = Duration::from_millis(25);
const PREFLIGHT_ALLOCATION_BYTES: u64 = 64 * 1024 * 1024;
const MIN_PREFLIGHT_DELTA_BYTES: u64 = PREFLIGHT_ALLOCATION_BYTES / 2;

/// Source of an authoritative peak RSS value.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RssProvenance {
    /// Native macOS `wait4` resource collection.
    DarwinWait4,
    /// Native Linux `wait4` resource collection.
    LinuxWait4,
    /// Historical GNU `/usr/bin/time -v` collection.
    GnuTimeV,
    /// Historical BSD `/usr/bin/time -l` collection.
    BsdTimeL,
}

impl RssProvenance {
    /// Stable report label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::DarwinWait4 => "darwin-wait4",
            Self::LinuxWait4 => "linux-wait4",
            Self::GnuTimeV => "gnu-time-v",
            Self::BsdTimeL => "bsd-time-l",
        }
    }
}

/// Evidence that the local host can collect the required RSS metrics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RssPreflight {
    /// Native collector used by the preflight child.
    pub provenance: RssProvenance,
}

/// Exact command measurement request.
#[derive(Clone, Debug)]
pub struct BenchmarkInvocation {
    /// Optional cancellation flag. The lifecycle terminates and reaps before returning an error.
    pub cancellation: Option<Arc<AtomicBool>>,
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
    /// Optional peak resident-memory limit enforced with a diagnostic sampler.
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
    /// Total wall duration from spawn to exit observation.
    pub wall_time_micros: u128,
    /// Time until the first stdout byte was observed, if observed while alive.
    pub first_result_micros: Option<u128>,
    /// User CPU time.
    pub user_cpu_micros: Option<u128>,
    /// System CPU time.
    pub system_cpu_micros: Option<u128>,
    /// Peak process resident bytes.
    pub peak_rss_bytes: Option<u64>,
    /// Explicit source of the authoritative peak RSS value.
    pub rss_provenance: RssProvenance,
    /// Measurement timing, input, and accounting protocol.
    pub measurement_protocol: MeasurementProtocol,
    /// Peak resident bytes observed by the optional process-group sampler.
    pub process_group_peak_rss_bytes: Option<u64>,
    /// Total stdout bytes observed, including bytes beyond capture limit.
    pub output_bytes: u64,
    /// Preserved stdout file for an unsuccessful invocation.
    pub stdout_path: Option<PathBuf>,
    /// Preserved stderr file for an unsuccessful invocation.
    pub stderr_path: Option<PathBuf>,
    /// Whether optional process-group sampling observed a positive RSS value.
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
    /// Configured resident-memory limit exceeded.
    RssLimit,
}

/// Benchmark process lifecycle failure.
#[derive(Debug, Error)]
pub enum MeasureError {
    /// Spawn, capture, or wait failure.
    #[error("benchmark process I/O failed: {0}")]
    Io(#[from] io::Error),
    /// Capture or sampler thread panicked.
    #[error("benchmark capture worker panicked")]
    CaptureWorker,
    /// The native collector did not emit usable RSS data.
    #[error("authoritative peak RSS unavailable: {0}")]
    RssUnavailable(String),
    /// The optional process-group sampler failed while enforcing a limit.
    #[error("process-group RSS inspection unavailable: {0}")]
    RssSamplerUnavailable(String),
    /// The current target has no supported native accounting implementation.
    #[error("native process accounting unsupported on this platform: {0}")]
    Unsupported(String),
    /// The caller cancelled the measurement; no campaign result may be published.
    #[error("benchmark measurement cancelled")]
    Cancelled,
    /// Post-spawn infrastructure failure with retained output and available status.
    #[error(
        "{source}; exit={exit_code:?}, signal={signal:?}, wall={wall_time_micros:?} us; stdout={stdout_path:?}, stderr={stderr_path:?}"
    )]
    Collection {
        /// Underlying measurement or cleanup error.
        #[source]
        source: Box<MeasureError>,
        /// Exit code if resource collection reached a terminal status.
        exit_code: Option<i32>,
        /// Terminating signal if available.
        signal: Option<i32>,
        /// Frozen exit-observation duration if available.
        wall_time_micros: Option<u128>,
        /// Retained stdout capture.
        stdout_path: PathBuf,
        /// Retained stderr capture.
        stderr_path: PathBuf,
    },
}

/// Verifies native RSS collection before a campaign allocates or replays a corpus.
///
/// The preflight launches the current benchmark executable in its hidden Rust
/// allocation-probe mode. The child allocates and releases 64 MiB without a
/// polling dwell. Its native wait4 high-water mark must exceed a separate
/// no-op child's peak by at least 32 MiB, demonstrating observed page touching.
///
/// # Errors
///
/// Returns an infrastructure error when native accounting is unavailable, the
/// probe cannot run, or its measured high-water mark is implausibly small.
pub fn preflight_rss(cancellation: Option<Arc<AtomicBool>>) -> Result<RssPreflight, MeasureError> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let executable = std::env::current_exe()?;
        let mut invocation = BenchmarkInvocation {
            cancellation,
            executable,
            args: vec!["--internal-rss-control".to_owned()],
            stdin: Vec::new(),
            current_dir: None,
            timeout: Duration::from_secs(10),
            output_limit: 1024,
            rss_limit: None,
            retain_output: false,
        };
        let control = measure_process(&invocation)?;
        if control.status != MeasuredStatus::Exited || control.exit_code != Some(0) {
            return Err(MeasureError::RssUnavailable(format!(
                "preflight control failed: {control:?}"
            )));
        }
        invocation.args = vec!["--internal-rss-probe".to_owned()];
        let measured = measure_process(&invocation)?;
        let expected = native_rss_provenance();
        if measured.status != MeasuredStatus::Exited || measured.exit_code != Some(0) {
            return Err(MeasureError::RssUnavailable(format!(
                "preflight child did not exit successfully (status {:?}, exit {:?})",
                measured.status, measured.exit_code
            )));
        }
        if measured.rss_provenance != expected || control.rss_provenance != expected {
            return Err(MeasureError::RssUnavailable(format!(
                "preflight used unexpected RSS provenance {}",
                measured.rss_provenance.label()
            )));
        }
        let peak = measured.peak_rss_bytes.ok_or_else(|| {
            MeasureError::RssUnavailable("preflight RSS was unavailable".to_owned())
        })?;
        let control_peak = control
            .peak_rss_bytes
            .filter(|peak| *peak > 0)
            .ok_or_else(|| {
                MeasureError::RssUnavailable("preflight control RSS was unavailable".to_owned())
            })?;
        if peak.saturating_sub(control_peak) < MIN_PREFLIGHT_DELTA_BYTES {
            return Err(MeasureError::RssUnavailable(format!(
                "preflight allocation did not increase RSS enough: control={:?}, allocation={peak} bytes for {PREFLIGHT_ALLOCATION_BYTES}-byte allocation",
                control.peak_rss_bytes,
            )));
        }
        Ok(RssPreflight {
            provenance: expected,
        })
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = cancellation;
        Err(MeasureError::Unsupported(
            "native wait4 accounting is implemented only on macOS and Linux".to_owned(),
        ))
    }
}

/// Measures one fresh process invocation with direct native accounting.
///
/// Capture files and a seekable stdin file are prepared before the monotonic
/// timer starts. The timer ends when the native owner observes the child exit;
/// sampler joins, resource conversion, and output persistence happen after
/// that timestamp. Without an RSS limit, no `ps` process is started.
///
/// # Errors
///
/// Returns spawn, native wait, capture, or authoritative RSS failures. Normal
/// nonzero, signal, timeout, output-limit, and RSS-limit outcomes remain
/// successful measurements when their resource data is collected.
pub fn measure_process(invocation: &BenchmarkInvocation) -> Result<MeasuredOutcome, MeasureError> {
    measure_process_with_sampling(invocation, true)
}

/// Measures one fresh process without starting the optional process-group RSS
/// sampler.
///
/// The native `wait4` high-water RSS value is still checked against
/// `invocation.rss_limit` after the child exits, but this path does not enforce
/// that limit while the child is running. Callers use it only for timing
/// repetitions after a separate instrumented run has checked the limit.
///
/// # Errors
///
/// Returns the same spawn, native wait, capture, and authoritative RSS errors
/// as [`measure_process`].
pub fn measure_process_uninstrumented(
    invocation: &BenchmarkInvocation,
) -> Result<MeasuredOutcome, MeasureError> {
    measure_process_with_sampling(invocation, false)
}

fn measure_process_with_sampling(
    invocation: &BenchmarkInvocation,
    sample_process_group: bool,
) -> Result<MeasuredOutcome, MeasureError> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        measure_process_native(invocation, sample_process_group)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = invocation;
        let _ = sample_process_group;
        Err(MeasureError::Unsupported(
            "native wait4 accounting is implemented only on macOS and Linux".to_owned(),
        ))
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn measure_process_native(
    invocation: &BenchmarkInvocation,
    sample_process_group: bool,
) -> Result<MeasuredOutcome, MeasureError> {
    measure_with_hooks(invocation, sample_process_group, || {}, || {})
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[allow(
    clippy::too_many_lines,
    reason = "keep owned child and capture cleanup in one lexical scope"
)]
fn measure_with_hooks(
    invocation: &BenchmarkInvocation,
    sample_process_group: bool,
    before_spawn: impl FnOnce(),
    after_observation: impl FnOnce(),
) -> Result<MeasuredOutcome, MeasureError> {
    if invocation
        .cancellation
        .as_ref()
        .is_some_and(|flag| flag.load(Ordering::Acquire))
    {
        return Err(MeasureError::Cancelled);
    }
    let mut stdin_file = NamedTempFile::new()?;
    stdin_file.write_all(&invocation.stdin)?;
    stdin_file.flush()?;
    let stdout_file = NamedTempFile::new()?;
    let stderr_file = NamedTempFile::new()?;

    let mut command = Command::new(&invocation.executable);
    command
        .args(&invocation.args)
        .stdin(Stdio::from(stdin_file.reopen()?))
        .stdout(Stdio::from(stdout_file.reopen()?))
        .stderr(Stdio::from(stderr_file.reopen()?));
    if let Some(directory) = &invocation.current_dir {
        command.current_dir(directory);
    }
    configure_process_group(&mut command);
    before_spawn();

    // Keep this immediately adjacent to spawn. All command and capture setup
    // above is intentionally outside the measured interval.
    let started = Instant::now();
    let child = command.spawn()?;
    let mut diagnostic_exit = None;
    let mut diagnostic_signal = None;
    let mut diagnostic_wall = None;
    let measured = (|| {
        let mut owner = NativeChild::new(child);
        let process_id = owner.id();
        let mut sampler = if sample_process_group {
            invocation
                .rss_limit
                .map(|_| spawn_rss_sampler(process_id))
                .transpose()?
        } else {
            None
        };
        let sampler_failed = sampler.as_ref().map(|sampler| Arc::clone(&sampler.failed));

        let mut first_result_micros = None;
        let mut forced_status = None;
        let mut cancelled = false;
        let mut missing_live_observation = false;
        let observed_micros = loop {
            if owner.observe_exit()? {
                break started.elapsed().as_micros();
            }
            if invocation
                .cancellation
                .as_ref()
                .is_some_and(|flag| flag.load(Ordering::Acquire))
            {
                cancelled = true;
                terminate_until_observed(&mut owner)?;
                break started.elapsed().as_micros();
            }

            if sampler_failed
                .as_ref()
                .is_some_and(|failed| failed.load(Ordering::Acquire))
            {
                terminate_until_observed(&mut owner)?;
                break started.elapsed().as_micros();
            }

            if sampler
                .as_ref()
                .is_some_and(|state| state.missing.load(Ordering::Acquire))
            {
                // A short child can exit before inspection. Only the owner may
                // confirm that race; unavailable inspection of a live child is
                // an enforcement failure, not a zero-memory observation.
                if !owner.observe_exit()? {
                    missing_live_observation = true;
                    terminate_until_observed(&mut owner)?;
                }
                break started.elapsed().as_micros();
            }

            if first_result_micros.is_none() && stdout_file.as_file().metadata()?.len() > 0 {
                first_result_micros = Some(started.elapsed().as_micros());
            }

            if forced_status.is_none() {
                let output_bytes = stdout_file.as_file().metadata()?.len();
                if output_bytes > invocation.output_limit {
                    forced_status = Some(MeasuredStatus::OutputLimit);
                } else if sample_process_group
                    && invocation.rss_limit.is_some_and(|limit| {
                        sampler
                            .as_ref()
                            .is_some_and(|state| state.peak.load(Ordering::Relaxed) > limit)
                    })
                {
                    forced_status = Some(MeasuredStatus::RssLimit);
                } else if started.elapsed() >= invocation.timeout {
                    forced_status = Some(MeasuredStatus::Timeout);
                }
                if forced_status.is_some() {
                    terminate_until_observed(&mut owner)?;
                    break started.elapsed().as_micros();
                }
            }
            thread::sleep(EXIT_POLL);
        };
        diagnostic_wall = Some(observed_micros);
        after_observation();

        let sampler_result = sampler.as_mut().map(RssSampler::finish).transpose();
        (diagnostic_exit, diagnostic_signal) = owner.observed_status();
        let resources = owner.finish()?;
        diagnostic_exit = resources.status.code();
        diagnostic_signal = exit_signal(resources.status);
        if cancelled {
            return Err(MeasureError::Cancelled);
        }
        if missing_live_observation {
            return Err(MeasureError::RssSamplerUnavailable(
                "ps returned no positive process-group RSS for a live child".to_owned(),
            ));
        }
        let process_group_peak_rss_bytes = sampler_result?.flatten();
        let process_group_rss_observed = process_group_peak_rss_bytes.is_some();

        let output_bytes = stdout_file.as_file().metadata()?.len();
        let signal = exit_signal(resources.status);
        let status = if output_bytes > invocation.output_limit {
            MeasuredStatus::OutputLimit
        } else if invocation
            .rss_limit
            .is_some_and(|limit| resources.rusage.maxrss > limit)
        {
            MeasuredStatus::RssLimit
        } else if let Some(forced) = forced_status {
            forced
        } else if signal.is_some() {
            MeasuredStatus::Signaled
        } else {
            MeasuredStatus::Exited
        };
        let peak_rss_bytes = validate_resources(
            resources.rusage.maxrss,
            resources.rusage.utime,
            resources.rusage.stime,
            native_rss_provenance(),
        )?;
        Ok(MeasuredOutcome {
            status,
            exit_code: resources.status.code(),
            signal,
            wall_time_micros: observed_micros,
            first_result_micros,
            user_cpu_micros: Some(resources.rusage.utime.as_micros()),
            system_cpu_micros: Some(resources.rusage.stime.as_micros()),
            peak_rss_bytes: Some(peak_rss_bytes),
            rss_provenance: native_rss_provenance(),
            measurement_protocol: measurement_protocol(
                sample_process_group && invocation.rss_limit.is_some(),
            ),
            process_group_peak_rss_bytes,
            output_bytes,
            stdout_path: None,
            stderr_path: None,
            process_group_rss_observed,
        })
    })();
    match measured {
        Ok(mut outcome) => {
            if invocation.retain_output
                || outcome.status != MeasuredStatus::Exited
                || outcome.exit_code != Some(0)
            {
                outcome.stdout_path = Some(stdout_file.keep().map_err(|error| error.error)?.1);
                outcome.stderr_path = Some(stderr_file.keep().map_err(|error| error.error)?.1);
            }
            Ok(outcome)
        }
        Err(error) => Err(MeasureError::Collection {
            source: Box::new(error),
            exit_code: diagnostic_exit,
            signal: diagnostic_signal,
            wall_time_micros: diagnostic_wall,
            stdout_path: stdout_file.keep().map_err(|error| error.error)?.1,
            stderr_path: stderr_file.keep().map_err(|error| error.error)?.1,
        }),
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
struct RssSampler {
    stop: Arc<AtomicBool>,
    peak: Arc<AtomicU64>,
    failed: Arc<AtomicBool>,
    missing: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<io::Result<()>>>,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
impl RssSampler {
    fn finish(&mut self) -> Result<Option<u64>, MeasureError> {
        self.stop.store(true, Ordering::Release);
        if let Some(handle) = self.handle.take() {
            handle.thread().unpark();
            handle
                .join()
                .map_err(|_| MeasureError::CaptureWorker)?
                .map_err(|error| MeasureError::RssSamplerUnavailable(error.to_string()))?;
        }
        let peak = self.peak.load(Ordering::Acquire);
        Ok((peak > 0).then_some(peak))
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
impl Drop for RssSampler {
    fn drop(&mut self) {
        if self.handle.is_some()
            && let Err(error) = self.finish()
        {
            eprintln!("tq-bench: RSS sampler cleanup failed: {error}");
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn spawn_rss_sampler(process_group: u32) -> io::Result<RssSampler> {
    spawn_rss_sampler_with_probe(process_group, process_group_rss)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn spawn_rss_sampler_with_probe(
    process_group: u32,
    probe: fn(u32) -> io::Result<Option<u64>>,
) -> io::Result<RssSampler> {
    let stop = Arc::new(AtomicBool::new(false));
    let peak = Arc::new(AtomicU64::new(0));
    let failed = Arc::new(AtomicBool::new(false));
    let missing = Arc::new(AtomicBool::new(false));
    let thread_stop = Arc::clone(&stop);
    let thread_peak = Arc::clone(&peak);
    let thread_failed = Arc::clone(&failed);
    let thread_missing = Arc::clone(&missing);
    let handle = thread::Builder::new()
        .name("benchmark-rss".to_owned())
        .spawn(move || {
            // Always attempt one scan, even if the target exits before this worker
            // is scheduled. A broken requested inspection must not pass silently.
            loop {
                match probe(process_group) {
                    Ok(Some(bytes)) => {
                        thread_peak.fetch_max(bytes, Ordering::Release);
                        thread_missing.store(false, Ordering::Release);
                    }
                    Ok(None) => thread_missing.store(true, Ordering::Release),
                    Err(error) => {
                        thread_failed.store(true, Ordering::Release);
                        return Err(error);
                    }
                }
                if thread_stop.load(Ordering::Acquire) {
                    return Ok(());
                }
                thread::park_timeout(RSS_SAMPLE_INTERVAL);
                if thread_stop.load(Ordering::Acquire) {
                    return Ok(());
                }
            }
        })?;
    Ok(RssSampler {
        stop,
        peak,
        failed,
        missing,
        handle: Some(handle),
    })
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn terminate_until_observed(owner: &mut NativeChild) -> io::Result<()> {
    owner.terminate()?;
    let deadline = Instant::now() + CLEANUP_TIMEOUT;
    while !owner.observe_exit()? {
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "child did not exit after process-group termination",
            ));
        }
        thread::sleep(EXIT_POLL);
    }
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn process_group_rss(process_group: u32) -> io::Result<Option<u64>> {
    let output = inspect_processes()?;
    parse_process_group_rss(process_group, &output)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn inspect_processes() -> io::Result<std::process::Output> {
    use wait_timeout::ChildExt as _;
    const CAPTURE_LIMIT: u64 = 4 * 1024 * 1024;
    let stdout = NamedTempFile::new()?;
    let stderr = NamedTempFile::new()?;
    let mut child = Command::new("ps")
        .args(["-axo", "pgid=,rss="])
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout.reopen()?))
        .stderr(Stdio::from(stderr.reopen()?))
        .spawn()?;
    let waited = (|| {
        let deadline = Instant::now() + Duration::from_millis(250);
        loop {
            if stdout.as_file().metadata()?.len() > CAPTURE_LIMIT
                || stderr.as_file().metadata()?.len() > CAPTURE_LIMIT
            {
                return Err(io::Error::other("ps capture limit exceeded"));
            }
            if let Some(status) = child.try_wait()? {
                return Ok(status);
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "ps inspection timed out",
                ));
            }
            thread::sleep(Duration::from_millis(1));
        }
    })();
    let status = match waited {
        Ok(status) => status,
        Err(error) => {
            child.kill()?;
            // This is the diagnostic ps child only, never the measured tool.
            if child.wait_timeout(Duration::from_secs(1))?.is_none() {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "ps did not exit after termination",
                ));
            }
            return Err(error);
        }
    };
    // Recheck after exit before allocating the bounded captured data.
    if stdout.as_file().metadata()?.len() > CAPTURE_LIMIT
        || stderr.as_file().metadata()?.len() > CAPTURE_LIMIT
    {
        return Err(io::Error::other("ps capture limit exceeded"));
    }
    Ok(std::process::Output {
        status,
        stdout: std::fs::read(stdout.path())?,
        stderr: std::fs::read(stderr.path())?,
    })
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
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
    let group = i32::try_from(process_group)
        .map_err(|_| io::Error::other("process-group identifier overflow"))?;
    let mut kibibytes = 0_u64;
    let text = std::str::from_utf8(&output.stdout).map_err(io::Error::other)?;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let mut fields = line.split_whitespace();
        let Some(candidate_group) = fields.next().and_then(|value| value.parse::<i32>().ok())
        else {
            return Err(io::Error::other("invalid ps process-group output"));
        };
        let Some(rss) = fields.next().and_then(|value| value.parse::<u64>().ok()) else {
            return Err(io::Error::other("invalid ps RSS output"));
        };
        if candidate_group == group {
            kibibytes = kibibytes
                .checked_add(rss)
                .ok_or_else(|| io::Error::other("process-group RSS overflow"))?;
        }
    }
    kibibytes
        .checked_mul(1024)
        .ok_or_else(|| io::Error::other("process-group RSS byte conversion overflow"))
        .map(|bytes| (bytes > 0).then_some(bytes))
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn measurement_protocol(has_rss_sampler: bool) -> MeasurementProtocol {
    let rss_method = if has_rss_sampler {
        format!(
            "process-group RSS sampler every {} micros",
            RSS_SAMPLE_INTERVAL.as_micros()
        )
    } else {
        "native wait4 RSS checked at exit; no in-flight RSS enforcement".to_owned()
    };
    MeasurementProtocol {
        timing_method: format!(
            "{} direct spawn to waitid-WNOWAIT exit observation; first-byte file metadata poll {} micros; {rss_method}; wait4-0.2.0 valid-OS-counter assumption",
            native_rss_provenance().label(),
            EXIT_POLL.as_micros()
        ),
        input_delivery: "prepared-seekable-stdin-file".to_owned(),
        rss_scope: "wait4-child-including-waited-descendants-and-threads".to_owned(),
        exit_poll_interval_micros: u64::try_from(EXIT_POLL.as_micros())
            .expect("fixed poll interval fits u64"),
        rss_poll_interval_micros: has_rss_sampler.then_some(
            u64::try_from(RSS_SAMPLE_INTERVAL.as_micros()).expect("fixed sample interval fits u64"),
        ),
        validated_accuracy_micros: None,
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn native_rss_provenance() -> RssProvenance {
    #[cfg(target_os = "macos")]
    {
        RssProvenance::DarwinWait4
    }
    #[cfg(target_os = "linux")]
    {
        RssProvenance::LinuxWait4
    }
}

fn validate_resources(
    rss: u64,
    user: Duration,
    system: Duration,
    source: RssProvenance,
) -> Result<u64, MeasureError> {
    // wait4 already normalized units. These checks reject detectable bad
    // results; they cannot recover raw fields lost to upstream conversion.
    // The approved contract assumes valid nonnegative kernel counters.
    if rss == 0
        || i64::try_from(rss).is_err()
        || (source == RssProvenance::LinuxWait4 && !rss.is_multiple_of(1024))
        || i64::try_from(user.as_micros()).is_err()
        || i64::try_from(system.as_micros()).is_err()
    {
        return Err(MeasureError::RssUnavailable(format!(
            "invalid {} RSS/CPU counters",
            source.label()
        )));
    }
    Ok(rss)
}

#[cfg(unix)]
fn configure_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt as _;
    command.process_group(0);
}

#[cfg(not(unix))]
fn configure_process_group(_command: &mut Command) {}

#[cfg(unix)]
fn exit_signal(status: std::process::ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt as _;
    status.signal()
}

#[cfg(not(unix))]
fn exit_signal(_status: std::process::ExitStatus) -> Option<i32> {
    None
}

#[cfg(test)]
mod tests {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn setup_and_cleanup_delays_do_not_extend_measured_duration() {
        let invocation = super::BenchmarkInvocation {
            cancellation: None,
            executable: "/bin/sleep".into(),
            args: vec!["0.01".to_owned()],
            stdin: Vec::new(),
            current_dir: None,
            timeout: super::Duration::from_secs(2),
            output_limit: 1024,
            rss_limit: None,
            retain_output: false,
        };
        let slow = || std::thread::sleep(super::Duration::from_millis(150));
        let started = std::time::Instant::now();
        let result = super::measure_with_hooks(&invocation, false, slow, slow).unwrap();
        assert!(started.elapsed() >= super::Duration::from_millis(300));
        assert!(
            result.wall_time_micros < 150_000,
            "{} us",
            result.wall_time_micros
        );
        assert!(result.wall_time_micros >= 10_000);
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn requested_sampler_failure_is_not_swallowed_on_immediate_exit() {
        let mut sampler = super::spawn_rss_sampler_with_probe(1, |_| {
            Err(std::io::Error::other("inspection denied"))
        })
        .unwrap();
        let error = sampler.finish().unwrap_err();
        assert!(error.to_string().contains("inspection denied"));
        assert!(sampler.handle.is_none());
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn dropping_sampler_stops_and_joins_its_worker() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static SCANS: AtomicUsize = AtomicUsize::new(0);
        let start = std::time::Instant::now();
        let sampler = super::spawn_rss_sampler_with_probe(1, |_| {
            SCANS.fetch_add(1, Ordering::Relaxed);
            Ok(Some(4096))
        })
        .unwrap();
        drop(sampler);
        assert!(start.elapsed() < std::time::Duration::from_secs(1));
        let scans = SCANS.load(Ordering::Relaxed);
        assert!(scans >= 1);
        std::thread::sleep(super::RSS_SAMPLE_INTERVAL * 2);
        assert_eq!(SCANS.load(Ordering::Relaxed), scans);
    }
    #[test]
    fn rejects_detectable_invalid_native_counters() {
        use super::{Duration, RssProvenance, validate_resources};
        for rss in [0, u64::MAX, 1025] {
            assert!(
                validate_resources(
                    rss,
                    Duration::ZERO,
                    Duration::ZERO,
                    RssProvenance::LinuxWait4
                )
                .is_err()
            );
        }
        assert!(
            validate_resources(
                4096,
                Duration::from_micros(u64::MAX),
                Duration::ZERO,
                RssProvenance::LinuxWait4
            )
            .is_err()
        );
        assert_eq!(
            validate_resources(
                4096,
                Duration::ZERO,
                Duration::ZERO,
                RssProvenance::LinuxWait4
            )
            .unwrap(),
            4096
        );
        assert_eq!(
            validate_resources(
                4096,
                Duration::ZERO,
                Duration::ZERO,
                RssProvenance::DarwinWait4
            )
            .unwrap(),
            4096
        );
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn process_group_rss_parser_rejects_overflow() {
        let output = std::process::Command::new("/bin/sh")
            .args(["-c", "printf '42 18446744073709551615\\n'"])
            .output()
            .expect("run parser fixture");
        assert!(super::parse_process_group_rss(42, &output).is_err());
    }

    #[test]
    fn historical_labels_remain_distinct() {
        assert_ne!(
            super::RssProvenance::DarwinWait4.label(),
            super::RssProvenance::BsdTimeL.label()
        );
        assert_ne!(
            super::RssProvenance::LinuxWait4.label(),
            super::RssProvenance::GnuTimeV.label()
        );
    }
}
