//! Bounded unknown-length array preparation with secure disk transition.

use std::{
    collections::{BTreeSet, hash_map::DefaultHasher},
    fs::{File, OpenOptions},
    hash::{Hash, Hasher},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use thiserror::Error;
use tq_core::{
    Object, Value,
    presentation::{ColorPalette, ColorRole, write_span},
};

use crate::{
    WriterConfig, replay,
    schema::{RowSchema, SchemaLimits},
    writer,
};

#[path = "event_tape.rs"]
pub(crate) mod event_tape;

static NEXT_SPOOL: AtomicU64 = AtomicU64::new(0);

/// Result-scoped preparation limits shared by every active container.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreparationLimits {
    /// Aggregate bytes retained in memory.
    pub memory_bytes: usize,
    /// Aggregate temporary bytes written.
    pub spool_bytes: u64,
    /// Aggregate prepared output bytes.
    pub output_bytes: u64,
    /// Maximum simultaneously active container frames.
    pub nesting: usize,
}

impl Default for PreparationLimits {
    fn default() -> Self {
        Self {
            memory_bytes: 8 * 1024 * 1024,
            spool_bytes: 8 * 1024 * 1024 * 1024,
            output_bytes: 8 * 1024 * 1024 * 1024,
            nesting: 256,
        }
    }
}

/// High-water and I/O observations from one result preparation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PreparationObservations {
    /// Highest aggregate in-memory retention.
    pub memory_high_water_bytes: usize,
    /// Temporary bytes written.
    pub spool_bytes_written: u64,
    /// Temporary bytes replayed.
    pub spool_bytes_replayed: u64,
    /// Prepared output bytes published or retained.
    pub output_bytes: u64,
    /// Highest simultaneously active container depth.
    pub nesting_high_water: usize,
    /// Number of object preparations before layout selection.
    pub object_preparations: u64,
    /// Number of arrays prepared before layout selection.
    pub array_preparations: u64,
}

#[derive(Debug, Default)]
struct PreparationState {
    memory_bytes: usize,
    active_nesting: usize,
    observations: PreparationObservations,
}

/// Cloneable handle to one result-scoped preparation ledger.
#[derive(Clone, Debug)]
pub struct PreparationArena {
    limits: PreparationLimits,
    state: Arc<Mutex<PreparationState>>,
}

impl PreparationArena {
    /// Creates an empty result-scoped ledger.
    #[must_use]
    pub fn new(limits: PreparationLimits) -> Self {
        Self {
            limits,
            state: Arc::new(Mutex::new(PreparationState::default())),
        }
    }

    /// Current aggregate observations.
    #[must_use]
    pub fn observations(&self) -> PreparationObservations {
        self.state().observations
    }

    /// Enters one active preparation frame.
    ///
    /// # Errors
    ///
    /// Returns a resource error if the shared nesting limit is exhausted.
    pub fn enter(&self) -> Result<PreparationFrame, SpoolError> {
        let mut state = self.state();
        if state.active_nesting >= self.limits.nesting {
            return Err(SpoolError::NestingLimit);
        }
        state.active_nesting += 1;
        state.observations.nesting_high_water = state
            .observations
            .nesting_high_water
            .max(state.active_nesting);
        drop(state);
        Ok(PreparationFrame {
            arena: self.clone(),
        })
    }

    /// Creates a growable charge for transient container values.
    #[must_use]
    pub fn memory_charge(&self) -> PreparationMemory {
        PreparationMemory {
            arena: self.clone(),
            bytes: 0,
        }
    }

    fn state(&self) -> std::sync::MutexGuard<'_, PreparationState> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn retain_memory(&self, bytes: usize) -> bool {
        let mut state = self.state();
        let retained = state.memory_bytes.saturating_add(bytes);
        if retained > self.limits.memory_bytes {
            return false;
        }
        state.memory_bytes = retained;
        state.observations.memory_high_water_bytes =
            state.observations.memory_high_water_bytes.max(retained);
        true
    }

    fn release_memory(&self, bytes: usize) {
        let mut state = self.state();
        state.memory_bytes = state.memory_bytes.saturating_sub(bytes);
    }

    fn can_write_spool(&self, bytes: u64) -> bool {
        self.state()
            .observations
            .spool_bytes_written
            .saturating_add(bytes)
            <= self.limits.spool_bytes
    }

    fn wrote_spool(&self, bytes: u64) {
        let mut state = self.state();
        state.observations.spool_bytes_written =
            state.observations.spool_bytes_written.saturating_add(bytes);
    }

    fn replayed_spool(&self, bytes: u64) {
        let mut state = self.state();
        state.observations.spool_bytes_replayed = state
            .observations
            .spool_bytes_replayed
            .saturating_add(bytes);
    }

    /// Charges prepared output bytes to the shared result limit.
    ///
    /// # Errors
    ///
    /// Returns a resource error when the output limit would be exceeded.
    pub fn record_output(&self, bytes: u64) -> Result<(), SpoolError> {
        let mut state = self.state();
        let output = state.observations.output_bytes.saturating_add(bytes);
        if output > self.limits.output_bytes {
            return Err(SpoolError::OutputLimit);
        }
        state.observations.output_bytes = output;
        Ok(())
    }

    fn record_array_preparation(&self) {
        let mut state = self.state();
        state.observations.array_preparations =
            state.observations.array_preparations.saturating_add(1);
    }

    fn record_object_preparation(&self) {
        let mut state = self.state();
        state.observations.object_preparations =
            state.observations.object_preparations.saturating_add(1);
    }
}

/// Active container charge released when its frame closes.
#[derive(Debug)]
pub struct PreparationFrame {
    arena: PreparationArena,
}

/// Memory retained by a transient container that cannot publish yet.
#[derive(Debug)]
pub struct PreparationMemory {
    arena: PreparationArena,
    bytes: usize,
}

impl PreparationMemory {
    /// Adds retained bytes to the shared result budget.
    ///
    /// # Errors
    ///
    /// Returns a resource error when the aggregate memory limit is exhausted.
    pub fn grow(&mut self, bytes: usize) -> Result<(), SpoolError> {
        if self.arena.retain_memory(bytes) {
            self.bytes = self.bytes.saturating_add(bytes);
            Ok(())
        } else {
            Err(SpoolError::MemoryLimit)
        }
    }
    /// Releases retained bytes after a storage buffer moves to disk.
    pub(crate) fn shrink(&mut self, bytes: usize) {
        let released = bytes.min(self.bytes);
        self.bytes -= released;
        self.arena.release_memory(released);
    }
}

