//! Incremental jq path/value stream projection for JSON and TOON.

use std::{fmt, io::BufRead, sync::Arc};

use serde::de::{self, DeserializeSeed, Error as _, IgnoredAny, MapAccess, SeqAccess, Visitor};
use tq_core::{Number, Object, PathComponent, SourceId, Value};
use tq_toon::{DecodeIntoError, Decoder, Event, EventConsumer, Scalar};

use crate::{FormatError, InputFormat, JsonEventOptions, decode_json_events_with_options};

const SERDE_JSON_NUMBER_TOKEN: &str = "$serde_json::private::Number";

/// Limits and error behavior for explicit jq path/value streaming.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StreamOptions {
    /// Maximum active JSON containers.
    pub maximum_depth: usize,
    /// Maximum decoded bytes in one string, key, or number token.
    pub maximum_token_bytes: usize,
    /// Emit JSON parse failures as jq-shaped stream error values.
    pub errors_as_values: bool,
}

impl Default for StreamOptions {
    fn default() -> Self {
        Self {
            maximum_depth: 256,
            maximum_token_bytes: 8 * 1024 * 1024,
            errors_as_values: false,
        }
    }
}

/// One decoded jq stream record before wrapper-value allocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamRecord {
    /// Ordered key and index path.
    pub path: Vec<PathComponent>,
    /// Leaf or empty-container value. `None` marks a completed non-empty container.
    pub value: Option<Value>,
    raw: bool,
}

/// Static decoder path admitted by an automatic projection proof.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamSelection {
    prefix: Vec<PathComponent>,
    projection: Option<Vec<PathComponent>>,
}

impl StreamSelection {
    /// Creates a selection for direct children below `prefix` and an optional
    /// path projected from each child.
    #[must_use]
    pub fn new(prefix: Vec<PathComponent>, projection: Option<Vec<PathComponent>>) -> Self {
        Self { prefix, projection }
    }

    /// Returns the statically selected array path.
    #[must_use]
    pub fn prefix(&self) -> &[PathComponent] {
        &self.prefix
    }

    /// Returns the static element-local projection, when present.
    #[must_use]
    pub fn projection(&self) -> Option<&[PathComponent]> {
        self.projection.as_deref()
    }

    fn keeps(&self, path: &[PathComponent]) -> bool {
        if path.len() <= self.prefix.len() || !path.starts_with(&self.prefix) {
            return false;
        }
        let relative = &path[self.prefix.len()..];
        if relative.len() == 1 {
            return true;
        }
        let Some(projection) = self.projection.as_deref() else {
            return true;
        };
        let item_relative = &relative[1..];
        projection.starts_with(item_relative)
            || item_relative.starts_with(projection)
            || path_kind_mismatch(item_relative, projection)
    }

    fn tracks(&self, path: &[PathComponent]) -> bool {
        if path.len() <= self.prefix.len() {
            return self.prefix.starts_with(path);
        }
        self.keeps(path)
    }
}

impl StreamRecord {
    /// Decomposes a normal structural record into its path and optional value.
    #[must_use]
    pub fn into_parts(self) -> (Vec<PathComponent>, Option<Value>) {
        (self.path, self.value)
    }

    fn into_value(self) -> Value {
        if self.raw {
            return self.value.unwrap_or(Value::Null);
        }
        let mut parts = vec![path_value(&self.path)];
        if let Some(value) = self.value {
            parts.push(value);
        }
        Value::array(parts)
    }

    fn path(path: Vec<PathComponent>, value: Option<Value>) -> Self {
        Self {
            path,
            value,
            raw: false,
        }
    }

    pub(crate) fn rebase_array_item(
        mut self,
        prefix: &[PathComponent],
        first_index: usize,
    ) -> Self {
        if self.raw || self.path.is_empty() {
            return self;
        }
        let PathComponent::Index(local_index) = self.path[0] else {
            return self;
        };
        let mut path = Vec::with_capacity(prefix.len().saturating_add(self.path.len()));
        path.extend_from_slice(prefix);
        path.push(PathComponent::Index(
            first_index.saturating_add(local_index),
        ));
        path.extend(self.path.drain(1..));
        self.path = path;
        self
    }

