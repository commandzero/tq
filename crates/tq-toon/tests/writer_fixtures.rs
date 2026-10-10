//! Exact-byte checks against the official TOON encode fixtures.

mod support;

use std::io::Cursor;

use tq_core::{Number, SourceId, Span, Value, presentation::ColorPalette};
use tq_toon::{
    ArrayPreparationConfig, Decoder, DecoderConfig, DomBuilder, DuplicateKeyPolicy, Event,
    EventConsumer, PreparationArena, PreparationLimits, TranscodeCommitment, TranscodeConsumer,
    WriterConfig, decode_to_value, encode, write_value,
};

use support::{assert_ordered, fixtures, header_ordered, writer_config};

#[test]
fn canonical_toon_numbers_do_not_inherit_json_literal_presentation() {
    for (literal, expected) in [
        ("1.000", "1"),
        ("100e-2", "1"),
        ("12.3400", "12.34"),
        ("-0.0", "0"),
        ("1e2", "100"),
    ] {
        let value = Value::Number(Number::parse(literal).unwrap());
        assert_eq!(
            encode(&value, WriterConfig::default()),
            expected,
            "{literal}"
        );
        let mut output = Vec::new();
        write_value(&mut output, &value, WriterConfig::default()).unwrap();
        assert_eq!(output, expected.as_bytes(), "{literal} sink output");
    }
    let computed_zero = Value::Number(Number::from_runtime_f64(-0.0));
    assert_eq!(encode(&computed_zero, WriterConfig::default()), "0");
}

#[test]
fn strings_preserve_canonical_quoting_across_unicode_controls_and_delimiters() {
    let palette = ColorPalette::from_jq_colors("10:11:12:13:14:15:16:17");
    for (value, expected) in [
        ("λ🦀", "λ🦀"),
        ("a,b\tc|d", "\"a,b\\tc|d\""),
        ("#heading", "\"#heading\""),
        ("literal\"number:1", "\"literal\\\"number:1\""),
        ("control\u{0001}bmp", "\"control\\u0001bmp\""),
    ] {
        let value = Value::String(value.into());
        let plain = encode(&value, WriterConfig::default());
        assert_eq!(plain, expected);
        let mut colored = Vec::new();
        tq_toon::write_value_colored(
            &mut colored,
            &value,
            WriterConfig::default(),
            Some(&palette),
        )
        .unwrap();
        assert_eq!(strip_sgr(&colored), plain.as_bytes());
    }
}

#[test]
fn literal_leading_bom_round_trips_at_root_field_and_array_positions() {
    for (input, expected) in [
        (serde_json::json!("\u{feff}data"), "\"\u{feff}data\""),
        (
            serde_json::json!({"payload": "\u{feff}data"}),
            "payload: \u{feff}data",
        ),
        (serde_json::json!(["\u{feff}data"]), "[1]: \u{feff}data"),
    ] {
        let value = Value::from_json(input).unwrap();
        let config = WriterConfig::default();
        assert_eq!(encode(&value, config), expected);
        let decoded = decode_to_value(
            Cursor::new(expected),
            SourceId::new(1),
            DecoderConfig::default(),
        )
        .unwrap();
        assert_ordered(&decoded, &value, "literal BOM round trip");
        for palette in [None, Some(ColorPalette::default())] {
            for threshold in [0, 4096] {
                assert_eq!(
                    transcode_value(&value, config, threshold, palette.clone()),
                    expected.as_bytes(),
                );
            }
        }
    }
}

fn official_encode_fixtures() -> Vec<(String, serde_json::Value)> {
    let mut all_fixtures = fixtures("encode");
    all_fixtures.push((
        "migration-v4.1.json".to_owned(),
        serde_json::from_str(include_str!("fixtures/migration-v4.1.json")).unwrap(),
    ));
    all_fixtures
}

