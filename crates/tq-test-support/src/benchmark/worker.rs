//! Isolated measurement worker protocol.
//!
//! The coordinator owns prepared capture files and sends only bounded command
//! metadata to a fresh worker image. The worker owns the target's exact-child
//! wait and writes one bounded reply file, then parks until the coordinator
//! terminates and reaps its process group.

use std::{
    env,
    fs::{self, OpenOptions},
    io::{self, Read as _, Seek as _, Write as _},
    os::unix::ffi::{OsStrExt as _, OsStringExt as _},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use nix::{
    sys::signal::{Signal, killpg},
    unistd::{Pid, getppid},
};
use serde::{Deserialize, Serialize};

use super::{
    BenchmarkInvocation, MeasureError, MeasuredOutcome, MeasuredStatus, collector_source_sha256,
    measure::{PreparedCapturePaths, measure_prepared, prepare_capture_files},
    native_process::{EXIT_POLL, NativeChild, poll_delay},
    report::WorkerIdentity,
};

const PROTOCOL_VERSION: u8 = 3;
const MAX_REQUEST_BYTES: usize = 64 * 1024;
const MAX_REPLY_BYTES: usize = 64 * 1024;
const WORKER_STARTUP_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Deserialize, Serialize)]
struct WorkerRequest {
    version: u8,
    executable: Vec<u8>,
    args: Vec<String>,
    current_dir: Option<Vec<u8>>,
    stdin_path: Vec<u8>,
    stdout_path: Vec<u8>,
    stderr_path: Vec<u8>,
    reply_path: Vec<u8>,
    timeout_micros: u64,
    output_limit: u64,
    rss_limit: Option<u64>,
    sample_process_group: bool,
    coordinator_pid: u32,
}

#[derive(Debug, Deserialize, Serialize)]
struct WorkerReply {
    version: u8,
    result: WorkerResult,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum WorkerResult {
    Outcome(Box<MeasuredOutcome>),
    Error {
        message: String,
        exit_code: Option<i32>,
        signal: Option<i32>,
        wall_time_micros: Option<u128>,
    },
}

struct WorkerFailure {
    message: String,
    exit_code: Option<i32>,
    signal: Option<i32>,
    wall_time_micros: Option<u128>,
}

/// Runs one invocation through a fresh worker process.
pub(crate) fn measure_process(
    invocation: &BenchmarkInvocation,
    sample_process_group: bool,
) -> Result<MeasuredOutcome, MeasureError> {
    if invocation
        .cancellation
        .as_ref()
        .is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Acquire))
    {
        return Err(MeasureError::Cancelled);
    }
    if invocation.executable.is_absolute() && !invocation.executable.is_file() {
        return Err(MeasureError::Io(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "benchmark executable not found: {}",
                invocation.executable.display()
            ),
        )));
    }
    let captures = prepare_capture_files(invocation)?;
    let paths = captures.paths();
    let worker_path = worker_executable().map_err(MeasureError::Io)?;
    let worker_identity = worker_identity(&worker_path).map_err(MeasureError::Io)?;
    let worker_result = run_worker(invocation, sample_process_group, &paths, &worker_path);
    match worker_result {
        Ok(WorkerReply {
            version: PROTOCOL_VERSION,
            result: WorkerResult::Outcome(mut outcome),
        }) => {
            outcome.measurement_protocol.worker = Some(worker_identity);
            if invocation.retain_output
                || outcome.status != MeasuredStatus::Exited
                || outcome.exit_code != Some(0)
            {
                let (stdout, stderr) = captures.keep_output()?;
                outcome.stdout_path = Some(stdout);
                outcome.stderr_path = Some(stderr);
            }
            Ok(*outcome)
        }
        Ok(WorkerReply {
            version: PROTOCOL_VERSION,
            result:
                WorkerResult::Error {
                    message,
                    exit_code,
                    signal,
                    wall_time_micros,
                },
        }) => {
            let (stdout, stderr) = captures.keep_output()?;
            Err(collection_error(
                format!("isolated worker target failed: {message}"),
                exit_code,
                signal,
                wall_time_micros,
                stdout,
                stderr,
            ))
        }
        Ok(WorkerReply { version, .. }) => {
            let (stdout, stderr) = captures.keep_output()?;
            Err(collection_error(
                format!("worker protocol version {version} did not match {PROTOCOL_VERSION}"),
                None,
                None,
                None,
                stdout,
                stderr,
            ))
        }
        Err(failure) => {
            let (stdout, stderr) = captures.keep_output()?;
            let source = if failure.message == "worker cancelled before reply" {
                MeasureError::Cancelled
            } else {
                MeasureError::Io(io::Error::other(format!(
                    "isolated worker failed: {}",
                    failure.message
                )))
            };
            Err(MeasureError::Collection {
                source: Box::new(source),
                exit_code: failure.exit_code,
                signal: failure.signal,
                wall_time_micros: failure.wall_time_micros,
                stdout_path: stdout,
                stderr_path: stderr,
            })
        }
    }
}