    fn raw(value: Value) -> Self {
        Self {
            path: Vec::new(),
            value: Some(value),
            raw: true,
        }
    }
}

/// Streams one JSON document as jq-compatible `[path,value]` leaf records and
/// `[path]` container-end records without constructing its DOM.
///
/// The callback is invoked before more input is consumed. Returning an error
/// stops decoding immediately; callers that need a typed callback error can
/// retain it beside the closure and return a bounded marker string here.
///
/// # Errors
///
/// Returns JSON syntax, numeric-envelope, depth, or callback failures.
pub fn stream_json<R, F>(reader: R, options: StreamOptions, mut emit: F) -> Result<(), FormatError>
where
    R: std::io::Read,
    F: FnMut(Value) -> Result<(), String>,
{
    let mut emit_record = |record: StreamRecord| emit(record.into_value());
    let mut projector = Projector::new(
        options.maximum_depth,
        options.maximum_token_bytes,
        &mut emit_record,
    );
    let mut deserializer = serde_json::Deserializer::from_reader(reader);
    let decoded = StreamSeed {
        projector: &mut projector,
    }
    .deserialize(&mut deserializer)
    .and_then(|()| deserializer.end());
    match decoded {
        Ok(()) => Ok(()),
        Err(error) if options.errors_as_values && !is_resource_error(&error) => projector
            .error_value(jq_stream_error(&error))
            .map_err(|message| FormatError::Parse {
                format: InputFormat::Json,
                message,
            }),
        Err(error) => Err(FormatError::Parse {
            format: InputFormat::Json,
            message: error.to_string(),
        }),
    }
}

/// Streams JSON through the shared structural decoder without constructing jq
/// `[path, value]` wrapper values.
///
/// # Errors
///
/// Returns JSON syntax, numeric-envelope, depth, token, or callback failures.
pub fn stream_json_records<R, F>(
    reader: R,
    options: StreamOptions,
    mut emit: F,
) -> Result<(), FormatError>
where
    R: std::io::Read,
    F: FnMut(StreamRecord) -> Result<(), String>,
{
    let mut consumer = EventProjector {
        projector: Projector::new(
            options.maximum_depth,
            options.maximum_token_bytes,
            &mut emit,
        ),
    };
    decode_json_events_with_options(
        reader,
        SourceId::new(0),
        &mut consumer,
        JsonEventOptions {
            maximum_depth: options.maximum_depth,
            maximum_token_bytes: options.maximum_token_bytes,
        },
    )
    .map_err(|message| FormatError::Parse {
        format: InputFormat::Json,
        message,
    })
}

/// Streams only records needed by a proven static automatic projection.
/// Discarded values are still fully decoded so syntax and resource failures
/// remain observable.
///
/// # Errors
///
/// Returns JSON syntax, numeric-envelope, depth, token, or callback failures.
pub fn stream_json_selected_records<R, F>(
    reader: R,
    options: StreamOptions,
    selection: StreamSelection,
    mut emit: F,
) -> Result<(), FormatError>
where
    R: std::io::Read,
    F: FnMut(StreamRecord) -> Result<(), String>,
{
    let mut projector = Projector::selected(
        options.maximum_depth,
        options.maximum_token_bytes,
        selection,
        &mut emit,
    );
    let mut deserializer = serde_json::Deserializer::from_reader(reader);
    StreamSeed {
        projector: &mut projector,
    }
    .deserialize(&mut deserializer)
    .and_then(|()| deserializer.end())
    .map_err(|error| FormatError::Parse {
        format: InputFormat::Json,
        message: error.to_string(),
    })
}

fn jq_stream_error(error: &serde_json::Error) -> String {
    let message = error.to_string();
    if message.starts_with("expected value at line ") {
        // jq consumes the invalid bare token before reporting its endpoint.
        // serde_json stops at the first byte; the MVP stream-error grammar's
        // only bare-token class is a three-byte JSON literal prefix.
        return format!(
            "Invalid numeric literal at line {}, column {}",
            error.line(),
            error.column().saturating_add(3)
        );
    }
    message
}

