//! Windows capture owns overlapped pipes and a kill-on-close Job Object.
//!
//! Never call the wrapper's `wait`: it may spawn a blocking Job completion
//! waiter. Use its nonblocking root observation, terminate the Job even on natural
//! root exit, and drain cancellable pipe reads within a separate cleanup bound.

use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io,
    os::windows::fs::OpenOptionsExt as _,
    process::Stdio,
    time::{Duration, Instant},
};

use process_wrap::tokio::{ChildWrapper, CommandWrap, JobObject, KillOnDrop};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::windows::named_pipe::{NamedPipeServer, PipeMode, ServerOptions},
    process::Command,
};

use super::{
    BoundedProcessOutcome, Invocation, OutputLimit, OutputStream, ProcessError, ProcessOutcome,
    ProcessStatus, redact,
};

const POLL_INTERVAL: Duration = Duration::from_millis(10);
const CLEANUP_TIMEOUT: Duration = Duration::from_millis(250);
const CHUNK: usize = 8192;

pub(super) fn run(
    invocation: &Invocation,
    environment: &BTreeMap<String, String>,
    output_limit: Option<u64>,
) -> Result<BoundedProcessOutcome, ProcessError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    // The public API is synchronous, but can also be called by async harnesses.
    // A scoped owner avoids nesting Runtime::block_on in their executor.
    if tokio::runtime::Handle::try_current().is_ok() {
        std::thread::scope(|scope| {
            scope
                .spawn(move || runtime.block_on(capture(invocation, environment, output_limit)))
                .join()
                .map_err(|_| ProcessError::CaptureWorker)?
        })
    } else {
        runtime.block_on(capture(invocation, environment, output_limit))
    }
}

struct CapturedStream {
    bytes: Vec<u8>,
    stream: OutputStream,
    limit: Option<u64>,
    exceeded: Option<OutputLimit>,
    eof: bool,
}

impl CapturedStream {
    fn new(stream: OutputStream, limit: Option<u64>) -> Self {
        Self {
            bytes: Vec::new(),
            stream,
            limit,
            exceeded: None,
            eof: false,
        }
    }

    fn read_len(&self) -> usize {
        self.limit.map_or(CHUNK, |limit| {
            let remaining = limit.saturating_sub(self.bytes.len() as u64);
            usize::try_from(remaining)
                .unwrap_or(usize::MAX)
                .min(CHUNK - 1)
                + 1
        })
    }

    fn accept(&mut self, buffer: &[u8]) {
        if buffer.is_empty() {
            self.eof = true;
            return;
        }
        let retained = self.limit.map_or(buffer.len(), |limit| {
            usize::try_from(limit.saturating_sub(self.bytes.len() as u64))
                .unwrap_or(usize::MAX)
                .min(buffer.len())
        });
        self.bytes.extend_from_slice(&buffer[..retained]);
        if retained < buffer.len() {
            self.exceeded = self.limit.map(|limit| OutputLimit {
                stream: self.stream,
                limit,
                observed_bytes: limit.saturating_add(1),
            });
        }
    }

    fn accept_read(&mut self, result: io::Result<usize>, buffer: &[u8]) -> io::Result<()> {
        match result {
            Ok(count) => self.accept(&buffer[..count]),
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => self.eof = true,
            Err(error) => return Err(error),
        }
        Ok(())
    }

    fn done(&self) -> bool {
        self.eof || self.exceeded.is_some()
    }
}

fn isolated_command(
    invocation: &Invocation,
    environment: &BTreeMap<String, String>,
    files: ChildStdioFiles,
) -> CommandWrap {
    let mut command = Command::new(&invocation.executable);
    command
        .args(&invocation.args)
        .envs(&invocation.environment)
        .envs(environment)
        .stdin(Stdio::from(files.stdin))
        .stdout(Stdio::from(files.stdout))
        .stderr(Stdio::from(files.stderr));
    if let Some(directory) = &invocation.current_dir {
        command.current_dir(directory);
    }
    let mut command = CommandWrap::from(command);
    // KillOnDrop is also consumed by JobObject when configuring kill-on-close.
    // JobObject suspends the root until assignment, closing the descendant race.
    command.wrap(KillOnDrop).wrap(JobObject);
    command
}

/// Synchronous client handles to move into the child's standard streams.
/// Never wrap these files in Tokio I/O: only the server handles are overlapped.
pub(crate) struct ChildStdioFiles {
    pub(crate) stdin: File,
    pub(crate) stdout: File,
    pub(crate) stderr: File,
}

/// Connected overlapped servers. Dropping them cancels kernel I/O rather than
/// leaving Tokio process-stdio blocking tasks waiting for an external client.
pub(crate) struct CapturePipes {
    pub(crate) stdin: Option<CapturePipe>,
    pub(crate) stdout: CapturePipe,
    pub(crate) stderr: CapturePipe,
    // Reserve a randomized namespace for the full lifetime of these servers.
    _namespace: tempfile::TempDir,
}