fn collection_error(
    message: String,
    exit_code: Option<i32>,
    signal: Option<i32>,
    wall_time_micros: Option<u128>,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
) -> MeasureError {
    MeasureError::Collection {
        source: Box::new(MeasureError::Io(io::Error::other(message))),
        exit_code,
        signal,
        wall_time_micros,
        stdout_path,
        stderr_path,
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "keep worker observation, shared-group cleanup, and reply handoff under one owner"
)]
fn run_worker(
    invocation: &BenchmarkInvocation,
    sample_process_group: bool,
    paths: &PreparedCapturePaths,
    worker_path: &Path,
) -> Result<WorkerReply, WorkerFailure> {
    let _reply_marker = ReplyMarkerGuard(reply_ready_path(&paths.reply));
    let request = WorkerRequest::from_invocation(invocation, sample_process_group, paths)
        .map_err(io_failure)?;
    let encoded = serde_json::to_vec(&request).map_err(|error| io_failure(error.to_string()))?;
    if encoded.len() > MAX_REQUEST_BYTES {
        return Err(io_failure(format!(
            "worker request exceeds {MAX_REQUEST_BYTES} bytes"
        )));
    }
    // Prepare the bounded control message before launching the worker. A
    // non-reading worker cannot block a pipe writer or leave a detached
    // request thread behind when process termination itself fails.
    let mut request_file = tempfile::tempfile().map_err(|error| io_failure(error.to_string()))?;
    request_file
        .write_all(&encoded)
        .and_then(|()| request_file.rewind())
        .map_err(|error| io_failure(format!("prepare worker request: {error}")))?;
    let mut command = Command::new(worker_path);
    command
        .stdin(Stdio::from(request_file))
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        command.process_group(0);
    }
    let child = command
        .spawn()
        .map_err(|error| io_failure(error.to_string()))?;
    let worker_started = Instant::now();
    let mut owner = NativeChild::new(child);
    let mut reply = None;
    let deadline = Instant::now()
        .checked_add(invocation.timeout.saturating_add(WORKER_STARTUP_TIMEOUT))
        .unwrap_or_else(|| Instant::now() + WORKER_STARTUP_TIMEOUT);
    loop {
        match owner.observe_exit() {
            Ok(true) => break,
            Ok(false) if cancellation_requested(invocation) => {
                terminate_worker(&mut owner).map_err(|error| io_failure(error.to_string()))?;
                return Err(WorkerFailure {
                    message: "worker cancelled before reply".to_owned(),
                    exit_code: None,
                    signal: None,
                    wall_time_micros: Some(worker_started.elapsed().as_micros()),
                });
            }
            Ok(false) if Instant::now() >= deadline => {
                terminate_worker(&mut owner).map_err(|error| io_failure(error.to_string()))?;
                return Err(WorkerFailure {
                    message: "worker did not exit before its bounded deadline".to_owned(),
                    exit_code: None,
                    signal: None,
                    wall_time_micros: None,
                });
            }
            Ok(false) => {
                if reply.is_none() && fs::metadata(reply_ready_path(&paths.reply)).is_ok() {
                    match read_reply(&paths.reply) {
                        Ok(value) => {
                            reply = Some(value);
                            terminate_worker(&mut owner)
                                .map_err(|error| io_failure(error.to_string()))?;
                            break;
                        }
                        Err(error) => {
                            terminate_worker(&mut owner)
                                .map_err(|error| io_failure(error.to_string()))?;
                            return Err(io_failure(format!(
                                "read worker reply before acknowledgement: {error}"
                            )));
                        }
                    }
                }
                poll_delay(EXIT_POLL);
            }
            Err(error) => {
                let cleanup = terminate_worker(&mut owner).err();
                return Err(io_failure(match cleanup {
                    Some(cleanup) => {
                        format!("observe worker exit: {error}; cleanup worker: {cleanup}")
                    }
                    None => format!("observe worker exit: {error}"),
                }));
            }
        }
    }
    let status = owner.observed_status();
    let accepted_reply = reply.is_some() && status == (None, Some(9));
    let finish_result = owner.finish();
    if let Err(error) = finish_result {
        let _ = fs::remove_file(reply_ready_path(&paths.reply));
        return Err(io_failure(format!("cleanup and reap worker: {error}")));
    }
    let _ = fs::remove_file(reply_ready_path(&paths.reply));
    if status != (Some(0), None) && !accepted_reply {
        return Err(WorkerFailure {
            message: format!(
                "worker exited abnormally: exit={:?}, signal={:?}",
                status.0, status.1
            ),
            exit_code: None,
            signal: None,
            wall_time_micros: None,
        });
    }
    let reply_result = reply.map_or_else(|| read_reply(&paths.reply), Ok);
    reply_result.map_err(|error| io_failure(format!("read worker reply: {error}")))
}