/// Streams one TOON document through its query-independent event decoder and
/// projects the same jq path/value contract as [`stream_json`].
///
/// # Errors
///
/// Returns strict TOON decoding, resource, structural, or callback failures.
pub fn stream_toon<R, F>(
    reader: R,
    config: tq_toon::DecoderConfig,
    options: StreamOptions,
    mut emit: F,
) -> Result<(), FormatError>
where
    R: BufRead,
    F: FnMut(Value) -> Result<(), String>,
{
    let mut emit_record = |record: StreamRecord| emit(record.into_value());
    let mut decoder = Decoder::new(reader, SourceId::new(0), config);
    let mut consumer = EventProjector {
        projector: Projector::new(
            options.maximum_depth,
            options.maximum_token_bytes,
            &mut emit_record,
        ),
    };
    decoder.decode_into(&mut consumer).map_err(|error| {
        let message = match error {
            DecodeIntoError::Decode(error) => error.to_string(),
            DecodeIntoError::Consumer(error) => error,
        };
        FormatError::Parse {
            format: InputFormat::Toon,
            message,
        }
    })
}

/// Streams TOON structural events without constructing jq wrapper values.
///
/// # Errors
///
/// Returns strict decoding, resource, structural, or callback failures.
pub fn stream_toon_records<R, F>(
    reader: R,
    config: tq_toon::DecoderConfig,
    options: StreamOptions,
    mut emit: F,
) -> Result<(), FormatError>
where
    R: BufRead,
    F: FnMut(StreamRecord) -> Result<(), String>,
{
    let mut decoder = Decoder::new(reader, SourceId::new(0), config);
    let mut consumer = EventProjector {
        projector: Projector::new(
            options.maximum_depth,
            options.maximum_token_bytes,
            &mut emit,
        ),
    };
    decoder.decode_into(&mut consumer).map_err(|error| {
        let message = match error {
            DecodeIntoError::Decode(error) => error.to_string(),
            DecodeIntoError::Consumer(error) => error,
        };
        FormatError::Parse {
            format: InputFormat::Toon,
            message,
        }
    })
}

