//! Bounded capture queues with explicitly owned file-writer threads.

use std::{
    fs::File,
    io::{self, Write as _},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use tokio::io::AsyncReadExt as _;

use super::{MeasureError, native_process_windows::EXIT_POLL, pipe_windows::OutputReader};

const DRAIN_TIMEOUT: Duration = Duration::from_millis(250);
const CHUNK: usize = 16 * 1024;
const QUEUE_DEPTH: usize = 2;
pub(super) const FIRST_UNSET: u64 = u64::MAX;

#[derive(Default)]
pub(super) struct StreamCounts {
    pub bytes: AtomicU64,
    pub limited: AtomicBool,
    pub failed: AtomicBool,
}

pub(super) enum CaptureSink {
    File(File),
    #[cfg(test)]
    Blocked {
        file: File,
        entered: Arc<AtomicBool>,
        release: Receiver<()>,
    },
}

impl CaptureSink {
    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
        match self {
            Self::File(file) => file.write_all(bytes),
            #[cfg(test)]
            Self::Blocked {
                file,
                entered,
                release,
            } => {
                if !entered.swap(true, Ordering::AcqRel) {
                    release.recv().map_err(io::Error::other)?;
                }
                file.write_all(bytes)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::File(file) => file.flush(),
            #[cfg(test)]
            Self::Blocked { file, .. } => file.flush(),
        }
    }
}

pub(super) struct CaptureWorker {
    sender: Option<SyncSender<Vec<u8>>>,
    reader: Option<tokio::task::JoinHandle<io::Result<()>>>,
    writer: Option<JoinHandle<io::Result<()>>>,
    stop: Arc<AtomicBool>,
    counts: Arc<StreamCounts>,
}

impl CaptureWorker {
    /// Reserve writer infrastructure before the target can exist. At most two
    /// queued chunks plus one active write are retained per stream.
    pub(super) fn prepare(sink: CaptureSink, counts: Arc<StreamCounts>) -> io::Result<Self> {
        let (sender, receiver) = mpsc::sync_channel(QUEUE_DEPTH);
        let stop = Arc::new(AtomicBool::new(false));
        let writer_stop = Arc::clone(&stop);
        let writer_counts = Arc::clone(&counts);
        let writer = thread::Builder::new()
            .name("tq-bench-capture-writer".into())
            .spawn(move || {
                let result = write_chunks(sink, &receiver, &writer_stop);
                if result.is_err() {
                    writer_counts.failed.store(true, Ordering::Release);
                }
                result
            })?;
        Ok(Self {
            sender: Some(sender),
            reader: None,
            writer: Some(writer),
            stop,
            counts,
        })
    }

    pub(super) fn start(
        &mut self,
        pipe: OutputReader,
        first: Option<Arc<AtomicU64>>,
        limit: u64,
        started: Instant,
    ) {
        let sender = self.sender.take().expect("capture reader starts once");
        let counts = Arc::clone(&self.counts);
        self.reader = Some(tokio::spawn(async move {
            let result = read_chunks(pipe, sender, &counts, first, limit, started).await;
            if result.is_err() {
                counts.failed.store(true, Ordering::Release);
            }
            result
        }));
    }

    pub(super) async fn finish(&mut self, drain: bool) -> Result<(), MeasureError> {
        let mut reader = self.reader.take().expect("started capture reader");
        let reader_result = if drain {
            match tokio::time::timeout(DRAIN_TIMEOUT, &mut reader).await {
                Ok(Ok(result)) => result.map_err(MeasureError::Io),
                Ok(Err(_)) => Err(MeasureError::CaptureWorker),
                Err(_) => {
                    reader.abort();
                    let _ = reader.await;
                    Err(MeasureError::CaptureDrainTimeout)
                }
            }
        } else {
            reader.abort();
            match reader.await {
                Err(error) if error.is_cancelled() => Ok(()),
                Ok(result) => result.map_err(MeasureError::Io),
                Err(_) => Err(MeasureError::CaptureWorker),
            }
        };
        // Aborting the reader closes its sender. Drain already-owned bounded
        // chunks and flush on the writer, even for a forced target outcome; do
        // not discard diagnostic bytes merely because the target was stopped.
        // A blocked synchronous file write cannot be safely aborted. Target
        // exit, resource collection and job cleanup already happened. Keep this
        // owner alive and yield while the coordinator bounds its caller wait.
        // No spawn_blocking task or detached writer can outlive this ownership.
        while !self.writer.as_ref().expect("owned writer").is_finished() {
            tokio::time::sleep(EXIT_POLL).await;
        }
        let writer_result = self
            .writer
            .take()
            .unwrap()
            .join()
            .map_err(|_| MeasureError::CaptureWorker)?
            .map_err(MeasureError::Io);
        reader_result?;
        writer_result
    }
}

impl Drop for CaptureWorker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(reader) = &self.reader {
            reader.abort();
        }
        self.sender.take();
        // Error/panic paths retain ownership too. recv_timeout lets an idle
        // writer observe stop without needing the runtime to poll reader abort.
        if let Some(writer) = self.writer.take() {
            let _ = writer.join();
        }
    }
}

fn write_chunks(
    mut sink: CaptureSink,
    receiver: &Receiver<Vec<u8>>,
    stop: &AtomicBool,
) -> io::Result<()> {
    loop {
        if stop.load(Ordering::Acquire) {
            return Ok(());
        }
        match receiver.recv_timeout(Duration::from_millis(1)) {
            Ok(chunk) => sink.write_all(&chunk)?,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return sink.flush(),
        }
    }
}

async fn read_chunks(
    mut pipe: OutputReader,
    sender: SyncSender<Vec<u8>>,
    counts: &StreamCounts,
    first: Option<Arc<AtomicU64>>,
    limit: u64,
    started: Instant,
) -> io::Result<()> {
    let mut buffer = Box::new([0_u8; CHUNK]);
    let mut retained = 0_u64;
    loop {
        let read = match pipe.server.read(buffer.as_mut()).await {
            Ok(0) => return Ok(()),
            Ok(read) => read,
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => return Ok(()),
            Err(error) => return Err(error),
        };
        let read64 = u64::try_from(read).map_err(io::Error::other)?;
        let bytes = counts
            .bytes
            .load(Ordering::Relaxed)
            .checked_add(read64)
            .ok_or_else(|| io::Error::other("capture byte count overflow"))?;
        counts.bytes.store(bytes, Ordering::Release);
        if let Some(first) = &first {
            let micros = u64::try_from(started.elapsed().as_micros()).map_err(io::Error::other)?;
            let _ =
                first.compare_exchange(FIRST_UNSET, micros, Ordering::AcqRel, Ordering::Acquire);
        }
        let keep = limit.saturating_sub(retained).min(read64);
        retained += keep;
        if keep < read64 {
            counts.limited.store(true, Ordering::Release);
        }
        if keep == 0 {
            tokio::task::yield_now().await;
            continue;
        }
        let mut chunk = buffer[..usize::try_from(keep).map_err(io::Error::other)?].to_vec();
        loop {
            match sender.try_send(chunk) {
                Ok(()) => break,
                Err(TrySendError::Full(returned)) => chunk = returned,
                Err(TrySendError::Disconnected(_)) => {
                    return Err(io::Error::other("owned capture writer stopped"));
                }
            }
            tokio::time::sleep(EXIT_POLL).await;
        }
        // Even an always-readable producer must let the owner service its
        // timeout, cancellation and RSS policies after each bounded chunk.
        tokio::task::yield_now().await;
    }
}
