//! A native row shape spans incremental Result writes.

use tq_core::Value;
use tq_formats::{NativeFormat, NativeOutputSequence, OutputOptions};

#[test]
fn delimited_output_enforces_row_field_and_count_limits_before_header() {
    for limits in [
        tq_formats::DelimitedLimits {
            row_bytes: 3,
            ..Default::default()
        },
        tq_formats::DelimitedLimits {
            field_bytes: 2,
            ..Default::default()
        },
        tq_formats::DelimitedLimits {
            fields: 0,
            ..Default::default()
        },
    ] {
        let selection = NativeFormat::Csv
            .select_output(OutputOptions {
                delimited_limits: limits,
                ..Default::default()
            })
            .unwrap();
        let mut output = NativeOutputSequence::new(selection.clone());
        let mut bytes = Vec::new();
        let value: Value = serde_json::from_str(r#"{"a":"bbb"}"#).unwrap();
        assert!(
            output
                .write_result(&mut bytes, &value)
                .unwrap_err()
                .to_string()
                .contains("resource limit exceeded")
        );
        assert_eq!(bytes, [] as [u8; 0]);
    }
}

#[test]
fn delimited_output_validates_the_whole_row_before_committing_bytes() {
    let selection = NativeFormat::Csv
        .select_output(OutputOptions::default())
        .unwrap();
    for invalid in [r#"{"a":[],"b":1}"#, "[]", "42"] {
        let mut output = NativeOutputSequence::new(selection.clone());
        let mut bytes = Vec::new();
        assert!(
            output
                .write_result(&mut bytes, &serde_json::from_str::<Value>(invalid).unwrap())
                .is_err()
        );
        assert_eq!(bytes, [] as [u8; 0]);
        assert!(output.finish(&mut bytes).is_err());
    }
    for invalid in [r#"{"a":1,"b":2}"#, r#"{"a":{}}"#] {
        let mut output = NativeOutputSequence::new(selection.clone());
        let mut bytes = Vec::new();
        output
            .write_result(
                &mut bytes,
                &serde_json::from_str::<Value>(r#"{"a":1}"#).unwrap(),
            )
            .unwrap();
        assert!(
            output
                .write_result(&mut bytes, &serde_json::from_str::<Value>(invalid).unwrap())
                .is_err()
        );
        assert_eq!(bytes, b"a\n1\n");
    }
}

#[test]
fn delimited_output_round_trips_escaped_strings_and_empty_objects() {
    use tq_formats::{DecodeOptions, InputRepresentation, NativeInputObservation};
    for json in [r#"{"a":"comma,tab\tquote\"newline\n","":"null"}"#, "{}"] {
        let value: Value = serde_json::from_str(json).unwrap();
        for format in [NativeFormat::Csv, NativeFormat::Tsv] {
            let mut output =
                NativeOutputSequence::new(format.select_output(OutputOptions::default()).unwrap());
            let mut bytes = Vec::new();
            output.write_result(&mut bytes, &value).unwrap();
            output.finish(&mut bytes).unwrap();
            let mut input = format
                .select_input(DecodeOptions::default(), InputRepresentation::Documents)
                .unwrap()
                .open(bytes.as_slice(), "roundtrip");
            let NativeInputObservation::Document(document) =
                input.next_observation().unwrap().unwrap()
            else {
                panic!("row")
            };
            assert_eq!(document.value, value);
            assert!(input.next_observation().unwrap().is_none());
        }
    }
}

#[test]
fn delimited_output_preserves_scalar_types_and_header_order() {
    for (format, delimiter) in [(NativeFormat::Csv, ","), (NativeFormat::Tsv, "\t")] {
        let selection = format.select_output(OutputOptions::default()).unwrap();
        let mut output = NativeOutputSequence::new(selection);
        let mut bytes = Vec::new();
        let first: Value =
            serde_json::from_str(r#"{"b":"42","a":42,"c":"","d":null,"e":true}"#).unwrap();
        output.write_result(&mut bytes, &first).unwrap();
        output
            .write_result(
                &mut bytes,
                &serde_json::from_str::<Value>(r#"{"a":1}"#).unwrap(),
            )
            .unwrap();
        output.finish(&mut bytes).unwrap();
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            "b,a,c,d,e\n\"42\",42,\"\",,true\n,1,,,\n".replace(',', delimiter)
        );
    }
}