#[test]
fn canonical_writer_matches_every_official_encode_fixture() {
    for (path, fixture) in official_encode_fixtures() {
        for (index, test) in fixture["tests"].as_array().unwrap().iter().enumerate() {
            let context = format!("{path}[{index}]: {}", test["name"].as_str().unwrap());
            let config = writer_config(test);
            let value = Value::from_json(test["input"].clone()).unwrap();
            let expected = test["expected"].as_str().unwrap();
            assert!(
                !test["shouldError"].as_bool().unwrap_or(false),
                "{context}: unhandled encode error fixture"
            );
            assert_eq!(
                encode(&value, config),
                expected,
                "{context}: native encoder bytes"
            );
            let mut sink_output = Vec::new();
            write_value(&mut sink_output, &value, config).unwrap();
            assert_eq!(sink_output, expected.as_bytes(), "{context}: sink bytes");
            assert!(
                !sink_output.ends_with(b"\n"),
                "{context}: document-internal trailing LF"
            );
            let reference_options = toon_format::EncodeOptions::new()
                .with_spaces(config.indent_size)
                .with_delimiter(match config.delimiter {
                    tq_toon::Delimiter::Comma => toon_format::Delimiter::Comma,
                    tq_toon::Delimiter::Tab => toon_format::Delimiter::Tab,
                    tq_toon::Delimiter::Pipe => toon_format::Delimiter::Pipe,
                });
            let reference = toon_format::encode(&test["input"], &reference_options)
                .unwrap_or_else(|error| panic!("{context}: oracle encode: {error}"));
            assert_eq!(
                reference, expected,
                "{context}: oracle/official encoder agreement"
            );
        }
    }
}

#[test]
fn colored_writer_matches_every_official_encode_fixture() {
    for (path, fixture) in official_encode_fixtures() {
        for (index, test) in fixture["tests"].as_array().unwrap().iter().enumerate() {
            let context = format!("{path}[{index}]: {}", test["name"].as_str().unwrap());
            let config = writer_config(test);
            let value = Value::from_json(test["input"].clone()).unwrap();
            let expected = test["expected"].as_str().unwrap().as_bytes();
            for palette in [None, Some(ColorPalette::default())] {
                let mut colored = Vec::new();
                tq_toon::write_value_colored(&mut colored, &value, config, palette.as_ref())
                    .unwrap();
                assert_eq!(
                    strip_sgr(&colored),
                    expected,
                    "{context}: colored sink bytes"
                );
            }
        }
    }
}

#[test]
fn semantic_event_transcode_matches_every_official_encode_fixture() {
    for (path, fixture) in official_encode_fixtures() {
        for (index, test) in fixture["tests"].as_array().unwrap().iter().enumerate() {
            let context = format!("{path}[{index}]: {}", test["name"].as_str().unwrap());
            let config = writer_config(test);
            let value = Value::from_json(test["input"].clone()).unwrap();
            let expected = test["expected"].as_str().unwrap().as_bytes();
            for palette in [None, Some(ColorPalette::default())] {
                for memory_threshold_bytes in [0, 4096] {
                    assert_eq!(
                        transcode_value(&value, config, memory_threshold_bytes, palette.clone()),
                        expected,
                        "{context}: semantic event transcode, memory threshold {memory_threshold_bytes}",
                    );
                }
            }
        }
    }
}

#[test]
fn official_encode_fixtures_round_trip_through_dom_and_events() {
    for (path, fixture) in official_encode_fixtures() {
        for (index, test) in fixture["tests"].as_array().unwrap().iter().enumerate() {
            let context = format!("{path}[{index}]: {}", test["name"].as_str().unwrap());
            let config = writer_config(test);
            let value = Value::from_json(test["input"].clone()).unwrap();
            let expected = test["expected"].as_str().unwrap();
            let decode_config = DecoderConfig {
                indent_size: config.indent_size,
                ..DecoderConfig::default()
            };
            let expected_value = header_ordered(&value);
            let decoded = decode_to_value(Cursor::new(expected), SourceId::new(1), decode_config)
                .unwrap_or_else(|error| panic!("{context}: official byte round trip: {error}"));
            assert_ordered(
                &decoded,
                &expected_value,
                &format!("{context} DOM round trip"),
            );
            let mut events = Decoder::new(Cursor::new(expected), SourceId::new(2), decode_config);
            let mut builder = DomBuilder::new();
            events.decode_into(&mut builder).unwrap();
            assert_ordered(
                &builder.finish().unwrap(),
                &decoded,
                &format!("{context} event round trip"),
            );
        }
    }
}

