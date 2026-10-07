//! Complete pinned TOON 4.1 decode expectations, semantic events and oracle checks.

mod support;

use std::io::{BufReader, Cursor};

use serde_json::Value as JsonValue;
use tq_core::{Number, SourceId, Value};
use tq_toon::{Decoder, DecoderConfig, DomBuilder, decode_to_value};

use support::{assert_ordered, decoder_config, fixtures};

#[test]
fn official_decode_fixtures_match_dom_events_and_reference() {
    for (path, fixture) in fixtures("decode") {
        for (index, test) in fixture["tests"].as_array().unwrap().iter().enumerate() {
            let context = format!("{path}[{index}]: {}", test["name"].as_str().unwrap());
            let input = test["input"].as_str().unwrap();
            let config = decoder_config(test);
            let reference_options = toon_format::DecodeOptions::new()
                .with_strict(config.strict)
                .with_indent(toon_format::Indent::Spaces(config.indent_size));
            let reference = toon_format::decode::<JsonValue>(input, &reference_options);
            let actual = decode_to_value(Cursor::new(input.as_bytes()), SourceId::new(1), config);
            // A one-byte buffered reader also exercises fragmented UTF-8, escapes,
            // BOM and CRLF without changing fixture bytes or source positions.
            let mut decoder = Decoder::new(
                BufReader::with_capacity(1, Cursor::new(input.as_bytes())),
                SourceId::new(2),
                config,
            );
            let mut builder = DomBuilder::new();
            let events = decoder.decode_into(&mut builder);
            if test["shouldError"].as_bool().unwrap_or(false) {
                assert!(actual.is_err(), "{context}: DOM accepted {input:?}");
                assert!(events.is_err(), "{context}: events accepted {input:?}");
                assert!(
                    reference.is_err(),
                    "{context}: oracle disagrees with official expected error: {reference:?}"
                );
                let mut discard =
                    Decoder::new(Cursor::new(input.as_bytes()), SourceId::new(3), config);
                let discarded = loop {
                    match discard.next_event() {
                        Ok(Some(_)) => {}
                        result => break result,
                    }
                };
                assert!(
                    discarded.is_err(),
                    "{context}: discarding event consumer accepted invalid input"
                );
            } else {
                let expected = Value::from_json(test["expected"].clone()).unwrap();
                let actual =
                    actual.unwrap_or_else(|error| panic!("{context}: {error}; input={input:?}"));
                events.unwrap_or_else(|error| panic!("{context}: event decode: {error:?}"));
                let event_value = builder.finish().unwrap();
                let reference = Value::from_json(reference.unwrap_or_else(|error| {
                    panic!("{context}: oracle rejected official valid input: {error}")
                }))
                .unwrap();
                assert_ordered(&actual, &expected, &context);
                assert_ordered(&event_value, &expected, &format!("{context} events"));
                assert_ordered(
                    &event_value,
                    &actual,
                    &format!("{context} DOM/event agreement"),
                );
                assert_ordered(
                    &reference,
                    &expected,
                    &format!("{context} oracle/official agreement"),
                );
            }
        }
    }
}

#[test]
fn literal_dotted_and_prototype_keys_survive_nested_and_keyed_events() {
    for (input, expected) in [
        (
            "records[2]{a.b,meta{__proto__,constructor}}:\n  x,1,2\n  y,3,4",
            r#"{"records":[{"a.b":"x","meta":{"__proto__":1,"constructor":2}},{"a.b":"y","meta":{"__proto__":3,"constructor":4}}]}"#,
        ),
        (
            "users[2:]{profile{a.b,prototype}}:\n  alice: x,1\n  bob: y,2",
            r#"{"users":{"alice":{"profile":{"a.b":"x","prototype":1}},"bob":{"profile":{"a.b":"y","prototype":2}}}}"#,
        ),
    ] {
        let expected: Value = serde_json::from_str(expected).unwrap();
        let config = DecoderConfig::default();
        let mut decoder = Decoder::new(Cursor::new(input), SourceId::new(1), config);
        let mut builder = DomBuilder::new();
        decoder.decode_into(&mut builder).unwrap();
        assert_ordered(&builder.finish().unwrap(), &expected, input);
        assert_ordered(
            &decode_to_value(Cursor::new(input), SourceId::new(1), config).unwrap(),
            &expected,
            input,
        );
    }
}

