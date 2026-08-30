use std::{
    collections::{BTreeMap, VecDeque},
    io::BufRead,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender, TryRecvError},
    },
};

use tq_core::{Number, PathComponent};

use crate::{
    FormatError, InputFormat, StreamOptions, StreamRecord, StreamSelection,
    stream_json_selected_records,
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
}

struct Completion {
    ordinal: usize,
    retained_bytes: usize,
    outcome: BatchOutcome,
}

struct BatchOutcome {
    records: Vec<StreamRecord>,
    error: Option<FormatError>,
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
                }
            } else {
                decode_work(&work.bytes, stream_options, &selection, work.first_index)
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
) -> BatchOutcome {
    let Some(first_index) = first_index else {
        let mut records = Vec::new();
        let result = stream_json_selected_records(bytes, options, selection.clone(), |record| {
            records.push(record);
            Ok(())
        });
        return BatchOutcome {
            records,
            error: result.err(),
        };
    };
    let relative = StreamSelection::new(
        Vec::new(),
        selection.projection().map(<[PathComponent]>::to_vec),
    );
    let options = StreamOptions {
        maximum_depth: options.maximum_depth.saturating_sub(1),
        ..options
    };
    let mut records = Vec::new();
    let result = stream_json_selected_records(bytes, options, relative, |record| {
        records.push(record.rebase_array_item(selection.prefix(), first_index));
        Ok(())
    });
    BatchOutcome {
        records,
        error: result.err(),
    }
}