/// Streams only TOON records needed by a proven static automatic projection.
///
/// # Errors
///
/// Returns strict decoding, resource, structural, or callback failures.
pub fn stream_toon_selected_records<R, F>(
    reader: R,
    config: tq_toon::DecoderConfig,
    options: StreamOptions,
    selection: StreamSelection,
    mut emit: F,
) -> Result<(), FormatError>
where
    R: BufRead,
    F: FnMut(StreamRecord) -> Result<(), String>,
{
    let mut decoder = Decoder::new(reader, SourceId::new(0), config);
    let mut consumer = EventProjector {
        projector: Projector::selected(
            options.maximum_depth,
            options.maximum_token_bytes,
            selection,
            &mut emit,
        ),
    };
    decoder.decode_into(&mut consumer).map_err(|error| {
        let message = match error {
            DecodeIntoError::Decode(error) => error.to_string(),
            DecodeIntoError::Consumer(error) => error,
        };
        FormatError::Parse {
            format: InputFormat::Toon,
            message,
        }
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ContainerKind {
    Array,
    Object,
}

#[derive(Debug)]
struct Frame {
    kind: ContainerKind,
    path: Option<Vec<PathComponent>>,
    children: usize,
    pending_key: Option<Arc<str>>,
}

struct Projector<'a, F> {
    frames: Vec<Frame>,
    last_path: Vec<PathComponent>,
    maximum_depth: usize,
    maximum_token_bytes: usize,
    selection: Option<StreamSelection>,
    emit: &'a mut F,
}

impl<'a, F> Projector<'a, F>
where
    F: FnMut(StreamRecord) -> Result<(), String>,
{
    fn new(maximum_depth: usize, maximum_token_bytes: usize, emit: &'a mut F) -> Self {
        Self {
            frames: Vec::new(),
            last_path: Vec::new(),
            maximum_depth,
            maximum_token_bytes,
            selection: None,
            emit,
        }
    }

    fn selected(
        maximum_depth: usize,
        maximum_token_bytes: usize,
        selection: StreamSelection,
        emit: &'a mut F,
    ) -> Self {
        Self {
            frames: Vec::new(),
            last_path: Vec::new(),
            maximum_depth,
            maximum_token_bytes,
            selection: Some(selection),
            emit,
        }
    }

    fn begin(&mut self, kind: ContainerKind) -> Result<(), String> {
        if self.frames.len() >= self.maximum_depth {
            return Err("stream depth limit exceeded".to_owned());
        }
        let path = self.take_value_path()?;
        self.frames.push(Frame {
            kind,
            path,
            children: 0,
            pending_key: None,
        });
        Ok(())
    }

    fn key(&mut self, key: String) -> Result<(), String> {
        self.check_token(key.len())?;
        let frame = self
            .frames
            .last_mut()
            .ok_or_else(|| "object key outside a container".to_owned())?;
        if frame.kind != ContainerKind::Object || frame.pending_key.is_some() {
            return Err("object key arrived in an invalid stream state".to_owned());
        }
        frame.pending_key = Some(key.into());
        Ok(())
    }

    fn scalar(&mut self, value: Value) -> Result<(), String> {
        let token_bytes = match &value {
            Value::Number(number) => number.to_string().len(),
            Value::String(value) => value.len(),
            Value::Null | Value::Bool(_) | Value::Array(_) | Value::Object(_) => 0,
        };
        self.check_token(token_bytes)?;
        let Some(path) = self.take_value_path()? else {
            return Ok(());
        };
        self.emit_pair(path, value)
    }

    fn structural_scalar(&mut self, value: Scalar) -> Result<(), String> {
        let Some(path) = self.take_value_path()? else {
            return Ok(());
        };
        self.emit_pair(
            path,
            match value {
                Scalar::Null => Value::Null,
                Scalar::Bool(value) => Value::Bool(value),
                Scalar::Number(value) => Value::Number(value),
                Scalar::String(value) => Value::string(value),
            },
        )
    }

    fn check_token(&self, bytes: usize) -> Result<(), String> {
        if bytes > self.maximum_token_bytes {
            return Err("input resource limit exceeded: token-bytes".to_owned());
        }
        Ok(())
    }

    fn end(&mut self, kind: ContainerKind) -> Result<(), String> {
        let frame = self
            .frames
            .pop()
            .ok_or_else(|| "container end without a matching start".to_owned())?;
        if frame.kind != kind || frame.pending_key.is_some() {
            return Err("mismatched container event".to_owned());
        }
        let Some(path) = frame.path else {
            return Ok(());
        };
        if frame.children == 0 {
            let empty = match kind {
                ContainerKind::Array => Value::array(Vec::new()),
                ContainerKind::Object => Value::object(Object::new()),
            };
            self.emit_pair(path, empty)
        } else if let Some(selection) = &self.selection {
            if selection.keeps(&path) {
                (self.emit)(StreamRecord::path(path.clone(), None))?;
            }
            self.last_path = path;
            Ok(())
        } else {
            let end_path = self.last_path.clone();
            (self.emit)(StreamRecord::path(end_path, None))?;
            self.last_path = path;
            Ok(())
        }
    }

    fn error_value(&mut self, message: String) -> Result<(), String> {
        let path = self.expected_path();
        let value = Value::array(vec![Value::string(message), path_value(&path)]);
        (self.emit)(StreamRecord::raw(value))
    }

    fn take_value_path(&mut self) -> Result<Option<Vec<PathComponent>>, String> {
        let Some(frame) = self.frames.last_mut() else {
            return Ok(Some(Vec::new()));
        };
        let component = match frame.kind {
            ContainerKind::Array => PathComponent::Index(frame.children),
            ContainerKind::Object => PathComponent::Key(
                frame
                    .pending_key
                    .take()
                    .ok_or_else(|| "object value has no key".to_owned())?,
            ),
        };
        frame.children = frame.children.saturating_add(1);
        let Some(parent) = frame.path.as_ref() else {
            return Ok(None);
        };
        let mut path = parent.clone();
        path.push(component);
        if self
            .selection
            .as_ref()
            .is_some_and(|selection| !selection.tracks(&path))
        {
            return Ok(None);
        }
        Ok(Some(path))
    }

    fn expected_path(&self) -> Vec<PathComponent> {
        let Some(frame) = self.frames.last() else {
            return Vec::new();
        };
        let Some(mut path) = frame.path.clone() else {
            return Vec::new();
        };
        match frame.kind {
            ContainerKind::Array => path.push(PathComponent::Index(frame.children)),
            ContainerKind::Object => {
                if let Some(key) = &frame.pending_key {
                    path.push(PathComponent::Key(Arc::clone(key)));
                }
            }
        }
        path
    }

    fn rejects_next_value(&self) -> bool {
        self.selection
            .as_ref()
            .is_some_and(|selection| !selection.tracks(&self.expected_path()))
    }

    fn emit_pair(&mut self, path: Vec<PathComponent>, value: Value) -> Result<(), String> {
        if self
            .selection
            .as_ref()
            .is_none_or(|selection| selection.keeps(&path))
        {
            (self.emit)(StreamRecord::path(path.clone(), Some(value)))?;
        }
        self.last_path = path;
        Ok(())
    }
}

fn path_kind_mismatch(actual: &[PathComponent], expected: &[PathComponent]) -> bool {
    for (actual, expected) in actual.iter().zip(expected) {
        if actual == expected {
            continue;
        }
        return matches!(actual, PathComponent::Key(_))
            != matches!(expected, PathComponent::Key(_));
    }
    false
}

fn path_value(path: &[PathComponent]) -> Value {
    Value::array(
        path.iter()
            .map(|component| match component {
                PathComponent::Key(key) => Value::string(Arc::clone(key)),
                PathComponent::Index(index) => Value::Number(
                    Number::parse(&index.to_string()).expect("usize is a valid bounded number"),
                ),
            })
            .collect::<Vec<_>>(),
    )
}

#[derive(Clone, Copy)]
struct DiscardSeed {
    depth: usize,
    maximum_depth: usize,
    maximum_token_bytes: usize,
}

impl<'de> DeserializeSeed<'de> for DiscardSeed {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(DiscardVisitor(self))
    }
}

struct DiscardVisitor(DiscardSeed);

impl DiscardVisitor {
    fn check_token<E: de::Error>(&self, bytes: usize) -> Result<(), E> {
        if bytes > self.0.maximum_token_bytes {
            return Err(E::custom("input resource limit exceeded: token-bytes"));
        }
        Ok(())
    }

    fn child_seed<E: de::Error>(&self) -> Result<DiscardSeed, E> {
        if self.0.depth >= self.0.maximum_depth {
            return Err(E::custom("stream depth limit exceeded"));
        }
        Ok(DiscardSeed {
            depth: self.0.depth.saturating_add(1),
            ..self.0
        })
    }

    fn validate_number<E: de::Error>(&self, literal: &str) -> Result<(), E> {
        self.check_token(literal.len())?;
        Number::validate_literal(literal).map_err(E::custom)
    }
}

impl<'de> Visitor<'de> for DiscardVisitor {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a discarded JSON value")
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        self.0.deserialize(deserializer)
    }

    fn visit_bool<E: de::Error>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
        self.check_token(decimal_i128_bytes(i128::from(value)))
    }

    fn visit_i128<E: de::Error>(self, value: i128) -> Result<Self::Value, E> {
        self.check_token(decimal_i128_bytes(value))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
        self.check_token(decimal_u128_bytes(u128::from(value)))
    }

    fn visit_u128<E: de::Error>(self, value: u128) -> Result<Self::Value, E> {
        self.check_token(decimal_u128_bytes(value))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
        self.validate_number(&value.to_string())
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        self.check_token(value.len())
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        self.check_token(value.len())
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let child = self.child_seed::<A::Error>()?;
        while sequence.next_element_seed(child)?.is_some() {}
        Ok(())
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let child = self.child_seed::<A::Error>()?;
        let Some(first) = map.next_key::<String>()? else {
            return Ok(());
        };
        if first == SERDE_JSON_NUMBER_TOKEN {
            let literal = map.next_value::<String>()?;
            self.validate_number::<A::Error>(&literal)?;
            if map.next_key::<IgnoredAny>()?.is_some() {
                return Err(A::Error::custom("invalid arbitrary-precision number"));
            }
            return Ok(());
        }
        self.check_token::<A::Error>(first.len())?;
        map.next_value_seed(child)?;
        while let Some(key) = map.next_key::<String>()? {
            self.check_token::<A::Error>(key.len())?;
            map.next_value_seed(child)?;
        }
        Ok(())
    }
}