#[test]
fn numeric_domains_are_explicit_without_tolerances_or_fixture_exclusions() {
    // These implementation-defined out-of-oracle-domain values are not
    // exclusions from the official suite. Each implementation's documented
    // policy is independently asserted; neither is used to bless the other.
    for (input, reference_expected) in [
        (
            "18446744073709551616",
            JsonValue::String("18446744073709551616".to_owned()),
        ),
        (
            "-9223372036854775809",
            JsonValue::String("-9223372036854775809".to_owned()),
        ),
        ("1E1234567890", JsonValue::String("1E1234567890".to_owned())),
    ] {
        let expected = Value::Number(Number::parse(input).unwrap());
        let config = DecoderConfig::default();
        let actual = decode_to_value(Cursor::new(input), SourceId::new(1), config).unwrap();
        assert_ordered(&actual, &expected, input);
        let mut decoder = Decoder::new(Cursor::new(input), SourceId::new(2), config);
        let mut builder = DomBuilder::new();
        decoder.decode_into(&mut builder).unwrap();
        assert_ordered(&builder.finish().unwrap(), &expected, input);
        let reference: JsonValue = toon_format::decode_default(input).unwrap();
        assert_eq!(
            reference, reference_expected,
            "{input}: documented oracle out-of-range policy"
        );
    }
    for input in [
        "-9223372036854775808",
        "18446744073709551615",
        "9007199254740993",
        "0.3333333333333333",
        "1e-10",
    ] {
        let expected = Value::Number(Number::parse(input).unwrap());
        let actual = decode_to_value(
            Cursor::new(input),
            SourceId::new(1),
            DecoderConfig::default(),
        )
        .unwrap();
        let reference: JsonValue = toon_format::decode_default(input).unwrap();
        assert_ordered(&actual, &expected, input);
        assert_ordered(&Value::from_json(reference).unwrap(), &expected, input);
    }
}

#[test]
fn strict_errors_are_enforced_without_dom_or_retained_values() {
    for input in [
        "a: 1\na: 2",
        "m[2:]{v}:\n  a: 1\n  a: 2",
        "items[0]{meta{x,x}}:",
        "m[0:]{meta{x,x}}:",
        "items[0]{meta{}}:",
        "m[0:]{a,a}:",
        "items[1|]{a,b}:\n  1|2",
        "items[2]{meta{x,y}}:\n  1,2",
        "m[1:]{meta{x,y}}:\n  a: 1",
        "m[1:]{meta{x,y}}:\n  a: 1,2,3",
    ] {
        let mut decoder = Decoder::new(
            Cursor::new(input),
            SourceId::new(1),
            DecoderConfig::default(),
        );
        let result = loop {
            match decoder.next_event() {
                Ok(Some(_)) => {}
                result => break result,
            }
        };
        assert!(
            result.is_err(),
            "discarding events accepted strict defect: {input:?}"
        );
    }
}

#[test]
fn surrogate_escapes_and_misplaced_scalars_fail_in_both_modes() {
    for input in [
        r#"value: "\uD800""#,
        r#"value: "\uDC00""#,
        r#"value: "\uD83D\uDE80""#,
        "items[1]:\n  - x: 1\n  bare",
        "object:\n  a: 1\n  bare",
    ] {
        for strict in [true, false] {
            let config = DecoderConfig {
                strict,
                ..DecoderConfig::default()
            };
            assert!(
                decode_to_value(Cursor::new(input), SourceId::new(1), config).is_err(),
                "{input:?}, strict={strict}"
            );
            let mut decoder = Decoder::new(Cursor::new(input), SourceId::new(2), config);
            let result = loop {
                match decoder.next_event() {
                    Ok(Some(_)) => {}
                    result => break result,
                }
            };
            assert!(
                result.is_err(),
                "{input:?}, strict={strict}: event-discard defect"
            );
        }
    }
}
