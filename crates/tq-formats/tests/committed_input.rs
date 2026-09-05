//! Committed input is lazy and preserves document order.

use tq_formats::{DecodeOptions, InputRepresentation, NativeFormat, NativeInputObservation};

#[test]
fn committed_input_codec_consumer_preserves_source_and_typed_failure() {
    use tq_core::SourceId;
    use tq_formats::{CodecConsumerError, InputDeliveryError};
    use tq_toon::{Event, EventConsumer};
    struct Reject;
    impl EventConsumer for Reject {
        type Error = std::io::Error;
        fn consume(&mut self, event: Event) -> Result<(), Self::Error> {
            let Event::DocumentStart { span } = event else {
                panic!("first event")
            };
            assert_eq!(span.source, SourceId::new(42));
            Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe))
        }
    }
    for format in [NativeFormat::Json, NativeFormat::Toon] {
        let input = format
            .select_input(DecodeOptions::default(), InputRepresentation::Events)
            .unwrap()
            .open(b"null".as_slice(), "test");
        let Err(InputDeliveryError::Consumer(CodecConsumerError::Event(error))) =
            input.consume_codec_events(SourceId::new(42), &mut Reject)
        else {
            panic!("typed consumer error")
        };
        assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
    }
}

#[test]
fn committed_input_selected_toon_decoding_keeps_resource_errors_typed() {
    use std::ops::ControlFlow;
    use tq_core::PathComponent;
    let input = NativeFormat::Toon
        .select_input(
            DecodeOptions {
                maximum_token_bytes: 6,
                ..DecodeOptions::default()
            },
            InputRepresentation::Events,
        )
        .unwrap()
        .open(b"items[0]:\nunused: long-value\n".as_slice(), "discarded");
    let result = input.consume_selected(
        tq_formats::StreamSelection::new(vec![PathComponent::Key("items".into())], None),
        None,
        None,
        |_| Ok::<_, ()>(ControlFlow::Continue(())),
    );
    assert!(
        matches!(
            result,
            Err(tq_formats::InputDeliveryError::Input(
                tq_formats::FormatError::Resource("token-bytes")
            ))
        ),
        "{result:?}"
    );
}

#[test]
fn committed_input_selected_decoding_keeps_resource_errors_typed() {
    use std::ops::ControlFlow;
    use tq_core::PathComponent;
    let input = NativeFormat::Json
        .select_input(
            DecodeOptions {
                maximum_token_bytes: 6,
                ..DecodeOptions::default()
            },
            InputRepresentation::Events,
        )
        .unwrap()
        .open(
            b"{\"items\":[],\"unused\":\"long-value\"}".as_slice(),
            "discarded",
        );
    let result = input.consume_selected(
        tq_formats::StreamSelection::new(vec![PathComponent::Key("items".into())], None),
        None,
        None,
        |_| Ok::<_, ()>(ControlFlow::Continue(())),
    );
    assert!(
        matches!(
            result,
            Err(tq_formats::InputDeliveryError::Input(
                tq_formats::FormatError::Resource("token-bytes")
            ))
        ),
        "{result:?}"
    );
}

#[test]
fn committed_input_selected_events_keep_json_lines_document_boundaries() {
    use std::ops::ControlFlow;
    use tq_core::PathComponent;
    use tq_formats::{SelectedInputObservation, StreamSelection};
    let input = NativeFormat::JsonLines
        .select_input(DecodeOptions::default(), InputRepresentation::Events)
        .unwrap()
        .open(
            b"{\"items\":[{\"id\":1}]}\n{\"items\":[{\"id\":2}]}\n".as_slice(),
            "selected",
        );
    let mut values = Vec::new();
    let mut ends = 0;
    input
        .consume_selected(
            StreamSelection::new(
                vec![PathComponent::Key("items".into())],
                Some(vec![PathComponent::Key("id".into())]),
            ),
            None,
            None,
            |observation| {
                match observation {
                    SelectedInputObservation::Record(record) => {
                        if let Some(value) = record.into_parts().1 {
                            values.push(value.to_string());
                        }
                    }
                    SelectedInputObservation::DocumentEnd => ends += 1,
                }
                Ok::<_, ()>(ControlFlow::Continue(()))
            },
        )
        .unwrap();
    assert_eq!(values, ["1", "2"]);
    assert_eq!(ends, 2);
}

