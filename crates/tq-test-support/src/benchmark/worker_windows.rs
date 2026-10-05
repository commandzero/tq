//! Windows isolated worker protocol and retained deferred lifecycle executor.

use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::sync::atomic::AtomicBool;
use std::{
    env,
    ffi::OsString,
    fs::{self, File},
    io::{self, Read as _, Write as _},
    os::windows::ffi::{OsStrExt as _, OsStringExt as _},
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU8, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use tempfile::NamedTempFile;
use winsafe::{HPROCESS, guard::CloseHandleGuard};

use super::{
    BenchmarkInvocation, MeasureError, MeasuredOutcome, MeasuredStatus, collector_source_sha256,
    measure_windows::measure_target,
    native_process_windows::{EXIT_POLL, io_error, open_process, process_has_exited},
    report::WorkerIdentity,
};

const PROTOCOL: &str = "tq-bench-worker-windows-v1";
const MESSAGE_LIMIT: usize = 64 * 1024;
const STARTUP_TIMEOUT: Duration = Duration::from_secs(5);
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(5);

// Only the worker owns target kill-on-close jobs. Enclosing the worker in a
// coordinator-owned job would destroy the sole collector when that owner dies.
struct WorkerProcess {
    child: tokio::process::Child,
    process: io::Result<CloseHandleGuard<HPROCESS>>,
    status: Option<std::process::ExitStatus>,
    abort: PathBuf,
}

impl WorkerProcess {
    fn spawn(mut command: tokio::process::Command, abort: PathBuf) -> io::Result<Self> {
        command.kill_on_drop(false);
        let child = command.spawn()?;
        // Before try_wait, Tokio's retained child anchors the PID even if the
        // worker exits immediately. Keep the independent handle through cleanup.
        let process = open_process(child.id().expect("new Tokio child has an identity"));
        Ok(Self {
            child,
            process,
            status: None,
            abort,
        })
    }

    #[cfg(test)]
    fn id(&self) -> u32 {
        self.child.id().expect("retained worker identity")
    }

    fn observe_exit(&mut self) -> io::Result<Option<std::process::ExitStatus>> {
        if self.status.is_none() {
            self.status = self.child.try_wait()?;
        }
        Ok(self.status)
    }

    fn terminate_after_collection(&mut self) -> io::Result<()> {
        if self.status.is_some() {
            return Ok(());
        }
        if let Ok(process) = &self.process
            && process_has_exited(process)?
        {
            return Ok(());
        }
        self.child.start_kill()
    }
}

impl Drop for WorkerProcess {
    fn drop(&mut self) {
        if self.status.is_none() {
            // An error/panic handoff asks the independent collector to cancel;
            // it never kills that collector before exact-target accounting.
            let _ = fs::write(&self.abort, []);
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct PreparedPaths {
    pub stdin: PathBuf,
    pub stdout: PathBuf,
    pub stderr: PathBuf,
    pub reply: PathBuf,
}

struct Captures {
    stdin: NamedTempFile,
    stdout: NamedTempFile,
    stderr: NamedTempFile,
    reply: NamedTempFile,
}

impl Captures {
    fn prepare(invocation: &BenchmarkInvocation) -> io::Result<Self> {
        let mut stdin = NamedTempFile::new()?;
        stdin.write_all(&invocation.stdin)?;
        stdin.flush()?;
        Ok(Self {
            stdin,
            stdout: NamedTempFile::new()?,
            stderr: NamedTempFile::new()?,
            reply: NamedTempFile::new()?,
        })
    }
    fn paths(&self) -> PreparedPaths {
        PreparedPaths {
            stdin: self.stdin.path().to_owned(),
            stdout: self.stdout.path().to_owned(),
            stderr: self.stderr.path().to_owned(),
            reply: self.reply.path().to_owned(),
        }
    }
    fn keep(self) -> io::Result<(PathBuf, PathBuf)> {
        Ok((
            self.stdout.keep().map_err(|e| e.error)?.1,
            self.stderr.keep().map_err(|e| e.error)?.1,
        ))
    }
}

#[derive(Serialize, Deserialize)]
struct Request {
    protocol: String,
    executable: Vec<u16>,
    args: Vec<String>,
    directory: Option<Vec<u16>>,
    stdin: Vec<u16>,
    stdout: Vec<u16>,
    stderr: Vec<u16>,
    reply: Vec<u16>,
    timeout_micros: u64,
    output_limit: u64,
    rss_limit: Option<u64>,
    sample_tree: bool,
    coordinator_pid: u32,
    coordinator_created: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Reply {
    Outcome {
        protocol: String,
        outcome: Box<MeasuredOutcome>,
    },
    Error {
        protocol: String,
        message: String,
        cancelled: bool,
        exit_code: Option<i32>,
        wall_time_micros: Option<u128>,
    },
}

#[derive(Clone, Copy)]
#[repr(u8)]
enum TargetCleanup {
    Pending,
    Verified,
    Fallback,
}

pub(super) struct WorkerControl {
    coordinator: CloseHandleGuard<HPROCESS>,
    abort: PathBuf,
    target_cleanup: AtomicU8,
}
impl WorkerControl {
    #[cfg(test)]
    pub(super) fn for_test(reply: &Path) -> io::Result<Self> {
        Ok(Self {
            coordinator: open_process(std::process::id())?,
            abort: marker(reply, "abort"),
            target_cleanup: AtomicU8::new(TargetCleanup::Pending as u8),
        })
    }

    fn new(request: &Request, reply: &Path) -> io::Result<Self> {
        let coordinator = open_process(request.coordinator_pid)?;
        let (created, _, _, _) = coordinator.GetProcessTimes().map_err(io_error)?;
        if u64::from(created) != request.coordinator_created {
            return Err(io::Error::other(
                "worker coordinator identity changed before launch",
            ));
        }
        Ok(Self {
            coordinator,
            abort: marker(reply, "abort"),
            target_cleanup: AtomicU8::new(TargetCleanup::Pending as u8),
        })
    }
    pub(super) fn mark_target_collected(&self) {
        self.target_cleanup
            .store(TargetCleanup::Verified as u8, Ordering::Release);
    }

    pub(super) fn mark_fallback_collected(&self) {
        self.target_cleanup
            .store(TargetCleanup::Fallback as u8, Ordering::Release);
    }

    pub(super) fn cancelled(&self) -> io::Result<bool> {
        if process_has_exited(&self.coordinator)? {
            return Ok(true);
        }
        match fs::metadata(&self.abort) {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }
}

// A reserved thread owns the runtime, worker, and capture files from before
// launch until cleanup finishes. A caller timeout never drops that owner.
// Healthy measurements may run concurrently. A deferred cleanup blocks new
// launches, as on Unix, until its explicit owner completes.
#[repr(u8)]
enum ExecutorPhase {
    Running,
    Deferred,
    CleanupComplete,
}

struct LifecycleExecutor {
    handle: JoinHandle<()>,
    phase: Arc<AtomicU8>,
}

const MAX_IN_FLIGHT_WORKERS: usize = 4;
static EXECUTOR: OnceLock<Mutex<Vec<LifecycleExecutor>>> = OnceLock::new();
// Separate from the registry: reaping may join CleanupComplete executors under
// its lock. Launch admission never joins; an executor publishes CleanupComplete
// only after its last launch-admission acquisition and owned cleanup are over.
static WORKER_ADMISSION: Mutex<()> = Mutex::new(());

fn lock_worker_admission() -> std::sync::MutexGuard<'static, ()> {
    WORKER_ADMISSION
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn deferred_cleanup_active() -> io::Result<bool> {
    let Some(slot) = EXECUTOR.get() else {
        return Ok(false);
    };
    let registry = slot
        .lock()
        .map_err(|_| io::Error::other("Windows lifecycle registry poisoned"))?;
    Ok(registry
        .iter()
        .any(|executor| executor.phase.load(Ordering::Acquire) == ExecutorPhase::Deferred as u8))
}

fn defer_executor(phase: &AtomicU8) {
    let _admission = lock_worker_admission();
    let _ = phase.compare_exchange(
        ExecutorPhase::Running as u8,
        ExecutorPhase::Deferred as u8,
        Ordering::AcqRel,
        Ordering::Acquire,
    );
}

#[cfg(test)]
struct LaunchPause {
    executable: PathBuf,
    reached: mpsc::SyncSender<()>,
    release: mpsc::Receiver<()>,
    spawned: Arc<AtomicBool>,
}

#[cfg(test)]
static LAUNCH_PAUSE: Mutex<Option<LaunchPause>> = Mutex::new(None);

#[cfg(test)]
fn pause_before_worker_spawn(executable: &Path) -> io::Result<Option<Arc<AtomicBool>>> {
    let pause = {
        let mut slot = LAUNCH_PAUSE.lock().unwrap();
        if slot
            .as_ref()
            .is_some_and(|pause| pause.executable == executable)
        {
            slot.take()
        } else {
            None
        }
    };
    if let Some(pause) = pause {
        pause.reached.send(()).map_err(io::Error::other)?;
        pause
            .release
            .recv_timeout(CLEANUP_TIMEOUT)
            .map_err(io::Error::other)?;
        Ok(Some(pause.spawned))
    } else {
        Ok(None)
    }
}

fn reap_finished_executors(registry: &mut Vec<LifecycleExecutor>) -> io::Result<()> {
    let mut index = 0;
    while index < registry.len() {
        let previous = &registry[index];
        let phase = previous.phase.load(Ordering::Acquire);
        if !previous.handle.is_finished() && phase == ExecutorPhase::Deferred as u8 {
            return Err(io::Error::other(
                "previous Windows worker cleanup is still owned by the deferred executor",
            ));
        }
        if !previous.handle.is_finished() && phase == ExecutorPhase::Running as u8 {
            index += 1;
            continue;
        }
        // A reply can arrive immediately before the sender exits. Cleanup is
        // already complete, but joining remains bounded against scheduling.
        let deadline = Instant::now() + CLEANUP_TIMEOUT;
        while !previous.handle.is_finished() {
            if Instant::now() >= deadline {
                return Err(io::Error::other(
                    "completed Windows lifecycle executor has not exited",
                ));
            }
            thread::sleep(EXIT_POLL);
        }
        if registry.swap_remove(index).handle.join().is_err() {
            return Err(io::Error::other("Windows lifecycle executor panicked"));
        }
    }
    Ok(())
}

#[allow(
    clippy::too_many_lines,
    reason = "keep pre-launch executor reservation and bounded caller handoff together"
)]
pub(crate) fn measure_process(
    invocation: &BenchmarkInvocation,
    sample_tree: bool,
) -> Result<MeasuredOutcome, MeasureError> {
    if invocation
        .cancellation
        .as_ref()
        .is_some_and(|c| c.load(Ordering::Acquire))
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
    // File preparation must not hold the registry while another executor's
    // final launch check needs to inspect its phases under admission.
    let captures = Captures::prepare(invocation)?;
    let paths = captures.paths();
    let slot = EXECUTOR.get_or_init(|| Mutex::new(Vec::with_capacity(MAX_IN_FLIGHT_WORKERS)));
    let mut registry = slot
        .lock()
        .map_err(|_| io::Error::other("Windows lifecycle registry poisoned"))?;
    reap_finished_executors(&mut registry)?;
    if registry.len() >= MAX_IN_FLIGHT_WORKERS {
        return Err(MeasureError::Io(io::Error::other(
            "Windows lifecycle executor capacity unavailable before launch",
        )));
    }
    let phase = Arc::new(AtomicU8::new(ExecutorPhase::Running as u8));
    let thread_phase = Arc::clone(&phase);
    let request = invocation.clone();
    let (sender, receiver) = mpsc::sync_channel(1);
    let handle = thread::Builder::new()
        .name("tq-bench-windows-lifecycle".to_owned())
        .spawn(move || {
            let result = runtime().map_err(MeasureError::Io).and_then(|runtime| {
                runtime.block_on(coordinate(&request, sample_tree, &captures.paths()))
            });
            // Persist before publishing. Only a caller that actually receives
            // a successful outcome may remove its captures; a deadline/reply
            // race must not delete deferred-cleanup diagnostics.
            let result = match captures.keep() {
                Ok((stdout, stderr)) => attach_captures(result, stdout, stderr),
                Err(error) => Err(MeasureError::Io(error)),
            };
            thread_phase.store(ExecutorPhase::CleanupComplete as u8, Ordering::Release);
            let _ = sender.send(result);
        })?;
    registry.push(LifecycleExecutor {
        handle,
        phase: Arc::clone(&phase),
    });
    drop(registry);
    let deadline = Instant::now()
        .checked_add(
            invocation
                .timeout
                .saturating_add(STARTUP_TIMEOUT)
                .saturating_add(CLEANUP_TIMEOUT),
        )
        .ok_or_else(|| io::Error::other("invocation deadline exceeds monotonic clock"))?;
    let mut cancelled_deadline = None;
    loop {
        match receiver.recv_timeout(Duration::from_millis(1)) {
            Ok(result) => {
                let mut outcome = result?;
                if !invocation.retain_output
                    && outcome.status == MeasuredStatus::Exited
                    && outcome.exit_code == Some(0)
                {
                    for path in [&mut outcome.stdout_path, &mut outcome.stderr_path] {
                        if let Some(path) = path.take() {
                            fs::remove_file(path)?;
                        }
                    }
                }
                return Ok(outcome);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(MeasureError::Io(io::Error::other(
                    "Windows lifecycle executor lost its reply",
                )));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        let cancelled = invocation
            .cancellation
            .as_ref()
            .is_some_and(|c| c.load(Ordering::Acquire));
        if cancelled {
            cancelled_deadline.get_or_insert_with(|| Instant::now() + CLEANUP_TIMEOUT);
        }
        if Instant::now() >= deadline || cancelled_deadline.is_some_and(|d| Instant::now() >= d) {
            defer_executor(&phase);
            return Err(MeasureError::Collection {
                source: Box::new(if cancelled {
                    MeasureError::Cancelled
                } else {
                    MeasureError::Io(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "Windows worker cleanup remains owned by the deferred executor",
                    ))
                }),
                exit_code: None,
                signal: None,
                wall_time_micros: None,
                stdout_path: paths.stdout,
                stderr_path: paths.stderr,
            });
        }
    }
}

fn attach_captures(
    mut result: Result<MeasuredOutcome, MeasureError>,
    stdout: PathBuf,
    stderr: PathBuf,
) -> Result<MeasuredOutcome, MeasureError> {
    match &mut result {
        Ok(outcome) => {
            outcome.stdout_path = Some(stdout);
            outcome.stderr_path = Some(stderr);
        }
        Err(MeasureError::Collection {
            stdout_path,
            stderr_path,
            ..
        }) => {
            *stdout_path = stdout;
            *stderr_path = stderr;
        }
        Err(_) => {
            result = result.map_err(|source| MeasureError::Collection {
                source: Box::new(source),
                exit_code: None,
                signal: None,
                wall_time_micros: None,
                stdout_path: stdout,
                stderr_path: stderr,
            });
        }
    }
    result
}

#[allow(
    clippy::too_many_lines,
    reason = "keep worker completion and target-parent ownership together"
)]
async fn coordinate(
    invocation: &BenchmarkInvocation,
    sample_tree: bool,
    paths: &PreparedPaths,
) -> Result<MeasuredOutcome, MeasureError> {
    let worker = worker_executable_path()?;
    let identity = WorkerIdentity {
        executable_sha256: hash_bytes(&fs::read(&worker)?),
        launch_protocol: PROTOCOL.to_owned(),
        collector_source_sha256: collector_source_sha256(),
    };
    let (created, _, _, _) = HPROCESS::GetCurrentProcess()
        .GetProcessTimes()
        .map_err(io_error)?;
    let request = Request {
        protocol: PROTOCOL.to_owned(),
        executable: wide(&invocation.executable),
        args: invocation.args.clone(),
        directory: invocation.current_dir.as_deref().map(wide),
        stdin: wide(&paths.stdin),
        stdout: wide(&paths.stdout),
        stderr: wide(&paths.stderr),
        reply: wide(&paths.reply),
        timeout_micros: u64::try_from(invocation.timeout.as_micros()).map_err(io_error)?,
        output_limit: invocation.output_limit,
        rss_limit: invocation.rss_limit,
        sample_tree,
        coordinator_pid: std::process::id(),
        coordinator_created: u64::from(created),
    };
    let encoded = serde_json::to_vec(&request).map_err(io_error)?;
    if encoded.len() > MESSAGE_LIMIT {
        return Err(MeasureError::Io(io::Error::other(
            "worker request exceeds protocol cap",
        )));
    }
    let mut request_file = NamedTempFile::new()?;
    request_file.write_all(&encoded)?;
    request_file.flush()?;
    let mut command = tokio::process::Command::new(worker);
    command
        .stdin(Stdio::from(File::open(request_file.path())?))
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(test)]
    {
        if let Some((executable, fault, gate)) = &*FAULT_WORKER.lock().unwrap()
            && executable == &invocation.executable
        {
            command = tokio::process::Command::new(env::current_exe()?);
            command.args([
                "--exact",
                "benchmark::worker::tests::persistent_job_error_worker",
                "--nocapture",
            ]);
            command.env("TQ_BENCH_TEST_JOB_FAULT", fault);
            command.env("TQ_BENCH_TEST_JOB_FAULT_GATE", gate);
            command.stdin(Stdio::from(File::open(request_file.path())?));
            command.stdout(Stdio::null()).stderr(Stdio::null());
        }
    }
    #[cfg(test)]
    let spawned = pause_before_worker_spawn(&invocation.executable)?;
    let mut owner = {
        let _admission = lock_worker_admission();
        if deferred_cleanup_active()? {
            return Err(MeasureError::Io(io::Error::other(
                "previous Windows worker cleanup became deferred before worker launch",
            )));
        }
        WorkerProcess::spawn(command, marker(&paths.reply, "abort"))?
    };
    #[cfg(test)]
    if let Some(spawned) = spawned {
        spawned.store(true, Ordering::Release);
    }
    #[cfg(test)]
    if env::var_os("TQ_BENCH_TEST_COORDINATOR_DEATH").is_some() {
        fs::write(marker(&paths.reply, "worker-pid"), owner.id().to_string())?;
    }
    let started = Instant::now();
    let deadline = started
        .checked_add(invocation.timeout.saturating_add(STARTUP_TIMEOUT))
        .ok_or_else(|| io::Error::other("worker deadline exceeds monotonic clock"))?;
    let ready = marker(&paths.reply, "ready");
    let abort = marker(&paths.reply, "abort");
    let fallback = marker(&paths.reply, "failed");
    let mut failure = owner.process.as_ref().err().map(|error| {
        MeasureError::Io(io::Error::other(format!(
            "acquire exact worker handle: {error}"
        )))
    });
    loop {
        match fs::metadata(&ready) {
            Ok(_) => break,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                failure.get_or_insert(MeasureError::Io(error));
            }
        }
        match owner.observe_exit() {
            Ok(Some(status)) => {
                // Ready publication may race the first metadata observation.
                if ready.is_file() || fallback.is_file() {
                    break;
                }
                failure.get_or_insert(MeasureError::Io(io::Error::other(format!(
                    "worker exited before accounting reply: {status}"
                ))));
                break;
            }
            Ok(None) => {}
            Err(error) => {
                failure.get_or_insert(MeasureError::Io(error));
            }
        }
        let user_cancelled = invocation
            .cancellation
            .as_ref()
            .is_some_and(|c| c.load(Ordering::Acquire));
        if Instant::now() >= deadline && !user_cancelled {
            failure.get_or_insert(MeasureError::Io(io::Error::new(
                io::ErrorKind::TimedOut,
                "Windows worker exceeded its startup/measurement deadline",
            )));
        }
        if failure.is_some() || user_cancelled {
            // Do not kill the sole target-parent reaper. Marker failures retry
            // on this executor; the bounded caller can hand off ownership.
            if let Err(error) = fs::write(&abort, []) {
                failure.get_or_insert(MeasureError::Io(error));
            }
        }
        tokio::time::sleep(EXIT_POLL).await;
    }
    // Ready means target accounting AND verified target tree cleanup. A
    // fallback reply reaches here only after exact failed-worker exit; it
    // never authorizes terminating the sole collector before accounting.
    // Worker-only termination is safe on the normal ready path.
    loop {
        match owner.observe_exit() {
            Ok(Some(_)) => break,
            Ok(None) => {}
            Err(error) => {
                failure.get_or_insert(MeasureError::Io(error));
            }
        }
        if let Err(error) = owner.terminate_after_collection() {
            failure.get_or_insert(MeasureError::Io(error));
        }
        tokio::time::sleep(EXIT_POLL).await;
    }
    let fallback_reply = fallback.is_file();
    if fallback_reply && owner.status.is_some_and(|status| status.success()) {
        failure.get_or_insert(MeasureError::Io(io::Error::other(
            "kill-on-close fallback worker did not exit as failed",
        )));
    }
    let _ = fs::remove_file(&abort);
    let _ = fs::remove_file(&ready);
    let _ = fs::remove_file(&fallback);
    if let Some(error) = failure {
        return Err(error);
    }
    let reply = read_bounded(File::open(&paths.reply)?)?;
    match serde_json::from_slice::<Reply>(&reply).map_err(io_error)? {
        Reply::Outcome {
            protocol,
            mut outcome,
        } if protocol == PROTOCOL && !fallback_reply => {
            outcome.measurement_protocol.worker = Some(identity);
            Ok(*outcome)
        }
        Reply::Error {
            protocol,
            message,
            cancelled,
            exit_code,
            wall_time_micros,
        } if protocol == PROTOCOL => Err(MeasureError::Collection {
            source: Box::new(if cancelled {
                MeasureError::Cancelled
            } else {
                MeasureError::Io(io::Error::other(message))
            }),
            exit_code,
            signal: None,
            wall_time_micros,
            stdout_path: paths.stdout.clone(),
            stderr_path: paths.stderr.clone(),
        }),
        _ => Err(MeasureError::Io(io::Error::other(
            "Windows worker protocol mismatch",
        ))),
    }
}

/// Runs the native Windows worker. Replies use files, never target output pipes.
///
/// # Errors
/// Returns a bounded request, coordinator-identity, measurement, or reply error.
pub fn run() -> Result<(), String> {
    let encoded = read_bounded(io::stdin()).map_err(|e| e.to_string())?;
    let request: Request = serde_json::from_slice(&encoded).map_err(|e| e.to_string())?;
    if request.protocol != PROTOCOL {
        return Err("Windows worker protocol mismatch".to_owned());
    }
    let paths = PreparedPaths {
        stdin: from_wide(&request.stdin),
        stdout: from_wide(&request.stdout),
        stderr: from_wide(&request.stderr),
        reply: from_wide(&request.reply),
    };
    let control = WorkerControl::new(&request, &paths.reply).map_err(|e| e.to_string())?;
    let invocation = BenchmarkInvocation {
        cancellation: None,
        executable: from_wide(&request.executable),
        args: request.args,
        stdin: Vec::new(),
        current_dir: request.directory.as_deref().map(from_wide),
        timeout: Duration::from_micros(request.timeout_micros),
        output_limit: request.output_limit,
        rss_limit: request.rss_limit,
        retain_output: false,
    };
    let runtime = runtime().map_err(|e| e.to_string())?;
    let result = runtime.block_on(run_owned_measurement(
        &control,
        measure_target(&invocation, &paths, &control, request.sample_tree),
    ));
    let fallback = control.target_cleanup.load(Ordering::Acquire) == TargetCleanup::Fallback as u8;
    let reply = match result {
        Ok(outcome) => Reply::Outcome {
            protocol: PROTOCOL.to_owned(),
            outcome: Box::new(outcome),
        },
        Err(error) => {
            let (exit_code, wall_time_micros) = match &error {
                MeasureError::Collection {
                    exit_code,
                    wall_time_micros,
                    ..
                } => (*exit_code, *wall_time_micros),
                _ => (None, None),
            };
            Reply::Error {
                protocol: PROTOCOL.to_owned(),
                message: error.to_string().chars().take(4096).collect(),
                cancelled: is_cancelled(&error),
                exit_code,
                wall_time_micros,
            }
        }
    };
    let encoded = serde_json::to_vec(&reply).map_err(|e| e.to_string())?;
    if encoded.len() > MESSAGE_LIMIT {
        return Err("worker reply exceeds protocol cap".to_owned());
    }
    fs::write(&paths.reply, encoded).map_err(|e| e.to_string())?;
    if fallback {
        // Never publish the verified-empty marker. The coordinator reads this
        // error reply only AFTER exact worker exit, retaining its admission slot
        // through the OS final-close backstop for both target jobs.
        fs::write(marker(&paths.reply, "failed"), []).map_err(|e| e.to_string())?;
        return Err("target cleanup required kill-on-close fallback".to_owned());
    }
    fs::write(marker(&paths.reply, "ready"), []).map_err(|e| e.to_string())?;
    // The independent coordinator handle watches death during measurement.
    // Collection, tree cleanup and reply publication are complete: exit even
    // if the coordinator vanished or its lifecycle thread lost ownership.
    Ok(())
}

async fn run_owned_measurement(
    control: &WorkerControl,
    measurement: impl std::future::Future<Output = Result<MeasuredOutcome, MeasureError>>,
) -> Result<MeasuredOutcome, MeasureError> {
    let mut measurement = std::pin::pin!(measurement);
    let mut orphan_deadline = None;
    loop {
        tokio::select! {
            result = &mut measurement => return result,
            () = tokio::time::sleep(Duration::from_millis(25)) => {
                let cleanup = control.target_cleanup.load(Ordering::Acquire);
                if cleanup == TargetCleanup::Fallback as u8
                    || (cleanup == TargetCleanup::Verified as u8
                        && matches!(process_has_exited(&control.coordinator), Ok(true)))
                {
                    let deadline = orphan_deadline.get_or_insert_with(|| Instant::now() + Duration::from_secs(1));
                    if Instant::now() >= *deadline {
                        // Exact-root exit/accounting precedes either verified
                        // tree cleanup or explicit closure of BOTH kill-on-close
                        // job guards. Failure fallback also bounds blocked file
                        // writers while the coordinator is alive. Worker exit
                        // releases OS handles; coordinator admission stays owned
                        // until that exit, never interrupting root collection.
                        std::process::exit(1);
                    }
                }
            }
        }
    }
}

fn is_cancelled(error: &MeasureError) -> bool {
    match error {
        MeasureError::Cancelled => true,
        MeasureError::Collection { source, .. } => is_cancelled(source),
        _ => false,
    }
}
fn runtime() -> io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
}
fn read_bounded(reader: impl io::Read) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take(u64::try_from(MESSAGE_LIMIT + 1).unwrap())
        .read_to_end(&mut bytes)?;
    if bytes.len() > MESSAGE_LIMIT {
        return Err(io::Error::other("worker message exceeds protocol cap"));
    }
    Ok(bytes)
}
fn marker(reply: &Path, suffix: &str) -> PathBuf {
    let mut path = reply.as_os_str().to_owned();
    path.push(format!(".{suffix}"));
    path.into()
}
fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().collect()
}
fn from_wide(path: &[u16]) -> PathBuf {
    OsString::from_wide(path).into()
}
fn hash_bytes(bytes: &[u8]) -> String {
    use sha2::{Digest as _, Sha256};
    const HEX: &[u8; 16] = b"0123456789abcdef";
    Sha256::digest(bytes)
        .iter()
        .flat_map(|byte| {
            [
                char::from(HEX[usize::from(byte >> 4)]),
                char::from(HEX[usize::from(byte & 15)]),
            ]
        })
        .collect()
}

