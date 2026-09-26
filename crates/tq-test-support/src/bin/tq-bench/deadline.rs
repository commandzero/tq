//! One cancellation owner for a campaign or a case, including its child phases.

use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub(super) struct Deadline {
    until: Instant,
    cancellation: Arc<AtomicBool>,
    expired: Arc<AtomicBool>,
    stop: mpsc::Sender<()>,
    worker: Option<JoinHandle<()>>,
}

impl Deadline {
    pub(super) fn start(
        cancellation: Arc<AtomicBool>,
        parent: Option<Arc<AtomicBool>>,
        duration: Duration,
    ) -> io::Result<Self> {
        let deadline = Instant::now().checked_add(duration).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "deadline exceeds clock range")
        })?;
        let expired = Arc::new(AtomicBool::new(false));
        let (stop, receiver) = mpsc::channel();
        let flag = Arc::clone(&cancellation);
        let timeout = Arc::clone(&expired);
        let worker = thread::Builder::new()
            .name("benchmark-deadline".to_owned())
            .spawn(move || {
                loop {
                    if parent
                        .as_ref()
                        .is_some_and(|parent| parent.load(Ordering::Acquire))
                    {
                        flag.store(true, Ordering::Release);
                        break;
                    }
                    if flag.load(Ordering::Acquire) {
                        break;
                    }
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    if remaining.is_zero() {
                        timeout.store(true, Ordering::Release);
                        flag.store(true, Ordering::Release);
                        break;
                    }
                    match receiver.recv_timeout(remaining.min(Duration::from_millis(20))) {
                        Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                    }
                }
            })?;
        Ok(Self {
            cancellation,
            expired,
            stop,
            worker: Some(worker),
            until: deadline,
        })
    }

    pub(super) fn cancelled(&self) -> bool {
        self.cancellation.load(Ordering::Acquire)
    }
    pub(super) fn expired(&self) -> bool {
        self.expired.load(Ordering::Acquire)
    }
    pub(super) fn remaining(&self) -> Duration {
        self.until.saturating_duration_since(Instant::now())
    }
    pub(super) fn flag(&self) -> &Arc<AtomicBool> {
        &self.cancellation
    }
}

impl Drop for Deadline {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
