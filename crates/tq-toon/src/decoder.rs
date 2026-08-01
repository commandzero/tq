//! Bounded line-oriented TOON-to-event decoder.

use std::{collections::VecDeque, io::BufRead, sync::Arc};

use tq_core::{Number, SourceId, SourcePosition, Span};

use crate::{DecodeError, DecoderConfig, Event, Scalar};

/// Incremental decoder retaining only one physical line, active container
/// state, tabular schemas, and pending events.
#[derive(Debug)]
pub struct Decoder<R> {
    reader: R,
    config: DecoderConfig,
    source: SourceId,
    pending: VecDeque<Event>,
    frames: Vec<Frame>,
    byte_offset: u64,
    line_number: u64,
    started: bool,
    root_complete: bool,
    finished: bool,
}

#[derive(Debug)]
struct Frame {
    content_depth: usize,
    kind: FrameKind,
}

#[derive(Debug)]
enum FrameKind {
    Object,
    Array {
        declared: u64,
        observed: u64,
    },
    Tabular {
        declared: u64,
        observed: u64,
        fields: Arc<[DecodedKey]>,
        delimiter: Delimiter,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Delimiter {
    Comma,
    Pipe,
    Tab,
}

impl Delimiter {
    const fn byte(self) -> u8 {
        match self {
            Self::Comma => b',',
            Self::Pipe => b'|',
            Self::Tab => b'\t',
        }
    }
}

#[derive(Debug)]
struct Line {
    text: String,
    start: u64,
    number: u64,
}

#[derive(Debug)]
struct Header {
    key: Option<DecodedKey>,
    declared: u64,
    delimiter: Delimiter,
    fields: Vec<DecodedKey>,
    inline: String,
}

#[derive(Clone, Debug)]
struct DecodedKey {
    value: Arc<str>,
    quoted: bool,
}

impl<R: BufRead> Decoder<R> {
    /// Creates a strict bounded decoder over a buffered reader.
    #[must_use]
    pub fn new(reader: R, source: SourceId, config: DecoderConfig) -> Self {
        Self {
            reader,
            config,
            source,
            pending: VecDeque::new(),
            frames: Vec::new(),
            byte_offset: 0,
            line_number: 0,
            started: false,
            root_complete: false,
            finished: false,
        }
    }

    /// Returns the next source-spanned event without materializing a document.
    ///
    /// # Errors
    ///
    /// Returns strict syntax, UTF-8, I/O, numeric, count, depth, token, or line
    /// limit failures at the best available position.
    pub fn next_event(&mut self) -> Result<Option<Event>, DecodeError> {
        loop {
            if let Some(event) = self.pending.pop_front() {
                return Ok(Some(event));
            }
            if self.finished {
                return Ok(None);
            }
            let Some(line) = self.read_line()? else {
                self.finish_document()?;
                continue;
            };
            self.process_line(&line)?;
        }
    }

    /// Feeds all events to a consumer while keeping decoder buffering bounded.
    ///
    /// # Errors
    ///
    /// Returns either a decoder error or consumer failure.
    pub fn decode_into<C: crate::EventConsumer>(
        &mut self,
        consumer: &mut C,
    ) -> Result<(), DecodeIntoError<C::Error>> {
        while let Some(event) = self.next_event()? {
            consumer.consume(event).map_err(DecodeIntoError::Consumer)?;
        }
        Ok(())
    }

    fn process_line(&mut self, line: &Line) -> Result<(), DecodeError> {
        let (depth, content) = self.indentation(line)?;
        if content.trim().is_empty() {
            let span = self.line_span(line);
            while self.frames.last().is_some_and(|frame| {
                matches!(
                    frame.kind,
                    FrameKind::Array { declared, observed }
                        | FrameKind::Tabular {
                            declared, observed, ..
                        } if declared == observed
                )
            }) {
                self.close_frame(span, line)?;
            }
            if self.config.strict
                && self
                    .frames
                    .iter()
                    .any(|frame| !matches!(frame.kind, FrameKind::Object))
            {
                return Err(self.syntax(line, 1, "blank line inside array"));
            }
            return Ok(());
        }
        let span = self.line_span(line);
        if !self.started {
            if depth != 0 {
                return Err(self.syntax(line, 1, "root value must begin at depth zero"));
            }
            self.started = true;
            self.pending.push_back(Event::DocumentStart { span });
            self.start_root(content, depth, line)?;
            return Ok(());
        }
        if self.root_complete {
            return Err(self.syntax(line, 1, "unexpected content after root value"));
        }

        self.close_for_line(depth, content, span, line)?;
        let Some(frame) = self.frames.last() else {
            self.root_complete = true;
            return Err(self.syntax(line, 1, "unexpected content after root container"));
        };
        if depth != frame.content_depth {
            return Err(self.syntax(
                line,
                1,
                "indentation does not match the active container depth",
            ));
        }
        match frame.kind {
            FrameKind::Object => self.object_member(content, depth, span, line),
            FrameKind::Array { .. } => self.list_item(content, depth, span, line),
            FrameKind::Tabular { .. } => self.tabular_row(content, span, line),
        }
    }

    fn start_root(&mut self, content: &str, depth: usize, line: &Line) -> Result<(), DecodeError> {
        let span = self.line_span(line);
        if content.starts_with('[') {
            let header = self.header(content, line)?;
            if header.key.is_some() {
                return Err(self.syntax(line, 1, "root array header cannot contain a key"));
            }
            self.emit_header(header, depth, span, line)?;
        } else if self.header_start(content).is_some()
            || find_unquoted(content.as_bytes(), b':').is_some()
        {
            self.pending.push_back(Event::ObjectStart { span });
            self.frames.push(Frame {
                content_depth: 0,
                kind: FrameKind::Object,
            });
            self.ensure_depth()?;
            self.object_member(content, depth, span, line)?;
        } else {
            let scalar = self.scalar(content.trim(), line, 1)?;
            self.pending.push_back(Event::Scalar {
                span,
                value: scalar,
            });
            self.root_complete = true;
        }
        Ok(())
    }

    fn close_for_line(
        &mut self,
        depth: usize,
        content: &str,
        span: Span,
        line: &Line,
    ) -> Result<(), DecodeError> {
        loop {
            let Some(frame) = self.frames.last() else {
                return Ok(());
            };
            let accepts_same_depth = match frame.kind {
                FrameKind::Object => true,
                FrameKind::Array { declared, observed } => {
                    observed < declared && list_marker(content)
                }
                FrameKind::Tabular {
                    declared, observed, ..
                } => observed < declared,
            };
            if depth < frame.content_depth || (depth == frame.content_depth && !accepts_same_depth)
            {
                self.close_frame(span, line)?;
                continue;
            }
            return Ok(());
        }
    }

    fn close_frame(&mut self, span: Span, line: &Line) -> Result<(), DecodeError> {
        let frame = self.frames.pop().expect("frame checked by caller");
        match frame.kind {
            FrameKind::Object => self.pending.push_back(Event::ObjectEnd { span }),
            FrameKind::Array { declared, observed }
            | FrameKind::Tabular {
                declared, observed, ..
            } => {
                if self.config.strict && declared != observed {
                    return Err(self.syntax(
                        line,
                        1,
                        &format!("array declared {declared} items but observed {observed}"),
                    ));
                }
                self.pending.push_back(Event::ArrayEnd {
                    span,
                    observed_count: observed,
                });
            }
        }
        if self.frames.is_empty() {
            self.root_complete = true;
        }
        Ok(())
    }

    fn object_member(
        &mut self,
        content: &str,
        depth: usize,
        span: Span,
        line: &Line,
    ) -> Result<(), DecodeError> {
        if let Some(header_start) = self.header_start(content) {
            let header = self.header(content, line)?;
            let key = header
                .key
                .clone()
                .ok_or_else(|| self.syntax(line, 1, "object array member requires a key"))?;
            self.token_limit(&key.value, line)?;
            self.pending.push_back(Event::Key {
                span,
                value: key.value,
                quoted: key.quoted,
            });
            self.emit_header(header, depth, span, line)?;
            debug_assert!(header_start <= content.len());
            return Ok(());
        }
        let colon = find_unquoted(content.as_bytes(), b':')
            .ok_or_else(|| self.syntax(line, 1, "object member is missing ':'"))?;
        let key = self.decode_key(content[..colon].trim(), line)?;
        self.token_limit(&key.value, line)?;
        self.pending.push_back(Event::Key {
            span,
            value: key.value,
            quoted: key.quoted,
        });
        let value = content[colon + 1..].trim();
        if value.is_empty() {
            self.pending.push_back(Event::ObjectStart { span });
            self.frames.push(Frame {
                content_depth: depth + 1,
                kind: FrameKind::Object,
            });
            self.ensure_depth()?;
        } else {
            let value = self.scalar(value, line, colon + 2)?;
            self.pending.push_back(Event::Scalar { span, value });
        }
        Ok(())
    }

    fn emit_header(
        &mut self,
        header: Header,
        depth: usize,
        span: Span,
        line: &Line,
    ) -> Result<(), DecodeError> {
        self.pending.push_back(Event::ArrayStart {
            span,
            declared_count: Some(header.declared),
        });
        if !header.fields.is_empty() {
            if !header.inline.is_empty() {
                return Err(self.syntax(line, 1, "tabular array header must end at ':'"));
            }
            if header.declared == 0 {
                self.pending.push_back(Event::ArrayEnd {
                    span,
                    observed_count: 0,
                });
                return Ok(());
            }
            self.frames.push(Frame {
                content_depth: depth + 1,
                kind: FrameKind::Tabular {
                    declared: header.declared,
                    observed: 0,
                    fields: header.fields.into(),
                    delimiter: header.delimiter,
                },
            });
            self.ensure_depth()?;
        } else if !header.inline.is_empty() {
            let tokens = split_delimited(&header.inline, header.delimiter, line, self)?;
            for token in &tokens {
                let value = self.scalar(token, line, 1)?;
                self.pending.push_back(Event::Scalar { span, value });
            }
            let observed = tokens.len() as u64;
            if self.config.strict && observed != header.declared {
                return Err(self.syntax(
                    line,
                    1,
                    &format!(
                        "array declared {} items but observed {observed}",
                        header.declared
                    ),
                ));
            }
            self.pending.push_back(Event::ArrayEnd {
                span,
                observed_count: observed,
            });
        } else if header.declared == 0 {
            self.pending.push_back(Event::ArrayEnd {
                span,
                observed_count: 0,
            });
        } else {
            self.frames.push(Frame {
                content_depth: depth + 1,
                kind: FrameKind::Array {
                    declared: header.declared,
                    observed: 0,
                },
            });
            self.ensure_depth()?;
        }
        Ok(())
    }

    fn list_item(
        &mut self,
        content: &str,
        depth: usize,
        span: Span,
        line: &Line,
    ) -> Result<(), DecodeError> {
        if !list_marker(content) {
            return Err(self.syntax(line, 1, "expanded array item must begin with '-'"));
        }
        let FrameKind::Array { declared, observed } =
            &mut self.frames.last_mut().expect("array frame").kind
        else {
            return Err(self.syntax(line, 1, "list item outside array"));
        };
        if *observed >= *declared && self.config.strict {
            return Err(self.syntax(line, 1, "array contains more items than declared"));
        }
        *observed += 1;
        let remainder = content[1..].trim_start();
        if remainder.is_empty() {
            self.pending.push_back(Event::ObjectStart { span });
            self.pending.push_back(Event::ObjectEnd { span });
        } else if remainder.starts_with('[') {
            let header = self.header(remainder, line)?;
            if header.key.is_some() {
                return Err(self.syntax(line, 1, "array item header must not contain a key"));
            }
            self.emit_header(header, depth, span, line)?;
        } else if self.header_start(remainder).is_some()
            || find_unquoted(remainder.as_bytes(), b':').is_some()
        {
            self.pending.push_back(Event::ObjectStart { span });
            self.frames.push(Frame {
                content_depth: depth + 1,
                kind: FrameKind::Object,
            });
            self.ensure_depth()?;
            self.object_member(remainder, depth + 1, span, line)?;
            if let Some(Frame {
                kind: FrameKind::Tabular { .. } | FrameKind::Array { .. },
                content_depth,
                ..
            }) = self.frames.last_mut()
            {
                *content_depth = depth + 2;
            }
        } else {
            let value = self.scalar(remainder, line, 2)?;
            self.pending.push_back(Event::Scalar { span, value });
        }
        Ok(())
    }

    fn tabular_row(&mut self, content: &str, span: Span, line: &Line) -> Result<(), DecodeError> {
        let (declared, observed, fields, delimiter) =
            match &self.frames.last().expect("tabular frame").kind {
                FrameKind::Tabular {
                    declared,
                    observed,
                    fields,
                    delimiter,
                } => (*declared, *observed, Arc::clone(fields), *delimiter),
                _ => return Err(self.syntax(line, 1, "tabular row outside array")),
            };
        if observed >= declared && self.config.strict {
            return Err(self.syntax(line, 1, "tabular array contains too many rows"));
        }
        let values = split_delimited(content, delimiter, line, self)?;
        if self.config.strict && values.len() != fields.len() {
            return Err(self.syntax(
                line,
                1,
                &format!(
                    "tabular row has {} values but schema declares {}",
                    values.len(),
                    fields.len()
                ),
            ));
        }
        self.pending.push_back(Event::ObjectStart { span });
        for (field, token) in fields.iter().zip(values.iter()) {
            self.pending.push_back(Event::Key {
                span,
                value: Arc::clone(&field.value),
                quoted: field.quoted,
            });
            let value = self.scalar(token, line, 1)?;
            self.pending.push_back(Event::Scalar { span, value });
        }
        self.pending.push_back(Event::ObjectEnd { span });
        if let FrameKind::Tabular { observed, .. } = &mut self.frames.last_mut().unwrap().kind {
            *observed += 1;
        }
        Ok(())
    }

    #[allow(
        clippy::too_many_lines,
        reason = "header grammar is parsed linearly with quoted-region awareness"
    )]
    fn header(&self, content: &str, line: &Line) -> Result<Header, DecodeError> {
        let start = self
            .header_start(content)
            .ok_or_else(|| self.syntax(line, 1, "invalid array header"))?;
        let close = find_closing(content.as_bytes(), start, b'[', b']')
            .ok_or_else(|| self.syntax(line, start + 1, "unterminated array header"))?;
        let key = if start == 0 {
            None
        } else {
            Some(self.decode_key(content[..start].trim(), line)?)
        };
        let mut declaration = &content[start + 1..close];
        let delimiter = match declaration.as_bytes().last() {
            Some(b'|') => {
                declaration = &declaration[..declaration.len() - 1];
                Delimiter::Pipe
            }
            Some(b'\t') => {
                declaration = &declaration[..declaration.len() - 1];
                Delimiter::Tab
            }
            Some(b',') => {
                declaration = &declaration[..declaration.len() - 1];
                Delimiter::Comma
            }
            _ => Delimiter::Comma,
        };
        if declaration.is_empty() || !declaration.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(self.syntax(line, start + 2, "array count must be an unsigned integer"));
        }
        let declared = declaration
            .parse::<u64>()
            .map_err(|_| self.resource("declared-count"))?;
        let mut cursor = close + 1;
        let mut fields = Vec::new();
        if content.as_bytes().get(cursor) == Some(&b'{') {
            let field_close = find_closing(content.as_bytes(), cursor, b'{', b'}')
                .ok_or_else(|| self.syntax(line, cursor + 1, "unterminated field list"))?;
            let field_text = &content[cursor + 1..field_close];
            fields = split_delimited(field_text, delimiter, line, self)?
                .into_iter()
                .map(|field| self.decode_key(&field, line))
                .collect::<Result<Vec<_>, _>>()?;
            if fields.is_empty() || fields.iter().any(|field| field.value.is_empty()) {
                return Err(self.syntax(line, cursor + 1, "tabular field list cannot be empty"));
            }
            let mut unique = std::collections::BTreeSet::new();
            if self.config.strict
                && !fields
                    .iter()
                    .all(|field| unique.insert(Arc::clone(&field.value)))
            {
                return Err(self.syntax(line, cursor + 1, "duplicate tabular field"));
            }
            cursor = field_close + 1;
        }
        if content.as_bytes().get(cursor) != Some(&b':') {
            return Err(self.syntax(line, cursor + 1, "array header must be followed by ':'"));
        }
        let inline = content[cursor + 1..].trim().to_owned();
        Ok(Header {
            key,
            declared,
            delimiter,
            fields,
            inline,
        })
    }

    #[allow(clippy::unused_self)]
    fn header_start(&self, content: &str) -> Option<usize> {
        find_unquoted(content.as_bytes(), b'[')
    }

    fn scalar(&self, token: &str, line: &Line, column: usize) -> Result<Scalar, DecodeError> {
        if token.len() > self.config.maximum_token_bytes {
            return Err(self.resource("token-bytes"));
        }
        if token.starts_with('"') {
            return Ok(Scalar::String(self.quoted(token, line, column)?.into()));
        }
        match token {
            "null" => return Ok(Scalar::Null),
            "true" => return Ok(Scalar::Bool(true)),
            "false" => return Ok(Scalar::Bool(false)),
            _ => {}
        }
        if !forbidden_leading_zero(token)
            && looks_numeric(token)
            && let Ok(number) = Number::parse(token)
        {
            return Ok(Scalar::Number(number));
        }
        Ok(Scalar::String(token.into()))
    }

    fn decode_key(&self, token: &str, line: &Line) -> Result<DecodedKey, DecodeError> {
        if token.starts_with('"') {
            Ok(DecodedKey {
                value: self.quoted(token, line, 1)?.into(),
                quoted: true,
            })
        } else if token.is_empty() {
            Err(self.syntax(line, 1, "object key cannot be empty"))
        } else {
            Ok(DecodedKey {
                value: token.into(),
                quoted: false,
            })
        }
    }

    fn quoted(&self, token: &str, line: &Line, column: usize) -> Result<String, DecodeError> {
        let bytes = token.as_bytes();
        if bytes.first() != Some(&b'"') {
            return Err(self.syntax(line, column, "quoted token must begin with a quote"));
        }
        let mut output = String::new();
        let mut index = 1;
        while index < bytes.len() {
            match bytes[index] {
                b'"' => {
                    if index + 1 != bytes.len() {
                        return Err(self.syntax(
                            line,
                            column + index,
                            "characters follow closing quote",
                        ));
                    }
                    return Ok(output);
                }
                b'\\' => {
                    index += 1;
                    let escaped = *bytes.get(index).ok_or_else(|| {
                        self.syntax(line, column + index, "unterminated string escape")
                    })?;
                    output.push(match escaped {
                        b'\\' => '\\',
                        b'"' => '"',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        _ => {
                            return Err(self.syntax(
                                line,
                                column + index,
                                "invalid TOON string escape",
                            ));
                        }
                    });
                }
                byte if byte < 0x20 && byte != b'\t' => {
                    return Err(self.syntax(
                        line,
                        column + index,
                        "unescaped control byte in string",
                    ));
                }
                _ => {
                    let tail = std::str::from_utf8(&bytes[index..]).map_err(|_| {
                        self.syntax(line, column + index, "invalid UTF-8 in string")
                    })?;
                    let character = tail.chars().next().expect("non-empty UTF-8 tail");
                    output.push(character);
                    index += character.len_utf8() - 1;
                }
            }
            index += 1;
        }
        Err(self.syntax(line, column, "unterminated quoted string"))
    }

    fn finish_document(&mut self) -> Result<(), DecodeError> {
        let position = Span::new(self.source, self.byte_offset, self.byte_offset);
        if self.started {
            while !self.frames.is_empty() {
                let synthetic = Line {
                    text: String::new(),
                    start: self.byte_offset,
                    number: self.line_number.max(1),
                };
                self.close_frame(position, &synthetic)?;
            }
        } else {
            self.started = true;
            self.pending
                .push_back(Event::DocumentStart { span: position });
            self.pending
                .push_back(Event::ObjectStart { span: position });
            self.pending.push_back(Event::ObjectEnd { span: position });
        }
        self.pending
            .push_back(Event::DocumentEnd { span: position });
        self.finished = true;
        Ok(())
    }

    fn indentation<'a>(&self, line: &'a Line) -> Result<(usize, &'a str), DecodeError> {
        let bytes = line.text.as_bytes();
        let mut spaces = 0;
        while bytes.get(spaces) == Some(&b' ') {
            spaces += 1;
        }
        if spaces == bytes.len() {
            return Ok((0, ""));
        }
        if bytes.get(spaces) == Some(&b'\t') {
            return Err(self.syntax(line, spaces + 1, "tabs are not allowed in indentation"));
        }
        if self.config.indent_size == 0 {
            if spaces != 0 {
                return Err(self.syntax(line, 1, "indentation is disabled"));
            }
            return Ok((0, &line.text));
        }
        if self.config.strict && spaces % self.config.indent_size != 0 {
            return Err(self.syntax(line, 1, "indentation is not a whole depth unit"));
        }
        Ok((spaces / self.config.indent_size, &line.text[spaces..]))
    }

    fn ensure_depth(&self) -> Result<(), DecodeError> {
        if self.frames.len() > self.config.maximum_depth {
            Err(self.resource("nesting-depth"))
        } else {
            Ok(())
        }
    }

    fn token_limit(&self, token: &str, _line: &Line) -> Result<(), DecodeError> {
        if token.len() > self.config.maximum_token_bytes {
            Err(self.resource("token-bytes"))
        } else {
            Ok(())
        }
    }

    fn line_span(&self, line: &Line) -> Span {
        Span::new(self.source, line.start, line.start + line.text.len() as u64)
    }

    #[allow(clippy::unused_self)]
    fn syntax(&self, line: &Line, column: usize, message: &str) -> DecodeError {
        DecodeError::Syntax {
            position: SourcePosition {
                byte: line.start + column.saturating_sub(1) as u64,
                line: line.number,
                column: column as u64,
            },
            message: message.into(),
        }
    }

    #[allow(clippy::unused_self)]
    fn resource(&self, resource: &str) -> DecodeError {
        DecodeError::Resource {
            resource: resource.into(),
        }
    }

    fn read_line(&mut self) -> Result<Option<Line>, DecodeError> {
        let mut bytes = Vec::new();
        let mut consumed = 0_u64;
        let mut exceeded = false;
        loop {
            let available = self.reader.fill_buf().map_err(|error| DecodeError::Io {
                message: error.to_string().into(),
            })?;
            if available.is_empty() {
                break;
            }
            let take = available
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(available.len(), |index| index + 1);
            if !exceeded {
                let remaining = self
                    .config
                    .maximum_line_bytes
                    .saturating_add(1)
                    .saturating_sub(bytes.len());
                bytes.extend_from_slice(&available[..take.min(remaining)]);
                exceeded = bytes.len() > self.config.maximum_line_bytes;
            }
            consumed = consumed.saturating_add(take as u64);
            let ended = available.get(take.saturating_sub(1)) == Some(&b'\n');
            self.reader.consume(take);
            if ended {
                break;
            }
        }
        if consumed == 0 && bytes.is_empty() {
            return Ok(None);
        }
        self.line_number += 1;
        let start = self.byte_offset;
        self.byte_offset = self.byte_offset.saturating_add(consumed);
        if exceeded {
            return Err(self.resource("line-bytes"));
        }
        if bytes.last() == Some(&b'\n') {
            bytes.pop();
        }
        if bytes.last() == Some(&b'\r') {
            if self.config.strict {
                let line = Line {
                    text: String::new(),
                    start,
                    number: self.line_number,
                };
                return Err(self.syntax(&line, 1, "CRLF is not valid canonical TOON"));
            }
            bytes.pop();
        }
        let text = String::from_utf8(bytes).map_err(|error| DecodeError::Syntax {
            position: SourcePosition {
                byte: start + error.utf8_error().valid_up_to() as u64,
                line: self.line_number,
                column: error.utf8_error().valid_up_to() as u64 + 1,
            },
            message: "invalid UTF-8".into(),
        })?;
        Ok(Some(Line {
            text,
            start,
            number: self.line_number,
        }))
    }
}

