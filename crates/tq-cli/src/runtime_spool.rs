//! Private bounded staging for runtime values produced by automatic input plans.
//!
//! This is deliberately separate from TOON preparation.  TOON preparation is
//! allowed to project runtime non-finite numbers for native output, while this
//! store must preserve the value that the VM would have received.

use std::{
    io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

#[cfg(test)]
use std::path::Path;

use tempfile::NamedTempFile;
use thiserror::Error;
use tq_core::{Number, NumberError, Object, Value};

const RECORD_HEADER_BYTES: usize = std::mem::size_of::<u64>();
const TAG_BYTES: usize = std::mem::size_of::<u8>();
/// Fixed per-file replay buffer size. Encoded-memory accounting excludes the
/// writer and reader buffers and reports this bounded overhead separately.
pub(crate) const REPLAY_BUFFER_BYTES: usize = 8192;
const NULL_TAG: u8 = 0;
const BOOL_TAG: u8 = 1;
const FINITE_NUMBER_TAG: u8 = 2;
const RUNTIME_NUMBER_TAG: u8 = 3;
const STRING_TAG: u8 = 4;
const ARRAY_TAG: u8 = 5;
const OBJECT_TAG: u8 = 6;

const PROJECTED_TAG: u8 = 1;
const ITEM_TAG: u8 = 2;
const BASE_TAG: u8 = 3;
const MAX_VARINT_BYTES: usize = 10;

/// Opaque location of one item in a single sealed root generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeRecordId {
    generation: u64,
    offset: u64,
}

/// One typed item retained until its containing JSON root has validated.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RuntimeSpoolItem {
    /// A value ready for direct output.
    Projected(Value),
    /// A selected item to pass to a VM plan.
    Item(Value),
    /// A base value for a VM iteration.
    Base(Value),
}

impl RuntimeSpoolItem {
    fn into_parts(self) -> (u8, Value) {
        match self {
            Self::Projected(value) => (PROJECTED_TAG, value),
            Self::Item(value) => (ITEM_TAG, value),
            Self::Base(value) => (BASE_TAG, value),
        }
    }

    fn from_parts(byte: u8, value: Value) -> Option<Self> {
        match byte {
            PROJECTED_TAG => Some(Self::Projected(value)),
            ITEM_TAG => Some(Self::Item(value)),
            BASE_TAG => Some(Self::Base(value)),
            _ => None,
        }
    }
}

/// Bounds for one private runtime-value staging store.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeSpoolConfig {
    /// Memory retained before the store moves to a private temporary file.
    pub memory_threshold_bytes: usize,
    /// Cumulative encoded bytes written over the store lifetime.
    pub maximum_total_bytes: u64,
    /// Cumulative bytes written to private temporary storage.
    pub maximum_spool_bytes: u64,
    /// Maximum encoded payload bytes in one item, excluding its record header.
    pub maximum_item_bytes: u64,
    /// Maximum active array/object nesting while replaying an item.
    pub maximum_depth: usize,
    /// Maximum decoded bytes in one retained string, key, or finite literal.
    pub maximum_token_bytes: usize,
    /// Maximum approximate heap bytes materialized while replaying one item.
    pub maximum_decoded_bytes: u64,
    /// Directory in which private temporary files are created.
    pub spool_directory: PathBuf,
    /// Whether moving from memory to a private temporary file is allowed.
    pub allow_spool: bool,
}

impl Default for RuntimeSpoolConfig {
    fn default() -> Self {
        Self {
            memory_threshold_bytes: 8 * 1024 * 1024,
            maximum_total_bytes: 8 * 1024 * 1024 * 1024,
            maximum_spool_bytes: 8 * 1024 * 1024 * 1024,
            maximum_item_bytes: 8 * 1024 * 1024 * 1024,
            maximum_depth: 256,
            maximum_token_bytes: 8 * 1024 * 1024,
            maximum_decoded_bytes: 256 * 1024 * 1024,
            spool_directory: std::env::temp_dir(),
            allow_spool: true,
        }
    }
}

/// Failure while staging or replaying a private runtime value.
#[derive(Debug, Error)]
pub(crate) enum RuntimeSpoolError {
    /// The caller's cancellation flag was set.
    #[error("runtime staging was cancelled")]
    Cancelled,
    /// The cumulative encoded-byte budget was exhausted.
    #[error("runtime staging exceeds its total byte limit")]
    TotalLimit,
    /// The cumulative temporary-file budget was exhausted.
    #[error("runtime staging exceeds its spool byte limit")]
    SpoolLimit,
    /// One encoded item exceeded its bound.
    #[error("runtime staging item exceeds its byte limit")]
    ItemLimit,
    /// A retained string, key, or finite literal exceeded its token bound.
    #[error("runtime staging token exceeds its byte limit")]
    TokenLimit,
    /// A nested array or object exceeded its bound.
    #[error("runtime staging exceeds its depth limit")]
    DepthLimit,
    /// Decoding one item would exceed its materialization bound.
    #[error("runtime staging exceeds its decoded-value byte limit")]
    DecodedLimit,
    /// Disk or memory I/O failed.
    #[error("runtime staging I/O failed: {0}")]
    Io(#[from] io::Error),
    /// A temporary-file transition was disabled.
    #[error("runtime staging exceeded memory and spooling is disabled")]
    SpoolDisabled,
    /// A private record was truncated or malformed.
    #[error("runtime staging record is invalid: {0}")]
    Decode(&'static str),
    /// A retained string or object key was not UTF-8.
    #[error("runtime staging string is not valid UTF-8")]
    Utf8,
    /// A finite literal failed the shared number grammar or envelope.
    #[error("runtime staging number is invalid: {0}")]
    Number(#[from] NumberError),
    /// A lifecycle call was made in the wrong state.
    #[error("runtime staging lifecycle error: {0}")]
    State(&'static str),
    /// The root-generation counter cannot advance without reusing an ID.
    #[error("runtime staging root-generation counter exhausted")]
    GenerationLimit,
    /// A record identifier came from another root generation.
    #[error("runtime staging record belongs to another root generation")]
    StaleRecord,
    /// A record identifier does not point inside the retained generation.
    #[error("runtime staging record offset is outside the retained generation")]
    RecordBounds,
}

/// Separates storage/decode failures from the caller's replay failure.
#[derive(Debug)]
pub(crate) enum RuntimeSpoolReplayError<E> {
    /// The private record could not be read or decoded.
    Storage(RuntimeSpoolError),
    /// The consumer rejected a decoded item.
    Consumer(E),
}

enum Storage {
    Memory(Vec<u8>),
    File {
        writer: BufWriter<std::fs::File>,
        temp: NamedTempFile,
    },
}

impl std::fmt::Debug for Storage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Memory(bytes) => formatter
                .debug_struct("Memory")
                .field("bytes", &bytes.len())
                .finish(),
            Self::File { temp, .. } => formatter
                .debug_struct("File")
                .field("path", &temp.path())
                .finish(),
        }
    }
}

impl Storage {
    fn append(&mut self, bytes: &[u8], memory_threshold: usize) -> io::Result<()> {
        match self {
            Self::Memory(storage) => {
                let needed = storage
                    .len()
                    .checked_add(bytes.len())
                    .ok_or_else(|| io::Error::other("runtime memory length overflow"))?;
                if needed > storage.capacity() {
                    let doubled = storage.capacity().saturating_mul(2).max(1);
                    let target = doubled.max(needed).min(memory_threshold.max(needed));
                    let additional = target.saturating_sub(storage.len());
                    storage
                        .try_reserve_exact(additional)
                        .map_err(|error| io::Error::other(error.to_string()))?;
                    if storage.capacity() < needed {
                        return Err(io::Error::other(
                            "runtime memory reserve did not reach requested length",
                        ));
                    }
                }
                storage.extend_from_slice(bytes);
            }
            Self::File { writer, .. } => writer.write_all(bytes)?,
        }
        Ok(())
    }

    fn patch(&mut self, offset: u64, bytes: &[u8]) -> io::Result<()> {
        match self {
            Self::Memory(storage) => {
                let offset = usize::try_from(offset)
                    .map_err(|_| io::Error::other("runtime record offset exceeds memory"))?;
                let end = offset
                    .checked_add(bytes.len())
                    .ok_or_else(|| io::Error::other("runtime record offset overflow"))?;
                let target = storage
                    .get_mut(offset..end)
                    .ok_or_else(|| io::Error::other("runtime record patch is out of bounds"))?;
                target.copy_from_slice(bytes);
            }
            Self::File { writer, .. } => {
                writer.seek(SeekFrom::Start(offset))?;
                writer.write_all(bytes)?;
                writer.seek(SeekFrom::End(0))?;
            }
        }
        Ok(())
    }

    fn truncate(&mut self, length: u64) -> io::Result<()> {
        match self {
            Self::Memory(storage) => {
                let length = usize::try_from(length)
                    .map_err(|_| io::Error::other("runtime storage length exceeds memory"))?;
                storage.truncate(length);
            }
            Self::File { writer, .. } => {
                writer.flush()?;
                writer.get_mut().set_len(length)?;
                writer.seek(SeekFrom::End(0))?;
            }
        }
        Ok(())
    }

