//! Direct structural consumers retain the codecs' allocation-avoiding hooks.

use crate::{DecodeOptions, FormatError, InputDeliveryError, InputFormat};
use std::{
    fmt,
    io::{BufReader, Read},
};
use tq_core::{SourceId, Span};
use tq_toon::{Event, EventConsumer};

/// The structural consumer trait has typed events and text-based fast hooks.
#[derive(Debug)]
pub enum CodecConsumerError<E> {
    /// Original error from the owned-event callback.
    Event(E),
    /// Original error text from an allocation-avoiding scalar/key callback.
    Text(String),
}

pub(crate) fn consume<R: Read, C: EventConsumer>(
    reader: R,
    options: DecodeOptions,
    source: SourceId,
    consumer: &mut C,
) -> Result<(), InputDeliveryError<CodecConsumerError<C::Error>>>
where
    C::Error: fmt::Display,
{
    let mut bridge = CodecConsumer {
        consumer,
        failure: None,
    };
    let result = match options.format {
        InputFormat::Json => crate::structural::decode_json_event_stream_classified(
            BufReader::with_capacity(64 * 1024, reader),
            source,
            &mut bridge,
            crate::JsonEventOptions {
                maximum_depth: options.maximum_depth,
                maximum_token_bytes: options.maximum_token_bytes,
            },
        )
        .map(|_| ()),
        InputFormat::Toon => tq_toon::Decoder::new(BufReader::new(reader), source, options.toon)
            .decode_into(&mut bridge)
            .map_err(|error| match error {
                tq_toon::DecodeIntoError::Decode(error) => crate::adapters::toon_input_error(error),
                tq_toon::DecodeIntoError::Consumer(message) => FormatError::Parse {
                    format: options.format,
                    message,
                },
            }),
        _ => Err(FormatError::Parse {
            format: options.format,
            message: "direct codec consumers require JSON or TOON event input".to_owned(),
        }),
    };
    if let Some(error) = bridge.failure {
        return Err(InputDeliveryError::Consumer(error));
    }
    result.map_err(InputDeliveryError::Input)
}

struct CodecConsumer<'a, C: EventConsumer> {
    consumer: &'a mut C,
    failure: Option<CodecConsumerError<C::Error>>,
}

impl<C: EventConsumer> CodecConsumer<'_, C> {
    fn text_result(&mut self, result: Result<(), String>) -> Result<(), String> {
        result.map_err(|message| {
            self.failure = Some(CodecConsumerError::Text(message));
            "native codec consumer failed".to_owned()
        })
    }
}

impl<C: EventConsumer> EventConsumer for CodecConsumer<'_, C>
where
    C::Error: fmt::Display,
{
    type Error = String;

    fn consume(&mut self, event: Event) -> Result<(), String> {
        self.consumer.consume(event).map_err(|error| {
            self.failure = Some(CodecConsumerError::Event(error));
            "native codec consumer failed".to_owned()
        })
    }

    fn consume_text_key(&mut self, span: Span, value: String, quoted: bool) -> Result<(), String> {
        let result = self.consumer.consume_text_key(span, value, quoted);
        self.text_result(result)
    }

    fn consume_null(&mut self, span: Span) -> Result<(), String> {
        let result = self.consumer.consume_null(span);
        self.text_result(result)
    }

    fn consume_bool(&mut self, span: Span, value: bool) -> Result<(), String> {
        let result = self.consumer.consume_bool(span, value);
        self.text_result(result)
    }

    fn consume_text_string(&mut self, span: Span, value: String) -> Result<(), String> {
        let result = self.consumer.consume_text_string(span, value);
        self.text_result(result)
    }

    fn consume_number_literal(&mut self, span: Span, literal: String) -> Result<(), String> {
        let result = self.consumer.consume_number_literal(span, literal);
        self.text_result(result)
    }
}
