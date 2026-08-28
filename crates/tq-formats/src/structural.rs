//! Query-independent JSON structural events without root materialization.

use std::{fmt, io::Read, sync::Arc};

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use tq_core::{Number, SourceId, Span};
use tq_toon::{DecoderCapabilities, Event, EventConsumer, Scalar};

const SERDE_JSON_NUMBER_TOKEN: &str = "$serde_json::private::Number";

/// Resource bounds for query-independent JSON structural decoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JsonEventOptions {
    /// Maximum container nesting below the root value.
    pub maximum_depth: usize,
    /// Maximum decoded bytes in one string, key, or number token.
    pub maximum_token_bytes: usize,
}

impl Default for JsonEventOptions {
    fn default() -> Self {
        Self {
            maximum_depth: 256,
            maximum_token_bytes: 8 * 1024 * 1024,
        }
    }
}

/// JSON structural behavior known before semantic input consumption.
#[must_use]
pub const fn json_decoder_capabilities() -> DecoderCapabilities {
    DecoderCapabilities::json()
}

/// Emits one JSON document as ordered structural events.
///
/// # Errors
///
/// Returns JSON syntax, numeric-envelope, or consumer failures.
pub fn decode_json_events<R, C>(reader: R, source: SourceId, consumer: &mut C) -> Result<(), String>
where
    R: Read,
    C: EventConsumer,
    C::Error: fmt::Display,
{
    decode_json_events_with_options(reader, source, consumer, JsonEventOptions::default())
}

/// Emits one bounded JSON document as ordered structural events.
///
/// # Errors
///
/// Returns JSON syntax, numeric-envelope, resource-limit, or consumer failures.
pub fn decode_json_events_with_options<R, C>(
    reader: R,
    source: SourceId,
    consumer: &mut C,
    options: JsonEventOptions,
) -> Result<(), String>
where
    R: Read,
    C: EventConsumer,
    C::Error: fmt::Display,
{
    let span = Span::new(source, 0, 0);
    emit(consumer, Event::DocumentStart { span })?;
    let mut deserializer = serde_json::Deserializer::from_reader(reader);
    EventSeed {
        consumer,
        source,
        options,
        depth: 0,
    }
    .deserialize(&mut deserializer)
    .map_err(|error| error.to_string())?;
    deserializer.end().map_err(|error| error.to_string())?;
    emit(consumer, Event::DocumentEnd { span })
}

/// Emits a whitespace-separated stream of bounded JSON documents.
///
/// # Errors
///
/// Returns JSON syntax, numeric-envelope, resource-limit, or consumer failures.
pub fn decode_json_event_stream<R, C>(
    reader: R,
    source: SourceId,
    consumer: &mut C,
    options: JsonEventOptions,
) -> Result<u64, String>
where
    R: Read,
    C: EventConsumer,
    C::Error: fmt::Display,
{
    let mut deserializer = serde_json::Deserializer::from_reader(reader);
    let mut documents = 0_u64;
    loop {
        let mut document = LazyDocumentConsumer::new(consumer, source);
        let result = EventSeed {
            consumer: &mut document,
            source,
            options,
            depth: 0,
        }
        .deserialize(&mut deserializer);
        match result {
            Ok(()) => {
                document.finish()?;
                documents = documents.saturating_add(1);
            }
            Err(error) if error.is_eof() && !document.started => return Ok(documents),
            Err(error) => return Err(error.to_string()),
        }
    }
}

struct LazyDocumentConsumer<'a, C> {
    consumer: &'a mut C,
    source: SourceId,
    started: bool,
}

impl<'a, C> LazyDocumentConsumer<'a, C> {
    const fn new(consumer: &'a mut C, source: SourceId) -> Self {
        Self {
            consumer,
            source,
            started: false,
        }
    }
}

impl<C> LazyDocumentConsumer<'_, C>
where
    C: EventConsumer,
    C::Error: fmt::Display,
{
    fn finish(&mut self) -> Result<(), String> {
        if !self.started {
            return Err("JSON document contained no value".to_owned());
        }
        emit(
            self.consumer,
            Event::DocumentEnd {
                span: Span::new(self.source, 0, 0),
            },
        )
    }
}

impl<C> EventConsumer for LazyDocumentConsumer<'_, C>
where
    C: EventConsumer,
    C::Error: fmt::Display,
{
    type Error = String;

    fn consume(&mut self, event: Event) -> Result<(), Self::Error> {
        if !self.started {
            emit(
                self.consumer,
                Event::DocumentStart {
                    span: Span::new(self.source, 0, 0),
                },
            )?;
            self.started = true;
        }
        emit(self.consumer, event)
    }
}

struct EventSeed<'a, C> {
    consumer: &'a mut C,
    source: SourceId,
    options: JsonEventOptions,
    depth: usize,
}