/// Resolves the native `.exe` worker beside the executable or Cargo test output.
///
/// # Errors
/// Returns an error if no worker exists in a recognized output layout.
pub fn worker_executable_path() -> io::Result<PathBuf> {
    if let Some(path) = env::var_os("TQ_BENCH_WORKER") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "TQ_BENCH_WORKER is not a file",
        ));
    }
    let current = env::current_exe()?;
    let directory = current
        .parent()
        .ok_or_else(|| io::Error::other("executable has no directory"))?;
    let direct = directory.join("tq-bench-worker.exe");
    if direct.is_file() {
        return Ok(direct);
    }
    if directory.file_name().is_some_and(|n| n == "deps")
        && let Some(parent) = directory.parent()
    {
        let sibling = parent.join("tq-bench-worker.exe");
        if sibling.is_file() {
            return Ok(sibling);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "native tq-bench-worker.exe not found",
    ))
}

#[cfg(test)]
static FAULT_WORKER: Mutex<Option<(PathBuf, String, PathBuf)>> = Mutex::new(None);

#[cfg(test)]
mod tests {
    use super::*;

    fn registry_test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: Mutex<()> = Mutex::new(());
        LOCK.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    #[test]
    fn deferred_handoff_after_reservation_rejects_worker_and_target_before_spawn() {
        let _test_guard = registry_test_lock();
        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("target.exe");
        fs::copy(
            env::var_os("COMSPEC").expect("native Windows command interpreter"),
            &executable,
        )
        .unwrap();
        let target_marker = directory.path().join("target-launched");
        let invocation = BenchmarkInvocation {
            cancellation: None,
            executable: executable.clone(),
            args: vec![
                "/D".into(),
                "/C".into(),
                format!("echo target > \"{}\"", target_marker.display()),
            ],
            stdin: Vec::new(),
            current_dir: None,
            timeout: Duration::from_secs(1),
            output_limit: 1024,
            rss_limit: None,
            retain_output: true,
        };
        let phase = Arc::new(AtomicU8::new(ExecutorPhase::Running as u8));
        let (cleanup_release, cleanup_wait) = mpsc::channel();
        let slot = EXECUTOR.get_or_init(|| Mutex::new(Vec::new()));
        {
            let mut registry = slot.lock().unwrap();
            reap_finished_executors(&mut registry).unwrap();
            assert!(registry.is_empty());
            registry.push(LifecycleExecutor {
                handle: thread::spawn(move || {
                    let _ = cleanup_wait.recv();
                }),
                phase: Arc::clone(&phase),
            });
        }
        let (reached, barrier) = mpsc::sync_channel(1);
        let (release, wait) = mpsc::channel();
        let spawned = Arc::new(AtomicBool::new(false));
        *LAUNCH_PAUSE.lock().unwrap() = Some(LaunchPause {
            executable,
            reached,
            release: wait,
            spawned: Arc::clone(&spawned),
        });
        let caller = thread::spawn(move || measure_process(&invocation, false));
        barrier.recv_timeout(CLEANUP_TIMEOUT).unwrap();
        // B now owns its production registry reservation, but has not checked
        // final launch admission. A hands off cleanup before B can spawn.
        let reserved = slot.lock().unwrap().len();
        defer_executor(&phase);
        let deferred = phase.load(Ordering::Acquire);
        release.send(()).unwrap();
        let result = caller.join().unwrap();
        phase.store(ExecutorPhase::CleanupComplete as u8, Ordering::Release);
        cleanup_release.send(()).unwrap();
        reap_finished_executors(&mut slot.lock().unwrap()).unwrap();
        if let Ok(outcome) = &result {
            for path in [&outcome.stdout_path, &outcome.stderr_path]
                .into_iter()
                .flatten()
            {
                fs::remove_file(path).unwrap();
            }
        }
        if let Err(MeasureError::Collection {
            stdout_path,
            stderr_path,
            ..
        }) = &result
        {
            fs::remove_file(stdout_path).unwrap();
            fs::remove_file(stderr_path).unwrap();
        }
        assert_eq!(reserved, 2, "B was not reserved before handoff");
        assert_eq!(deferred, ExecutorPhase::Deferred as u8);
        assert!(
            !spawned.load(Ordering::Acquire),
            "worker launched after deferred handoff"
        );
        assert!(
            !target_marker.exists(),
            "target launched after deferred handoff"
        );
        let Err(MeasureError::Collection {
            source,
            exit_code,
            wall_time_micros,
            ..
        }) = result
        else {
            panic!("late deferred admission did not reject B");
        };
        assert!(
            source
                .to_string()
                .contains("cleanup became deferred before worker launch")
        );
        assert!(exit_code.is_none());
        assert!(wall_time_micros.is_none());
    }