impl Drop for PreparationMemory {
    fn drop(&mut self) {
        self.arena.release_memory(self.bytes);
    }
}

impl Drop for PreparationFrame {
    fn drop(&mut self) {
        let mut state = self.arena.state();
        state.active_nesting = state.active_nesting.saturating_sub(1);
    }
}

/// Unknown-length array buffering and spool limits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArrayPreparationConfig {
    /// Encoded bytes retained before disk transition.
    pub memory_threshold_bytes: usize,
    /// Maximum total framed bytes allowed in a spool.
    pub maximum_spool_bytes: u64,
    /// Directory that owns temporary spool files.
    pub spool_directory: PathBuf,
    /// Whether disk transition is permitted.
    pub allow_spool: bool,
}

impl Default for ArrayPreparationConfig {
    fn default() -> Self {
        Self {
            memory_threshold_bytes: 8 * 1024 * 1024,
            maximum_spool_bytes: 8 * 1024 * 1024 * 1024,
            spool_directory: std::env::temp_dir(),
            allow_spool: true,
        }
    }
}

/// Array preparation or replay failure.
#[derive(Debug, Error)]
pub enum SpoolError {
    /// Preparation or replay was cancelled cooperatively.
    #[error("array preparation was cancelled")]
    Cancelled,
    /// A transient nested container exceeded the aggregate memory budget.
    #[error("nested container exceeds configured preparation memory limit")]
    MemoryLimit,
    /// Spooling was required but disabled.
    #[error("array preparation exceeded memory threshold and spooling is disabled")]
    Disabled,
    /// Configured disk limit would be exceeded.
    #[error("array spool exceeds configured byte limit")]
    Limit,
    /// Shared output preparation limit would be exceeded.
    #[error("prepared output exceeds configured byte limit")]
    OutputLimit,
    /// Shared active-container nesting limit would be exceeded.
    #[error("preparation nesting exceeds configured limit")]
    NestingLimit,
    /// Temporary-file or output I/O failed.
    #[error("array spool I/O failed: {0}")]
    Io(#[from] io::Error),
    /// Canonical writer failed during prepared replay.
    #[error(transparent)]
    Writer(#[from] crate::WriterError),
    /// An internal structural replay record could not be decoded.
    #[error("array spool record is invalid: {0}")]
    Decode(&'static str),
}

/// Atomic publication failure for unframed output.
#[derive(Debug, Error)]
pub enum PublicationError {
    /// Exactly-one output cardinality failed.
    #[error(transparent)]
    Cardinality(#[from] crate::CardinalityError),
    /// Preparation or output failed.
    #[error(transparent)]
    Spool(#[from] SpoolError),
    /// Publication output failed.
    #[error("publication output failed: {0}")]
    Io(#[from] io::Error),
}

/// Result-sized bytes retained in bounded memory or private temporary storage
/// until publication succeeds.
#[derive(Debug)]
pub struct PublicationBuffer {
    config: ArrayPreparationConfig,
    arena: PreparationArena,
    memory: Vec<u8>,
    memory_charge: PreparationMemory,
    spool: Option<Spool>,
    bytes: u64,
    published: bool,
}

struct PublicationProgress<'a, W> {
    output: &'a mut W,
    written: u64,
}

impl<W: Write> Write for PublicationProgress<'_, W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let count = self.output.write(bytes)?;
        self.written = self.written.saturating_add(count as u64);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.output.flush()
    }
}

impl PublicationBuffer {
    /// Creates an empty atomic publication buffer.
    #[must_use]
    pub fn new(config: ArrayPreparationConfig, arena: PreparationArena) -> Self {
        let memory_charge = arena.memory_charge();
        Self {
            config,
            arena,
            memory: Vec::new(),
            memory_charge,
            spool: None,
            bytes: 0,
            published: false,
        }
    }

    /// Prepared byte count.
    #[must_use]
    pub const fn len(&self) -> u64 {
        self.bytes
    }

    /// Whether no output has been prepared.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.bytes == 0
    }

    /// Whether preparation moved to disk.
    #[must_use]
    pub const fn spooled(&self) -> bool {
        self.spool.is_some()
    }

    /// Current private publication spool path.
    #[must_use]
    pub fn spool_path(&self) -> Option<&Path> {
        self.spool.as_ref().map(|spool| spool.path.as_path())
    }

    /// Publishes only after exactly-one-result validation.
    ///
    /// # Errors
    ///
    /// Returns cardinality, spool replay, or output failures. A cardinality
    /// failure writes no bytes.
    pub fn publish_single<W: Write>(
        &mut self,
        output: &mut W,
        result_count: u64,
    ) -> Result<(), PublicationError> {
        self.publish_single_colored(output, result_count, None)
    }

    /// Publishes only after exactly-one-result validation, with palette-aware
    /// recovery for a partial decorated sink write.
    ///
    /// The prepared bytes are already decorated. The palette is used only to
    /// decide whether a best-effort terminal reset is appropriate after a
    /// non-broken-pipe publication failure.
    ///
    /// # Errors
    ///
    /// Returns cardinality, spool replay, or output failures.
    pub fn publish_single_colored<W: Write>(
        &mut self,
        output: &mut W,
        result_count: u64,
        palette: Option<&ColorPalette>,
    ) -> Result<(), PublicationError> {
        match result_count {
            0 => return Err(crate::CardinalityError::Zero.into()),
            1 => {}
            _ => return Err(crate::CardinalityError::Multiple.into()),
        }
        self.publish_colored(output, palette)
    }

    pub(crate) fn publish_colored<W: Write>(
        &mut self,
        output: &mut W,
        palette: Option<&ColorPalette>,
    ) -> Result<(), PublicationError> {
        if self.published {
            return Err(SpoolError::Decode("publication buffer already committed").into());
        }
        let mut progress = PublicationProgress { output, written: 0 };
        let result = (|| -> Result<(), PublicationError> {
            if let Some(spool) = &mut self.spool {
                spool.file.flush()?;
                spool.file.seek(SeekFrom::Start(0))?;
                let mut copied = 0_u64;
                let capacity = self
                    .arena
                    .limits
                    .memory_bytes
                    .saturating_sub(self.arena.state().memory_bytes)
                    .min(64 * 1024);
                if capacity == 0 {
                    return Err(SpoolError::MemoryLimit.into());
                }
                let mut charge = self.arena.memory_charge();
                charge.grow(capacity)?;
                let mut chunk = vec![0_u8; capacity];
                loop {
                    let read = spool.file.read(&mut chunk)?;
                    if read == 0 {
                        break;
                    }
                    progress.write_all(&chunk[..read])?;
                    copied = copied.saturating_add(read as u64);
                }
                self.arena.replayed_spool(copied);
            } else {
                progress.write_all(&self.memory)?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            let broken_pipe = matches!(&error, PublicationError::Io(error) if error.kind() == io::ErrorKind::BrokenPipe);
            if palette.is_some() && progress.written != 0 && !broken_pipe {
                let _ = progress.output.write_all(b"\x1b[0m");
            }
            return Err(error);
        }
        self.published = true;
        Ok(())
    }

    fn append(&mut self, bytes: &[u8]) -> Result<(), SpoolError> {
        if bytes.is_empty() {
            return Ok(());
        }
        let next = self.bytes.saturating_add(bytes.len() as u64);
        self.arena.record_output(bytes.len() as u64)?;
        if self.spool.is_none() {
            let needed = self.memory.len().saturating_add(bytes.len());
            if needed > self.config.memory_threshold_bytes {
                self.transition_to_disk()?;
            } else if needed > self.memory.capacity() {
                let previous = self.memory.capacity();
                let target = needed
                    .max(previous.saturating_mul(2))
                    .min(self.config.memory_threshold_bytes);
                let reserved = target.saturating_sub(previous);
                if self.memory_charge.grow(reserved).is_err() {
                    self.transition_to_disk()?;
                } else if self
                    .memory
                    .try_reserve_exact(target.saturating_sub(self.memory.len()))
                    .is_err()
                {
                    self.memory_charge.shrink(reserved);
                    self.transition_to_disk()?;
                } else {
                    let actual = self.memory.capacity().saturating_sub(previous);
                    if actual <= reserved {
                        self.memory_charge.shrink(reserved - actual);
                    } else {
                        self.memory_charge.grow(actual - reserved)?;
                    }
                }
            }
        }
        if let Some(spool) = &mut self.spool {
            if next > self.config.maximum_spool_bytes
                || !self.arena.can_write_spool(bytes.len() as u64)
            {
                return Err(SpoolError::Limit);
            }
            spool.file.write_all(bytes)?;
            self.arena.wrote_spool(bytes.len() as u64);
        } else {
            self.memory.extend_from_slice(bytes);
        }
        self.bytes = next;
        Ok(())
    }

    fn transition_to_disk(&mut self) -> Result<(), SpoolError> {
        if !self.config.allow_spool {
            return Err(SpoolError::Disabled);
        }
        let bytes = self.memory.len() as u64;
        if bytes > self.config.maximum_spool_bytes || !self.arena.can_write_spool(bytes) {
            return Err(SpoolError::Limit);
        }
        let mut spool = create_spool(&self.config.spool_directory)?;
        spool.file.write_all(&self.memory)?;
        self.arena.wrote_spool(bytes);
        self.memory_charge.shrink(self.memory.capacity());
        self.memory = Vec::new();
        self.spool = Some(spool);
        Ok(())
    }
}

impl Write for PublicationBuffer {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.append(buffer)
            .map(|()| buffer.len())
            .map_err(io::Error::other)
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Some(spool) = &mut self.spool {
            spool.file.flush()
        } else {
            Ok(())
        }
    }
}

/// Prepared unknown-length array that retains values in memory or a private
/// length-framed temporary file, never both after transition.
#[derive(Debug)]
pub struct PreparedArray {
    config: ArrayPreparationConfig,
    arena: PreparationArena,
    cancellation: Option<Arc<std::sync::atomic::AtomicBool>>,
    memory: Vec<Vec<u8>>,
    memory_bytes: usize,
    schema_bytes: usize,
    spool: Option<Spool>,
    count: u64,
    framed_bytes: u64,
    layout: Layout,
}

#[derive(Debug)]
struct Spool {
    file: File,
    path: PathBuf,
}

impl Drop for Spool {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[derive(Clone, Debug)]
enum Layout {
    Empty,
    Scalars,
    Tabular(Arc<RowSchema>),
    Expanded,
}

const KEY_BLOOM_BYTES: usize = 256 * 1024;

/// Exact duplicate-name index with bounded in-memory runs.
#[derive(Debug)]
pub struct PreparedKeySet {
    config: ArrayPreparationConfig,
    arena: PreparationArena,
    memory: BTreeSet<Arc<str>>,
    memory_bytes: usize,
    runs: Vec<Spool>,
    bloom: Vec<u8>,
    bloom_bytes: usize,
}

impl PreparedKeySet {
    /// Creates an empty key index charged to an existing result arena.
    #[must_use]
    pub fn new(config: ArrayPreparationConfig, arena: PreparationArena) -> Self {
        Self {
            config,
            arena,
            memory: BTreeSet::new(),
            memory_bytes: 0,
            runs: Vec::new(),
            bloom: Vec::new(),
            bloom_bytes: 0,
        }
    }

    /// Inserts a key, returning false when it was encountered before.
    ///
    /// # Errors
    ///
    /// Returns a temporary-storage or configured spool-limit failure.
    pub fn insert(&mut self, key: Arc<str>) -> Result<bool, SpoolError> {
        if self.memory.contains(&key) || self.contains_in_runs(&key)? {
            return Ok(false);
        }

        let charge = key_set_charge(&key);
        let retained = if self.arena.retain_memory(charge) {
            true
        } else {
            self.flush_run()?;
            self.arena.retain_memory(charge)
        };
        if retained {
            self.memory_bytes = self.memory_bytes.saturating_add(charge);
            self.memory.insert(key);
        } else {
            self.write_single_key_run(&key)?;
        }
        Ok(true)
    }

    fn contains_in_runs(&mut self, key: &str) -> Result<bool, SpoolError> {
        if self.runs.is_empty() || !self.bloom_might_contain(key) {
            return Ok(false);
        }
        for run in &mut self.runs {
            run.file.seek(SeekFrom::Start(0))?;
            let mut replayed = 0_u64;
            loop {
                let mut length = [0_u8; 8];
                if run.file.read(&mut length[..1])? == 0 {
                    self.arena.replayed_spool(replayed);
                    break;
                }
                run.file.read_exact(&mut length[1..])?;
                replayed = replayed.saturating_add(8);
                let length =
                    usize::try_from(u64::from_le_bytes(length)).map_err(|_| SpoolError::Limit)?;
                if length == key.len() {
                    let mut candidate = vec![0_u8; length];
                    run.file.read_exact(&mut candidate)?;
                    replayed = replayed.saturating_add(length as u64);
                    if candidate == key.as_bytes() {
                        self.arena.replayed_spool(replayed);
                        return Ok(true);
                    }
                } else {
                    run.file.seek(SeekFrom::Current(
                        i64::try_from(length).map_err(|_| SpoolError::Limit)?,
                    ))?;
                    replayed = replayed.saturating_add(length as u64);
                }
            }
        }
        Ok(false)
    }

    fn flush_run(&mut self) -> Result<(), SpoolError> {
        if self.memory.is_empty() {
            return Ok(());
        }
        let memory = std::mem::take(&mut self.memory);
        self.arena.release_memory(self.memory_bytes);
        self.memory_bytes = 0;
        self.ensure_bloom();
        let mut run = create_spool(&self.config.spool_directory)?;
        let mut written = 0_u64;
        for key in &memory {
            let bytes = 8_u64.saturating_add(key.len() as u64);
            if !self.arena.can_write_spool(bytes) {
                return Err(SpoolError::Limit);
            }
            write_record(&mut run.file, key.as_bytes())?;
            self.arena.wrote_spool(bytes);
            written = written.saturating_add(bytes);
            bloom_insert(&mut self.bloom, key);
        }
        if written > self.config.maximum_spool_bytes {
            return Err(SpoolError::Limit);
        }
        self.runs.push(run);
        Ok(())
    }

    fn write_single_key_run(&mut self, key: &str) -> Result<(), SpoolError> {
        let bytes = 8_u64.saturating_add(key.len() as u64);
        if bytes > self.config.maximum_spool_bytes || !self.arena.can_write_spool(bytes) {
            return Err(SpoolError::Limit);
        }
        self.ensure_bloom();
        let mut run = create_spool(&self.config.spool_directory)?;
        write_record(&mut run.file, key.as_bytes())?;
        self.arena.wrote_spool(bytes);
        bloom_insert(&mut self.bloom, key);
        self.runs.push(run);
        Ok(())
    }

    fn ensure_bloom(&mut self) {
        if self.bloom.is_empty() && self.arena.retain_memory(KEY_BLOOM_BYTES) {
            self.bloom = vec![0_u8; KEY_BLOOM_BYTES];
            self.bloom_bytes = KEY_BLOOM_BYTES;
        }
    }

    fn bloom_might_contain(&self, key: &str) -> bool {
        self.bloom.is_empty()
            || bloom_positions(key, self.bloom.len())
                .into_iter()
                .all(|position| self.bloom[position / 8] & (1 << (position % 8)) != 0)
    }
}

impl Drop for PreparedKeySet {
    fn drop(&mut self) {
        self.arena
            .release_memory(self.memory_bytes.saturating_add(self.bloom_bytes));
    }
}

fn key_set_charge(key: &str) -> usize {
    key.len()
        .saturating_add(std::mem::size_of::<Arc<str>>())
        .saturating_add(4 * std::mem::size_of::<usize>())
}

fn bloom_positions(key: &str, bytes: usize) -> [usize; 3] {
    let bits = bytes.saturating_mul(8).max(1);
    let bits_u64 = u64::try_from(bits).unwrap_or(u64::MAX);
    let mut first = DefaultHasher::new();
    key.hash(&mut first);
    let first = first.finish();
    let mut second = DefaultHasher::new();
    0x9e37_79b9_u32.hash(&mut second);
    key.hash(&mut second);
    let second = second.finish() | 1;
    [0_u64, 1, 2].map(|step| {
        usize::try_from(first.wrapping_add(step.wrapping_mul(second)) % bits_u64).unwrap_or(0)
    })
}

fn bloom_insert(bloom: &mut [u8], key: &str) {
    if bloom.is_empty() {
        return;
    }
    for position in bloom_positions(key, bloom.len()) {
        bloom[position / 8] |= 1 << (position % 8);
    }
}

impl PreparedArray {
    /// Creates an empty unknown-length array preparation.
    #[must_use]
    pub fn new(config: ArrayPreparationConfig) -> Self {
        let arena = PreparationArena::new(PreparationLimits {
            memory_bytes: config.memory_threshold_bytes,
            spool_bytes: config.maximum_spool_bytes,
            ..PreparationLimits::default()
        });
        Self::in_arena(config, arena)
    }

    /// Creates an array preparation charged to an existing result arena.
    #[must_use]
    pub fn in_arena(config: ArrayPreparationConfig, arena: PreparationArena) -> Self {
        arena.record_array_preparation();
        Self {
            config,
            arena,
            cancellation: None,
            memory: Vec::new(),
            memory_bytes: 0,
            schema_bytes: 0,
            spool: None,
            count: 0,
            framed_bytes: 0,
            layout: Layout::Empty,
        }
    }

    /// Adds a cooperative cancellation flag checked during preparation and replay.
    #[must_use]
    pub fn with_cancellation(mut self, cancellation: Arc<std::sync::atomic::AtomicBool>) -> Self {
        self.cancellation = Some(cancellation);
        self
    }

    /// Adds one value while enforcing memory and disk limits.
    ///
    /// # Errors
    ///
    /// Returns temporary-file, disabled-spool, or limit errors.
    pub fn push(&mut self, value: &Value) -> Result<(), SpoolError> {
        let layout = self.next_layout(value)?;
        let new_schema =
            matches!(&self.layout, Layout::Empty) && matches!(&layout, Layout::Tabular(_));
        match self.push_value(value, layout) {
            Ok(()) => Ok(()),
            Err(error) => {
                if new_schema {
                    self.arena.release_memory(self.schema_bytes);
                    self.schema_bytes = 0;
                }
                Err(error)
            }
        }
    }

    fn push_value(&mut self, value: &Value, layout: Layout) -> Result<(), SpoolError> {
        self.push_record(replay::encoded_len(value), layout, |output| {
            replay::encode_to(value, output)
        })
    }

    fn push_record(
        &mut self,
        encoded_length: usize,
        layout: Layout,
        encode: impl FnOnce(&mut dyn Write) -> io::Result<()>,
    ) -> Result<(), SpoolError> {
        self.check_cancelled()?;
        let framed_usize = encoded_length.checked_add(8).ok_or(SpoolError::Limit)?;
        let framed = u64::try_from(framed_usize).map_err(|_| SpoolError::Limit)?;
        let local_memory_available = self
            .memory_bytes
            .checked_add(framed_usize)
            .is_some_and(|bytes| bytes <= self.config.memory_threshold_bytes);
        let retain_memory = self.spool.is_none()
            && local_memory_available
            && self.arena.retain_memory(framed_usize);
        if self.spool.is_none() && !retain_memory {
            let disk_bytes = u64::try_from(self.memory_bytes)
                .map_err(|_| SpoolError::Limit)?
                .checked_add(framed)
                .ok_or(SpoolError::Limit)?;
            if disk_bytes > self.config.maximum_spool_bytes
                || !self.arena.can_write_spool(disk_bytes)
            {
                return Err(SpoolError::Limit);
            }
            self.transition_to_disk()?;
        } else if self.spool.is_some()
            && (self
                .framed_bytes
                .checked_add(framed)
                .ok_or(SpoolError::Limit)?
                > self.config.maximum_spool_bytes)
        {
            return Err(SpoolError::Limit);
        }
        if let Some(spool) = &mut self.spool {
            if !self.arena.can_write_spool(framed) {
                return Err(SpoolError::Limit);
            }
            spool.file.write_all(
                &u64::try_from(encoded_length)
                    .unwrap_or(u64::MAX)
                    .to_le_bytes(),
            )?;
            encode(&mut spool.file)?;
            self.arena.wrote_spool(framed);
        } else {
            let mut encoded = Vec::with_capacity(encoded_length);
            if let Err(error) = encode(&mut encoded) {
                self.arena.release_memory(framed_usize);
                return Err(SpoolError::Io(error));
            }
            self.memory_bytes = self.memory_bytes.saturating_add(framed_usize);
            self.memory.push(encoded);
        }
        if matches!(&self.layout, Layout::Tabular(_)) && matches!(&layout, Layout::Expanded) {
            self.arena.release_memory(self.schema_bytes);
            self.schema_bytes = 0;
        }
        self.layout = layout;
        self.count = self.count.saturating_add(1);
        self.framed_bytes = self.framed_bytes.saturating_add(framed);
        Ok(())
    }

    /// Number of prepared values.
    #[must_use]
    pub const fn len(&self) -> u64 {
        self.count
    }

    /// Whether no values have been prepared.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Whether preparation transitioned to disk.
    #[must_use]
    pub const fn spooled(&self) -> bool {
        self.spool.is_some()
    }

    /// Current private spool path, exposed for observability and tests.
    #[must_use]
    pub fn spool_path(&self) -> Option<&Path> {
        self.spool.as_ref().map(|spool| spool.path.as_path())
    }

    /// Replays the prepared array as canonical root TOON without rebuilding it.
    ///
    /// # Errors
    ///
    /// Returns spool decode/read or output failures.
    pub fn write_to<W: Write>(
        &mut self,
        output: W,
        writer_config: WriterConfig,
    ) -> Result<(), SpoolError> {
        self.write_to_colored(output, writer_config, None)
    }

    /// Replays the prepared array with optional semantic ANSI presentation.
    ///
    /// Styling is applied only during replay. Structural records, layout
    /// metadata, and spool accounting remain independent of presentation.
    ///
    /// # Errors
    ///
    /// Returns spool, cancellation, decoding, or output failures.
    #[allow(clippy::too_many_lines)]
    pub fn write_to_colored<W: Write>(
        &mut self,
        mut output: W,
        writer_config: WriterConfig,
        palette: Option<&ColorPalette>,
    ) -> Result<(), SpoolError> {
        let layout = self.layout.clone();
        if matches!(&layout, Layout::Empty) {
            write_span(&mut output, palette, ColorRole::Array, b"[]")?;
            return Ok(());
        }
        let count = usize::try_from(self.count).map_err(|_| SpoolError::Limit)?;
        let schema = match &layout {
            Layout::Tabular(schema) => Some(schema.as_ref()),
            _ => None,
        };
        writer::write_table_header_colored(
            &mut output,
            None,
            count,
            false,
            schema,
            writer_config,
            palette,
        )?;
        match layout {
            Layout::Empty => {}
            Layout::Scalars => {
                output.write_all(b" ")?;
                let mut index = 0_usize;
                self.for_each_record(|record| {
                    if index != 0 {
                        let mut bytes = [0_u8; 4];
                        write_span(
                            &mut output,
                            palette,
                            ColorRole::Array,
                            delimiter_character(writer_config)
                                .encode_utf8(&mut bytes)
                                .as_bytes(),
                        )?;
                    }
                    let value = replay::decode_scalar(record).map_err(SpoolError::Decode)?;
                    writer::write_scalar_token_colored(
                        &mut output,
                        value,
                        writer_config,
                        writer::ScalarContext::Array,
                        palette,
                    )?;
                    index += 1;
                    Ok(())
                })?;
            }
            Layout::Tabular(schema) => {
                self.for_each_value(|value| {
                    let Value::Object(object) = value else {
                        unreachable!("layout tracked during preparation")
                    };
                    output.write_all(b"\n")?;
                    for _ in 0..writer_config.indent_size {
                        output.write_all(b" ")?;
                    }
                    writer::write_tabular_row_colored(
                        &mut output,
                        object,
                        &schema,
                        writer_config,
                        palette,
                    )?;
                    Ok(())
                })?;
            }
            Layout::Expanded => {
                self.for_each_value(|value| {
                    output.write_all(b"\n")?;
                    let mut indented = LineIndentWriter {
                        output: &mut output,
                        indentation: writer_config.indent_size,
                        line_start: true,
                    };
                    writer::write_list_item_colored(&mut indented, value, writer_config, palette)?;
                    Ok(())
                })?;
            }
        }
        Ok(())
    }

    fn next_layout(&mut self, value: &Value) -> Result<Layout, SpoolError> {
        match &self.layout {
            Layout::Empty | Layout::Scalars if scalar(value) => Ok(Layout::Scalars),
            Layout::Empty => {
                let Value::Object(object) = value else {
                    return Ok(Layout::Expanded);
                };
                if object.is_empty() {
                    return Ok(Layout::Expanded);
                }
                let charge = schema_memory_estimate(object)?;
                if !self.arena.retain_memory(charge) {
                    return Err(SpoolError::MemoryLimit);
                }
                let schema = match RowSchema::from_object(
                    object,
                    SchemaLimits {
                        bytes: charge,
                        ..SchemaLimits::default()
                    },
                ) {
                    Ok(Some(schema)) => Arc::new(schema),
                    Ok(None) => {
                        self.arena.release_memory(charge);
                        return Ok(Layout::Expanded);
                    }
                    Err(_) => {
                        self.arena.release_memory(charge);
                        return Err(SpoolError::MemoryLimit);
                    }
                };
                self.schema_bytes = charge;
                Ok(Layout::Tabular(schema))
            }
            Layout::Tabular(schema) if schema.matches_value(value) => {
                Ok(Layout::Tabular(Arc::clone(schema)))
            }
            Layout::Expanded | Layout::Scalars | Layout::Tabular(_) => Ok(Layout::Expanded),
        }
    }

    fn transition_to_disk(&mut self) -> Result<(), SpoolError> {
        self.check_cancelled()?;
        if !self.config.allow_spool {
            return Err(SpoolError::Disabled);
        }
        let mut spool = create_spool(&self.config.spool_directory)?;
        let transition_bytes = u64::try_from(self.memory_bytes).unwrap_or(u64::MAX);
        if !self.arena.can_write_spool(transition_bytes) {
            return Err(SpoolError::Limit);
        }
        for record in &self.memory {
            self.check_cancelled()?;
            write_record(&mut spool.file, record)?;
        }
        self.arena.wrote_spool(transition_bytes);
        self.memory.clear();
        self.arena.release_memory(self.memory_bytes);
        self.memory_bytes = 0;
        self.spool = Some(spool);
        Ok(())
    }

    fn for_each_value(
        &mut self,
        mut consume: impl FnMut(&Value) -> Result<(), SpoolError>,
    ) -> Result<(), SpoolError> {
        self.for_each_record(|bytes| {
            let value = replay::decode(bytes).map_err(SpoolError::Decode)?;
            consume(&value)
        })
    }

    fn for_each_record(
        &mut self,
        mut consume: impl FnMut(&[u8]) -> Result<(), SpoolError>,
    ) -> Result<(), SpoolError> {
        let cancellation = self.cancellation.clone();
        if let Some(spool) = &mut self.spool {
            spool.file.flush()?;
            spool.file.seek(SeekFrom::Start(0))?;
            loop {
                check_cancelled(cancellation.as_deref())?;
                let mut length = [0_u8; 8];
                if spool.file.read(&mut length[..1])? == 0 {
                    break;
                }
                spool.file.read_exact(&mut length[1..])?;
                let length =
                    usize::try_from(u64::from_le_bytes(length)).map_err(|_| SpoolError::Limit)?;
                let mut transient = self.arena.memory_charge();
                transient.grow(decoded_value_charge(length)?)?;
                let mut bytes = vec![0; length];
                spool.file.read_exact(&mut bytes)?;
                consume(&bytes)?;
            }
            self.arena.replayed_spool(self.framed_bytes);
        } else {
            for bytes in &self.memory {
                self.check_cancelled()?;
                let mut transient = self.arena.memory_charge();
                transient.grow(decoded_value_charge(bytes.len())?)?;
                consume(bytes)?;
            }
        }
        Ok(())
    }

    fn check_cancelled(&self) -> Result<(), SpoolError> {
        check_cancelled(self.cancellation.as_deref())
    }
}

struct LineIndentWriter<'a, W> {
    output: &'a mut W,
    indentation: usize,
    line_start: bool,
}

impl<W: Write> Write for LineIndentWriter<'_, W> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let mut start = 0;
        while start < buffer.len() {
            if self.line_start {
                for _ in 0..self.indentation {
                    self.output.write_all(b" ")?;
                }
                self.line_start = false;
            }
            if let Some(relative) = buffer[start..].iter().position(|byte| *byte == b'\n') {
                let end = start + relative + 1;
                self.output.write_all(&buffer[start..end])?;
                self.line_start = true;
                start = end;
            } else {
                self.output.write_all(&buffer[start..])?;
                break;
            }
        }
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.output.flush()
    }
}

fn check_cancelled(cancellation: Option<&std::sync::atomic::AtomicBool>) -> Result<(), SpoolError> {
    if cancellation.is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Relaxed)) {
        Err(SpoolError::Cancelled)
    } else {
        Ok(())
    }
}

