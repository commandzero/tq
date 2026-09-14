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
    sync::{Mutex, OnceLock},
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
    measure::{
        PreparedCaptureFiles, PreparedCapturePaths, measure_prepared, prepare_capture_files,
    },
    native_process::{EXIT_POLL, NativeChild, poll_delay},
    report::WorkerIdentity,
};

const PROTOCOL_VERSION: u8 = 4;
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
        /// True only when the target measurement reached the cancellation
        /// path.  Worker transport/lifecycle failures must never be inferred
        /// from their diagnostic wording.
        #[serde(default)]
        cancelled: bool,
    },
}

struct WorkerFailure {
    message: String,
    exit_code: Option<i32>,
    signal: Option<i32>,
    wall_time_micros: Option<u128>,
    cancelled: bool,
    pending: Option<PendingWorker>,
}

/// A worker retained after the caller's bounded deadline. The worker remains
/// the target's parent and exact-wait owner until it observes the abort marker,
/// terminates the target, and writes its terminal reply. Dropping this owner
/// before that handoff would make exact resource collection impossible on
/// macOS.
struct PendingWorker {
    owner: NativeChild,
    captures: Option<PreparedCaptureFiles>,
}

const MAX_IN_FLIGHT_WORKERS: usize = 16;
const DEFERRED_CLEANUP_POLL: Duration = Duration::from_millis(25);
const CLEANUP_IDLE: u8 = 0;
const CLEANUP_PENDING: u8 = 1;
const CLEANUP_RUNNING: u8 = 2;
const CLEANUP_DONE: u8 = 3;

enum CleanupCommand {
    Wait,
    Pending(PendingWorker),
    Shutdown,
}

struct CleanupSlot {
    command: Mutex<CleanupCommand>,
    wake: std::sync::Condvar,
    phase: std::sync::atomic::AtomicU8,
    handle: Mutex<Option<thread::JoinHandle<()>>>,
}

struct PendingRegistry(Vec<Option<std::sync::Arc<CleanupSlot>>>);

struct PendingReservation {
    index: usize,
    slot: std::sync::Arc<CleanupSlot>,
    held: bool,
}

static PENDING_WORKER: OnceLock<Mutex<PendingRegistry>> = OnceLock::new();
// Serializes the short transition between checking pending cleanup and
// launching a worker with the handoff that makes cleanup pending. Healthy
// workers still overlap because this lock is held only across `spawn`.
static WORKER_ADMISSION: OnceLock<Mutex<()>> = OnceLock::new();

fn pending_worker_slot() -> &'static Mutex<PendingRegistry> {
    PENDING_WORKER.get_or_init(|| {
        Mutex::new(PendingRegistry(
            (0..MAX_IN_FLIGHT_WORKERS).map(|_| None).collect(),
        ))
    })
}

fn lock_pending_registry() -> std::sync::MutexGuard<'static, PendingRegistry> {
    match pending_worker_slot().lock() {
        Ok(registry) => registry,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn lock_worker_admission() -> std::sync::MutexGuard<'static, ()> {
    match WORKER_ADMISSION.get_or_init(|| Mutex::new(())).lock() {
        Ok(admission) => admission,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn pending_cleanup_active() -> bool {
    lock_pending_registry().0.iter().flatten().any(|slot| {
        matches!(
            slot.phase.load(std::sync::atomic::Ordering::Acquire),
            CLEANUP_PENDING | CLEANUP_RUNNING
        )
    })
}

fn reserve_pending_slot() -> io::Result<PendingReservation> {
    let mut registry = lock_pending_registry();
    for slot in &mut registry.0 {
        if slot.as_ref().is_some_and(|slot| {
            slot.phase.load(std::sync::atomic::Ordering::Acquire) == CLEANUP_DONE
        }) {
            if let Some(slot_ref) = slot.as_ref()
                && let Some(handle) = slot_ref
                    .handle
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .take()
            {
                let _ = handle.join();
            }
            *slot = None;
        }
    }
    if registry.0.iter().flatten().any(|slot| {
        matches!(
            slot.phase.load(std::sync::atomic::Ordering::Acquire),
            CLEANUP_PENDING | CLEANUP_RUNNING
        )
    }) {
        return Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "a previous worker cleanup remains pending",
        ));
    }
    let index = registry.0.iter().position(Option::is_none).ok_or_else(|| {
        io::Error::new(io::ErrorKind::WouldBlock, "native worker capacity is full")
    })?;
    let slot = std::sync::Arc::new(CleanupSlot {
        command: Mutex::new(CleanupCommand::Wait),
        wake: std::sync::Condvar::new(),
        phase: std::sync::atomic::AtomicU8::new(CLEANUP_IDLE),
        handle: Mutex::new(None),
    });
    registry.0[index] = Some(std::sync::Arc::clone(&slot));
    drop(registry);
    let thread_slot = std::sync::Arc::clone(&slot);
    let handle = thread::Builder::new()
        .name("benchmark-pending-worker-reaper".to_owned())
        .spawn(move || cleanup_executor(thread_slot));
    let handle = match handle {
        Ok(handle) => handle,
        Err(error) => {
            let mut registry = lock_pending_registry();
            registry.0[index] = None;
            return Err(io::Error::other(format!(
                "start reserved worker cleanup executor: {error}"
            )));
        }
    };
    *slot
        .handle
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(handle);
    Ok(PendingReservation {
        index,
        slot,
        held: true,
    })
}