/// Decoder-or-consumer error from [`Decoder::decode_into`].
#[derive(Debug)]
pub enum DecodeIntoError<E> {
    /// TOON decoder failure.
    Decode(DecodeError),
    /// Event consumer failure.
    Consumer(E),
}

impl<E> From<DecodeError> for DecodeIntoError<E> {
    fn from(value: DecodeError) -> Self {
        Self::Decode(value)
    }
}

fn list_marker(content: &str) -> bool {
    content == "-" || content.starts_with("- ")
}

fn find_unquoted(bytes: &[u8], target: u8) -> Option<usize> {
    let mut quoted = false;
    let mut escaped = false;
    for (index, byte) in bytes.iter().copied().enumerate() {
        if escaped {
            escaped = false;
        } else if byte == b'\\' && quoted {
            escaped = true;
        } else if byte == b'"' {
            quoted = !quoted;
        } else if byte == target && !quoted {
            return Some(index);
        }
    }
    None
}

fn find_closing(bytes: &[u8], start: usize, open: u8, close: u8) -> Option<usize> {
    let mut quoted = false;
    let mut escaped = false;
    let mut depth = 0_usize;
    for (index, byte) in bytes.iter().copied().enumerate().skip(start) {
        if escaped {
            escaped = false;
        } else if byte == b'\\' && quoted {
            escaped = true;
        } else if byte == b'"' {
            quoted = !quoted;
        } else if !quoted && byte == open {
            depth += 1;
        } else if !quoted && byte == close {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn split_delimited<R: BufRead>(
    text: &str,
    delimiter: Delimiter,
    line: &Line,
    decoder: &Decoder<R>,
) -> Result<Vec<String>, DecodeError> {
    if text.is_empty() {
        return Ok(Vec::new());
    }
    let bytes = text.as_bytes();
    let mut values = Vec::new();
    let mut start = 0;
    let mut quoted = false;
    let mut escaped = false;
    for (index, byte) in bytes.iter().copied().enumerate() {
        if escaped {
            escaped = false;
        } else if byte == b'\\' && quoted {
            escaped = true;
        } else if byte == b'"' {
            quoted = !quoted;
        } else if byte == delimiter.byte() && !quoted {
            values.push(text[start..index].trim().to_owned());
            start = index + 1;
        }
    }
    if quoted || escaped {
        return Err(decoder.syntax(line, 1, "unterminated quote in delimited values"));
    }
    values.push(text[start..].trim().to_owned());
    Ok(values)
}

fn forbidden_leading_zero(token: &str) -> bool {
    let digits = token.strip_prefix('-').unwrap_or(token);
    digits.len() > 1
        && digits.starts_with('0')
        && digits.as_bytes().get(1).is_some_and(u8::is_ascii_digit)
}

fn looks_numeric(token: &str) -> bool {
    token
        .as_bytes()
        .first()
        .is_some_and(|byte| byte.is_ascii_digit() || *byte == b'-')
}

#[cfg(test)]
mod tests {
    use std::io::{BufReader, Cursor};

    use tq_core::SourceId;

    use crate::{DecoderConfig, Event};

    use super::Decoder;

    fn events(input: &[u8]) -> Result<Vec<Event>, crate::DecodeError> {
        let mut decoder = Decoder::new(
            BufReader::with_capacity(3, Cursor::new(input)),
            SourceId::new(1),
            DecoderConfig::default(),
        );
        let mut events = Vec::new();
        while let Some(event) = decoder.next_event()? {
            events.push(event);
        }
        Ok(events)
    }

    #[test]
    fn event_contract_covers_object_inline_tabular_and_expanded_arrays() {
        for input in [
            "name: Ada\nage: 30",
            "tags[3]: admin,ops,dev",
            "items[2]{id,name}:\n  1,Ada\n  2,Bob",
            "items[2]:\n  - id: 1\n    name: A\n  - [2]: x,y",
        ] {
            let decoded =
                events(input.as_bytes()).unwrap_or_else(|error| panic!("{input}: {error}"));
            assert!(matches!(decoded.first(), Some(Event::DocumentStart { .. })));
            assert!(matches!(decoded.last(), Some(Event::DocumentEnd { .. })));
        }
    }

    #[test]
    fn hostile_limits_and_syntax_fail_without_proportional_allocation() {
        assert!(events(b"items[999999999999999999999999]:").is_err());
        assert!(events(b"items[2]: a").is_err());
        assert!(events(b"name: \"bad\\x\"").is_err());
        assert!(events(b"a:\n   b: 1").is_err());
        assert!(events(b"\xff").is_err());

        let config = DecoderConfig {
            maximum_line_bytes: 4,
            ..DecoderConfig::default()
        };
        let mut decoder = Decoder::new(
            BufReader::new(Cursor::new(b"12345\n")),
            SourceId::new(0),
            config,
        );
        assert!(decoder.next_event().is_err());
    }

    #[test]
    fn event_stream_is_invariant_across_reader_chunk_boundaries() {
        let input = b"meta:\n  ok: true\nitems[3]{id,name}:\n  1,Ada\n  2,\"B, B\"\n  3,Cyd\n";
        let expected = events(input).unwrap();
        for capacity in 1..=input.len() + 2 {
            let mut decoder = Decoder::new(
                BufReader::with_capacity(capacity, Cursor::new(input)),
                SourceId::new(1),
                DecoderConfig::default(),
            );
            let mut actual = Vec::new();
            while let Some(event) = decoder.next_event().unwrap() {
                actual.push(event);
            }
            assert_eq!(actual, expected, "reader capacity {capacity}");
        }
    }
}