impl Drop for PreparedArray {
    fn drop(&mut self) {
        self.arena.release_memory(self.memory_bytes);
        self.arena.release_memory(self.schema_bytes);
    }
}

fn decoded_value_charge(encoded_bytes: usize) -> Result<usize, SpoolError> {
    encoded_bytes
        .checked_mul(32)
        .and_then(|bytes| bytes.checked_add(128))
        .ok_or(SpoolError::MemoryLimit)
}

fn write_record(mut writer: impl Write, bytes: &[u8]) -> Result<(), io::Error> {
    writer.write_all(&(bytes.len() as u64).to_le_bytes())?;
    writer.write_all(bytes)
}

fn create_spool(directory: &Path) -> Result<Spool, io::Error> {
    for _ in 0..128 {
        let id = NEXT_SPOOL.fetch_add(1, Ordering::Relaxed);
        let path = directory.join(format!(".tq-spool-{}-{id}", std::process::id()));
        let mut options = OpenOptions::new();
        options.read(true).write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&path) {
            Ok(file) => return Ok(Spool { file, path }),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate unique tq spool",
    ))
}

fn scalar(value: &Value) -> bool {
    matches!(
        value,
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)
    )
}

fn schema_memory_estimate(object: &Object) -> Result<usize, SpoolError> {
    fn visit(object: &Object, depth: usize, fields: &mut usize) -> Result<usize, SpoolError> {
        if depth > 256 {
            return Err(SpoolError::MemoryLimit);
        }
        if object.is_empty() {
            return Ok(std::mem::size_of::<RowSchema>());
        }
        *fields = fields
            .checked_add(object.len())
            .filter(|count| *count <= 65_536)
            .ok_or(SpoolError::MemoryLimit)?;
        let mut bytes = std::mem::size_of::<RowSchema>();
        for (key, value) in object {
            let key_bytes = key
                .len()
                .saturating_add(std::mem::align_of::<usize>() - 1)
                .saturating_div(std::mem::align_of::<usize>())
                .saturating_mul(std::mem::align_of::<usize>())
                .saturating_add(2 * std::mem::size_of::<usize>());
            bytes = bytes
                .checked_add(std::mem::size_of::<crate::schema::FieldSchema>())
                .and_then(|bytes| bytes.checked_add(key_bytes))
                .ok_or(SpoolError::MemoryLimit)?;
            if let Value::Object(nested) = value {
                bytes = bytes
                    .checked_add(visit(nested, depth + 1, fields)?)
                    .ok_or(SpoolError::MemoryLimit)?;
            }
        }
        Ok(bytes)
    }

    let mut fields = 0;
    visit(object, 1, &mut fields)?
        .checked_add(2 * std::mem::size_of::<usize>())
        .ok_or(SpoolError::MemoryLimit)
}