#[allow(
    clippy::needless_pass_by_value,
    reason = "the executor thread must own the slot for its entire lifetime"
)]
fn cleanup_executor(slot: std::sync::Arc<CleanupSlot>) {
    let command = {
        let mut command = slot
            .command
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        while matches!(&*command, CleanupCommand::Wait) {
            command = slot
                .wake
                .wait(command)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        std::mem::replace(&mut *command, CleanupCommand::Shutdown)
    };
    match command {
        CleanupCommand::Pending(pending) => {
            slot.phase
                .store(CLEANUP_RUNNING, std::sync::atomic::Ordering::Release);
            if let Err(error) = drive_pending_reaper(pending) {
                eprintln!("tq-bench: deferred worker cleanup failed: {error}");
            }
        }
        CleanupCommand::Shutdown | CleanupCommand::Wait => {}
    }
    slot.phase
        .store(CLEANUP_DONE, std::sync::atomic::Ordering::Release);
}

impl PendingReservation {
    fn handoff(&mut self, pending: &mut Option<PendingWorker>) -> io::Result<()> {
        // Make the pending transition atomic with another invocation's launch
        // admission. This lock is not held while the executor drains the
        // owner, so healthy workers retain their normal overlap.
        let _admission = lock_worker_admission();
        let mut command = self
            .slot
            .command
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !matches!(*command, CleanupCommand::Wait) {
            return Err(io::Error::other("reserved cleanup executor is not idle"));
        }
        *command = CleanupCommand::Pending(pending.take().expect("pending worker handoff payload"));
        self.slot
            .phase
            .store(CLEANUP_PENDING, std::sync::atomic::Ordering::Release);
        self.slot.wake.notify_one();
        self.held = false;
        Ok(())
    }

    fn release(&mut self) {
        if !self.held {
            return;
        }
        {
            let mut command = self
                .slot
                .command
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *command = CleanupCommand::Shutdown;
            self.slot
                .phase
                .store(CLEANUP_DONE, std::sync::atomic::Ordering::Release);
            self.slot.wake.notify_one();
        }
        if let Some(handle) = self
            .slot
            .handle
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
        {
            let _ = handle.join();
        }
        let mut registry = lock_pending_registry();
        if registry.0.get(self.index).is_some_and(|current| {
            current
                .as_ref()
                .is_some_and(|slot| std::sync::Arc::ptr_eq(slot, &self.slot))
        }) {
            registry.0[self.index] = None;
        }
        self.held = false;
    }
}

impl Drop for PendingReservation {
    fn drop(&mut self) {
        self.release();
    }
}

fn drive_pending_reaper(mut pending: PendingWorker) -> io::Result<()> {
    let mut abort_error_reported = false;
    let mut termination_error_reported = false;
    let mut observation_error_reported = false;
    let mut reap_error_reported = false;
    loop {
        let paths = pending.captures.as_ref().map(PreparedCaptureFiles::paths);
        let ready = paths
            .as_ref()
            .is_some_and(|paths| fs::metadata(reply_ready_path(&paths.reply)).is_ok());
        match pending.owner.observe_exit() {
            Ok(false) if !ready => {
                // The caller may have lost the race with marker creation (or
                // the initial marker write may have failed).  Keep trying from
                // the owned cleanup thread; the caller must never kill the
                // worker merely because this control write was unavailable.
                if let Some(paths) = paths.as_ref()
                    && let Err(error) = request_worker_abort(paths)
                    && !abort_error_reported
                {
                    eprintln!("tq-bench: deferred worker abort marker will retry: {error}");
                    abort_error_reported = true;
                }
                poll_delay(DEFERRED_CLEANUP_POLL);
                continue;
            }
            Ok(false) => {
                if let Err(error) = terminate_worker(&mut pending.owner) {
                    if error.raw_os_error() == Some(nix::libc::ECHILD) {
                        return finalize_pending_reaper(
                            &mut pending,
                            Err(io::Error::other(format!(
                                "deferred worker lost exact ownership while terminating: {error}"
                            ))),
                        );
                    }
                    if !termination_error_reported {
                        eprintln!("tq-bench: deferred worker termination will retry: {error}");
                        termination_error_reported = true;
                    }
                    poll_delay(DEFERRED_CLEANUP_POLL);
                    continue;
                }
            }
            Ok(true) => {}
            Err(error) if error.raw_os_error() == Some(nix::libc::ECHILD) => {
                return finalize_pending_reaper(
                    &mut pending,
                    Err(io::Error::other(format!(
                        "deferred worker lost exact ownership: {error}"
                    ))),
                );
            }
            Err(error) => {
                if !observation_error_reported {
                    eprintln!("tq-bench: deferred worker observation will retry: {error}");
                    observation_error_reported = true;
                }
                poll_delay(DEFERRED_CLEANUP_POLL);
                continue;
            }
        }
        match pending.owner.try_finish4() {
            Ok(Some(_resources)) => {
                let result = finalize_pending_reaper(&mut pending, Ok(()));
                if let Some(paths) = paths {
                    let _ = fs::remove_file(reply_ready_path(&paths.reply));
                    let _ = fs::remove_file(cancel_path(&paths.reply));
                }
                return result;
            }
            Ok(None) => poll_delay(DEFERRED_CLEANUP_POLL),
            Err(error) if error.raw_os_error() == Some(nix::libc::ECHILD) => {
                return finalize_pending_reaper(
                    &mut pending,
                    Err(io::Error::other(format!(
                        "deferred worker lost exact ownership while reaping: {error}"
                    ))),
                );
            }
            Err(error) => {
                if !reap_error_reported {
                    eprintln!("tq-bench: deferred worker reap will retry: {error}");
                    reap_error_reported = true;
                }
                poll_delay(DEFERRED_CLEANUP_POLL);
            }
        }
    }
}

fn finalize_pending_reaper(pending: &mut PendingWorker, result: io::Result<()>) -> io::Result<()> {
    let paths = pending.captures.as_ref().map(PreparedCaptureFiles::paths);
    let capture_result = pending
        .captures
        .take()
        .map_or(Ok(()), |captures| captures.keep_output().map(|_| ()));
    if let Some(paths) = paths {
        let _ = fs::remove_file(reply_ready_path(&paths.reply));
        let _ = fs::remove_file(cancel_path(&paths.reply));
    }
    match (result, capture_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(error)) => Err(io::Error::other(format!(
            "preserve deferred worker captures: {error}"
        ))),
        (Err(error), Err(capture_error)) => Err(io::Error::other(format!(
            "{error}; preserve deferred worker captures: {capture_error}"
        ))),
    }
}

