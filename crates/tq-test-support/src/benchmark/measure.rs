//! Per-process wall, latency, CPU, memory, and byte measurement.

use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Write},
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
use super::native_process::{EXIT_POLL, NativeChild, poll_delay};
use super::report::MeasurementProtocol;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use nix::unistd::Pid;

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
        include_bytes!("worker.rs").as_slice(),
        include_bytes!("../bin/tq-bench-probe.rs").as_slice(),
        include_bytes!("../bin/tq-bench-worker.rs").as_slice(),
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
const CAPTURE_POLL: Duration = Duration::from_millis(1);
const CAPTURE_DRAIN_TIMEOUT: Duration = Duration::from_millis(250);
const CAPTURE_READ_CHUNK: usize = 16 * 1024;
const UNSET_FIRST_RESULT: u64 = u64::MAX;
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
    /// Maximum bytes retained from each output stream before the harness stops the process.
    pub output_limit: u64,
    /// Optional peak resident-memory limit enforced with a diagnostic sampler.
    pub rss_limit: Option<u64>,
    /// Retain output files after a successful invocation for semantic checks.
    pub retain_output: bool,
}

pub(crate) struct PreparedCaptureFiles {
    stdin: NamedTempFile,
    stdout: NamedTempFile,
    stderr: NamedTempFile,
    reply: NamedTempFile,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[derive(Clone, Copy)]
enum CaptureStream {
    Stdout,
    Stderr,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[derive(Clone)]
struct CaptureStreamState {
    observed_bytes: Arc<AtomicU64>,
    limit_reached: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
struct CaptureWorker {
    state: CaptureStreamState,
    handle: Option<thread::JoinHandle<io::Result<()>>>,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
struct CaptureWorkers {
    stop: Arc<AtomicBool>,
    first_stdout_micros: Arc<AtomicU64>,
    stdout: CaptureWorker,
    stderr: CaptureWorker,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
struct CaptureResult {
    stdout_bytes: u64,
    stderr_bytes: u64,
    stdout_limited: bool,
    stderr_limited: bool,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
impl CaptureWorkers {
    fn spawn(
        stdout: std::process::ChildStdout,
        stderr: std::process::ChildStderr,
        stdout_file: File,
        stderr_file: File,
        output_limit: u64,
        started: Instant,
    ) -> io::Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let first_stdout_micros = Arc::new(AtomicU64::new(UNSET_FIRST_RESULT));
        let stdout_state = capture_state();
        let stderr_state = capture_state();
        let mut stdout_worker = CaptureWorker::spawn(
            stdout,
            stdout_file,
            CaptureStream::Stdout,
            Arc::clone(&stop),
            Arc::clone(&first_stdout_micros),
            stdout_state,
            output_limit,
            started,
        )?;
        let stderr_worker = match CaptureWorker::spawn(
            stderr,
            stderr_file,
            CaptureStream::Stderr,
            Arc::clone(&stop),
            Arc::clone(&first_stdout_micros),
            stderr_state,
            output_limit,
            started,
        ) {
            Ok(worker) => worker,
            Err(error) => {
                stop.store(true, Ordering::Release);
                if let Some(handle) = stdout_worker.handle.take() {
                    handle.thread().unpark();
                    let _ = handle.join();
                }
                return Err(error);
            }
        };
        Ok(Self {
            stop,
            first_stdout_micros,
            stdout: stdout_worker,
            stderr: stderr_worker,
        })
    }

    fn output_limit_reached(&self) -> bool {
        self.stdout.state.limit_reached.load(Ordering::Acquire)
            || self.stderr.state.limit_reached.load(Ordering::Acquire)
    }

    fn capture_failed(&self) -> bool {
        self.stdout.state.failed.load(Ordering::Acquire)
            || self.stderr.state.failed.load(Ordering::Acquire)
    }

    fn first_stdout_micros(&self) -> Option<u128> {
        let micros = self.first_stdout_micros.load(Ordering::Acquire);
        (micros != UNSET_FIRST_RESULT).then_some(u128::from(micros))
    }