fn delimiter_character(config: WriterConfig) -> char {
    match config.delimiter {
        crate::Delimiter::Comma => ',',
        crate::Delimiter::Tab => '\t',
        crate::Delimiter::Pipe => '|',
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{self, Write as _},
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };

    use tq_core::{Value, presentation::ColorPalette};

    use super::{
        ArrayPreparationConfig, PreparationArena, PreparationLimits, PreparedArray,
        PublicationBuffer, PublicationError, SpoolError,
    };
    use crate::WriterConfig;

    fn strip_sgr(bytes: &[u8]) -> Vec<u8> {
        let mut output = Vec::with_capacity(bytes.len());
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index..].starts_with(b"\x1b[") {
                index += 2;
                while index < bytes.len() && bytes[index] != b'm' {
                    index += 1;
                }
                assert!(index < bytes.len(), "unterminated SGR");
                index += 1;
            } else {
                output.push(bytes[index]);
                index += 1;
            }
        }
        output
    }

    #[test]
    fn threshold_transition_preserves_tabular_schema_and_cleans_up() {
        let directory = tempfile::tempdir().unwrap();
        let mut prepared = PreparedArray::in_arena(
            ArrayPreparationConfig {
                memory_threshold_bytes: 1,
                maximum_spool_bytes: 1024 * 1024,
                spool_directory: directory.path().to_owned(),
                allow_spool: true,
            },
            PreparationArena::new(PreparationLimits::default()),
        );
        for json in [r#"{"id":1,"name":"Ada"}"#, r#"{"name":"Bob","id":2}"#] {
            prepared
                .push(&serde_json::from_str::<Value>(json).unwrap())
                .unwrap();
        }
        assert!(prepared.spooled());
        let path = prepared.spool_path().unwrap().to_owned();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;

            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        let mut output = Vec::new();
        prepared
            .write_to(&mut output, WriterConfig::default())
            .unwrap();
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "[2]{id,name}:\n  1,Ada\n  2,Bob"
        );
        drop(prepared);
        assert!(!path.exists());
    }

    #[test]
    fn colored_prepared_replay_preserves_spooled_layouts() {
        let directory = tempfile::tempdir().unwrap();
        let config = ArrayPreparationConfig {
            memory_threshold_bytes: 1,
            maximum_spool_bytes: 1024 * 1024,
            spool_directory: directory.path().to_owned(),
            allow_spool: true,
        };
        let palette = ColorPalette::from_jq_colors("10:11:12:13:14:15:16:17");

        let mut table = PreparedArray::in_arena(
            config.clone(),
            PreparationArena::new(PreparationLimits::default()),
        );
        table
            .push(&serde_json::from_str::<Value>(r#"{"name":"a,b","count":1}"#).unwrap())
            .unwrap();
        table
            .push(&serde_json::from_str::<Value>(r#"{"name":"c,d","count":2}"#).unwrap())
            .unwrap();
        let mut plain_table = Vec::new();
        let mut colored_table = Vec::new();
        table
            .write_to(&mut plain_table, WriterConfig::default())
            .unwrap();
        table
            .write_to_colored(&mut colored_table, WriterConfig::default(), Some(&palette))
            .unwrap();
        assert!(table.spooled());
        assert_eq!(strip_sgr(&colored_table), plain_table);
        assert!(
            colored_table
                .windows(b"\x1b[16m\"\x1b[0m".len())
                .any(|window| window == b"\x1b[16m\"\x1b[0m")
        );

        let mut expanded =
            PreparedArray::in_arena(config, PreparationArena::new(PreparationLimits::default()));
        expanded
            .push(&serde_json::from_str::<Value>(r#"{"name":"a,b"}"#).unwrap())
            .unwrap();
        expanded.push(&Value::Bool(true)).unwrap();
        let mut plain_expanded = Vec::new();
        let mut colored_expanded = Vec::new();
        expanded
            .write_to(&mut plain_expanded, WriterConfig::default())
            .unwrap();
        expanded
            .write_to_colored(
                &mut colored_expanded,
                WriterConfig::default(),
                Some(&palette),
            )
            .unwrap();
        assert_eq!(strip_sgr(&colored_expanded), plain_expanded);
    }

    #[test]
    fn later_schema_change_falls_back_without_losing_prior_values() {
        let mut prepared = PreparedArray::new(ArrayPreparationConfig::default());
        prepared
            .push(&serde_json::from_str::<Value>(r#"{"id":1}"#).unwrap())
            .unwrap();
        prepared.push(&Value::Bool(true)).unwrap();
        let mut output = Vec::new();
        prepared
            .write_to(&mut output, WriterConfig::default())
            .unwrap();
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "[2]:\n  - id: 1\n  - true"
        );
    }

    #[test]
    fn recursive_tabular_schema_matches_nested_fields_independent_of_row_order() {
        let config = ArrayPreparationConfig {
            memory_threshold_bytes: 1,
            ..ArrayPreparationConfig::default()
        };
        let arena = PreparationArena::new(PreparationLimits {
            memory_bytes: 4096,
            ..PreparationLimits::default()
        });
        let mut prepared = PreparedArray::in_arena(config, arena);
        prepared
            .push(
                &serde_json::from_str::<Value>(r#"{"user":{"name":"Ada","age":36},"active":true}"#)
                    .unwrap(),
            )
            .unwrap();
        prepared
            .push(
                &serde_json::from_str::<Value>(
                    r#"{"active":false,"user":{"age":85,"name":"Grace"}}"#,
                )
                .unwrap(),
            )
            .unwrap();
        assert!(prepared.spooled());
        let mut output = Vec::new();
        prepared
            .write_to(&mut output, WriterConfig::default())
            .unwrap();
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "[2]{user{name,age},active}:\n  Ada,36,true\n  Grace,85,false"
        );
    }

    #[test]
    fn disabled_and_limited_spools_fail_before_output() {
        let mut disabled = PreparedArray::new(ArrayPreparationConfig {
            memory_threshold_bytes: 0,
            allow_spool: false,
            ..ArrayPreparationConfig::default()
        });
        assert!(matches!(
            disabled.push(&Value::Null),
            Err(SpoolError::Disabled)
        ));

        let mut limited = PreparedArray::new(ArrayPreparationConfig {
            memory_threshold_bytes: 0,
            maximum_spool_bytes: 1,
            ..ArrayPreparationConfig::default()
        });
        assert!(matches!(limited.push(&Value::Null), Err(SpoolError::Limit)));
    }

    #[test]
    fn memory_records_do_not_consume_spool_quota() {
        let mut prepared = PreparedArray::new(ArrayPreparationConfig {
            maximum_spool_bytes: 0,
            ..ArrayPreparationConfig::default()
        });
        prepared.push(&Value::Null).unwrap();
        assert!(!prepared.spooled());
        assert_eq!(prepared.len(), 1);
    }

    #[test]
    fn simultaneous_arrays_cannot_bypass_shared_memory_or_spill_denial() {
        for allow_spool in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let arena = PreparationArena::new(PreparationLimits {
                memory_bytes: 768,
                spool_bytes: if allow_spool { 0 } else { 4096 },
                ..PreparationLimits::default()
            });
            let config = ArrayPreparationConfig {
                memory_threshold_bytes: 4096,
                maximum_spool_bytes: 4096,
                spool_directory: directory.path().to_owned(),
                allow_spool,
            };
            let mut first = PreparedArray::in_arena(config.clone(), arena.clone());
            let mut second = PreparedArray::in_arena(config, arena.clone());
            let payload = Value::String("x".repeat(512).into());
            first.push(&payload).unwrap();
            assert!(!first.spooled());
            let error = second.push(&payload).unwrap_err();
            if allow_spool {
                assert!(matches!(error, SpoolError::Limit));
            } else {
                assert!(matches!(error, SpoolError::Disabled));
            }
            assert!(arena.observations().memory_high_water_bytes <= 768);
        }
    }

    #[test]
    fn arena_enforces_nesting_and_output_limits() {
        let arena = PreparationArena::new(PreparationLimits {
            output_bytes: 4,
            nesting: 2,
            ..PreparationLimits::default()
        });
        let first = arena.enter().unwrap();
        let second = arena.enter().unwrap();
        assert!(matches!(arena.enter(), Err(SpoolError::NestingLimit)));
        assert_eq!(arena.observations().nesting_high_water, 2);
        assert!(arena.record_output(4).is_ok());
        assert!(matches!(
            arena.record_output(1),
            Err(SpoolError::OutputLimit)
        ));
        drop((first, second));
        assert!(arena.enter().is_ok());
    }

    #[test]
    fn cancellation_stops_replay_and_drop_cleans_private_spool() {
        let directory = tempfile::tempdir().unwrap();
        let cancellation = Arc::new(AtomicBool::new(false));
        let mut prepared = PreparedArray::new(ArrayPreparationConfig {
            memory_threshold_bytes: 0,
            spool_directory: directory.path().to_owned(),
            ..ArrayPreparationConfig::default()
        })
        .with_cancellation(Arc::clone(&cancellation));
        prepared.push(&Value::Null).unwrap();
        let path = prepared.spool_path().unwrap().to_owned();
        cancellation.store(true, Ordering::Relaxed);

        assert!(matches!(
            prepared.write_to(Vec::new(), WriterConfig::default()),
            Err(SpoolError::Cancelled)
        ));
        drop(prepared);
        assert!(!path.exists());
    }

    #[test]
    fn atomic_publication_spools_and_rejects_bad_cardinality_without_output() {
        let directory = tempfile::tempdir().unwrap();
        let arena = PreparationArena::new(PreparationLimits {
            memory_bytes: 4,
            spool_bytes: 1024,
            output_bytes: 1024,
            ..PreparationLimits::default()
        });
        let mut publication = PublicationBuffer::new(
            ArrayPreparationConfig {
                memory_threshold_bytes: 4,
                maximum_spool_bytes: 1024,
                spool_directory: directory.path().to_owned(),
                allow_spool: true,
            },
            arena.clone(),
        );
        publication.write_all(b"abcdef").unwrap();
        assert!(publication.spooled());
        let path = publication.spool_path().unwrap().to_owned();
        let mut output = Vec::new();
        assert!(matches!(
            publication.publish_single(&mut output, 2),
            Err(PublicationError::Cardinality(_))
        ));
        assert_eq!(output, [] as [u8; 0]);

        publication.publish_single(&mut output, 1).unwrap();
        assert_eq!(output, b"abcdef");
        assert_eq!(arena.observations().output_bytes, 6);
        assert_eq!(arena.observations().spool_bytes_written, 6);
        assert_eq!(arena.observations().spool_bytes_replayed, 6);
        drop(publication);
        assert!(!path.exists());
    }

    #[test]
    fn colored_publication_attempts_reset_after_partial_non_broken_pipe() {
        let mut publication = PublicationBuffer::new(
            ArrayPreparationConfig::default(),
            PreparationArena::new(PreparationLimits::default()),
        );
        publication.write_all(b"\x1b[31mvalue\x1b[0m").unwrap();
        let palette = ColorPalette::default();
        let mut sink = FailAfterPartial {
            bytes: Vec::new(),
            failed: false,
        };

        assert!(matches!(
            publication.publish_single_colored(&mut sink, 1, Some(&palette)),
            Err(PublicationError::Io(_))
        ));
        assert!(sink.bytes.ends_with(b"\x1b[0m"));
    }

    struct FailAfterPartial {
        bytes: Vec<u8>,
        failed: bool,
    }

    impl io::Write for FailAfterPartial {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if !self.failed {
                if self.bytes.is_empty() {
                    let count = bytes.len().min(3);
                    self.bytes.extend_from_slice(&bytes[..count]);
                    return Ok(count);
                }
                self.failed = true;
                return Err(io::Error::other("injected partial failure"));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn rejected_colored_publication_does_not_emit_a_styling_only_reset() {
        struct RejectFirst {
            calls: usize,
        }
        impl io::Write for RejectFirst {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                self.calls += 1;
                if self.calls == 1 {
                    Err(io::Error::other("rejected before publication"))
                } else {
                    Ok(bytes.len())
                }
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        for memory_threshold_bytes in [0, 1024] {
            let mut publication = PublicationBuffer::new(
                ArrayPreparationConfig {
                    memory_threshold_bytes,
                    ..ArrayPreparationConfig::default()
                },
                PreparationArena::new(PreparationLimits::default()),
            );
            publication.write_all(b"\x1b[31mvalue\x1b[0m").unwrap();
            let mut sink = RejectFirst { calls: 0 };
            let error = publication
                .publish_single_colored(&mut sink, 1, Some(&ColorPalette::default()))
                .unwrap_err();
            assert!(error.to_string().contains("rejected before publication"));
            assert_eq!(sink.calls, 1, "no reset when no payload was published");
        }
    }
}