fn transcode_value(
    value: &Value,
    config: WriterConfig,
    memory_threshold_bytes: usize,
    palette: Option<ColorPalette>,
) -> Vec<u8> {
    let span = Span::new(SourceId::new(1), 0, 1);
    let mut transcode = TranscodeConsumer::new(
        Vec::new(),
        config,
        ArrayPreparationConfig {
            memory_threshold_bytes,
            ..ArrayPreparationConfig::default()
        },
        PreparationArena::new(PreparationLimits::default()),
        DuplicateKeyPolicy::Reject,
        TranscodeCommitment::AtomicUnframed,
    );
    if let Some(palette) = palette {
        transcode = transcode.with_palette(palette);
    }
    transcode.consume(Event::DocumentStart { span }).unwrap();
    feed_value(&mut transcode, value, span);
    transcode.consume(Event::DocumentEnd { span }).unwrap();
    let raw = transcode.into_inner();
    assert!(std::str::from_utf8(&raw).is_ok(), "encoded colored UTF-8");
    strip_sgr(&raw)
}

fn feed_value(transcode: &mut TranscodeConsumer<Vec<u8>>, value: &Value, span: Span) {
    match value {
        Value::Null => transcode.consume_null(span).unwrap(),
        Value::Bool(value) => transcode.consume_bool(span, *value).unwrap(),
        Value::Number(value) => transcode
            .consume_number_literal(span, value.to_string())
            .unwrap(),
        Value::String(value) => transcode
            .consume_text_string(span, value.to_string())
            .unwrap(),
        Value::Object(values) => {
            transcode.consume(Event::ObjectStart { span }).unwrap();
            for (key, value) in values.iter() {
                transcode
                    .consume_text_key(span, key.to_string(), false)
                    .unwrap();
                feed_value(transcode, value, span);
            }
            transcode.consume(Event::ObjectEnd { span }).unwrap();
        }
        Value::Array(values) => {
            let count = u64::try_from(values.len()).unwrap();
            transcode
                .consume(Event::ArrayStart {
                    span,
                    declared_count: Some(count),
                })
                .unwrap();
            for value in values.iter() {
                feed_value(transcode, value, span);
            }
            transcode
                .consume(Event::ArrayEnd {
                    span,
                    observed_count: count,
                })
                .unwrap();
        }
    }
}

#[test]
fn lightweight_transcode_matches_dom_numeric_projection() {
    for literal in ["1.000", "100e-2", "-0.0", "1E1234567890"] {
        let span = Span::new(SourceId::new(1), 0, u64::try_from(literal.len()).unwrap());
        let arena = PreparationArena::new(PreparationLimits::default());
        let mut transcode = TranscodeConsumer::new(
            Vec::new(),
            WriterConfig::default(),
            ArrayPreparationConfig::default(),
            arena,
            DuplicateKeyPolicy::LastValueFirstPosition,
            TranscodeCommitment::AtomicUnframed,
        );
        transcode.consume(Event::DocumentStart { span }).unwrap();
        transcode
            .consume_number_literal(span, literal.to_owned())
            .unwrap();
        transcode.consume(Event::DocumentEnd { span }).unwrap();
        let fast = String::from_utf8(transcode.into_inner()).unwrap();
        let dom = encode(
            &Value::Number(Number::parse(literal).unwrap()),
            WriterConfig::default(),
        );
        assert_eq!(fast, dom, "{literal}");
    }
    assert_eq!(
        encode(
            &Value::Number(Number::from_runtime_f64(-0.0)),
            WriterConfig::default()
        ),
        "0"
    );
}

fn strip_sgr(bytes: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"\x1b[") {
            index += 2;
            while index < bytes.len() && bytes[index] != b'm' {
                index += 1;
            }
            assert!(index < bytes.len(), "unterminated SGR");
            index += 1;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    output
}

fn transcode_object(commitment: TranscodeCommitment, palette: Option<ColorPalette>) -> Vec<u8> {
    let span = Span::new(SourceId::new(1), 0, 1);
    let mut transcode = TranscodeConsumer::new(
        Vec::new(),
        WriterConfig::default(),
        ArrayPreparationConfig::default(),
        PreparationArena::new(PreparationLimits::default()),
        DuplicateKeyPolicy::Reject,
        commitment,
    );
    if let Some(palette) = palette {
        transcode = transcode.with_palette(palette);
    }
    transcode.consume(Event::DocumentStart { span }).unwrap();
    transcode.consume(Event::ObjectStart { span }).unwrap();
    transcode
        .consume_text_key(span, "arr".to_owned(), false)
        .unwrap();
    transcode
        .consume(Event::ArrayStart {
            span,
            declared_count: Some(1),
        })
        .unwrap();
    transcode
        .consume_text_string(span, "a,b".to_owned())
        .unwrap();
    transcode
        .consume(Event::ArrayEnd {
            span,
            observed_count: 1,
        })
        .unwrap();
    transcode
        .consume_text_key(span, "count".to_owned(), false)
        .unwrap();
    transcode
        .consume_number_literal(span, "2".to_owned())
        .unwrap();
    transcode.consume(Event::ObjectEnd { span }).unwrap();
    transcode.consume(Event::DocumentEnd { span }).unwrap();
    transcode.into_inner()
}