    fn finish(&mut self, wait_for_eof: bool) -> Result<CaptureResult, MeasureError> {
        let mut drained = !wait_for_eof;
        let mut drain_timed_out = false;
        if wait_for_eof {
            let deadline = Instant::now() + CAPTURE_DRAIN_TIMEOUT;
            loop {
                if self.stdout.state.finished.load(Ordering::Acquire)
                    && self.stderr.state.finished.load(Ordering::Acquire)
                {
                    drained = true;
                    break;
                }
                // A limit or worker error is already a terminal capture
                // observation. Stop immediately instead of waiting for EOF
                // that a descendant may prevent indefinitely.
                if self.output_limit_reached() || self.capture_failed() {
                    break;
                }
                if Instant::now() >= deadline {
                    drain_timed_out = true;
                    break;
                }
                thread::park_timeout(CAPTURE_POLL);
            }
        }
        self.stop.store(true, Ordering::Release);
        if let Some(handle) = &self.stdout.handle {
            handle.thread().unpark();
        }
        if let Some(handle) = &self.stderr.handle {
            handle.thread().unpark();
        }
        self.join_workers()?;
        if drain_timed_out && !drained {
            return Err(MeasureError::CaptureDrainTimeout);
        }
        Ok(CaptureResult {
            stdout_bytes: self.stdout.state.observed_bytes.load(Ordering::Acquire),
            stderr_bytes: self.stderr.state.observed_bytes.load(Ordering::Acquire),
            stdout_limited: self.stdout.state.limit_reached.load(Ordering::Acquire),
            stderr_limited: self.stderr.state.limit_reached.load(Ordering::Acquire),
        })
    }

