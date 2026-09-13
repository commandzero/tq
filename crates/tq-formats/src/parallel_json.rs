use std::{
    collections::{BTreeMap, VecDeque},
    io::{self, BufRead, Cursor, Read},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender, TryRecvError},
    },
};

use tq_core::{JsonInput, JsonInputOptions, PathComponent, Value};

use crate::{
    FormatError, InputFormat, SelectedStreamObservations, StreamOptions, StreamRecord,
    StreamSelection, stream_json_selected_records_with_control,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Finite batching and retention bounds for selected-array decoding.
pub struct ParallelJsonOptions {
    /// Maximum elements grouped into one worker batch.
    pub batch_values: usize,
    /// Target maximum encoded bytes grouped into one worker batch.
    pub batch_bytes: usize,
    /// Maximum submitted batches not yet delivered.
    pub in_flight_batches: usize,
    /// Maximum encoded bytes submitted but not yet delivered.
    pub in_flight_bytes: usize,
}

impl Default for ParallelJsonOptions {
    fn default() -> Self {
        Self {
            batch_values: 4 * 1024,
            batch_bytes: 2 * 1024 * 1024,
            in_flight_batches: 32,
            in_flight_bytes: 64 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
/// High-water observations from one parallel JSON document.
pub struct ParallelJsonObservations {
    /// Largest number of simultaneously active source containers.
    pub depth_high_water: usize,
    /// Worker batches submitted.
    pub batches: usize,
    /// Largest number of submitted, undelivered batches.
    pub in_flight_batches_high_water: usize,
    /// Largest number of submitted, undelivered encoded bytes.
    pub in_flight_bytes_high_water: usize,
    /// Largest number of out-of-order completed batches retained.
    pub reordered_batches_high_water: usize,
}

struct Work {
    ordinal: usize,
    first_index: Option<usize>,
    bytes: Vec<u8>,
    source: BatchSource,
}

#[derive(Clone, Copy)]
struct BatchSource {
    byte_offset: u64,
    line: usize,
    column: usize,
    synthetic_prefix_bytes: usize,
}

struct Completion {
    ordinal: usize,
    retained_bytes: usize,
    outcome: BatchOutcome,
}

struct BatchOutcome {
    records: Vec<StreamRecord>,
    error: Option<FormatError>,
    depth_high_water: usize,
}

struct Scheduler {
    sender: Sender<Completion>,
    receiver: Receiver<Completion>,
    selection: StreamSelection,
    stream_options: StreamOptions,
    cancellation: Arc<AtomicBool>,
    external_cancellation: Option<Arc<AtomicBool>>,
    next_ordinal: usize,
    next_delivery: usize,
    outstanding_bytes: usize,
    outstanding: VecDeque<(usize, usize)>,
    completed: BTreeMap<usize, Completion>,
    options: ParallelJsonOptions,
    observations: ParallelJsonObservations,
}

impl Scheduler {
    fn new(
        selection: StreamSelection,
        stream_options: StreamOptions,
        options: ParallelJsonOptions,
        external_cancellation: Option<Arc<AtomicBool>>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            sender,
            receiver,
            selection,
            stream_options,
            cancellation: Arc::new(AtomicBool::new(false)),
            external_cancellation,
            next_ordinal: 0,
            next_delivery: 0,
            outstanding_bytes: 0,
            outstanding: VecDeque::new(),
            completed: BTreeMap::new(),
            options,
            observations: ParallelJsonObservations::default(),
        }
    }

    fn submit<F>(
        &mut self,
        first_index: Option<usize>,
        bytes: Vec<u8>,
        source: BatchSource,
        emit: &mut F,
    ) -> Result<(), FormatError>
    where
        F: FnMut(StreamRecord) -> Result<(), String>,
    {
        let retained_bytes = bytes.len();
        if retained_bytes > self.options.in_flight_bytes.max(1) {
            return Err(FormatError::Resource("parallel-decode-in-flight-bytes"));
        }
        while !self.outstanding.is_empty()
            && (self.outstanding.len() >= self.options.in_flight_batches.max(1)
                || self.outstanding_bytes.saturating_add(retained_bytes)
                    > self.options.in_flight_bytes.max(1))
        {
            self.drain_one(true, emit)?;
        }
        if self.cancelled() {
            return Err(cancelled_error());
        }
        let work = Work {
            ordinal: self.next_ordinal,
            first_index,
            bytes,
            source,
        };
        self.next_ordinal = self.next_ordinal.saturating_add(1);
        self.outstanding.push_back((work.ordinal, retained_bytes));
        self.outstanding_bytes = self.outstanding_bytes.saturating_add(retained_bytes);
        self.observations.batches = self.observations.batches.saturating_add(1);
        self.observations.in_flight_batches_high_water = self
            .observations
            .in_flight_batches_high_water
            .max(self.outstanding.len());
        self.observations.in_flight_bytes_high_water = self
            .observations
            .in_flight_bytes_high_water
            .max(self.outstanding_bytes);

        let sender = self.sender.clone();
        let selection = self.selection.clone();
        let stream_options = self.stream_options;
        let cancellation = Arc::clone(&self.cancellation);
        let external_cancellation = self.external_cancellation.clone();
        rayon::spawn_fifo(move || {
            let outcome = if cancellation.load(Ordering::Relaxed)
                || external_cancellation
                    .as_ref()
                    .is_some_and(|flag| flag.load(Ordering::Relaxed))
            {
                BatchOutcome {
                    records: Vec::new(),
                    error: Some(cancelled_error()),
                    depth_high_water: 0,
                }
            } else {
                decode_work(
                    &work.bytes,
                    stream_options,
                    &selection,
                    work.first_index,
                    work.source,
                    cancellation,
                    external_cancellation,
                )
            };
            let _ = sender.send(Completion {
                ordinal: work.ordinal,
                retained_bytes,
                outcome,
            });
        });
        self.drain_ready(emit)
    }

    fn finish<F>(mut self, emit: &mut F) -> Result<ParallelJsonObservations, FormatError>
    where
        F: FnMut(StreamRecord) -> Result<(), String>,
    {
        while !self.outstanding.is_empty() {
            self.drain_one(true, emit)?;
        }
        Ok(self.observations)
    }

    fn drain_ready<F>(&mut self, emit: &mut F) -> Result<(), FormatError>
    where
        F: FnMut(StreamRecord) -> Result<(), String>,
    {
        loop {
            match self.receiver.try_recv() {
                Ok(completion) => self.store(completion),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    return Err(parse_error("parallel JSON worker channel disconnected"));
                }
            }
        }
        self.deliver_ready(emit)
    }

    fn drain_one<F>(&mut self, block: bool, emit: &mut F) -> Result<(), FormatError>
    where
        F: FnMut(StreamRecord) -> Result<(), String>,
    {
        self.drain_ready(emit)?;
        if self.completed.contains_key(&self.next_delivery) || self.outstanding.is_empty() {
            return self.deliver_ready(emit);
        }
        if block {
            let completion = self
                .receiver
                .recv()
                .map_err(|_| parse_error("parallel JSON worker channel disconnected"))?;
            self.store(completion);
            self.deliver_ready(emit)?;
        }
        Ok(())
    }

    fn store(&mut self, completion: Completion) {
        self.completed.insert(completion.ordinal, completion);
        self.observations.reordered_batches_high_water = self
            .observations
            .reordered_batches_high_water
            .max(self.completed.len());
    }

    fn deliver_ready<F>(&mut self, emit: &mut F) -> Result<(), FormatError>
    where
        F: FnMut(StreamRecord) -> Result<(), String>,
    {
        while let Some(completion) = self.completed.remove(&self.next_delivery) {
            let Some((ordinal, retained_bytes)) = self.outstanding.pop_front() else {
                return Err(parse_error(
                    "parallel JSON completion had no outstanding batch",
                ));
            };
            if ordinal != completion.ordinal || retained_bytes != completion.retained_bytes {
                return Err(parse_error(
                    "parallel JSON completion order accounting failed",
                ));
            }
            self.outstanding_bytes = self.outstanding_bytes.saturating_sub(retained_bytes);
            self.next_delivery = self.next_delivery.saturating_add(1);
            self.observations.depth_high_water = self
                .observations
                .depth_high_water
                .max(completion.outcome.depth_high_water);
            for record in completion.outcome.records {
                if let Err(message) = emit(record) {
                    self.cancellation.store(true, Ordering::Relaxed);
                    return Err(parse_error(message));
                }
            }
            if let Some(error) = completion.outcome.error {
                self.cancellation.store(true, Ordering::Relaxed);
                return Err(error);
            }
        }
        Ok(())
    }

    fn cancelled(&self) -> bool {
        self.cancellation.load(Ordering::Relaxed)
            || self
                .external_cancellation
                .as_ref()
                .is_some_and(|flag| flag.load(Ordering::Relaxed))
    }
}

fn decode_work(
    bytes: &[u8],
    options: StreamOptions,
    selection: &StreamSelection,
    first_index: Option<usize>,
    source: BatchSource,
    cancellation: Arc<AtomicBool>,
    external_cancellation: Option<Arc<AtomicBool>>,
) -> BatchOutcome {
    let reader = CancellationReader {
        bytes,
        cancellation,
        external_cancellation,
        bytes_until_check: 0,
    };
    let Some(first_index) = first_index else {
        let mut records = Vec::new();
        let mut observations = SelectedStreamObservations::default();
        let result = stream_json_selected_records_with_control(
            reader,
            options,
            selection.clone(),
            None,
            &mut observations,
            |record| {
                records.push(record);
                Ok(())
            },
        );
        return BatchOutcome {
            records,
            error: result
                .err()
                .map(|error| translate_worker_error(error, source)),
            depth_high_water: observations.depth_high_water,
        };
    };
    let relative = StreamSelection::new(
        Vec::new(),
        selection.projection().map(<[PathComponent]>::to_vec),
    );
    let options = StreamOptions {
        maximum_depth: options
            .maximum_depth
            .saturating_sub(selection.prefix().len()),
        ..options
    };
    let mut records = Vec::new();
    let mut observations = SelectedStreamObservations::default();
    let result = stream_json_selected_records_with_control(
        reader,
        options,
        relative,
        None,
        &mut observations,
        |record| {
            records.push(record.rebase_array_item(selection.prefix(), first_index));
            Ok(())
        },
    );
    BatchOutcome {
        records,
        error: result
            .err()
            .map(|error| translate_worker_error(error, source)),
        depth_high_water: observations
            .depth_high_water
            .saturating_add(selection.prefix().len()),
    }
}

struct CancellationReader<'a> {
    bytes: &'a [u8],
    cancellation: Arc<AtomicBool>,
    external_cancellation: Option<Arc<AtomicBool>>,
    bytes_until_check: usize,
}

impl Read for CancellationReader<'_> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if self.bytes_until_check == 0 {
            if self.cancellation.load(Ordering::Relaxed)
                || self
                    .external_cancellation
                    .as_ref()
                    .is_some_and(|flag| flag.load(Ordering::Relaxed))
            {
                // serde_json retries `Interrupted` reads indefinitely.
                return Err(io::Error::other("parallel JSON decoding interrupted"));
            }
            self.bytes_until_check = 16 * 1024;
        }
        let length = output
            .len()
            .min(self.bytes.len())
            .min(self.bytes_until_check);
        output[..length].copy_from_slice(&self.bytes[..length]);
        self.bytes = &self.bytes[length..];
        self.bytes_until_check = self.bytes_until_check.saturating_sub(length);
        Ok(length)
    }
}