    fn transition_to_file(
        &mut self,
        config: &RuntimeSpoolConfig,
        cumulative_spool_bytes: &mut u64,
    ) -> Result<(), RuntimeSpoolError> {
        let Self::Memory(memory) = self else {
            return Ok(());
        };
        if !config.allow_spool {
            return Err(RuntimeSpoolError::SpoolDisabled);
        }
        let memory_bytes = u64::try_from(memory.len()).unwrap_or(u64::MAX);
        let next_spool = cumulative_spool_bytes.saturating_add(memory_bytes);
        if next_spool > config.maximum_spool_bytes {
            return Err(RuntimeSpoolError::SpoolLimit);
        }
        let temp = NamedTempFile::new_in(&config.spool_directory)?;
        let mut writer = BufWriter::with_capacity(REPLAY_BUFFER_BYTES, temp.reopen()?);
        writer.write_all(memory)?;
        writer.flush()?;
        writer.seek(SeekFrom::End(0))?;
        *cumulative_spool_bytes = next_spool;
        let old = std::mem::take(memory);
        drop(old);
        *self = Self::File { temp, writer };
        Ok(())
    }

    fn replay_source(&mut self) -> Result<ReplaySource<'_>, RuntimeSpoolError> {
        match self {
            Self::Memory(bytes) => Ok(ReplaySource::Memory { bytes, offset: 0 }),
            Self::File { temp, writer } => {
                writer.flush()?;
                let mut file = temp.reopen()?;
                file.seek(SeekFrom::Start(0))?;
                Ok(ReplaySource::File {
                    reader: BufReader::with_capacity(REPLAY_BUFFER_BYTES, file),
                })
            }
        }
    }

    #[cfg(test)]
    fn spool_path(&self) -> Option<&Path> {
        match self {
            Self::Memory(_) => None,
            Self::File { temp, .. } => Some(temp.path()),
        }
    }

    fn memory_capacity(&self) -> usize {
        match self {
            Self::Memory(bytes) => bytes.capacity(),
            Self::File { .. } => 0,
        }
    }
}

/// Bounded private runtime-value staging.
#[derive(Debug)]
pub(crate) struct RuntimeSpool {
    config: RuntimeSpoolConfig,
    storage: Storage,
    retained_bytes: u64,
    item_count: u64,
    total_bytes_written: u64,
    spool_bytes_written: u64,
    root_active: bool,
    root_sealed: bool,
    generation: u64,
    replay_decoded_bytes: u64,
    maximum_decoded_requirement: u64,
    cancellation: Option<Arc<AtomicBool>>,
}

impl RuntimeSpool {
    /// Creates an empty staging store.
    #[must_use]
    pub(crate) fn new(config: RuntimeSpoolConfig) -> Self {
        let replay_decoded_bytes = config.maximum_decoded_bytes;
        Self {
            storage: Storage::Memory(Vec::new()),
            retained_bytes: 0,
            config,
            item_count: 0,
            total_bytes_written: 0,
            spool_bytes_written: 0,
            root_active: false,
            root_sealed: false,
            generation: 0,
            replay_decoded_bytes,
            maximum_decoded_requirement: 0,
            cancellation: None,
        }
    }

    /// Attaches a cooperative cancellation flag.
    #[must_use]
    pub(crate) fn with_cancellation(mut self, cancellation: Arc<AtomicBool>) -> Self {
        self.cancellation = Some(cancellation);
        self
    }

    /// Starts a fresh root generation, retaining cumulative byte accounting.
    pub(crate) fn begin_root(&mut self) -> Result<(), RuntimeSpoolError> {
        if self.root_active {
            return Err(RuntimeSpoolError::State("root is already active"));
        }
        self.check_cancelled()?;
        let result = self.reset_contents();
        if result.is_err() {
            self.root_active = false;
            self.root_sealed = false;
            return result;
        }
        self.root_active = true;
        self.root_sealed = false;
        Ok(())
    }

    /// Discards the active generation when a replacement root is announced.
    pub(crate) fn replace_root(&mut self) -> Result<(), RuntimeSpoolError> {
        if !self.root_active {
            return Err(RuntimeSpoolError::State("root is not active"));
        }
        let result = self.reset_contents();
        if result.is_err() {
            self.root_active = false;
            self.root_sealed = false;
        }
        result
    }

    /// Seals the active generation for replay.
    pub(crate) fn finish_root(&mut self) -> Result<(), RuntimeSpoolError> {
        if !self.root_active {
            return Err(RuntimeSpoolError::State("root is not active"));
        }
        self.root_active = false;
        self.root_sealed = true;
        Ok(())
    }

    /// Aborts and removes the active generation.
    pub(crate) fn abort_root(&mut self) -> Result<(), RuntimeSpoolError> {
        if !self.root_active {
            return Err(RuntimeSpoolError::State("root is not active"));
        }
        let result = self.reset_contents();
        self.root_active = false;
        self.root_sealed = false;
        result
    }

    /// Removes all retained records and the private file.
    ///
    /// Cumulative byte counters intentionally remain.  A caller cannot reset
    /// the store to bypass the invocation-wide total or spool budget.
    #[cfg(test)]
    pub(crate) fn clear(&mut self) -> Result<(), RuntimeSpoolError> {
        let result = self.reset_contents();
        self.root_active = false;
        self.root_sealed = false;
        result
    }

    /// Stages a typed item and returns its opaque location in this root.
    pub(crate) fn push_item(
        &mut self,
        item: RuntimeSpoolItem,
    ) -> Result<RuntimeRecordId, RuntimeSpoolError> {
        if !self.root_active {
            return Err(RuntimeSpoolError::State("root is not active"));
        }
        self.check_cancelled()?;
        let record_start = self.retained_bytes;
        let payload_start = record_start.saturating_add(RECORD_HEADER_BYTES as u64);
        match self.push_item_inner(item, record_start, payload_start) {
            Ok(decoded_requirement) => {
                self.maximum_decoded_requirement =
                    self.maximum_decoded_requirement.max(decoded_requirement);
                Ok(RuntimeRecordId {
                    generation: self.generation,
                    offset: record_start,
                })
            }
            Err(error) => {
                // A failed item must not become visible to replay.  The byte
                // counters remain cumulative, so a caller cannot retry forever
                // after a failed write and bypass the invocation-wide limits.
                if let Err(rollback) = self.storage.truncate(record_start) {
                    return Err(RuntimeSpoolError::Io(rollback));
                }
                self.retained_bytes = record_start;
                Err(error)
            }
        }
    }

    fn push_item_inner(
        &mut self,
        item: RuntimeSpoolItem,
        record_start: u64,
        payload_start: u64,
    ) -> Result<u64, RuntimeSpoolError> {
        self.append_piece(&[0; RECORD_HEADER_BYTES], record_start, record_start)?;
        let (tag, value) = item.into_parts();
        self.append_piece(&[tag], record_start, payload_start)?;
        let mut decoded_budget = DecodeBudget::new(u64::MAX);
        encode_value(
            self,
            &value,
            0,
            record_start,
            payload_start,
            &mut decoded_budget,
        )?;
        let payload_bytes = self.retained_bytes.saturating_sub(payload_start);
        self.patch_record_length(record_start, &payload_bytes.to_le_bytes())?;
        self.item_count = self.item_count.saturating_add(1);
        Ok(decoded_budget.used)
    }

    /// Replays typed values in source order without retaining an ID list.
    pub(crate) fn replay_items<F, E>(
        &mut self,
        mut emit: F,
    ) -> Result<(), RuntimeSpoolReplayError<E>>
    where
        F: FnMut(RuntimeSpoolItem) -> Result<(), E>,
    {
        if self.root_active || !self.root_sealed {
            return Err(RuntimeSpoolReplayError::Storage(RuntimeSpoolError::State(
                "root is not sealed",
            )));
        }
        // Replaying is a commit-side effect: callers must not retry the
        // callback after it has emitted a prefix and then failed.
        self.root_sealed = false;
        let cancellation = self.cancellation.clone();
        check_cancellation(cancellation.as_ref()).map_err(RuntimeSpoolReplayError::Storage)?;
        let total_bytes = self.retained_bytes;
        let item_count = self.item_count;
        let mut config = self.config.clone();
        config.maximum_decoded_bytes = self.replay_decoded_bytes;
        let mut source = self
            .storage
            .replay_source()
            .map_err(RuntimeSpoolReplayError::Storage)?;
        let mut consumed = 0_u64;
        let mut count = 0_u64;
        while consumed < total_bytes {
            let (item, record_bytes) = decode_record_at_current_position(
                &mut source,
                consumed,
                total_bytes,
                &config,
                cancellation.as_ref(),
            )
            .map_err(RuntimeSpoolReplayError::Storage)?;
            consumed =
                consumed
                    .checked_add(record_bytes)
                    .ok_or(RuntimeSpoolReplayError::Storage(
                        RuntimeSpoolError::RecordBounds,
                    ))?;
            count = count.saturating_add(1);
            emit(item).map_err(RuntimeSpoolReplayError::Consumer)?;
        }
        if count != item_count {
            return Err(RuntimeSpoolReplayError::Storage(RuntimeSpoolError::Decode(
                "item count does not match records",
            )));
        }
        Ok(())
    }

