//! Ordered, loss-aware format adapters and independent per-source probing.

use std::{
    fmt,
    io::{self, BufRead, BufReader, Cursor, Read},
    sync::Arc,
};

use serde::{
    Deserialize,
    de::{self, MapAccess, SeqAccess, Visitor},
};
use tq_core::{
    JsonInput, JsonInputError, JsonInputOptions, JsonLimit, Number, Object, SourceId, Value,
};
use tq_toon::{DecoderConfig, decode_to_value};

use crate::json5_input::{PreprocessError, preprocess};
use crate::{Document, DocumentSource, FormatError, InputFormat, VecDeque};

/// Structured decode controls shared by CLI sources.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodeOptions {
    /// Explicit parser or best-effort detection.
    pub format: InputFormat,
    /// Maximum bytes accepted for a document-at-a-time source.
    pub maximum_source_bytes: usize,
    /// Maximum structured nesting depth.
    pub maximum_depth: usize,
    /// Maximum bytes in a string, key, or numeric token.
    pub maximum_token_bytes: usize,
    /// Maximum bytes in one physical JSON Lines record.
    pub maximum_line_bytes: usize,
    /// TOON decoder controls.
    pub toon: DecoderConfig,
}

/// Observable bounded auto-detection decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProbeReport {
    /// Parser selected at the commitment point.
    pub selected: InputFormat,
    /// Prefix bytes inspected, bounded by the configured lookahead.
    pub lookahead_bytes: usize,
    /// Byte offset after which faildown is no longer permitted.
    pub commitment_bytes: usize,
    /// Earlier candidate rejections in probe order.
    pub rejections: Vec<(InputFormat, String)>,
}

/// Reader that replays the bounded detection prefix before continuing with the
/// untouched source.
#[derive(Debug)]
pub struct ReplayReader<R> {
    prefix: Cursor<Vec<u8>>,
    reader: R,
}

impl<R: Read> Read for ReplayReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let replayed = self.prefix.read(buffer)?;
        if replayed == 0 {
            self.reader.read(buffer)
        } else {
            Ok(replayed)
        }
    }
}

impl Default for DecodeOptions {
    fn default() -> Self {
        Self {
            format: InputFormat::Auto,
            maximum_source_bytes: 2 * 1024 * 1024 * 1024,
            maximum_depth: 256,
            maximum_token_bytes: 8 * 1024 * 1024,
            maximum_line_bytes: 16 * 1024 * 1024,
            toon: DecoderConfig::default(),
        }
    }
}

/// In-memory pull source used by document-at-a-time adapters.
#[derive(Debug, Default)]
pub struct VecDocumentSource {
    documents: VecDeque<Document>,
}

impl VecDocumentSource {
    /// Wraps documents in pull order.
    #[must_use]
    pub fn new(documents: Vec<Document>) -> Self {
        Self {
            documents: documents.into(),
        }
    }
}

impl DocumentSource for VecDocumentSource {
    fn next_document(&mut self) -> Result<Option<Document>, FormatError> {
        Ok(self.documents.pop_front())
    }
}

/// Incremental source for whitespace-separated JSON values.
pub struct JsonDocumentSource<R: Read> {
    reader: JsonInput<R>,
    identity: String,
    index: u64,
}

impl<R: Read> JsonDocumentSource<R> {
    /// Creates a pull source that parses only the next requested JSON value.
    #[must_use]
    pub fn new(reader: R, identity: impl Into<String>) -> Self {
        Self {
            reader: JsonInput::new(reader, JsonInputOptions::default()),
            identity: identity.into(),
            index: 0,
        }
    }

    /// Creates a pull source with explicit JSON resource limits.
    #[must_use]
    pub fn with_options(reader: R, identity: impl Into<String>, options: JsonInputOptions) -> Self {
        Self {
            reader: JsonInput::new(reader, options),
            identity: identity.into(),
            index: 0,
        }
    }
}

impl<R: Read> DocumentSource for JsonDocumentSource<R> {
    fn next_document(&mut self) -> Result<Option<Document>, FormatError> {
        let mut checkpoint = || Ok(());
        let Some(value) = self
            .reader
            .next_value(&mut checkpoint)
            .map_err(format_json_input_error)?
        else {
            return Ok(None);
        };
        let index = self.index;
        self.index = self.index.saturating_add(1);
        let line_number = self.reader.position().line as u64;
        Ok(Some(Document {
            value,
            identity: self.identity.clone(),
            format: InputFormat::Json,
            index,
            line_number,
        }))
    }
}

fn format_json_input_error(error: JsonInputError) -> FormatError {
    match error {
        JsonInputError::Io(error) => FormatError::Io(error),
        JsonInputError::Limit { limit, .. } => FormatError::Resource(match limit {
            tq_core::JsonLimit::TokenBytes => "token-bytes",
            tq_core::JsonLimit::Depth => "depth",
        }),
        JsonInputError::Consumer(error) => FormatError::Parse {
            format: InputFormat::Json,
            message: error.to_string(),
        },
        error @ JsonInputError::Syntax { .. } => FormatError::Parse {
            format: InputFormat::Json,
            message: error.to_string(),
        },
    }
}

fn parse_json_record(bytes: &[u8], options: JsonInputOptions) -> Result<Value, JsonInputError> {
    let mut reader = JsonInput::new(Cursor::new(bytes), options);
    let mut checkpoint = || Ok(());
    let value = reader.next_value(&mut checkpoint).and_then(|value| {
        value.ok_or_else(|| JsonInputError::Syntax {
            position: reader.position(),
            message: "expected JSON value".into(),
        })
    })?;
    reader.check_end(&mut checkpoint)?;
    Ok(value)
}

/// Incremental source for jq's Record Separator framed JSON sequence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum JsonSequenceScan {
    Empty,
    NeedMore,
    Complete { start: usize, end: usize },
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum JsonSequenceErrorKind {
    Generic,
    Literal,
    Numeric,
}

#[derive(Clone, Copy, Debug)]
enum JsonSequenceToken {
    Structured {
        start: usize,
        cursor: usize,
        depth: usize,
        in_string: bool,
        escaped: bool,
        string_raw_bytes: usize,
        scalar_start: Option<usize>,
    },
    String {
        start: usize,
        cursor: usize,
        escaped: bool,
        raw_bytes: usize,
    },
    Number {
        start: usize,
        cursor: usize,
    },
    Literal {
        start: usize,
        cursor: usize,
        expected: &'static [u8],
    },
    Word {
        start: usize,
        cursor: usize,
    },
}

const fn is_json_sequence_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r')
}

const fn is_json_sequence_token_delimiter(byte: u8) -> bool {
    is_json_sequence_whitespace(byte)
        || matches!(byte, b'[' | b'{' | b'}' | b']' | b',' | b':' | b'"')
}

fn is_keyword_prefix(token: &[u8]) -> bool {
    [b"true".as_slice(), b"false", b"null"]
        .iter()
        .any(|keyword| keyword.starts_with(token))
}

/// Pull-oriented RS source. It scans only until one complete JSON token is
/// available, then decodes that slice once. This preserves finite consumers
/// on live pipes while avoiding reparsing an ever-growing record.
#[allow(
    clippy::struct_excessive_bools,
    reason = "the source tracks independent JSON-sequence and record boundaries"
)]
pub struct JsonSequenceDocumentSource<R: BufRead> {
    reader: R,
    identity: String,
    json_options: JsonInputOptions,
    started: bool,
    prefix_seen: bool,
    eof: bool,
    record_boundary: bool,
    physical_line: u64,
    record_line: u64,
    record_column: u64,
    record_index: u64,
    document_index: u64,
    record_document_count: usize,
    record: Vec<u8>,
    scan_offset: usize,
    scan_token: Option<JsonSequenceToken>,
    scan_error: Option<(usize, JsonSequenceErrorKind)>,
    scan_resource: Option<JsonLimit>,
    record_newlines: Vec<usize>,
    last_byte: Option<u8>,
    terminal_error_emitted: bool,
    terminal_resource_error: bool,
}

impl<R: BufRead> JsonSequenceDocumentSource<R> {
    /// Creates a pull source that resynchronizes at each ASCII RS marker.
    #[must_use]
    pub fn new(reader: R, identity: impl Into<String>) -> Self {
        Self::with_options(reader, identity, JsonInputOptions::default())
    }

    /// Creates a pull source with explicit JSON resource limits.
    #[must_use]
    pub fn with_options(
        reader: R,
        identity: impl Into<String>,
        json_options: JsonInputOptions,
    ) -> Self {
        Self {
            reader,
            identity: identity.into(),
            json_options,
            started: false,
            prefix_seen: false,
            eof: false,
            record_boundary: false,
            physical_line: 1,
            record_line: 1,
            record_column: 1,
            record_index: 0,
            document_index: 0,
            record_document_count: 0,
            record: Vec::new(),
            scan_offset: 0,
            scan_token: None,
            scan_error: None,
            scan_resource: None,
            record_newlines: Vec::new(),
            last_byte: None,
            terminal_error_emitted: false,
            terminal_resource_error: false,
        }
    }

    fn reset_record(&mut self) {
        self.record.clear();
        self.scan_offset = 0;
        self.scan_token = None;
        self.scan_error = None;
        self.scan_resource = None;
        self.record_newlines.clear();
        self.last_byte = None;
        self.record_document_count = 0;
        self.record_line = self.physical_line;
        self.record_column = 1;
        self.record_boundary = false;
    }

    fn append_record_bytes(&mut self, bytes: &[u8]) {
        let offset = self.record.len();
        for (index, byte) in bytes.iter().enumerate() {
            if *byte == b'\n' {
                self.record_newlines.push(offset + index + 1);
            }
        }
        self.record.extend_from_slice(bytes);
        if let Some(byte) = bytes.last().copied() {
            self.last_byte = Some(byte);
        }
        self.physical_line = self
            .physical_line
            .saturating_add(memchr::memchr_iter(b'\n', bytes).count() as u64);
    }

    fn read_more(&mut self) -> Result<(), FormatError> {
        if self.record_boundary || self.eof {
            return Ok(());
        }
        let byte = {
            let available = self.reader.fill_buf().map_err(FormatError::Io)?;
            let Some(byte) = available.first().copied() else {
                self.eof = true;
                self.record_boundary = true;
                return Ok(());
            };
            byte
        };
        if byte == 0x1e {
            self.record_boundary = true;
        } else {
            self.reader.consume(1);
            self.append_record_bytes(&[byte]);
        }
        Ok(())
    }