fn translate_worker_error(error: FormatError, source: BatchSource) -> FormatError {
    let FormatError::Parse {
        format: InputFormat::Json,
        message,
    } = error
    else {
        return error;
    };
    FormatError::Parse {
        format: InputFormat::Json,
        message: translate_json_position(&message, source),
    }
}

fn translate_json_position(message: &str, source: BatchSource) -> String {
    let (prefix, line, column, detail) =
        if let Some((prefix, position)) = message.rsplit_once(", line ") {
            let Some((line, position)) = position.split_once(", column ") else {
                return message.to_owned();
            };
            let Some((column, detail)) = position.split_once(": ") else {
                return message.to_owned();
            };
            let (Ok(line), Ok(column)) = (line.parse::<usize>(), column.parse::<usize>()) else {
                return message.to_owned();
            };
            (prefix, line, column, detail)
        } else if let Some((prefix, position)) = message.rsplit_once(" at line ") {
            let Some((line, column)) = position.split_once(" column ") else {
                return message.to_owned();
            };
            let (Ok(line), Ok(column)) = (line.parse::<usize>(), column.parse::<usize>()) else {
                return message.to_owned();
            };
            (prefix, line, column, "")
        } else {
            return message.to_owned();
        };
    let (line, column) = if line == 1 {
        (
            source.line,
            source
                .column
                .saturating_add(column.saturating_sub(source.synthetic_prefix_bytes + 1)),
        )
    } else {
        (source.line.saturating_add(line - 1), column)
    };
    let prefix =
        if let Some((head, local_byte)) = prefix.rsplit_once(" at byte ") {
            let Ok(local_byte) = local_byte.parse::<u64>() else {
                return message.to_owned();
            };
            format!(
                "{head} at byte {}",
                source.byte_offset.saturating_add(local_byte.saturating_sub(
                    u64::try_from(source.synthetic_prefix_bytes).unwrap_or(u64::MAX),
                ))
            )
        } else {
            prefix.to_owned()
        };
    if detail.is_empty() {
        format!("{prefix} at line {line} column {column}")
    } else {
        format!("{prefix}, line {line}, column {column}: {detail}")
    }
}

