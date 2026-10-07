use std::ops::Deref;
use std::{
    collections::{BTreeMap, btree_map::Entry},
    io::{self, Read, Seek, SeekFrom, Write},
    sync::Arc,
};

use crate::schema::{FieldSchema, RowSchema};
use tq_core::presentation::ColorPalette;

use crate::{
    DuplicateKeyPolicy, Scalar, ScalarToken, WriterConfig,
    spool::{
        ArrayPreparationConfig, PreparationArena, PreparationFrame, PreparationMemory, Spool,
        SpoolError, create_spool,
    },
    writer::{self, ScalarContext, WriterError},
};

#[path = "tape_render.rs"]
mod tape_render;
#[path = "tape_scalar.rs"]
mod tape_scalar;

#[derive(Debug, thiserror::Error)]
pub(crate) enum TapeError {
    #[error(transparent)]
    Spool(#[from] SpoolError),
    #[error(transparent)]
    Writer(#[from] WriterError),
    #[error("duplicate object key: {0}")]
    Duplicate(Arc<str>),
    #[error("invalid prepared tape structure: {0}")]
    Structure(&'static str),
}
impl From<io::Error> for TapeError {
    fn from(e: io::Error) -> Self {
        Self::Writer(WriterError::Io(e))
    }
}
impl From<crate::schema::SchemaError> for TapeError {
    fn from(error: crate::schema::SchemaError) -> Self {
        Self::Writer(error.into())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ContainerKind {
    Object,
    Array,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct PreparedNode {
    index: u64,
    truthy: bool,
}
#[derive(Debug)]
pub(crate) struct PreparedContainer {
    kind: ContainerKind,
    children: Vec<u64>,
    chunks: Vec<u64>,
    members: BTreeMap<Arc<str>, usize>,
    ordered_members: Vec<(Arc<str>, u64)>,
    pending: Option<usize>,
    frame: PreparationFrame,
    charge: PreparationMemory,
    count: u64,
}
#[derive(Debug)]
enum Store {
    Memory(Vec<u8>),
    File(Spool),
}
#[derive(Debug)]
struct ObjectCursor {
    count: u64,
    start: u64,
    next: u64,
    remaining: u64,
    _charge: PreparationMemory,
}
impl ObjectCursor {
    fn rewind(&mut self) {
        self.next = self.start;
        self.remaining = self.count;
    }
}
struct ObjectEntry {
    key_at: u64,
    key_bytes: u64,
    child: u64,
}
#[derive(Debug)]
struct ObjectFields {
    fields: Vec<(String, u64)>,
    _charge: PreparationMemory,
}
impl Deref for ObjectFields {
    type Target = [(String, u64)];
    fn deref(&self) -> &Self::Target {
        &self.fields
    }
}
#[derive(Debug)]
pub(crate) struct ArrayCursor {
    count: u64,
    offsets: Vec<u64>,
    cached_chunk: Option<usize>,
    cache: [u8; CHUNK * 8],
    _charge: PreparationMemory,
}
#[derive(Debug)]
pub(crate) struct PreparedTape {
    config: ArrayPreparationConfig,
    arena: PreparationArena,
    policy: DuplicateKeyPolicy,
    store: Store,
    store_end: u64,
    file_cursor: Option<u64>,
    store_charge: PreparationMemory,
    write_buffer: Vec<u8>,
    write_charge: PreparationMemory,
    read_cache: Vec<u8>,
    read_cache_at: Option<u64>,
    read_charge: PreparationMemory,
    scratch: Vec<u8>,
    scratch_charge: PreparationMemory,
}
const SCALAR_HEADER: usize = 10;
const CHUNK: usize = 64;
const IO_BUFFER_TARGET: usize = 8 * 1024;
// A std B-tree node has eleven entry slots, a parent/header, and up to twelve
// child pointers. Charge a whole node per unique member before insertion:
// every retained node owns at least one member, including the first leaf.
const OBJECT_MEMBER_METADATA_BYTES: usize =
    11 * std::mem::size_of::<(Arc<str>, (usize, u64))>() + 16 * std::mem::size_of::<usize>();
fn numeric_scratch_bound(value: &str) -> usize {
    value.len().saturating_mul(16).saturating_add(8192)
}
fn small_canonical_integer(value: &str) -> bool {
    let bytes = value.as_bytes();
    let digits = if bytes.first() == Some(&b'-') {
        &bytes[1..]
    } else {
        bytes
    };
    !digits.is_empty()
        && digits.len() <= 15
        && digits.iter().all(u8::is_ascii_digit)
        && (digits.len() == 1 || digits[0] != b'0')
        && value != "-0"
}

fn write_spool_counted(
    file: &mut std::fs::File,
    arena: &PreparationArena,
    mut bytes: &[u8],
) -> io::Result<()> {
    while !bytes.is_empty() {
        match file.write(bytes) {
            Ok(0) => return Err(io::Error::from(io::ErrorKind::WriteZero)),
            Ok(written) => {
                arena.wrote_spool(u64::try_from(written).map_err(io::Error::other)?);
                bytes = &bytes[written..];
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}
impl PreparedTape {
    pub(crate) fn new(
        config: ArrayPreparationConfig,
        arena: PreparationArena,
        policy: DuplicateKeyPolicy,
    ) -> Self {
        let store_charge = arena.memory_charge();
        let write_charge = arena.memory_charge();
        let read_charge = arena.memory_charge();
        let scratch_charge = arena.memory_charge();
        Self {
            config,
            arena,
            policy,
            store: Store::Memory(Vec::new()),
            store_end: 0,
            file_cursor: None,
            store_charge,
            write_buffer: Vec::new(),
            write_charge,
            read_cache: Vec::new(),
            read_cache_at: None,
            read_charge,
            scratch: Vec::new(),
            scratch_charge,
        }
    }
    pub(crate) fn begin(&mut self, kind: ContainerKind) -> Result<PreparedContainer, TapeError> {
        let frame = self.arena.enter()?;
        match kind {
            ContainerKind::Object => self.arena.record_object_preparation(),
            ContainerKind::Array => self.arena.record_array_preparation(),
        }
        let mut charge = self.arena.memory_charge();
        let mut children = Vec::new();
        if kind == ContainerKind::Array {
            charge.grow(CHUNK * std::mem::size_of::<u64>())?;
            children
                .try_reserve_exact(CHUNK)
                .map_err(|_| SpoolError::MemoryLimit)?;
        }
        Ok(PreparedContainer {
            kind,
            children,
            chunks: Vec::new(),
            members: BTreeMap::new(),
            ordered_members: Vec::new(),
            pending: None,
            frame,
            charge,
            count: 0,
        })
    }
    pub(crate) fn key(
        &mut self,
        parent: &mut PreparedContainer,
        key: Arc<str>,
    ) -> Result<(), TapeError> {
        if parent.kind != ContainerKind::Object || parent.pending.is_some() {
            return Err(TapeError::Structure(
                "key outside object or key already pending",
            ));
        }
        let position = parent.ordered_members.len();
        let position = match parent.members.entry(key) {
            Entry::Occupied(entry) => {
                if self.policy == DuplicateKeyPolicy::Reject {
                    return Err(TapeError::Duplicate(Arc::clone(entry.key())));
                }
                *entry.get()
            }
            Entry::Vacant(entry) => {
                parent.charge.grow(
                    entry.key().len().saturating_add(
                        std::mem::size_of::<Arc<str>>() + 3 * std::mem::size_of::<usize>(),
                    ) + OBJECT_MEMBER_METADATA_BYTES,
                )?;
                if parent.ordered_members.len() == parent.ordered_members.capacity() {
                    let previous = parent.ordered_members.capacity();
                    let additional = previous.max(4);
                    let element_bytes = std::mem::size_of::<(Arc<str>, u64)>();
                    let requested_bytes = additional.saturating_mul(element_bytes);
                    parent.charge.grow(requested_bytes)?;
                    if parent
                        .ordered_members
                        .try_reserve_exact(additional)
                        .is_err()
                    {
                        parent.charge.shrink(requested_bytes);
                        return Err(SpoolError::MemoryLimit.into());
                    }
                    let actual = parent.ordered_members.capacity().saturating_sub(previous);
                    let excess = actual.saturating_sub(additional);
                    if excess != 0
                        && let Err(error) = parent.charge.grow(excess.saturating_mul(element_bytes))
                    {
                        // An abort must not retain capacity that could not be admitted.
                        parent.ordered_members = Vec::new();
                        parent.charge.shrink(
                            previous
                                .saturating_mul(element_bytes)
                                .saturating_add(requested_bytes),
                        );
                        return Err(error.into());
                    }
                }
                // The pending slot cannot be replayed until push supplies its value.
                parent.ordered_members.push((Arc::clone(entry.key()), 0));
                entry.insert(position);
                position
            }
        };
        parent.pending = Some(position);
        Ok(())
    }
    pub(crate) fn scalar(&mut self, value: Scalar) -> Result<PreparedNode, TapeError> {
        match value {
            Scalar::Null => self.scalar_token(ScalarToken::Null),
            Scalar::Bool(v) => self.scalar_token(ScalarToken::Bool(v)),
            Scalar::String(v) => self.scalar_token(ScalarToken::String(&v)),
            Scalar::Number(v) => {
                if let Some(literal) = v.exact_literal()
                    && small_canonical_integer(literal)
                {
                    return self.store_scalar_bytes(2, literal.as_bytes(), 0);
                }
                let mut charge = self.arena.memory_charge();
                charge.grow(v.exact_literal().map_or(8192, numeric_scratch_bound))?;
                let n = v.canonical_numeric();
                self.store_scalar_bytes(2, n.as_bytes(), 0)
            }
        }
    }
    pub(crate) fn scalar_token(
        &mut self,
        value: ScalarToken<'_>,
    ) -> Result<PreparedNode, TapeError> {
        if let ScalarToken::Number(value) = value {
            if small_canonical_integer(value) {
                return self.store_scalar_bytes(2, value.as_bytes(), 0);
            }
            let mut charge = self.arena.memory_charge();
            charge.grow(numeric_scratch_bound(value))?;
            let canonical = tq_core::Number::canonicalize_output_literal(value)
                .map_err(|_| TapeError::Structure("invalid number token"))?;
            return self.store_scalar_bytes(2, canonical.as_bytes(), 0);
        }
        let (tag, data, mask) = match value {
            ScalarToken::Null => (0, &b""[..], 0),
            ScalarToken::Bool(false) => (3, &b""[..], 0),
            ScalarToken::Bool(true) => (4, &b""[..], 0),
            ScalarToken::String(s) => (1, s.as_bytes(), writer::string_quote_mask(s)),
            ScalarToken::Number(_) => unreachable!(),
        };
        self.store_scalar_bytes(tag, data, mask)
    }
    fn store_scalar_bytes(
        &mut self,
        tag: u8,
        data: &[u8],
        mask: u8,
    ) -> Result<PreparedNode, TapeError> {
        let mut header = [0u8; SCALAR_HEADER];
        header[0] = tag;
        header[1..9].copy_from_slice(
            &u64::try_from(data.len())
                .map_err(|_| SpoolError::Limit)?
                .to_le_bytes(),
        );
        header[9] = mask;
        let index = self.append(&header)?;
        self.append(data)?;
        Ok(PreparedNode {
            index,
            truthy: !matches!(tag, 0 | 3),
        })
    }
    fn append(&mut self, bytes: &[u8]) -> Result<u64, TapeError> {
        let offset = self.store_end;
        let end = offset
            .checked_add(u64::try_from(bytes.len()).map_err(|_| SpoolError::Limit)?)
            .ok_or(SpoolError::Limit)?;
        if let Store::Memory(memory) = &mut self.store {
            let needed = memory
                .len()
                .checked_add(bytes.len())
                .ok_or(SpoolError::Limit)?;
            if needed <= self.config.memory_threshold_bytes {
                if needed > memory.capacity() {
                    let previous = memory.capacity();
                    let target = needed
                        .max(previous.saturating_mul(2))
                        .min(self.config.memory_threshold_bytes);
                    let reserved = target.saturating_sub(previous);
                    if self.store_charge.grow(reserved).is_ok() {
                        if memory
                            .try_reserve_exact(target.saturating_sub(memory.len()))
                            .is_ok()
                        {
                            let actual = memory.capacity().saturating_sub(previous);
                            if actual <= reserved {
                                self.store_charge.shrink(reserved - actual);
                                memory.extend_from_slice(bytes);
                                self.store_end = end;
                                return Ok(offset);
                            }
                            if self.store_charge.grow(actual - reserved).is_ok() {
                                memory.extend_from_slice(bytes);
                                self.store_end = end;
                                return Ok(offset);
                            }
                            return Err(SpoolError::MemoryLimit.into());
                        }
                        self.store_charge.shrink(reserved);
                    }
                } else {
                    memory.extend_from_slice(bytes);
                    self.store_end = end;
                    return Ok(offset);
                }
            }
            if !self.config.allow_spool {
                return Err(SpoolError::Disabled.into());
            }
            let prior = u64::try_from(memory.len()).map_err(|_| SpoolError::Limit)?;
            let disk = prior
                .checked_add(u64::try_from(bytes.len()).map_err(|_| SpoolError::Limit)?)
                .ok_or(SpoolError::Limit)?;
            if disk > self.config.maximum_spool_bytes || !self.arena.can_write_spool(disk) {
                return Err(SpoolError::Limit.into());
            }
            let mut file = create_spool(&self.config.spool_directory).map_err(SpoolError::from)?;
            write_spool_counted(&mut file.file, &self.arena, memory).map_err(SpoolError::from)?;
            self.store_charge.shrink(memory.capacity());
            self.store = Store::File(file);
            self.file_cursor = Some(prior);
        }
        let buffered = self
            .write_buffer
            .len()
            .checked_add(bytes.len())
            .ok_or(SpoolError::Limit)?;
        if end > self.config.maximum_spool_bytes
            || !self
                .arena
                .can_write_spool(u64::try_from(buffered).map_err(|_| SpoolError::Limit)?)
        {
            return Err(SpoolError::Limit.into());
        }
        self.ensure_file_buffers();
        if bytes.len() > self.write_buffer.capacity() {
            self.flush_writes()?;
            let Store::File(file) = &mut self.store else {
                unreachable!()
            };
            if self.file_cursor != Some(self.store_end) {
                file.file
                    .seek(SeekFrom::Start(self.store_end))
                    .map_err(SpoolError::from)?;
            }
            write_spool_counted(&mut file.file, &self.arena, bytes).map_err(SpoolError::from)?;
        } else {
            if self.write_buffer.len() + bytes.len() > self.write_buffer.capacity() {
                self.flush_writes()?;
            }
            self.write_buffer.extend_from_slice(bytes);
        }
        self.store_end = end;
        Ok(offset)
    }
    fn ensure_file_buffers(&mut self) {
        if !self.write_buffer.is_empty() || self.write_buffer.capacity() != 0 {
            return;
        }
        let mut size = IO_BUFFER_TARGET;
        while size >= 256 {
            let mut wc = self.arena.memory_charge();
            let mut rc = self.arena.memory_charge();
            if wc.grow(size).is_ok() && rc.grow(size).is_ok() {
                let mut wb = Vec::new();
                let mut rb = Vec::new();
                if wb.try_reserve_exact(size).is_ok() && rb.try_reserve_exact(size).is_ok() {
                    self.write_buffer = wb;
                    self.read_cache = rb;
                    self.write_charge = wc;
                    self.read_charge = rc;
                    return;
                }
            }
            size /= 2;
        }
    }
    fn flush_writes(&mut self) -> Result<(), TapeError> {
        if self.write_buffer.is_empty() {
            return Ok(());
        }
        let start = self.store_end
            - u64::try_from(self.write_buffer.len()).map_err(|_| SpoolError::Limit)?;
        let Store::File(file) = &mut self.store else {
            return Ok(());
        };
        if self.file_cursor != Some(start) {
            file.file
                .seek(SeekFrom::Start(start))
                .map_err(SpoolError::from)?;
        }
        write_spool_counted(&mut file.file, &self.arena, &self.write_buffer)
            .map_err(SpoolError::from)?;
        self.file_cursor = Some(self.store_end);
        self.write_buffer.clear();
        self.read_cache_at = None;
        Ok(())
    }
    fn read_at(&mut self, offset: u64, out: &mut [u8]) -> Result<(), TapeError> {
        if matches!(self.store, Store::File(_)) {
            self.flush_writes()?;
        }
        match &mut self.store {
            Store::Memory(v) => {
                let a = usize::try_from(offset).map_err(|_| SpoolError::Limit)?;
                let b = a.checked_add(out.len()).ok_or(SpoolError::Limit)?;
                out.copy_from_slice(
                    v.get(a..b)
                        .ok_or(TapeError::Structure("record range invalid"))?,
                );
            }
            Store::File(_) => {
                self.ensure_file_buffers();
                let mut copied = 0;
                while copied < out.len() {
                    let at = offset
                        .checked_add(u64::try_from(copied).map_err(|_| SpoolError::Limit)?)
                        .ok_or(SpoolError::Limit)?;
                    let cache_end = self
                        .read_cache_at
                        .unwrap_or(0)
                        .checked_add(
                            u64::try_from(self.read_cache.len()).map_err(|_| SpoolError::Limit)?,
                        )
                        .ok_or(SpoolError::Limit)?;
                    if let Some(start) = self.read_cache_at.filter(|s| at >= *s && at < cache_end) {
                        let from = usize::try_from(at - start).map_err(|_| SpoolError::Limit)?;
                        let n = (out.len() - copied).min(self.read_cache.len() - from);
                        out[copied..copied + n].copy_from_slice(&self.read_cache[from..from + n]);
                        copied += n;
                        continue;
                    }
                    let cap = self.read_cache.capacity();
                    let cache_start = if cap == 0 {
                        at
                    } else {
                        at - at % u64::try_from(cap).map_err(|_| SpoolError::Limit)?
                    };
                    let Store::File(file) = &mut self.store else {
                        unreachable!()
                    };
                    if self.file_cursor != Some(cache_start) {
                        file.file
                            .seek(SeekFrom::Start(cache_start))
                            .map_err(SpoolError::from)?;
                    }
                    if cap == 0 {
                        let n = out.len() - copied;
                        file.file
                            .read_exact(&mut out[copied..])
                            .map_err(SpoolError::from)?;
                        self.file_cursor = Some(
                            at.checked_add(u64::try_from(n).map_err(|_| SpoolError::Limit)?)
                                .ok_or(SpoolError::Limit)?,
                        );
                        self.arena
                            .replayed_spool(u64::try_from(n).map_err(|_| SpoolError::Limit)?);
                        return Ok(());
                    }
                    self.read_cache.resize(cap, 0);
                    let got = file
                        .file
                        .read(&mut self.read_cache)
                        .map_err(SpoolError::from)?;
                    if got == 0 {
                        return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into());
                    }
                    self.read_cache.truncate(got);
                    self.read_cache_at = Some(cache_start);
                    let got_u64 = u64::try_from(got).map_err(|_| SpoolError::Limit)?;
                    self.file_cursor =
                        Some(cache_start.checked_add(got_u64).ok_or(SpoolError::Limit)?);
                    self.arena.replayed_spool(got_u64);
                }
            }
        }
        Ok(())
    }
    pub(crate) fn push(
        &mut self,
        parent: &mut PreparedContainer,
        child: PreparedNode,
    ) -> Result<(), TapeError> {
        parent.count = parent.count.checked_add(1).ok_or(SpoolError::Limit)?;
        match parent.kind {
            ContainerKind::Array => {
                if parent.children.len() == parent.children.capacity() {
                    let additional = parent.children.capacity().max(CHUNK);
                    parent
                        .charge
                        .grow(additional * std::mem::size_of::<u64>())?;
                    parent
                        .children
                        .try_reserve_exact(additional)
                        .map_err(|_| SpoolError::MemoryLimit)?;
                }
                parent.children.push(child.index);
                if parent.children.len() == CHUNK {
                    self.flush_chunk(parent)?;
                }
            }
            ContainerKind::Object => {
                let position = parent
                    .pending
                    .take()
                    .ok_or(TapeError::Structure("object value has no key"))?;
                parent
                    .ordered_members
                    .get_mut(position)
                    .ok_or(TapeError::Structure("object key has no ordered slot"))?
                    .1 = child.index;
            }
        }
        Ok(())
    }
    fn flush_chunk(&mut self, parent: &mut PreparedContainer) -> Result<(), TapeError> {
        if parent.children.is_empty() {
            return Ok(());
        }
        let length = parent.children.len();
        let mut record = [0u8; 1 + 8 + CHUNK * 8];
        record[0] = 7;
        record[1..9].copy_from_slice(
            &u64::try_from(length)
                .map_err(|_| SpoolError::Limit)?
                .to_le_bytes(),
        );
        for (i, c) in parent.children.drain(..).enumerate() {
            record[9 + i * 8..17 + i * 8].copy_from_slice(&c.to_le_bytes());
        }
        let at = self.append(&record[..9 + length * 8])?;
        if parent.chunks.len() == parent.chunks.capacity() {
            let additional = parent.chunks.capacity().max(1);
            parent
                .charge
                .grow(additional * std::mem::size_of::<u64>())?;
            parent
                .chunks
                .try_reserve_exact(additional)
                .map_err(|_| SpoolError::MemoryLimit)?;
        }
        parent.chunks.push(at);
        Ok(())
    }
    pub(crate) fn finish(
        &mut self,
        mut parent: PreparedContainer,
    ) -> Result<PreparedNode, TapeError> {
        if parent.pending.is_some() {
            return Err(TapeError::Structure("object ended with pending key"));
        }
        let index = match parent.kind {
            ContainerKind::Array => {
                self.flush_chunk(&mut parent)?;
                let mut head = [0u8; 17];
                head[0] = 5;
                head[1..9].copy_from_slice(&parent.count.to_le_bytes());
                head[9..17].copy_from_slice(
                    &u64::try_from(parent.chunks.len())
                        .map_err(|_| SpoolError::Limit)?
                        .to_le_bytes(),
                );
                let at = self.append(&head)?;
                for chunk in parent.chunks {
                    self.append(&chunk.to_le_bytes())?;
                }
                at
            }
            ContainerKind::Object => {
                drop(parent.members);
                let ordered = parent.ordered_members;
                parent
                    .charge
                    .shrink(ordered.len().saturating_mul(OBJECT_MEMBER_METADATA_BYTES));
                let mut head = [0u8; 9];
                head[0] = 6;
                head[1..9].copy_from_slice(
                    &u64::try_from(ordered.len())
                        .map_err(|_| SpoolError::Limit)?
                        .to_le_bytes(),
                );
                let at = self.append(&head)?;
                for (key, node) in ordered {
                    self.append(
                        &u64::try_from(key.len())
                            .map_err(|_| SpoolError::Limit)?
                            .to_le_bytes(),
                    )?;
                    self.append(key.as_bytes())?;
                    self.append(&node.to_le_bytes())?;
                }
                at
            }
        };
        drop(parent.frame);
        drop(parent.charge);
        Ok(PreparedNode {
            index,
            truthy: true,
        })
    }
    fn header(&mut self, index: u64) -> Result<(u8, u64), TapeError> {
        let mut h = [0u8; 9];
        self.read_at(index, &mut h)?;
        Ok((
            h[0],
            u64::from_le_bytes(
                h[1..]
                    .try_into()
                    .map_err(|_| TapeError::Structure("invalid record"))?,
            ),
        ))
    }
    pub(crate) fn array_cursor(&mut self, index: u64) -> Result<ArrayCursor, TapeError> {
        let (tag, count) = self.header(index)?;
        if tag != 5 {
            return Err(TapeError::Structure("not array"));
        }
        let mut b = [0u8; 8];
        self.read_at(index + 9, &mut b)?;
        let chunks = u64::from_le_bytes(b);
        let size = usize::try_from(chunks).map_err(|_| SpoolError::Limit)?;
        let bytes = size
            .checked_mul(std::mem::size_of::<u64>())
            .and_then(|bytes| bytes.checked_add(CHUNK * 8))
            .ok_or(SpoolError::Limit)?;
        let mut charge = self.arena.memory_charge();
        charge.grow(bytes)?;
        let mut offsets = Vec::new();
        offsets
            .try_reserve_exact(size)
            .map_err(|_| SpoolError::MemoryLimit)?;
        let mut pos = index + 17;
        let mut start = 0usize;
        let mut buffer = [0u8; CHUNK * 8];
        while start < size {
            let items = (size - start).min(CHUNK);
            let length = items * 8;
            self.read_at(pos, &mut buffer[..length])?;
            let (records, _) = buffer[..length].as_chunks::<8>();
            for raw in records {
                offsets.push(u64::from_le_bytes(*raw));
            }
            start += items;
            pos += length as u64;
        }
        Ok(ArrayCursor {
            count,
            offsets,
            cached_chunk: None,
            cache: [0; CHUNK * 8],
            _charge: charge,
        })
    }
    pub(crate) fn array_cursor_child(
        &mut self,
        cursor: &mut ArrayCursor,
        position: u64,
    ) -> Result<u64, TapeError> {
        if position >= cursor.count {
            return Err(TapeError::Structure("array index invalid"));
        }
        let chunk_index =
            usize::try_from(position / CHUNK as u64).map_err(|_| SpoolError::Limit)?;
        let chunk = *cursor
            .offsets
            .get(chunk_index)
            .ok_or(TapeError::Structure("array chunk invalid"))?;
        if cursor.cached_chunk != Some(chunk_index) {
            let chunk_start =
                u64::try_from(chunk_index).map_err(|_| SpoolError::Limit)? * CHUNK as u64;
            let items = usize::try_from((cursor.count - chunk_start).min(CHUNK as u64))
                .map_err(|_| SpoolError::Limit)?;
            self.read_at(chunk + 9, &mut cursor.cache[..items * 8])?;
            cursor.cached_chunk = Some(chunk_index);
        }
        let within = usize::try_from(position % CHUNK as u64).map_err(|_| SpoolError::Limit)? * 8;
        Ok(u64::from_le_bytes(
            cursor.cache[within..within + 8]
                .try_into()
                .map_err(|_| TapeError::Structure("invalid array child"))?,
        ))
    }
    pub(crate) fn ordered_object_children(
        &mut self,
        index: u64,
        schema: &RowSchema,
        children: &mut [u64],
    ) -> Result<bool, TapeError> {
        let (tag, count) = self.header(index)?;
        if tag != 6
            || usize::try_from(count).map_err(|_| SpoolError::Limit)? != schema.fields.len()
            || children.len() < schema.fields.len()
        {
            return Ok(false);
        }

        let mut pos = index.checked_add(9).ok_or(SpoolError::Limit)?;
        for (field, child_slot) in schema.fields.iter().zip(children.iter_mut()) {
            let mut raw_length = [0u8; 8];
            self.read_at(pos, &mut raw_length)?;
            pos = pos.checked_add(8).ok_or(SpoolError::Limit)?;
            let length = u64::from_le_bytes(raw_length);
            if length != u64::try_from(field.key.len()).map_err(|_| SpoolError::Limit)? {
                return Ok(false);
            }
            let key_end = pos.checked_add(length).ok_or(SpoolError::Limit)?;
            if let Store::Memory(bytes) = &self.store {
                let start = usize::try_from(pos).map_err(|_| SpoolError::Limit)?;
                let end = usize::try_from(key_end).map_err(|_| SpoolError::Limit)?;
                let key = bytes
                    .get(start..end)
                    .ok_or(TapeError::Structure("record range invalid"))?;
                if key != field.key.as_bytes() {
                    return Ok(false);
                }
            } else {
                if self.scratch.len() < CHUNK {
                    let previous = self.scratch.capacity();
                    if previous < CHUNK {
                        self.scratch_charge.grow(CHUNK - previous)?;
                        if self
                            .scratch
                            .try_reserve_exact(CHUNK - self.scratch.len())
                            .is_err()
                        {
                            self.scratch_charge.shrink(CHUNK - previous);
                            return Err(SpoolError::MemoryLimit.into());
                        }
                        let actual = self.scratch.capacity().saturating_sub(previous);
                        if actual < CHUNK - previous {
                            self.scratch_charge.shrink(CHUNK - previous - actual);
                        } else if actual > CHUNK - previous {
                            self.scratch_charge.grow(actual - (CHUNK - previous))?;
                        }
                    }
                    self.scratch.resize(CHUNK, 0);
                }
                let mut scratch = std::mem::take(&mut self.scratch);
                let comparison = (|| -> Result<bool, TapeError> {
                    let mut compared = 0u64;
                    while compared < length {
                        let amount = usize::try_from((length - compared).min(CHUNK as u64))
                            .map_err(|_| SpoolError::Limit)?;
                        let at = pos.checked_add(compared).ok_or(SpoolError::Limit)?;
                        self.read_at(at, &mut scratch[..amount])?;
                        let start = usize::try_from(compared).map_err(|_| SpoolError::Limit)?;
                        let end = start.checked_add(amount).ok_or(SpoolError::Limit)?;
                        let expected = field
                            .key
                            .as_bytes()
                            .get(start..end)
                            .ok_or(TapeError::Structure("record range invalid"))?;
                        if scratch[..amount] != *expected {
                            return Ok(false);
                        }
                        compared = compared
                            .checked_add(u64::try_from(amount).map_err(|_| SpoolError::Limit)?)
                            .ok_or(SpoolError::Limit)?;
                    }
                    Ok(true)
                })();
                self.scratch = scratch;
                if !comparison? {
                    return Ok(false);
                }
            }
            pos = key_end;
            let mut raw_child = [0u8; 8];
            self.read_at(pos, &mut raw_child)?;
            pos = pos.checked_add(8).ok_or(SpoolError::Limit)?;
            *child_slot = u64::from_le_bytes(raw_child);
        }
        Ok(true)
    }
    fn object_cursor(&self, index: u64, count: u64) -> Result<ObjectCursor, TapeError> {
        let mut charge = self.arena.memory_charge();
        charge.grow(
            std::mem::size_of::<ObjectCursor>()
                + std::mem::size_of::<Option<ObjectEntry>>()
                + std::mem::size_of::<[u8; 8]>(),
        )?;
        let start = index.checked_add(9).ok_or(SpoolError::Limit)?;
        Ok(ObjectCursor {
            count,
            start,
            next: start,
            remaining: count,
            _charge: charge,
        })
    }
    fn object_cursor_next(
        &mut self,
        cursor: &mut ObjectCursor,
    ) -> Result<Option<ObjectEntry>, TapeError> {
        if cursor.remaining == 0 {
            return Ok(None);
        }
        let mut raw = [0u8; 8];
        self.read_at(cursor.next, &mut raw)?;
        let key_bytes = u64::from_le_bytes(raw);
        let key_at = cursor.next.checked_add(8).ok_or(SpoolError::Limit)?;
        let child_at = key_at.checked_add(key_bytes).ok_or(SpoolError::Limit)?;
        self.read_at(child_at, &mut raw)?;
        let child = u64::from_le_bytes(raw);
        cursor.next = child_at.checked_add(8).ok_or(SpoolError::Limit)?;
        cursor.remaining -= 1;
        Ok(Some(ObjectEntry {
            key_at,
            key_bytes,
            child,
        }))
    }
    fn write_object_entry_key<W: Write>(
        &mut self,
        entry: &ObjectEntry,
        out: &mut W,
        palette: Option<&ColorPalette>,
    ) -> Result<(), TapeError> {
        let length = usize::try_from(entry.key_bytes).map_err(|_| SpoolError::Limit)?;
        if let Store::Memory(bytes) = &self.store {
            let start = usize::try_from(entry.key_at).map_err(|_| SpoolError::Limit)?;
            let end = start.checked_add(length).ok_or(SpoolError::Limit)?;
            let key = bytes
                .get(start..end)
                .ok_or(TapeError::Structure("record range invalid"))?;
            let key =
                std::str::from_utf8(key).map_err(|_| TapeError::Structure("invalid key utf8"))?;
            writer::write_key(out, key, palette)?;
        } else {
            let mut charge = self.arena.memory_charge();
            charge.grow(length.saturating_add(std::mem::size_of::<String>()))?;
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(length)
                .map_err(|_| SpoolError::MemoryLimit)?;
            charge.grow(bytes.capacity().saturating_sub(length))?;
            bytes.resize(length, 0);
            self.read_at(entry.key_at, &mut bytes)?;
            let key =
                String::from_utf8(bytes).map_err(|_| TapeError::Structure("invalid key utf8"))?;
            writer::write_key(out, &key, palette)?;
        }
        Ok(())
    }
    fn object_info(&mut self, index: u64) -> Result<ObjectFields, TapeError> {
        let (t, n) = self.header(index)?;
        if t != 6 {
            return Err(TapeError::Structure("not object"));
        }
        let size = usize::try_from(n).map_err(|_| SpoolError::Limit)?;
        let mut charge = self.arena.memory_charge();
        charge.grow(size.saturating_mul(std::mem::size_of::<(String, u64)>()))?;
        let mut pos = index + 9;
        let mut out = Vec::with_capacity(size);
        for _ in 0..n {
            let mut b = [0; 8];
            self.read_at(pos, &mut b)?;
            pos += 8;
            let len = usize::try_from(u64::from_le_bytes(b)).map_err(|_| SpoolError::Limit)?;
            charge.grow(len)?;
            let mut k = vec![0; len];
            self.read_at(pos, &mut k)?;
            pos += len as u64;
            let mut child = [0; 8];
            self.read_at(pos, &mut child)?;
            pos += 8;
            out.push((
                String::from_utf8(k).map_err(|_| TapeError::Structure("invalid key utf8"))?,
                u64::from_le_bytes(child),
            ));
        }
        Ok(ObjectFields {
            fields: out,
            _charge: charge,
        })
    }
    fn scalar_render<W: Write>(
        &mut self,
        index: u64,
        out: &mut W,
        cfg: WriterConfig,
        palette: Option<&ColorPalette>,
        context: ScalarContext,
    ) -> Result<(), TapeError> {
        let (tag, len) = self.header(index)?;
        if tag > 4 {
            return Err(TapeError::Structure("not scalar"));
        }
        let mut m = [0];
        if tag == 1 {
            self.read_at(index + 9, &mut m)?;
        }
        if self.scratch.is_empty() && matches!(tag, 1 | 2) {
            self.scratch_charge.grow(4096)?;
            self.scratch.resize(4096, 0);
        }
        let mut scratch = std::mem::take(&mut self.scratch);
        let result = tape_scalar::write_scalar_range(
            &mut RangeReader {
                tape: self,
                at: index + SCALAR_HEADER as u64,
                left: len,
            },
            &mut scratch,
            out,
            tape_scalar::ScalarRenderOptions {
                tag,
                length: len,
                quote_mask: m[0],
                config: cfg,
                context,
                palette,
            },
        );
        self.scratch = scratch;
        result?;
        Ok(())
    }
}
impl ArrayCursor {
    pub(crate) fn count(&self) -> u64 {
        self.count
    }
}
struct RangeReader<'a> {
    tape: &'a mut PreparedTape,
    at: u64,
    left: u64,
}
impl Read for RangeReader<'_> {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        if self.left == 0 {
            return Ok(0);
        }
        let n = b
            .len()
            .min(usize::try_from(self.left).unwrap_or(usize::MAX));
        self.tape
            .read_at(self.at, &mut b[..n])
            .map_err(|error| io::Error::other(error.to_string()))?;
        self.at += n as u64;
        self.left -= n as u64;
        Ok(n)
    }
}
impl PreparedNode {
    pub(crate) fn truthy(&self) -> bool {
        self.truthy
    }
}