    #[test]
    fn persistent_job_error_worker() {
        use super::super::native_process_windows::{JOB_FAULT, JobFault};
        let Ok(fault) = env::var("TQ_BENCH_TEST_JOB_FAULT") else {
            return;
        };
        JOB_FAULT.with(|slot| {
            slot.set(Some(match fault.as_str() {
                "query" => JobFault::Query,
                "terminate" => JobFault::Terminate,
                "observe" => JobFault::ExitObservation,
                "observe-no-accounting" => JobFault::ExitObservationWithoutAccounting,
                _ => panic!("unknown job fault"),
            }));
        });
        let result = run();
        assert!(result.is_err(), "fallback worker must exit as failed");
        let gate = PathBuf::from(env::var_os("TQ_BENCH_TEST_JOB_FAULT_GATE").unwrap());
        fs::write(gate.join("waiting"), std::process::id().to_string()).unwrap();
        while !gate.join("release").is_file() {
            thread::sleep(Duration::from_millis(5));
        }
        std::process::exit(1);
    }

    #[test]
    fn persistent_job_errors_fail_boundedly_and_release_admission_after_worker_exit() {
        let _test_guard = registry_test_lock();
        for fault in ["query", "terminate", "observe", "observe-no-accounting"] {
            persistent_job_error_case(fault);
        }
    }