/// Frames one statically selected array and decodes its element batches on the
/// shared Rayon pool while delivering records in source order.
///
/// # Errors
///
/// Returns framing, JSON decoding, resource-limit, cancellation, or callback
/// failures. Unsupported selection shapes transparently use the serial path.
pub fn stream_json_selected_records_parallel<R, F>(
    mut reader: R,
    stream_options: StreamOptions,
    selection: StreamSelection,
    parallel_options: ParallelJsonOptions,
    cancellation: Option<Arc<AtomicBool>>,
    mut emit: F,
) -> Result<ParallelJsonObservations, FormatError>
where
    R: BufRead,
    F: FnMut(StreamRecord) -> Result<(), String>,
{
    let mut framer = Framer::new(&mut reader, stream_options, cancellation.clone());
    framer.skip_ws()?;

    let prefix = selection.prefix().to_vec();
    let mut scheduler = Scheduler::new(selection, stream_options, parallel_options, cancellation);
    let framing = framer.parse_document(&prefix, parallel_options, &mut |index, bytes, source| {
        scheduler.submit(index, bytes, source, &mut emit)
    });
    drop(framer);
    let worker_result = scheduler.finish(&mut emit);
    match (worker_result, framing) {
        (Err(worker), _) => Err(worker),
        (Ok(_), Err(frame)) => Err(frame),
        (Ok(observations), Ok(())) => Ok(observations),
    }
}

struct Framer<'a, R> {
    reader: &'a mut R,
    options: StreamOptions,
    capture: Option<Vec<u8>>,
    capture_limit: usize,
    byte_offset: u64,
    line: usize,
    column: usize,
    cancellation: Option<Arc<AtomicBool>>,
    bytes_until_cancellation_check: usize,
}

impl<'a, R: BufRead> Framer<'a, R> {
    fn new(
        reader: &'a mut R,
        options: StreamOptions,
        cancellation: Option<Arc<AtomicBool>>,
    ) -> Self {
        Self {
            reader,
            options,
            capture: None,
            capture_limit: usize::MAX,
            byte_offset: 0,
            line: 1,
            column: 1,
            cancellation,
            bytes_until_cancellation_check: 0,
        }
    }

    fn parse_document<F>(
        &mut self,
        prefix: &[PathComponent],
        parallel: ParallelJsonOptions,
        submit: &mut F,
    ) -> Result<(), FormatError>
    where
        F: FnMut(Option<usize>, Vec<u8>, BatchSource) -> Result<(), FormatError>,
    {
        // A captured selected value or worker batch must fit in the same
        // retained-byte budget enforced by the scheduler. Check before
        // extending the capture buffer so a single oversized element cannot
        // allocate without bound while being framed.
        self.capture_limit = parallel.in_flight_bytes.max(1);
        self.check_cancellation()?;
        self.parse_prefix_value(prefix, prefix, 0, parallel, submit)?;
        self.finish_document()
    }

    fn parse_prefix_value<F>(
        &mut self,
        remaining: &[PathComponent],
        full_prefix: &[PathComponent],
        depth: usize,
        parallel: ParallelJsonOptions,
        submit: &mut F,
    ) -> Result<(), FormatError>
    where
        F: FnMut(Option<usize>, Vec<u8>, BatchSource) -> Result<(), FormatError>,
    {
        let Some((component, rest)) = remaining.split_first() else {
            return self.parse_selected_value(full_prefix, depth, parallel, submit);
        };
        match component {
            PathComponent::Key(selected_key) => {
                if self.peek()? != Some(b'{') {
                    return self.scan_value(depth);
                }
                self.check_container_depth(depth)?;
                self.expect(b'{')?;
                self.skip_ws()?;
                if self.take_if(b'}')? {
                    return Ok(());
                }
                loop {
                    let key = self.read_string()?;
                    self.skip_ws()?;
                    self.expect(b':')?;
                    self.skip_ws()?;
                    if key == selected_key.as_ref() {
                        self.parse_prefix_value(
                            rest,
                            full_prefix,
                            depth.saturating_add(1),
                            parallel,
                            submit,
                        )?;
                    } else {
                        self.scan_value(depth.saturating_add(1))?;
                    }
                    self.skip_ws()?;
                    if self.take_if(b'}')? {
                        return Ok(());
                    }
                    self.expect(b',')?;
                    self.skip_ws()?;
                }
            }
            PathComponent::Index(selected_index) => {
                if self.peek()? != Some(b'[') {
                    return self.scan_value(depth);
                }
                self.check_container_depth(depth)?;
                self.expect(b'[')?;
                self.skip_ws()?;
                if self.take_if(b']')? {
                    return Ok(());
                }
                let mut index = 0usize;
                loop {
                    if index == *selected_index {
                        self.parse_prefix_value(
                            rest,
                            full_prefix,
                            depth.saturating_add(1),
                            parallel,
                            submit,
                        )?;
                    } else {
                        self.scan_value(depth.saturating_add(1))?;
                    }
                    self.skip_ws()?;
                    if self.take_if(b']')? {
                        return Ok(());
                    }
                    self.expect(b',')?;
                    self.skip_ws()?;
                    index = index.saturating_add(1);
                }
            }
        }
    }