    /// Replays only the supplied records, in iterator order.
    ///
    /// IDs are generation-scoped and are validated before any record bytes are
    /// decoded.  The iterator is consumed directly, so sequential callers do
    /// not need to allocate an index-sized ID vector.
    pub(crate) fn replay_records<I, F, E>(
        &mut self,
        ids: I,
        mut emit: F,
    ) -> Result<(), RuntimeSpoolReplayError<E>>
    where
        I: IntoIterator<Item = RuntimeRecordId>,
        F: FnMut(RuntimeSpoolItem) -> Result<(), E>,
    {
        if self.root_active || !self.root_sealed {
            return Err(RuntimeSpoolReplayError::Storage(RuntimeSpoolError::State(
                "root is not sealed",
            )));
        }
        // Consume the sealed generation before invoking user code.  This
        // makes a callback failure a one-shot operation with no replay retry.
        self.root_sealed = false;
        let cancellation = self.cancellation.clone();
        check_cancellation(cancellation.as_ref()).map_err(RuntimeSpoolReplayError::Storage)?;
        let total_bytes = self.retained_bytes;
        let generation = self.generation;
        let mut config = self.config.clone();
        config.maximum_decoded_bytes = self.replay_decoded_bytes;
        let mut source = self
            .storage
            .replay_source()
            .map_err(RuntimeSpoolReplayError::Storage)?;
        for id in ids {
            if id.generation != generation {
                return Err(RuntimeSpoolReplayError::Storage(
                    RuntimeSpoolError::StaleRecord,
                ));
            }
            let (item, _) = decode_record_at(
                &mut source,
                id.offset,
                total_bytes,
                &config,
                cancellation.as_ref(),
            )
            .map_err(RuntimeSpoolReplayError::Storage)?;
            emit(item).map_err(RuntimeSpoolReplayError::Consumer)?;
        }
        Ok(())
    }

    /// Whether no values are currently retained.
    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.item_count == 0
    }

    /// Encoded bytes retained by the current generation, including headers.
    #[must_use]
    pub(crate) const fn retained_bytes(&self) -> u64 {
        self.retained_bytes
    }

    /// Number of currently retained records.
    #[must_use]
    #[cfg(test)]
    pub(crate) const fn item_count(&self) -> u64 {
        self.item_count
    }

    /// Cumulative encoded bytes written, including rolled-back partial items.
    #[must_use]
    #[cfg(test)]
    pub(crate) const fn total_bytes_written(&self) -> u64 {
        self.total_bytes_written
    }

    /// Cumulative bytes written to temporary storage.
    #[must_use]
    pub(crate) const fn spool_bytes_written(&self) -> u64 {
        self.spool_bytes_written
    }

    /// Returns the largest decoded replay budget required by one retained
    /// item in the current root.  This mirrors the authoritative decoder's
    /// accounting and is reset when a new root replaces the current one.
    #[must_use]
    pub(crate) const fn maximum_decoded_requirement(&self) -> u64 {
        self.maximum_decoded_requirement
    }

    /// Whether the current generation uses a private temporary file.
    #[must_use]
    pub(crate) fn spooled(&self) -> bool {
        matches!(self.storage, Storage::File { .. })
    }

    /// Current private temporary-file path, if the store spilled.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn spool_path(&self) -> Option<&Path> {
        self.storage.spool_path()
    }

    /// Returns the actual `Vec` capacity retained by the active memory store.
    ///
    /// This is intentionally capacity, rather than logical bytes: callers
    /// sharing a preparation budget need to account for allocator headroom.
    /// A spilled store reports zero; this excludes the fixed `BufWriter` and
    /// replay `BufReader` buffers, which callers account for separately.
    #[must_use]
    pub(crate) fn retained_memory_capacity_bytes(&self) -> usize {
        self.storage.memory_capacity()
    }

    /// Moves the current generation to private temporary storage now.
    pub(crate) fn spill_to_disk(&mut self) -> Result<(), RuntimeSpoolError> {
        self.check_cancelled()?;
        self.storage
            .transition_to_file(&self.config, &mut self.spool_bytes_written)
    }

    /// Lowers or raises the memory threshold and spills when the live store is
    /// already larger than the new threshold.
    pub(crate) fn set_memory_threshold(
        &mut self,
        threshold: usize,
    ) -> Result<(), RuntimeSpoolError> {
        self.check_cancelled()?;
        self.config.memory_threshold_bytes = threshold;
        let should_spill =
            matches!(&self.storage, Storage::Memory(bytes) if bytes.capacity() > threshold);
        if should_spill {
            return self.spill_to_disk();
        }
        Ok(())
    }

    /// Limits one decoded replay value to the caller's remaining shared
    /// preparation budget without changing the configured ceiling. Encoded
    /// records remain charged separately by the spool, so moving the store
    /// into a local replay variable cannot conceal its resident memory from
    /// the value decoder.
    pub(crate) fn set_replay_decoded_budget(
        &mut self,
        limit: usize,
    ) -> Result<(), RuntimeSpoolError> {
        let limit = u64::try_from(limit).unwrap_or(u64::MAX);
        if limit > self.config.maximum_decoded_bytes {
            return Err(RuntimeSpoolError::DecodedLimit);
        }
        self.replay_decoded_bytes = limit;
        Ok(())
    }

    fn reset_contents(&mut self) -> Result<(), RuntimeSpoolError> {
        self.storage = Storage::Memory(Vec::new());
        self.retained_bytes = 0;
        self.item_count = 0;
        self.maximum_decoded_requirement = 0;
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(RuntimeSpoolError::GenerationLimit)?;
        self.replay_decoded_bytes = self.config.maximum_decoded_bytes;
        Ok(())
    }

    fn check_cancelled(&self) -> Result<(), RuntimeSpoolError> {
        check_cancellation(self.cancellation.as_ref())
    }

    fn append_piece(
        &mut self,
        bytes: &[u8],
        record_start: u64,
        payload_start: u64,
    ) -> Result<(), RuntimeSpoolError> {
        if bytes.is_empty() {
            return Ok(());
        }
        self.check_cancelled()?;
        let total = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        if self.total_bytes_written.saturating_add(total) > self.config.maximum_total_bytes {
            return Err(RuntimeSpoolError::TotalLimit);
        }
        if record_start != payload_start {
            let current_length = self.retained_bytes;
            let current_payload = if current_length < payload_start {
                0
            } else {
                current_length.saturating_sub(payload_start)
            };
            let next_payload = current_payload.saturating_add(total);
            if next_payload > self.config.maximum_item_bytes {
                return Err(RuntimeSpoolError::ItemLimit);
            }
        }
        if matches!(&self.storage, Storage::Memory(memory) if memory.len().saturating_add(bytes.len()) > self.config.memory_threshold_bytes)
        {
            self.storage
                .transition_to_file(&self.config, &mut self.spool_bytes_written)?;
        }
        self.account_spool_write(bytes.len())?;
        self.storage
            .append(bytes, self.config.memory_threshold_bytes)?;
        self.retained_bytes = self.retained_bytes.saturating_add(total);
        self.total_bytes_written = self.total_bytes_written.saturating_add(total);
        Ok(())
    }

    fn patch_record_length(&mut self, offset: u64, bytes: &[u8]) -> Result<(), RuntimeSpoolError> {
        self.account_spool_write(bytes.len())?;
        self.storage.patch(offset, bytes)?;
        Ok(())
    }

    fn account_spool_write(&mut self, bytes: usize) -> Result<(), RuntimeSpoolError> {
        if !matches!(&self.storage, Storage::File { .. }) {
            return Ok(());
        }
        let bytes = u64::try_from(bytes).unwrap_or(u64::MAX);
        let next_spool = self.spool_bytes_written.saturating_add(bytes);
        if next_spool > self.config.maximum_spool_bytes {
            return Err(RuntimeSpoolError::SpoolLimit);
        }
        self.spool_bytes_written = next_spool;
        Ok(())
    }
}

fn check_cancellation(cancellation: Option<&Arc<AtomicBool>>) -> Result<(), RuntimeSpoolError> {
    if cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
        Err(RuntimeSpoolError::Cancelled)
    } else {
        Ok(())
    }
}

fn encode_value(
    spool: &mut RuntimeSpool,
    value: &Value,
    depth: usize,
    record_start: u64,
    payload_start: u64,
    budget: &mut DecodeBudget,
) -> Result<(), RuntimeSpoolError> {
    spool.check_cancelled()?;
    match value {
        Value::Null => {
            budget.charge(std::mem::size_of::<Value>() as u64)?;
            spool.append_piece(&[NULL_TAG], record_start, payload_start)
        }
        Value::Bool(value) => {
            budget.charge(std::mem::size_of::<Value>() as u64)?;
            spool.append_piece(&[BOOL_TAG, u8::from(*value)], record_start, payload_start)
        }
        Value::Number(number) => {
            if let Some(literal) = number.exact_literal() {
                if literal.len() > spool.config.maximum_token_bytes {
                    return Err(RuntimeSpoolError::TokenLimit);
                }
                budget.charge(u64::try_from(literal.len()).unwrap_or(u64::MAX))?;
                budget.charge(std::mem::size_of::<Value>() as u64)?;
                spool.append_piece(&[FINITE_NUMBER_TAG], record_start, payload_start)?;
                write_len_prefixed(spool, literal.as_bytes(), record_start, payload_start)
            } else {
                budget.charge(std::mem::size_of::<Value>() as u64)?;
                spool.append_piece(&[RUNTIME_NUMBER_TAG], record_start, payload_start)?;
                spool.append_piece(
                    &number.as_f64().to_bits().to_le_bytes(),
                    record_start,
                    payload_start,
                )
            }
        }
        Value::String(value) => {
            if value.len() > spool.config.maximum_token_bytes {
                return Err(RuntimeSpoolError::TokenLimit);
            }
            budget.charge(u64::try_from(value.len()).unwrap_or(u64::MAX))?;
            budget.charge(std::mem::size_of::<Value>() as u64)?;
            spool.append_piece(&[STRING_TAG], record_start, payload_start)?;
            write_len_prefixed(spool, value.as_bytes(), record_start, payload_start)
        }
        Value::Array(values) => {
            if depth >= spool.config.maximum_depth {
                return Err(RuntimeSpoolError::DepthLimit);
            }
            budget.charge_count(
                u64::try_from(values.len()).unwrap_or(u64::MAX),
                std::mem::size_of::<Value>(),
            )?;
            spool.append_piece(&[ARRAY_TAG], record_start, payload_start)?;
            write_count(spool, values.len(), record_start, payload_start)?;
            for value in values.iter() {
                encode_value(
                    spool,
                    value,
                    depth.saturating_add(1),
                    record_start,
                    payload_start,
                    budget,
                )?;
            }
            Ok(())
        }
        Value::Object(values) => {
            if depth >= spool.config.maximum_depth {
                return Err(RuntimeSpoolError::DepthLimit);
            }
            budget.charge_count(
                u64::try_from(values.len()).unwrap_or(u64::MAX),
                std::mem::size_of::<Value>() * 2,
            )?;
            spool.append_piece(&[OBJECT_TAG], record_start, payload_start)?;
            write_count(spool, values.len(), record_start, payload_start)?;
            for (key, value) in values.iter() {
                if key.len() > spool.config.maximum_token_bytes {
                    return Err(RuntimeSpoolError::TokenLimit);
                }
                budget.charge(u64::try_from(key.len()).unwrap_or(u64::MAX))?;
                budget.charge(std::mem::size_of::<Value>() as u64)?;
                write_len_prefixed(spool, key.as_bytes(), record_start, payload_start)?;
                encode_value(
                    spool,
                    value,
                    depth.saturating_add(1),
                    record_start,
                    payload_start,
                    budget,
                )?;
            }
            Ok(())
        }
    }
}

