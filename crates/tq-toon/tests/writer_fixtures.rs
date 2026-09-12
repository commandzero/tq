//! Exact-byte checks against the official TOON encode fixtures.

use std::{fs, io::Cursor, path::PathBuf};

use serde_json::Value as JsonValue;
use tq_core::{Number, SourceId, Span, Value};
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