    fn parse_selected_value<F>(
        &mut self,
        prefix: &[PathComponent],
        depth: usize,
        parallel: ParallelJsonOptions,
        submit: &mut F,
    ) -> Result<(), FormatError>
    where
        F: FnMut(Option<usize>, Vec<u8>, BatchSource) -> Result<(), FormatError>,
    {
        if self.peek()? != Some(b'[') {
            let position = self.position();
            self.capture = Some(Vec::new());
            self.scan_value(depth)?;
            let value = self.capture.take().expect("target capture is active");
            let (wrapper, synthetic_prefix_bytes) = wrap_selected_value(prefix, value);
            return submit(
                None,
                wrapper,
                BatchSource {
                    synthetic_prefix_bytes,
                    ..position
                },
            );
        }
        self.check_container_depth(depth)?;
        self.expect(b'[')?;
        self.skip_ws()?;
        if self.take_if(b']')? {
            return Ok(());
        }

        let mut first_index = 0usize;
        let mut count = 0usize;
        let mut batch = Vec::with_capacity(parallel.batch_bytes.clamp(2, 8 * 1024 * 1024));
        batch.push(b'[');
        let mut batch_source = BatchSource {
            synthetic_prefix_bytes: 1,
            ..self.position()
        };
        loop {
            self.capture = Some(batch);
            self.scan_raw_array_element()?;
            batch = self.capture.take().expect("element capture is active");
            count = count.saturating_add(1);
            let ended = self.peek()? == Some(b']');
            let flush = count >= parallel.batch_values.max(1)
                || batch.len() >= parallel.batch_bytes.max(1)
                || ended;
            if flush {
                batch.push(b']');
                submit(Some(first_index), batch, batch_source)?;
                first_index = first_index.saturating_add(count);
                count = 0;
                batch = Vec::with_capacity(parallel.batch_bytes.clamp(2, 8 * 1024 * 1024));
                batch.push(b'[');
            }
            if self.take_if(b']')? {
                break;
            }
            if flush {
                self.expect(b',')?;
                self.skip_ws()?;
                batch_source = BatchSource {
                    synthetic_prefix_bytes: 1,
                    ..self.position()
                };
            } else {
                self.capture = Some(batch);
                self.expect(b',')?;
                self.skip_ws()?;
                batch = self.capture.take().expect("batch capture is active");
            }
        }
        Ok(())
    }

    fn position(&self) -> BatchSource {
        BatchSource {
            byte_offset: self.byte_offset,
            line: self.line,
            column: self.column,
            synthetic_prefix_bytes: 0,
        }
    }

    fn scan_raw_array_element(&mut self) -> Result<(), FormatError> {
        let mut containers = Vec::new();
        let mut in_string = false;
        let mut escaped = false;
        let mut saw_value_byte = false;
        loop {
            let available = self.reader.fill_buf()?;
            if available.is_empty() {
                return Err(parse_error("EOF while framing an array element"));
            }
            let mut used = 0usize;
            while used < available.len() {
                let byte = available[used];
                if in_string {
                    used += 1;
                    if escaped {
                        escaped = false;
                    } else if byte == b'\\' {
                        escaped = true;
                    } else if byte == b'"' {
                        in_string = false;
                    }
                    continue;
                }
                if containers.is_empty() && matches!(byte, b',' | b']') {
                    if !saw_value_byte {
                        return Err(parse_error("expected value"));
                    }
                    if used > 0 {
                        self.consume_slice(used)?;
                    }
                    return Ok(());
                }
                used += 1;
                if !byte.is_ascii_whitespace() {
                    saw_value_byte = true;
                }
                match byte {
                    b'"' => in_string = true,
                    b'{' | b'[' => containers.push(byte),
                    b'}' if containers.last() == Some(&b'{') => {
                        containers.pop();
                    }
                    b']' if containers.last() == Some(&b'[') => {
                        containers.pop();
                    }
                    _ => {}
                }
            }
            self.consume_slice(used)?;
        }
    }

    fn finish_document(&mut self) -> Result<(), FormatError> {
        self.skip_ws()?;
        if self.peek()?.is_some() {
            return Err(parse_error("trailing characters"));
        }
        Ok(())
    }

    fn scan_value(&mut self, depth: usize) -> Result<(), FormatError> {
        match self.peek()? {
            Some(b'"') => self.scan_string(),
            Some(b'{') => {
                self.check_container_depth(depth)?;
                self.scan_object(depth)
            }
            Some(b'[') => {
                self.check_container_depth(depth)?;
                self.scan_array(depth)
            }
            Some(
                b't'
                | b'f'
                | b'n'
                | b'-'
                | b'0'..=b'9'
                | b'+'
                | b'N'
                | b'I'
                | b'i'
                | b's'
                | b'S'
                | b'.',
            ) => self.scan_word(),
            Some(_) => Err(parse_error("expected value")),
            None => Err(parse_error("EOF while parsing a value")),
        }
    }

    fn check_container_depth(&self, depth: usize) -> Result<(), FormatError> {
        if depth >= self.options.maximum_depth {
            Err(parse_error("stream depth limit exceeded"))
        } else {
            Ok(())
        }
    }

    fn scan_object(&mut self, depth: usize) -> Result<(), FormatError> {
        self.expect(b'{')?;
        self.skip_ws()?;
        if self.take_if(b'}')? {
            return Ok(());
        }
        loop {
            self.scan_string()?;
            self.skip_ws()?;
            self.expect(b':')?;
            self.skip_ws()?;
            self.scan_value(depth.saturating_add(1))?;
            self.skip_ws()?;
            if self.take_if(b'}')? {
                return Ok(());
            }
            self.expect(b',')?;
            self.skip_ws()?;
        }
    }

