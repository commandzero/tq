//! Optional event-to-DOM adapter for non-streaming query paths.

use std::{io::BufRead, sync::Arc};

use thiserror::Error;
use tq_core::{Object, SourceId, Value};

use crate::{DecodeError, Decoder, DecoderConfig, Event, EventConsumer, Scalar};

/// Invalid event sequence received by [`DomBuilder`].
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum DomError {
    /// A document boundary appeared in an invalid state.
    #[error("invalid document boundary in TOON event stream")]
    DocumentBoundary,
    /// A key appeared outside an object or before the previous value.
    #[error("unexpected object key in TOON event stream")]
    UnexpectedKey,
    /// A value appeared where its parent could not accept it.
    #[error("unexpected value in TOON event stream")]
    UnexpectedValue,
    /// A container ended with the wrong active container kind.
    #[error("mismatched container end in TOON event stream")]
    MismatchedContainer,
    /// An object ended while a key was still waiting for a value.
    #[error("object key has no value in TOON event stream")]
    MissingObjectValue,
    /// No complete document value was produced.
    #[error("TOON event stream did not produce one complete value")]
    IncompleteDocument,
}

/// Error returned by [`decode_to_value`].
#[derive(Debug, Error)]
pub enum DomDecodeError {
    /// Input decoding failed.
    #[error(transparent)]
    Decode(#[from] DecodeError),
    /// The decoder emitted an invalid structural event sequence.
    #[error(transparent)]
    Dom(#[from] DomError),
}

/// Incremental consumer that materializes one immutable [`Value`].
#[derive(Debug, Default)]
pub struct DomBuilder {
    frames: Vec<Frame>,
    root: Option<Value>,
    started: bool,
    ended: bool,
}

#[derive(Debug)]
enum Frame {
    Object {
        values: Object,
        pending_key: Option<Arc<str>>,
    },
    Array(Vec<Value>),
}

impl DomBuilder {
    /// Creates an empty document builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the single completed document.
    ///
    /// # Errors
    ///
    /// Returns an error when event boundaries or containers are incomplete.
    pub fn finish(self) -> Result<Value, DomError> {
        if self.started && self.ended && self.frames.is_empty() {
            self.root.ok_or(DomError::IncompleteDocument)
        } else {
            Err(DomError::IncompleteDocument)
        }
    }

    fn attach(&mut self, value: Value) -> Result<(), DomError> {
        match self.frames.last_mut() {
            Some(Frame::Array(values)) => {
                values.push(value);
                Ok(())
            }
            Some(Frame::Object {
                values,
                pending_key,
            }) => {
                let key = pending_key.take().ok_or(DomError::UnexpectedValue)?;
                values.insert(key, value);
                Ok(())
            }
            None if self.root.is_none() => {
                self.root = Some(value);
                Ok(())
            }
            None => Err(DomError::UnexpectedValue),
        }
    }

    fn close_object(&mut self) -> Result<(), DomError> {
        let Some(Frame::Object {
            values,
            pending_key,
        }) = self.frames.pop()
        else {
            return Err(DomError::MismatchedContainer);
        };
        if pending_key.is_some() {
            return Err(DomError::MissingObjectValue);
        }
        self.attach(Value::object(values))
    }

    fn close_array(&mut self) -> Result<(), DomError> {
        let Some(Frame::Array(values)) = self.frames.pop() else {
            return Err(DomError::MismatchedContainer);
        };
        self.attach(Value::array(values))
    }
}

impl EventConsumer for DomBuilder {
    type Error = DomError;

    fn consume(&mut self, event: Event) -> Result<(), Self::Error> {
        match event {
            Event::DocumentStart { .. }
                if !self.started
                    && !self.ended
                    && self.root.is_none()
                    && self.frames.is_empty() =>
            {
                self.started = true;
                Ok(())
            }
            Event::DocumentEnd { .. }
                if self.started && !self.ended && self.root.is_some() && self.frames.is_empty() =>
            {
                self.ended = true;
                Ok(())
            }
            Event::DocumentStart { .. } | Event::DocumentEnd { .. } => {
                Err(DomError::DocumentBoundary)
            }
            Event::ObjectStart { .. } if self.started && !self.ended => {
                self.frames.push(Frame::Object {
                    values: Object::new(),
                    pending_key: None,
                });
                Ok(())
            }
            Event::ObjectEnd { .. } if self.started && !self.ended => self.close_object(),
            Event::ArrayStart { .. } if self.started && !self.ended => {
                self.frames.push(Frame::Array(Vec::new()));
                Ok(())
            }
            Event::ArrayEnd { .. } if self.started && !self.ended => self.close_array(),
            Event::Key { value, .. } if self.started && !self.ended => {
                let Some(Frame::Object { pending_key, .. }) = self.frames.last_mut() else {
                    return Err(DomError::UnexpectedKey);
                };
                if pending_key.replace(value).is_some() {
                    return Err(DomError::UnexpectedKey);
                }
                Ok(())
            }
            Event::Key { .. } => Err(DomError::UnexpectedKey),
            Event::Scalar { value, .. } if self.started && !self.ended => {
                self.attach(match value {
                    Scalar::Null => Value::Null,
                    Scalar::Bool(value) => Value::Bool(value),
                    Scalar::Number(value) => Value::Number(value),
                    Scalar::String(value) => Value::String(value),
                })
            }
            Event::ObjectStart { .. } | Event::ArrayStart { .. } | Event::Scalar { .. } => {
                Err(DomError::UnexpectedValue)
            }
            Event::ObjectEnd { .. } | Event::ArrayEnd { .. } => Err(DomError::MismatchedContainer),
        }
    }
}

/// Decodes one TOON document into the immutable runtime value model.
///
/// This is deliberately an adapter over the public event contract. Streaming
/// queries can consume [`Decoder`] directly without paying for a full DOM.
///
/// # Errors
///
/// Returns a decoder or structural event-consumer failure.
pub fn decode_to_value<R: BufRead>(
    reader: R,
    source: SourceId,
    config: DecoderConfig,
) -> Result<Value, DomDecodeError> {
    let mut decoder = Decoder::new(reader, source, config);
    let mut builder = DomBuilder::new();
    while let Some(event) = decoder.next_event()? {
        builder.consume(event)?;
    }
    builder.finish().map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use std::io::{BufReader, Cursor};

    use tq_core::SourceId;

    use super::decode_to_value;
    use crate::DecoderConfig;

    #[test]
    fn materializes_nested_values_without_changing_event_decoder() {
        let input = b"users[2]{id,name}:\n  1,Ada\n  2,Bob\nactive: true\n";
        let value = decode_to_value(
            BufReader::with_capacity(4, Cursor::new(input)),
            SourceId::new(7),
            DecoderConfig::default(),
        )
        .unwrap();
        assert_eq!(
            value.to_string(),
            r#"{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Bob"}],"active":true}"#
        );
    }
}