fn write_len_prefixed(
    spool: &mut RuntimeSpool,
    bytes: &[u8],
    record_start: u64,
    payload_start: u64,
) -> Result<(), RuntimeSpoolError> {
    let length = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    write_varint(spool, length, record_start, payload_start)?;
    spool.append_piece(bytes, record_start, payload_start)
}

fn write_count(
    spool: &mut RuntimeSpool,
    count: usize,
    record_start: u64,
    payload_start: u64,
) -> Result<(), RuntimeSpoolError> {
    let count = u64::try_from(count).unwrap_or(u64::MAX);
    write_varint(spool, count, record_start, payload_start)
}

fn write_varint(
    spool: &mut RuntimeSpool,
    value: u64,
    record_start: u64,
    payload_start: u64,
) -> Result<(), RuntimeSpoolError> {
    let (bytes, length) = encode_varint(value);
    spool.append_piece(&bytes[..length], record_start, payload_start)
}

fn encode_varint(mut value: u64) -> ([u8; MAX_VARINT_BYTES], usize) {
    let mut bytes = [0_u8; MAX_VARINT_BYTES];
    let mut length = 0_usize;
    loop {
        debug_assert!(length < MAX_VARINT_BYTES);
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        bytes[length] = byte;
        length += 1;
        if value == 0 {
            return (bytes, length);
        }
    }
}

fn decode_record_at(
    source: &mut ReplaySource<'_>,
    offset: u64,
    total_bytes: u64,
    config: &RuntimeSpoolConfig,
    cancellation: Option<&Arc<AtomicBool>>,
) -> Result<(RuntimeSpoolItem, u64), RuntimeSpoolError> {
    let header_bytes = u64::try_from(RECORD_HEADER_BYTES).unwrap_or(u64::MAX);
    if offset > total_bytes || total_bytes.saturating_sub(offset) < header_bytes {
        return Err(RuntimeSpoolError::RecordBounds);
    }
    source.seek_abs(offset)?;
    decode_record_at_current_position(source, offset, total_bytes, config, cancellation)
}

fn decode_record_at_current_position(
    source: &mut ReplaySource<'_>,
    offset: u64,
    total_bytes: u64,
    config: &RuntimeSpoolConfig,
    cancellation: Option<&Arc<AtomicBool>>,
) -> Result<(RuntimeSpoolItem, u64), RuntimeSpoolError> {
    let header_bytes = u64::try_from(RECORD_HEADER_BYTES).unwrap_or(u64::MAX);
    if offset > total_bytes || total_bytes.saturating_sub(offset) < header_bytes {
        return Err(RuntimeSpoolError::RecordBounds);
    }
    let mut header = [0_u8; RECORD_HEADER_BYTES];
    source.read_exact_bounded(&mut header, cancellation)?;
    let payload_bytes = u64::from_le_bytes(header);
    if payload_bytes < TAG_BYTES as u64 {
        return Err(RuntimeSpoolError::Decode("record has no item tag"));
    }
    if payload_bytes > config.maximum_item_bytes {
        return Err(RuntimeSpoolError::ItemLimit);
    }
    if payload_bytes
        > total_bytes
            .saturating_sub(offset)
            .saturating_sub(header_bytes)
    {
        return Err(RuntimeSpoolError::Decode("record exceeds remaining bytes"));
    }
    let mut payload = LimitedReader {
        source,
        remaining: payload_bytes,
        consumed: 0,
        cancellation,
    };
    let tag = payload.read_u8()?;
    let mut budget = DecodeBudget::new(config.maximum_decoded_bytes);
    let value = decode_value(&mut payload, config, 0, &mut budget)?;
    if payload.remaining != 0 {
        return Err(RuntimeSpoolError::Decode("trailing bytes in runtime item"));
    }
    let item = RuntimeSpoolItem::from_parts(tag, value)
        .ok_or(RuntimeSpoolError::Decode("unknown runtime item tag"))?;
    Ok((
        item,
        header_bytes
            .checked_add(payload_bytes)
            .ok_or(RuntimeSpoolError::RecordBounds)?,
    ))
}

enum ReplaySource<'a> {
    Memory { bytes: &'a [u8], offset: usize },
    File { reader: BufReader<std::fs::File> },
}

impl Read for ReplaySource<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Memory { bytes, offset } => {
                let remaining = bytes.len().saturating_sub(*offset);
                let length = remaining.min(buffer.len());
                if length == 0 {
                    return Ok(0);
                }
                buffer[..length].copy_from_slice(&bytes[*offset..*offset + length]);
                *offset = offset.saturating_add(length);
                Ok(length)
            }
            Self::File { reader } => reader.read(buffer),
        }
    }
}

impl ReplaySource<'_> {
    fn seek_abs(&mut self, offset: u64) -> io::Result<()> {
        match self {
            Self::Memory {
                bytes,
                offset: current,
            } => {
                let offset = usize::try_from(offset)
                    .map_err(|_| io::Error::other("runtime record offset exceeds memory"))?;
                if offset > bytes.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "runtime record offset exceeds storage",
                    ));
                }
                *current = offset;
                Ok(())
            }
            Self::File { reader } => {
                reader.seek(SeekFrom::Start(offset))?;
                Ok(())
            }
        }
    }

    fn read_exact_bounded(
        &mut self,
        buffer: &mut [u8],
        cancellation: Option<&Arc<AtomicBool>>,
    ) -> Result<(), RuntimeSpoolError> {
        let mut offset = 0usize;
        while offset < buffer.len() {
            if cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
                return Err(RuntimeSpoolError::Cancelled);
            }
            let end = offset.saturating_add(REPLAY_BUFFER_BYTES).min(buffer.len());
            let read = self.read(&mut buffer[offset..end])?;
            if read == 0 {
                return Err(RuntimeSpoolError::Decode("truncated runtime record"));
            }
            offset = offset.saturating_add(read);
        }
        Ok(())
    }
}

struct LimitedReader<'source, 'storage> {
    source: &'source mut ReplaySource<'storage>,
    remaining: u64,
    consumed: u64,
    cancellation: Option<&'source Arc<AtomicBool>>,
}

impl LimitedReader<'_, '_> {
    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), RuntimeSpoolError> {
        let requested = u64::try_from(buffer.len()).unwrap_or(u64::MAX);
        if requested > self.remaining {
            return Err(RuntimeSpoolError::Decode(
                "field exceeds the record boundary",
            ));
        }
        let mut offset = 0usize;
        while offset < buffer.len() {
            if self
                .cancellation
                .is_some_and(|flag| flag.load(Ordering::Relaxed))
            {
                return Err(RuntimeSpoolError::Cancelled);
            }
            let end = offset.saturating_add(REPLAY_BUFFER_BYTES).min(buffer.len());
            let read = self.source.read(&mut buffer[offset..end])?;
            if read == 0 {
                return Err(RuntimeSpoolError::Decode("truncated runtime field"));
            }
            offset = offset.saturating_add(read);
            let read = u64::try_from(read).unwrap_or(u64::MAX);
            self.remaining = self.remaining.saturating_sub(read);
            self.consumed = self.consumed.saturating_add(read);
        }
        Ok(())
    }

    fn read_u8(&mut self) -> Result<u8, RuntimeSpoolError> {
        let mut byte = [0_u8; 1];
        self.read_exact(&mut byte)?;
        Ok(byte[0])
    }
}

struct DecodeBudget {
    used: u64,
    maximum: u64,
}

impl DecodeBudget {
    fn new(maximum: u64) -> Self {
        Self { used: 0, maximum }
    }

    fn charge(&mut self, bytes: u64) -> Result<(), RuntimeSpoolError> {
        let next = self
            .used
            .checked_add(bytes)
            .ok_or(RuntimeSpoolError::DecodedLimit)?;
        if next > self.maximum {
            return Err(RuntimeSpoolError::DecodedLimit);
        }
        self.used = next;
        Ok(())
    }

    fn charge_count(&mut self, count: u64, bytes_per_item: usize) -> Result<(), RuntimeSpoolError> {
        let bytes_per_item = u64::try_from(bytes_per_item).unwrap_or(u64::MAX);
        self.charge(
            count
                .checked_mul(bytes_per_item)
                .ok_or(RuntimeSpoolError::DecodedLimit)?,
        )
    }
}