/// Frames one proven root-object array and decodes its element batches on the
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
    let [PathComponent::Key(selected_key)] = selection.prefix() else {
        stream_json_selected_records(reader, stream_options, selection, emit)?;
        return Ok(ParallelJsonObservations::default());
    };
    let selected_key = Arc::clone(selected_key);
    let mut framer = Framer::new(&mut reader, stream_options);
    framer.skip_ws()?;
    if framer.peek()? != Some(b'{') {
        drop(framer);
        stream_json_selected_records(reader, stream_options, selection, emit)?;
        return Ok(ParallelJsonObservations::default());
    }

    let mut scheduler = Scheduler::new(selection, stream_options, parallel_options, cancellation);
    let framing = framer.parse_root_object(&selected_key, parallel_options, &mut |index, bytes| {
        scheduler.submit(index, bytes, &mut emit)
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
}

impl<'a, R: BufRead> Framer<'a, R> {
    fn new(reader: &'a mut R, options: StreamOptions) -> Self {
        Self {
            reader,
            options,
            capture: None,
        }
    }

    fn parse_root_object<F>(
        &mut self,
        selected_key: &str,
        parallel: ParallelJsonOptions,
        submit: &mut F,
    ) -> Result<(), FormatError>
    where
        F: FnMut(Option<usize>, Vec<u8>) -> Result<(), FormatError>,
    {
        if self.options.maximum_depth == 0 {
            return Err(parse_error("stream depth limit exceeded"));
        }
        self.expect(b'{')?;
        self.skip_ws()?;
        if self.take_if(b'}')? {
            return self.finish_document();
        }
        loop {
            let key = self.read_string()?;
            self.skip_ws()?;
            self.expect(b':')?;
            self.skip_ws()?;
            if key == selected_key {
                self.parse_selected_value(selected_key, parallel, submit)?;
            } else {
                self.scan_value(1)?;
            }
            self.skip_ws()?;
            if self.take_if(b'}')? {
                break;
            }
            self.expect(b',')?;
            self.skip_ws()?;
        }
        self.finish_document()
    }

    fn parse_selected_value<F>(
        &mut self,
        selected_key: &str,
        parallel: ParallelJsonOptions,
        submit: &mut F,
    ) -> Result<(), FormatError>
    where
        F: FnMut(Option<usize>, Vec<u8>) -> Result<(), FormatError>,
    {
        if self.peek()? != Some(b'[') {
            let mut wrapper = Vec::new();
            wrapper.push(b'{');
            wrapper.extend_from_slice(
                serde_json::to_string(selected_key)
                    .map_err(|error| json_error(&error))?
                    .as_bytes(),
            );
            wrapper.push(b':');
            self.capture = Some(wrapper);
            self.scan_value(1)?;
            let mut wrapper = self.capture.take().expect("target capture is active");
            wrapper.push(b'}');
            return submit(None, wrapper);
        }
        self.check_container_depth(1)?;
        self.expect(b'[')?;
        self.skip_ws()?;
        if self.take_if(b']')? {
            return Ok(());
        }

        let mut first_index = 0usize;
        let mut count = 0usize;
        let mut batch = Vec::with_capacity(parallel.batch_bytes.clamp(2, 8 * 1024 * 1024));
        batch.push(b'[');
        loop {
            if count > 0 {
                batch.push(b',');
            }
            self.capture = Some(batch);
            self.scan_raw_array_element()?;
            batch = self.capture.take().expect("element capture is active");
            count = count.saturating_add(1);
            self.skip_ws()?;
            let ended = self.peek()? == Some(b']');
            if count >= parallel.batch_values.max(1)
                || batch.len() >= parallel.batch_bytes.max(1)
                || ended
            {
                batch.push(b']');
                submit(Some(first_index), batch)?;
                first_index = first_index.saturating_add(count);
                count = 0;
                batch = Vec::with_capacity(parallel.batch_bytes.clamp(2, 8 * 1024 * 1024));
                batch.push(b'[');
            }
            if self.take_if(b']')? {
                break;
            }
            self.expect(b',')?;
            self.skip_ws()?;
        }
        Ok(())
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
            Some(b't') => self.scan_literal(b"true"),
            Some(b'f') => self.scan_literal(b"false"),
            Some(b'n') => self.scan_literal(b"null"),
            Some(b'-' | b'0'..=b'9') => self.scan_number(),
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
        let decoded: String = serde_json::from_slice(&bytes).map_err(|error| json_error(&error))?;
        if decoded.len() > self.options.maximum_token_bytes {
            return Err(parse_error("input resource limit exceeded: token-bytes"));
        }
        Ok(decoded)
    }

    fn scan_string(&mut self) -> Result<(), FormatError> {
        let validate_token = self.capture.is_none();
        if validate_token {
            self.capture = Some(Vec::new());
        }
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
                self.consume_slice(used)?;
                continue;
            }
            match self.peek()? {
                Some(b'"') => {
                    self.consume_slice(1)?;
                    if validate_token {
                        let bytes = self.capture.take().expect("string capture is active");
                        let decoded: String =
                            serde_json::from_slice(&bytes).map_err(|error| json_error(&error))?;
                        if decoded.len() > self.options.maximum_token_bytes {
                            return Err(parse_error("input resource limit exceeded: token-bytes"));
                        }
                    }
                    return Ok(());
                }
                Some(b'\\') => {
                    self.consume_slice(1)?;
                    let escape = self.take()?.ok_or_else(|| parse_error("EOF in escape"))?;
                    if escape == b'u' {
                        for _ in 0..4 {
                            let digit = self
                                .take()?
                                .ok_or_else(|| parse_error("EOF in unicode escape"))?;
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

    fn scan_literal(&mut self, literal: &[u8]) -> Result<(), FormatError> {
        for expected in literal {
            if self.take()? != Some(*expected) {
                return Err(parse_error("invalid literal"));
            }
        }
        Ok(())
    }

    fn scan_number(&mut self) -> Result<(), FormatError> {
        let mut bytes = Vec::new();
        while let Some(byte) = self.peek()? {
            if byte.is_ascii_digit() || matches!(byte, b'-' | b'+' | b'.' | b'e' | b'E') {
                bytes.push(byte);
                self.consume_slice(1)?;
            } else {
                break;
            }
        }
        if self.capture.is_none() {
            if bytes.len() > self.options.maximum_token_bytes {
                return Err(parse_error("input resource limit exceeded: token-bytes"));
            }
            let literal = std::str::from_utf8(&bytes).map_err(|_| parse_error("invalid number"))?;
            Number::validate_literal(literal).map_err(|error| parse_error(error.to_string()))?;
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
        let available = self.reader.fill_buf()?;
        if length > available.len() {
            return Err(parse_error("internal JSON framing boundary error"));
        }
        if let Some(capture) = self.capture.as_mut() {
            capture.extend_from_slice(&available[..length]);
        }
        self.reader.consume(length);
        Ok(())
    }
}

fn parse_error(message: impl Into<String>) -> FormatError {
    FormatError::Parse {
        format: InputFormat::Json,
        message: message.into(),
    }
}

fn json_error(error: &serde_json::Error) -> FormatError {
    parse_error(error.to_string())
}

fn cancelled_error() -> FormatError {
    parse_error("interrupted")
}

#[cfg(test)]
mod tests {
    use std::{
        io::BufReader,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };

    use tq_core::PathComponent;

    use super::{
        ParallelJsonOptions, StreamOptions, StreamRecord, StreamSelection,
        stream_json_selected_records, stream_json_selected_records_parallel,
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

    fn serial(input: &[u8], options: StreamOptions) -> (Vec<StreamRecord>, bool) {
        let mut records = Vec::new();
        let result = stream_json_selected_records(input, options, selection(), |record| {
            records.push(record);
            Ok(())
        });
        (records, result.is_ok())
    }

    fn parallel(input: &[u8], options: StreamOptions) -> (Vec<StreamRecord>, bool) {
        let mut records = Vec::new();
        let result = stream_json_selected_records_parallel(
            BufReader::with_capacity(7, input),
            options,
            selection(),
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
}