fn decimal_i128_bytes(value: i128) -> usize {
    decimal_u128_bytes(value.unsigned_abs()).saturating_add(usize::from(value < 0))
}

fn decimal_u128_bytes(value: u128) -> usize {
    if value == 0 {
        1
    } else {
        usize::try_from(value.ilog10()).unwrap_or(usize::MAX) + 1
    }
}

struct StreamSeed<'a, 'b, F> {
    projector: &'a mut Projector<'b, F>,
}

impl<'de, F> DeserializeSeed<'de> for StreamSeed<'_, '_, F>
where
    F: FnMut(StreamRecord) -> Result<(), String>,
{
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if self.projector.rejects_next_value() {
            self.projector.take_value_path().map_err(D::Error::custom)?;
            DiscardSeed {
                depth: self.projector.frames.len(),
                maximum_depth: self.projector.maximum_depth,
                maximum_token_bytes: self.projector.maximum_token_bytes,
            }
            .deserialize(deserializer)
        } else {
            deserializer.deserialize_any(StreamVisitor {
                projector: self.projector,
            })
        }
    }
}

struct StreamVisitor<'a, 'b, F> {
    projector: &'a mut Projector<'b, F>,
}

impl<'de, F> Visitor<'de> for StreamVisitor<'_, '_, F>
where
    F: FnMut(StreamRecord) -> Result<(), String>,
{
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.projector.scalar(Value::Null).map_err(E::custom)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_unit()
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.projector.scalar(Value::Bool(value)).map_err(E::custom)
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.number(&value.to_string())
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.number(&value.to_string())
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .map_err(E::custom)
            .and_then(|value| self.projector.scalar(value).map_err(E::custom))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.projector
            .scalar(Value::string(value))
            .map_err(E::custom)
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.projector
            .scalar(Value::string(value))
            .map_err(E::custom)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        self.projector
            .begin(ContainerKind::Array)
            .map_err(A::Error::custom)?;
        while sequence
            .next_element_seed(StreamSeed {
                projector: self.projector,
            })?
            .is_some()
        {}
        self.projector
            .end(ContainerKind::Array)
            .map_err(A::Error::custom)
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let first = map.next_key::<String>()?;
        if first.as_deref() == Some(SERDE_JSON_NUMBER_TOKEN) {
            let literal = map.next_value::<String>()?;
            if map.next_key::<IgnoredAny>()?.is_some() {
                return Err(A::Error::custom("invalid arbitrary-precision number"));
            }
            return self.number(&literal);
        }
        self.projector
            .begin(ContainerKind::Object)
            .map_err(A::Error::custom)?;
        if let Some(key) = first {
            self.projector.key(key).map_err(A::Error::custom)?;
            map.next_value_seed(StreamSeed {
                projector: self.projector,
            })?;
        }
        while let Some(key) = map.next_key::<String>()? {
            self.projector.key(key).map_err(A::Error::custom)?;
            map.next_value_seed(StreamSeed {
                projector: self.projector,
            })?;
        }
        self.projector
            .end(ContainerKind::Object)
            .map_err(A::Error::custom)
    }
}