impl CapturePipes {
    /// Creates local, single-instance byte pipes and preconnects their clients.
    /// Call inside an I/O-enabled Tokio runtime, before spawning the child.
    pub(crate) async fn new() -> io::Result<(Self, ChildStdioFiles)> {
        let namespace = tempfile::Builder::new()
            .prefix("tq-capture-")
            .rand_bytes(24)
            .tempdir()?;
        let name = namespace
            .path()
            .file_name()
            .ok_or_else(|| io::Error::other("missing pipe namespace"))?
            .to_string_lossy();
        let (stdin, stdin_file) =
            stdio_pipe(&format!(r"\\.\pipe\{name}-stdin"), PipeDirection::ToChild).await?;
        let (stdout, stdout_file) = stdio_pipe(
            &format!(r"\\.\pipe\{name}-stdout"),
            PipeDirection::FromChild,
        )
        .await?;
        let (stderr, stderr_file) = stdio_pipe(
            &format!(r"\\.\pipe\{name}-stderr"),
            PipeDirection::FromChild,
        )
        .await?;
        Ok((
            Self {
                stdin: Some(stdin),
                stdout,
                stderr,
                _namespace: namespace,
            },
            ChildStdioFiles {
                stdin: stdin_file,
                stdout: stdout_file,
                stderr: stderr_file,
            },
        ))
    }
}

/// Abortive drop disconnects clients so Mio's deliberately retained pending
/// writes cannot keep a kernel pipe alive after a deadline or cancelled future.
pub(crate) struct CapturePipe {
    pub(crate) server: NamedPipeServer,
    disconnect_on_drop: bool,
}

impl Drop for CapturePipe {
    fn drop(&mut self) {
        if self.disconnect_on_drop {
            let _ = self.server.disconnect();
        }
    }
}

#[derive(Clone, Copy)]
enum PipeDirection {
    ToChild,
    FromChild,
}

async fn stdio_pipe(name: &str, direction: PipeDirection) -> io::Result<(CapturePipe, File)> {
    const SECURITY_IDENTIFICATION: u32 = 0x0001_0000;
    let buffer_size = u32::try_from(CHUNK).expect("fixed pipe buffer size fits u32");
    let to_child = matches!(direction, PipeDirection::ToChild);
    let server = ServerOptions::new()
        .pipe_mode(PipeMode::Byte)
        .first_pipe_instance(true)
        .reject_remote_clients(true)
        .max_instances(1)
        .access_inbound(!to_child)
        .access_outbound(to_child)
        .in_buffer_size(buffer_size)
        .out_buffer_size(buffer_size)
        .create(name)?;
    // ServerOptions::create uses FILE_FLAG_OVERLAPPED. Its safe API does not
    // accept a custom DACL: retain the default DACL, restrict directions, reject
    // remote clients, and preconnect the sole client before exposing child data.
    // A collision or competing client fails closed; never retry a busy name.
    // SECURITY_IDENTIFICATION prevents the server from impersonating its client.
    let client = OpenOptions::new()
        .read(to_child)
        .write(!to_child)
        .security_qos_flags(SECURITY_IDENTIFICATION)
        .open(name)?;
    server.connect().await?;
    Ok((
        CapturePipe {
            server,
            disconnect_on_drop: true,
        },
        client,
    ))
}

async fn spawn_capture(
    invocation: &Invocation,
    environment: &BTreeMap<String, String>,
) -> io::Result<(Box<dyn ChildWrapper>, CapturePipes)> {
    let (pipes, files) = CapturePipes::new().await?;
    let mut command = isolated_command(invocation, environment, files);
    let child = command.spawn()?;
    // Command retains the original stdio Files after Windows duplicates them
    // into the child. Drop those parent client handles before capture/EOF checks.
    drop(command);
    Ok((child, pipes))
}

/// Bounded-step stdin delivery with graceful EOF and abortive cancellation.
pub(crate) struct StdinWriter<'input> {
    pipe: Option<CapturePipe>,
    bytes: &'input [u8],
    written: usize,
}

impl<'input> StdinWriter<'input> {
    pub(crate) fn new(pipe: Option<CapturePipe>, bytes: &'input [u8]) -> Self {
        let mut writer = Self {
            pipe,
            bytes,
            written: 0,
        };
        if bytes.is_empty() {
            writer.finish();
        }
        writer
    }