/// Runs one invocation through a fresh worker process.
#[allow(
    clippy::too_many_lines,
    reason = "keep capture ownership and deferred cleanup handoff together"
)]
pub(crate) fn measure_process(
    invocation: &BenchmarkInvocation,
    sample_process_group: bool,
) -> Result<MeasuredOutcome, MeasureError> {
    poll_pending_worker().map_err(MeasureError::Io)?;
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
    let mut pending_reservation = reserve_pending_slot().map_err(MeasureError::Io)?;
    let paths = captures.paths();
    let worker_path = worker_executable_path().map_err(MeasureError::Io)?;
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
                    cancelled,
                },
        }) => {
            let (stdout, stderr) = captures.keep_output()?;
            let source = if cancelled {
                MeasureError::Cancelled
            } else {
                MeasureError::Io(io::Error::other(format!(
                    "isolated worker target failed: {message}"
                )))
            };
            Err(MeasureError::Collection {
                source: Box::new(source),
                exit_code,
                signal,
                wall_time_micros,
                stdout_path: stdout,
                stderr_path: stderr,
            })
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
            if let Some(mut pending) = failure.pending {
                let output_paths = captures.paths();
                pending.captures = Some(captures);
                let mut pending = Some(pending);
                while let Err(error) = pending_reservation.handoff(&mut pending) {
                    // The reservation is preallocated before spawn. A retry
                    // here is only a poisoned/inconsistent registry handoff;
                    // retaining the owner is safer than dropping the exact
                    // wait4 owner and orphaning its target.
                    eprintln!("tq-bench: pending worker handoff delayed: {error}");
                    poll_delay(EXIT_POLL);
                }
                let source = if failure.cancelled {
                    MeasureError::Cancelled
                } else {
                    MeasureError::Io(io::Error::other(format!(
                        "isolated worker cleanup remains pending: {}",
                        failure.message
                    )))
                };
                return Err(MeasureError::Collection {
                    source: Box::new(source),
                    exit_code: failure.exit_code,
                    signal: failure.signal,
                    wall_time_micros: failure.wall_time_micros,
                    stdout_path: output_paths.stdout,
                    stderr_path: output_paths.stderr,
                });
            }
            let (stdout, stderr) = captures.keep_output()?;
            let source = if failure.cancelled {
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

/// Reject new native samples while a deferred owner is being drained. Healthy
/// measurements may overlap; the reservation itself is created before each
/// worker launch, and becomes active only after a caller hands off a pending
/// owner.
fn poll_pending_worker() -> io::Result<()> {
    let _admission = lock_worker_admission();
    if pending_cleanup_active() {
        Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "a previous worker cleanup remains pending",
        ))
    } else {
        Ok(())
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "keep worker observation, shared-group cleanup, and reply handoff under one owner"
)]
#[allow(
    clippy::result_large_err,
    reason = "the failure carries the exact child needed for deferred reaping"
)]
fn run_worker(
    invocation: &BenchmarkInvocation,
    sample_process_group: bool,
    paths: &PreparedCapturePaths,
    worker_path: &Path,
) -> Result<WorkerReply, WorkerFailure> {
    let mut reply_marker = ReplyMarkerGuard {
        path: reply_ready_path(&paths.reply),
        armed: true,
    };
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
    let child = {
        let _admission = lock_worker_admission();
        if pending_cleanup_active() {
            return Err(io_failure(
                "a previous worker cleanup became pending before worker launch",
            ));
        }
        command
            .spawn()
            .map_err(|error| io_failure(error.to_string()))?
    };
    let mut owner = NativeChild::new(child);
    let mut reply = None;
    let mut abort_requested = false;
    let mut cancellation_deadline = None;
    let deadline = Instant::now()
        .checked_add(invocation.timeout.saturating_add(WORKER_STARTUP_TIMEOUT))
        .unwrap_or_else(|| Instant::now() + WORKER_STARTUP_TIMEOUT);
    loop {
        match owner.observe_exit() {
            Ok(true) => break,
            Ok(false) if cancellation_requested(invocation) && !abort_requested => {
                let abort_error = request_worker_abort(paths).err();
                if let Some(error) = abort_error {
                    reply_marker.disarm();
                    return Err(WorkerFailure {
                        message: format!(
                            "worker cancellation marker failed; cleanup remains pending: {error}"
                        ),
                        exit_code: None,
                        signal: None,
                        wall_time_micros: None,
                        cancelled: true,
                        pending: Some(PendingWorker {
                            owner,
                            captures: None,
                        }),
                    });
                }
                abort_requested = true;
                cancellation_deadline = Some(Instant::now() + WORKER_STARTUP_TIMEOUT);
            }
            Ok(false)
                if abort_requested
                    && cancellation_deadline.is_some_and(|deadline| Instant::now() >= deadline) =>
            {
                reply_marker.disarm();
                return Err(WorkerFailure {
                    message: "worker cancellation cleanup remains pending after bounded grace"
                        .to_owned(),
                    exit_code: None,
                    signal: None,
                    wall_time_micros: None,
                    cancelled: true,
                    pending: Some(PendingWorker {
                        owner,
                        captures: None,
                    }),
                });
            }
            Ok(false) if Instant::now() >= deadline => {
                let abort_error = request_worker_abort(paths).err();
                reply_marker.disarm();
                return Err(WorkerFailure {
                    message: abort_error.map_or_else(
                        || "worker cleanup remains pending after bounded deadline".to_owned(),
                        |error| format!("worker cancellation marker failed; cleanup remains pending: {error}"),
                    ),
                    exit_code: None,
                    signal: None,
                    wall_time_micros: None,
                    cancelled: false,
                    pending: Some(PendingWorker {
                        owner,
                        captures: None,
                    }),
                });
            }
            Ok(false) => {
                if reply.is_none() && fs::metadata(reply_ready_path(&paths.reply)).is_ok() {
                    match read_reply(&paths.reply) {
                        Ok(value) => {
                            reply = Some(value);
                            if let Err(error) = terminate_worker(&mut owner) {
                                reply_marker.disarm();
                                return Err(WorkerFailure {
                                    message: format!(
                                        "worker acknowledgement cleanup remains pending: {error}"
                                    ),
                                    exit_code: None,
                                    signal: None,
                                    wall_time_micros: None,
                                    cancelled: false,
                                    pending: Some(PendingWorker {
                                        owner,
                                        captures: None,
                                    }),
                                });
                            }
                            break;
                        }
                        Err(error) => {
                            // A malformed or truncated reply is a
                            // post-spawn control failure.  Do not kill the
                            // worker group here: the worker remains the exact
                            // target-wait owner until the reserved cleanup
                            // executor takes over.
                            let abort_error = request_worker_abort(paths).err();
                            reply_marker.disarm();
                            return Err(WorkerFailure {
                                message: abort_error.map_or_else(
                                    || format!(
                                        "read worker reply before acknowledgement: {error}; cleanup remains pending"
                                    ),
                                    |abort| format!(
                                        "read worker reply before acknowledgement: {error}; abort marker failed: {abort}"
                                    ),
                                ),
                                exit_code: None,
                                signal: None,
                                wall_time_micros: None,
                                cancelled: false,
                                pending: Some(PendingWorker { owner, captures: None }),
                            });
                        }
                    }
                }
                poll_delay(EXIT_POLL);
            }
            Err(error) => {
                if error.raw_os_error() == Some(nix::libc::ECHILD) {
                    return Err(io_failure(format!(
                        "observe worker exit lost ownership: {error}"
                    )));
                }
                let abort_error = request_worker_abort(paths).err();
                reply_marker.disarm();
                return Err(WorkerFailure {
                    message: abort_error.map_or_else(
                        || format!("observe worker exit: {error}; cleanup remains pending"),
                        |abort| {
                            format!("observe worker exit: {error}; abort marker failed: {abort}")
                        },
                    ),
                    exit_code: None,
                    signal: None,
                    wall_time_micros: None,
                    cancelled: false,
                    pending: Some(PendingWorker {
                        owner,
                        captures: None,
                    }),
                });
            }
        }
    }
    let status = owner.observed_status();
    let accepted_reply = reply.is_some() && status == (None, Some(9));
    if let Err(error) = owner.finish() {
        reply_marker.disarm();
        return Err(WorkerFailure {
            message: format!("cleanup and reap worker remains pending: {error}"),
            exit_code: None,
            signal: None,
            wall_time_micros: None,
            cancelled: false,
            pending: Some(PendingWorker {
                owner,
                captures: None,
            }),
        });
    }
    let _ = fs::remove_file(reply_ready_path(&paths.reply));
    let _ = fs::remove_file(cancel_path(&paths.reply));
    if status != (Some(0), None) && !accepted_reply {
        return Err(WorkerFailure {
            message: format!(
                "worker exited abnormally: exit={:?}, signal={:?}",
                status.0, status.1
            ),
            exit_code: None,
            signal: None,
            wall_time_micros: None,
            cancelled: false,
            pending: None,
        });
    }
    let reply_result = reply.map_or_else(|| read_reply(&paths.reply), Ok);
    reply_result.map_err(|error| io_failure(format!("read worker reply: {error}")))
}