    fn scan_array(&mut self, depth: usize) -> Result<(), FormatError> {
        self.expect(b'[')?;
        self.skip_ws()?;
        if self.take_if(b']')? {
            return Ok(());
        }
        loop {
            self.scan_value(depth.saturating_add(1))?;
            self.skip_ws()?;
            if self.take_if(b']')? {
                return Ok(());
            }
            self.expect(b',')?;
            self.skip_ws()?;
        }
    }

    fn read_string(&mut self) -> Result<String, FormatError> {
        let prior = self.capture.take();
        self.capture = Some(Vec::new());
        self.scan_string()?;
        let bytes = self.capture.take().expect("string capture is active");
        self.capture = prior;
        let value = parse_json_fragment(
            &bytes,
            self.options.maximum_depth,
            self.options.maximum_token_bytes,
        )?;
        match value {
            Value::String(value) => Ok(value.to_string()),
            _ => Err(parse_error("expected JSON string key")),
        }
    }

    fn scan_string(&mut self) -> Result<(), FormatError> {
        let validate_token = self.capture.is_none();
        if validate_token {
            self.capture = Some(Vec::new());
        }
        let mut raw_bytes = 1usize;
        let raw_limit = self
            .options
            .maximum_token_bytes
            .saturating_mul(6)
            .saturating_add(2);
        self.expect(b'"')?;
        loop {
            let available = self.reader.fill_buf()?;
            if available.is_empty() {
                return Err(parse_error("EOF while parsing a string"));
            }
            let used = available
                .iter()
                .position(|byte| *byte == b'"' || *byte == b'\\' || *byte < 0x20)
                .unwrap_or(available.len());
            if used > 0 {
                raw_bytes = raw_bytes.saturating_add(used);
                if raw_bytes > raw_limit {
                    return Err(resource_error("token-bytes"));
                }
                self.consume_slice(used)?;
                continue;
            }
            match self.peek()? {
                Some(b'"') => {
                    raw_bytes = raw_bytes.saturating_add(1);
                    if raw_bytes > raw_limit {
                        return Err(resource_error("token-bytes"));
                    }
                    self.consume_slice(1)?;
                    if validate_token {
                        let bytes = self.capture.take().expect("string capture is active");
                        parse_json_fragment(
                            &bytes,
                            self.options.maximum_depth,
                            self.options.maximum_token_bytes,
                        )?;
                    }
                    return Ok(());
                }
                Some(b'\\') => {
                    raw_bytes = raw_bytes.saturating_add(1);
                    if raw_bytes > raw_limit {
                        return Err(resource_error("token-bytes"));
                    }
                    self.consume_slice(1)?;
                    let escape = self.take()?.ok_or_else(|| parse_error("EOF in escape"))?;
                    raw_bytes = raw_bytes.saturating_add(1);
                    if raw_bytes > raw_limit {
                        return Err(parse_error("input resource limit exceeded: token-bytes"));
                    }
                    if escape == b'u' {
                        for _ in 0..4 {
                            let digit = self
                                .take()?
                                .ok_or_else(|| parse_error("EOF in unicode escape"))?;
                            raw_bytes = raw_bytes.saturating_add(1);
                            if raw_bytes > raw_limit {
                                return Err(resource_error("token-bytes"));
                            }
                            if !digit.is_ascii_hexdigit() {
                                return Err(parse_error("invalid unicode escape"));
                            }
                        }
                    } else if !matches!(
                        escape,
                        b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't'
                    ) {
                        return Err(parse_error("invalid escape"));
                    }
                }
                Some(_) => return Err(parse_error("control character while parsing a string")),
                None => return Err(parse_error("EOF while parsing a string")),
            }
        }
    }

    fn scan_word(&mut self) -> Result<(), FormatError> {
        // Skipped scalars still go through the shared parser. Keep their
        // validation copy bounded by the configured token budget, with the
        // small exception needed to recognize the three unlimited JSON
        // keywords. A malformed prefix such as `truex` must not stay exempt.
        let validate_token = self.capture.is_none();
        if validate_token {
            self.capture = Some(Vec::new());
        }
        let mut prefix = [0_u8; 5];
        let mut prefix_len = 0usize;
        let mut token_bytes = 0usize;
        while let Some(byte) = self.peek()? {
            if matches!(
                byte,
                b' ' | b'\n' | b'\r' | b'\t' | b',' | b']' | b'}' | b'[' | b'{' | b':' | b'"'
            ) {
                break;
            }
            token_bytes = token_bytes.saturating_add(1);
            if prefix_len < prefix.len() {
                prefix[prefix_len] = byte;
                prefix_len += 1;
            }
            self.consume_slice(1)?;

            if token_bytes >= prefix.len()
                && !is_exempt_word(&prefix[..prefix_len], token_bytes)
                && token_bytes > self.options.maximum_token_bytes
            {
                return Err(resource_error("token-bytes"));
            }
        }
        if token_bytes == 0 {
            return Err(parse_error("expected value"));
        }
        if !is_exempt_word(&prefix[..prefix_len], token_bytes)
            && token_bytes > self.options.maximum_token_bytes
        {
            return Err(resource_error("token-bytes"));
        }
        if validate_token {
            let bytes = self.capture.take().expect("scalar capture is active");
            parse_json_fragment(
                &bytes,
                self.options.maximum_depth,
                self.options.maximum_token_bytes,
            )?;
        }
        Ok(())
    }

    fn skip_ws(&mut self) -> Result<(), FormatError> {
        loop {
            let available = self.reader.fill_buf()?;
            let used = available
                .iter()
                .take_while(|byte| matches!(byte, b' ' | b'\n' | b'\r' | b'\t'))
                .count();
            if used == 0 {
                return Ok(());
            }
            self.consume_slice(used)?;
        }
    }

    fn peek(&mut self) -> Result<Option<u8>, FormatError> {
        Ok(self.reader.fill_buf()?.first().copied())
    }