fn reply_ready_path(reply: &Path) -> PathBuf {
    reply.with_extension("ready")
}

fn mark_reply_ready(reply: &Path) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(reply_ready_path(reply))?;
    file.write_all(b"1")
}

struct ReplyMarkerGuard(PathBuf);

impl Drop for ReplyMarkerGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn terminate_worker(owner: &mut NativeChild) -> io::Result<()> {
    owner.terminate()?;
    let deadline = Instant::now() + WORKER_STARTUP_TIMEOUT;
    while !owner.observe_exit()? {
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "worker did not exit after process-group termination",
            ));
        }
        poll_delay(EXIT_POLL);
    }
    Ok(())
}

fn cancellation_requested(invocation: &BenchmarkInvocation) -> bool {
    invocation
        .cancellation
        .as_ref()
        .is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Acquire))
}

fn read_reply(path: &Path) -> io::Result<WorkerReply> {
    let size = fs::metadata(path)?.len();
    let size = usize::try_from(size)
        .map_err(|_| io::Error::other("worker reply size does not fit usize"))?;
    if size == 0 {
        return Err(io::Error::other("worker reply was empty"));
    }
    if size > MAX_REPLY_BYTES {
        return Err(io::Error::other(format!(
            "worker reply exceeds {MAX_REPLY_BYTES} bytes"
        )));
    }
    let bytes = fs::read(path)?;
    serde_json::from_slice(&bytes).map_err(io::Error::other)
}

fn io_failure(message: impl Into<String>) -> WorkerFailure {
    WorkerFailure {
        message: message.into(),
        exit_code: None,
        signal: None,
        wall_time_micros: None,
    }
}

impl WorkerRequest {
    fn from_invocation(
        invocation: &BenchmarkInvocation,
        sample_process_group: bool,
        paths: &PreparedCapturePaths,
    ) -> Result<Self, String> {
        let timeout_micros = u64::try_from(invocation.timeout.as_micros())
            .map_err(|_| "worker timeout does not fit u64 microseconds".to_owned())?;
        Ok(Self {
            version: PROTOCOL_VERSION,
            executable: path_bytes(&invocation.executable),
            args: invocation.args.clone(),
            current_dir: invocation.current_dir.as_deref().map(path_bytes),
            stdin_path: path_bytes(&paths.stdin),
            stdout_path: path_bytes(&paths.stdout),
            stderr_path: path_bytes(&paths.stderr),
            reply_path: path_bytes(&paths.reply),
            timeout_micros,
            output_limit: invocation.output_limit,
            rss_limit: invocation.rss_limit,
            sample_process_group,
            coordinator_pid: std::process::id(),
        })
    }
}

/// Kills the worker's process group if its coordinator disappears.
///
/// The target deliberately shares this group, so a coordinator crash cannot
/// orphan a target. The watchdog is a small Rust thread rather than a second
/// process, which keeps the target's launch floor and resource owner stable.
struct CoordinatorWatchdog {
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    failure: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    handle: Option<thread::JoinHandle<()>>,
}

impl CoordinatorWatchdog {
    fn start(group: Pid, coordinator_pid: u32) -> Result<Self, String> {
        let expected = i32::try_from(coordinator_pid)
            .map_err(|_| "coordinator PID does not fit process identifier".to_owned())?;
        if getppid().as_raw() != expected {
            return Err(format!(
                "worker coordinator changed before target launch: expected {coordinator_pid}, actual {}",
                getppid().as_raw()
            ));
        }
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let failure = std::sync::Arc::new(std::sync::Mutex::new(None));
        let thread_stop = std::sync::Arc::clone(&stop);
        let thread_failure = std::sync::Arc::clone(&failure);
        let handle = thread::Builder::new()
            .name("benchmark-coordinator-watchdog".to_owned())
            .spawn(move || {
                while !thread_stop.load(std::sync::atomic::Ordering::Acquire) {
                    if getppid().as_raw() != expected {
                        // The signal includes this watchdog and the target,
                        // and therefore cannot leave an orphaned target.
                        if let Err(error) = killpg(group, Signal::SIGKILL)
                            && let Ok(mut failure) = thread_failure.lock()
                        {
                            *failure = Some(error.to_string());
                        }
                        return;
                    }
                    thread::park_timeout(Duration::from_millis(25));
                }
            })
            .map_err(|error| format!("start coordinator watchdog: {error}"))?;
        Ok(Self {
            stop,
            failure,
            handle: Some(handle),
        })
    }