fn reply_ready_path(reply: &Path) -> PathBuf {
    reply.with_extension("ready")
}

fn cancel_path(reply: &Path) -> PathBuf {
    reply.with_extension("abort")
}

fn request_worker_abort(paths: &PreparedCapturePaths) -> io::Result<()> {
    let path = cancel_path(&paths.reply);
    match OpenOptions::new().create_new(true).write(true).open(path) {
        Ok(mut file) => file.write_all(b"abort"),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error),
    }
}

fn mark_reply_ready(reply: &Path) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(reply_ready_path(reply))?;
    file.write_all(b"1")
}

struct ReplyMarkerGuard {
    path: PathBuf,
    armed: bool,
}

impl ReplyMarkerGuard {
    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for ReplyMarkerGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_file(&self.path);
        }
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
        cancelled: false,
        pending: None,
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

/// Requests the target owner to perform its own bounded cleanup if its
/// coordinator disappears or sends an abort marker.
///
/// The target deliberately shares the worker's process group, but this thread
/// never signals that group: doing so could kill the target-parent before its
/// exact wait4. It only sets the worker-owned cancellation flag; the worker's
/// lifecycle loop then terminates, reaps, and records the target before the
/// worker exits. A thread cannot outlive this worker process.
struct CoordinatorWatchdog {
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    coordinator_lost: std::sync::Arc<std::sync::atomic::AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl CoordinatorWatchdog {
    fn start(
        coordinator_pid: u32,
        cancel: &std::sync::Arc<std::sync::atomic::AtomicBool>,
        abort_path: PathBuf,
    ) -> Result<Self, String> {
        let expected = i32::try_from(coordinator_pid)
            .map_err(|_| "coordinator PID does not fit process identifier".to_owned())?;
        if getppid().as_raw() != expected {
            return Err(format!(
                "worker coordinator changed before target launch: expected {coordinator_pid}, actual {}",
                getppid().as_raw()
            ));
        }
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let coordinator_lost = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let thread_stop = std::sync::Arc::clone(&stop);
        let thread_cancel = std::sync::Arc::clone(cancel);
        let thread_lost = std::sync::Arc::clone(&coordinator_lost);
        let handle = thread::Builder::new()
            .name("benchmark-coordinator-watchdog".to_owned())
            .spawn(move || {
                while !thread_stop.load(std::sync::atomic::Ordering::Acquire) {
                    if getppid().as_raw() != expected {
                        thread_lost.store(true, std::sync::atomic::Ordering::Release);
                        thread_cancel.store(true, std::sync::atomic::Ordering::Release);
                        return;
                    }
                    if fs::metadata(&abort_path).is_ok() {
                        thread_cancel.store(true, std::sync::atomic::Ordering::Release);
                    }
                    thread::park_timeout(Duration::from_millis(25));
                }
            })
            .map_err(|error| format!("start coordinator watchdog: {error}"))?;
        Ok(Self {
            stop,
            coordinator_lost,
            handle: Some(handle),
        })
    }

    fn coordinator_lost(&self) -> bool {
        self.coordinator_lost
            .load(std::sync::atomic::Ordering::Acquire)
    }

    fn stop(&mut self) -> Result<(), String> {
        self.stop.store(true, std::sync::atomic::Ordering::Release);
        if let Some(handle) = self.handle.take() {
            handle.thread().unpark();
            handle
                .join()
                .map_err(|_| "watchdog thread panicked".to_owned())?;
        }
        Ok(())
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
    let reply_path = path_from_bytes(request.reply_path);
    let cancellation = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut watchdog = CoordinatorWatchdog::start(
        request.coordinator_pid,
        &cancellation,
        cancel_path(&reply_path),
    )?;
    let paths = PreparedCapturePaths {
        stdin: path_from_bytes(request.stdin_path),
        stdout: path_from_bytes(request.stdout_path),
        stderr: path_from_bytes(request.stderr_path),
        reply: reply_path.clone(),
    };
    let invocation = BenchmarkInvocation {
        cancellation: Some(cancellation),
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
    // Once `measure_prepared` returns, its NativeChild has either been
    // resource-reaped or never launched.  Only now is it safe for a lost
    // coordinator to terminate this anchored group; doing so earlier would
    // kill the worker while it still owns the target's exact wait.
    if watchdog.coordinator_lost() {
        return Err(cleanup_worker_group_after_target(
            &mut watchdog,
            worker_group,
            "coordinator disappeared before worker reply",
        ));
    }
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
                    cancelled: error_is_cancelled(&error),
                },
            }
        }
    };
    publish_reply_and_park(&reply_path, &reply, &mut watchdog, worker_group)
}