#[test]
fn json_sequence_input_recovery_and_indices_are_chunk_invariant() {
    use std::io::{self, Read};
    struct Chunks<'a> {
        bytes: &'a [u8],
        size: usize,
    }
    impl Read for Chunks<'_> {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            let length = output.len().min(self.size);
            self.bytes.read(&mut output[..length])
        }
    }
    for size in 1..=16 {
        let mut input = NativeFormat::JsonSequence
            .select_input(DecodeOptions::default(), InputRepresentation::Documents)
            .unwrap()
            .open(
                Chunks {
                    bytes: b"preamble\x1e\x1e{\"a\":\"\xc3\xa9\"}\n\x1e\xff\n\x1etrue\n",
                    size,
                },
                "chunks",
            );
        let Some(NativeInputObservation::Document(first)) = input.next_observation().unwrap()
        else {
            panic!("first Document");
        };
        assert_eq!(first.index, 0);
        assert_eq!(first.value.to_string(), "{\"a\":\"é\"}");
        let Some(NativeInputObservation::Failure(failure)) = input.next_observation().unwrap()
        else {
            panic!("UTF-8 failure");
        };
        assert_eq!(failure.recovery_segment_index, 2);
        assert_eq!(failure.document_index, 1);
        let Some(NativeInputObservation::Document(last)) = input.next_observation().unwrap() else {
            panic!("recovered Document");
        };
        assert_eq!(last.index, 1);
        assert_eq!(last.value.to_string(), "true");
        assert!(input.next_observation().unwrap().is_none());
    }
}

#[test]
fn committed_input_buffered_limits_preserve_prior_documents_and_event_stop() {
    use std::ops::ControlFlow;
    let options = DecodeOptions {
        maximum_depth: 1,
        maximum_token_bytes: 3,
        ..DecodeOptions::default()
    };
    for bytes in [b"1 \"long\"".as_slice(), b"1 [[0]]"] {
        let mut input = NativeFormat::Json
            .select_input(options, InputRepresentation::Documents)
            .unwrap()
            .open(bytes, "later");
        let Some(NativeInputObservation::Document(document)) = input.next_observation().unwrap()
        else {
            panic!("prior Document");
        };
        assert_eq!(document.value.to_string(), "1");
        assert!(matches!(
            input.next_observation(),
            Err(tq_formats::FormatError::Resource(_))
        ));
        NativeFormat::Json
            .select_input(options, InputRepresentation::Events)
            .unwrap()
            .open(bytes, "later")
            .consume_events(|observation| {
                Ok::<_, ()>(
                    if matches!(
                        observation,
                        NativeInputObservation::Event(tq_toon::Event::DocumentEnd { .. })
                    ) {
                        ControlFlow::Break(())
                    } else {
                        ControlFlow::Continue(())
                    },
                )
            })
            .unwrap();
    }
}

#[test]
fn committed_input_json_limits_include_overwritten_values_and_decoded_escapes() {
    for (bytes, limit, accepted) in [
        (b"{\"a\":\"long\",\"a\":1}".as_slice(), 3, false),
        (b"\"\\u0061\\u0062\"", 2, true),
        (b"\"\\u0061\\u0062\"", 1, false),
        (b"\"\\ud83d\\ude00\"", 4, true),
        (b"\"\\ud83d\\ude00\"", 3, false),
        (b"\"\\n\\t\"", 2, true),
        (b"\"\\n\\t\"", 1, false),
    ] {
        let mut input = NativeFormat::Json
            .select_input(
                DecodeOptions {
                    maximum_token_bytes: limit,
                    ..DecodeOptions::default()
                },
                InputRepresentation::Documents,
            )
            .unwrap()
            .open(bytes, "tokens");
        let result = input.next_observation();
        if accepted {
            assert!(result.is_ok(), "{result:?}");
        } else {
            assert!(
                matches!(
                    result,
                    Err(tq_formats::FormatError::Resource("token-bytes"))
                ),
                "{result:?}"
            );
        }
    }
}