impl<F> StreamVisitor<'_, '_, F>
where
    F: FnMut(StreamRecord) -> Result<(), String>,
{
    fn number<E>(self, literal: &str) -> Result<(), E>
    where
        E: de::Error,
    {
        self.projector
            .check_token(literal.len())
            .map_err(E::custom)?;
        Number::parse(literal)
            .map(Value::Number)
            .map_err(E::custom)
            .and_then(|value| self.projector.scalar(value).map_err(E::custom))
    }
}

fn is_resource_error(error: &serde_json::Error) -> bool {
    error.to_string().contains("input resource limit exceeded")
}

struct EventProjector<'a, F> {
    projector: Projector<'a, F>,
}

impl<F> EventConsumer for EventProjector<'_, F>
where
    F: FnMut(StreamRecord) -> Result<(), String>,
{
    type Error = String;

    fn consume(&mut self, event: Event) -> Result<(), Self::Error> {
        match event {
            Event::DocumentStart { .. } | Event::DocumentEnd { .. } => Ok(()),
            Event::ObjectStart { .. } => self.projector.begin(ContainerKind::Object),
            Event::ObjectEnd { .. } => self.projector.end(ContainerKind::Object),
            Event::ArrayStart { .. } => self.projector.begin(ContainerKind::Array),
            Event::ArrayEnd { .. } => self.projector.end(ContainerKind::Array),
            Event::Key { value, .. } => self.projector.key(value.to_string()),
            Event::Scalar { value, .. } => self.projector.structural_scalar(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufReader, Cursor};

    use std::sync::Arc;

    use tq_core::{PathComponent, SourceId};

    use crate::{JsonEventOptions, decode_json_events_with_options};

    use super::{
        EventProjector, Projector, StreamOptions, StreamRecord, StreamSelection, stream_json,
        stream_json_records, stream_json_selected_records, stream_toon,
    };

    fn json_lines(values: &[tq_core::Value]) -> Vec<String> {
        values.iter().map(ToString::to_string).collect()
    }

    fn selected_outcome(
        input: &[u8],
        options: StreamOptions,
        selection: StreamSelection,
    ) -> (Vec<StreamRecord>, Result<(), String>) {
        let mut records = Vec::new();
        let result = stream_json_selected_records(input, options, selection, |record| {
            records.push(record);
            Ok(())
        })
        .map_err(|error| error.to_string());
        (records, result)
    }

    fn structural_selected_outcome(
        input: &[u8],
        options: StreamOptions,
        selection: StreamSelection,
    ) -> (Vec<StreamRecord>, Result<(), String>) {
        let mut records = Vec::new();
        let result = {
            let mut emit = |record| {
                records.push(record);
                Ok(())
            };
            let mut consumer = EventProjector {
                projector: Projector::selected(
                    options.maximum_depth,
                    options.maximum_token_bytes,
                    selection,
                    &mut emit,
                ),
            };
            decode_json_events_with_options(
                input,
                SourceId::new(0),
                &mut consumer,
                JsonEventOptions {
                    maximum_depth: options.maximum_depth,
                    maximum_token_bytes: options.maximum_token_bytes,
                },
            )
        };
        (records, result)
    }

    fn release_selection() -> StreamSelection {
        StreamSelection::new(
            vec![PathComponent::Key(Arc::from("features"))],
            Some(vec![
                PathComponent::Key(Arc::from("properties")),
                PathComponent::Key(Arc::from("release")),
            ]),
        )
    }

    #[test]
    fn json_and_toon_form_identical_jq_stream_records() {
        let expected = [
            r#"[["a",0],1]"#,
            r#"[["a",1],{}]"#,
            r#"[["a",1]]"#,
            r#"[["b"],[]]"#,
            r#"[["b"]]"#,
        ];
        let mut json = Vec::new();
        stream_json(
            br#"{"a":[1,{}],"b":[]}"#.as_slice(),
            StreamOptions::default(),
            |value| {
                json.push(value);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(json_lines(&json), expected);

        let mut toon = Vec::new();
        stream_toon(
            BufReader::new(Cursor::new(b"a[2]:\n  - 1\n  -\nb[0]:")),
            tq_toon::DecoderConfig::default(),
            StreamOptions::default(),
            |value| {
                toon.push(value);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(json_lines(&toon), expected);
    }

    #[test]
    fn stream_errors_include_the_next_value_path() {
        let mut values = Vec::new();
        stream_json(
            b"[1, bad, 2]".as_slice(),
            StreamOptions {
                errors_as_values: true,
                ..StreamOptions::default()
            },
            |value| {
                values.push(value);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(values[0].to_string(), "[[0],1]");
        assert!(values[1].to_string().ends_with(",[1]]"));
    }

    #[test]
    fn stream_enforces_token_limits_before_emitting() {
        let mut values = Vec::new();
        let error = stream_json(
            b"12345".as_slice(),
            StreamOptions {
                maximum_token_bytes: 3,
                ..StreamOptions::default()
            },
            |value| {
                values.push(value);
                Ok(())
            },
        )
        .unwrap_err()
        .to_string();
        assert!(values.is_empty());
        assert!(error.contains("token-bytes"));
        assert!(error.contains("resource limit"));
    }

    #[test]
    fn structural_json_records_avoid_jq_wrapper_values_and_keep_duplicates() {
        let mut records = Vec::new();
        stream_json_records(
            br#"{"items":[{"x":1,"x":2}]}"#.as_slice(),
            StreamOptions::default(),
            |record| {
                records.push(record.into_parts());
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(records.len(), 5);
        assert_eq!(records[0].0.len(), 3);
        assert_eq!(records[0].1.as_ref().unwrap().to_string(), "1");
        assert_eq!(records[1].1.as_ref().unwrap().to_string(), "2");
    }

    #[test]
    fn selected_records_discard_unrelated_object_members_but_keep_item_boundaries() {
        let mut records = Vec::new();
        stream_json_selected_records(
            br#"{"features":[{"geometry":{"coordinates":[1,2]},"properties":{"release":3,"other":4}},{"geometry":{"coordinates":[5,6]}}]}"#.as_slice(),
            StreamOptions::default(),
            StreamSelection::new(
                vec![PathComponent::Key(Arc::from("features"))],
                Some(vec![
                    PathComponent::Key(Arc::from("properties")),
                    PathComponent::Key(Arc::from("release")),
                ]),
            ),
            |record| {
                records.push(record.into_parts());
                Ok(())
            },
        )
        .unwrap();
        assert!(records.iter().all(|(path, _)| {
            !path
                .iter()
                .any(|component| matches!(component, PathComponent::Key(key) if &**key == "geometry" || &**key == "other"))
        }));
        assert!(records.iter().any(|(path, value)| {
            path.last().is_some_and(
                |component| matches!(component, PathComponent::Key(key) if &**key == "release"),
            ) && value.as_ref().is_some_and(|value| value.to_string() == "3")
        }));
        assert_eq!(
            records
                .iter()
                .filter(|(path, value)| path.len() == 2 && value.is_none())
                .count(),
            2
        );
    }

    #[test]
    fn selected_fast_discard_matches_structural_projection() {
        let input = br#"{"features":[{"geometry":{"coordinates":[1,-2.50e3],"label":"discard"},"properties":{"release":3,"other":4,"release":5}},{"geometry":{"coordinates":[]},"properties":{}},{"geometry":null}]}"#;
        let fast = selected_outcome(input, StreamOptions::default(), release_selection());
        let structural =
            structural_selected_outcome(input, StreamOptions::default(), release_selection());
        assert_eq!(fast, structural);
    }

    #[test]
    fn selected_fast_discard_preserves_discarded_subtree_failures() {
        let default = StreamOptions::default();
        let depth_limited = StreamOptions {
            maximum_depth: 6,
            ..default
        };
        let token_limited = StreamOptions {
            maximum_token_bytes: 10,
            ..default
        };
        let oversized_coefficient = format!(
            "{{\"features\":[{{\"geometry\":[{}],\"properties\":{{\"release\":1}}}}]}}",
            "1".repeat(4097)
        );
        let cases = [
            (
                br#"{"features":[{"properties":{"release":1},"geometry":[1,]}]}"#.to_vec(),
                default,
                "malformed",
            ),
            (
                br#"{"features":[{"properties":{"release":1},"geometry":[[[[[[0]]]]]]}]}"#.to_vec(),
                depth_limited,
                "depth",
            ),
            (
                br#"{"features":[{"properties":{"release":1},"geometry":{"coordinates":1}}]}"#
                    .to_vec(),
                token_limited,
                "token",
            ),
            (
                br#"{"features":[{"properties":{"release":1},"geometry":[1e1000001]}]}"#.to_vec(),
                default,
                "exponent",
            ),
            (oversized_coefficient.into_bytes(), default, "coefficient"),
        ];

        for (input, options, label) in cases {
            let fast = selected_outcome(&input, options, release_selection());
            let structural = structural_selected_outcome(&input, options, release_selection());
            assert_eq!(fast.0, structural.0, "records for {label}");
            assert!(fast.1.is_err(), "fast path accepted {label}");
            assert!(structural.1.is_err(), "structural path accepted {label}");
        }
    }
}