    fn start_record(&mut self) -> Result<bool, FormatError> {
        if self.eof {
            return Ok(false);
        }
        if self.record_boundary {
            let available = self.reader.fill_buf().map_err(FormatError::Io)?;
            if available.first() == Some(&0x1e) {
                self.reader.consume(1);
                self.record_index = self.record_index.saturating_add(1);
                self.reset_record();
                return Ok(true);
            }
            self.eof = true;
            return Ok(false);
        }
        if self.started {
            return Ok(true);
        }
        loop {
            let byte = {
                let available = self.reader.fill_buf().map_err(FormatError::Io)?;
                let Some(byte) = available.first().copied() else {
                    self.eof = true;
                    return Ok(false);
                };
                byte
            };
            self.reader.consume(1);
            self.prefix_seen = true;
            if byte == 0x1e {
                self.started = true;
                self.record_index = self.record_index.saturating_add(1);
                self.reset_record();
                return Ok(true);
            }
            if byte == b'\n' {
                self.physical_line = self.physical_line.saturating_add(1);
            }
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "JSON-sequence scanning keeps token, framing, and recovery state together"
    )]
    fn scan_value(&mut self) -> JsonSequenceScan {
        if self.scan_token.is_none() {
            let mut start = self.scan_offset;
            while start < self.record.len() && is_json_sequence_whitespace(self.record[start]) {
                start += 1;
            }
            self.scan_offset = start;
            if start == self.record.len() {
                return if self.record_boundary || self.eof {
                    JsonSequenceScan::Empty
                } else {
                    self.compact_record();
                    JsonSequenceScan::NeedMore
                };
            }
            self.scan_token = Some(match self.record[start] {
                b'{' | b'[' => JsonSequenceToken::Structured {
                    start,
                    cursor: start,
                    depth: 0,
                    in_string: false,
                    escaped: false,
                    string_raw_bytes: 0,
                    scalar_start: None,
                },
                b'"' => JsonSequenceToken::String {
                    start,
                    cursor: start + 1,
                    escaped: false,
                    raw_bytes: 1,
                },
                b'0'..=b'9' => JsonSequenceToken::Number {
                    start,
                    cursor: start,
                },
                b't' => JsonSequenceToken::Literal {
                    start,
                    cursor: start,
                    expected: b"true",
                },
                b'f' => JsonSequenceToken::Literal {
                    start,
                    cursor: start,
                    expected: b"false",
                },
                b'n' | b's' | b'S' | b'.' | b'-' | b'+' | b'N' | b'I' | b'i' => {
                    JsonSequenceToken::Word {
                        start,
                        cursor: start,
                    }
                }
                _ => {
                    // jq's sequence parser reports an unknown token as a
                    // numeric literal unless it starts a reserved literal.
                    // Keep the offset at the token start so recovery reports
                    // the RS-delimited line rather than a later byte.
                    self.scan_error = Some((
                        start,
                        if matches!(self.record[start], b'f' | b't') {
                            JsonSequenceErrorKind::Literal
                        } else {
                            JsonSequenceErrorKind::Numeric
                        },
                    ));
                    return JsonSequenceScan::Invalid;
                }
            });
        }

        let token = self
            .scan_token
            .expect("a JSON sequence token is initialized above");
        match token {
            JsonSequenceToken::Structured {
                start,
                mut cursor,
                mut depth,
                mut in_string,
                mut escaped,
                mut string_raw_bytes,
                mut scalar_start,
            } => {
                while cursor < self.record.len() {
                    let byte = self.record[cursor];
                    cursor += 1;
                    if in_string {
                        string_raw_bytes = string_raw_bytes.saturating_add(1);
                        if string_raw_bytes
                            > self
                                .json_options
                                .maximum_token_bytes
                                .saturating_mul(6)
                                .saturating_add(2)
                        {
                            self.scan_resource = Some(JsonLimit::TokenBytes);
                            self.scan_token = None;
                            return JsonSequenceScan::Invalid;
                        }
                        if escaped {
                            escaped = false;
                        } else if byte == b'\\' {
                            escaped = true;
                        } else if byte == b'"' {
                            in_string = false;
                            string_raw_bytes = 0;
                        }
                        continue;
                    }
                    match byte {
                        b'"' => {
                            scalar_start = None;
                            in_string = true;
                            string_raw_bytes = 1;
                        }
                        b'{' | b'[' => {
                            scalar_start = None;
                            if depth >= self.json_options.maximum_depth {
                                self.scan_resource = Some(JsonLimit::Depth);
                                self.scan_token = None;
                                return JsonSequenceScan::Invalid;
                            }
                            depth = depth.saturating_add(1);
                        }
                        b'}' | b']' => {
                            scalar_start = None;
                            if depth == 0 {
                                self.scan_error = Some((
                                    cursor.saturating_sub(1),
                                    JsonSequenceErrorKind::Generic,
                                ));
                                self.scan_token = None;
                                return JsonSequenceScan::Invalid;
                            }
                            depth -= 1;
                            if depth == 0 {
                                self.scan_token = None;
                                return JsonSequenceScan::Complete { start, end: cursor };
                            }
                        }
                        byte if is_json_sequence_whitespace(byte)
                            || matches!(byte, b',' | b':') =>
                        {
                            scalar_start = None;
                        }
                        _ => {
                            let token_start = scalar_start.get_or_insert(cursor - 1);
                            let token = &self.record[*token_start..cursor];
                            if token.len() > self.json_options.maximum_token_bytes
                                && !is_keyword_prefix(token)
                            {
                                self.scan_resource = Some(JsonLimit::TokenBytes);
                                self.scan_token = None;
                                return JsonSequenceScan::Invalid;
                            }
                        }
                    }
                }
                self.scan_token = Some(JsonSequenceToken::Structured {
                    start,
                    cursor,
                    depth,
                    in_string,
                    escaped,
                    string_raw_bytes,
                    scalar_start,
                });
                if self.record_boundary || self.eof {
                    self.scan_error = Some((self.record.len(), JsonSequenceErrorKind::Generic));
                    self.scan_token = None;
                    JsonSequenceScan::Invalid
                } else {
                    JsonSequenceScan::NeedMore
                }
            }
            JsonSequenceToken::String {
                start,
                mut cursor,
                mut escaped,
                mut raw_bytes,
            } => {
                while cursor < self.record.len() {
                    let byte = self.record[cursor];
                    cursor += 1;
                    raw_bytes = raw_bytes.saturating_add(1);
                    if raw_bytes
                        > self
                            .json_options
                            .maximum_token_bytes
                            .saturating_mul(6)
                            .saturating_add(2)
                    {
                        self.scan_resource = Some(JsonLimit::TokenBytes);
                        self.scan_token = None;
                        return JsonSequenceScan::Invalid;
                    }
                    if escaped {
                        escaped = false;
                    } else if byte == b'\\' {
                        escaped = true;
                    } else if byte == b'"' {
                        self.scan_token = None;
                        return JsonSequenceScan::Complete { start, end: cursor };
                    }
                }
                self.scan_token = Some(JsonSequenceToken::String {
                    start,
                    cursor,
                    escaped,
                    raw_bytes,
                });
                if self.record_boundary || self.eof {
                    self.scan_error = Some((self.record.len(), JsonSequenceErrorKind::Generic));
                    self.scan_token = None;
                    JsonSequenceScan::Invalid
                } else {
                    JsonSequenceScan::NeedMore
                }
            }
            JsonSequenceToken::Number { start, mut cursor } => {
                while cursor < self.record.len()
                    && matches!(
                        self.record[cursor],
                        b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E'
                    )
                {
                    cursor += 1;
                    if cursor.saturating_sub(start) > self.json_options.maximum_token_bytes {
                        self.scan_resource = Some(JsonLimit::TokenBytes);
                        self.scan_token = None;
                        return JsonSequenceScan::Invalid;
                    }
                }
                if cursor == self.record.len() {
                    self.scan_token = Some(JsonSequenceToken::Number { start, cursor });
                    if self.record_boundary || self.eof {
                        self.scan_token = None;
                        JsonSequenceScan::Complete { start, end: cursor }
                    } else {
                        JsonSequenceScan::NeedMore
                    }
                } else if is_json_sequence_token_delimiter(self.record[cursor]) {
                    self.scan_token = None;
                    JsonSequenceScan::Complete { start, end: cursor }
                } else {
                    self.scan_error = Some((cursor, JsonSequenceErrorKind::Numeric));
                    self.scan_token = None;
                    JsonSequenceScan::Invalid
                }
            }
            JsonSequenceToken::Literal {
                start,
                mut cursor,
                expected,
            } => {
                while cursor < self.record.len() && cursor - start < expected.len() {
                    if self.record[cursor] != expected[cursor - start] {
                        self.scan_error = Some((cursor, JsonSequenceErrorKind::Literal));
                        self.scan_token = None;
                        return JsonSequenceScan::Invalid;
                    }
                    cursor += 1;
                }
                if cursor - start < expected.len() {
                    self.scan_token = Some(JsonSequenceToken::Literal {
                        start,
                        cursor,
                        expected,
                    });
                    if self.record_boundary || self.eof {
                        self.scan_error = Some((self.record.len(), JsonSequenceErrorKind::Literal));
                        self.scan_token = None;
                        JsonSequenceScan::Invalid
                    } else {
                        JsonSequenceScan::NeedMore
                    }
                } else if cursor == self.record.len() {
                    self.scan_token = Some(JsonSequenceToken::Literal {
                        start,
                        cursor,
                        expected,
                    });
                    if self.record_boundary || self.eof {
                        self.scan_token = None;
                        JsonSequenceScan::Complete { start, end: cursor }
                    } else {
                        JsonSequenceScan::NeedMore
                    }
                } else if is_json_sequence_token_delimiter(self.record[cursor]) {
                    self.scan_token = None;
                    JsonSequenceScan::Complete { start, end: cursor }
                } else {
                    self.scan_error = Some((cursor, JsonSequenceErrorKind::Literal));
                    self.scan_token = None;
                    JsonSequenceScan::Invalid
                }
            }
            JsonSequenceToken::Word { start, mut cursor } => {
                while cursor < self.record.len()
                    && !is_json_sequence_token_delimiter(self.record[cursor])
                {
                    cursor += 1;
                    let token = &self.record[start..cursor];
                    if token.len() > self.json_options.maximum_token_bytes
                        && !is_keyword_prefix(token)
                    {
                        self.scan_resource = Some(JsonLimit::TokenBytes);
                        self.scan_token = None;
                        return JsonSequenceScan::Invalid;
                    }
                }
                if cursor == self.record.len() {
                    self.scan_token = Some(JsonSequenceToken::Word { start, cursor });
                    if self.record_boundary || self.eof {
                        self.scan_token = None;
                        JsonSequenceScan::Complete { start, end: cursor }
                    } else {
                        JsonSequenceScan::NeedMore
                    }
                } else {
                    self.scan_token = None;
                    JsonSequenceScan::Complete { start, end: cursor }
                }
            }
        }
    }

    fn line_number(&self, end: usize) -> u64 {
        self.record_line.saturating_add(
            self.record_newlines
                .iter()
                .take_while(|newline| **newline < end)
                .count() as u64,
        )
    }