#[test]
fn committed_input_bounds_json_token_storage_before_reading_the_whole_token() {
    use std::{
        cell::Cell,
        io::{self, Read},
        ops::ControlFlow,
        rc::Rc,
    };
    struct LongString {
        read: Rc<Cell<usize>>,
    }
    impl Read for LongString {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            let count = bytes
                .len()
                .min(1_000_000_usize.saturating_sub(self.read.get()));
            bytes[..count].fill(b'a');
            if self.read.get() == 0 && count > 0 {
                bytes[0] = b'"';
            }
            self.read.set(self.read.get() + count);
            Ok(count)
        }
    }
    for representation in [InputRepresentation::Documents, InputRepresentation::Events] {
        let read = Rc::new(Cell::new(0));
        let mut input = NativeFormat::Json
            .select_input(
                DecodeOptions {
                    maximum_token_bytes: 3,
                    ..DecodeOptions::default()
                },
                representation,
            )
            .unwrap()
            .open(
                LongString {
                    read: Rc::clone(&read),
                },
                "long",
            );
        let error = if representation == InputRepresentation::Documents {
            input.next_observation().unwrap_err()
        } else {
            match input
                .consume_events(|_| Ok::<_, ()>(ControlFlow::Continue(())))
                .unwrap_err()
            {
                tq_formats::InputDeliveryError::Input(error) => error,
                tq_formats::InputDeliveryError::Consumer(()) => panic!("infallible consumer"),
            }
        };
        assert!(
            matches!(error, tq_formats::FormatError::Resource("token-bytes")),
            "{error:?}"
        );
        assert!(
            read.get() <= 64 * 1024,
            "read past bounded input buffer: {}",
            read.get()
        );
    }
}