    fn persistent_job_error_case(fault: &str) {
        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("target.exe");
        fs::copy(env::var_os("COMSPEC").unwrap(), &executable).unwrap();
        let invocation = BenchmarkInvocation {
            cancellation: None,
            executable: executable.clone(),
            args: vec![
                "/D".into(),
                "/C".into(),
                if fault == "query" {
                    "exit 17".into()
                } else {
                    "ping -n 30 127.0.0.1 > nul".into()
                },
            ],
            stdin: Vec::new(),
            current_dir: None,
            timeout: Duration::from_millis(200),
            output_limit: 1024,
            rss_limit: None,
            retain_output: false,
        };
        *FAULT_WORKER.lock().unwrap() =
            Some((executable, fault.to_owned(), directory.path().to_owned()));
        let healthy = BenchmarkInvocation {
            executable: env::var_os("COMSPEC").unwrap().into(),
            args: vec!["/D".into(), "/C".into(), "exit 0".into()],
            timeout: Duration::from_secs(1),
            cancellation: None,
            stdin: Vec::new(),
            current_dir: None,
            output_limit: 1024,
            rss_limit: None,
            retain_output: false,
        };
        let started = Instant::now();
        let result = assert_fallback_keeps_admission(&invocation, &healthy, directory.path());
        *FAULT_WORKER.lock().unwrap() = None;
        let Err(MeasureError::Collection {
            source,
            exit_code,
            wall_time_micros,
            stdout_path,
            stderr_path,
            ..
        }) = result
        else {
            panic!("persistent job error returned success");
        };
        let message = source.to_string();
        let _ = fs::remove_file(stdout_path);
        let _ = fs::remove_file(stderr_path);
        let expected = if fault == "observe-no-accounting" {
            "injected accounting access failure"
        } else {
            "injected persistent job"
        };
        assert!(message.contains(expected), "{message}");
        assert!(exit_code.is_some(), "lost exact-root exit: {message}");
        assert!(wall_time_micros.is_some());
        assert!(started.elapsed() < Duration::from_secs(4));
        assert_eq!(measure_process(&healthy, false).unwrap().exit_code, Some(0));
    }