fn decode_value(
    reader: &mut LimitedReader<'_, '_>,
    config: &RuntimeSpoolConfig,
    depth: usize,
    budget: &mut DecodeBudget,
) -> Result<Value, RuntimeSpoolError> {
    let tag = reader.read_u8()?;
    match tag {
        NULL_TAG => {
            budget.charge(std::mem::size_of::<Value>() as u64)?;
            Ok(Value::Null)
        }
        BOOL_TAG => match reader.read_u8()? {
            0 => {
                budget.charge(std::mem::size_of::<Value>() as u64)?;
                Ok(Value::Bool(false))
            }
            1 => {
                budget.charge(std::mem::size_of::<Value>() as u64)?;
                Ok(Value::Bool(true))
            }
            _ => Err(RuntimeSpoolError::Decode("invalid boolean payload")),
        },
        FINITE_NUMBER_TAG => {
            let bytes = read_token(reader, config.maximum_token_bytes, budget)?;
            let literal = std::str::from_utf8(&bytes).map_err(|_| RuntimeSpoolError::Utf8)?;
            budget.charge(std::mem::size_of::<Value>() as u64)?;
            Ok(Value::Number(Number::parse(literal)?))
        }
        RUNTIME_NUMBER_TAG => {
            let mut bytes = [0_u8; 8];
            reader.read_exact(&mut bytes)?;
            budget.charge(std::mem::size_of::<Value>() as u64)?;
            Ok(Value::Number(Number::from_runtime_f64(f64::from_bits(
                u64::from_le_bytes(bytes),
            ))))
        }
        STRING_TAG => {
            let bytes = read_token(reader, config.maximum_token_bytes, budget)?;
            let value = String::from_utf8(bytes).map_err(|_| RuntimeSpoolError::Utf8)?;
            budget.charge(std::mem::size_of::<Value>() as u64)?;
            Ok(Value::string(value))
        }
        ARRAY_TAG => {
            ensure_container_depth(depth, config.maximum_depth)?;
            let count = read_count(reader)?;
            let count = usize::try_from(count)
                .map_err(|_| RuntimeSpoolError::Decode("array count exceeds memory"))?;
            let count_u64 = u64::try_from(count).unwrap_or(u64::MAX);
            budget.charge_count(count_u64, std::mem::size_of::<Value>())?;
            let mut values = Vec::new();
            values
                .try_reserve_exact(count)
                .map_err(|_| RuntimeSpoolError::DecodedLimit)?;
            for _ in 0..count {
                values.push(decode_value(
                    reader,
                    config,
                    depth.saturating_add(1),
                    budget,
                )?);
            }
            Ok(Value::array(values))
        }
        OBJECT_TAG => {
            ensure_container_depth(depth, config.maximum_depth)?;
            let count = read_count(reader)?;
            let count = usize::try_from(count)
                .map_err(|_| RuntimeSpoolError::Decode("object count exceeds memory"))?;
            let count_u64 = u64::try_from(count).unwrap_or(u64::MAX);
            budget.charge_count(count_u64, std::mem::size_of::<Value>() * 2)?;
            let mut values = Object::new();
            values
                .try_reserve(count)
                .map_err(|_| RuntimeSpoolError::DecodedLimit)?;
            for _ in 0..count {
                let key = read_token(reader, config.maximum_token_bytes, budget)?;
                let key = String::from_utf8(key).map_err(|_| RuntimeSpoolError::Utf8)?;
                budget.charge(std::mem::size_of::<Value>() as u64)?;
                values.insert(
                    key.into(),
                    decode_value(reader, config, depth.saturating_add(1), budget)?,
                );
            }
            Ok(Value::object(values))
        }
        _ => Err(RuntimeSpoolError::Decode("unknown runtime value tag")),
    }
}

fn read_token(
    reader: &mut LimitedReader<'_, '_>,
    maximum_token_bytes: usize,
    budget: &mut DecodeBudget,
) -> Result<Vec<u8>, RuntimeSpoolError> {
    let length = read_varint(reader)?;
    if length > reader.remaining {
        return Err(RuntimeSpoolError::Decode("token exceeds record boundary"));
    }
    let length = usize::try_from(length)
        .map_err(|_| RuntimeSpoolError::Decode("token length exceeds memory"))?;
    if length > maximum_token_bytes {
        return Err(RuntimeSpoolError::TokenLimit);
    }
    budget.charge(u64::try_from(length).unwrap_or(u64::MAX))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|_| RuntimeSpoolError::DecodedLimit)?;
    bytes.resize(length, 0);
    reader.read_exact(&mut bytes)?;
    Ok(bytes)
}

fn read_count(reader: &mut LimitedReader<'_, '_>) -> Result<u64, RuntimeSpoolError> {
    let count = read_varint(reader)?;
    if count > reader.remaining {
        return Err(RuntimeSpoolError::Decode("container count exceeds record"));
    }
    Ok(count)
}

fn read_varint(reader: &mut LimitedReader<'_, '_>) -> Result<u64, RuntimeSpoolError> {
    let mut value = 0_u64;
    for index in 0..MAX_VARINT_BYTES {
        let byte = reader.read_u8()?;
        let payload = u64::from(byte & 0x7f);
        if index == MAX_VARINT_BYTES - 1 && (byte & 0x80 != 0 || payload > 1) {
            return Err(RuntimeSpoolError::Decode("runtime varint overflows u64"));
        }
        value |= payload << (index * 7);
        if byte & 0x80 == 0 {
            if index != 0 && payload == 0 {
                return Err(RuntimeSpoolError::Decode("runtime varint is not canonical"));
            }
            return Ok(value);
        }
    }
    Err(RuntimeSpoolError::Decode("runtime varint overflows u64"))
}