fn publish_reply_and_park(
    reply_path: &Path,
    reply: &WorkerReply,
    watchdog: &mut CoordinatorWatchdog,
    worker_group: Pid,
) -> Result<(), String> {
    if let Err(error) = write_reply(reply_path, reply) {
        return Err(cleanup_worker_group_after_target(
            watchdog,
            worker_group,
            &format!("write worker reply: {error}"),
        ));
    }
    if let Err(error) = mark_reply_ready(reply_path) {
        return Err(cleanup_worker_group_after_target(
            watchdog,
            worker_group,
            &format!("mark worker reply ready: {error}"),
        ));
    }
    if watchdog.coordinator_lost() {
        return Err(cleanup_worker_group_after_target(
            watchdog,
            worker_group,
            "coordinator disappeared after target reap",
        ));
    }
    // The coordinator kills and reaps this worker after validating the ready
    // reply. Keeping the worker alive preserves the process-group anchor until
    // cleanup has completed; the watchdog remains live so a coordinator that
    // disappears after the reply still lets this worker exit cleanly.
    loop {
        if watchdog.coordinator_lost() {
            return Err(cleanup_worker_group_after_target(
                watchdog,
                worker_group,
                "coordinator disappeared after target reap",
            ));
        }
        thread::park_timeout(Duration::from_millis(25));
    }
}