    struct ReleaseFallback(PathBuf);
    impl Drop for ReleaseFallback {
        fn drop(&mut self) {
            let _ = fs::write(self.0.join("release"), []);
        }
    }

    fn assert_fallback_keeps_admission(
        invocation: &BenchmarkInvocation,
        healthy: &BenchmarkInvocation,
        gate: &Path,
    ) -> Result<MeasuredOutcome, MeasureError> {
        thread::scope(|scope| {
            let release = ReleaseFallback(gate.to_owned());
            let (send, receive) = mpsc::channel();
            scope.spawn(move || send.send(measure_process(invocation, false)).unwrap());
            let deadline = Instant::now() + Duration::from_secs(3);
            while !gate.join("waiting").is_file() {
                assert!(Instant::now() < deadline, "fallback worker did not collect");
                thread::sleep(Duration::from_millis(5));
            }
            let pid = fs::read_to_string(gate.join("waiting"))
                .unwrap()
                .parse()
                .unwrap();
            let worker = open_process(pid).unwrap();
            assert!(!process_has_exited(&worker).unwrap());
            assert!(matches!(receive.try_recv(), Err(mpsc::TryRecvError::Empty)));
            let phase = {
                let registry = EXECUTOR.get().unwrap().lock().unwrap();
                assert_eq!(registry.len(), 1, "fallback lost its owned slot");
                Arc::clone(&registry[0].phase)
            };
            // Exercise the actual deadline handoff while the failed collector
            // is deliberately kept alive after accounting and job guard close.
            defer_executor(&phase);
            assert!(
                measure_process(healthy, false).is_err(),
                "admitted before failed worker exit"
            );
            drop(release);
            let result = receive.recv_timeout(Duration::from_secs(2)).unwrap();
            assert!(process_has_exited(&worker).unwrap());
            result
        })
    }

