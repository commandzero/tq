//! Bounded line-oriented TOON-to-event decoder.

use std::{collections::VecDeque, io::BufRead, sync::Arc};

use tq_core::{Number, SourceId, SourcePosition, Span};

use crate::{DecodeError, DecoderCapabilities, DecoderConfig, Event, Scalar};

/// Incremental decoder retaining only one physical line, active container
/// state, tabular schemas, and pending events.
#[derive(Debug)]
pub struct Decoder<R> {
    reader: R,
    config: DecoderConfig,
    source: SourceId,
    pending: VecDeque<Event>,
    frames: Vec<Frame>,
    event_depth: usize,
    byte_offset: u64,
    line_number: u64,
    started: bool,
    root_complete: bool,
    pending_blank: Option<(u64, u64)>,
    finished: bool,
}

#[derive(Debug)]
struct Frame {
    content_depth: usize,
    kind: FrameKind,
}

#[derive(Debug)]
enum FrameKind {
    Object {
        keys: std::collections::HashSet<Arc<str>>,
        name_bytes: usize,
    },
    Array {
        declared: u64,
        observed: u64,
    },
    Tabular {
        declared: u64,
        observed: u64,
        fields: Arc<Schema>,
        delimiter: Delimiter,
    },
    Keyed {
        declared: u64,
        observed: u64,
        fields: Arc<Schema>,
        delimiter: Delimiter,
        keys: std::collections::HashSet<Arc<str>>,
        name_bytes: usize,
    },
}

#[derive(Debug)]
struct Field {
    key: DecodedKey,
    children: Vec<Field>,
}

#[derive(Debug)]
struct Schema {
    fields: Vec<Field>,
    field_count: usize,
    name_bytes: usize,
    leaf_width: usize,
}

impl Field {
    fn statistics(&self) -> (usize, usize, usize) {
        let mut count = 1;
        let mut names = self.key.value.len();
        let mut leaves = usize::from(self.children.is_empty());
        for child in &self.children {
            let (child_count, child_names, child_leaves) = child.statistics();
            count += child_count;
            names += child_names;
            leaves += child_leaves;
        }
        (count, names, leaves)
    }
}

impl Schema {
    fn new(fields: Vec<Field>) -> Self {
        let (mut field_count, mut name_bytes, mut leaf_width) = (0, 0, 0);
        for field in &fields {
            let (count, names, leaves) = field.statistics();
            field_count += count;
            name_bytes += names;
            leaf_width += leaves;
        }
        Self {
            fields,
            field_count,
            name_bytes,
            leaf_width,
        }
    }
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
    fields: Vec<Field>,
    inline: String,
    keyed: bool,
}

#[derive(Clone, Debug)]
struct DecodedKey {
    value: Arc<str>,
    quoted: bool,
}

impl<R: BufRead> Decoder<R> {
    /// Structural behavior known before input consumption.
    #[must_use]
    pub const fn capabilities() -> DecoderCapabilities {
        DecoderCapabilities::strict_toon()
    }

    /// Creates a strict bounded decoder over a buffered reader.
    #[must_use]
    pub fn new(reader: R, source: SourceId, config: DecoderConfig) -> Self {
        Self {
            reader,
            config,
            source,
            pending: VecDeque::new(),
            frames: Vec::new(),
            event_depth: 0,
            byte_offset: 0,
            line_number: 0,
            started: false,
            root_complete: false,
            pending_blank: None,
            finished: false,
        }
    }

    fn emit(&mut self, event: Event) -> Result<(), DecodeError> {
        match &event {
            Event::ObjectStart { .. } | Event::ArrayStart { .. } => {
                if self.event_depth >= self.config.maximum_depth {
                    return Err(self.resource("depth"));
                }
                self.event_depth += 1;
            }
            Event::ObjectEnd { .. } | Event::ArrayEnd { .. } => {
                self.event_depth -= 1;
            }
            _ => {}
        }
        self.pending.push_back(event);
        Ok(())
    }