#[test]
fn colored_transcode_direct_and_atomic_paths_strip_to_plain_bytes() {
    let palette = ColorPalette::from_jq_colors("10:11:12:13:14:15:16:17");
    for commitment in [
        TranscodeCommitment::DirectSequence,
        TranscodeCommitment::DirectValues,
        TranscodeCommitment::AtomicUnframed,
    ] {
        let plain = transcode_object(commitment, None);
        let colored = transcode_object(commitment, Some(palette.clone()));
        assert_eq!(strip_sgr(&colored), plain, "{commitment:?}");
        if commitment != TranscodeCommitment::AtomicUnframed {
            assert!(colored.ends_with(b"\n"), "{commitment:?}");
            assert!(colored.ends_with(b"\x1b[0m\n"), "{commitment:?}");
        }
    }
}

fn assert_colons_are_plain(bytes: &[u8]) {
    let mut styled = false;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"\x1b[") {
            let end = index + bytes[index..].iter().position(|b| *b == b'm').unwrap();
            styled = &bytes[index..=end] != b"\x1b[0m";
            index = end;
        } else if bytes[index] == b':' {
            assert!(!styled, "colon has a style: {bytes:?}");
        }
        index += 1;
    }
}

#[test]
fn toon_colons_are_unstyled_in_native_and_prepared_layouts() {
    for palette in [
        ColorPalette::default(),
        ColorPalette::from_jq_colors("10:11:12:13:14:15:16:17"),
    ] {
        for source in [
            r#"{"x":1,"child":{"y":2}}"#,
            "[]",
            "[1,2]",
            r#"[{"x":1},{"x":2}]"#,
            "[{},[1]]",
        ] {
            let value: Value = serde_json::from_str(source).unwrap();
            let mut output = Vec::new();
            tq_toon::write_value_colored(
                &mut output,
                &value,
                WriterConfig::default(),
                Some(&palette),
            )
            .unwrap();
            assert_colons_are_plain(&output);
            assert_eq!(
                strip_sgr(&output),
                encode(&value, WriterConfig::default()).as_bytes()
            );
            if let Value::Array(values) = &value {
                for memory_threshold_bytes in [0, 4096] {
                    let config = ArrayPreparationConfig {
                        memory_threshold_bytes,
                        ..ArrayPreparationConfig::default()
                    };
                    let arena = PreparationArena::new(PreparationLimits {
                        memory_bytes: 1024 * 1024,
                        ..PreparationLimits::default()
                    });
                    let mut prepared = tq_toon::PreparedArray::in_arena(config, arena);
                    for value in values.iter() {
                        prepared.push(value).unwrap();
                    }
                    let mut replay = Vec::new();
                    prepared
                        .write_to_colored(&mut replay, WriterConfig::default(), Some(&palette))
                        .unwrap();
                    assert_colons_are_plain(&replay);
                    assert_eq!(strip_sgr(&replay), strip_sgr(&output));
                }
            }
        }
        for commitment in [
            TranscodeCommitment::DirectValues,
            TranscodeCommitment::AtomicUnframed,
        ] {
            assert_colons_are_plain(&transcode_object(commitment, Some(palette.clone())));
        }
    }
}