    struct TestChild(std::process::Child);
    impl std::ops::Deref for TestChild {
        type Target = std::process::Child;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl std::ops::DerefMut for TestChild {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
    impl Drop for TestChild {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    #[test]
    fn orphaned_blocked_writer_helper() {
        let Some(parent_pid) = env::var_os("TQ_BENCH_TEST_ORPHAN_PARENT") else {
            return;
        };
        let parent_pid: u32 = parent_pid.to_str().unwrap().parse().unwrap();
        let directory = PathBuf::from(env::var_os("TQ_BENCH_TEST_ORPHAN_DIRECTORY").unwrap());
        let control = WorkerControl {
            coordinator: open_process(parent_pid).unwrap(),
            abort: directory.join("abort"),
            target_cleanup: AtomicU8::new(TargetCleanup::Pending as u8),
        };
        let (created, _, _, _) = control.coordinator.GetProcessTimes().unwrap();
        assert_eq!(
            u64::from(created).to_string(),
            env::var("TQ_BENCH_TEST_ORPHAN_CREATED").unwrap()
        );
        if let Ok(fault) = env::var("TQ_BENCH_TEST_BLOCKED_JOB_FAULT") {
            use super::super::native_process_windows::{JOB_FAULT, JobFault};
            JOB_FAULT.with(|slot| {
                slot.set(Some(match fault.as_str() {
                    "query" => JobFault::Query,
                    "terminate" => JobFault::Terminate,
                    _ => panic!("unknown job fault"),
                }));
            });
        }
        let paths = PreparedPaths {
            stdin: directory.join("stdin"),
            stdout: directory.join("stdout"),
            stderr: directory.join("stderr"),
            reply: directory.join("reply"),
        };
        File::create(&paths.stdin).unwrap();
        let target_pid = directory
            .join("target-pid")
            .display()
            .to_string()
            .replace('\'', "''");
        let invocation = BenchmarkInvocation {
            cancellation: None,
            executable: "powershell.exe".into(),
            args: vec![
                "-NoProfile".into(),
                "-Command".into(),
                format!(
                    "Set-Content -LiteralPath '{target_pid}' -Value $PID; Write-Output blocked; Start-Sleep -Seconds 30"
                ),
            ],
            stdin: Vec::new(),
            current_dir: None,
            timeout: Duration::from_secs(20),
            output_limit: 1024,
            rss_limit: None,
            retain_output: true,
        };
        let entered = Arc::new(AtomicBool::new(false));
        let (_release, receiver) = mpsc::channel();
        let sink = super::super::capture_windows::CaptureSink::Blocked {
            file: File::create(&paths.stdout).unwrap(),
            entered: Arc::clone(&entered),
            release: receiver,
        };
        let stderr =
            super::super::capture_windows::CaptureSink::File(File::create(&paths.stderr).unwrap());
        runtime().unwrap().block_on(async {
            let measurement = super::super::measure_windows::measure_target_with_sinks(
                &invocation, &paths, &control, false, sink, stderr,
            );
            tokio::select! {
                result = run_owned_measurement(&control, measurement) => panic!("blocked writer unexpectedly completed: {result:?}"),
                () = async {
                    while !entered.load(Ordering::Acquire) {
                        tokio::time::sleep(EXIT_POLL).await;
                    }
                    fs::write(directory.join("writer-entered"), []).unwrap();
                    std::future::pending::<()>().await;
                } => unreachable!(),
            }
        });
    }

    #[test]
    fn coordinator_death_with_blocked_writer_exits_only_after_target_cleanup() {
        blocked_writer_watchdog_case(None);
    }

    #[test]
    fn persistent_job_errors_with_blocked_writer_fail_closed_with_live_coordinator() {
        for fault in ["query", "terminate"] {
            blocked_writer_watchdog_case(Some(fault));
        }
    }

    fn blocked_writer_watchdog_case(fault: Option<&str>) {
        let directory = tempfile::tempdir().unwrap();
        let mut coordinator = TestChild(
            std::process::Command::new("powershell.exe")
                .args(["-NoProfile", "-Command", "Start-Sleep -Seconds 30"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let parent = open_process(coordinator.id()).unwrap();
        let (created, _, _, _) = parent.GetProcessTimes().unwrap();
        let mut worker = TestChild(
            std::process::Command::new(env::current_exe().unwrap())
                .args([
                    "--exact",
                    "benchmark::worker::tests::orphaned_blocked_writer_helper",
                    "--nocapture",
                ])
                .env("TQ_BENCH_TEST_ORPHAN_PARENT", coordinator.id().to_string())
                .env(
                    "TQ_BENCH_TEST_ORPHAN_CREATED",
                    u64::from(created).to_string(),
                )
                .env("TQ_BENCH_TEST_ORPHAN_DIRECTORY", directory.path())
                .envs(fault.map(|fault| ("TQ_BENCH_TEST_BLOCKED_JOB_FAULT", fault)))
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        while !directory.path().join("writer-entered").is_file() {
            assert!(
                Instant::now() < deadline,
                "worker file write was never blocked"
            );
            assert!(
                worker.try_wait().unwrap().is_none(),
                "worker exited before blocking"
            );
            thread::sleep(Duration::from_millis(5));
        }
        let target_pid: u32 = fs::read_to_string(directory.path().join("target-pid"))
            .unwrap()
            .trim()
            .trim_start_matches('\u{feff}')
            .parse()
            .unwrap();
        let target = open_process(target_pid).unwrap();
        if fault.is_some() {
            fs::write(directory.path().join("abort"), []).unwrap();
        } else {
            coordinator.kill().unwrap();
            coordinator.wait().unwrap();
        }
        while !process_has_exited(&target).unwrap() {
            assert!(
                Instant::now() < deadline,
                "writer prevented coordinator-death target cleanup"
            );
            assert!(
                worker.try_wait().unwrap().is_none(),
                "worker died before target cleanup"
            );
            thread::sleep(Duration::from_millis(5));
        }
        assert!(
            worker.try_wait().unwrap().is_none(),
            "worker exited before post-collection grace"
        );
        let status = loop {
            if let Some(status) = worker.try_wait().unwrap() {
                break status;
            }
            assert!(
                Instant::now() < deadline,
                "blocked writer left an orphan worker"
            );
            thread::sleep(Duration::from_millis(5));
        };
        assert_eq!(status.code(), Some(1), "watchdog did not fail closed");
        if fault.is_some() {
            assert!(!process_has_exited(&parent).unwrap());
        }
        assert!(
            !directory.path().join("reply.ready").is_file(),
            "blocked capture published success"
        );
    }
    #[test]
    fn coordinator_death_helper() {
        let Some(directory) = env::var_os("TQ_BENCH_TEST_COORDINATOR_DEATH") else {
            return;
        };
        let directory = PathBuf::from(directory);
        let paths = PreparedPaths {
            stdin: directory.join("stdin"),
            stdout: directory.join("stdout"),
            stderr: directory.join("stderr"),
            reply: directory.join("reply"),
        };
        let target_pid = directory
            .join("target-pid")
            .display()
            .to_string()
            .replace('\'', "''");
        let descendant_pid = directory
            .join("descendant-pid")
            .display()
            .to_string()
            .replace('\'', "''");
        let invocation = BenchmarkInvocation {
            cancellation: None,
            executable: "powershell.exe".into(),
            args: vec![
                "-NoProfile".into(),
                "-Command".into(),
                format!(
                    "$child = Start-Process powershell.exe -ArgumentList '-NoProfile -Command Start-Sleep -Seconds 30' -PassThru; Set-Content -LiteralPath '{descendant_pid}' -Value $child.Id; Set-Content -LiteralPath '{target_pid}' -Value $PID; Start-Sleep -Seconds 30"
                ),
            ],
            stdin: Vec::new(),
            current_dir: None,
            timeout: Duration::from_secs(20),
            output_limit: 1024,
            rss_limit: None,
            retain_output: true,
        };
        let _ = runtime()
            .unwrap()
            .block_on(coordinate(&invocation, false, &paths));
    }

    #[test]
    fn coordinator_death_preserves_worker_collection_then_exits_without_orphans() {
        lifecycle_death_case(true);
    }

    #[test]
    fn unexpected_worker_death_closes_target_jobs_without_orphans() {
        lifecycle_death_case(false);
    }

    fn lifecycle_death_case(kill_coordinator: bool) {
        let directory = tempfile::tempdir().unwrap();
        for name in ["stdin", "stdout", "stderr", "reply"] {
            File::create(directory.path().join(name)).unwrap();
        }
        let mut coordinator = TestChild(
            std::process::Command::new(env::current_exe().unwrap())
                .args([
                    "--exact",
                    "benchmark::worker::tests::coordinator_death_helper",
                    "--nocapture",
                ])
                .env("TQ_BENCH_TEST_COORDINATOR_DEATH", directory.path())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        let read_pid = |name: &str| loop {
            if let Ok(text) = fs::read_to_string(directory.path().join(name))
                && let Ok(pid) = text.trim().trim_start_matches('\u{feff}').parse::<u32>()
            {
                break pid;
            }
            assert!(Instant::now() < deadline, "missing {name}");
            thread::sleep(Duration::from_millis(5));
        };
        let worker_pid = read_pid("reply.worker-pid");
        let worker = open_process(worker_pid).unwrap();
        let target = open_process(read_pid("target-pid")).unwrap();
        let descendant = open_process(read_pid("descendant-pid")).unwrap();
        if kill_coordinator {
            coordinator.kill().unwrap();
        } else {
            // The already retained query handle pins this same identity while
            // acquiring the additional terminate right for the test injection.
            let terminate =
                HPROCESS::OpenProcess(winsafe::co::PROCESS::TERMINATE, false, worker_pid).unwrap();
            terminate.TerminateProcess(1).unwrap();
        }
        while coordinator.try_wait().unwrap().is_none() {
            assert!(
                Instant::now() < deadline,
                "coordinator failed to finish after worker death"
            );
            thread::sleep(Duration::from_millis(5));
        }
        while !process_has_exited(&worker).unwrap()
            || !process_has_exited(&target).unwrap()
            || !process_has_exited(&descendant).unwrap()
        {
            assert!(
                Instant::now() < deadline,
                "worker orphaned after coordinator death"
            );
            thread::sleep(Duration::from_millis(5));
        }
        assert!(
            process_has_exited(&target).unwrap(),
            "target survived worker"
        );
        assert!(
            process_has_exited(&descendant).unwrap(),
            "descendant survived worker"
        );
        if !kill_coordinator {
            assert!(
                !directory.path().join("reply.ready").is_file(),
                "dead collector published success"
            );
            return;
        }
        let reply = read_bounded(File::open(directory.path().join("reply")).unwrap()).unwrap();
        let Reply::Error {
            cancelled,
            exit_code,
            wall_time_micros,
            ..
        } = serde_json::from_slice(&reply).expect("worker must collect and publish before exiting")
        else {
            panic!("coordinator death was not cancellation")
        };
        assert!(cancelled);
        assert!(exit_code.is_some(), "missing exact target exit");
        assert!(wall_time_micros.is_some(), "missing accounting boundary");
        assert!(directory.path().join("reply.ready").is_file());
    }

    #[test]
    fn wire_paths_preserve_unpaired_utf16_and_spaces() {
        let encoded = [
            u16::from(b'C'),
            u16::from(b':'),
            u16::from(b'\\'),
            u16::from(b' '),
            0xd800,
        ];
        assert_eq!(wide(&from_wide(&encoded)), encoded);
    }
    #[test]
    fn reply_round_trip_preserves_u128_wall_time() {
        let reply = Reply::Error {
            protocol: PROTOCOL.to_owned(),
            message: "collection failed".to_owned(),
            cancelled: false,
            exit_code: Some(17),
            wall_time_micros: Some(u128::MAX),
        };
        let bytes = serde_json::to_vec(&reply).unwrap();
        let Reply::Error {
            wall_time_micros,
            exit_code,
            ..
        } = serde_json::from_slice(&bytes).unwrap()
        else {
            panic!("wrong reply variant");
        };
        assert_eq!(wall_time_micros, Some(u128::MAX));
        assert_eq!(exit_code, Some(17));
    }

    #[test]
    fn deferred_cleanup_blocks_admission_but_healthy_workers_can_coexist() {
        let mut registry = Vec::new();
        let mut releases = Vec::new();
        for _ in 0..2 {
            let (sender, receiver) = mpsc::channel();
            let handle = thread::spawn(move || {
                receiver.recv().unwrap();
            });
            registry.push(LifecycleExecutor {
                handle,
                phase: Arc::new(AtomicU8::new(ExecutorPhase::Running as u8)),
            });
            releases.push(sender);
        }
        assert!(reap_finished_executors(&mut registry).is_ok());
        assert_eq!(registry.len(), 2);
        registry[0]
            .phase
            .store(ExecutorPhase::Deferred as u8, Ordering::Release);
        assert!(reap_finished_executors(&mut registry).is_err());
        assert_eq!(registry.len(), 2);
        for executor in &registry {
            executor
                .phase
                .store(ExecutorPhase::CleanupComplete as u8, Ordering::Release);
        }
        for release in releases {
            release.send(()).unwrap();
        }
        reap_finished_executors(&mut registry).unwrap();
        assert!(registry.is_empty());
    }

    #[test]
    fn full_executor_registry_rejects_before_launch_without_expanding_capacity() {
        let _test_guard = registry_test_lock();
        let slot = EXECUTOR.get_or_init(|| Mutex::new(Vec::new()));
        let mut registry = slot.lock().unwrap();
        assert!(registry.is_empty());
        let mut releases = Vec::new();
        for _ in 0..MAX_IN_FLIGHT_WORKERS {
            let (sender, receiver) = mpsc::channel();
            registry.push(LifecycleExecutor {
                handle: thread::spawn(move || receiver.recv().unwrap()),
                phase: Arc::new(AtomicU8::new(ExecutorPhase::Running as u8)),
            });
            releases.push(sender);
        }
        drop(registry);
        let invocation = BenchmarkInvocation {
            cancellation: None,
            executable: "cmd.exe".into(),
            args: vec!["/D".into(), "/C".into(), "exit 0".into()],
            stdin: Vec::new(),
            current_dir: None,
            timeout: Duration::from_secs(1),
            output_limit: 1024,
            rss_limit: None,
            retain_output: false,
        };
        let result = measure_process(&invocation, false);
        let mut registry = slot.lock().unwrap();
        let count = registry.len();
        for release in releases {
            release.send(()).unwrap();
        }
        for executor in registry.drain(..) {
            executor.handle.join().unwrap();
        }
        drop(registry);
        assert_eq!(count, MAX_IN_FLIGHT_WORKERS);
        let Err(MeasureError::Io(error)) = result else {
            panic!("full registry did not reject launch: {result:?}");
        };
        assert!(
            error
                .to_string()
                .contains("capacity unavailable before launch")
        );
    }

    #[test]
    fn messages_remain_bounded() {
        assert!(read_bounded(io::Cursor::new(vec![0; MESSAGE_LIMIT + 1])).is_err());
        assert_eq!(
            read_bounded(io::Cursor::new(vec![0; MESSAGE_LIMIT]))
                .unwrap()
                .len(),
            MESSAGE_LIMIT
        );
    }
}