    fn register_key(
        &mut self,
        key: &DecodedKey,
        line: &Line,
        duplicate: &'static str,
    ) -> Result<(), DecodeError> {
        let (FrameKind::Object { keys, .. } | FrameKind::Keyed { keys, .. }) =
            &self.frames.last().expect("object frame").kind
        else {
            return Err(self.syntax(line, 1, "key outside object"));
        };
        if keys.contains(&key.value) {
            return if self.config.strict {
                Err(self.syntax(line, 1, duplicate))
            } else {
                Ok(())
            };
        }
        self.validate_additional_budget(1, key.value.len())?;
        match &mut self.frames.last_mut().expect("object frame").kind {
            FrameKind::Object { keys, name_bytes }
            | FrameKind::Keyed {
                keys, name_bytes, ..
            } => {
                keys.insert(Arc::clone(&key.value));
                *name_bytes += key.value.len();
            }
            _ => unreachable!("object frame checked"),
        }
        Ok(())
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
        let candidate = if line.number == 1 {
            line.text.strip_prefix('\u{feff}').unwrap_or(&line.text)
        } else {
            &line.text
        };
        if candidate.trim_start_matches(' ').starts_with('#') {
            return Ok(());
        }
        let (depth, content) = self.indentation(line)?;
        if content.is_empty() {
            if self.config.strict
                && self.frames.iter().any(|frame| match &frame.kind {
                    FrameKind::Array { observed, .. }
                    | FrameKind::Tabular { observed, .. }
                    | FrameKind::Keyed { observed, .. } => *observed > 0,
                    FrameKind::Object { .. } => false,
                })
            {
                self.pending_blank.get_or_insert((line.start, line.number));
            }
            return Ok(());
        }
        let span = self.line_span(line);
        if !self.started {
            if depth != 0 {
                return Err(self.syntax(line, 1, "root value must begin at depth zero"));
            }
            self.started = true;
            self.emit(Event::DocumentStart { span })?;
            self.start_root(content, depth, line)?;
            return Ok(());
        }
        if self.root_complete {
            return Err(self.syntax(line, 1, "unexpected content after root value"));
        }

        self.close_for_line(depth, content, span, line)?;
        if let Some((start, number)) = self.pending_blank.take()
            && self.frames.iter().any(|frame| {
                matches!(
                    frame.kind,
                    FrameKind::Array { .. } | FrameKind::Tabular { .. } | FrameKind::Keyed { .. }
                )
            })
        {
            let blank = Line {
                text: String::new(),
                start,
                number,
            };
            return Err(self.syntax(&blank, 1, "blank line inside array"));
        }
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
        if !self.config.strict
            && matches!(frame.kind, FrameKind::Keyed { .. })
            && find_unquoted(content.as_bytes(), b':').is_none()
        {
            return Ok(());
        }
        match frame.kind {
            FrameKind::Object { .. } => self.object_member(content, depth, span, line),
            FrameKind::Array { .. } => self.list_item(content, depth, span, line),
            FrameKind::Tabular { .. } => self.tabular_row(content, span, line),
            FrameKind::Keyed { .. } => self.keyed_row(content, span, line),
        }
    }

    fn start_root(&mut self, content: &str, depth: usize, line: &Line) -> Result<(), DecodeError> {
        let span = self.line_span(line);
        if content == "[]" {
            self.emit(Event::ArrayStart {
                span,
                declared_count: Some(0),
            })?;
            self.emit(Event::ArrayEnd {
                span,
                observed_count: 0,
            })?;
            self.root_complete = true;
        } else if content.starts_with('[') {
            match self.header(content, line) {
                Ok(header)
                    if !self.config.strict
                        && ((!header.fields.is_empty() && !header.inline.is_empty())
                            || (header.keyed && header.fields.is_empty())) =>
                {
                    self.start_root_object(content, depth, span, line)?;
                }
                Ok(header) if header.key.is_none() => {
                    self.emit_header(header, depth, span, line)?;
                }
                Ok(_) => {
                    return Err(self.syntax(line, 1, "root array header cannot contain a key"));
                }
                Err(error)
                    if !self.config.strict
                        && find_unquoted(content.as_bytes(), b':').is_some()
                        && header_fallback_allowed(&error) =>
                {
                    self.start_root_object(content, depth, span, line)?;
                }
                Err(error) => return Err(error),
            }
        } else if self.header_start(content).is_some()
            || find_unquoted(content.as_bytes(), b':').is_some()
        {
            self.start_root_object(content, depth, span, line)?;
        } else {
            let scalar = self.scalar(trim_spaces(content), line, 1)?;
            self.emit(Event::Scalar {
                span,
                value: scalar,
            })?;
            self.root_complete = true;
        }
        Ok(())
    }