    fn jq_line_column(&self, offset: usize, boundary_invalid: bool) -> (u64, u64) {
        let mut end = offset.min(self.record.len());
        let mut zero_column = false;
        if boundary_invalid {
            if matches!(self.record.get(end), Some(b'\x0b' | b'\x0c')) {
                end = end.saturating_add(1).min(self.record.len());
                zero_column = true;
            } else if let Some(relative) = self.record[end..]
                .iter()
                .position(|byte| matches!(*byte, b'\n' | b'\r'))
            {
                end = end.saturating_add(relative).saturating_add(1);
                zero_column = true;
            }
        }
        let mut line = self.record_line;
        let mut column = self.record_column.saturating_sub(1);
        for byte in &self.record[..end] {
            if matches!(*byte, b'\n' | b'\r' | b'\x0b' | b'\x0c') {
                line = line.saturating_add(1);
                column = 0;
            } else {
                column = column.saturating_add(1);
            }
        }
        if !zero_column {
            column = column.saturating_add(1);
        }
        (line, column)
    }

    fn jq_scan_error(&self, offset: usize, kind: JsonSequenceErrorKind) -> String {
        let (line, column) = self.jq_line_column(offset, true);
        let eof = if self.eof { " at EOF" } else { "" };
        let label = match kind {
            JsonSequenceErrorKind::Literal => "Invalid literal",
            JsonSequenceErrorKind::Numeric => "Invalid numeric literal",
            JsonSequenceErrorKind::Generic => "Invalid JSON value",
        };
        if kind == JsonSequenceErrorKind::Generic {
            format!("record {}: {label}", self.record_index.saturating_sub(1))
        } else {
            format!("{label}{eof} at line {line}, column {column} (need RS to resync)")
        }
    }

    fn jq_structured_error(&self, start: usize, end: usize, error: &str) -> String {
        let detail = error.rsplit_once(": ").map_or(error, |(_, detail)| detail);
        let (line, column) = self.jq_line_column(end, false);
        if detail.starts_with("expected ':' after object key") {
            return format!(
                "Expected separator between values at line {line}, column {column} (need RS to resync)"
            );
        }
        let value = &self.record[start.min(self.record.len())..end.min(self.record.len())];
        if detail.starts_with("expected JSON value") || detail.starts_with("expected object key") {
            match value.last() {
                Some(b']') if value.get(value.len().saturating_sub(2)) == Some(&b',') => {
                    return format!(
                        "Expected another array element at line {line}, column {column} (need RS to resync)"
                    );
                }
                Some(b'}') if value.get(value.len().saturating_sub(2)) == Some(&b',') => {
                    return format!(
                        "Expected another key-value pair at line {line}, column {column} (need RS to resync)"
                    );
                }
                Some(b'}') if value.get(value.len().saturating_sub(2)) == Some(&b':') => {
                    return format!(
                        "Unmatched '}}' at line {line}, column {column} (need RS to resync)"
                    );
                }
                _ => {}
            }
            return format!(
                "Invalid numeric literal at line {line}, column {column} (need RS to resync)"
            );
        }
        format!("Invalid JSON value at line {line}, column {column} (need RS to resync)")
    }

    fn compact_record(&mut self) {
        const COMPACTION_THRESHOLD: usize = 64 * 1024;
        let consumed = self.scan_offset;
        if consumed < COMPACTION_THRESHOLD || consumed.saturating_mul(2) < self.record.len() {
            return;
        }
        let consumed_newlines = self
            .record_newlines
            .iter()
            .take_while(|newline| **newline <= consumed)
            .count() as u64;
        let consumed_column = self
            .record_newlines
            .iter()
            .take_while(|newline| **newline <= consumed)
            .last()
            .map_or_else(
                || self.record_column.saturating_add(consumed as u64),
                |newline| consumed.saturating_sub(*newline).saturating_add(1) as u64,
            );
        self.record_line = self.record_line.saturating_add(consumed_newlines);
        self.record_column = consumed_column;
        self.record.drain(..consumed);
        self.record_newlines.retain(|newline| *newline > consumed);
        for newline in &mut self.record_newlines {
            *newline -= consumed;
        }
        self.scan_offset = 0;
    }

    fn discard_record(&mut self) -> Result<(), FormatError> {
        self.record.clear();
        self.scan_offset = 0;
        self.scan_token = None;
        self.scan_error = None;
        self.record_newlines.clear();
        self.last_byte = None;
        while !self.record_boundary && !self.eof {
            let (take, found_rs) = {
                let available = self.reader.fill_buf().map_err(FormatError::Io)?;
                if available.is_empty() {
                    self.eof = true;
                    self.record_boundary = true;
                    break;
                }
                let position = available.iter().position(|byte| *byte == 0x1e);
                (position.unwrap_or(available.len()), position.is_some())
            };
            if take > 0 {
                let available = self.reader.fill_buf().map_err(FormatError::Io)?;
                self.physical_line = self
                    .physical_line
                    .saturating_add(memchr::memchr_iter(b'\n', &available[..take]).count() as u64);
                self.reader.consume(take);
            }
            if found_rs && take == 0 {
                self.record_boundary = true;
            }
        }
        Ok(())
    }

    /// Returns the next decoded result or one recoverable record error.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the source cannot be read, or a framing error
    /// when the JSON-sequence stream cannot be advanced.
    #[allow(
        clippy::too_many_lines,
        reason = "JSON-sequence recovery keeps scanner state, diagnostics, and document emission ordered"
    )]
    pub fn next_record(&mut self) -> Result<Option<Result<Document, FormatError>>, FormatError> {
        if self.terminal_resource_error {
            return Ok(None);
        }
        loop {
            let needs_next_record =
                !self.started || (self.record_boundary && self.scan_offset >= self.record.len());
            if needs_next_record && !self.start_record()? {
                if !self.started && self.prefix_seen && !self.terminal_error_emitted {
                    self.terminal_error_emitted = true;
                    return Ok(Some(Err(FormatError::Parse {
                        format: InputFormat::JsonSequence,
                        message: format!(
                            "{}: sequence input ended before the first record separator",
                            self.identity
                        ),
                    })));
                }
                return Ok(None);
            }
            let scan = self.scan_value();
            if scan == JsonSequenceScan::NeedMore {
                self.read_more()?;
                continue;
            }
            let JsonSequenceScan::Complete { start, end } = scan else {
                if matches!(scan, JsonSequenceScan::Empty) {
                    if self.record_boundary || self.eof {
                        self.scan_offset = self.record.len();
                        self.compact_record();
                        self.started = true;
                        if self.eof {
                            return Ok(None);
                        }
                        continue;
                    }
                    self.read_more()?;
                    continue;
                }
                if let Some(limit) = self.scan_resource.take() {
                    self.terminal_resource_error = true;
                    return Err(FormatError::Resource(match limit {
                        JsonLimit::TokenBytes => "token-bytes",
                        JsonLimit::Depth => "depth",
                    }));
                }
                let scan_error = self
                    .scan_error
                    .take()
                    .unwrap_or((self.scan_offset, JsonSequenceErrorKind::Generic));
                let has_position = self.record_boundary
                    || self.eof
                    || self.record[scan_error.0..]
                        .iter()
                        .any(|byte| matches!(*byte, b'\n' | b'\r' | b'\x0b' | b'\x0c'));
                let message = has_position.then(|| self.jq_scan_error(scan_error.0, scan_error.1));
                self.discard_record()?;
                self.scan_offset = self.record.len();
                return Ok(Some(Err(FormatError::Parse {
                    format: InputFormat::JsonSequence,
                    message: message.unwrap_or_else(|| match scan_error.1 {
                        JsonSequenceErrorKind::Literal => format!(
                            "Invalid literal at line {}, column 0 (need RS to resync)",
                            self.physical_line
                        ),
                        JsonSequenceErrorKind::Numeric => format!(
                            "Invalid numeric literal at line {}, column 0 (need RS to resync)",
                            self.physical_line
                        ),
                        JsonSequenceErrorKind::Generic => format!(
                            "record {}: invalid JSON value",
                            self.record_index.saturating_sub(1)
                        ),
                    }),
                })));
            };
            let value = match parse_json_record(&self.record[start..end], self.json_options) {
                Ok(value) => value,
                Err(error) => {
                    if matches!(error, JsonInputError::Limit { .. }) {
                        self.terminal_resource_error = true;
                        return Err(format_json_input_error(error));
                    }
                    let error = error.to_string();
                    let message = self.jq_structured_error(start, end, &error);
                    self.discard_record()?;
                    self.scan_offset = self.record.len();
                    self.scan_token = None;
                    return Ok(Some(Err(FormatError::Parse {
                        format: InputFormat::JsonSequence,
                        message,
                    })));
                }
            };
            let truncated_number = matches!(&value, Value::Number(_))
                && (self.record_boundary || self.eof)
                && self.record[end..]
                    .iter()
                    .all(|byte| is_json_sequence_whitespace(*byte))
                && !self.last_byte.is_some_and(is_json_sequence_whitespace);
            if truncated_number {
                let (line, mut column) = self.jq_line_column(end, false);
                // jq reports the RS delimiter column for a number that ends
                // immediately before the next record. Its EOF diagnostic
                // instead points one column after the final digit.
                if self.record_boundary
                    && !self.eof
                    && !self.record[..end].contains(&b'\n')
                    && !self.record[..end].contains(&b'\r')
                {
                    column = column.saturating_add(1);
                }
                self.discard_record()?;
                self.scan_offset = self.record.len();
                self.scan_token = None;
                let eof = if self.eof { " at EOF" } else { "" };
                return Ok(Some(Err(FormatError::Parse {
                    format: InputFormat::JsonSequence,
                    message: format!(
                        "Potentially truncated top-level numeric value{eof} at line {line}, column {column}"
                    ),
                })));
            }
            let line_number = self.line_number(end);
            self.scan_offset = end;
            self.compact_record();
            self.record_document_count = self.record_document_count.saturating_add(1);
            let document = Document {
                value,
                identity: self.identity.clone(),
                format: InputFormat::JsonSequence,
                index: self.document_index,
                line_number,
            };
            self.document_index = self.document_index.saturating_add(1);
            return Ok(Some(Ok(document)));
        }
    }
}

impl<R: BufRead> DocumentSource for JsonSequenceDocumentSource<R> {
    fn next_document(&mut self) -> Result<Option<Document>, FormatError> {
        match self.next_record()? {
            None => Ok(None),
            Some(Ok(document)) => Ok(Some(document)),
            Some(Err(error)) => Err(error),
        }
    }
}

/// Decodes one byte source using an override or bounded syntax detection.
///
/// Each call probes independently using a bounded prefix. Once probing commits,
/// later syntax failures belong to the selected format and do not restart
/// detection.
///
/// # Errors
///
/// Returns a selected-parser error, combined probe error, or resource failure.
pub fn decode_bytes(
    bytes: &[u8],
    identity: impl Into<String>,
    options: DecodeOptions,
) -> Result<Vec<Document>, FormatError> {
    if bytes.len() > options.maximum_source_bytes {
        return Err(FormatError::Resource("source-bytes"));
    }
    let identity = identity.into();
    match options.format {
        InputFormat::Toon => decode_toon(bytes, identity, options.toon),
        InputFormat::Yaml => decode_yaml(bytes, identity),
        InputFormat::Json => decode_json_with_options(bytes, identity, options),
        InputFormat::Json5 => decode_json5(bytes, identity, options),
        InputFormat::JsonLines => decode_json_lines(bytes, identity, options),
        InputFormat::ToonSequence => decode_toon_sequence(bytes, identity, options.toon),
        InputFormat::JsonSequence => decode_json_sequence_with_options(bytes, identity, options),
        InputFormat::Auto => {
            let report = probe_format(bytes, options.toon.maximum_lookahead_bytes)?;
            match report.selected {
                InputFormat::Toon => decode_toon(bytes, identity, options.toon),
                InputFormat::Yaml => decode_yaml(bytes, identity),
                InputFormat::Json => decode_json_with_options(bytes, identity, options),
                InputFormat::Auto
                | InputFormat::Json5
                | InputFormat::JsonLines
                | InputFormat::ToonSequence
                | InputFormat::JsonSequence => {
                    unreachable!("probe candidate")
                }
            }
        }
    }
}

