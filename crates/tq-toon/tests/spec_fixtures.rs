//! Vendored TOON specification fixtures and decoder differential tests.

use std::{fs, io::Cursor, path::PathBuf};

use serde_json::Value as JsonValue;
use toon_format::types::PathExpansionMode as ReferencePathExpansion;
use tq_core::{SourceId, Value};
use tq_toon::{DecoderConfig, PathExpansion, decode_to_value};

fn fixture_files() -> Vec<PathBuf> {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/spec-v3/decode");
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

fn option<'a>(test: &'a JsonValue, name: &str) -> Option<&'a JsonValue> {
    test.get("options").and_then(|options| options.get(name))
}

#[test]
fn specification_decode_fixtures_match_expected_values_and_errors() {
    let mut exercised = 0_usize;
    for path in fixture_files() {
        let fixture: JsonValue = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        for test in fixture["tests"].as_array().unwrap() {
            let name = test["name"].as_str().unwrap();
            let input = test["input"].as_str().unwrap();
            let config = DecoderConfig {
                strict: option(test, "strict")
                    .and_then(JsonValue::as_bool)
                    .unwrap_or(true),
                indent_size: option(test, "indent")
                    .and_then(JsonValue::as_u64)
                    .map_or(2, |value| usize::try_from(value).unwrap()),
                path_expansion: match option(test, "expandPaths").and_then(JsonValue::as_str) {
                    Some("safe") => PathExpansion::Safe,
                    _ => PathExpansion::Off,
                },
                ..DecoderConfig::default()
            };

            let actual = decode_to_value(Cursor::new(input.as_bytes()), SourceId::new(1), config);
            let should_error = test
                .get("shouldError")
                .and_then(JsonValue::as_bool)
                .unwrap_or(false);
            if should_error {
                assert!(actual.is_err(), "{name}: unexpectedly decoded {input:?}");
            } else {
                let expected = Value::from_json(test["expected"].clone()).unwrap();
                assert_eq!(
                    actual.unwrap_or_else(|error| panic!("{name}: {error}; input={input:?}")),
                    expected,
                    "{name}; input={input:?}"
                );
            }
            exercised += 1;
        }
    }

    assert_eq!(exercised, 202, "fixture coverage unexpectedly changed");
}

#[test]
fn successful_default_strict_cases_match_reference_decoder() {
    let mut compared = 0_usize;
    for path in fixture_files() {
        let fixture: JsonValue = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        for test in fixture["tests"].as_array().unwrap() {
            if test
                .get("shouldError")
                .and_then(JsonValue::as_bool)
                .unwrap_or(false)
            {
                continue;
            }
            let name = test["name"].as_str().unwrap();
            let input = test["input"].as_str().unwrap();
            let strict = option(test, "strict")
                .and_then(JsonValue::as_bool)
                .unwrap_or(true);
            let indent = option(test, "indent")
                .and_then(JsonValue::as_u64)
                .map_or(2, |value| usize::try_from(value).unwrap());
            let expansion = match option(test, "expandPaths").and_then(JsonValue::as_str) {
                Some("safe") => ReferencePathExpansion::Safe,
                _ => ReferencePathExpansion::Off,
            };
            let reference_options = toon_format::DecodeOptions::new()
                .with_strict(strict)
                .with_indent(toon_format::Indent::Spaces(indent))
                .with_expand_paths(expansion);
            let reference = toon_format::decode::<JsonValue>(input, &reference_options)
                .unwrap_or_else(|error| panic!("reference rejected {name}: {error}"));
            let config = DecoderConfig {
                strict,
                indent_size: indent,
                path_expansion: match expansion {
                    ReferencePathExpansion::Safe => PathExpansion::Safe,
                    ReferencePathExpansion::Off => PathExpansion::Off,
                },
                ..DecoderConfig::default()
            };
            let actual = decode_to_value(Cursor::new(input.as_bytes()), SourceId::new(2), config)
                .unwrap_or_else(|error| panic!("tq rejected {name}: {error}"));
            assert_eq!(actual, Value::from_json(reference).unwrap(), "{name}");
            compared += 1;
        }
    }
    assert!(compared >= 160, "differential coverage unexpectedly shrank");
}