    fn stop(&mut self) -> Result<(), String> {
        self.stop.store(true, std::sync::atomic::Ordering::Release);
        if let Some(handle) = self.handle.take() {
            handle.thread().unpark();
            handle
                .join()
                .map_err(|_| "watchdog thread panicked".to_owned())?;
        }
        self.failure
            .lock()
            .map_err(|_| "watchdog failure state poisoned".to_owned())?
            .take()
            .map_or(Ok(()), Err)
    }
}

impl Drop for CoordinatorWatchdog {
    fn drop(&mut self) {
        if self.handle.is_some()
            && let Err(error) = self.stop()
        {
            eprintln!("tq-bench: coordinator watchdog cleanup failed: {error}");
        }
    }
}

/// Entry point used by the dedicated worker binary.
///
/// # Errors
///
/// Returns a bounded protocol, capture, native lifecycle, or reply-file error.
/// Target failures are encoded in the reply whenever the reply file remains
/// writable; they are not returned as an unstructured process failure.
///
/// # Panics
///
/// Panics only if the fixed protocol cap cannot be represented as a `u64`,
/// which is guarded by the compile-time-sized constant used here.
pub fn run() -> Result<(), String> {
    // The request cap is a compile-time usize constant well below u64::MAX.
    // Keeping this conversion explicit documents the bound passed to `take`.
    let mut input = Vec::new();
    io::stdin()
        .take(u64::try_from(MAX_REQUEST_BYTES + 1).expect("request cap fits u64"))
        .read_to_end(&mut input)
        .map_err(|error| format!("read worker request: {error}"))?;
    if input.len() > MAX_REQUEST_BYTES {
        return Err(format!("worker request exceeds {MAX_REQUEST_BYTES} bytes"));
    }
    let request: WorkerRequest = serde_json::from_slice(&input)
        .map_err(|error| format!("decode worker request: {error}"))?;
    if request.version != PROTOCOL_VERSION {
        return Err(format!(
            "worker request version {} did not match {PROTOCOL_VERSION}",
            request.version
        ));
    }
    let worker_group = Pid::from_raw(
        i32::try_from(std::process::id())
            .map_err(|_| "worker PID does not fit process-group identifier".to_owned())?,
    );
    let watchdog = CoordinatorWatchdog::start(worker_group, request.coordinator_pid)?;
    let reply_path = path_from_bytes(request.reply_path);
    let paths = PreparedCapturePaths {
        stdin: path_from_bytes(request.stdin_path),
        stdout: path_from_bytes(request.stdout_path),
        stderr: path_from_bytes(request.stderr_path),
        reply: reply_path.clone(),
    };
    let invocation = BenchmarkInvocation {
        cancellation: None,
        executable: path_from_bytes(request.executable),
        args: request.args,
        stdin: Vec::new(),
        current_dir: request.current_dir.map(path_from_bytes),
        timeout: Duration::from_micros(request.timeout_micros),
        output_limit: request.output_limit,
        rss_limit: request.rss_limit,
        retain_output: false,
    };
    let result = measure_prepared(
        &invocation,
        request.sample_process_group,
        Some(worker_group),
        &paths,
        || {},
        || {},
    );
    let reply = match result {
        Ok(outcome) => WorkerReply {
            version: PROTOCOL_VERSION,
            result: WorkerResult::Outcome(Box::new(outcome)),
        },
        Err(error) => {
            let (exit_code, signal, wall_time_micros) = error_diagnostics(&error);
            WorkerReply {
                version: PROTOCOL_VERSION,
                result: WorkerResult::Error {
                    message: error.to_string(),
                    exit_code,
                    signal,
                    wall_time_micros,
                },
            }
        }
    };
    write_reply(&reply_path, &reply)?;
    mark_reply_ready(&reply_path).map_err(|error| format!("mark worker reply ready: {error}"))?;
    // The coordinator kills and reaps this worker after validating the ready
    // reply. Keeping the worker alive preserves the process-group anchor until
    // cleanup has completed; the watchdog handles coordinator loss meanwhile.
    std::mem::forget(watchdog);
    loop {
        thread::park();
    }
}