/// Incremental JSON Lines document source with bounded physical records.
#[derive(Debug)]
pub struct JsonLinesDocumentSource<R> {
    reader: R,
    identity: String,
    options: DecodeOptions,
    physical_line: u64,
    record_index: u64,
    source_bytes: usize,
    line: Vec<u8>,
}

impl<R: BufRead> JsonLinesDocumentSource<R> {
    /// Creates a JSON Lines source over a buffered reader.
    #[must_use]
    pub fn new(reader: R, identity: impl Into<String>, options: DecodeOptions) -> Self {
        Self {
            reader,
            identity: identity.into(),
            options,
            physical_line: 0,
            record_index: 0,
            source_bytes: 0,
            line: Vec::new(),
        }
    }

    fn read_line(&mut self) -> Result<bool, FormatError> {
        self.line.clear();
        loop {
            let available = self.reader.fill_buf()?;
            if available.is_empty() {
                return Ok(!self.line.is_empty());
            }
            let newline = available.iter().position(|byte| *byte == b'\n');
            let content_bytes = newline.unwrap_or(available.len());
            if self.line.len().saturating_add(content_bytes) > self.options.maximum_line_bytes {
                return Err(FormatError::ResourceLine {
                    identity: self.identity.clone(),
                    line: self.physical_line.saturating_add(1),
                    resource: "line-bytes",
                });
            }
            let consumed = content_bytes.saturating_add(usize::from(newline.is_some()));
            let next_source_bytes = self.source_bytes.saturating_add(consumed);
            if next_source_bytes > self.options.maximum_source_bytes {
                return Err(FormatError::ResourceLine {
                    identity: self.identity.clone(),
                    line: self.physical_line.saturating_add(1),
                    resource: "source-bytes",
                });
            }
            self.line.extend_from_slice(&available[..content_bytes]);
            self.reader.consume(consumed);
            self.source_bytes = next_source_bytes;
            if newline.is_some() {
                return Ok(true);
            }
        }
    }

    /// Returns the next non-empty physical record and its one-based line.
    ///
    /// # Errors
    ///
    /// Returns bounded line or input I/O failures.
    pub fn next_record(&mut self) -> Result<Option<(Vec<u8>, u64)>, FormatError> {
        loop {
            if !self.read_line()? {
                return Ok(None);
            }
            self.physical_line = self.physical_line.saturating_add(1);
            if self.line.is_empty() {
                continue;
            }
            return Ok(Some((self.line.clone(), self.physical_line)));
        }
    }

    /// Returns the next ordered JSON Lines document.
    ///
    /// # Errors
    ///
    /// Returns bounded line, JSON syntax, numeric, token, or depth failures.
    pub fn next_document(&mut self) -> Result<Option<Document>, FormatError> {
        let Some((line, physical_line)) = self.next_record()? else {
            return Ok(None);
        };
        let value = decode_json_line(&line, &self.identity, physical_line, self.options)?;
        let index = self.record_index;
        self.record_index = self.record_index.saturating_add(1);
        Ok(Some(Document {
            value,
            identity: self.identity.clone(),
            format: InputFormat::JsonLines,
            index,
            line_number: physical_line,
        }))
    }
}

impl<R: BufRead> DocumentSource for JsonLinesDocumentSource<R> {
    fn next_document(&mut self) -> Result<Option<Document>, FormatError> {
        Self::next_document(self)
    }
}

/// Decodes strict one-value-per-line JSON into ordered documents.
///
/// # Errors
///
/// Returns line, syntax, numeric, token, or depth failures with physical line context.
pub fn decode_json_lines(
    bytes: &[u8],
    identity: impl Into<String>,
    options: DecodeOptions,
) -> Result<Vec<Document>, FormatError> {
    if bytes.len() > options.maximum_source_bytes {
        return Err(FormatError::Resource("source-bytes"));
    }
    let mut source = JsonLinesDocumentSource::new(BufReader::new(bytes), identity, options);
    let mut documents = Vec::new();
    while let Some(document) = source.next_document()? {
        documents.push(document);
    }
    Ok(documents)
}

fn decode_json_line(
    bytes: &[u8],
    identity: &str,
    physical_line: u64,
    options: DecodeOptions,
) -> Result<Value, FormatError> {
    let parser_options = JsonInputOptions {
        maximum_depth: options.maximum_depth,
        maximum_token_bytes: options.maximum_token_bytes,
    };
    let mut reader = JsonInput::new(Cursor::new(bytes), parser_options);
    let mut checkpoint = || Ok(());
    let value = reader
        .next_value(&mut checkpoint)
        .map_err(|error| match error {
            JsonInputError::Io(error) => FormatError::Io(error),
            JsonInputError::Limit { limit, .. } => FormatError::ResourceLine {
                identity: identity.to_owned(),
                line: physical_line,
                resource: match limit {
                    tq_core::JsonLimit::TokenBytes => "token-bytes",
                    tq_core::JsonLimit::Depth => "depth",
                },
            },
            error => FormatError::Parse {
                format: InputFormat::JsonLines,
                message: format!("{identity}:{physical_line}: {error}"),
            },
        })?;
    let Some(value) = value else {
        return Err(FormatError::Parse {
            format: InputFormat::JsonLines,
            message: format!("{identity}:{physical_line}: expected JSON value"),
        });
    };
    if reader.check_end(&mut checkpoint).is_err() {
        return Err(FormatError::Parse {
            format: InputFormat::JsonLines,
            message: format!("{identity}:{physical_line}: multiple JSON values"),
        });
    }
    Ok(value)
}

/// Selects a parser from a bounded prefix and records the commitment point.
///
/// # Errors
///
/// Returns combined bounded context when no candidate can safely inspect the
/// prefix, such as invalid UTF-8 shared by every structured parser.
pub fn probe_format(
    bytes: &[u8],
    maximum_lookahead_bytes: usize,
) -> Result<ProbeReport, FormatError> {
    let inspected = bytes.len().min(maximum_lookahead_bytes);
    let prefix = &bytes[..inspected];
    let text = match std::str::from_utf8(prefix) {
        Ok(text) => text,
        Err(error) if error.error_len().is_none() && inspected < bytes.len() => {
            std::str::from_utf8(&prefix[..error.valid_up_to()]).map_err(|_| FormatError::Probe {
                summary: "TOON, YAML, and JSON could not validate the lookahead prefix".to_owned(),
            })?
        }
        Err(error) => {
            return Err(FormatError::Probe {
                summary: format!(
                    "TOON: invalid UTF-8 at {}; YAML: invalid UTF-8; JSON: invalid UTF-8",
                    error.valid_up_to()
                ),
            });
        }
    };
    let trimmed = text.trim_start();
    let first_line = trimmed.lines().next().unwrap_or("");
    let toon_header = root_toon_array_header(first_line);
    let (selected, rejection) = if trimmed.starts_with('{') {
        (
            InputFormat::Json,
            Some("JSON object opener is not canonical TOON".to_owned()),
        )
    } else if trimmed.starts_with('[') && !toon_header {
        (
            InputFormat::Json,
            Some("JSON array opener is not a TOON counted-array header".to_owned()),
        )
    } else if trimmed.starts_with("---") || trimmed.starts_with('%') {
        (
            InputFormat::Yaml,
            Some("YAML document or directive marker".to_owned()),
        )
    } else if trimmed.starts_with("- ") {
        (
            InputFormat::Yaml,
            Some("YAML root-sequence marker".to_owned()),
        )
    } else if json_scalar_stream_is_complete(trimmed) {
        (
            InputFormat::Json,
            Some("JSON scalar stream is not canonical TOON".to_owned()),
        )
    } else {
        (InputFormat::Toon, None)
    };
    let commitment = first_line.len().min(inspected);
    Ok(if let Some(rejection) = rejection {
        ProbeReport {
            selected,
            lookahead_bytes: inspected,
            commitment_bytes: commitment,
            rejections: vec![(InputFormat::Toon, rejection)],
        }
    } else {
        ProbeReport {
            selected: InputFormat::Toon,
            lookahead_bytes: inspected,
            commitment_bytes: commitment,
            rejections: Vec::new(),
        }
    })
}

fn json_scalar_stream_is_complete(input: &str) -> bool {
    let first = input.as_bytes().first().copied();
    if !matches!(
        first,
        Some(
            b'"' | b'+' | b'-' | b'.' | b'0'
                ..=b'9' | b'I' | b'N' | b'S' | b'f' | b'i' | b'n' | b's' | b't',
        )
    ) {
        return false;
    }
    let mut reader = JsonInput::new(Cursor::new(input.as_bytes()), JsonInputOptions::default());
    let mut checkpoint = || Ok(());
    let mut count = 0_u8;
    loop {
        match reader.next_value(&mut checkpoint) {
            Ok(Some(_)) => count = count.saturating_add(1),
            Ok(None) => break,
            Err(_) => return false,
        }
    }
    count >= 2 || (count == 1 && (first == Some(b'"') || input.as_bytes().last() == Some(&b'\n')))
}

/// Reads bounded lookahead, returns its decision, and preserves every byte in a
/// replay reader for the selected parser.
///
/// Up to three continuation bytes beyond the configured lookahead are retained
/// solely to distinguish a split UTF-8 scalar from invalid input; the reported
/// inspected and commitment offsets remain within the configured bound.
///
/// # Errors
///
/// Returns lookahead I/O or the same combined rejection as [`probe_format`].
pub fn probe_reader<R: Read>(
    mut reader: R,
    maximum_lookahead_bytes: usize,
) -> Result<(ProbeReport, ReplayReader<R>), FormatError> {
    let capacity = maximum_lookahead_bytes.saturating_add(3);
    let mut prefix = Vec::with_capacity(capacity);
    while prefix.len() < capacity {
        let mut byte = [0_u8; 1];
        if reader.read(&mut byte)? == 0 {
            break;
        }
        prefix.push(byte[0]);
        if probe_commit_boundary(&prefix, maximum_lookahead_bytes) {
            break;
        }
    }
    let report = probe_format(&prefix, maximum_lookahead_bytes)?;
    Ok((
        report,
        ReplayReader {
            prefix: Cursor::new(prefix),
            reader,
        },
    ))
}

