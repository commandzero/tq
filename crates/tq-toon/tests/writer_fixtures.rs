//! Exact-byte checks against the official TOON encode fixtures.

use std::{fs, io::Cursor, path::PathBuf};

use serde_json::Value as JsonValue;
use tq_core::{Number, SourceId, Span, Value, presentation::ColorPalette};
use tq_toon::{
    ArrayPreparationConfig, DecoderConfig, Delimiter, DuplicateKeyPolicy, Event, EventConsumer,
    KeyFolding, PathExpansion, PreparationArena, PreparationLimits, TranscodeCommitment,
    TranscodeConsumer, WriterConfig, decode_to_value, encode, write_value,
};

fn fixture_files() -> Vec<PathBuf> {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/spec-v3/encode");
    let mut files = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect::<Vec<_>>();
    files.sort();
    files
}

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
fn canonical_writer_matches_every_official_encode_fixture() {
    let mut exercised = 0_usize;
    for path in fixture_files() {
        let fixture: JsonValue = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        for test in fixture["tests"].as_array().unwrap() {
            let options = test.get("options").unwrap_or(&JsonValue::Null);
            let config = WriterConfig {
                indent_size: options
                    .get("indent")
                    .and_then(JsonValue::as_u64)
                    .map_or(2, |value| usize::try_from(value).unwrap()),
                delimiter: match options.get("delimiter").and_then(JsonValue::as_str) {
                    Some("\t") => Delimiter::Tab,
                    Some("|") => Delimiter::Pipe,
                    _ => Delimiter::Comma,
                },
                key_folding: match options.get("keyFolding").and_then(JsonValue::as_str) {
                    Some("safe") => KeyFolding::Safe,
                    _ => KeyFolding::Off,
                },
                flatten_depth: options
                    .get("flattenDepth")
                    .and_then(JsonValue::as_u64)
                    .map_or(usize::MAX, |value| usize::try_from(value).unwrap()),
            };
            let value = Value::from_json(test["input"].clone()).unwrap();
            let actual = encode(&value, config);
            let mut sink_output = Vec::new();
            write_value(&mut sink_output, &value, config).unwrap();
            assert_eq!(
                actual,
                test["expected"].as_str().unwrap(),
                "{}",
                test["name"].as_str().unwrap()
            );
            assert_eq!(
                sink_output,
                test["expected"].as_str().unwrap().as_bytes(),
                "{} sink output",
                test["name"].as_str().unwrap()
            );
            assert!(!sink_output.ends_with(b"\n"));
            let name = test["name"].as_str().unwrap();
            if name == "skips folding on sibling literal-key collision (safe mode)" {
                exercised += 1;
                continue;
            }
            let decoded = decode_to_value(
                Cursor::new(actual.as_bytes()),
                SourceId::new(1),
                DecoderConfig {
                    indent_size: config.indent_size,
                    path_expansion: if config.key_folding == KeyFolding::Safe {
                        PathExpansion::Safe
                    } else {
                        PathExpansion::Off
                    },
                    ..DecoderConfig::default()
                },
            )
            .unwrap_or_else(|error| panic!("{name} round trip: {error}"));
            assert_eq!(decoded, value, "{name} round trip");
            exercised += 1;
        }
    }
    assert_eq!(exercised, 147, "encode fixture coverage changed");
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
                    let mut prepared = tq_toon::PreparedArray::new(ArrayPreparationConfig {
                        memory_threshold_bytes,
                        ..ArrayPreparationConfig::default()
                    });
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
fn direct_transcode_preserves_plain_dash_prefixed_scalar_spelling() {
    let span = Span::new(SourceId::new(1), 0, 1);
    for palette in [None, Some(ColorPalette::default())] {
        let mut transcode = TranscodeConsumer::new(
            Vec::new(),
            WriterConfig::default(),
            ArrayPreparationConfig::default(),
            PreparationArena::new(PreparationLimits::default()),
            DuplicateKeyPolicy::Reject,
            TranscodeCommitment::DirectValues,
        );
        if let Some(palette) = palette {
            transcode = transcode.with_palette(palette);
        }
        transcode.consume(Event::DocumentStart { span }).unwrap();
        transcode.consume(Event::ObjectStart { span }).unwrap();
        transcode.consume_text_key(span, "x".into(), false).unwrap();
        transcode
            .consume_text_string(span, "-draft".into())
            .unwrap();
        transcode.consume(Event::ObjectEnd { span }).unwrap();
        transcode.consume(Event::DocumentEnd { span }).unwrap();
        assert_eq!(strip_sgr(&transcode.into_inner()), b"x: -draft\n");
    }
}