    fn finish(&mut self) {
        if let Some(mut pipe) = self.pipe.take() {
            // Normal EOF must not disconnect and discard buffered stdin bytes.
            pipe.disconnect_on_drop = false;
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        self.pipe.is_some()
    }

    pub(crate) fn abort(&mut self) {
        self.pipe.take();
    }

    pub(crate) async fn write_next(&mut self) -> io::Result<()> {
        let pipe = self
            .pipe
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "stdin pipe closed"))?;
        let end = self.written.saturating_add(CHUNK).min(self.bytes.len());
        // Mio reports enqueued writes before completion; Tokio's flush is a
        // no-op. A final zero-byte write waits behind the preceding write without
        // appending bytes, so normal EOF does not abandon an incomplete write.
        let result = pipe.server.write(&self.bytes[self.written..end]).await;
        self.accept_write(result)
    }

    fn accept_write(&mut self, result: io::Result<usize>) -> io::Result<()> {
        match result {
            Ok(0) if self.written == self.bytes.len() => self.finish(),
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "stdin pipe closed",
                ));
            }
            Ok(count) => self.written += count,
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => {
                self.pipe.take();
            }
            Err(error) => return Err(error),
        }
        Ok(())
    }
}

async fn capture(
    invocation: &Invocation,
    environment: &BTreeMap<String, String>,
    output_limit: Option<u64>,
) -> Result<BoundedProcessOutcome, ProcessError> {
    let started = Instant::now();
    let (child, pipes) = spawn_capture(invocation, environment).await?;
    capture_child(invocation, output_limit, child, pipes, started).await
}

async fn capture_child(
    invocation: &Invocation,
    output_limit: Option<u64>,
    mut child: Box<dyn ChildWrapper>,
    pipes: CapturePipes,
    started: Instant,
) -> Result<BoundedProcessOutcome, ProcessError> {
    // Ownership stays here across I/O errors. Dropping this future closes the
    // Job and overlapped servers even when an outside process retains clients.
    let CapturePipes {
        stdin,
        mut stdout,
        mut stderr,
        _namespace,
    } = pipes;

    let mut stdin = StdinWriter::new(stdin, &invocation.stdin);
    let mut out = CapturedStream::new(OutputStream::Stdout, output_limit);
    let mut err = CapturedStream::new(OutputStream::Stderr, output_limit);
    // Keep the reusable I/O storage off the future's stack. In particular,
    // cancellation tests and async callers should not embed 16 KiB per capture.
    let mut out_buffer = Box::new([0; CHUNK]);
    let mut err_buffer = Box::new([0; CHUNK]);
    let mut exit = None;
    let mut timed_out = false;
    let mut cleanup_started = None;

    loop {
        // A retained process handle makes native try_wait safe even after root
        // exit; cleanup addresses the Job handle, never a potentially reused PID.
        if exit.is_none() {
            // JobObject::try_wait polls its completion port with a zero timeout
            // and returns the root's status, unlike its whole-Job async wait.
            exit = child.try_wait()?;
        }
        if cleanup_started.is_none() {
            let limited = out.exceeded.is_some() || err.exceeded.is_some();
            timed_out = exit.is_none() && !limited && started.elapsed() >= invocation.timeout;
            if exit.is_some() || limited || timed_out {
                child.start_kill()?;
                cleanup_started = Some(Instant::now());
                // Cancels the overlapped server write. Do not substitute Tokio
                // ChildStdin here: on Windows it uses Blocking<ArcFile>.
                stdin.abort();
            }
        }
        if exit.is_some() && out.done() && err.done() {
            break;
        }
        if cleanup_started.is_some_and(|cleanup: Instant| cleanup.elapsed() >= CLEANUP_TIMEOUT) {
            if exit.is_none() || (!timed_out && out.exceeded.is_none() && err.exceeded.is_none()) {
                return Err(ProcessError::CaptureTimeout);
            }
            // A deadline/limit observation already declares incomplete output.
            // Preserve its prefix even when outside-Job clients retain pipes.
            break;
        }

        let out_len = out.read_len();
        let err_len = err.read_len();
        let delay = cleanup_started
            .map_or_else(
                || invocation.timeout.saturating_sub(started.elapsed()),
                |cleanup| CLEANUP_TIMEOUT.saturating_sub(cleanup.elapsed()),
            )
            .min(POLL_INTERVAL);
        tokio::select! {
            // A continuously ready producer cannot starve the deadline: each
            // selected read is a single bounded chunk, then the loop checks time.
            result = stdout.server.read(&mut out_buffer[..out_len]), if !out.done() => {
                out.accept_read(result, &out_buffer[..])?;
            }
            result = stderr.server.read(&mut err_buffer[..err_len]), if !err.done() => {
                err.accept_read(result, &err_buffer[..])?;
            }
            result = stdin.write_next(), if stdin.is_open() => {
                result?;
            }
            () = tokio::time::sleep(delay) => {}
        }
    }
    let exit = exit.expect("capture completion requires root exit");
    Ok(BoundedProcessOutcome {
        outcome: ProcessOutcome {
            status: if timed_out {
                ProcessStatus::TimedOut
            } else {
                ProcessStatus::Exited
            },
            exit_code: exit.code(),
            signal: None,
            stdout: out.bytes,
            stderr: err.bytes,
            wall_time_micros: started.elapsed().as_micros(),
            recorded_command: redact(invocation),
        },
        output_limit: out.exceeded.or(err.exceeded),
    })
}

#[cfg(test)]
#[path = "process_windows_tests.rs"]
mod tests;