fn probe_commit_boundary(bytes: &[u8], maximum_lookahead_bytes: usize) -> bool {
    let Ok(report) = probe_format(bytes, maximum_lookahead_bytes) else {
        return false;
    };
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    let trimmed = text.trim_start();
    if trimmed.is_empty() {
        return false;
    }
    // A root TOON counted-array header begins with the same `[` byte as a JSON
    // array.  Do not commit to JSON while the count/header is still arriving;
    // a one-byte probe otherwise permanently selects JSON for a fragmented
    // `[2]:` or `[2]{key}:` document.  A physical newline ends that candidate
    // because TOON headers cannot span lines.
    let first_line = trimmed.lines().next().unwrap_or("");
    if root_toon_array_header_prefix(first_line) && bytes.last() != Some(&b'\n') {
        return false;
    }
    let structural_prefix = trimmed.starts_with(['{', '[', '%'])
        || trimmed.starts_with("---")
        || trimmed.starts_with("- ");
    structural_prefix && report.selected != InputFormat::Toon || bytes.last() == Some(&b'\n')
}

fn root_toon_array_header_prefix(line: &str) -> bool {
    if !line.starts_with('[') {
        return false;
    }
    if line == "[" {
        return true;
    }
    if let Some(close) = line.find(']') {
        if root_toon_array_header(line) {
            return true;
        }
        if close + 1 == line.len() {
            let declaration = &line[1..close];
            let count = declaration
                .strip_suffix([',', '|', '\t'])
                .unwrap_or(declaration);
            return !count.is_empty() && count.bytes().all(|byte| byte.is_ascii_digit());
        }
        return false;
    }
    let declaration = &line[1..];
    let count = declaration
        .strip_suffix([',', '|', '\t'])
        .unwrap_or(declaration);
    !count.is_empty() && count.bytes().all(|byte| byte.is_ascii_digit())
}

fn root_toon_array_header(line: &str) -> bool {
    let Some(close) = line.find(']') else {
        return false;
    };
    let declaration = &line[1..close];
    let count = declaration
        .strip_suffix([',', '|', '\t'])
        .unwrap_or(declaration);
    !count.is_empty()
        && count.bytes().all(|byte| byte.is_ascii_digit())
        && line[close + 1..].starts_with([':', '{'])
}

/// Decodes one strict TOON document.
///
/// # Errors
///
/// Returns the source-positioned TOON failure.
pub fn decode_toon(
    bytes: &[u8],
    identity: impl Into<String>,
    config: DecoderConfig,
) -> Result<Vec<Document>, FormatError> {
    let value = decode_to_value(Cursor::new(bytes), SourceId::new(0), config).map_err(|error| {
        FormatError::Parse {
            format: InputFormat::Toon,
            message: error.to_string(),
        }
    })?;
    Ok(vec![Document {
        value,
        identity: identity.into(),
        format: InputFormat::Toon,
        index: 0,
        line_number: 1,
    }])
}

/// Decodes ordered, arbitrary-precision-aware JSON documents.
///
/// # Errors
///
/// Returns JSON syntax/trailing-content or numeric-envelope failures.
pub fn decode_json(
    bytes: &[u8],
    identity: impl Into<String>,
) -> Result<Vec<Document>, FormatError> {
    decode_json_with_options(bytes, identity, DecodeOptions::default())
}

/// Decodes ordered JSON documents with explicit depth and token limits.
///
/// # Errors
///
/// Returns source I/O, JSON syntax/trailing-content, numeric-envelope, or
/// resource-limit failures.
pub fn decode_json_with_options(
    bytes: &[u8],
    identity: impl Into<String>,
    options: DecodeOptions,
) -> Result<Vec<Document>, FormatError> {
    if bytes.len() > options.maximum_source_bytes {
        return Err(FormatError::Resource("source-bytes"));
    }
    let json_options = JsonInputOptions {
        maximum_depth: options.maximum_depth,
        maximum_token_bytes: options.maximum_token_bytes,
    };
    let mut source = JsonDocumentSource::with_options(Cursor::new(bytes), identity, json_options);
    let mut documents = Vec::new();
    while let Some(document) = source.next_document()? {
        documents.push(document);
    }
    Ok(documents)
}

/// Decodes all JSON Text Sequence records, returning the first malformed
/// record as an input error.
///
/// # Errors
///
/// Returns source I/O, framing, or JSON syntax failures.
pub fn decode_json_sequence(
    bytes: &[u8],
    identity: impl Into<String>,
) -> Result<Vec<Document>, FormatError> {
    decode_json_sequence_with_options(bytes, identity, DecodeOptions::default())
}

/// Decodes JSON Text Sequence records with explicit JSON resource limits.
///
/// # Errors
///
/// Returns source I/O, framing, JSON syntax, numeric-envelope, or
/// resource-limit failures.
pub fn decode_json_sequence_with_options(
    bytes: &[u8],
    identity: impl Into<String>,
    options: DecodeOptions,
) -> Result<Vec<Document>, FormatError> {
    if bytes.len() > options.maximum_source_bytes {
        return Err(FormatError::Resource("source-bytes"));
    }
    let json_options = JsonInputOptions {
        maximum_depth: options.maximum_depth,
        maximum_token_bytes: options.maximum_token_bytes,
    };
    let mut source =
        JsonSequenceDocumentSource::with_options(BufReader::new(bytes), identity, json_options);
    let mut documents = Vec::new();
    while let Some(record) = source.next_record()? {
        documents.push(record?);
    }
    Ok(documents)
}

/// Decodes one JSON5 document into tq's ordered value model.
///
/// # Errors
///
/// Returns UTF-8, JSON5 syntax, numeric-envelope, or resource-limit failures.
pub fn decode_json5(
    bytes: &[u8],
    identity: impl Into<String>,
    options: DecodeOptions,
) -> Result<Vec<Document>, FormatError> {
    if bytes.len() > options.maximum_source_bytes {
        return Err(FormatError::Resource("source-bytes"));
    }
    let identity = identity.into();
    let text = std::str::from_utf8(bytes).map_err(|error| FormatError::Parse {
        format: InputFormat::Json5,
        message: error.to_string(),
    })?;
    let normalized =
        preprocess(text, options.maximum_token_bytes, options.maximum_depth).map_err(|error| {
            match error {
                PreprocessError::Parse { offset, message } => FormatError::Parse {
                    format: InputFormat::Json5,
                    message: format!(
                        "{message} at {}",
                        json5::Position::from_offset(offset, text)
                    ),
                },
                PreprocessError::Resource(resource) => FormatError::Resource(resource),
            }
        })?;
    let value =
        json5::from_str::<Value>(normalized.text()).map_err(|error| FormatError::Parse {
            format: InputFormat::Json5,
            message: normalized.translate_error(&error, text),
        })?;
    Ok(vec![Document {
        value,
        identity,
        format: InputFormat::Json5,
        index: 0,
        line_number: 1,
    }])
}

/// Decodes a YAML stream one document at a time through `yaml_serde`.
///
/// Mapping keys must deserialize as strings; duplicates and tags are rejected
/// by the custom runtime-value visitor. YAML floats enter the explicit binary64
/// arithmetic side of tq's hybrid number model, and non-finite values fail.
///
/// # Errors
///
/// Returns YAML syntax or tq profile failures.
pub fn decode_yaml(
    bytes: &[u8],
    identity: impl Into<String>,
) -> Result<Vec<Document>, FormatError> {
    let identity = identity.into();
    // JSON is a YAML 1.2 subset. Prefer the exact-literal JSON decoder when
    // the complete source satisfies that subset so yaml_serde cannot round a
    // large decimal through its binary64 visitor before tq sees it.
    if is_json_number(bytes) || serde_json::from_slice::<serde_json::Value>(bytes).is_ok() {
        let mut documents = decode_json(bytes, identity.clone())?;
        for document in &mut documents {
            document.format = InputFormat::Yaml;
        }
        return Ok(documents);
    }
    let mut documents = Vec::new();
    for (index, document) in yaml_serde::Deserializer::from_slice(bytes).enumerate() {
        let value = YamlRuntime::deserialize(document)
            .map_err(|error| FormatError::Parse {
                format: InputFormat::Yaml,
                message: error.to_string(),
            })?
            .0;
        documents.push(Document {
            value,
            identity: identity.clone(),
            format: InputFormat::Yaml,
            index: index as u64,
            line_number: index as u64 + 1,
        });
    }
    Ok(documents)
}

fn is_json_number(bytes: &[u8]) -> bool {
    let bytes = bytes.trim_ascii();
    let mut index = usize::from(bytes.first() == Some(&b'-'));
    match bytes.get(index) {
        Some(b'0') => index += 1,
        Some(b'1'..=b'9') => {
            index += 1;
            while bytes.get(index).is_some_and(u8::is_ascii_digit) {
                index += 1;
            }
        }
        _ => return false,
    }
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        let fraction_start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if index == fraction_start {
            return false;
        }
    }
    if bytes
        .get(index)
        .is_some_and(|byte| matches!(byte, b'e' | b'E'))
    {
        index += 1;
        if bytes
            .get(index)
            .is_some_and(|byte| matches!(byte, b'+' | b'-'))
        {
            index += 1;
        }
        let exponent_start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if index == exponent_start {
            return false;
        }
    }
    index == bytes.len()
}

/// Decodes an RS-prefix/LF-suffix TOON Text Sequence into ordered documents.
///
/// # Errors
///
/// Returns framing or per-record TOON failures.
pub fn decode_toon_sequence(
    bytes: &[u8],
    identity: impl Into<String>,
    config: DecoderConfig,
) -> Result<Vec<Document>, FormatError> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    if bytes.first() != Some(&0x1e) {
        return Err(FormatError::Parse {
            format: InputFormat::ToonSequence,
            message: "record must begin with ASCII RS".to_owned(),
        });
    }
    let identity = identity.into();
    let mut documents = Vec::new();
    for (index, record) in bytes[1..].split(|byte| *byte == 0x1e).enumerate() {
        let Some(record) = record.strip_suffix(b"\n") else {
            return Err(FormatError::Parse {
                format: InputFormat::ToonSequence,
                message: format!("record {index} is missing LF suffix"),
            });
        };
        let mut decoded = decode_toon(record, identity.clone(), config)?;
        let mut document = decoded.pop().ok_or_else(|| FormatError::Parse {
            format: InputFormat::ToonSequence,
            message: format!("record {index} produced no document"),
        })?;
        document.format = InputFormat::ToonSequence;
        document.index = index as u64;
        documents.push(document);
    }
    Ok(documents)
}

struct YamlRuntime(Value);

impl<'de> Deserialize<'de> for YamlRuntime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(YamlVisitor)
    }
}

struct YamlVisitor;