#[test]
fn committed_input_json_and_json_lines_honor_configured_depth_in_both_representations() {
    use std::ops::ControlFlow;
    for format in [NativeFormat::Json, NativeFormat::JsonLines] {
        for representation in [InputRepresentation::Documents, InputRepresentation::Events] {
            for maximum_depth in [100, 256] {
                let bytes = format!("{}0{}\n", "[".repeat(130), "]".repeat(130));
                let mut input = format
                    .select_input(
                        DecodeOptions {
                            maximum_depth,
                            ..DecodeOptions::default()
                        },
                        representation,
                    )
                    .unwrap()
                    .open(bytes.as_bytes(), "deep");
                let result = if representation == InputRepresentation::Documents {
                    input.next_observation().map(|_| ())
                } else {
                    input
                        .consume_events(|_| Ok::<_, ()>(ControlFlow::Continue(())))
                        .map_err(|error| match error {
                            tq_formats::InputDeliveryError::Input(error) => error,
                            tq_formats::InputDeliveryError::Consumer(()) => {
                                panic!("consumer cannot fail")
                            }
                        })
                };
                if maximum_depth == 256 {
                    assert!(result.is_ok(), "{format:?} {representation:?}: {result:?}");
                } else {
                    assert!(
                        matches!(
                            result,
                            Err(tq_formats::FormatError::Resource("depth")
                                | tq_formats::FormatError::ResourceLine {
                                    resource: "depth",
                                    ..
                                })
                        ),
                        "{format:?} {representation:?}: {result:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn json_sequence_input_events_preserve_partial_values_and_reset_after_failure() {
    use std::ops::ControlFlow;
    let input = NativeFormat::JsonSequence
        .select_input(DecodeOptions::default(), InputRepresentation::Events)
        .unwrap()
        .open(b"\x1e[1,broken]\x1e{\"b\":2}\n".as_slice(), "events");
    let mut observed = Vec::new();
    input
        .consume_events(|observation| {
            observed.push(match observation {
                NativeInputObservation::Event(tq_toon::Event::DocumentStart { .. }) => {
                    "start".to_owned()
                }
                NativeInputObservation::Event(tq_toon::Event::DocumentEnd { .. }) => {
                    "end".to_owned()
                }
                NativeInputObservation::Event(tq_toon::Event::Scalar {
                    value: tq_toon::Scalar::Number(value),
                    ..
                }) => value.to_string(),
                NativeInputObservation::Failure(_) => "failure".to_owned(),
                _ => return Ok::<_, ()>(ControlFlow::Continue(())),
            });
            Ok(ControlFlow::Continue(()))
        })
        .unwrap();
    assert_eq!(observed, ["start", "1", "failure", "start", "2", "end"]);
}

#[test]
fn json_sequence_input_keeps_documents_before_failure_and_resumes_in_the_segment() {
    let mut input = NativeFormat::JsonSequence
        .select_input(DecodeOptions::default(), InputRepresentation::Documents)
        .unwrap()
        .open(
            b"preamble\x1e{\"a\":1} {\"b\":2} broken {\"skip\":3}\x1e{\"c\":4}\n".as_slice(),
            "sequence.jsonseq",
        );
    for (expected_index, expected) in [(0, "{\"a\":1}"), (1, "{\"b\":2}")] {
        let NativeInputObservation::Document(document) = input.next_observation().unwrap().unwrap()
        else {
            panic!("document")
        };
        assert_eq!(document.index, expected_index);
        assert_eq!(document.value.to_string(), expected);
    }
    let NativeInputObservation::Failure(failure) = input.next_observation().unwrap().unwrap()
    else {
        panic!("ordered failure")
    };
    assert_eq!(failure.identity, "sequence.jsonseq");
    assert_eq!(failure.recovery_segment_index, 0);
    assert_eq!(failure.document_index, 2);
    let NativeInputObservation::Document(document) = input.next_observation().unwrap().unwrap()
    else {
        panic!("recovered document")
    };
    assert_eq!(document.value.to_string(), "{\"skip\":3}");
    assert_eq!(document.index, 2);
    let NativeInputObservation::Document(document) = input.next_observation().unwrap().unwrap()
    else {
        panic!("document in the next segment")
    };
    assert_eq!(document.value.to_string(), "{\"c\":4}");
    assert_eq!(document.index, 3);
    assert!(input.next_observation().unwrap().is_none());
}

#[test]
fn json_sequence_input_exempts_fixed_keywords_from_token_limits() {
    let mut input = NativeFormat::JsonSequence
        .select_input(
            DecodeOptions {
                maximum_token_bytes: 1,
                ..DecodeOptions::default()
            },
            InputRepresentation::Documents,
        )
        .unwrap()
        .open(b"\x1etrue false null 12\n".as_slice(), "keywords");
    for expected in ["true", "false", "null"] {
        let NativeInputObservation::Document(document) = input.next_observation().unwrap().unwrap()
        else {
            panic!("keyword")
        };
        assert_eq!(document.value.to_string(), expected);
    }
    assert!(matches!(
        input.next_observation(),
        Err(tq_formats::FormatError::Resource("token-bytes"))
    ));
}

#[test]
fn json_sequence_input_limits_decoded_strings_and_keys() {
    for bytes in [
        b"\x1e\"abc\"\n".as_slice(),
        b"\x1e\"\\u0061bc\"\n",
        b"\x1e{\"abc\":1}\n",
    ] {
        for maximum_token_bytes in [2, 3] {
            let mut input = NativeFormat::JsonSequence
                .select_input(
                    DecodeOptions {
                        maximum_token_bytes,
                        ..DecodeOptions::default()
                    },
                    InputRepresentation::Documents,
                )
                .unwrap()
                .open(bytes, "strings");
            let result = input.next_observation();
            if maximum_token_bytes == 3 {
                assert!(
                    matches!(result, Ok(Some(NativeInputObservation::Document(_)))),
                    "{result:?}"
                );
            } else {
                assert!(
                    matches!(
                        result,
                        Err(tq_formats::FormatError::Resource("token-bytes"))
                    ),
                    "{result:?}"
                );
            }
        }
    }
}

#[test]
fn json_sequence_input_rejects_potentially_truncated_root_numbers() {
    for bytes in [b"\x1e123\x1e{}\n".as_slice(), b"\x1e123".as_slice()] {
        let mut input = NativeFormat::JsonSequence
            .select_input(DecodeOptions::default(), InputRepresentation::Documents)
            .unwrap()
            .open(bytes, "numbers");
        let NativeInputObservation::Failure(failure) = input.next_observation().unwrap().unwrap()
        else {
            panic!("unterminated number must not be published")
        };
        assert!(
            failure
                .message
                .contains("Potentially truncated top-level numeric value")
        );
    }
}

#[test]
fn committed_input_uses_the_selected_depth_limit_not_serdes_default() {
    let bytes = format!("\x1e{}0{}\n", "[".repeat(130), "]".repeat(130));
    for (maximum_depth, accepted) in [(256, true), (100, false)] {
        let mut input = NativeFormat::JsonSequence
            .select_input(
                DecodeOptions {
                    maximum_depth,
                    ..DecodeOptions::default()
                },
                InputRepresentation::Documents,
            )
            .unwrap()
            .open(bytes.as_bytes(), "deep");
        let result = input.next_observation();
        if accepted {
            assert!(
                matches!(result, Ok(Some(NativeInputObservation::Document(_)))),
                "{result:?}"
            );
        } else {
            assert!(
                matches!(result, Err(tq_formats::FormatError::Resource("depth"))),
                "{result:?}"
            );
        }
    }
}

#[test]
fn committed_input_toon_sequence_publishes_before_a_later_framing_error() {
    let mut input = NativeFormat::ToonSequence
        .select_input(DecodeOptions::default(), InputRepresentation::Documents)
        .unwrap()
        .open(b"\x1ea: 1\n\x1eb: 2".as_slice(), "sequence");
    let NativeInputObservation::Document(document) = input.next_observation().unwrap().unwrap()
    else {
        panic!("document")
    };
    assert_eq!(document.value.to_string(), "{\"a\":1}");
    assert_eq!(document.index, 0);
    assert!(
        input
            .next_observation()
            .unwrap_err()
            .to_string()
            .contains("missing LF")
    );
    assert!(input.next_observation().unwrap().is_none());
}

#[test]
fn committed_input_pulls_a_complete_document_before_a_later_failure() {
    let selection = NativeFormat::Json
        .select_input(DecodeOptions::default(), InputRepresentation::Documents)
        .unwrap();
    let mut input = selection.open(b"{\"a\":1} broken".as_slice(), "source.json");
    let NativeInputObservation::Document(document) = input.next_observation().unwrap().unwrap()
    else {
        panic!("document observation")
    };
    assert_eq!(document.value.to_string(), "{\"a\":1}");
    assert_eq!(document.index, 0);
    assert_eq!(document.identity, "source.json");
    assert!(input.next_observation().is_err());
    assert!(input.next_observation().unwrap().is_none());
}

#[test]
fn committed_input_enforces_source_limits_for_json_documents() {
    let selection = NativeFormat::Json
        .select_input(
            DecodeOptions {
                maximum_source_bytes: 3,
                ..DecodeOptions::default()
            },
            InputRepresentation::Documents,
        )
        .unwrap();
    let mut input = selection.open(b"true".as_slice(), "limited.json");
    assert!(matches!(
        input.next_observation(),
        Err(tq_formats::FormatError::Resource("source-bytes"))
    ));
}

#[test]
fn committed_input_events_can_stop_before_later_invalid_syntax() {
    use std::ops::ControlFlow;
    let selection = NativeFormat::Json
        .select_input(DecodeOptions::default(), InputRepresentation::Events)
        .unwrap();
    let input = selection.open(b"[1, broken".as_slice(), "events.json");
    let mut observations = 0;
    input
        .consume_events(|_| {
            observations += 1;
            Ok::<_, ()>(ControlFlow::Break(()))
        })
        .unwrap();
    assert_eq!(observations, 1);
    assert!(
        NativeFormat::Json5
            .select_input(DecodeOptions::default(), InputRepresentation::Events)
            .is_err()
    );
}

#[test]
fn committed_input_events_preserve_resource_and_io_failure_classes() {
    use std::{io, ops::ControlFlow};
    use tq_formats::{FormatError, InputDeliveryError};
    struct Broken;
    impl io::Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::ErrorKind::PermissionDenied.into())
        }
    }
    for (format, bytes, options) in [
        (
            NativeFormat::Json,
            b"[[0]]".as_slice(),
            DecodeOptions {
                maximum_depth: 1,
                ..DecodeOptions::default()
            },
        ),
        (
            NativeFormat::JsonLines,
            b"null\n".as_slice(),
            DecodeOptions {
                maximum_line_bytes: 2,
                ..DecodeOptions::default()
            },
        ),
    ] {
        let input = format
            .select_input(options, InputRepresentation::Events)
            .unwrap()
            .open(bytes, "limited");
        let error = input
            .consume_events(|_| Ok::<_, ()>(ControlFlow::Continue(())))
            .unwrap_err();
        assert!(
            matches!(
                error,
                InputDeliveryError::Input(
                    FormatError::Resource(_) | FormatError::ResourceLine { .. }
                )
            ),
            "{error:?}"
        );
    }
    let input = NativeFormat::Json
        .select_input(DecodeOptions::default(), InputRepresentation::Events)
        .unwrap()
        .open(Broken, "broken");
    assert!(matches!(
        input.consume_events(|_| Ok::<_, ()>(ControlFlow::Continue(()))),
        Err(InputDeliveryError::Input(FormatError::Io(_)))
    ));
}
