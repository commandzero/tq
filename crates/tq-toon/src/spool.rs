//! Bounded unknown-length array preparation with secure disk transition.

use std::{
    collections::{BTreeMap, BTreeSet, hash_map::DefaultHasher},
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
use tq_core::Value;

use crate::{ScalarToken, WriterConfig, replay, writer};

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
    /// Number of object key-index runs spilled for external merge.
    pub object_index_spills: u64,
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

    fn record_object_index_spill(&self) {
        let mut state = self.state();
        state.observations.object_index_spills =
            state.observations.object_index_spills.saturating_add(1);
    }

    fn record_array_preparation(&self) {
        let mut state = self.state();
        state.observations.array_preparations =
            state.observations.array_preparations.saturating_add(1);
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
    spool: Option<Spool>,
    bytes: u64,
    published: bool,
}

impl PublicationBuffer {
    /// Creates an empty atomic publication buffer.
    #[must_use]
    pub fn new(config: ArrayPreparationConfig, arena: PreparationArena) -> Self {
        Self {
            config,
            arena,
            memory: Vec::new(),
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
        match result_count {
            0 => return Err(crate::CardinalityError::Zero.into()),
            1 => {}
            _ => return Err(crate::CardinalityError::Multiple.into()),
        }
        self.publish(output)
    }

    /// Publishes prepared bytes after the caller validates them.
    pub(crate) fn publish<W: Write>(&mut self, output: &mut W) -> Result<(), PublicationError> {
        if self.published {
            return Err(SpoolError::Decode("publication buffer already committed").into());
        }
        if let Some(spool) = &mut self.spool {
            spool.file.flush()?;
            spool.file.seek(SeekFrom::Start(0))?;
            let mut copied = 0_u64;
            let mut chunk = vec![0_u8; 64 * 1024];
            loop {
                let read = spool.file.read(&mut chunk)?;
                if read == 0 {
                    break;
                }
                output.write_all(&chunk[..read])?;
                copied = copied.saturating_add(read as u64);
            }
            self.arena.replayed_spool(copied);
        } else {
            output.write_all(&self.memory)?;
        }
        self.published = true;
        Ok(())
    }

    fn append(&mut self, bytes: &[u8]) -> Result<(), SpoolError> {
        if bytes.is_empty() {
            return Ok(());
        }
        let next = self.bytes.saturating_add(bytes.len() as u64);
        if next > self.config.maximum_spool_bytes {
            return Err(SpoolError::Limit);
        }
        self.arena.record_output(bytes.len() as u64)?;
        if self.spool.is_none()
            && (self.memory.len().saturating_add(bytes.len()) > self.config.memory_threshold_bytes
                || !self.arena.retain_memory(bytes.len()))
        {
            self.transition_to_disk()?;
        }
        if let Some(spool) = &mut self.spool {
            if !self.arena.can_write_spool(bytes.len() as u64) {
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
        if !self.arena.can_write_spool(bytes) {
            return Err(SpoolError::Limit);
        }
        let mut spool = create_spool(&self.config.spool_directory)?;
        spool.file.write_all(&self.memory)?;
        self.arena.wrote_spool(bytes);
        self.arena.release_memory(self.memory.len());
        self.memory.clear();
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

impl Drop for PublicationBuffer {
    fn drop(&mut self) {
        self.arena.release_memory(self.memory.len());
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
    Tabular(Vec<Arc<str>>),
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
        self.arena.record_object_index_spill();
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
        self.arena.record_object_index_spill();
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

/// Bounded JSON object normalization with first-position and last-value semantics.
#[derive(Debug)]
pub struct PreparedObject {
    config: ArrayPreparationConfig,
    arena: PreparationArena,
    values: ValueStore,
    index: BTreeMap<Arc<str>, ObjectIndexEntry>,
    index_bytes: usize,
    index_runs: Vec<Spool>,
    members: u64,
    index_spills: u64,
}

#[derive(Clone, Copy, Debug)]
enum ValueLocation {
    Memory(usize),
    Spool(u64),
}

#[derive(Debug)]
struct ValueStore {
    memory: Vec<Vec<u8>>,
    memory_bytes: usize,
    spool: Option<Spool>,
}

#[derive(Clone, Debug)]
struct ObjectIndexEntry {
    first_position: u64,
    last_position: u64,
    value: ValueLocation,
}

#[derive(Clone, Debug)]
struct MergedMember {
    key: Arc<str>,
    first_position: u64,
    value_offset: u64,
}

impl PreparedObject {
    /// Creates an object preparation charged to an existing result arena.
    #[must_use]
    pub fn new(config: ArrayPreparationConfig, arena: PreparationArena) -> Self {
        Self {
            config,
            arena,
            values: ValueStore {
                memory: Vec::new(),
                memory_bytes: 0,
                spool: None,
            },
            index: BTreeMap::new(),
            index_bytes: 0,
            index_runs: Vec::new(),
            members: 0,
            index_spills: 0,
        }
    }

    /// Adds one encountered member, replacing any earlier value for its key.
    ///
    /// # Errors
    ///
    /// Returns a bounded preparation or temporary-file failure.
    pub fn push(&mut self, key: impl Into<Arc<str>>, value: &Value) -> Result<(), SpoolError> {
        let key = key.into();
        let existing = self.index.contains_key(&key);
        let charge = object_index_charge(&key);
        let mut flush_single = false;
        if !existing && !self.arena.retain_memory(charge) {
            self.ensure_values_spooled()?;
            self.flush_index_run()?;
            if !self.arena.retain_memory(charge) {
                flush_single = true;
            }
        }
        if !existing && !flush_single {
            self.index_bytes = self.index_bytes.saturating_add(charge);
        }

        let location = self.store_value(replay::encode(value))?;
        let position = self.members;
        self.members = self.members.saturating_add(1);
        if let Some(entry) = self.index.get_mut(&key) {
            entry.last_position = position;
            entry.value = location;
            return Ok(());
        }
        self.index.insert(
            key,
            ObjectIndexEntry {
                first_position: position,
                last_position: position,
                value: location,
            },
        );
        if flush_single {
            self.ensure_values_spooled()?;
            self.flush_index_run()?;
        }
        Ok(())
    }

    /// Number of encountered members before duplicate normalization.
    #[must_use]
    pub const fn encountered_len(&self) -> u64 {
        self.members
    }

    /// Number of key-index runs written to temporary storage.
    #[must_use]
    pub const fn index_spills(&self) -> u64 {
        self.index_spills
    }

    /// Replays normalized members in first-encounter order.
    ///
    /// # Errors
    ///
    /// Returns structural replay, merge, or consumer failures.
    pub fn for_each_member(
        &mut self,
        mut consume: impl FnMut(&str, &Value) -> Result<(), SpoolError>,
    ) -> Result<(), SpoolError> {
        if self.index_runs.is_empty() {
            let mut members = self
                .index
                .iter()
                .map(|(key, entry)| (entry.first_position, Arc::clone(key), entry.value))
                .collect::<Vec<_>>();
            members.sort_by_key(|member| member.0);
            for (_, key, location) in members {
                let value = self.read_value(location)?;
                consume(&key, &value)?;
            }
            return Ok(());
        }

        self.ensure_values_spooled()?;
        self.flush_index_run()?;
        let mut index_runs = std::mem::take(&mut self.index_runs);
        let mut position_runs = Vec::new();
        let mut position_chunk = Vec::new();
        let mut position_bytes = 0_usize;
        let arena = self.arena.clone();
        let directory = self.config.spool_directory.clone();
        for run in &index_runs {
            arena.replayed_spool(run.file.metadata()?.len());
        }
        merge_index_runs(&mut index_runs, |member| {
            let charge = object_index_charge(&member.key);
            if !arena.retain_memory(charge) {
                if !position_chunk.is_empty() {
                    position_runs.push(flush_position_run(
                        &mut position_chunk,
                        position_bytes,
                        &directory,
                        &arena,
                    )?);
                    position_bytes = 0;
                }
                if !arena.retain_memory(charge) {
                    position_chunk.push(member);
                    position_runs.push(flush_position_run(
                        &mut position_chunk,
                        0,
                        &directory,
                        &arena,
                    )?);
                    return Ok(());
                }
            }
            position_bytes = position_bytes.saturating_add(charge);
            position_chunk.push(member);
            Ok(())
        })?;

        if position_runs.is_empty() {
            position_chunk.sort_by_key(|member| member.first_position);
            for member in position_chunk {
                let value = self.read_value(ValueLocation::Spool(member.value_offset))?;
                consume(&member.key, &value)?;
            }
            self.arena.release_memory(position_bytes);
        } else {
            if !position_chunk.is_empty() {
                position_runs.push(flush_position_run(
                    &mut position_chunk,
                    position_bytes,
                    &directory,
                    &arena,
                )?);
            }
            for run in &position_runs {
                self.arena.replayed_spool(run.file.metadata()?.len());
            }
            merge_position_runs(&mut position_runs, |member| {
                let value = self.read_value(ValueLocation::Spool(member.value_offset))?;
                consume(&member.key, &value)
            })?;
        }
        Ok(())
    }

    fn store_value(&mut self, encoded: Vec<u8>) -> Result<ValueLocation, SpoolError> {
        let framed = encoded.len().saturating_add(8);
        if self.values.spool.is_none()
            && self.values.memory_bytes.saturating_add(framed) <= self.config.memory_threshold_bytes
            && self.arena.retain_memory(framed)
        {
            let index = self.values.memory.len();
            self.values.memory.push(encoded);
            self.values.memory_bytes = self.values.memory_bytes.saturating_add(framed);
            return Ok(ValueLocation::Memory(index));
        }
        self.ensure_values_spooled()?;
        let spool = self.values.spool.as_mut().expect("spool created");
        let offset = spool.file.seek(SeekFrom::End(0))?;
        let framed = u64::try_from(framed).unwrap_or(u64::MAX);
        if !self.arena.can_write_spool(framed) {
            return Err(SpoolError::Limit);
        }
        write_record(&mut spool.file, &encoded)?;
        self.arena.wrote_spool(framed);
        Ok(ValueLocation::Spool(offset))
    }

    fn ensure_values_spooled(&mut self) -> Result<(), SpoolError> {
        if self.values.spool.is_some() {
            return Ok(());
        }
        if !self.config.allow_spool {
            return Err(SpoolError::Disabled);
        }
        let mut spool = create_spool(&self.config.spool_directory)?;
        let bytes = u64::try_from(self.values.memory_bytes).unwrap_or(u64::MAX);
        if !self.arena.can_write_spool(bytes) {
            return Err(SpoolError::Limit);
        }
        let mut offsets = Vec::with_capacity(self.values.memory.len());
        for record in &self.values.memory {
            offsets.push(spool.file.seek(SeekFrom::End(0))?);
            write_record(&mut spool.file, record)?;
        }
        for entry in self.index.values_mut() {
            if let ValueLocation::Memory(index) = entry.value {
                entry.value = ValueLocation::Spool(offsets[index]);
            }
        }
        self.arena.wrote_spool(bytes);
        self.arena.release_memory(self.values.memory_bytes);
        self.values.memory.clear();
        self.values.memory_bytes = 0;
        self.values.spool = Some(spool);
        Ok(())
    }

    fn flush_index_run(&mut self) -> Result<(), SpoolError> {
        if self.index.is_empty() {
            return Ok(());
        }
        if !self.config.allow_spool {
            return Err(SpoolError::Disabled);
        }
        let mut run = create_spool(&self.config.spool_directory)?;
        let mut written = 0_u64;
        for (key, entry) in &self.index {
            let ValueLocation::Spool(offset) = entry.value else {
                return Err(SpoolError::Decode(
                    "object index references memory during spill",
                ));
            };
            written = written.saturating_add(write_index_entry(
                &mut run.file,
                key,
                entry.first_position,
                entry.last_position,
                offset,
            )?);
        }
        if !self.arena.can_write_spool(written) {
            return Err(SpoolError::Limit);
        }
        self.arena.wrote_spool(written);
        self.index.clear();
        self.arena.release_memory(self.index_bytes);
        self.index_bytes = 0;
        self.index_runs.push(run);
        self.index_spills = self.index_spills.saturating_add(1);
        self.arena.record_object_index_spill();
        Ok(())
    }

    fn read_value(&mut self, location: ValueLocation) -> Result<Value, SpoolError> {
        let encoded = match location {
            ValueLocation::Memory(index) => self
                .values
                .memory
                .get(index)
                .ok_or(SpoolError::Decode("missing in-memory object value"))?
                .clone(),
            ValueLocation::Spool(offset) => {
                let spool = self
                    .values
                    .spool
                    .as_mut()
                    .ok_or(SpoolError::Decode("missing object value spool"))?;
                let encoded = read_record_at(&mut spool.file, offset)?;
                self.arena.replayed_spool(
                    u64::try_from(encoded.len().saturating_add(8)).unwrap_or(u64::MAX),
                );
                encoded
            }
        };
        replay::decode(&encoded).map_err(SpoolError::Decode)
    }
}

impl Drop for PreparedObject {
    fn drop(&mut self) {
        self.arena.release_memory(self.values.memory_bytes);
        self.arena.release_memory(self.index_bytes);
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
        let layout = self.next_layout(value);
        self.push_encoded(replay::encode(value), layout)
    }

    pub(crate) fn push_scalar(&mut self, value: ScalarToken<'_>) -> Result<(), SpoolError> {
        let layout = match self.layout {
            Layout::Empty | Layout::Scalars => Layout::Scalars,
            Layout::Expanded | Layout::Tabular(_) => Layout::Expanded,
        };
        self.push_encoded(replay::encode_scalar(value), layout)
    }

    fn push_encoded(&mut self, encoded: Vec<u8>, layout: Layout) -> Result<(), SpoolError> {
        self.check_cancelled()?;
        let framed_usize = encoded.len().saturating_add(8);
        let framed = u64::try_from(encoded.len())
            .unwrap_or(u64::MAX)
            .saturating_add(8);
        if self.framed_bytes.saturating_add(framed) > self.config.maximum_spool_bytes {
            return Err(SpoolError::Limit);
        }
        // Composite elements need transient arena memory while they are decoded.
        // Spool them as soon as the first complete element is available so the
        // parent array cannot consume the entire shared budget and starve the
        // next element before it has a chance to spill.
        if self.spool.is_none()
            && self.config.allow_spool
            && matches!(layout, Layout::Tabular(_) | Layout::Expanded)
        {
            self.transition_to_disk()?;
        }
        let local_memory_available =
            self.memory_bytes.saturating_add(framed_usize) <= self.config.memory_threshold_bytes;
        if self.spool.is_none()
            && (!local_memory_available || !self.arena.retain_memory(framed_usize))
        {
            self.transition_to_disk()?;
        }
        if let Some(spool) = &mut self.spool {
            if !self.arena.can_write_spool(framed) {
                return Err(SpoolError::Limit);
            }
            write_record(&mut spool.file, &encoded)?;
            self.arena.wrote_spool(framed);
        } else {
            self.memory_bytes = self.memory_bytes.saturating_add(framed_usize);
            self.memory.push(encoded);
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
        mut output: W,
        writer_config: WriterConfig,
    ) -> Result<(), SpoolError> {
        let layout = self.layout.clone();
        write!(
            output,
            "[{}{}]",
            self.count,
            delimiter_suffix(writer_config)
        )?;
        match layout {
            Layout::Empty => output.write_all(b":")?,
            Layout::Scalars => {
                output.write_all(b": ")?;
                let mut index = 0_usize;
                self.for_each_record(|record| {
                    if index != 0 {
                        write!(output, "{}", delimiter_character(writer_config))?;
                    }
                    let value = replay::decode_scalar(record).map_err(SpoolError::Decode)?;
                    output.write_all(
                        writer::encode_array_scalar_token(value, writer_config).as_bytes(),
                    )?;
                    index += 1;
                    Ok(())
                })?;
            }
            Layout::Tabular(fields) => {
                output.write_all(b"{")?;
                for (index, field) in fields.iter().enumerate() {
                    if index != 0 {
                        write!(output, "{}", delimiter_character(writer_config))?;
                    }
                    output.write_all(writer::render_key(field).as_bytes())?;
                }
                output.write_all(b"}:")?;
                self.for_each_value(|value| {
                    let Value::Object(object) = value else {
                        unreachable!("layout tracked during preparation")
                    };
                    output.write_all(b"\n")?;
                    output.write_all(" ".repeat(writer_config.indent_size).as_bytes())?;
                    output.write_all(
                        writer::encode_tabular_row(object, &fields, writer_config).as_bytes(),
                    )?;
                    Ok(())
                })?;
            }
            Layout::Expanded => {
                output.write_all(b":")?;
                self.for_each_value(|value| {
                    output.write_all(b"\n")?;
                    let item = writer::encode_list_item(value, writer_config);
                    for (line_index, line) in item.lines().enumerate() {
                        if line_index != 0 {
                            output.write_all(b"\n")?;
                        }
                        output.write_all(" ".repeat(writer_config.indent_size).as_bytes())?;
                        output.write_all(line.as_bytes())?;
                    }
                    Ok(())
                })?;
            }
        }
        Ok(())
    }

    fn next_layout(&self, value: &Value) -> Layout {
        match &self.layout {
            Layout::Empty | Layout::Scalars if scalar(value) => Layout::Scalars,
            Layout::Empty => tabular_schema(value).map_or(Layout::Expanded, Layout::Tabular),
            Layout::Tabular(fields) if matches_schema(value, fields) => {
                Layout::Tabular(fields.clone())
            }
            Layout::Expanded | Layout::Scalars | Layout::Tabular(_) => Layout::Expanded,
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
                let mut bytes = vec![0; length];
                spool.file.read_exact(&mut bytes)?;
                consume(&bytes)?;
            }
            self.arena.replayed_spool(self.framed_bytes);
        } else {
            for bytes in &self.memory {
                self.check_cancelled()?;
                consume(bytes)?;
            }
        }
        Ok(())
    }

    fn check_cancelled(&self) -> Result<(), SpoolError> {
        check_cancelled(self.cancellation.as_deref())
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
    }
}

fn write_record(mut writer: impl Write, bytes: &[u8]) -> Result<(), io::Error> {
    writer.write_all(&(bytes.len() as u64).to_le_bytes())?;
    writer.write_all(bytes)
}

fn read_record_at(file: &mut File, offset: u64) -> Result<Vec<u8>, SpoolError> {
    file.seek(SeekFrom::Start(offset))?;
    let mut length = [0_u8; 8];
    file.read_exact(&mut length)?;
    let length = usize::try_from(u64::from_le_bytes(length)).map_err(|_| SpoolError::Limit)?;
    let mut bytes = vec![0; length];
    file.read_exact(&mut bytes)?;
    Ok(bytes)
}

fn object_index_charge(key: &str) -> usize {
    key.len()
        .saturating_add(std::mem::size_of::<ObjectIndexEntry>())
        .saturating_add(std::mem::size_of::<Arc<str>>())
}

fn write_index_entry(
    mut writer: impl Write,
    key: &str,
    first_position: u64,
    last_position: u64,
    value_offset: u64,
) -> Result<u64, io::Error> {
    let key_length = u64::try_from(key.len()).unwrap_or(u64::MAX);
    writer.write_all(&key_length.to_le_bytes())?;
    writer.write_all(key.as_bytes())?;
    writer.write_all(&first_position.to_le_bytes())?;
    writer.write_all(&last_position.to_le_bytes())?;
    writer.write_all(&value_offset.to_le_bytes())?;
    Ok(32_u64.saturating_add(key_length))
}

#[derive(Debug)]
struct IndexRunReader {
    file: File,
    head: Option<IndexRunEntry>,
}

#[derive(Debug)]
struct IndexRunEntry {
    key: Arc<str>,
    first_position: u64,
    last_position: u64,
    value_offset: u64,
}

impl IndexRunReader {
    fn new(run: &mut Spool) -> Result<Self, SpoolError> {
        run.file.flush()?;
        let mut reader = Self {
            file: run.file.try_clone()?,
            head: None,
        };
        reader.file.seek(SeekFrom::Start(0))?;
        reader.advance()?;
        Ok(reader)
    }

    fn advance(&mut self) -> Result<(), SpoolError> {
        let mut length = [0_u8; 8];
        if self.file.read(&mut length[..1])? == 0 {
            self.head = None;
            return Ok(());
        }
        self.file.read_exact(&mut length[1..])?;
        let length = usize::try_from(u64::from_le_bytes(length)).map_err(|_| SpoolError::Limit)?;
        let mut key = vec![0; length];
        self.file.read_exact(&mut key)?;
        let key = String::from_utf8(key)
            .map_err(|_| SpoolError::Decode("invalid object index key UTF-8"))?;
        let first_position = read_u64(&mut self.file)?;
        let last_position = read_u64(&mut self.file)?;
        let value_offset = read_u64(&mut self.file)?;
        self.head = Some(IndexRunEntry {
            key: Arc::from(key),
            first_position,
            last_position,
            value_offset,
        });
        Ok(())
    }
}

fn read_u64(mut reader: impl Read) -> Result<u64, io::Error> {
    let mut bytes = [0_u8; 8];
    reader.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

fn merge_index_runs(
    runs: &mut [Spool],
    mut consume: impl FnMut(MergedMember) -> Result<(), SpoolError>,
) -> Result<(), SpoolError> {
    let mut readers = runs
        .iter_mut()
        .map(IndexRunReader::new)
        .collect::<Result<Vec<_>, _>>()?;
    while let Some(key) = readers
        .iter()
        .filter_map(|reader| reader.head.as_ref().map(|entry| Arc::clone(&entry.key)))
        .min()
    {
        let mut first_position = u64::MAX;
        let mut last_position = 0_u64;
        let mut value_offset = 0_u64;
        for reader in &mut readers {
            if reader.head.as_ref().is_some_and(|entry| entry.key == key) {
                let entry = reader.head.take().expect("matching head");
                first_position = first_position.min(entry.first_position);
                if entry.last_position >= last_position {
                    last_position = entry.last_position;
                    value_offset = entry.value_offset;
                }
                reader.advance()?;
            }
        }
        consume(MergedMember {
            key,
            first_position,
            value_offset,
        })?;
    }
    Ok(())
}

fn flush_position_run(
    entries: &mut Vec<MergedMember>,
    charged_bytes: usize,
    directory: &Path,
    arena: &PreparationArena,
) -> Result<Spool, SpoolError> {
    entries.sort_by_key(|entry| entry.first_position);
    let expected = entries.iter().fold(0_u64, |bytes, entry| {
        bytes.saturating_add(24_u64.saturating_add(entry.key.len() as u64))
    });
    if !arena.can_write_spool(expected) {
        return Err(SpoolError::Limit);
    }
    let mut run = create_spool(directory)?;
    for entry in entries.iter() {
        run.file.write_all(&entry.first_position.to_le_bytes())?;
        run.file.write_all(&entry.value_offset.to_le_bytes())?;
        let key_length = u64::try_from(entry.key.len()).unwrap_or(u64::MAX);
        run.file.write_all(&key_length.to_le_bytes())?;
        run.file.write_all(entry.key.as_bytes())?;
    }
    arena.wrote_spool(expected);
    arena.release_memory(charged_bytes);
    entries.clear();
    Ok(run)
}

#[derive(Debug)]
struct PositionRunReader {
    file: File,
    head: Option<MergedMember>,
}

impl PositionRunReader {
    fn new(run: &mut Spool) -> Result<Self, SpoolError> {
        run.file.flush()?;
        let mut reader = Self {
            file: run.file.try_clone()?,
            head: None,
        };
        reader.file.seek(SeekFrom::Start(0))?;
        reader.advance()?;
        Ok(reader)
    }

    fn advance(&mut self) -> Result<(), SpoolError> {
        let mut first = [0_u8; 8];
        if self.file.read(&mut first[..1])? == 0 {
            self.head = None;
            return Ok(());
        }
        self.file.read_exact(&mut first[1..])?;
        let value_offset = read_u64(&mut self.file)?;
        let key_length =
            usize::try_from(read_u64(&mut self.file)?).map_err(|_| SpoolError::Limit)?;
        let mut key = vec![0; key_length];
        self.file.read_exact(&mut key)?;
        let key = String::from_utf8(key)
            .map_err(|_| SpoolError::Decode("invalid position-run key UTF-8"))?;
        self.head = Some(MergedMember {
            key: Arc::from(key),
            first_position: u64::from_le_bytes(first),
            value_offset,
        });
        Ok(())
    }
}

fn merge_position_runs(
    runs: &mut [Spool],
    mut consume: impl FnMut(MergedMember) -> Result<(), SpoolError>,
) -> Result<(), SpoolError> {
    let mut readers = runs
        .iter_mut()
        .map(PositionRunReader::new)
        .collect::<Result<Vec<_>, _>>()?;
    while let Some((reader_index, _)) = readers
        .iter()
        .enumerate()
        .filter_map(|(index, reader)| {
            reader
                .head
                .as_ref()
                .map(|entry| (index, entry.first_position))
        })
        .min_by_key(|(_, position)| *position)
    {
        let member = readers[reader_index].head.take().expect("selected head");
        consume(member)?;
        readers[reader_index].advance()?;
    }
    Ok(())
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

fn tabular_schema(value: &Value) -> Option<Vec<Arc<str>>> {
    let Value::Object(object) = value else {
        return None;
    };
    if object.is_empty() || object.values().any(|value| !scalar(value)) {
        None
    } else {
        Some(object.keys().cloned().collect())
    }
}

fn matches_schema(value: &Value, fields: &[Arc<str>]) -> bool {
    let Value::Object(object) = value else {
        return false;
    };
    object.len() == fields.len()
        && fields.iter().all(|field| object.contains_key(field))
        && object.values().all(scalar)
}

fn delimiter_suffix(config: WriterConfig) -> &'static str {
    match config.delimiter {
        crate::Delimiter::Comma => "",
        crate::Delimiter::Tab => "\t",
        crate::Delimiter::Pipe => "|",
    }
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
        io::Write as _,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };

    use tq_core::Value;

    use super::{
        ArrayPreparationConfig, PreparationArena, PreparationLimits, PreparedArray, PreparedObject,
        PublicationBuffer, PublicationError, SpoolError,
    };
    use crate::WriterConfig;

    #[test]
    fn threshold_transition_preserves_tabular_schema_and_cleans_up() {
        let directory = tempfile::tempdir().unwrap();
        let mut prepared = PreparedArray::new(ArrayPreparationConfig {
            memory_threshold_bytes: 1,
            maximum_spool_bytes: 1024 * 1024,
            spool_directory: directory.path().to_owned(),
            allow_spool: true,
        });
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
            maximum_spool_bytes: 1,
            ..ArrayPreparationConfig::default()
        });
        assert!(matches!(limited.push(&Value::Null), Err(SpoolError::Limit)));
    }

    #[test]
    fn active_arrays_share_one_memory_and_spool_ledger() {
        let directory = tempfile::tempdir().unwrap();
        let arena = PreparationArena::new(PreparationLimits {
            memory_bytes: 12,
            spool_bytes: 1024,
            ..PreparationLimits::default()
        });
        let config = ArrayPreparationConfig {
            memory_threshold_bytes: 1024,
            maximum_spool_bytes: 1024,
            spool_directory: directory.path().to_owned(),
            allow_spool: true,
        };
        let mut first = PreparedArray::in_arena(config.clone(), arena.clone());
        let mut second = PreparedArray::in_arena(config, arena.clone());
        first.push(&Value::Null).unwrap();
        second.push(&Value::Null).unwrap();

        assert!(!first.spooled());
        assert!(second.spooled());
        assert_eq!(arena.observations().memory_high_water_bytes, 9);
        assert_eq!(arena.observations().spool_bytes_written, 9);

        second
            .write_to(Vec::new(), WriterConfig::default())
            .unwrap();
        assert_eq!(arena.observations().spool_bytes_replayed, 9);
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
    fn object_normalization_keeps_first_position_and_last_value() {
        let arena = PreparationArena::new(PreparationLimits::default());
        let mut object = PreparedObject::new(ArrayPreparationConfig::default(), arena.clone());
        object
            .push("b", &serde_json::from_str::<Value>("1").unwrap())
            .unwrap();
        object
            .push("a", &serde_json::from_str::<Value>("2").unwrap())
            .unwrap();
        object
            .push("b", &serde_json::from_str::<Value>("3").unwrap())
            .unwrap();

        let mut members = Vec::new();
        object
            .for_each_member(|key, value| {
                members.push((key.to_owned(), value.to_string()));
                Ok(())
            })
            .unwrap();
        assert_eq!(
            members,
            [
                ("b".to_owned(), "3".to_owned()),
                ("a".to_owned(), "2".to_owned())
            ]
        );
        assert_eq!(object.index_spills(), 0);
    }

    #[test]
    fn object_index_sorted_runs_merge_duplicates_deterministically() {
        let directory = tempfile::tempdir().unwrap();
        let arena = PreparationArena::new(PreparationLimits {
            memory_bytes: 1,
            spool_bytes: 1024 * 1024,
            ..PreparationLimits::default()
        });
        let mut object = PreparedObject::new(
            ArrayPreparationConfig {
                memory_threshold_bytes: 1,
                maximum_spool_bytes: 1024 * 1024,
                spool_directory: directory.path().to_owned(),
                allow_spool: true,
            },
            arena.clone(),
        );
        for (key, number) in [("z", "1"), ("a", "2"), ("z", "3"), ("m", "4")] {
            object
                .push(key, &Value::Number(tq_core::Number::parse(number).unwrap()))
                .unwrap();
        }

        let mut members = Vec::new();
        object
            .for_each_member(|key, value| {
                members.push((key.to_owned(), value.to_string()));
                Ok(())
            })
            .unwrap();
        assert_eq!(
            members,
            [
                ("z".to_owned(), "3".to_owned()),
                ("a".to_owned(), "2".to_owned()),
                ("m".to_owned(), "4".to_owned())
            ]
        );
        assert!(object.index_spills() >= 4);
        assert!(arena.observations().spool_bytes_written > 0);
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
}