fn write_reply(path: &Path, reply: &WorkerReply) -> Result<(), String> {
    let encoded =
        serde_json::to_vec(reply).map_err(|error| format!("encode worker reply: {error}"))?;
    if encoded.len() > MAX_REPLY_BYTES {
        return Err(format!("worker reply exceeds {MAX_REPLY_BYTES} bytes"));
    }
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(path)
        .map_err(|error| format!("open worker reply: {error}"))?;
    file.write_all(&encoded)
        .and_then(|()| file.write_all(b"\n"))
        .map_err(|error| format!("write worker reply: {error}"))
}

fn error_diagnostics(error: &MeasureError) -> (Option<i32>, Option<i32>, Option<u128>) {
    match error {
        MeasureError::Collection {
            exit_code,
            signal,
            wall_time_micros,
            ..
        } => (*exit_code, *signal, *wall_time_micros),
        _ => (None, None, None),
    }
}

fn worker_executable() -> io::Result<PathBuf> {
    if let Some(path) = env::var_os("TQ_BENCH_WORKER") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("TQ_BENCH_WORKER is not a file: {}", path.display()),
        ));
    }
    let current = env::current_exe()?;
    let directory = current
        .parent()
        .ok_or_else(|| io::Error::other("current executable has no parent directory"))?;
    let direct = directory.join("tq-bench-worker");
    if direct.is_file() {
        return Ok(direct);
    }
    // Integration-test binaries live in `target/*/deps`, while Cargo places
    // ordinary binaries beside that directory. Never select a hashed `deps`
    // artifact: it may be stale or a `.d` metadata sibling from another build.
    let deps_parent = directory
        .parent()
        .ok_or_else(|| io::Error::other("worker test executable has no target directory"))?;
    let sibling = deps_parent.join("tq-bench-worker");
    if sibling.is_file() {
        return Ok(sibling);
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "tq-bench-worker was not found beside the current target directory",
    ))
}

fn worker_identity(path: &Path) -> io::Result<WorkerIdentity> {
    use sha2::{Digest as _, Sha256};
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut hash = Sha256::new();
    hash.update(fs::read(path)?);
    let executable_sha256 = hash
        .finalize()
        .iter()
        .flat_map(|byte| {
            [
                char::from(HEX[usize::from(byte >> 4)]),
                char::from(HEX[usize::from(byte & 15)]),
            ]
        })
        .collect();
    Ok(WorkerIdentity {
        executable_sha256,
        launch_protocol: format!("tq-bench-worker-protocol-v{PROTOCOL_VERSION}"),
        collector_source_sha256: collector_source_sha256(),
    })
}

fn path_bytes(path: &Path) -> Vec<u8> {
    path.as_os_str().as_bytes().to_vec()
}

fn path_from_bytes(bytes: Vec<u8>) -> PathBuf {
    std::ffi::OsString::from_vec(bytes).into()
}

#[cfg(test)]
mod tests {
    use super::{CoordinatorWatchdog, PROTOCOL_VERSION, WorkerReply, WorkerResult};

    #[test]
    fn reply_round_trip_preserves_u128_wall_time() {
        let reply = WorkerReply {
            version: PROTOCOL_VERSION,
            result: WorkerResult::Error {
                message: "bounded test error".to_owned(),
                exit_code: None,
                signal: Some(9),
                wall_time_micros: Some(u128::MAX - 7),
            },
        };
        let encoded = serde_json::to_vec(&reply).expect("encode worker reply");
        let decoded: WorkerReply = serde_json::from_slice(&encoded).expect("decode worker reply");
        match decoded.result {
            WorkerResult::Error {
                message,
                signal,
                wall_time_micros,
                ..
            } => {
                assert_eq!(message, "bounded test error");
                assert_eq!(signal, Some(9));
                assert_eq!(wall_time_micros, Some(u128::MAX - 7));
            }
            WorkerResult::Outcome(_) => panic!("round-trip changed the result variant"),
        }
    }

    #[test]
    fn coordinator_watchdog_stops_cleanly_while_parent_is_alive() {
        let group = nix::unistd::getpid();
        let coordinator = u32::try_from(nix::unistd::getppid().as_raw()).expect("parent PID fits");
        let mut watchdog = CoordinatorWatchdog::start(group, coordinator)
            .expect("current test process is the coordinator");
        watchdog.stop().expect("watchdog should stop cleanly");
    }
}