#[test]
fn repeated_replacements_release_keys_and_keep_first_positions_under_budget() {
    use std::fmt::Write as _;

    let mut input = String::from("same: 0\ntail: 9\n");
    for value in 1..2_000 {
        writeln!(input, "same: {value}").unwrap();
    }
    let arena = PreparationArena::new(PreparationLimits {
        memory_bytes: 32_768,
        spool_bytes: 16 * 1024 * 1024,
        ..PreparationLimits::default()
    });
    let mut output = Vec::new();
    let mut consumer = TranscodeConsumer::new(
        &mut output,
        WriterConfig::default(),
        ArrayPreparationConfig::default(),
        arena.clone(),
        DuplicateKeyPolicy::LastValueFirstPosition,
        TranscodeCommitment::DirectSequence,
    );
    let mut decoder = Decoder::new(
        Cursor::new(input.as_bytes()),
        SourceId::new(1),
        DecoderConfig {
            strict: false,
            ..DecoderConfig::default()
        },
    );
    while let Some(event) = decoder.next_event().unwrap() {
        consumer.consume(event).unwrap();
    }
    drop(consumer);
    assert_eq!(output, b"\x1esame: 1999\ntail: 9\n");
    let observations = arena.observations();
    assert!(observations.memory_high_water_bytes <= 32_768);
    assert!(observations.spool_bytes_written > 0);
}

#[test]
fn bounded_identity_values_remain_in_memory_when_disk_is_denied() {
    let arena = PreparationArena::new(PreparationLimits {
        memory_bytes: 1024 * 1024,
        spool_bytes: 0,
        ..PreparationLimits::default()
    });
    let preparation = ArrayPreparationConfig {
        memory_threshold_bytes: 1024 * 1024,
        maximum_spool_bytes: 0,
        ..ArrayPreparationConfig::default()
    };
    let mut output = Vec::new();
    let mut consumer = TranscodeConsumer::new(
        &mut output,
        WriterConfig::default(),
        preparation,
        arena.clone(),
        DuplicateKeyPolicy::Reject,
        TranscodeCommitment::DirectValues,
    );
    let mut decoder = Decoder::new(
        Cursor::new(b"x: null\nflags[2]: false,true"),
        SourceId::new(1),
        DecoderConfig::default(),
    );
    while let Some(event) = decoder.next_event().unwrap() {
        consumer.consume(event).unwrap();
    }
    drop(consumer);
    assert_eq!(output, b"x: null\nflags[2]: false,true\n");
    let observations = arena.observations();
    assert_eq!(observations.spool_bytes_written, 0);
    assert_eq!(observations.spool_bytes_replayed, 0);
    assert!(observations.memory_high_water_bytes <= 1024 * 1024);
}

#[test]
fn transcode_and_document_writing_share_the_container_depth_boundary() {
    let span = Span::new(SourceId::new(1), 0, 1);
    let mut value = Value::Null;
    for _ in 0..256 {
        value = Value::array([value]);
    }
    let mut consumer = TranscodeConsumer::new(
        Vec::new(),
        WriterConfig::default(),
        ArrayPreparationConfig::default(),
        PreparationArena::new(PreparationLimits::default()),
        DuplicateKeyPolicy::Reject,
        TranscodeCommitment::DirectValues,
    );
    consumer.consume(Event::DocumentStart { span }).unwrap();
    feed_value(&mut consumer, &value, span);
    consumer.consume(Event::DocumentEnd { span }).unwrap();
    assert_eq!(
        consumer.into_inner(),
        format!("{}\n", encode(&value, WriterConfig::default())).as_bytes()
    );

    let value = Value::array([value]);
    assert!(matches!(
        write_value(Vec::new(), &value, WriterConfig::default()),
        Err(tq_toon::WriterError::Schema {
            resource: "depth",
            limit: 256
        })
    ));
    let mut consumer = TranscodeConsumer::new(
        Vec::new(),
        WriterConfig::default(),
        ArrayPreparationConfig::default(),
        PreparationArena::new(PreparationLimits::default()),
        DuplicateKeyPolicy::Reject,
        TranscodeCommitment::DirectValues,
    );
    consumer.consume(Event::DocumentStart { span }).unwrap();
    for _ in 0..256 {
        consumer
            .consume(Event::ArrayStart {
                span,
                declared_count: Some(1),
            })
            .unwrap();
    }
    assert!(matches!(
        consumer.consume(Event::ArrayStart {
            span,
            declared_count: Some(1)
        }),
        Err(tq_toon::TranscodeError::Writer(
            tq_toon::WriterError::Schema {
                resource: "depth",
                limit: 256
            }
        ))
    ));
    assert_eq!(consumer.into_inner(), [] as [u8; 0]);
}