impl<'de, C> DeserializeSeed<'de> for EventSeed<'_, C>
where
    C: EventConsumer,
    C::Error: fmt::Display,
{
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(EventVisitor {
            consumer: self.consumer,
            source: self.source,
            options: self.options,
            depth: self.depth,
        })
    }
}

struct EventVisitor<'a, C> {
    consumer: &'a mut C,
    source: SourceId,
    options: JsonEventOptions,
    depth: usize,
}

impl<C> EventVisitor<'_, C>
where
    C: EventConsumer,
    C::Error: fmt::Display,
{
    fn scalar<E: de::Error>(&mut self, value: Scalar) -> Result<(), E> {
        self.consumer
            .consume(Event::Scalar {
                span: self.span(),
                value,
            })
            .map_err(|error| E::custom(error.to_string()))
    }

    const fn span(&self) -> Span {
        Span::new(self.source, 0, 0)
    }
}

impl<'de, C> Visitor<'de> for EventVisitor<'_, C>
where
    C: EventConsumer,
    C::Error: fmt::Display,
{
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_bool<E: de::Error>(mut self, value: bool) -> Result<(), E> {
        self.scalar(Scalar::Bool(value))
    }

    fn visit_i64<E: de::Error>(mut self, value: i64) -> Result<(), E> {
        self.number(&value.to_string())
    }

    fn visit_i128<E: de::Error>(mut self, value: i128) -> Result<(), E> {
        self.number(&value.to_string())
    }

    fn visit_u64<E: de::Error>(mut self, value: u64) -> Result<(), E> {
        self.number(&value.to_string())
    }

    fn visit_u128<E: de::Error>(mut self, value: u128) -> Result<(), E> {
        self.number(&value.to_string())
    }

    fn visit_f64<E: de::Error>(mut self, value: f64) -> Result<(), E> {
        let number = Number::from_f64(value).map_err(E::custom)?;
        self.scalar(Scalar::Number(number))
    }

    fn visit_str<E: de::Error>(mut self, value: &str) -> Result<(), E> {
        self.check_token::<E>(value.len())?;
        self.scalar(Scalar::String(Arc::from(value)))
    }

    fn visit_string<E: de::Error>(mut self, value: String) -> Result<(), E> {
        self.check_token::<E>(value.len())?;
        self.scalar(Scalar::String(Arc::from(value)))
    }

    fn visit_none<E: de::Error>(mut self) -> Result<(), E> {
        self.scalar(Scalar::Null)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<(), D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        EventSeed {
            consumer: self.consumer,
            source: self.source,
            options: self.options,
            depth: self.depth,
        }
        .deserialize(deserializer)
    }

    fn visit_unit<E: de::Error>(mut self) -> Result<(), E> {
        self.scalar(Scalar::Null)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<(), A::Error>
    where
        A: SeqAccess<'de>,
    {
        let child_depth = self.child_depth::<A::Error>()?;
        self.consumer
            .consume(Event::ArrayStart {
                span: self.span(),
                declared_count: None,
            })
            .map_err(|error| de::Error::custom(error.to_string()))?;
        let mut observed = 0_u64;
        while sequence
            .next_element_seed(EventSeed {
                consumer: self.consumer,
                source: self.source,
                options: self.options,
                depth: child_depth,
            })?
            .is_some()
        {
            observed = observed.saturating_add(1);
        }
        self.consumer
            .consume(Event::ArrayEnd {
                span: self.span(),
                observed_count: observed,
            })
            .map_err(|error| de::Error::custom(error.to_string()))
    }

    fn visit_map<A>(mut self, mut map: A) -> Result<(), A::Error>
    where
        A: MapAccess<'de>,
    {
        let child_depth = self.child_depth::<A::Error>()?;
        let Some(first_key) = map.next_key::<String>()? else {
            self.consumer
                .consume(Event::ObjectStart { span: self.span() })
                .map_err(|error| de::Error::custom(error.to_string()))?;
            return self
                .consumer
                .consume(Event::ObjectEnd { span: self.span() })
                .map_err(|error| de::Error::custom(error.to_string()));
        };
        if first_key == SERDE_JSON_NUMBER_TOKEN {
            let literal = map.next_value::<String>()?;
            self.check_token::<A::Error>(literal.len())?;
            if map.next_key::<de::IgnoredAny>()?.is_some() {
                return Err(de::Error::custom(
                    "invalid arbitrary-precision number envelope",
                ));
            }
            return self.number(&literal);
        }

        self.consumer
            .consume(Event::ObjectStart { span: self.span() })
            .map_err(|error| de::Error::custom(error.to_string()))?;
        self.key::<A::Error>(first_key)?;
        map.next_value_seed(EventSeed {
            consumer: self.consumer,
            source: self.source,
            options: self.options,
            depth: child_depth,
        })?;
        while let Some(key) = map.next_key::<String>()? {
            self.key::<A::Error>(key)?;
            map.next_value_seed(EventSeed {
                consumer: self.consumer,
                source: self.source,
                options: self.options,
                depth: child_depth,
            })?;
        }
        self.consumer
            .consume(Event::ObjectEnd { span: self.span() })
            .map_err(|error| de::Error::custom(error.to_string()))
    }
}

impl<C> EventVisitor<'_, C>
where
    C: EventConsumer,
    C::Error: fmt::Display,
{
    fn number<E: de::Error>(&mut self, literal: &str) -> Result<(), E> {
        self.check_token::<E>(literal.len())?;
        let number = Number::parse(literal).map_err(E::custom)?;
        self.scalar(Scalar::Number(number))
    }

    fn key<E: de::Error>(&mut self, value: String) -> Result<(), E> {
        self.check_token::<E>(value.len())?;
        self.consumer
            .consume(Event::Key {
                span: self.span(),
                value: Arc::from(value),
                quoted: true,
            })
            .map_err(|error| E::custom(error.to_string()))
    }

    fn check_token<E: de::Error>(&self, bytes: usize) -> Result<(), E> {
        if bytes > self.options.maximum_token_bytes {
            return Err(E::custom("JSON token byte limit exceeded"));
        }
        Ok(())
    }

    fn child_depth<E: de::Error>(&self) -> Result<usize, E> {
        if self.depth >= self.options.maximum_depth {
            return Err(E::custom("JSON nesting depth limit exceeded"));
        }
        Ok(self.depth + 1)
    }
}

fn emit<C>(consumer: &mut C, event: Event) -> Result<(), String>
where
    C: EventConsumer,
    C::Error: fmt::Display,
{
    consumer.consume(event).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use tq_core::SourceId;
    use tq_toon::{Event, EventConsumer, Scalar};

    use super::{JsonEventOptions, decode_json_event_stream, decode_json_events};

    #[derive(Default)]
    struct Collector(Vec<Event>);

    impl EventConsumer for Collector {
        type Error = Infallible;

        fn consume(&mut self, event: Event) -> Result<(), Self::Error> {
            self.0.push(event);
            Ok(())
        }
    }

    #[test]
    fn emits_ordered_duplicate_keys_and_exact_numbers_without_a_root_value() {
        let mut collector = Collector::default();
        decode_json_events(
            br#"{"b":1,"a":[9007199254740993],"b":2}"#.as_slice(),
            SourceId::new(1),
            &mut collector,
        )
        .unwrap();

        let keys = collector
            .0
            .iter()
            .filter_map(|event| match event {
                Event::Key { value, .. } => Some(value.as_ref()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(keys, ["b", "a", "b"]);
        assert!(collector.0.iter().any(|event| matches!(
            event,
            Event::Scalar {
                value: Scalar::Number(number),
                ..
            } if number.to_string() == "9007199254740993"
        )));
    }

    #[test]
    fn malformed_late_input_does_not_emit_document_end() {
        let mut collector = Collector::default();
        assert!(
            decode_json_events(
                br#"{"a":1,"b":"#.as_slice(),
                SourceId::new(1),
                &mut collector
            )
            .is_err()
        );
        assert!(
            !collector
                .0
                .iter()
                .any(|event| matches!(event, Event::DocumentEnd { .. }))
        );
    }

    #[test]
    fn streams_multiple_documents_without_a_phantom_eof_document() {
        let mut collector = Collector::default();
        assert_eq!(
            decode_json_event_stream(
                b"1 {\"x\":2}  \n".as_slice(),
                SourceId::new(1),
                &mut collector,
                JsonEventOptions::default(),
            )
            .unwrap(),
            2
        );
        assert_eq!(
            collector
                .0
                .iter()
                .filter(|event| matches!(event, Event::DocumentStart { .. }))
                .count(),
            2
        );
    }

    #[test]
    fn enforces_depth_and_token_limits() {
        let options = JsonEventOptions {
            maximum_depth: 1,
            maximum_token_bytes: 3,
        };
        let mut collector = Collector::default();
        assert!(
            super::decode_json_events_with_options(
                b"[[1]]".as_slice(),
                SourceId::new(1),
                &mut collector,
                options,
            )
            .unwrap_err()
            .contains("nesting depth")
        );
        let mut collector = Collector::default();
        assert!(
            super::decode_json_events_with_options(
                br#""oversized""#.as_slice(),
                SourceId::new(1),
                &mut collector,
                options,
            )
            .unwrap_err()
            .contains("token byte")
        );
    }
}