fn ensure_container_depth(depth: usize, maximum_depth: usize) -> Result<(), RuntimeSpoolError> {
    if depth >= maximum_depth {
        Err(RuntimeSpoolError::DepthLimit)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    use tq_core::{Number, Object, Value};

    use super::{
        DecodeBudget, LimitedReader, RECORD_HEADER_BYTES, REPLAY_BUFFER_BYTES, ReplaySource,
        RuntimeRecordId, RuntimeSpool, RuntimeSpoolConfig, RuntimeSpoolError, RuntimeSpoolItem,
        RuntimeSpoolReplayError, Storage, encode_varint, read_count, read_token, read_varint,
    };

    fn config(directory: &std::path::Path) -> RuntimeSpoolConfig {
        RuntimeSpoolConfig {
            spool_directory: directory.to_owned(),
            maximum_total_bytes: 1024 * 1024,
            maximum_spool_bytes: 1024 * 1024,
            maximum_item_bytes: 1024 * 1024,
            ..RuntimeSpoolConfig::default()
        }
    }

    fn decode_varint_bytes(
        bytes: &[u8],
        declared_remaining: u64,
    ) -> Result<u64, RuntimeSpoolError> {
        let mut source = ReplaySource::Memory { bytes, offset: 0 };
        let mut reader = LimitedReader {
            source: &mut source,
            remaining: declared_remaining,
            consumed: 0,
            cancellation: None,
        };
        read_varint(&mut reader)
    }

    #[test]
    fn varint_roundtrips_bounded_integer_boundaries() {
        for value in [
            0,
            1,
            0x7f,
            0x80,
            0xff,
            0x100,
            0x3fff,
            0x4000,
            u64::from(u32::MAX),
            u64::MAX,
        ] {
            let (bytes, length) = encode_varint(value);
            let decoded = decode_varint_bytes(&bytes[..length], length as u64).unwrap();
            assert_eq!(decoded, value);
            assert!(length <= super::MAX_VARINT_BYTES);
        }
    }

    #[test]
    fn varint_rejects_truncation_overflow_and_noncanonical_forms() {
        assert!(matches!(
            decode_varint_bytes(&[0x80], 1),
            Err(RuntimeSpoolError::Decode(
                "field exceeds the record boundary"
            ))
        ));
        assert!(matches!(
            decode_varint_bytes(&[0x80], 2),
            Err(RuntimeSpoolError::Decode("truncated runtime field"))
        ));
        assert!(matches!(
            decode_varint_bytes(&[0x80, 0], 2),
            Err(RuntimeSpoolError::Decode("runtime varint is not canonical"))
        ));
        assert!(matches!(
            decode_varint_bytes(&[0x80; 10], 10),
            Err(RuntimeSpoolError::Decode("runtime varint overflows u64"))
        ));
        let mut overflowing = [0x80_u8; 10];
        overflowing[9] = 2;
        assert!(matches!(
            decode_varint_bytes(&overflowing, 10),
            Err(RuntimeSpoolError::Decode("runtime varint overflows u64"))
        ));
    }

    #[test]
    fn varint_checks_cancellation_before_consuming_a_byte() {
        let cancellation = Arc::new(AtomicBool::new(true));
        let bytes = [0x80, 0x01];
        let mut source = ReplaySource::Memory {
            bytes: &bytes,
            offset: 0,
        };
        let mut reader = LimitedReader {
            source: &mut source,
            remaining: bytes.len() as u64,
            consumed: 0,
            cancellation: Some(&cancellation),
        };
        assert!(matches!(
            read_varint(&mut reader),
            Err(RuntimeSpoolError::Cancelled)
        ));
        assert_eq!(reader.consumed, 0);
    }

    #[test]
    fn compact_lengths_preserve_record_boundaries_and_token_limits() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut spool = RuntimeSpool::new(config(directory.path()));
        spool.begin_root().unwrap();
        spool
            .push_item(RuntimeSpoolItem::Projected(Value::string("abc")))
            .unwrap();
        assert_eq!(spool.retained_bytes(), 14);
        spool.finish_root().unwrap();
        let mut replayed = Vec::new();
        spool
            .replay_items(|item| {
                replayed.push(item);
                Ok::<(), ()>(())
            })
            .unwrap();
        assert_eq!(
            replayed,
            [RuntimeSpoolItem::Projected(Value::string("abc"))]
        );

        let mut bytes = vec![0_u8; 130];
        bytes[0] = 0x80;
        bytes[1] = 0x01;
        let mut source = ReplaySource::Memory {
            bytes: &bytes,
            offset: 0,
        };
        let mut reader = LimitedReader {
            source: &mut source,
            remaining: bytes.len() as u64,
            consumed: 0,
            cancellation: None,
        };
        let mut budget = DecodeBudget::new(u64::MAX);
        assert!(matches!(
            read_token(&mut reader, 127, &mut budget),
            Err(RuntimeSpoolError::TokenLimit)
        ));
        assert_eq!(reader.consumed, 2);
        assert_eq!(budget.used, 0);
    }

    #[test]
    fn compact_count_rejects_nested_truncation_before_reserve() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut spool = RuntimeSpool::new(config(directory.path()));
        spool.begin_root().unwrap();
        spool
            .push_item(RuntimeSpoolItem::Item(Value::array(Vec::new())))
            .unwrap();
        spool.finish_root().unwrap();
        let Storage::Memory(bytes) = &mut spool.storage else {
            panic!("small record should remain in memory");
        };
        // Header (8), item tag (1), array tag (1), then a continuation byte
        // with no room for the next count byte.
        bytes[10] = 0x80;
        assert!(matches!(
            spool.replay_items(|_| Ok::<(), ()>(())),
            Err(RuntimeSpoolReplayError::Storage(RuntimeSpoolError::Decode(
                "field exceeds the record boundary"
            )))
        ));
    }

    #[test]
    fn compact_count_checks_record_remaining_before_container_reserve() {
        let (bytes, length) = encode_varint(u64::MAX);
        let mut source = ReplaySource::Memory {
            bytes: &bytes[..length],
            offset: 0,
        };
        let mut reader = LimitedReader {
            source: &mut source,
            remaining: length as u64,
            consumed: 0,
            cancellation: None,
        };
        assert!(matches!(
            read_count(&mut reader),
            Err(RuntimeSpoolError::Decode("container count exceeds record"))
        ));
    }

    #[test]
    fn roundtrip_preserves_number_provenance_runtime_bits_and_object_order() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut object = Object::new();
        object.insert(
            "literal".into(),
            Value::Number(Number::parse("1.2300e+4").expect("finite literal")),
        );
        object.insert(
            "nan".into(),
            Value::Number(Number::from_runtime_f64(f64::from_bits(
                0x7ff8_0000_0000_0042,
            ))),
        );
        object.insert(
            "positive-infinity".into(),
            Value::Number(Number::from_runtime_f64(f64::INFINITY)),
        );
        object.insert(
            "negative-zero".into(),
            Value::Number(Number::from_runtime_f64(-0.0)),
        );
        let value = Value::object(object);

        let mut spool = RuntimeSpool::new(RuntimeSpoolConfig {
            memory_threshold_bytes: 1,
            ..config(directory.path())
        });
        spool.begin_root().unwrap();
        spool.push_item(RuntimeSpoolItem::Projected(value)).unwrap();
        spool.finish_root().unwrap();
        assert!(spool.spooled());
        let mut replayed = Vec::new();
        spool
            .replay_items(|item| {
                replayed.push(item);
                Ok::<(), ()>(())
            })
            .unwrap();
        let RuntimeSpoolItem::Projected(Value::Object(object)) = &replayed[0] else {
            panic!("expected projected object");
        };
        let keys = object.keys().map(AsRef::as_ref).collect::<Vec<_>>();
        assert_eq!(
            keys,
            ["literal", "nan", "positive-infinity", "negative-zero"]
        );
        let Value::Number(literal) = object.get("literal").expect("literal") else {
            panic!("expected literal number");
        };
        assert_eq!(literal.exact_literal(), Some("12300"));
        let Value::Number(nan) = object.get("nan").expect("nan") else {
            panic!("expected NaN");
        };
        assert_eq!(nan.as_f64().to_bits(), 0x7ff8_0000_0000_0042);
        let Value::Number(infinity) = object.get("positive-infinity").expect("infinity") else {
            panic!("expected infinity");
        };
        assert_eq!(infinity.as_f64().to_bits(), f64::INFINITY.to_bits());
        let Value::Number(negative_zero) = object.get("negative-zero").expect("negative zero")
        else {
            panic!("expected negative zero");
        };
        assert_eq!(negative_zero.as_f64().to_bits(), (-0.0_f64).to_bits());
    }

    #[test]
    fn memory_threshold_spills_and_drop_removes_private_file() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut spool = RuntimeSpool::new(RuntimeSpoolConfig {
            memory_threshold_bytes: 1,
            spool_directory: directory.path().to_owned(),
            ..config(directory.path())
        });
        spool.begin_root().unwrap();
        spool
            .push_item(RuntimeSpoolItem::Item(Value::string("value")))
            .unwrap();
        spool.finish_root().unwrap();
        let path = spool.spool_path().expect("spool path").to_owned();
        assert!(spool.spooled());
        assert!(path.exists());
        drop(spool);
        assert!(!path.exists());
    }

    #[test]
    fn replay_rejects_corrupt_length_before_allocating_payload() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut spool = RuntimeSpool::new(RuntimeSpoolConfig {
            maximum_item_bytes: 32,
            ..config(directory.path())
        });
        spool.begin_root().unwrap();
        spool
            .push_item(RuntimeSpoolItem::Item(Value::Null))
            .unwrap();
        spool.finish_root().unwrap();
        let Storage::Memory(bytes) = &mut spool.storage else {
            panic!("small record should remain in memory");
        };
        bytes[0..8].copy_from_slice(&u64::MAX.to_le_bytes());
        let error = spool.replay_items(|_| Ok::<(), ()>(())).unwrap_err();
        assert!(matches!(
            error,
            RuntimeSpoolReplayError::Storage(RuntimeSpoolError::ItemLimit)
        ));
    }

    #[test]
    fn cancellation_applies_to_push_and_replay() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let cancellation = Arc::new(AtomicBool::new(true));
        let mut spool = RuntimeSpool::new(config(directory.path()))
            .with_cancellation(Arc::clone(&cancellation));
        assert!(matches!(
            spool.begin_root(),
            Err(RuntimeSpoolError::Cancelled)
        ));
        cancellation.store(false, Ordering::Relaxed);
        spool.begin_root().unwrap();
        spool
            .push_item(RuntimeSpoolItem::Item(Value::Null))
            .unwrap();
        spool.finish_root().unwrap();
        cancellation.store(true, Ordering::Relaxed);
        assert!(matches!(
            spool.replay_items(|_| Ok::<(), ()>(())),
            Err(RuntimeSpoolReplayError::Storage(
                RuntimeSpoolError::Cancelled
            ))
        ));
    }

    #[test]
    fn lifecycle_reset_keeps_cumulative_budget_but_removes_records() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut spool = RuntimeSpool::new(config(directory.path()));
        spool.begin_root().unwrap();
        spool
            .push_item(RuntimeSpoolItem::Item(Value::Bool(true)))
            .unwrap();
        let total = spool.total_bytes_written();
        spool.finish_root().unwrap();
        spool.clear().unwrap();
        assert!(spool.is_empty());
        assert_eq!(spool.total_bytes_written(), total);
        assert_eq!(spool.item_count(), 0);
        assert!(matches!(
            spool.finish_root(),
            Err(RuntimeSpoolError::State("root is not active"))
        ));
    }

    #[test]
    fn typed_tags_roundtrip_in_source_order() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut spool = RuntimeSpool::new(config(directory.path()));
        spool.begin_root().unwrap();
        spool
            .push_item(RuntimeSpoolItem::Projected(Value::Null))
            .unwrap();
        spool
            .push_item(RuntimeSpoolItem::Base(Value::Bool(false)))
            .unwrap();
        spool
            .push_item(RuntimeSpoolItem::Item(Value::Bool(true)))
            .unwrap();
        spool.finish_root().unwrap();
        let mut tags = Vec::new();
        spool
            .replay_items(|item| {
                tags.push(match item {
                    RuntimeSpoolItem::Projected(_) => "projected",
                    RuntimeSpoolItem::Item(_) => "item",
                    RuntimeSpoolItem::Base(_) => "base",
                });
                Ok::<(), ()>(())
            })
            .unwrap();
        assert_eq!(tags, ["projected", "base", "item"]);
    }

    #[test]
    fn item_limit_excludes_record_header() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut spool = RuntimeSpool::new(RuntimeSpoolConfig {
            maximum_total_bytes: 10,
            maximum_item_bytes: 2,
            ..config(directory.path())
        });
        spool.begin_root().unwrap();
        spool
            .push_item(RuntimeSpoolItem::Item(Value::Null))
            .expect("one-byte tag and eight-byte header fit exactly");
        spool.finish_root().unwrap();
        spool
            .replay_items(|_| Ok::<(), ()>(()))
            .expect("exact-fit record replays");

        let mut too_small = RuntimeSpool::new(RuntimeSpoolConfig {
            maximum_total_bytes: 10,
            maximum_item_bytes: 1,
            ..config(directory.path())
        });
        too_small.begin_root().unwrap();
        assert!(matches!(
            too_small.push_item(RuntimeSpoolItem::Item(Value::Null)),
            Err(RuntimeSpoolError::ItemLimit)
        ));
        assert_eq!(too_small.retained_bytes(), 0);

        let mut header_too_small = RuntimeSpool::new(RuntimeSpoolConfig {
            maximum_total_bytes: 7,
            ..config(directory.path())
        });
        header_too_small.begin_root().unwrap();
        assert!(matches!(
            header_too_small.push_item(RuntimeSpoolItem::Item(Value::Null)),
            Err(RuntimeSpoolError::TotalLimit)
        ));
        assert_eq!(header_too_small.retained_bytes(), 0);
    }

    #[test]
    fn bounded_memory_reserve_handles_len_below_capacity() {
        let mut storage = Storage::Memory(Vec::with_capacity(4));
        let Storage::Memory(bytes) = &mut storage else {
            panic!("expected memory storage");
        };
        bytes.extend_from_slice(&[1, 2, 3]);
        storage
            .append(&[4, 5, 6], 6)
            .expect("exact threshold reserve");
        let Storage::Memory(bytes) = storage else {
            panic!("expected memory storage");
        };
        assert_eq!(bytes.len(), 6);
        assert!(bytes.capacity() <= 6);
    }

    #[test]
    fn lifecycle_rejects_unstaged_and_unsealed_access() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut spool = RuntimeSpool::new(config(directory.path()));
        assert!(matches!(
            spool.push_item(RuntimeSpoolItem::Item(Value::Null)),
            Err(RuntimeSpoolError::State("root is not active"))
        ));
        assert!(matches!(
            spool.replay_items(|_| Ok::<(), ()>(())),
            Err(RuntimeSpoolReplayError::Storage(RuntimeSpoolError::State(
                "root is not sealed"
            )))
        ));
        spool.begin_root().unwrap();
        assert!(matches!(
            spool.replay_items(|_| Ok::<(), ()>(())),
            Err(RuntimeSpoolReplayError::Storage(RuntimeSpoolError::State(
                "root is not sealed"
            )))
        ));
    }

    #[test]
    fn indexed_replay_validates_generation_and_offset_bounds() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut spool = RuntimeSpool::new(config(directory.path()));
        spool.begin_root().unwrap();
        let old = spool
            .push_item(RuntimeSpoolItem::Item(Value::string("old")))
            .unwrap();
        spool.replace_root().unwrap();
        spool
            .push_item(RuntimeSpoolItem::Item(Value::string("current")))
            .unwrap();
        spool.finish_root().unwrap();
        let stale = spool
            .replay_records([old], |_| Ok::<(), ()>(()))
            .unwrap_err();
        assert!(matches!(
            stale,
            RuntimeSpoolReplayError::Storage(RuntimeSpoolError::StaleRecord)
        ));

        for memory_threshold_bytes in [usize::MAX, 1] {
            let mut bounded = RuntimeSpool::new(RuntimeSpoolConfig {
                memory_threshold_bytes,
                ..config(directory.path())
            });
            bounded.begin_root().unwrap();
            let current = bounded
                .push_item(RuntimeSpoolItem::Item(Value::Null))
                .unwrap();
            bounded.finish_root().unwrap();
            let out_of_bounds = RuntimeRecordId {
                generation: current.generation,
                offset: bounded.retained_bytes().saturating_add(1),
            };
            let bounds = bounded
                .replay_records([out_of_bounds], |_| Ok::<(), ()>(()))
                .unwrap_err();
            assert!(matches!(
                bounds,
                RuntimeSpoolReplayError::Storage(RuntimeSpoolError::RecordBounds)
            ));

            let mut short_header = RuntimeSpool::new(RuntimeSpoolConfig {
                memory_threshold_bytes,
                ..config(directory.path())
            });
            short_header.begin_root().unwrap();
            short_header
                .push_item(RuntimeSpoolItem::Item(Value::Null))
                .unwrap();
            short_header.finish_root().unwrap();
            let short_header_id = RuntimeRecordId {
                generation: short_header.generation,
                offset: short_header.retained_bytes().saturating_sub(4),
            };
            let short_bounds = short_header
                .replay_records([short_header_id], |_| Ok::<(), ()>(()))
                .unwrap_err();
            assert!(matches!(
                short_bounds,
                RuntimeSpoolReplayError::Storage(RuntimeSpoolError::RecordBounds)
            ));
        }
    }

    #[test]
    fn indexed_file_replay_is_out_of_order_and_preserves_nested_values() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut spool = RuntimeSpool::new(RuntimeSpoolConfig {
            memory_threshold_bytes: 1,
            ..config(directory.path())
        });
        spool.begin_root().unwrap();
        let first = spool
            .push_item(RuntimeSpoolItem::Item(Value::array(vec![
                Value::string("first"),
                Value::array(vec![Value::string("nested")]),
            ])))
            .unwrap();
        let mut object = Object::new();
        object.insert("key".into(), Value::string("second"));
        let second = spool
            .push_item(RuntimeSpoolItem::Base(Value::object(object)))
            .unwrap();
        spool.finish_root().unwrap();
        assert!(spool.spooled());
        let mut replayed = Vec::new();
        spool
            .replay_records([second, first], |item| {
                replayed.push(item);
                Ok::<(), ()>(())
            })
            .unwrap();
        assert_eq!(replayed.len(), 2);
        assert!(matches!(
            &replayed[0],
            RuntimeSpoolItem::Base(Value::Object(object))
                if object.get("key") == Some(&Value::string("second"))
        ));
        assert!(matches!(
            &replayed[1],
            RuntimeSpoolItem::Item(Value::Array(values))
                if values.len() == 2
                    && values[0] == Value::string("first")
                    && values[1]
                        == Value::array(vec![Value::string("nested")])
        ));
    }

    #[test]
    fn indexed_replay_failure_consumes_the_sealed_generation() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut spool = RuntimeSpool::new(config(directory.path()));
        spool.begin_root().unwrap();
        let id = spool
            .push_item(RuntimeSpoolItem::Projected(Value::Null))
            .unwrap();
        spool.finish_root().unwrap();
        assert!(matches!(
            spool.replay_records([id], |_| Err::<(), u8>(9)),
            Err(RuntimeSpoolReplayError::Consumer(9))
        ));
        assert!(matches!(
            spool.replay_records([id], |_| Ok::<(), ()>(())),
            Err(RuntimeSpoolReplayError::Storage(RuntimeSpoolError::State(
                "root is not sealed"
            )))
        ));
    }

    #[test]
    fn explicit_memory_pressure_spills_shared_stage() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut spool = RuntimeSpool::new(RuntimeSpoolConfig {
            memory_threshold_bytes: 1024,
            ..config(directory.path())
        });
        spool.begin_root().unwrap();
        spool
            .push_item(RuntimeSpoolItem::Item(Value::array(vec![Value::string(
                "retained value",
            )])))
            .unwrap();
        assert!(spool.retained_memory_capacity_bytes() > 0);
        spool.set_memory_threshold(1).unwrap();
        assert!(spool.spooled());
        assert_eq!(spool.retained_memory_capacity_bytes(), 0);
        spool.finish_root().unwrap();
        spool
            .replay_items(|_| Ok::<(), ()>(()))
            .expect("spilled stage replays");
    }

    #[test]
    fn capacity_pressure_spills_even_when_logical_bytes_fit() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut spool = RuntimeSpool::new(RuntimeSpoolConfig {
            memory_threshold_bytes: 1024,
            ..config(directory.path())
        });
        spool.begin_root().unwrap();
        spool
            .push_item(RuntimeSpoolItem::Item(Value::Null))
            .unwrap();
        let logical_bytes = {
            let Storage::Memory(bytes) = &mut spool.storage else {
                panic!("record should initially remain in memory");
            };
            bytes.reserve(32);
            bytes.len()
        };
        assert!(spool.retained_memory_capacity_bytes() > logical_bytes);
        spool.set_memory_threshold(logical_bytes).unwrap();
        assert!(spool.spooled());
        assert_eq!(spool.retained_memory_capacity_bytes(), 0);
    }

    #[test]
    fn physical_spool_accounting_includes_copy_and_header_patch() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut spool = RuntimeSpool::new(RuntimeSpoolConfig {
            memory_threshold_bytes: RECORD_HEADER_BYTES,
            maximum_spool_bytes: 18,
            ..config(directory.path())
        });
        spool.begin_root().unwrap();
        spool
            .push_item(RuntimeSpoolItem::Item(Value::Null))
            .expect("one item fits the exact physical budget");
        assert!(spool.spooled());
        assert_eq!(spool.retained_bytes(), 10);
        assert_eq!(spool.total_bytes_written(), 10);
        assert_eq!(spool.spool_bytes_written(), 18);
    }

    #[test]
    fn physical_spool_budget_is_cumulative_across_replace_and_abort() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut spool = RuntimeSpool::new(RuntimeSpoolConfig {
            memory_threshold_bytes: RECORD_HEADER_BYTES,
            maximum_spool_bytes: 36,
            ..config(directory.path())
        });
        spool.begin_root().unwrap();
        spool
            .push_item(RuntimeSpoolItem::Item(Value::Null))
            .unwrap();
        assert_eq!(spool.spool_bytes_written(), 18);
        spool.replace_root().unwrap();
        spool
            .push_item(RuntimeSpoolItem::Item(Value::Null))
            .unwrap();
        assert_eq!(spool.spool_bytes_written(), 36);
        spool.abort_root().unwrap();
        assert_eq!(spool.spool_bytes_written(), 36);
        assert!(spool.is_empty());

        spool.begin_root().unwrap();
        assert!(matches!(
            spool.push_item(RuntimeSpoolItem::Item(Value::Null)),
            Err(RuntimeSpoolError::SpoolLimit)
        ));
        assert_eq!(spool.spool_bytes_written(), 36);
        assert_eq!(spool.retained_bytes(), 0);
    }

    #[test]
    fn replacements_and_abort_preserve_cumulative_limits() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut spool = RuntimeSpool::new(RuntimeSpoolConfig {
            maximum_total_bytes: 20,
            ..config(directory.path())
        });
        spool.begin_root().unwrap();
        spool
            .push_item(RuntimeSpoolItem::Item(Value::Null))
            .unwrap();
        let first_total = spool.total_bytes_written();
        spool.replace_root().unwrap();
        assert_eq!(spool.total_bytes_written(), first_total);
        spool
            .push_item(RuntimeSpoolItem::Item(Value::Null))
            .unwrap();
        spool.abort_root().unwrap();
        assert_eq!(spool.total_bytes_written(), first_total * 2);
        spool.begin_root().unwrap();
        assert!(matches!(
            spool.push_item(RuntimeSpoolItem::Item(Value::Null)),
            Err(RuntimeSpoolError::TotalLimit)
        ));
    }

    #[test]
    fn generation_exhaustion_cleans_storage_before_failing_closed() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut aborted = RuntimeSpool::new(RuntimeSpoolConfig {
            memory_threshold_bytes: 1,
            ..config(directory.path())
        });
        aborted.begin_root().unwrap();
        aborted
            .push_item(RuntimeSpoolItem::Item(Value::string("spooled")))
            .unwrap();
        let path = aborted.spool_path().expect("spool path").to_owned();
        aborted.generation = u64::MAX;
        assert!(matches!(
            aborted.abort_root(),
            Err(RuntimeSpoolError::GenerationLimit)
        ));
        assert!(aborted.is_empty());
        assert!(!path.exists());
        assert!(!aborted.root_active);

        let mut cleared = RuntimeSpool::new(config(directory.path()));
        cleared.begin_root().unwrap();
        cleared
            .push_item(RuntimeSpoolItem::Item(Value::Null))
            .unwrap();
        cleared.generation = u64::MAX;
        assert!(matches!(
            cleared.clear(),
            Err(RuntimeSpoolError::GenerationLimit)
        ));
        assert!(cleared.is_empty());
        assert!(!cleared.root_active);
        assert!(!cleared.root_sealed);
        assert!(matches!(
            cleared.begin_root(),
            Err(RuntimeSpoolError::GenerationLimit)
        ));
    }

    #[test]
    fn replay_preserves_consumer_error_and_applies_decoded_budget() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut spool = RuntimeSpool::new(config(directory.path()));
        spool.begin_root().unwrap();
        spool
            .push_item(RuntimeSpoolItem::Item(Value::Null))
            .unwrap();
        spool.finish_root().unwrap();
        let error = spool.replay_items(|_| Err::<(), u8>(7)).unwrap_err();
        assert!(matches!(error, RuntimeSpoolReplayError::Consumer(7)));
        assert!(matches!(
            spool.replay_items(|_| Ok::<(), ()>(())),
            Err(RuntimeSpoolReplayError::Storage(RuntimeSpoolError::State(
                "root is not sealed"
            )))
        ));

        let mut bounded = RuntimeSpool::new(RuntimeSpoolConfig {
            maximum_decoded_bytes: 1,
            ..config(directory.path())
        });
        bounded.begin_root().unwrap();
        bounded
            .push_item(RuntimeSpoolItem::Item(Value::Null))
            .unwrap();
        bounded.finish_root().unwrap();
        assert!(matches!(
            bounded.replay_items(|_| Ok::<(), ()>(())),
            Err(RuntimeSpoolReplayError::Storage(
                RuntimeSpoolError::DecodedLimit
            ))
        ));

        let mut bounded_string = RuntimeSpool::new(RuntimeSpoolConfig {
            maximum_decoded_bytes: 1,
            ..config(directory.path())
        });
        bounded_string.begin_root().unwrap();
        bounded_string
            .push_item(RuntimeSpoolItem::Item(Value::string("large")))
            .unwrap();
        bounded_string.finish_root().unwrap();
        assert!(matches!(
            bounded_string.replay_items(|_| Ok::<(), ()>(())),
            Err(RuntimeSpoolReplayError::Storage(
                RuntimeSpoolError::DecodedLimit
            ))
        ));
    }

    #[test]
    fn replay_budget_is_bounded_and_restores_on_a_fresh_root() {
        assert_eq!(REPLAY_BUFFER_BYTES, 8192);
        for memory_threshold_bytes in [usize::MAX, 1] {
            let directory = tempfile::tempdir().expect("temporary directory");
            let mut spool = RuntimeSpool::new(RuntimeSpoolConfig {
                memory_threshold_bytes,
                maximum_decoded_bytes: 64,
                ..config(directory.path())
            });
            spool.begin_root().unwrap();
            spool
                .push_item(RuntimeSpoolItem::Item(Value::string("value")))
                .unwrap();
            spool.finish_root().unwrap();
            assert_eq!(spool.spooled(), memory_threshold_bytes == 1);

            spool.set_replay_decoded_budget(1).unwrap();
            let mut callback_count = 0;
            assert!(matches!(
                spool.replay_items(|_| {
                    callback_count += 1;
                    Ok::<(), ()>(())
                }),
                Err(RuntimeSpoolReplayError::Storage(
                    RuntimeSpoolError::DecodedLimit
                ))
            ));
            assert_eq!(callback_count, 0);

            assert!(matches!(
                spool.set_replay_decoded_budget(65),
                Err(RuntimeSpoolError::DecodedLimit)
            ));

            spool.begin_root().unwrap();
            spool
                .push_item(RuntimeSpoolItem::Item(Value::string("value")))
                .unwrap();
            spool
                .set_replay_decoded_budget(64)
                .expect("fresh root can restore the configured ceiling");
            spool.finish_root().unwrap();
            let mut replayed = 0;
            spool
                .replay_items(|_| {
                    replayed += 1;
                    Ok::<(), ()>(())
                })
                .unwrap();
            assert_eq!(replayed, 1);
        }
    }

    #[test]
    fn reported_decoded_requirement_covers_replay_charge() {
        let mut object = Object::new();
        object.insert(
            "decimal".into(),
            Value::Number(Number::parse("1.2300e+4").expect("finite literal")),
        );
        let values = [
            Value::Null,
            Value::array(vec![Value::Null, Value::string("array")]),
            Value::object(object),
            Value::Number(Number::parse("1.2300e+4").expect("finite literal")),
        ];

        for value in values {
            let directory = tempfile::tempdir().expect("temporary directory");
            let mut spool = RuntimeSpool::new(config(directory.path()));
            spool.begin_root().unwrap();
            spool.push_item(RuntimeSpoolItem::Item(value)).unwrap();
            let requirement = spool.maximum_decoded_requirement();
            assert!(requirement > 0);
            spool
                .set_replay_decoded_budget(
                    usize::try_from(requirement).expect("test value fits usize"),
                )
                .unwrap();
            spool.finish_root().unwrap();
            spool
                .replay_items(|_| Ok::<(), ()>(()))
                .expect("reported bound admits authoritative decoder charge");
        }
    }

    #[test]
    fn decoded_requirement_resets_and_does_not_bypass_ceiling() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut spool = RuntimeSpool::new(config(directory.path()));
        spool.begin_root().unwrap();
        spool
            .push_item(RuntimeSpoolItem::Item(Value::array(vec![
                Value::Null,
                Value::string("value"),
            ])))
            .unwrap();
        assert!(spool.maximum_decoded_requirement() > 0);
        spool.replace_root().unwrap();
        assert_eq!(spool.maximum_decoded_requirement(), 0);

        let mut bounded = RuntimeSpool::new(RuntimeSpoolConfig {
            maximum_decoded_bytes: 1,
            ..config(directory.path())
        });
        bounded.begin_root().unwrap();
        bounded
            .push_item(RuntimeSpoolItem::Item(Value::string("value")))
            .unwrap();
        let requirement = bounded.maximum_decoded_requirement();
        assert!(requirement > 1);
        assert!(matches!(
            bounded.set_replay_decoded_budget(
                usize::try_from(requirement).expect("test value fits usize")
            ),
            Err(RuntimeSpoolError::DecodedLimit)
        ));
        bounded.finish_root().unwrap();
        assert!(matches!(
            bounded.replay_items(|_| Ok::<(), ()>(())),
            Err(RuntimeSpoolReplayError::Storage(
                RuntimeSpoolError::DecodedLimit
            ))
        ));
    }

    #[test]
    fn clear_removes_spool_even_after_cancellation() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let cancellation = Arc::new(AtomicBool::new(false));
        let mut spool = RuntimeSpool::new(RuntimeSpoolConfig {
            memory_threshold_bytes: 1,
            ..config(directory.path())
        })
        .with_cancellation(Arc::clone(&cancellation));
        spool.begin_root().unwrap();
        spool
            .push_item(RuntimeSpoolItem::Item(Value::string("value")))
            .unwrap();
        let path = spool.spool_path().expect("spool path").to_owned();
        cancellation.store(true, Ordering::Relaxed);
        spool.clear().unwrap();
        assert!(!path.exists());
    }
}