    fn take(&mut self) -> Result<Option<u8>, FormatError> {
        let byte = self.peek()?;
        if byte.is_some() {
            self.consume_slice(1)?;
        }
        Ok(byte)
    }

    fn take_if(&mut self, expected: u8) -> Result<bool, FormatError> {
        if self.peek()? == Some(expected) {
            self.consume_slice(1)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn expect(&mut self, expected: u8) -> Result<(), FormatError> {
        if self.take()? == Some(expected) {
            Ok(())
        } else {
            Err(parse_error(format!("expected `{}`", char::from(expected))))
        }
    }

    fn consume_slice(&mut self, length: usize) -> Result<(), FormatError> {
        if self.bytes_until_cancellation_check == 0 {
            self.check_cancellation()?;
            self.bytes_until_cancellation_check = 16 * 1024;
        }
        let available = self.reader.fill_buf()?;
        if length > available.len() {
            return Err(parse_error("internal JSON framing boundary error"));
        }
        if let Some(capture) = self.capture.as_mut() {
            if capture.len().saturating_add(length) > self.capture_limit {
                return Err(resource_error("parallel-decode-in-flight-bytes"));
            }
            capture.extend_from_slice(&available[..length]);
        }
        let consumed = &available[..length];
        let mut newline_count = 0usize;
        let mut last_newline = None;
        for index in memchr::memchr_iter(b'\n', consumed) {
            newline_count = newline_count.saturating_add(1);
            last_newline = Some(index);
        }
        self.byte_offset = self
            .byte_offset
            .saturating_add(u64::try_from(length).unwrap_or(u64::MAX));
        if let Some(last_newline) = last_newline {
            self.line = self.line.saturating_add(newline_count);
            self.column = length.saturating_sub(last_newline);
        } else {
            self.column = self.column.saturating_add(length);
        }
        self.reader.consume(length);
        self.bytes_until_cancellation_check =
            self.bytes_until_cancellation_check.saturating_sub(length);
        Ok(())
    }

    fn check_cancellation(&self) -> Result<(), FormatError> {
        if self
            .cancellation
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
        {
            Err(cancelled_error())
        } else {
            Ok(())
        }
    }
}

fn is_exempt_word(prefix: &[u8], token_bytes: usize) -> bool {
    matches!(
        (prefix, token_bytes),
        (b"true" | b"null", 4) | (b"false", 5)
    )
}

fn wrap_selected_value(prefix: &[PathComponent], mut value: Vec<u8>) -> (Vec<u8>, usize) {
    let mut value_offset = 0usize;
    for component in prefix.iter().rev() {
        let mut wrapper = Vec::new();
        match component {
            PathComponent::Key(key) => {
                wrapper.push(b'{');
                wrapper.extend_from_slice(Value::string(key.as_ref()).to_string().as_bytes());
                wrapper.push(b':');
            }
            PathComponent::Index(index) => {
                wrapper.push(b'[');
                for _ in 0..*index {
                    wrapper.extend_from_slice(b"null,");
                }
            }
        }
        value_offset = value_offset.saturating_add(wrapper.len());
        wrapper.append(&mut value);
        wrapper.push(match component {
            PathComponent::Key(_) => b'}',
            PathComponent::Index(_) => b']',
        });
        value = wrapper;
    }
    (value, value_offset)
}

fn parse_json_fragment(
    bytes: &[u8],
    maximum_depth: usize,
    maximum_token_bytes: usize,
) -> Result<Value, FormatError> {
    let mut input = JsonInput::new(
        Cursor::new(bytes),
        JsonInputOptions {
            maximum_depth,
            maximum_token_bytes,
        },
    );
    let mut checkpoint = || Ok(());
    let value = input
        .next_value(&mut checkpoint)
        .map_err(format_json_input_error)?
        .ok_or_else(|| parse_error("expected value"))?;
    input
        .check_end(&mut checkpoint)
        .map_err(format_json_input_error)?;
    Ok(value)
}

fn format_json_input_error(error: tq_core::JsonInputError) -> FormatError {
    match error {
        tq_core::JsonInputError::Limit { limit, .. } => resource_error(match limit {
            tq_core::JsonLimit::TokenBytes => "token-bytes",
            tq_core::JsonLimit::Depth => "depth",
        }),
        error => parse_error(error.to_string()),
    }
}

fn resource_error(resource: &'static str) -> FormatError {
    FormatError::Resource(resource)
}

fn parse_error(message: impl Into<String>) -> FormatError {
    FormatError::Parse {
        format: InputFormat::Json,
        message: message.into(),
    }
}

fn cancelled_error() -> FormatError {
    parse_error("interrupted")
}

#[cfg(test)]
mod tests {
    use std::{
        io::{BufReader, Read},
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };

    use tq_core::PathComponent;

    use crate::{FormatError, stream_json_selected_records};

    use super::{
        BatchOutcome, CancellationReader, Completion, ParallelJsonOptions, Scheduler,
        StreamOptions, StreamRecord, StreamSelection, stream_json_selected_records_parallel,
    };

    fn selection() -> StreamSelection {
        StreamSelection::new(
            vec![PathComponent::Key(Arc::from("features"))],
            Some(vec![
                PathComponent::Key(Arc::from("properties")),
                PathComponent::Key(Arc::from("release")),
            ]),
        )
    }

    fn short_selection() -> StreamSelection {
        StreamSelection::new(
            vec![PathComponent::Key(Arc::from("f"))],
            Some(vec![
                PathComponent::Key(Arc::from("p")),
                PathComponent::Key(Arc::from("r")),
            ]),
        )
    }

    fn serial(input: &[u8], options: StreamOptions) -> (Vec<StreamRecord>, bool) {
        let mut records = Vec::new();
        let result = stream_json_selected_records(input, options, selection(), |record| {
            records.push(record);
            Ok(())
        });
        (records, result.is_ok())
    }

    fn parallel(input: &[u8], options: StreamOptions) -> (Vec<StreamRecord>, bool) {
        parallel_with_selection(input, options, selection())
    }

    fn parallel_with_selection(
        input: &[u8],
        options: StreamOptions,
        selection: StreamSelection,
    ) -> (Vec<StreamRecord>, bool) {
        let mut records = Vec::new();
        let result = stream_json_selected_records_parallel(
            BufReader::with_capacity(7, input),
            options,
            selection,
            ParallelJsonOptions {
                batch_values: 1,
                batch_bytes: 32,
                in_flight_batches: 4,
                in_flight_bytes: 256,
            },
            None,
            |record| {
                records.push(record);
                Ok(())
            },
        );
        (records, result.is_ok())
    }

    fn serial_with_selection(
        input: &[u8],
        options: StreamOptions,
        selection: StreamSelection,
    ) -> (Vec<StreamRecord>, bool) {
        let mut records = Vec::new();
        let result = stream_json_selected_records(input, options, selection, |record| {
            records.push(record);
            Ok(())
        });
        (records, result.is_ok())
    }

    #[test]
    fn parallel_batches_match_serial_projection_and_duplicate_order() {
        let input = br#"{"before":"a\\u0062","features":[{"properties":{"release":3,"release":4},"geometry":[1,2]},{"properties":{}},{"properties":{"release":{"x":5}}}],"after":true}"#;
        assert_eq!(
            parallel(input, StreamOptions::default()),
            serial(input, StreamOptions::default())
        );
    }

    #[test]
    fn parallel_non_array_target_matches_serial_behavior() {
        let input = br#"{"features":{"properties":{"release":3}}}"#;
        assert_eq!(
            parallel(input, StreamOptions::default()),
            serial(input, StreamOptions::default())
        );
    }

    #[test]
    fn parallel_nested_key_and_index_prefixes_match_serial() {
        let cases = [
            (br#"[{"value":2},{"value":1}]"#.as_slice(), Vec::new()),
            (
                br#"{"root":{"features":[{"value":2},{"value":1}]}}"#.as_slice(),
                vec![
                    PathComponent::Key(Arc::from("root")),
                    PathComponent::Key(Arc::from("features")),
                ],
            ),
            (
                br#"{"roots":[{"skip":true},{"features":[{"value":2},{"value":1}]}]}"#.as_slice(),
                vec![
                    PathComponent::Key(Arc::from("roots")),
                    PathComponent::Index(1),
                    PathComponent::Key(Arc::from("features")),
                ],
            ),
        ];
        for (input, prefix) in cases {
            let selected =
                StreamSelection::new(prefix, Some(vec![PathComponent::Key(Arc::from("value"))]));
            assert_eq!(
                parallel_with_selection(input, StreamOptions::default(), selected.clone()),
                serial_with_selection(input, StreamOptions::default(), selected)
            );
        }

        let non_array = br#"{"roots":[null,{"features":{"value":2}}]}"#;
        let selected = StreamSelection::new(
            vec![
                PathComponent::Key(Arc::from("roots")),
                PathComponent::Index(1),
                PathComponent::Key(Arc::from("features")),
            ],
            Some(vec![PathComponent::Key(Arc::from("value"))]),
        );
        assert_eq!(
            parallel_with_selection(non_array, StreamOptions::default(), selected.clone()),
            serial_with_selection(non_array, StreamOptions::default(), selected)
        );
    }

    #[test]
    fn worker_diagnostics_are_rebased_to_document_lines_and_columns() {
        let input = b"{\n  \"features\": [\n    {\"properties\": {\"release\": 1}},\n    {\"properties\": {\"release\": truX}}\n  ]\n}";
        let serial = stream_json_selected_records(
            input.as_slice(),
            StreamOptions::default(),
            selection(),
            |_| Ok(()),
        )
        .unwrap_err()
        .to_string();
        let parallel = stream_json_selected_records_parallel(
            BufReader::with_capacity(7, input.as_slice()),
            StreamOptions::default(),
            selection(),
            ParallelJsonOptions::default(),
            None,
            |_| Ok(()),
        )
        .unwrap_err()
        .to_string();
        assert_eq!(parallel, serial);
    }

    #[test]
    fn parallel_worker_matches_serial_for_large_but_admitted_exponents() {
        let input = br#"{"features":[{"properties":{"release":1e1000001}}]}"#;
        let admitted_serial = serial(input, StreamOptions::default());
        let admitted_parallel = parallel(input, StreamOptions::default());
        assert!(admitted_serial.1);
        assert_eq!(admitted_parallel, admitted_serial);

        let limited = StreamOptions {
            maximum_token_bytes: 8,
            ..StreamOptions::default()
        };
        let limited_serial = serial(input, limited);
        let limited_parallel = parallel(input, limited);
        assert!(
            !limited_serial.1,
            "the explicit token envelope must reject the exponent"
        );
        assert_eq!(limited_parallel, limited_serial);
    }

    #[test]
    fn parallel_worker_rejects_exponents_beyond_the_default_numeric_envelope() {
        let input = br#"{"features":[{"properties":{"release":1e2000000001}}]}"#;
        let serial = serial(input, StreamOptions::default());
        let parallel = parallel(input, StreamOptions::default());
        assert!(!serial.1, "serial decoding accepted the oversized exponent");
        assert!(
            !parallel.1,
            "parallel decoding accepted the oversized exponent"
        );
        assert_eq!(parallel.0, serial.0);
    }

    #[test]
    fn parallel_rejects_the_same_malformed_and_limited_inputs() {
        let cases = [
            (
                br#"{"features":[{"properties":{"release":1},"x":[1,]}]}"#.as_slice(),
                StreamOptions::default(),
            ),
            (br#"{"features":[1,]}"#.as_slice(), StreamOptions::default()),
            (br#"{"features":[,1]}"#.as_slice(), StreamOptions::default()),
            (
                br#"{"features":[{}]}"#.as_slice(),
                StreamOptions {
                    maximum_depth: 2,
                    ..StreamOptions::default()
                },
            ),
            (
                br#"{"outside":"too long","features":[]}"#.as_slice(),
                StreamOptions {
                    maximum_token_bytes: 3,
                    ..StreamOptions::default()
                },
            ),
        ];
        for (input, options) in cases {
            let serial = serial(input, options);
            let parallel = parallel(input, options);
            assert!(!serial.1);
            assert!(!parallel.1);
        }
    }

    #[test]
    fn parallel_selected_and_skipped_tokens_share_json_limits() {
        let cases = [
            (br#"{"o":"long","f":[{"p":{"r":1}}]}"#.as_slice(), 3),
            (br#"{"o":"ok","f":[{"p":{"r":"long"}}]}"#.as_slice(), 3),
            (br#"{"o":"\u0061\u0062","f":[{"p":{"r":1}}]}"#.as_slice(), 1),
            (
                br#"{"o":"o","f":[{"p":{"r":"\u0061\u0062"}}]}"#.as_slice(),
                1,
            ),
        ];
        for (input, maximum_token_bytes) in cases {
            let options = StreamOptions {
                maximum_token_bytes,
                ..StreamOptions::default()
            };
            let serial = serial_with_selection(input, options, short_selection());
            let parallel = parallel_with_selection(input, options, short_selection());
            assert!(
                !serial.1,
                "the input must exceed the selected or skipped limit"
            );
            assert!(!parallel.1, "parallel framing must reject the same input");
            assert_eq!(parallel, serial);
        }

        let options = StreamOptions {
            maximum_token_bytes: 1,
            ..StreamOptions::default()
        };
        let input = br#"{"o":"o","f":[{"p":{"r":1}}]}"#;
        assert_eq!(
            parallel_with_selection(input, options, short_selection()),
            serial_with_selection(input, options, short_selection())
        );
    }

    #[test]
    fn parallel_skipped_scalar_validation_is_bounded_and_keyword_exact() {
        let input = br"[trueeeeeeeeeeeee,[]]";
        let limited = StreamOptions {
            maximum_token_bytes: 0,
            ..StreamOptions::default()
        };
        let result = stream_json_selected_records_parallel(
            BufReader::with_capacity(1, input.as_slice()),
            limited,
            StreamSelection::new(vec![PathComponent::Index(1)], None),
            ParallelJsonOptions::default(),
            None,
            |_| Ok(()),
        )
        .unwrap_err();
        assert!(matches!(result, FormatError::Resource("token-bytes")));

        let input = br"[true,[]]";
        let mut records = Vec::new();
        let result = stream_json_selected_records_parallel(
            BufReader::with_capacity(1, input.as_slice()),
            limited,
            StreamSelection::new(
                vec![
                    PathComponent::Index(1),
                    PathComponent::Key(Arc::from("missing")),
                ],
                None,
            ),
            ParallelJsonOptions::default(),
            None,
            |record| {
                records.push(record);
                Ok(())
            },
        );
        assert!(
            result.is_ok(),
            "true remains exempt when the budget is zero"
        );
    }

    #[test]
    fn parallel_capture_limit_is_enforced_before_batch_growth() {
        let input = br#"{"features":[{"properties":{"release":"long-value"}}]}"#;
        let result = stream_json_selected_records_parallel(
            BufReader::with_capacity(1, input.as_slice()),
            StreamOptions::default(),
            selection(),
            ParallelJsonOptions {
                in_flight_bytes: 4,
                ..ParallelJsonOptions::default()
            },
            None,
            |_| Ok(()),
        )
        .unwrap_err();
        assert!(matches!(
            result,
            FormatError::Resource("parallel-decode-in-flight-bytes")
        ));
    }

    #[test]
    fn parallel_honors_cancellation_and_in_flight_byte_limits() {
        let input = br#"{"features":[{"properties":{"release":1}}]}"#;
        let cancellation = Arc::new(AtomicBool::new(true));
        let cancelled = stream_json_selected_records_parallel(
            BufReader::new(input.as_slice()),
            StreamOptions::default(),
            selection(),
            ParallelJsonOptions::default(),
            Some(Arc::clone(&cancellation)),
            |_| Ok(()),
        );
        assert!(cancelled.is_err());
        assert!(cancellation.load(Ordering::Relaxed));

        let limited = stream_json_selected_records_parallel(
            BufReader::new(input.as_slice()),
            StreamOptions::default(),
            selection(),
            ParallelJsonOptions {
                in_flight_bytes: 1,
                ..ParallelJsonOptions::default()
            },
            None,
            |_| Ok(()),
        )
        .unwrap_err();
        assert!(
            limited
                .to_string()
                .contains("parallel-decode-in-flight-bytes")
        );
    }

    #[test]
    fn cancellation_reader_stops_an_active_decode() {
        let cancellation = Arc::new(AtomicBool::new(false));
        let bytes = vec![0; 32 * 1024];
        let mut reader = CancellationReader {
            bytes: &bytes,
            cancellation: Arc::clone(&cancellation),
            external_cancellation: None,
            bytes_until_check: 0,
        };
        let mut output = [0; 16 * 1024];
        assert_eq!(reader.read(&mut output).unwrap(), output.len());
        cancellation.store(true, Ordering::Relaxed);
        assert_eq!(
            reader.read(&mut output).unwrap_err().kind(),
            std::io::ErrorKind::Other
        );
    }

    #[test]
    fn scheduler_delivers_the_earliest_failure_and_bounds_reordering() {
        let mut scheduler = Scheduler::new(
            selection(),
            StreamOptions::default(),
            ParallelJsonOptions::default(),
            None,
        );
        scheduler.outstanding.push_back((0, 4));
        scheduler.outstanding.push_back((1, 4));
        scheduler.outstanding_bytes = 8;
        scheduler.store(Completion {
            ordinal: 1,
            retained_bytes: 4,
            outcome: BatchOutcome {
                records: Vec::new(),
                error: Some(super::parse_error("later failure")),
                depth_high_water: 0,
            },
        });
        let mut emit = |_| Ok(());
        scheduler.deliver_ready(&mut emit).unwrap();
        assert_eq!(scheduler.completed.len(), 1);
        assert_eq!(scheduler.observations.reordered_batches_high_water, 1);

        scheduler.store(Completion {
            ordinal: 0,
            retained_bytes: 4,
            outcome: BatchOutcome {
                records: Vec::new(),
                error: Some(super::parse_error("earliest failure")),
                depth_high_water: 0,
            },
        });
        let error = scheduler.deliver_ready(&mut emit).unwrap_err();
        assert!(error.to_string().contains("earliest failure"));
    }
}