    fn start_root_object(
        &mut self,
        content: &str,
        depth: usize,
        span: Span,
        line: &Line,
    ) -> Result<(), DecodeError> {
        self.emit(Event::ObjectStart { span })?;
        self.frames.push(Frame {
            content_depth: 0,
            kind: FrameKind::Object {
                keys: std::collections::HashSet::new(),
                name_bytes: 0,
            },
        });
        self.ensure_depth()?;
        self.object_member(content, depth, span, line)
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
                FrameKind::Object { .. } | FrameKind::Keyed { .. } => true,
                FrameKind::Array { .. } => list_marker(content),
                FrameKind::Tabular { delimiter, .. } => {
                    let colon = find_unquoted(content.as_bytes(), b':');
                    let cell = find_unquoted(content.as_bytes(), delimiter.byte());
                    colon.is_none_or(|at| cell.is_some_and(|cell_at| at >= cell_at))
                }
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
            FrameKind::Object { .. } => self.emit(Event::ObjectEnd { span })?,
            FrameKind::Keyed {
                declared, observed, ..
            } => {
                if self.config.strict && declared != observed {
                    return Err(self.syntax(
                        line,
                        1,
                        &format!(
                            "keyed object declared {declared} entries but observed {observed}"
                        ),
                    ));
                }
                self.emit(Event::ObjectEnd { span })?;
            }
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
                self.emit(Event::ArrayEnd {
                    span,
                    observed_count: observed,
                })?;
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
            let header = match self.header(content, line) {
                Ok(header) => header,
                Err(error)
                    if !self.config.strict
                        && find_unquoted(content.as_bytes(), b':').is_some()
                        && header_fallback_allowed(&error) =>
                {
                    return self.object_member_key_value(content, depth, span, line);
                }
                Err(error) => return Err(error),
            };
            if !self.config.strict
                && (header.key.is_none()
                    || (header.keyed && (header.fields.is_empty() || !header.inline.is_empty()))
                    || (!header.fields.is_empty() && !header.inline.is_empty()))
            {
                return self.object_member_key_value(content, depth, span, line);
            }
            let key = header
                .key
                .clone()
                .ok_or_else(|| self.syntax(line, 1, "object array member requires a key"))?;
            self.token_limit(&key.value, line)?;
            self.register_key(&key, line, "duplicate object key")?;
            self.emit(Event::Key {
                span,
                value: Arc::clone(&key.value),
                quoted: key.quoted,
            })?;
            self.emit_header(header, depth, span, line)?;
            debug_assert!(header_start <= content.len());
            return Ok(());
        }
        self.object_member_key_value(content, depth, span, line)
    }

    fn object_member_key_value(
        &mut self,
        content: &str,
        depth: usize,
        span: Span,
        line: &Line,
    ) -> Result<(), DecodeError> {
        let colon = find_unquoted(content.as_bytes(), b':')
            .ok_or_else(|| self.syntax(line, 1, "object member is missing ':'"))?;
        let key = self.decode_key(trim_spaces(&content[..colon]), line)?;
        self.token_limit(&key.value, line)?;
        self.register_key(&key, line, "duplicate object key")?;
        self.emit(Event::Key {
            span,
            value: Arc::clone(&key.value),
            quoted: key.quoted,
        })?;
        let value = trim_spaces(&content[colon + 1..]);
        if value == "[]" {
            self.emit(Event::ArrayStart {
                span,
                declared_count: Some(0),
            })?;
            self.emit(Event::ArrayEnd {
                span,
                observed_count: 0,
            })?;
        } else if value.is_empty() {
            self.emit(Event::ObjectStart { span })?;
            self.frames.push(Frame {
                content_depth: depth + 1,
                kind: FrameKind::Object {
                    keys: std::collections::HashSet::new(),
                    name_bytes: 0,
                },
            });
            self.ensure_depth()?;
        } else {
            let value = self.scalar(value, line, colon + 2)?;
            self.emit(Event::Scalar { span, value })?;
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
        if header.keyed && (header.fields.is_empty() || !header.inline.is_empty()) {
            return Err(self.syntax(line, 1, "keyed header requires fields and no inline value"));
        }
        if !header.keyed && !header.fields.is_empty() && !header.inline.is_empty() {
            return Err(self.syntax(line, 1, "tabular array header must end at ':'"));
        }
        let schema = Schema::new(header.fields);
        self.validate_additional_budget(schema.field_count, schema.name_bytes)?;
        if header.keyed {
            self.emit(Event::ObjectStart { span })?;
            if header.declared == 0 && self.config.strict {
                self.emit(Event::ObjectEnd { span })?;
            } else {
                self.frames.push(Frame {
                    content_depth: depth + 1,
                    kind: FrameKind::Keyed {
                        declared: header.declared,
                        observed: 0,
                        fields: Arc::new(schema),
                        delimiter: header.delimiter,
                        keys: std::collections::HashSet::new(),
                        name_bytes: 0,
                    },
                });
                self.ensure_depth()?;
            }
            return Ok(());
        }
        self.emit(Event::ArrayStart {
            span,
            declared_count: Some(header.declared),
        })?;
        if !schema.fields.is_empty() {
            if header.declared == 0 && self.config.strict {
                self.emit(Event::ArrayEnd {
                    span,
                    observed_count: 0,
                })?;
                return Ok(());
            }
            self.frames.push(Frame {
                content_depth: depth + 1,
                kind: FrameKind::Tabular {
                    declared: header.declared,
                    observed: 0,
                    fields: Arc::new(schema),
                    delimiter: header.delimiter,
                },
            });
            self.ensure_depth()?;
        } else if !header.inline.is_empty() {
            let tokens = split_delimited(&header.inline, header.delimiter, line, self)?;
            for token in &tokens {
                let value = self.scalar(token, line, 1)?;
                self.emit(Event::Scalar { span, value })?;
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
            self.emit(Event::ArrayEnd {
                span,
                observed_count: observed,
            })?;
        } else if header.declared == 0 && self.config.strict {
            self.emit(Event::ArrayEnd {
                span,
                observed_count: 0,
            })?;
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
        let remainder = content[1..].trim_start_matches(' ');
        if remainder == "[]" {
            self.emit(Event::ArrayStart {
                span,
                declared_count: Some(0),
            })?;
            self.emit(Event::ArrayEnd {
                span,
                observed_count: 0,
            })?;
        } else if remainder.is_empty() {
            self.emit(Event::ObjectStart { span })?;
            self.emit(Event::ObjectEnd { span })?;
        } else if remainder.starts_with('[') {
            let header = self.header(remainder, line)?;
            if header.key.is_some() {
                return Err(self.syntax(line, 1, "array item header must not contain a key"));
            }
            if header.keyed || !header.fields.is_empty() {
                return Err(self.syntax(
                    line,
                    1,
                    "anonymous list-item tabular headers are forbidden",
                ));
            }
            self.emit_header(header, depth, span, line)?;
        } else if self.header_start(remainder).is_some()
            || find_unquoted(remainder.as_bytes(), b':').is_some()
        {
            self.emit(Event::ObjectStart { span })?;
            self.frames.push(Frame {
                content_depth: depth + 1,
                kind: FrameKind::Object {
                    keys: std::collections::HashSet::new(),
                    name_bytes: 0,
                },
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
            self.emit(Event::Scalar { span, value })?;
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
        let expected = fields.leaf_width;
        if self.config.strict && values.len() != expected {
            return Err(self.syntax(
                line,
                1,
                &format!(
                    "tabular row has {} values but schema declares {expected}",
                    values.len()
                ),
            ));
        }
        self.emit(Event::ObjectStart { span })?;
        let mut index = 0;
        self.emit_field_values(&fields.fields, &values, &mut index, span, line)?;
        self.emit(Event::ObjectEnd { span })?;
        if let FrameKind::Tabular { observed, .. } = &mut self.frames.last_mut().unwrap().kind {
            *observed += 1;
        }
        Ok(())
    }

    fn emit_field_values(
        &mut self,
        fields: &[Field],
        values: &[String],
        index: &mut usize,
        span: Span,
        line: &Line,
    ) -> Result<(), DecodeError> {
        for field in fields {
            if field.children.is_empty() {
                let Some(token) = values.get(*index) else {
                    *index += 1;
                    continue;
                };
                self.emit(Event::Key {
                    span,
                    value: Arc::clone(&field.key.value),
                    quoted: field.key.quoted,
                })?;
                let value = self.scalar(token, line, 1)?;
                self.emit(Event::Scalar { span, value })?;
                *index += 1;
            } else {
                self.emit(Event::Key {
                    span,
                    value: Arc::clone(&field.key.value),
                    quoted: field.key.quoted,
                })?;
                self.emit(Event::ObjectStart { span })?;
                self.emit_field_values(&field.children, values, index, span, line)?;
                self.emit(Event::ObjectEnd { span })?;
            }
        }
        Ok(())
    }

    fn validate_additional_budget(
        &self,
        mut field_count: usize,
        mut name_bytes: usize,
    ) -> Result<(), DecodeError> {
        for frame in &self.frames {
            match &frame.kind {
                FrameKind::Object {
                    keys,
                    name_bytes: retained,
                } => {
                    field_count = field_count.saturating_add(keys.len());
                    name_bytes = name_bytes.saturating_add(*retained);
                }
                FrameKind::Keyed {
                    keys,
                    fields,
                    name_bytes: retained,
                    ..
                } => {
                    field_count = field_count
                        .saturating_add(keys.len())
                        .saturating_add(fields.field_count);
                    name_bytes = name_bytes
                        .saturating_add(*retained)
                        .saturating_add(fields.name_bytes);
                }
                FrameKind::Tabular { fields, .. } => {
                    field_count = field_count.saturating_add(fields.field_count);
                    name_bytes = name_bytes.saturating_add(fields.name_bytes);
                }
                FrameKind::Array { .. } => {}
            }
        }
        if field_count > self.config.maximum_fields {
            return Err(self.resource("fields"));
        }
        if name_bytes > self.config.maximum_name_bytes {
            return Err(self.resource("name-bytes"));
        }
        Ok(())
    }
    fn keyed_row(&mut self, content: &str, span: Span, line: &Line) -> Result<(), DecodeError> {
        let colon = find_unquoted(content.as_bytes(), b':')
            .ok_or_else(|| self.syntax(line, 1, "keyed entry is missing ':'"))?;
        let (declared, observed, fields, delimiter) =
            match &self.frames.last().expect("keyed frame").kind {
                FrameKind::Keyed {
                    declared,
                    observed,
                    fields,
                    delimiter,
                    ..
                } => (*declared, *observed, Arc::clone(fields), *delimiter),
                _ => return Err(self.syntax(line, 1, "keyed entry outside keyed object")),
            };
        if observed >= declared && self.config.strict {
            return Err(self.syntax(line, 1, "keyed object contains too many entries"));
        }
        let key = self.decode_key(trim_spaces(&content[..colon]), line)?;
        self.token_limit(&key.value, line)?;
        self.register_key(&key, line, "duplicate keyed entry")?;
        let values = split_delimited(trim_spaces(&content[colon + 1..]), delimiter, line, self)?;
        let expected = fields.leaf_width;
        if self.config.strict && values.len() != expected {
            return Err(self.syntax(
                line,
                colon + 2,
                "keyed entry row width does not match schema",
            ));
        }
        self.emit(Event::Key {
            span,
            value: key.value,
            quoted: key.quoted,
        })?;
        self.emit(Event::ObjectStart { span })?;
        let mut index = 0;
        self.emit_field_values(&fields.fields, &values, &mut index, span, line)?;
        self.emit(Event::ObjectEnd { span })?;
        if let FrameKind::Keyed { observed, .. } = &mut self.frames.last_mut().unwrap().kind {
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
        if start > 0
            && content[..start]
                .chars()
                .last()
                .is_some_and(char::is_whitespace)
        {
            return Err(self.syntax(line, start + 1, "whitespace before array header bracket"));
        }
        let close = find_closing(content.as_bytes(), start, b'[', b']')
            .ok_or_else(|| self.syntax(line, start + 1, "unterminated array header"))?;
        let key = if start == 0 {
            None
        } else {
            Some(self.decode_key(&content[..start], line)?)
        };
        let mut declaration = &content[start + 1..close];
        let raw_declaration = declaration;
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
                return Err(self.syntax(line, start + 2, "comma is the implicit delimiter"));
            }
            _ => Delimiter::Comma,
        };
        let keyed = declaration.ends_with(':');
        if keyed {
            declaration = &declaration[..declaration.len() - 1];
        }
        if let Some(colon) = raw_declaration.find(':') {
            let suffix = &raw_declaration[colon + 1..];
            if !keyed
                || colon == 0
                || !raw_declaration[..colon]
                    .bytes()
                    .all(|byte| byte.is_ascii_digit())
                || !matches!(suffix, "" | "|" | "\t")
            {
                return Err(self.syntax(line, start + 2, "malformed keyed header marker"));
            }
        }
        if declaration.is_empty()
            || !declaration.bytes().all(|byte| byte.is_ascii_digit())
            || (declaration.len() > 1 && declaration.starts_with('0'))
        {
            return Err(self.syntax(
                line,
                start + 2,
                "array count must be a canonical unsigned integer",
            ));
        }
        let declared = declaration
            .parse::<u64>()
            .map_err(|_| self.resource("declared-count"))?;
        let mut cursor = close + 1;
        let mut fields = Vec::new();
        if content.as_bytes().get(cursor) == Some(&b'{') {
            let field_close = find_closing(content.as_bytes(), cursor, b'{', b'}')
                .ok_or_else(|| self.syntax(line, cursor + 1, "unterminated field list"))?;
            fields = self.parse_fields(&content[cursor + 1..field_close], delimiter, line)?;
            if fields.is_empty() {
                return Err(self.syntax(line, cursor + 1, "tabular field list cannot be empty"));
            }
            cursor = field_close + 1;
        }
        if content.as_bytes().get(cursor) != Some(&b':') {
            return Err(self.syntax(line, cursor + 1, "array header must be followed by ':'"));
        }
        Ok(Header {
            key,
            declared,
            delimiter,
            fields,
            inline: trim_spaces(&content[cursor + 1..]).to_owned(),
            keyed,
        })
    }

    fn parse_fields(
        &self,
        text: &str,
        delimiter: Delimiter,
        line: &Line,
    ) -> Result<Vec<Field>, DecodeError> {
        let (mut field_count, mut name_bytes) = (0, 0);
        self.parse_fields_inner(text, delimiter, line, 1, &mut field_count, &mut name_bytes)
    }

    fn parse_fields_inner(
        &self,
        text: &str,
        delimiter: Delimiter,
        line: &Line,
        depth: usize,
        field_count: &mut usize,
        name_bytes: &mut usize,
    ) -> Result<Vec<Field>, DecodeError> {
        if depth > self.config.maximum_depth {
            return Err(self.resource("depth"));
        }
        let bytes = text.as_bytes();
        let mut ranges = Vec::new();
        let (mut start, mut brace_depth, mut quoted, mut escaped) = (0, 0usize, false, false);
        for (index, byte) in bytes.iter().copied().enumerate() {
            if quoted {
                if escaped {
                    escaped = false;
                } else if byte == b'\\' {
                    escaped = true;
                } else if byte == b'"' {
                    quoted = false;
                }
                continue;
            }
            match byte {
                b'"' => quoted = true,
                b'{' => brace_depth += 1,
                b'}' => brace_depth = brace_depth.saturating_sub(1),
                value if value == delimiter.byte() && brace_depth == 0 => {
                    if ranges.len() >= self.config.maximum_fields {
                        return Err(self.resource("fields"));
                    }
                    ranges.push(&text[start..index]);
                    start = index + 1;
                }
                b',' | b'|' | b'\t' if byte != delimiter.byte() && self.config.strict => {
                    return Err(self.syntax(
                        line,
                        index + 1,
                        "field list delimiter does not match header",
                    ));
                }
                _ => {}
            }
        }
        if ranges.len() >= self.config.maximum_fields {
            return Err(self.resource("fields"));
        }
        ranges.push(&text[start..]);
        let mut fields = Vec::new();
        let mut unique = std::collections::HashSet::new();
        for range in ranges {
            let entry = trim_spaces(range);
            let nested = find_unquoted(entry.as_bytes(), b'{');
            let (name, nested_fields) = if let Some(open) = nested {
                let close = find_closing(entry.as_bytes(), open, b'{', b'}').ok_or_else(|| {
                    self.syntax(line, open + 1, "unterminated nested field group")
                })?;
                if close + 1 != entry.len() {
                    return Err(self.syntax(
                        line,
                        close + 2,
                        "characters follow nested field group",
                    ));
                }
                (&entry[..open], Some((open, &entry[open + 1..close])))
            } else {
                (entry, None)
            };
            let key = self.decode_key(trim_spaces(name), line)?;
            self.token_limit(&key.value, line)?;
            *field_count = field_count
                .checked_add(1)
                .ok_or_else(|| self.resource("fields"))?;
            *name_bytes = name_bytes
                .checked_add(key.value.len())
                .ok_or_else(|| self.resource("name-bytes"))?;
            self.validate_additional_budget(*field_count, *name_bytes)?;
            if !unique.insert(Arc::clone(&key.value)) && self.config.strict {
                return Err(self.syntax(line, 1, "duplicate field in group"));
            }
            let children = if let Some((open, nested_fields)) = nested_fields {
                let children = self.parse_fields_inner(
                    nested_fields,
                    delimiter,
                    line,
                    depth + 1,
                    field_count,
                    name_bytes,
                )?;
                if children.is_empty() {
                    return Err(self.syntax(line, open + 1, "nested field group cannot be empty"));
                }
                children
            } else {
                Vec::new()
            };
            fields.push(Field { key, children });
        }
        Ok(fields)
    }

    #[allow(clippy::unused_self)]
    fn header_start(&self, content: &str) -> Option<usize> {
        let bytes = content.as_bytes();
        let bracket = find_unquoted(bytes, b'[')?;
        let colon = find_unquoted(bytes, b':');
        if colon.is_some_and(|colon| colon < bracket) {
            None
        } else {
            Some(bracket)
        }
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
        if !forbidden_leading_zero(token) && looks_numeric(token) {
            match Number::parse(token) {
                Ok(number) => return Ok(Scalar::Number(number)),
                Err(tq_core::NumberError::Invalid) => {}
                Err(error) => {
                    return Err(self.resource(&format!("numeric-envelope: {error}")));
                }
            }
        }
        Ok(Scalar::String(token.into()))
    }

    fn decode_key(&self, token: &str, line: &Line) -> Result<DecodedKey, DecodeError> {
        if token.len() > self.config.maximum_token_bytes
            || token.len() > self.config.maximum_name_bytes
        {
            return Err(self.resource("name-bytes"));
        }
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
                    if escaped == b'u' {
                        let end = index + 5;
                        let digits = bytes.get(index + 1..end).ok_or_else(|| {
                            self.syntax(line, column + index, "incomplete Unicode escape")
                        })?;
                        let text = std::str::from_utf8(digits).expect("hex escape is ASCII");
                        let code = u32::from_str_radix(text, 16).map_err(|_| {
                            self.syntax(line, column + index, "invalid Unicode escape")
                        })?;
                        let character = char::from_u32(code).ok_or_else(|| {
                            self.syntax(line, column + index, "surrogate escape is invalid")
                        })?;
                        output.push(character);
                        index += 4;
                    } else {
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
            self.emit(Event::DocumentStart { span: position })?;
            self.emit(Event::ObjectStart { span: position })?;
            self.emit(Event::ObjectEnd { span: position })?;
        }
        self.emit(Event::DocumentEnd { span: position })?;
        self.finished = true;
        Ok(())
    }

    fn indentation<'a>(&self, line: &'a Line) -> Result<(usize, &'a str), DecodeError> {
        let bom = usize::from(line.number == 1 && line.text.starts_with('\u{feff}'));
        let prefix = if bom == 1 { '\u{feff}'.len_utf8() } else { 0 };
        let bytes = line.text.as_bytes();
        let mut spaces = prefix;
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
            if spaces != prefix {
                return Err(self.syntax(line, 1, "indentation is disabled"));
            }
            return Ok((0, &line.text[prefix..]));
        }
        let indentation = spaces - prefix;
        if self.config.strict && indentation % self.config.indent_size != 0 {
            return Err(self.syntax(line, 1, "indentation is not a whole depth unit"));
        }
        Ok((indentation / self.config.indent_size, &line.text[spaces..]))
    }

    fn ensure_depth(&self) -> Result<(), DecodeError> {
        if self.frames.len() > self.config.maximum_depth {
            Err(self.resource("depth"))
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
                byte: line.start
                    + usize::from(line.number == 1 && line.text.starts_with('\u{feff}')) as u64 * 3
                    + column.saturating_sub(1) as u64,
                line: line.number,
                column: column as u64
                    + usize::from(line.number == 1 && line.text.starts_with('\u{feff}')) as u64,
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

fn header_fallback_allowed(error: &DecodeError) -> bool {
    let DecodeError::Syntax { message, .. } = error else {
        return false;
    };
    !["escape", "quoted", "quote", "UTF-8", "surrogate", "string"]
        .iter()
        .any(|marker| message.contains(marker))
}

fn trim_spaces(text: &str) -> &str {
    text.trim_matches(' ')
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
            values.push(trim_spaces(&text[start..index]).to_owned());
            start = index + 1;
        }
    }
    if quoted || escaped {
        return Err(decoder.syntax(line, 1, "unterminated quote in delimited values"));
    }
    values.push(trim_spaces(&text[start..]).to_owned());
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

    #[test]
    fn recursive_and_keyed_headers_reconstruct_depth_first_objects() {
        let nested = events(b"people[1]{name,profile{city,zip}}:\n  Ada,Paris,75000").unwrap();
        let keys = nested
            .iter()
            .filter_map(|event| match event {
                Event::Key { value, .. } => Some(value.as_ref()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(keys, ["people", "name", "profile", "city", "zip"]);

        let keyed = events(b"[2:]{age}:\n  alice: 30\n  bob: 40").unwrap();
        let keys = keyed
            .iter()
            .filter_map(|event| match event {
                Event::Key { value, .. } => Some(value.as_ref()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(keys, ["alice", "age", "bob", "age"]);
    }

    #[test]
    fn lexical_comments_crlf_bom_and_control_escapes_are_accepted() {
        let decoded = events(b"\xef\xbb\xbf# comment\r\nname: \"\\u0001\"\r\n").unwrap();
        assert!(decoded.iter().any(|event| matches!(
            event,
            Event::Scalar { value: crate::Scalar::String(value), .. } if value.as_ref() == "\u{1}"
        )));
    }
    #[test]
    fn malformed_root_headers_fall_through_only_in_non_strict_mode() {
        let config = DecoderConfig {
            strict: false,
            ..DecoderConfig::default()
        };
        let mut decoder = Decoder::new(Cursor::new(b"[03]: a,b"), SourceId::new(1), config);
        let mut actual = Vec::new();
        while let Some(event) = decoder.next_event().unwrap() {
            actual.push(event);
        }
        assert!(actual.iter().any(|event| matches!(
            event, Event::Key { value, .. } if value.as_ref() == "[03]"
        )));
        assert!(events(b"[03]: a,b").is_err());

        let mut decoder = Decoder::new(Cursor::new(b"a[2|]{x,y}: 1|2"), SourceId::new(1), config);
        let mut actual = Vec::new();
        while let Some(event) = decoder.next_event().unwrap() {
            actual.push(event);
        }
        assert!(actual.iter().any(|event| matches!(
            event, Event::Key { value, .. } if value.as_ref() == "a[2|]{x,y}"
        )));
    }

    #[test]
    fn strict_header_grammar_and_blank_span_boundaries_are_enforced() {
        for input in [
            "m[2|:]{v}:",
            "m[2:,]{v}:",
            "items[2]{a|b}:",
            "foo [2]: x,y",
            "items[1]{a}: value",
        ] {
            assert!(events(input.as_bytes()).is_err(), "{input:?}");
        }
        assert!(events(b"items[2]:\n\n  - a\n  - b").is_ok());
        assert!(events(b"items[2]:\n  - a\n\n  - b").is_err());
    }
    #[test]
    fn short_non_strict_nested_rows_keep_the_declared_object_shape() {
        let config = DecoderConfig {
            strict: false,
            ..DecoderConfig::default()
        };
        let mut decoder = Decoder::new(
            Cursor::new(b"items[1]{x,meta{y}}:\n  1"),
            SourceId::new(1),
            config,
        );
        let mut events = Vec::new();
        while let Some(event) = decoder.next_event().unwrap() {
            events.push(event);
        }
        let keys = events
            .iter()
            .filter_map(|event| match event {
                Event::Key { value, .. } => Some(value.as_ref()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(keys, ["items", "x", "meta"]);
    }

    #[test]
    fn discarded_events_obey_semantic_container_depth() {
        for (input, depth) in [
            ("", 1),
            ("[]", 1),
            ("x: []", 2),
            ("[1]{x}:\n  1", 2),
            ("[1]{g{x}}:\n  1", 3),
            ("[1:]{x}:\n  a: 1", 2),
            ("[1:]{g{x}}:\n  a: 1", 3),
            ("[1]:\n  - x: []", 3),
        ] {
            let decode = |maximum_depth, retain| {
                let mut decoder = Decoder::new(
                    Cursor::new(input.as_bytes()),
                    SourceId::new(1),
                    DecoderConfig {
                        maximum_depth,
                        ..DecoderConfig::default()
                    },
                );
                let mut retained = Vec::new();
                while let Some(event) = decoder.next_event()? {
                    if retain {
                        retained.push(event);
                    }
                }
                Ok::<_, crate::DecodeError>(retained)
            };
            assert!(
                matches!(
                    decode(depth - 1, false),
                    Err(crate::DecodeError::Resource { .. })
                ),
                "{input:?} escaped depth {depth}"
            );
            assert_eq!(
                decode(depth, true).unwrap(),
                events(input.as_bytes()).unwrap()
            );
        }
    }

    #[test]
    fn active_names_and_schema_nodes_share_aggregate_limits() {
        for (input, fields, bytes) in [
            ("a:\n  b: 1", 2, 2),
            ("aaa:\n  bb: 1", 2, 5),
            ("a[1]{b}:\n  1", 2, 2),
            ("[1:]{a{b}}:\n  c: 1", 3, 3),
            ("a:\n  b[1]{c{d}}:\n    1", 4, 4),
        ] {
            let decode = |maximum_fields, maximum_name_bytes, retain| {
                let mut decoder = Decoder::new(
                    Cursor::new(input.as_bytes()),
                    SourceId::new(1),
                    DecoderConfig {
                        maximum_fields,
                        maximum_name_bytes,
                        ..DecoderConfig::default()
                    },
                );
                let mut retained = Vec::new();
                while let Some(event) = decoder.next_event()? {
                    if retain {
                        retained.push(event);
                    }
                }
                Ok::<_, crate::DecodeError>(retained)
            };
            for limits in [(fields - 1, bytes), (fields, bytes - 1)] {
                assert!(
                    matches!(
                        decode(limits.0, limits.1, false),
                        Err(crate::DecodeError::Resource { .. })
                    ),
                    "{input:?} escaped aggregate limit {limits:?}"
                );
            }
            assert_eq!(
                decode(fields, bytes, true).unwrap(),
                events(input.as_bytes()).unwrap()
            );
        }
    }
}
