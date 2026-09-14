//! Allocation behavior shared by benchmark preflight and subprocess tests.

use std::{
    hint::black_box,
    sync::{Arc, Barrier},
    thread,
};

use thiserror::Error;

const PAGE_TOUCH_STRIDE_BYTES: usize = 4096;

/// Failure while running an allocation probe.
#[derive(Debug, Error)]
pub enum AllocationProbeError {
    /// A probe must have at least one allocation worker.
    #[error("allocation thread count must be greater than zero")]
    ZeroThreads,
    /// An allocation worker unexpectedly panicked.
    #[error("allocation worker thread panicked")]
    WorkerPanicked,
}

/// Allocates, touches, and releases memory before returning.
///
/// Each worker allocates `bytes` bytes and writes an opaque value into every
/// 4096-byte chunk so the operating system must back the pages. With more than
/// one worker, a barrier keeps all allocations resident until every worker has
/// touched its allocation. The workers then release their allocations and
/// return without an intentional sleep or polling dwell.
///
/// # Errors
///
/// Returns an error when `threads` is zero or a worker thread panics.
pub fn run_allocation_probe(bytes: usize, threads: usize) -> Result<(), AllocationProbeError> {
    if threads == 0 {
        return Err(AllocationProbeError::ZeroThreads);
    }

    if threads == 1 {
        let allocation = touched_allocation(bytes);
        black_box(allocation.as_slice());
        drop(allocation);
        return Ok(());
    }

    let barrier = Arc::new(Barrier::new(threads));
    let workers = (0..threads)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let allocation = touched_allocation(bytes);
                // Keep every worker's allocation resident until all workers
                // have touched their pages, then release before process exit.
                barrier.wait();
                black_box(allocation.as_slice());
                drop(allocation);
            })
        })
        .collect::<Vec<_>>();

    for worker in workers {
        worker
            .join()
            .map_err(|_| AllocationProbeError::WorkerPanicked)?;
    }
    Ok(())
}

fn touched_allocation(bytes: usize) -> Vec<u8> {
    let mut allocation = vec![0_u8; bytes];
    for (index, chunk) in allocation.chunks_mut(PAGE_TOUCH_STRIDE_BYTES).enumerate() {
        let value = black_box(index.to_le_bytes()[0].wrapping_mul(31).wrapping_add(1));
        chunk[0] = value;
        // Read the stored value through black_box as well. This keeps the
        // page-touching store observable to optimizers in release builds.
        black_box(chunk[0]);
    }
    black_box(allocation.as_slice());
    allocation
}