impl<'de> Visitor<'de> for YamlVisitor {
    type Value = YamlRuntime;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON-shaped YAML value")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(YamlRuntime(Value::Null))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(YamlRuntime(Value::Null))
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(YamlRuntime(Value::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        number(value.to_string()).map_err(E::custom)
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        number(value.to_string()).map_err(E::custom)
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        const MAX_EXACT_INTEGER: f64 = 9_007_199_254_740_992.0;
        if value.fract() == 0.0 && value.abs() > MAX_EXACT_INTEGER {
            return Err(E::custom(
                "YAML integer is outside binary64's exact envelope; use a JSON-subset scalar to preserve it",
            ));
        }
        Number::from_f64(value)
            .map(Value::Number)
            .map(YamlRuntime)
            .map_err(E::custom)
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(YamlRuntime(Value::string(value)))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(YamlRuntime(Value::string(value)))
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element::<YamlRuntime>()? {
            values.push(value.0);
        }
        Ok(YamlRuntime(Value::array(values)))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = Object::new();
        while let Some(key) = map.next_key::<yaml_serde::Value>()? {
            let yaml_serde::Value::String(key) = key else {
                return Err(de::Error::custom("YAML mapping keys must be strings"));
            };
            let key: Arc<str> = key.into();
            if values.contains_key(&key) {
                return Err(de::Error::custom(format!("duplicate mapping key '{key}'")));
            }
            let value = map.next_value::<YamlRuntime>()?;
            values.insert(key, value.0);
        }
        Ok(YamlRuntime(Value::object(values)))
    }

    fn visit_enum<A>(self, _data: A) -> Result<Self::Value, A::Error>
    where
        A: de::EnumAccess<'de>,
    {
        Err(de::Error::custom("custom YAML tags are unsupported"))
    }
}

fn number<E: fmt::Display>(literal: E) -> Result<YamlRuntime, tq_core::NumberError> {
    Number::parse(&literal.to_string())
        .map(Value::Number)
        .map(YamlRuntime)
}

#[cfg(test)]
mod tests {
    use std::io::{self, BufRead, BufReader, Cursor, Read};

    use tq_core::Value;
    use tq_toon::DecoderConfig;

    use super::{
        DecodeOptions, DocumentSource, JsonDocumentSource, JsonLinesDocumentSource,
        JsonSequenceDocumentSource, decode_bytes, decode_json, decode_json_lines, decode_json5,
        decode_toon_sequence, decode_yaml,
    };
    use crate::{FormatError, InputFormat};

    struct BoundedReader<'a> {
        bytes: &'a [u8],
        offset: usize,
        reads: usize,
        maximum_reads: usize,
    }

    impl<'a> BoundedReader<'a> {
        fn new(bytes: &'a [u8], maximum_reads: usize) -> Self {
            Self {
                bytes,
                offset: 0,
                reads: 0,
                maximum_reads,
            }
        }
    }