    fn join_workers(&mut self) -> Result<(), MeasureError> {
        let mut first_error = None;
        for worker in [&mut self.stdout, &mut self.stderr] {
            if let Some(handle) = worker.handle.take() {
                match handle.join() {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        first_error.get_or_insert(MeasureError::Io(error));
                    }
                    Err(_) => {
                        first_error.get_or_insert(MeasureError::CaptureWorker);
                    }
                }
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
impl Drop for CaptureWorkers {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(handle) = &self.stdout.handle {
            handle.thread().unpark();
        }
        if let Some(handle) = &self.stderr.handle {
            handle.thread().unpark();
        }
        if let Err(error) = self.join_workers() {
            eprintln!("tq-bench: output capture cleanup failed: {error}");
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
impl CaptureWorker {
    #[allow(clippy::too_many_arguments)]
    fn spawn(
        reader: impl Read + std::os::fd::AsFd + Send + 'static,
        file: File,
        stream: CaptureStream,
        stop: Arc<AtomicBool>,
        first_stdout_micros: Arc<AtomicU64>,
        state: CaptureStreamState,
        output_limit: u64,
        started: Instant,
    ) -> io::Result<Self> {
        let thread_state = CaptureStreamState {
            observed_bytes: Arc::clone(&state.observed_bytes),
            limit_reached: Arc::clone(&state.limit_reached),
            failed: Arc::clone(&state.failed),
            finished: Arc::clone(&state.finished),
        };
        let capture_state = state.clone();
        let handle = thread::Builder::new()
            .name(match stream {
                CaptureStream::Stdout => "benchmark-stdout-capture".to_owned(),
                CaptureStream::Stderr => "benchmark-stderr-capture".to_owned(),
            })
            .spawn(move || {
                let result = capture_stream(
                    reader,
                    file,
                    stream,
                    &stop,
                    &first_stdout_micros,
                    &capture_state,
                    output_limit,
                    started,
                );
                if result.is_err() {
                    thread_state.failed.store(true, Ordering::Release);
                }
                thread_state.finished.store(true, Ordering::Release);
                result
            })?;
        Ok(Self {
            state,
            handle: Some(handle),
        })
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn capture_state() -> CaptureStreamState {
    CaptureStreamState {
        observed_bytes: Arc::new(AtomicU64::new(0)),
        limit_reached: Arc::new(AtomicBool::new(false)),
        failed: Arc::new(AtomicBool::new(false)),
        finished: Arc::new(AtomicBool::new(false)),
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[allow(clippy::too_many_arguments)]
fn capture_stream(
    mut reader: impl Read + std::os::fd::AsFd,
    mut file: File,
    stream: CaptureStream,
    stop: &Arc<AtomicBool>,
    first_stdout_micros: &Arc<AtomicU64>,
    state: &CaptureStreamState,
    output_limit: u64,
    started: Instant,
) -> io::Result<()> {
    use rustix::fs::{OFlags, fcntl_getfl, fcntl_setfl};

    let flags = fcntl_getfl(&reader)?;
    fcntl_setfl(&reader, flags | OFlags::NONBLOCK)?;
    let mut buffer = [0_u8; CAPTURE_READ_CHUNK];
    let mut retained_bytes = 0_u64;
    (|| {
        loop {
            if stop.load(Ordering::Acquire) {
                break;
            }
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    add_observed_bytes(&state.observed_bytes, read);
                    if matches!(stream, CaptureStream::Stdout) {
                        record_first_stdout(first_stdout_micros, started);
                    }
                    let remaining = output_limit.saturating_sub(retained_bytes);
                    let retained = remaining.min(u64::try_from(read).unwrap_or(u64::MAX));
                    if retained > 0 {
                        let retained =
                            usize::try_from(retained).expect("capture chunk length fits usize");
                        file.write_all(&buffer[..retained])?;
                        retained_bytes = retained_bytes
                            .saturating_add(u64::try_from(retained).unwrap_or(u64::MAX));
                    }
                    if u64::try_from(read).unwrap_or(u64::MAX) > remaining {
                        state.limit_reached.store(true, Ordering::Release);
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    thread::park_timeout(CAPTURE_POLL);
                }
                Err(error) => return Err(error),
            }
        }
        file.flush()
    })()
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[allow(deprecated, reason = "fetch_update is available on the supported MSRV")]
fn add_observed_bytes(observed: &AtomicU64, read: usize) {
    let read = u64::try_from(read).unwrap_or(u64::MAX);
    let _ = observed.fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
        Some(current.saturating_add(read))
    });
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn record_first_stdout(first_stdout_micros: &AtomicU64, started: Instant) {
    let micros = started
        .elapsed()
        .as_micros()
        .try_into()
        .unwrap_or(u64::MAX - 1);
    let _ = first_stdout_micros.compare_exchange(
        UNSET_FIRST_RESULT,
        micros,
        Ordering::AcqRel,
        Ordering::Acquire,
    );
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(crate) struct PreparedCapturePaths {
    pub(crate) stdin: PathBuf,
    pub(crate) stdout: PathBuf,
    pub(crate) stderr: PathBuf,
    pub(crate) reply: PathBuf,
}

impl PreparedCaptureFiles {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub(crate) fn paths(&self) -> PreparedCapturePaths {
        PreparedCapturePaths {
            stdin: self.stdin.path().to_owned(),
            stdout: self.stdout.path().to_owned(),
            stderr: self.stderr.path().to_owned(),
            reply: self.reply.path().to_owned(),
        }
    }

    pub(crate) fn keep_output(self) -> io::Result<(PathBuf, PathBuf)> {
        let stdout = self.stdout.keep().map_err(|error| error.error)?.1;
        let stderr = self.stderr.keep().map_err(|error| error.error)?.1;
        Ok((stdout, stderr))
    }
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
    /// Time until the first stdout byte was observed, or completion when
    /// non-empty output was first observable only after exit.
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
    /// Total stderr bytes observed, including bytes beyond capture limit.
    #[serde(default)]
    pub stderr_bytes: u64,
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
    /// A descendant retained an output descriptor past the bounded drain window.
    #[error("benchmark output capture did not drain before cleanup deadline")]
    CaptureDrainTimeout,
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

/// Measures one fresh process invocation through the isolated Rust worker.
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
    measure_process_worker_with_sampling(invocation, true)
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
    measure_process_worker_with_sampling(invocation, false)
}

/// Measures one invocation through the isolated Rust worker.
///
/// This explicit seam is also the default measurement path. Input and capture payloads stay in
/// coordinator-owned files; only bounded command metadata and file paths cross
/// the worker channel.
///
/// # Errors
///
/// Returns a worker startup, protocol, target lifecycle, capture, or native
/// accounting failure. Worker failures never fall back to direct measurement.
pub fn measure_process_worker(
    invocation: &BenchmarkInvocation,
) -> Result<MeasuredOutcome, MeasureError> {
    measure_process_worker_with_sampling(invocation, true)
}

/// Measures one invocation through the isolated Rust worker without starting
/// the optional process-group RSS sampler.
///
/// The worker still owns target launch, native waiting, capture, and worker
/// identity attachment. Callers use this path for timing repetitions after a
/// separate worker-backed enforcement repetition.
///
/// # Errors
///
/// Returns a worker startup, protocol, target lifecycle, capture, or native
/// accounting failure.
pub fn measure_process_worker_uninstrumented(
    invocation: &BenchmarkInvocation,
) -> Result<MeasuredOutcome, MeasureError> {
    measure_process_worker_with_sampling(invocation, false)
}

fn measure_process_worker_with_sampling(
    invocation: &BenchmarkInvocation,
    sample_process_group: bool,
) -> Result<MeasuredOutcome, MeasureError> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        super::worker::measure_process(invocation, sample_process_group)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = invocation;
        let _ = sample_process_group;
        Err(MeasureError::Unsupported(
            "isolated native worker is implemented only on macOS and Linux".to_owned(),
        ))
    }
}

pub(crate) fn prepare_capture_files(
    invocation: &BenchmarkInvocation,
) -> io::Result<PreparedCaptureFiles> {
    let mut stdin = NamedTempFile::new()?;
    stdin.write_all(&invocation.stdin)?;
    stdin.flush()?;
    Ok(PreparedCaptureFiles {
        stdin,
        stdout: NamedTempFile::new()?,
        stderr: NamedTempFile::new()?,
        reply: NamedTempFile::new()?,
    })
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[cfg(test)]
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
    let captures = prepare_capture_files(invocation)?;
    let paths = captures.paths();
    let measured = measure_prepared_impl(
        invocation,
        sample_process_group,
        None,
        &paths,
        before_spawn,
        after_observation,
        false,
    );
    match measured {
        Ok(mut outcome) => {
            if invocation.retain_output
                || outcome.status != MeasuredStatus::Exited
                || outcome.exit_code != Some(0)
            {
                let (stdout, stderr) = captures.keep_output()?;
                outcome.stdout_path = Some(stdout);
                outcome.stderr_path = Some(stderr);
            }
            Ok(outcome)
        }
        Err(mut error) => {
            let (stdout, stderr) = captures.keep_output()?;
            if let MeasureError::Collection {
                stdout_path,
                stderr_path,
                ..
            } = &mut error
            {
                *stdout_path = stdout;
                *stderr_path = stderr;
            }
            Err(error)
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[allow(
    clippy::too_many_lines,
    reason = "keep prepared target lifecycle and diagnostic cleanup together"
)]
pub(crate) fn measure_prepared(
    invocation: &BenchmarkInvocation,
    sample_process_group: bool,
    shared_process_group: Option<Pid>,
    paths: &PreparedCapturePaths,
    before_spawn: impl FnOnce(),
    after_observation: impl FnOnce(),
) -> Result<MeasuredOutcome, MeasureError> {
    measure_prepared_impl(
        invocation,
        sample_process_group,
        shared_process_group,
        paths,
        before_spawn,
        after_observation,
        true,
    )
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[allow(
    clippy::too_many_lines,
    reason = "keep prepared target lifecycle and diagnostic cleanup together"
)]
fn measure_prepared_impl(
    invocation: &BenchmarkInvocation,
    sample_process_group: bool,
    shared_process_group: Option<Pid>,
    paths: &PreparedCapturePaths,
    before_spawn: impl FnOnce(),
    after_observation: impl FnOnce(),
    observe_first_result: bool,
) -> Result<MeasuredOutcome, MeasureError> {
    if invocation
        .cancellation
        .as_ref()
        .is_some_and(|flag| flag.load(Ordering::Acquire))
    {
        return Err(MeasureError::Cancelled);
    }
    let stdin_file = File::open(&paths.stdin)?;
    let stdout_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&paths.stdout)?;
    let stderr_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&paths.stderr)?;

    let mut command = Command::new(&invocation.executable);
    command
        .args(&invocation.args)
        .stdin(Stdio::from(stdin_file))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(directory) = &invocation.current_dir {
        command.current_dir(directory);
    }
    configure_process_group(&mut command, shared_process_group);
    before_spawn();

    // Keep this immediately adjacent to spawn. All command and capture setup
    // above is intentionally outside the measured interval.
    let started = Instant::now();
    let mut child = command.spawn()?;
    let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take()) else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(MeasureError::Io(io::Error::other(
            "piped benchmark output was not created",
        )));
    };
    let mut diagnostic_exit = None;
    let mut diagnostic_signal = None;
    let mut diagnostic_wall = None;
    let measured = (|| {
        let mut owner = match shared_process_group {
            Some(group) => NativeChild::new_in_shared_group(child, group),
            None => NativeChild::new(child),
        };
        let mut captures = CaptureWorkers::spawn(
            stdout,
            stderr,
            stdout_file,
            stderr_file,
            invocation.output_limit,
            started,
        )?;
        let process_id = owner.id();
        let mut sampler = if sample_process_group {
            invocation
                .rss_limit
                // A worker target shares the worker's process group so the
                // coordinator can kill both on loss. Keep that scope explicit
                // for sampled enforcement; authoritative wait4 RSS remains
                // exact-child-only.
                .map(|_| spawn_rss_sampler(shared_process_group.map_or(process_id, |group| {
                    u32::try_from(group.as_raw()).expect("worker process group fits u32")
                })))
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

            if observe_first_result && first_result_micros.is_none() {
                first_result_micros = captures.first_stdout_micros();
            }

            if forced_status.is_none() {
                if captures.capture_failed() {
                    terminate_until_observed(&mut owner)?;
                    break started.elapsed().as_micros();
                }
                if captures.output_limit_reached() {
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
            poll_delay(EXIT_POLL);
        };
        diagnostic_wall = Some(observed_micros);
        after_observation();

        let sampler_result = sampler.as_mut().map(RssSampler::finish).transpose();
        (diagnostic_exit, diagnostic_signal) = owner.observed_status();
        let resources_result = owner.finish();
        // Forced outcomes do not have a valid complete stream to preserve.
        // Stop their readers immediately after exact-child cleanup; a
        // descendant retaining a pipe descriptor must not delay bounded
        // timeout, cancellation, or output-limit completion. A natural
        // successful exit still drains both streams to EOF.
        let captures_result = captures.finish(forced_status.is_none() && !cancelled);
        let resources = resources_result?;
        let captures = captures_result?;
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

        // A child may exit before the capture worker reports its first byte.
        // Completion is the only honest timestamp available in that race; an
        // actually empty stdout remains untimestamped.
        let first_result_micros =
            first_result_micros.or_else(|| (captures.stdout_bytes > 0).then_some(observed_micros));
        let signal = exit_signal(resources.status);
        let status = if captures.stdout_limited || captures.stderr_limited {
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
                shared_process_group.is_some(),
            ),
            process_group_peak_rss_bytes,
            output_bytes: captures.stdout_bytes,
            stderr_bytes: captures.stderr_bytes,
            stdout_path: None,
            stderr_path: None,
            process_group_rss_observed,
        })
    })();
    measured.map_err(|error| MeasureError::Collection {
        source: Box::new(error),
        exit_code: diagnostic_exit,
        signal: diagnostic_signal,
        wall_time_micros: diagnostic_wall,
        stdout_path: paths.stdout.clone(),
        stderr_path: paths.stderr.clone(),
    })
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
        poll_delay(EXIT_POLL);
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
    const CAPTURE_LIMIT: u64 = 4 * 1024 * 1024;
    let stdout = NamedTempFile::new()?;
    let stderr = NamedTempFile::new()?;
    let mut command = Command::new("ps");
    command
        .args(["-axo", "pgid=,rss="])
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout.reopen()?))
        .stderr(Stdio::from(stderr.reopen()?));
    // Keep the diagnostic sampler outside the measured process group. The
    // worker may terminate the group while this child is still draining its
    // bounded output, and the sampler must own its own cleanup in that race.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        command.process_group(0);
    }
    let child = command.spawn()?;
    let mut owner = NativeChild::new(child);
    let waited = (|| {
        let deadline = Instant::now() + Duration::from_millis(250);
        loop {
            if stdout.as_file().metadata()?.len() > CAPTURE_LIMIT
                || stderr.as_file().metadata()?.len() > CAPTURE_LIMIT
            {
                return Err(io::Error::other("ps capture limit exceeded"));
            }
            if owner.observe_exit()? {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "ps inspection timed out",
                ));
            }
            poll_delay(Duration::from_millis(1));
        }
    })();
    let resources = match waited {
        Ok(()) => owner.finish(),
        Err(error) => {
            let cleanup = terminate_until_observed(&mut owner).and_then(|()| owner.finish());
            return match cleanup {
                Ok(_resources) => Err(error),
                Err(cleanup_error) => Err(io::Error::other(format!(
                    "{error}; ps cleanup failed: {cleanup_error}"
                ))),
            };
        }
    }?;
    let status = resources.status;
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
fn measurement_protocol(has_rss_sampler: bool, worker_process_group: bool) -> MeasurementProtocol {
    let rss_method = if has_rss_sampler {
        let scope = if worker_process_group {
            "worker process-group RSS sampler"
        } else {
            "target process-group RSS sampler"
        };
        format!("{scope} every {} micros", RSS_SAMPLE_INTERVAL.as_micros())
    } else {
        "native wait4 RSS checked at exit; no in-flight RSS enforcement".to_owned()
    };
    MeasurementProtocol {
        timing_method: format!(
            "{} direct spawn to waitid-WNOWAIT exit observation; bounded stdout/stderr pipe capture workers; first stdout byte capture-worker observation; nonblocking read retry interval {} micros; non-empty output first observable only at exit uses completion timestamp; {rss_method}; wait4-0.2.0 valid-OS-counter assumption",
            native_rss_provenance().label(),
            CAPTURE_POLL.as_micros()
        ),
        input_delivery: "prepared-seekable-stdin-file".to_owned(),
        rss_scope: if worker_process_group && has_rss_sampler {
            "wait4-child-lifetime-including-pre-exec-waited-descendants-and-threads; sampled worker process-group"
                .to_owned()
        } else {
            "wait4-child-lifetime-including-pre-exec-waited-descendants-and-threads".to_owned()
        },
        exit_poll_interval_micros: u64::try_from(EXIT_POLL.as_micros())
            .expect("fixed poll interval fits u64"),
        rss_poll_interval_micros: has_rss_sampler.then_some(
            u64::try_from(RSS_SAMPLE_INTERVAL.as_micros()).expect("fixed sample interval fits u64"),
        ),
        validated_accuracy_micros: None,
        worker: None,
        isolation_evidence: None,
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
fn configure_process_group(command: &mut Command, shared_group: Option<Pid>) {
    use std::os::unix::process::CommandExt as _;
    command.process_group(shared_group.map_or(0, Pid::as_raw));
}

#[cfg(not(unix))]
fn configure_process_group(_command: &mut Command, _shared_group: Option<()>) {}

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
    fn nonempty_output_exit_fallback_uses_observed_completion() {
        let invocation = super::BenchmarkInvocation {
            cancellation: None,
            executable: "/bin/echo".into(),
            args: vec!["output".to_owned()],
            stdin: Vec::new(),
            current_dir: None,
            timeout: super::Duration::from_secs(2),
            output_limit: 1024,
            rss_limit: None,
            retain_output: true,
        };
        // The test-only seam suppresses the live capture-worker observation,
        // forcing the real lifecycle through the exit-before-poll race.
        let result = super::measure_with_hooks(&invocation, false, || {}, || {}).unwrap();
        assert!(result.output_bytes > 0);
        assert_eq!(result.first_result_micros, Some(result.wall_time_micros));
        let stdout = result.stdout_path.as_ref().expect("retained stdout");
        assert_eq!(std::fs::read(stdout).unwrap(), b"output\n");
        std::fs::remove_file(stdout).unwrap();
        std::fs::remove_file(result.stderr_path.as_ref().expect("retained stderr")).unwrap();

        let empty = super::BenchmarkInvocation {
            executable: "/usr/bin/true".into(),
            retain_output: false,
            ..invocation
        };
        let empty_result = super::measure_with_hooks(&empty, false, || {}, || {}).unwrap();
        assert_eq!(empty_result.output_bytes, 0);
        assert_eq!(empty_result.first_result_micros, None);
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
