//! Incremental jq path/value stream projection for JSON and TOON.

use std::{fmt, io::BufRead, sync::Arc};

use serde::de::{self, DeserializeSeed, Error as _, IgnoredAny, MapAccess, SeqAccess, Visitor};
use tq_core::{Number, Object, PathComponent, SourceId, Value};
use tq_toon::{DecodeIntoError, Decoder, Event, EventConsumer, Scalar};

use crate::{FormatError, InputFormat};

const SERDE_JSON_NUMBER_TOKEN: &str = "$serde_json::private::Number";

/// Limits and error behavior for explicit jq path/value streaming.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StreamOptions {
    /// Maximum active JSON containers.
    pub maximum_depth: usize,
    /// Emit JSON parse failures as jq-shaped stream error values.
    pub errors_as_values: bool,
}

impl Default for StreamOptions {
    fn default() -> Self {
        Self {
            maximum_depth: 256,
            errors_as_values: false,
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
    let mut projector = Projector::new(options.maximum_depth, &mut emit);
    let mut deserializer = serde_json::Deserializer::from_reader(reader);
    let decoded = StreamSeed {
        projector: &mut projector,
    }
    .deserialize(&mut deserializer)
    .and_then(|()| deserializer.end());
    match decoded {
        Ok(()) => Ok(()),
        Err(error) if options.errors_as_values => projector
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
    let mut decoder = Decoder::new(reader, SourceId::new(0), config);
    let mut consumer = ToonProjector {
        projector: Projector::new(options.maximum_depth, &mut emit),
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
    path: Vec<PathComponent>,
    children: usize,
    pending_key: Option<Arc<str>>,
}

struct Projector<'a, F> {
    frames: Vec<Frame>,
    last_path: Vec<PathComponent>,
    maximum_depth: usize,
    emit: &'a mut F,
}

impl<'a, F> Projector<'a, F>
where
    F: FnMut(Value) -> Result<(), String>,
{
    fn new(maximum_depth: usize, emit: &'a mut F) -> Self {
        Self {
            frames: Vec::new(),
            last_path: Vec::new(),
            maximum_depth,
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
        let path = self.take_value_path()?;
        self.emit_pair(path, value)
    }

    fn end(&mut self, kind: ContainerKind) -> Result<(), String> {
        let frame = self
            .frames
            .pop()
            .ok_or_else(|| "container end without a matching start".to_owned())?;
        if frame.kind != kind || frame.pending_key.is_some() {
            return Err("mismatched container event".to_owned());
        }
        if frame.children == 0 {
            let empty = match kind {
                ContainerKind::Array => Value::array(Vec::new()),
                ContainerKind::Object => Value::object(Object::new()),
            };
            self.emit_pair(frame.path, empty)
        } else {
            let path = self.last_path.clone();
            (self.emit)(Value::array(vec![path_value(&path)]))?;
            self.last_path = frame.path;
            Ok(())
        }
    }

    fn error_value(&mut self, message: String) -> Result<(), String> {
        let path = self.expected_path();
        (self.emit)(Value::array(vec![
            Value::string(message),
            path_value(&path),
        ]))
    }

    fn take_value_path(&mut self) -> Result<Vec<PathComponent>, String> {
        let Some(frame) = self.frames.last_mut() else {
            return Ok(Vec::new());
        };
        let mut path = frame.path.clone();
        match frame.kind {
            ContainerKind::Array => path.push(PathComponent::Index(frame.children)),
            ContainerKind::Object => path.push(PathComponent::Key(
                frame
                    .pending_key
                    .take()
                    .ok_or_else(|| "object value has no key".to_owned())?,
            )),
        }
        frame.children = frame.children.saturating_add(1);
        Ok(path)
    }

    fn expected_path(&self) -> Vec<PathComponent> {
        let Some(frame) = self.frames.last() else {
            return Vec::new();
        };
        let mut path = frame.path.clone();
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

    fn emit_pair(&mut self, path: Vec<PathComponent>, value: Value) -> Result<(), String> {
        (self.emit)(Value::array(vec![path_value(&path), value]))?;
        self.last_path = path;
        Ok(())
    }
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

struct StreamSeed<'a, 'b, F> {
    projector: &'a mut Projector<'b, F>,
}

impl<'de, F> DeserializeSeed<'de> for StreamSeed<'_, '_, F>
where
    F: FnMut(Value) -> Result<(), String>,
{
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(StreamVisitor {
            projector: self.projector,
        })
    }
}

struct StreamVisitor<'a, 'b, F> {
    projector: &'a mut Projector<'b, F>,
}

impl<'de, F> Visitor<'de> for StreamVisitor<'_, '_, F>
where
    F: FnMut(Value) -> Result<(), String>,
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
    F: FnMut(Value) -> Result<(), String>,
{
    fn number<E>(self, literal: &str) -> Result<(), E>
    where
        E: de::Error,
    {
        Number::parse(literal)
            .map(Value::Number)
            .map_err(E::custom)
            .and_then(|value| self.projector.scalar(value).map_err(E::custom))
    }
}

struct ToonProjector<'a, F> {
    projector: Projector<'a, F>,
}

impl<F> EventConsumer for ToonProjector<'_, F>
where
    F: FnMut(Value) -> Result<(), String>,
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
            Event::Scalar { value, .. } => self.projector.scalar(match value {
                Scalar::Null => Value::Null,
                Scalar::Bool(value) => Value::Bool(value),
                Scalar::Number(value) => Value::Number(value),
                Scalar::String(value) => Value::string(value),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufReader, Cursor};

    use super::{StreamOptions, stream_json, stream_toon};

    fn json_lines(values: &[tq_core::Value]) -> Vec<String> {
        values.iter().map(ToString::to_string).collect()
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
}
