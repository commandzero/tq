//! Isolated measurement worker protocol.
//!
//! The coordinator owns prepared capture files and sends only bounded command
//! metadata to a fresh worker image. The worker owns the target's exact-child
//! wait and writes one bounded reply file before it exits.

use std::{
    env,
    fs::{self, OpenOptions},
    io::{self, Read as _, Write as _},
    os::unix::ffi::{OsStrExt as _, OsStringExt as _},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use nix::unistd::Pid;
use serde::{Deserialize, Serialize};

use super::{
    BenchmarkInvocation, MeasureError, MeasuredOutcome, MeasuredStatus, collector_source_sha256,
    measure::{PreparedCapturePaths, measure_prepared, prepare_capture_files},
    native_process::{EXIT_POLL, NativeChild},
    report::WorkerIdentity,
};

const PROTOCOL_VERSION: u8 = 1;
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
) -> Result<MeasuredOutcome, MeasureError> {
    if invocation.rss_limit.is_some() {
        return Err(MeasureError::Unsupported(
            "isolated worker prototype does not support sampled RSS limits".to_owned(),
        ));
    }
    let captures = prepare_capture_files(invocation)?;
    let paths = captures.paths();
    let worker_path = worker_executable().map_err(MeasureError::Io)?;
    let worker_identity = worker_identity(&worker_path).map_err(MeasureError::Io)?;
    let worker_result = run_worker(invocation, &paths, &worker_path);
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
            Err(collection_error(
                format!("isolated worker failed: {}", failure.message),
                failure.exit_code,
                failure.signal,
                failure.wall_time_micros,
                stdout,
                stderr,
            ))
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

fn run_worker(
    invocation: &BenchmarkInvocation,
    paths: &PreparedCapturePaths,
    worker_path: &Path,
) -> Result<WorkerReply, WorkerFailure> {
    let request = WorkerRequest::from_invocation(invocation, paths).map_err(io_failure)?;
    let encoded = serde_json::to_vec(&request).map_err(|error| io_failure(error.to_string()))?;
    if encoded.len() > MAX_REQUEST_BYTES {
        return Err(io_failure(format!(
            "worker request exceeds {MAX_REQUEST_BYTES} bytes"
        )));
    }
    let mut command = Command::new(worker_path);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        command.process_group(0);
    }
    let mut child = command
        .spawn()
        .map_err(|error| io_failure(error.to_string()))?;
    let request_pipe = child
        .stdin
        .take()
        .ok_or_else(|| io_failure("isolated worker stdin was not piped".to_owned()))?;
    // A pipe write can block if a malformed/stale worker never reads stdin.
    // Keep it off the coordinator thread so the bounded lifecycle deadline
    // remains effective. Killing the worker closes the pipe and releases this
    // writer before its result is joined below.
    let encoded_for_writer = encoded;
    let request_writer = thread::spawn(move || {
        let mut request_pipe = request_pipe;
        request_pipe
            .write_all(&encoded_for_writer)
            .and_then(|()| request_pipe.write_all(b"\n"))
    });

    let mut owner = NativeChild::new(child);
    let deadline = Instant::now()
        .checked_add(invocation.timeout.saturating_add(WORKER_STARTUP_TIMEOUT))
        .unwrap_or_else(|| Instant::now() + WORKER_STARTUP_TIMEOUT);
    loop {
        match owner.observe_exit() {
            Ok(true) => break,
            Ok(false) if cancellation_requested(invocation) => {
                terminate_worker(&mut owner).map_err(|error| io_failure(error.to_string()))?;
                let _ = request_writer.join();
                return Err(WorkerFailure {
                    message: "worker cancelled before reply".to_owned(),
                    exit_code: None,
                    signal: None,
                    wall_time_micros: None,
                });
            }
            Ok(false) if Instant::now() >= deadline => {
                terminate_worker(&mut owner).map_err(|error| io_failure(error.to_string()))?;
                let _ = request_writer.join();
                return Err(WorkerFailure {
                    message: "worker did not exit before its bounded deadline".to_owned(),
                    exit_code: None,
                    signal: None,
                    wall_time_micros: None,
                });
            }
            Ok(false) => thread::sleep(EXIT_POLL),
            Err(error) => return Err(io_failure(format!("observe worker exit: {error}"))),
        }
    }
    let status = owner.observed_status();
    let finish_result = owner.finish();
    // Clean up and reap the worker group before joining the bounded request
    // writer. A malformed worker descendant can inherit stdin; joining first
    // would then wait forever for a pipe that group cleanup is meant to close.
    if let Err(error) = finish_result {
        drop(request_writer);
        return Err(io_failure(format!("cleanup and reap worker: {error}")));
    }
    let request_result = request_writer
        .join()
        .map_err(|_| io_failure("worker request writer panicked".to_owned()))?;
    if let Err(error) = request_result {
        return Err(io_failure(format!("send worker request: {error}")));
    }
    if status != (Some(0), None) {
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
    read_reply(&paths.reply).map_err(|error| io_failure(format!("read worker reply: {error}")))
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
        thread::sleep(EXIT_POLL);
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
        })
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
    let group = Pid::from_raw(
        i32::try_from(std::process::id())
            .map_err(|_| "worker PID does not fit process-group identifier".to_owned())?,
    );
    let result = measure_prepared(&invocation, false, Some(group), &paths, || {}, || {});
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
    write_reply(&reply_path, &reply)
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
    use super::{PROTOCOL_VERSION, WorkerReply, WorkerResult};

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
}