fn cleanup_worker_group_after_target(
    watchdog: &mut CoordinatorWatchdog,
    worker_group: Pid,
    reason: &str,
) -> String {
    let watchdog_result = watchdog.stop();
    let group_result = match killpg(worker_group, Signal::SIGKILL) {
        Ok(()) | Err(nix::errno::Errno::ESRCH) => Ok(()),
        Err(error) => Err(format!("kill worker process group: {error}")),
    };
    match (watchdog_result, group_result) {
        (Ok(()), Ok(())) => reason.to_owned(),
        (Err(watchdog), Ok(())) => format!("{reason}; stop coordinator watchdog: {watchdog}"),
        (Ok(()), Err(group)) => format!("{reason}; {group}"),
        (Err(watchdog), Err(group)) => {
            format!("{reason}; stop coordinator watchdog: {watchdog}; {group}")
        }
    }
}

fn write_reply(path: &Path, reply: &WorkerReply) -> Result<(), String> {
    let encoded =
        serde_json::to_vec(reply).map_err(|error| format!("encode worker reply: {error}"))?;
    if encoded.len().saturating_add(1) > MAX_REPLY_BYTES {
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

fn error_is_cancelled(error: &MeasureError) -> bool {
    match error {
        MeasureError::Cancelled => true,
        MeasureError::Collection { source, .. } => error_is_cancelled(source),
        _ => false,
    }
}

/// Resolves the worker executable used by both measurement and calibration.
///
/// An explicit `TQ_BENCH_WORKER` path wins. Otherwise the worker is resolved
/// beside the current benchmark executable, including Cargo's integration
/// test and build-script output layouts.
///
/// # Errors
///
/// Returns an I/O error when the explicit worker path is invalid, the current
/// executable cannot be located, or no worker exists in a recognized layout.
pub fn worker_executable_path() -> io::Result<PathBuf> {
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
    worker_beside(&current)
}

fn worker_beside(current: &Path) -> io::Result<PathBuf> {
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
    // Newer Cargo versions put tests in
    // `target/<profile>/build/<package>/<hash>/out`. Only recognize this
    // specific layout; do not search arbitrary ancestors for executables.
    if directory.file_name().is_some_and(|name| name == "out")
        && let Some(build) = directory.ancestors().nth(3)
        && build.file_name().is_some_and(|name| name == "build")
        && let Some(profile) = build.parent()
    {
        let worker = profile.join("tq-bench-worker");
        if worker.is_file() {
            return Ok(worker);
        }
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
    use std::{
        os::unix::{fs::PermissionsExt as _, process::CommandExt as _},
        process::Command,
        sync::{Mutex, OnceLock, atomic::Ordering},
        thread,
        time::{Duration, Instant},
    };

    use super::{
        BenchmarkInvocation, CoordinatorWatchdog, MAX_IN_FLIGHT_WORKERS, MAX_REPLY_BYTES,
        PROTOCOL_VERSION, WorkerReply, WorkerResult, read_reply, write_reply,
    };

    fn registry_test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    struct ReleaseOnDrop(std::path::PathBuf);

    impl Drop for ReleaseOnDrop {
        fn drop(&mut self) {
            let _ = std::fs::write(&self.0, b"release");
        }
    }

    #[test]
    fn worker_lookup_preserves_direct_and_deps_layouts() {
        let target = tempfile::tempdir().expect("target directory");
        let worker = target.path().join("tq-bench-worker");
        std::fs::write(&worker, []).expect("worker executable fixture");
        for relative in ["tq-bench", "deps/benchmark_gate-test-hash"] {
            assert_eq!(
                super::worker_beside(&target.path().join(relative)).unwrap(),
                worker
            );
        }
    }

    #[test]
    fn worker_lookup_supports_cargo_build_directory_layout() {
        let target = tempfile::tempdir().expect("target directory");
        let profile = target.path().join("debug");
        let directory = profile.join("build/tq-test-support/test-hash/out");
        std::fs::create_dir_all(&directory).expect("test executable directory");
        let worker = profile.join("tq-bench-worker");
        std::fs::write(&worker, []).expect("worker executable fixture");
        assert_eq!(
            super::worker_beside(&directory.join("benchmark_gate-test-hash")).unwrap(),
            worker
        );
    }

    #[test]
    fn worker_lookup_does_not_search_unrecognized_ancestors() {
        let target = tempfile::tempdir().expect("target directory");
        let directory = target.path().join("unrelated/package/hash/out");
        std::fs::create_dir_all(&directory).expect("test executable directory");
        std::fs::write(target.path().join("tq-bench-worker"), [])
            .expect("unrelated executable fixture");
        assert_eq!(
            super::worker_beside(&directory.join("test"))
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::NotFound
        );
    }

    #[test]
    fn reply_round_trip_preserves_u128_wall_time() {
        let reply = WorkerReply {
            version: PROTOCOL_VERSION,
            result: WorkerResult::Error {
                message: "bounded test error".to_owned(),
                exit_code: None,
                signal: Some(9),
                wall_time_micros: Some(u128::MAX - 7),
                cancelled: false,
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

    fn error_reply(message_len: usize) -> WorkerReply {
        WorkerReply {
            version: PROTOCOL_VERSION,
            result: WorkerResult::Error {
                message: "x".repeat(message_len),
                exit_code: None,
                signal: None,
                wall_time_micros: None,
                cancelled: false,
            },
        }
    }

    #[test]
    fn reply_limit_includes_the_terminal_newline() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("reply");
        std::fs::File::create(&path).expect("reply file");
        let empty_len = serde_json::to_vec(&error_reply(0)).unwrap().len();
        let message_len = MAX_REPLY_BYTES - 1 - empty_len;
        let exact = error_reply(message_len);
        assert_eq!(
            serde_json::to_vec(&exact).unwrap().len(),
            MAX_REPLY_BYTES - 1
        );
        write_reply(&path, &exact).expect("payload plus newline fits reply cap");
        assert_eq!(
            std::fs::metadata(&path).unwrap().len(),
            MAX_REPLY_BYTES as u64
        );
        read_reply(&path).expect("bounded reply parses with newline");

        let too_large = error_reply(message_len + 1);
        assert!(write_reply(&path, &too_large).is_err());
    }

    #[test]
    fn coordinator_watchdog_stops_cleanly_while_parent_is_alive() {
        let directory = tempfile::tempdir().expect("watchdog marker directory");
        let coordinator = u32::try_from(nix::unistd::getppid().as_raw()).expect("parent PID fits");
        let cancellation = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let mut watchdog =
            CoordinatorWatchdog::start(coordinator, &cancellation, directory.path().join("abort"))
                .expect("current test process is the coordinator");
        watchdog.stop().expect("watchdog should stop cleanly");
    }

    #[test]
    fn deferred_cleanup_executor_reaps_without_a_followup_measurement() {
        let _registry_guard = registry_test_lock();
        let directory = tempfile::tempdir().expect("held-child directory");
        let release = directory.path().join("release");
        let _release_guard = ReleaseOnDrop(release.clone());

        // Reserve the executor before launching the child.  This is the
        // ordering the production coordinator must preserve when a later
        // control failure needs to hand off the exact owner.
        let mut reservation = super::reserve_pending_slot().expect("reserve cleanup slot");
        let slot = std::sync::Arc::clone(&reservation.slot);
        let captures = super::prepare_capture_files(&BenchmarkInvocation {
            cancellation: None,
            executable: "/bin/sh".into(),
            args: Vec::new(),
            stdin: Vec::new(),
            current_dir: None,
            timeout: Duration::from_secs(1),
            output_limit: 1024,
            rss_limit: None,
            retain_output: false,
        })
        .expect("prepare retained captures");
        let paths = captures.paths();
        let mut command = Command::new("/bin/sh");
        command
            .args([
                "-c",
                "for attempt in $(seq 1 1000); do [ -f \"$1\" ] && exit 0; sleep 0.01; done; exit 99",
                "held-child",
                release.to_str().expect("release path is UTF-8"),
            ])
            .process_group(0);
        let child = command.spawn().expect("spawn held child after reservation");
        let owner = super::NativeChild::new(child);
        let child_pid = owner.id();
        let mut pending = Some(super::PendingWorker {
            owner,
            captures: Some(captures),
        });
        reservation
            .handoff(&mut pending)
            .expect("handoff owner to prestarted executor");
        assert!(pending.is_none(), "handoff retained the owner locally");
        assert!(
            super::poll_pending_worker().is_err(),
            "pending cleanup must block new measurement admission"
        );

        std::fs::write(&release, b"release").expect("release held child");
        let deadline = Instant::now() + Duration::from_secs(2);
        while slot.phase.load(Ordering::Acquire) != super::CLEANUP_DONE {
            assert!(
                Instant::now() < deadline,
                "background cleanup did not finish without another measurement"
            );
            thread::sleep(Duration::from_millis(10));
        }

        assert!(
            paths.stdout.is_file(),
            "deferred stdout capture was preserved"
        );
        assert!(
            paths.stderr.is_file(),
            "deferred stderr capture was preserved"
        );

        // The background executor must have performed the exact wait4.  A
        // second non-consuming wait observes the terminal ECHILD rather than
        // finding an unreaped zombie or silently accepting a reused PID.
        let pid =
            rustix::process::Pid::from_raw(i32::try_from(child_pid).expect("child PID fits i32"))
                .expect("valid child PID");
        let wait = rustix::process::waitid(
            rustix::process::WaitId::Pid(pid),
            rustix::process::WaitIdOptions::EXITED
                | rustix::process::WaitIdOptions::NOHANG
                | rustix::process::WaitIdOptions::NOWAIT,
        );
        assert!(
            matches!(wait, Err(rustix::io::Errno::CHILD)),
            "waitid={wait:?}"
        );

        std::fs::remove_file(paths.stdout).expect("remove deferred stdout");
        std::fs::remove_file(paths.stderr).expect("remove deferred stderr");

        // Reclaim the completed join handle and release the fresh idle slot so
        // this test leaves the process-global registry reusable.
        let replacement = super::reserve_pending_slot().expect("reclaim completed slot");
        drop(replacement);
    }

    #[test]
    fn pending_handoff_after_reservation_blocks_worker_launch() {
        let _registry_guard = registry_test_lock();
        let directory = tempfile::tempdir().expect("launch marker directory");
        let marker = directory.path().join("launched");
        let worker = directory.path().join("worker.sh");
        std::fs::write(
            &worker,
            format!("#!/bin/sh\nprintf launched > '{}'\n", marker.display()),
        )
        .expect("write worker fixture");
        let mut permissions = std::fs::metadata(&worker)
            .expect("worker fixture metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&worker, permissions).expect("make worker executable");

        // The reservation is deliberately acquired first. A pending handoff
        // can then win the race between a second invocation's preflight and
        // its actual spawn; the launch admission check must reject it.
        let blocker = super::reserve_pending_slot().expect("reserve cleanup slot");
        blocker
            .slot
            .phase
            .store(super::CLEANUP_PENDING, Ordering::Release);
        let invocation = BenchmarkInvocation {
            cancellation: None,
            executable: "/bin/true".into(),
            args: Vec::new(),
            stdin: Vec::new(),
            current_dir: None,
            timeout: Duration::from_secs(1),
            output_limit: 1024,
            rss_limit: None,
            retain_output: false,
        };
        let captures = super::prepare_capture_files(&invocation).expect("prepare captures");
        let paths = captures.paths();
        let failure = super::run_worker(&invocation, false, &paths, &worker)
            .expect_err("pending cleanup must reject the launch");
        assert!(failure.pending.is_none(), "rejected launch owns no child");
        assert!(!marker.exists(), "worker launched despite pending handoff");

        drop(captures);
        drop(blocker);
    }

    #[test]
    fn reservation_capacity_fails_before_a_worker_launch() {
        let _registry_guard = registry_test_lock();
        let reservations = (0..MAX_IN_FLIGHT_WORKERS)
            .map(|_| super::reserve_pending_slot().expect("reserve capacity slot"))
            .collect::<Vec<_>>();
        let error = super::reserve_pending_slot()
            .err()
            .expect("capacity must be bounded");
        assert_eq!(error.kind(), std::io::ErrorKind::WouldBlock);
        drop(reservations);
    }

    #[test]
    fn cancellation_transport_is_typed_not_message_classified() {
        let reply = WorkerReply {
            version: PROTOCOL_VERSION,
            result: WorkerResult::Error {
                message: "benchmark measurement cancelled".to_owned(),
                exit_code: None,
                signal: None,
                wall_time_micros: None,
                cancelled: false,
            },
        };
        let encoded = serde_json::to_vec(&reply).expect("encode reply");
        let decoded: WorkerReply = serde_json::from_slice(&encoded).expect("decode reply");
        let WorkerResult::Error { cancelled, .. } = decoded.result else {
            panic!("typed cancellation reply changed variant")
        };
        assert!(
            !cancelled,
            "diagnostic wording must not classify cancellation"
        );
    }

    #[test]
    fn missing_cancellation_field_decodes_as_false_for_v4_fixtures() {
        let value = serde_json::json!({
            "version": PROTOCOL_VERSION,
            "result": {
                "error": {
                    "message": "old fixture",
                    "exit_code": null,
                    "signal": null,
                    "wall_time_micros": null
                }
            }
        });
        let decoded: WorkerReply = serde_json::from_value(value).expect("decode fixture");
        let WorkerResult::Error { cancelled, .. } = decoded.result else {
            panic!("fixture changed result variant")
        };
        assert!(!cancelled);
    }
}