    impl Read for BoundedReader<'_> {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            if self.reads >= self.maximum_reads {
                return Err(io::Error::other("bounded test reader was read past limit"));
            }
            self.reads = self.reads.saturating_add(1);
            if self.offset == self.bytes.len() {
                return Ok(0);
            }
            output[0] = self.bytes[self.offset];
            self.offset += 1;
            Ok(1)
        }
    }

    impl BufRead for BoundedReader<'_> {
        fn fill_buf(&mut self) -> io::Result<&[u8]> {
            if self.reads >= self.maximum_reads {
                return Err(io::Error::other("bounded test reader was read past limit"));
            }
            Ok(self
                .bytes
                .get(self.offset..self.offset.saturating_add(1))
                .unwrap_or(&[]))
        }

        fn consume(&mut self, amount: usize) {
            self.offset = self.offset.saturating_add(amount).min(self.bytes.len());
        }
    }

    #[test]
    fn equivalent_formats_share_one_ordered_value_model() {
        let toon = decode_bytes(b"z: 1\na[2]: true,x", "t", DecodeOptions::default()).unwrap();
        let yaml = decode_yaml(b"z: 1\na: [true, x]", "y").unwrap();
        let json = decode_bytes(
            br#"{"z":1,"a":[true,"x"]}"#,
            "j",
            DecodeOptions {
                format: InputFormat::Json,
                ..DecodeOptions::default()
            },
        )
        .unwrap();
        assert_eq!(toon[0].value, yaml[0].value);
        assert_eq!(yaml[0].value, json[0].value);
        assert_eq!(toon[0].format, InputFormat::Toon);
    }

    #[test]
    fn json_documents_preserve_order_and_exact_numbers() {
        let documents =
            decode_json(br#"{"z":9007199254740993,"a":1} [2,3]"#, "documents.json").unwrap();

        assert_eq!(documents.len(), 2);
        assert_eq!(
            documents[0].value.to_string(),
            r#"{"z":9007199254740993,"a":1}"#
        );
        assert_eq!(documents[0].identity, "documents.json");
        assert_eq!(documents[0].index, 0);
        assert_eq!(documents[1].value.to_string(), "[2,3]");
        assert_eq!(documents[1].index, 1);
    }

    #[test]
    fn direct_json_source_preserves_nested_number_lexemes_and_limits() {
        let mut source = JsonDocumentSource::new(
            Cursor::new(br#"{"n":1E+3,"nested":[-0.0,9007199254740993]}"#),
            "direct.json",
        );
        let document = source.next_document().unwrap().unwrap();
        assert_eq!(
            serde_json::to_string(&document.value).unwrap(),
            r#"{"n":1E+3,"nested":[-0.0,9007199254740993]}"#
        );
        assert!(source.next_document().unwrap().is_none());

        let oversized = format!("[{}]", "1".repeat(4097));
        let mut source = JsonDocumentSource::new(Cursor::new(oversized), "limit.json");
        assert!(source.next_document().is_err());
    }

    #[test]
    fn json_documents_track_physical_end_lines() {
        let documents = decode_json(b"\n\n1 2\n3\n", "lines.json").unwrap();
        assert_eq!(
            documents
                .iter()
                .map(|document| document.line_number)
                .collect::<Vec<_>>(),
            vec![3, 3, 4]
        );

        let documents = decode_json(b"[\n1\n]\n", "multiline.json").unwrap();
        assert_eq!(documents[0].line_number, 3);
    }

    #[test]
    fn json_document_source_is_send_when_its_reader_is_send() {
        fn assert_send<T: Send>() {}

        assert_send::<JsonDocumentSource<Cursor<&'static [u8]>>>();
    }

    #[test]
    fn json5_document_accepts_standard_syntax_and_preserves_order() {
        let documents = decode_json5(
            br"{/* comment */ z: 0x20000000000001, ratio: 1.5, a: ['value',],}",
            "document.json5",
            DecodeOptions::default(),
        )
        .unwrap();

        assert_eq!(documents.len(), 1);
        assert_eq!(
            documents[0].value.to_string(),
            r#"{"z":9007199254740993,"ratio":1.5,"a":["value"]}"#
        );
        assert_eq!(documents[0].identity, "document.json5");
        assert_eq!(documents[0].format, InputFormat::Json5);
        assert_eq!(documents[0].index, 0);
    }

    #[test]
    fn json5_document_accepts_required_escape_and_number_forms() {
        let documents = decode_json5(
            br"{escaped: '\x41\u0042', continued: 'first\
second', leading: .5, trailing: 5., positive: +6, negative: -7, exponent: 1e2}",
            "grammar.json5",
            DecodeOptions::default(),
        )
        .unwrap();

        assert_eq!(
            documents[0].value.to_string(),
            r#"{"escaped":"AB","continued":"firstsecond","leading":0.5,"trailing":5,"positive":6,"negative":-7,"exponent":100}"#
        );
    }

    #[test]
    fn json5_document_accepts_unicode_identifiers_and_whitespace() {
        let identifier = decode_json5(
            "{café: 1}".as_bytes(),
            "unicode.json5",
            DecodeOptions::default(),
        )
        .unwrap();
        assert_eq!(identifier[0].value.to_string(), r#"{"café":1}"#);

        let whitespace = decode_json5(
            "{\"a\":\u{00a0}1}".as_bytes(),
            "unicode-whitespace.json5",
            DecodeOptions {
                maximum_token_bytes: 1,
                ..DecodeOptions::default()
            },
        )
        .unwrap();
        assert_eq!(whitespace[0].value.to_string(), r#"{"a":1}"#);
    }

    #[test]
    fn json5_document_preserves_literal_triple_quoted_content() {
        let documents = decode_json5(
            br#"{markdown: """first line
second \n line with "quotes""""}"#,
            "markdown.json",
            DecodeOptions::default(),
        )
        .unwrap();

        assert_eq!(
            documents[0].value.to_string(),
            r#"{"markdown":"first line\nsecond \\n line with \"quotes\""}"#
        );
    }

    #[test]
    fn json5_document_enforces_token_and_depth_limits_lexically() {
        let shallow = decode_json5(
            br#"{/* [[[ */ value: "[not depth]"}"#,
            "shallow.json5",
            DecodeOptions {
                maximum_depth: 1,
                ..DecodeOptions::default()
            },
        )
        .unwrap();
        assert_eq!(shallow[0].value.to_string(), r#"{"value":"[not depth]"}"#);

        let exact_string = decode_json5(
            br#"{a: "abc"}"#,
            "exact-token.json5",
            DecodeOptions {
                maximum_token_bytes: 3,
                ..DecodeOptions::default()
            },
        )
        .unwrap();
        assert_eq!(exact_string[0].value.to_string(), r#"{"a":"abc"}"#);

        for input in [
            br"{long: 1}".as_slice(),
            br#"{a: "long"}"#.as_slice(),
            br#"{a: """long"""}"#.as_slice(),
        ] {
            let error = decode_json5(
                input,
                "token.json5",
                DecodeOptions {
                    maximum_token_bytes: 3,
                    ..DecodeOptions::default()
                },
            )
            .unwrap_err();
            assert!(matches!(error, crate::FormatError::Resource("token-bytes")));
        }

        let error = decode_json5(
            b"[[0]]",
            "depth.json5",
            DecodeOptions {
                maximum_depth: 1,
                ..DecodeOptions::default()
            },
        )
        .unwrap_err();
        assert!(matches!(error, crate::FormatError::Resource("depth")));

        let error = decode_json5(
            b"{value: 1}",
            "source.json5",
            DecodeOptions {
                maximum_source_bytes: 4,
                ..DecodeOptions::default()
            },
        )
        .unwrap_err();
        assert!(matches!(
            error,
            crate::FormatError::Resource("source-bytes")
        ));
    }

    #[test]
    fn json5_document_reports_original_locations_and_profile_errors() {
        let quote_run = decode_json5(
            br#"{value: """a""""}"#,
            "quotes.json5",
            DecodeOptions::default(),
        )
        .unwrap();
        assert_eq!(quote_run[0].value.to_string(), r#"{"value":"a\""}"#);

        let unterminated = decode_json5(
            br#"{value: """unfinished}"#,
            "unterminated.json5",
            DecodeOptions::default(),
        )
        .unwrap_err()
        .to_string();
        assert!(unterminated.contains("unterminated triple-quoted string"));
        assert!(unterminated.contains("line 1 column 9"));

        let translated = decode_json5(
            b"{value: \"\"\"first\nsecond\"\"\", broken:}",
            "location.json5",
            DecodeOptions::default(),
        )
        .unwrap_err()
        .to_string();
        assert!(translated.contains("line 2"), "{translated}");

        for input in [
            b"NaN".as_slice(),
            b"Infinity",
            b"-Infinity",
            b"{} {}",
            &[0xff],
        ] {
            assert!(decode_json5(input, "invalid.json5", DecodeOptions::default()).is_err());
        }
    }

    #[test]
    fn json5_document_decodes_esdiag_saved_object_fixture() {
        let fixture = tq_toon::decode_to_value(
            include_bytes!("../../../tests/fixtures/esdiag-saved-object.toon").as_slice(),
            tq_core::SourceId::new(0),
            tq_toon::DecoderConfig::default(),
        )
        .unwrap()
        .to_json()
        .unwrap();
        let documents = decode_json5(
            fixture["source_text"].as_str().unwrap().as_bytes(),
            "esdiag-saved-object.json",
            DecodeOptions::default(),
        )
        .unwrap();
        let tq_core::Value::Object(root) = &documents[0].value else {
            panic!("saved object root")
        };
        let Some(tq_core::Value::Object(attributes)) = root.get("attributes") else {
            panic!("saved object attributes")
        };
        let markdown = attributes.get("markdown");

        assert_eq!(
            markdown,
            Some(&tq_core::Value::string(
                "### About\n\nElastic Stack Diagnostics simplifies collecting and analyzing deployment health.\nUse the `Diagnostics List` to select a report."
            ))
        );
    }

    #[test]
    fn yaml_is_multi_document_and_rejects_profile_violations() {
        let documents = decode_yaml(b"---\na: 1\n---\na: 2\n", "stream").unwrap();
        assert_eq!(documents.len(), 2);
        assert!(decode_yaml(b"true: value", "bad-key").is_err());
        assert!(decode_yaml(b"a: 1\na: 2", "duplicate").is_err());
        assert!(decode_yaml(b"a: .nan", "non-finite").is_err());
        assert!(decode_yaml(b"a: !custom value", "tag").is_err());
    }

    #[test]
    fn yaml_never_silently_rounds_large_json_subset_integers() {
        let exact = b"1111111111111111111111111111111111111111";
        let documents = decode_yaml(exact, "exact-json-subset").unwrap();
        assert_eq!(
            documents[0].value.to_string(),
            String::from_utf8_lossy(exact)
        );

        let block = b"value: 1111111111111111111111111111111111111111";
        assert!(decode_yaml(block, "inexact-block-scalar").is_err());

        let over_limit = "1".repeat(4097);
        assert!(decode_yaml(over_limit.as_bytes(), "over-limit-json-subset").is_err());

        let quoted_digits = format!("\"{over_limit}\"");
        let documents = decode_yaml(quoted_digits.as_bytes(), "quoted-digits").unwrap();
        assert_eq!(documents[0].value, tq_core::Value::string(over_limit));
    }

    #[test]
    fn sequence_framing_is_strict_and_ordered() {
        let documents = decode_toon_sequence(
            b"\x1ea: 1\n\x1eb: 2\n",
            "sequence",
            DecoderConfig::default(),
        )
        .unwrap();
        assert_eq!(documents.len(), 2);
        assert_eq!(documents[1].index, 1);
        assert!(decode_toon_sequence(b"a: 1\n", "bad", DecoderConfig::default()).is_err());
        assert!(decode_toon_sequence(b"\x1ea: 1", "bad", DecoderConfig::default()).is_err());
    }

    #[test]
    fn json_sequence_source_recovers_and_preserves_order() {
        let input = b"\x1e1 2\n\x1ebad\n\x1e3\n";
        let mut source = JsonSequenceDocumentSource::new(BufReader::new(&input[..]), "sequence");
        let mut values = Vec::new();
        let mut errors = Vec::new();
        while let Some(record) = source.next_record().unwrap() {
            match record {
                Ok(document) => values.push(document.value.to_string()),
                Err(error) => errors.push(error.to_string()),
            }
        }
        assert_eq!(values, ["1", "2", "3"]);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Invalid numeric literal at line 3, column 0"));
    }

    #[test]
    fn json_sequence_rejects_adjacent_scalar_tokens_without_delimiter() {
        for (input, expected) in [
            (
                b"\x1etruefalse\n\x1e4\n".as_slice(),
                "Invalid literal at line 2, column 0",
            ),
            (
                b"\x1e1true\n\x1e4\n".as_slice(),
                "Invalid numeric literal at line 2, column 0",
            ),
        ] {
            let mut source =
                JsonSequenceDocumentSource::new(BufReader::with_capacity(1, input), "sequence");
            let mut values = Vec::new();
            let mut errors = Vec::new();
            while let Some(record) = source.next_record().unwrap() {
                match record {
                    Ok(document) => values.push(document.value.to_string()),
                    Err(error) => errors.push(error.to_string()),
                }
            }
            assert_eq!(values, ["4"]);
            assert_eq!(errors.len(), 1);
            assert!(errors[0].contains(expected), "{}", errors[0]);
        }
    }

    #[test]
    fn json_sequence_uses_shared_nonfinite_scalar_grammar() {
        let input = b"\x1eNaN\n\x1eInfinity\n\x1e-Infinity\n\x1e4\n";
        let mut source =
            JsonSequenceDocumentSource::new(BufReader::with_capacity(1, &input[..]), "sequence");
        let mut values = Vec::new();
        while let Some(record) = source.next_record().unwrap() {
            values.push(record.unwrap().value);
        }
        assert_eq!(values.len(), 4);
        assert!(matches!(
            values[0],
            Value::Number(ref number) if number.as_f64().is_nan()
        ));
        assert!(matches!(
            values[1],
            Value::Number(ref number) if number.as_f64().is_infinite() && number.as_f64().is_sign_positive()
        ));
        assert!(matches!(
            values[2],
            Value::Number(ref number) if number.as_f64().is_infinite() && number.as_f64().is_sign_negative()
        ));
        assert_eq!(values[3].to_string(), "4");
    }

    #[test]
    fn json_sequence_marks_unterminated_nonfinite_and_extended_numeric_roots() {
        let input = b"\x1eNaN\x1eInfinity\x1e-Infinity\x1e+1\x1e.1";
        let mut source =
            JsonSequenceDocumentSource::new(BufReader::with_capacity(1, &input[..]), "sequence");
        let mut errors = Vec::new();
        while let Some(record) = source.next_record().unwrap() {
            if let Err(error) = record {
                errors.push(error.to_string());
            }
        }
        assert_eq!(errors.len(), 5);
        assert!(
            errors
                .iter()
                .all(|error| { error.contains("Potentially truncated top-level numeric value") })
        );
    }

    #[test]
    fn json_sequence_matches_jq_eof_and_record_boundary_numeric_positions() {
        for (token, column) in [
            ("NaN", 4),
            ("Infinity", 9),
            ("-Inf", 5),
            ("+1", 3),
            (".1", 3),
        ] {
            let input = format!("\x1e{token}");
            let mut source = JsonSequenceDocumentSource::new(
                BufReader::with_capacity(1, input.as_bytes()),
                "sequence",
            );
            let error = source
                .next_record()
                .unwrap()
                .unwrap()
                .unwrap_err()
                .to_string();
            assert!(
                error.contains(&format!(
                    "Potentially truncated top-level numeric value at EOF at line 1, column {column}"
                )),
                "{token}: {error}"
            );
            assert!(source.next_record().unwrap().is_none());
        }

        let mut source = JsonSequenceDocumentSource::new(
            BufReader::with_capacity(1, b"\x1e123\x1e4\n".as_slice()),
            "sequence",
        );
        let error = source
            .next_record()
            .unwrap()
            .unwrap()
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("Potentially truncated top-level numeric value at line 1, column 5"),
            "{error}"
        );
        assert_eq!(
            source
                .next_record()
                .unwrap()
                .unwrap()
                .unwrap()
                .value
                .to_string(),
            "4"
        );
    }

    #[test]
    fn json_sequence_allows_punctuation_adjacent_roots() {
        let input = b"\x1eNaN{}\n\x1e1{}\n\x1etrue{}\n";
        let mut source =
            JsonSequenceDocumentSource::new(BufReader::with_capacity(1, &input[..]), "sequence");
        let mut values = Vec::new();
        while let Some(record) = source.next_record().unwrap() {
            values.push(record.unwrap().value);
        }
        assert_eq!(values.len(), 6);
        assert!(matches!(values[0], Value::Number(ref number) if number.as_f64().is_nan()));
        assert!(matches!(values[1], Value::Object(_)));
        assert_eq!(values[2].to_string(), "1");
        assert!(matches!(values[3], Value::Object(_)));
        assert_eq!(values[4].to_string(), "true");
        assert!(matches!(values[5], Value::Object(_)));
    }

    #[test]
    fn json_sequence_resource_limit_is_terminal_and_typed() {
        let input = b"\x1e123\x1e4\n";
        let options = tq_core::JsonInputOptions {
            maximum_depth: 256,
            maximum_token_bytes: 2,
        };
        let mut source = JsonSequenceDocumentSource::with_options(
            BufReader::with_capacity(1, &input[..]),
            "sequence",
            options,
        );
        let first = source.next_record().unwrap_err();
        assert_eq!(
            first.to_string(),
            "input resource limit exceeded: token-bytes"
        );
        assert!(source.next_record().unwrap().is_none());
    }

    #[test]
    fn json_sequence_enforces_depth_before_reading_an_unterminated_record() {
        let reader = BufReader::with_capacity(1, BoundedReader::new(b"\x1e[[", 4));
        let mut source = JsonSequenceDocumentSource::with_options(
            reader,
            "sequence",
            tq_core::JsonInputOptions {
                maximum_depth: 1,
                maximum_token_bytes: 1024,
            },
        );
        assert!(matches!(
            source.next_record(),
            Err(FormatError::Resource("depth"))
        ));
    }

    #[test]
    fn json_sequence_enforces_scalar_limit_before_reading_to_record_end() {
        let reader = BufReader::with_capacity(1, BoundedReader::new(b"\x1e123456789", 5));
        let mut source = JsonSequenceDocumentSource::with_options(
            reader,
            "sequence",
            tq_core::JsonInputOptions {
                maximum_depth: 256,
                maximum_token_bytes: 3,
            },
        );
        assert!(matches!(
            source.next_record(),
            Err(FormatError::Resource("token-bytes"))
        ));
    }

    #[test]
    fn json_sequence_enforces_nested_scalar_limit_before_record_end() {
        let reader = BufReader::with_capacity(1, BoundedReader::new(b"\x1e[123456789", 6));
        let mut source = JsonSequenceDocumentSource::with_options(
            reader,
            "sequence",
            tq_core::JsonInputOptions {
                maximum_depth: 1,
                maximum_token_bytes: 3,
            },
        );
        assert!(matches!(
            source.next_record(),
            Err(FormatError::Resource("token-bytes"))
        ));
    }

    #[test]
    fn json_sequence_rejects_non_json_whitespace_as_a_delimiter() {
        for (input, expected) in [
            (
                b"\x1e1\x0c2\n\x1e3\n".as_slice(),
                "Invalid numeric literal at line 2, column 0",
            ),
            (
                b"\x1etrue\x0bfalse\n\x1e3\n".as_slice(),
                "Invalid literal at line 2, column 0",
            ),
        ] {
            let mut source =
                JsonSequenceDocumentSource::new(BufReader::with_capacity(1, input), "sequence");
            let mut values = Vec::new();
            let mut errors = Vec::new();
            while let Some(record) = source.next_record().unwrap() {
                match record {
                    Ok(document) => values.push(document.value.to_string()),
                    Err(error) => errors.push(error.to_string()),
                }
            }
            assert_eq!(values, ["3"]);
            assert_eq!(errors.len(), 1);
            assert!(errors[0].contains(expected), "{}", errors[0]);
        }
    }

    #[test]
    fn json_sequence_rejects_truncated_numeric_after_prior_value() {
        let input = b"\x1e1\n2\x1e4\n";
        let mut source =
            JsonSequenceDocumentSource::new(BufReader::with_capacity(1, &input[..]), "sequence");
        let mut values = Vec::new();
        let mut errors = Vec::new();
        while let Some(record) = source.next_record().unwrap() {
            match record {
                Ok(document) => values.push(document.value.to_string()),
                Err(error) => errors.push(error.to_string()),
            }
        }
        assert_eq!(values, ["1", "4"]);
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0].contains("Potentially truncated top-level numeric value at line 2, column 2"),
            "{}",
            errors[0]
        );
    }

    #[test]
    fn json_sequence_reports_jq_positions_for_structured_and_truncated_records() {
        for (input, expected) in [
            (
                b"\x1e{\"x\":1}\n\x1e[bad]\n\x1e2\n".as_slice(),
                "Invalid numeric literal at line 2, column 6",
            ),
            (
                b"\x1e1\x0c2\n\x1e3\n".as_slice(),
                "Invalid numeric literal at line 2, column 0",
            ),
        ] {
            let mut source =
                JsonSequenceDocumentSource::new(BufReader::with_capacity(1, input), "sequence");
            let mut errors = Vec::new();
            while let Some(record) = source.next_record().unwrap() {
                if let Err(error) = record {
                    errors.push(error.to_string());
                }
            }
            assert_eq!(errors.len(), 1);
            assert!(errors[0].contains(expected), "{}", errors[0]);
        }
    }

    #[test]
    fn json_sequence_reports_jq_structured_recovery_diagnostics() {
        for (input, expected) in [
            (
                b"\x1e{\"a\":}\n\x1e2\n".as_slice(),
                "Unmatched '}' at line 1, column 7 (need RS to resync)",
            ),
            (
                b"\x1e[1,]\n\x1e2\n".as_slice(),
                "Expected another array element at line 1, column 5 (need RS to resync)",
            ),
            (
                b"\x1e{\"a\":1,}\n\x1e2\n".as_slice(),
                "Expected another key-value pair at line 1, column 9 (need RS to resync)",
            ),
            (
                b"\x1e{\"a\" 1}\n\x1e2\n".as_slice(),
                "Expected separator between values at line 1, column 8 (need RS to resync)",
            ),
        ] {
            let mut source =
                JsonSequenceDocumentSource::new(BufReader::with_capacity(1, input), "sequence");
            let mut values = Vec::new();
            let mut errors = Vec::new();
            while let Some(record) = source.next_record().unwrap() {
                match record {
                    Ok(document) => values.push(document.value.to_string()),
                    Err(error) => errors.push(error.to_string()),
                }
            }
            assert_eq!(values, ["2"]);
            assert_eq!(errors, [format!("JsonSequence input rejected: {expected}")]);
        }
    }

    #[test]
    fn json_sequence_classifies_unknown_tokens_like_jq() {
        let input = b"\x1e1\n\x1ebad\n\x1e2\n";
        let mut source =
            JsonSequenceDocumentSource::new(BufReader::with_capacity(1, &input[..]), "sequence");
        let mut values = Vec::new();
        let mut errors = Vec::new();
        while let Some(record) = source.next_record().unwrap() {
            match record {
                Ok(document) => values.push(document.value.to_string()),
                Err(error) => errors.push(error.to_string()),
            }
        }
        assert_eq!(values, ["1", "2"]);
        assert_eq!(
            errors,
            [
                "JsonSequence input rejected: Invalid numeric literal at line 3, column 0 (need RS to resync)"
            ]
        );
    }

    #[test]
    fn json_sequence_tiny_buffer_matches_default_and_compacts_large_record() {
        let mut input = b"\x1e{\"value\":\"".to_vec();
        input.extend(std::iter::repeat_n(b'a', 70 * 1024));
        input.extend_from_slice(b"\"}\n\x1e[1,2]\n3\n");

        let collect = |reader| {
            let mut source = JsonSequenceDocumentSource::new(reader, "sequence");
            let mut documents = Vec::new();
            while let Some(record) = source.next_record().unwrap() {
                let document = record.unwrap();
                documents.push((
                    document.value.to_string(),
                    document.index,
                    document.line_number,
                ));
            }
            documents
        };
        let default = collect(BufReader::new(&input[..]));
        let tiny = collect(BufReader::with_capacity(1, &input[..]));

        assert_eq!(tiny, default);
        assert_eq!(tiny.len(), 3);
        assert_eq!(tiny[0].1, 0);
        assert_eq!(tiny[2].1, 2);
    }

    #[test]
    fn json_sequence_compaction_preserves_physical_line_numbers() {
        let mut input = Vec::from(*b"\x1e");
        for _ in 0..40_000 {
            input.extend_from_slice(b"0\n");
        }
        let mut source =
            JsonSequenceDocumentSource::new(BufReader::with_capacity(1, &input[..]), "sequence");
        let mut lines = Vec::new();
        while let Some(record) = source.next_record().unwrap() {
            lines.push(record.unwrap().line_number);
        }
        assert_eq!(lines.len(), 40_000);
        assert_eq!(lines[32_767], 32_768);
        assert_eq!(lines[32_768], 32_769);
        assert_eq!(lines[39_999], 40_000);
    }

    #[test]
    fn json_sequence_compaction_preserves_same_line_diagnostic_column() {
        let mut input = Vec::from(*b"\x1e");
        input.extend(std::iter::repeat_n(b' ', 70 * 1024));
        input.extend_from_slice(b"[bad]\x1e2\n");
        let mut source =
            JsonSequenceDocumentSource::new(BufReader::with_capacity(1, &input[..]), "sequence");
        let mut values = Vec::new();
        let mut errors = Vec::new();
        while let Some(record) = source.next_record().unwrap() {
            match record {
                Ok(document) => values.push(document.value.to_string()),
                Err(error) => errors.push(error.to_string()),
            }
        }
        assert_eq!(values, ["2"]);
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0].contains("Invalid numeric literal at line 1, column 71686"),
            "{}",
            errors[0]
        );
    }

    #[test]
    fn bounded_probe_is_observable_and_late_failures_do_not_fail_down() {
        let json = super::probe_format(br#"{"a":1}"#, 4).unwrap();
        assert_eq!(json.selected, InputFormat::Json);
        assert_eq!(json.lookahead_bytes, 4);
        assert_eq!(json.rejections[0].0, InputFormat::Toon);

        let error = decode_bytes(br#"{"a":"#, "late", DecodeOptions::default()).unwrap_err();
        assert!(matches!(
            error,
            crate::FormatError::Parse {
                format: InputFormat::Json,
                ..
            }
        ));
        assert!(super::probe_format(&[0xff], 64).is_err());

        let source = "😀: value\nrest: intact".as_bytes();
        let (report, mut replay) = super::probe_reader(source, 2).unwrap();
        assert!(report.lookahead_bytes <= 2);
        let mut recovered = Vec::new();
        replay.read_to_end(&mut recovered).unwrap();
        assert_eq!(recovered, source);
    }

    #[test]
    fn fragmented_probe_distinguishes_root_toon_headers_from_json_arrays() {
        for (source, selected) in [
            (b"[2]: 1,2\n".as_slice(), InputFormat::Toon),
            (b"[2]{a}:\n  1\n  2\n".as_slice(), InputFormat::Toon),
            (b"[1,2]\n".as_slice(), InputFormat::Json),
        ] {
            let (report, mut replay) =
                super::probe_reader(BufReader::with_capacity(1, source), 64).unwrap();
            assert_eq!(report.selected, selected);
            let mut recovered = Vec::new();
            replay.read_to_end(&mut recovered).unwrap();
            assert_eq!(recovered, source);
        }
    }

    #[test]
    fn auto_probe_selects_json_string_scalars_with_unicode_escapes() {
        let report = super::probe_format(br#""\u03bc""#, 64).unwrap();
        assert_eq!(report.selected, InputFormat::Json);
        let documents =
            super::decode_bytes(br#""\u03bc""#, "scalar.json", DecodeOptions::default()).unwrap();
        assert_eq!(documents[0].value.to_string(), "\"μ\"");
    }

    #[test]
    fn json_lines_preserves_records_lines_and_exact_numbers() {
        let input = b"{\"n\":9007199254740993}\n\ntrue\n[1,2]";
        let documents =
            decode_json_lines(input, "records.jsonl", DecodeOptions::default()).unwrap();
        assert_eq!(documents.len(), 3);
        assert_eq!(documents[0].value.to_string(), r#"{"n":9007199254740993}"#);
        assert_eq!(documents[2].value.to_string(), "[1,2]");
        assert_eq!(documents[2].format, InputFormat::JsonLines);

        let error = decode_json_lines(b"true\n1 2\n", "bad.jsonl", DecodeOptions::default())
            .unwrap_err()
            .to_string();
        assert!(error.contains("bad.jsonl:2"));
    }

    #[test]
    fn json_lines_enforces_line_token_and_depth_limits() {
        let line_error = decode_json_lines(
            b"{\"long\":true}\n",
            "line.jsonl",
            DecodeOptions {
                maximum_line_bytes: 4,
                ..DecodeOptions::default()
            },
        )
        .unwrap_err()
        .to_string();
        assert!(line_error.contains("line.jsonl"));
        assert!(line_error.contains("line 1"));

        assert!(
            decode_json_lines(
                b"\"long\"\n",
                "token.jsonl",
                DecodeOptions {
                    maximum_token_bytes: 3,
                    ..DecodeOptions::default()
                },
            )
            .is_err()
        );
        assert!(
            decode_json_lines(
                b"[[true]]\n",
                "depth.jsonl",
                DecodeOptions {
                    maximum_depth: 1,
                    ..DecodeOptions::default()
                },
            )
            .is_err()
        );
    }

    #[test]
    fn json_lines_source_enforces_cumulative_source_limit() {
        let mut source = JsonLinesDocumentSource::new(
            BufReader::new(b"1\n2\n".as_slice()),
            "limited.jsonl",
            DecodeOptions {
                format: InputFormat::JsonLines,
                maximum_source_bytes: 2,
                ..DecodeOptions::default()
            },
        );
        assert!(source.next_document().unwrap().is_some());
        let error = source.next_document().unwrap_err().to_string();
        assert!(error.contains("limited.jsonl"));
        assert!(error.contains("source-bytes"));
    }
}
