//! Header-shaped row documents through committed native input.

use tq_formats::{DecodeOptions, InputRepresentation, NativeFormat, NativeInputObservation};

#[test]
fn delimited_input_rejects_duplicate_headers_and_excess_fields_without_retracting_rows() {
    let selection = NativeFormat::Csv
        .select_input(DecodeOptions::default(), InputRepresentation::Documents)
        .unwrap();
    let mut duplicate = selection.open(b"a,a\n1,2\n".as_slice(), "duplicate");
    assert!(
        duplicate
            .next_observation()
            .unwrap_err()
            .to_string()
            .contains("duplicate header")
    );
    assert!(duplicate.next_observation().unwrap().is_none());
    let mut input = selection.open(b"a,b\n1\n2,3,4\n".as_slice(), "width");
    let NativeInputObservation::Document(document) = input.next_observation().unwrap().unwrap()
    else {
        panic!("row")
    };
    assert_eq!(document.value.to_string(), r#"{"a":1,"b":null}"#);
    assert!(
        input
            .next_observation()
            .unwrap_err()
            .to_string()
            .contains("more fields")
    );
    assert!(input.next_observation().unwrap().is_none());
}

#[test]
fn delimited_input_preserves_header_order_scalar_types_and_quoted_newlines() {
    for format in [NativeFormat::Csv, NativeFormat::Tsv] {
        let delimiter = if format == NativeFormat::Csv {
            ","
        } else {
            "\t"
        };
        let bytes = [
            "name,age,active,note",
            "\"a\nb\",42,true,\"42\"",
            "c,\"\",false,",
        ]
        .join("\n")
        .replace(',', delimiter);
        let mut input = format
            .select_input(DecodeOptions::default(), InputRepresentation::Documents)
            .unwrap()
            .open(bytes.as_bytes(), "rows");
        for (index, expected) in [
            r#"{"name":"a\nb","age":42,"active":true,"note":"42"}"#,
            r#"{"name":"c","age":"","active":false,"note":null}"#,
        ]
        .into_iter()
        .enumerate()
        {
            let NativeInputObservation::Document(document) =
                input.next_observation().unwrap().unwrap()
            else {
                panic!("row document")
            };
            assert_eq!(document.value.to_string(), expected);
            assert_eq!(document.index, index as u64);
        }
        assert!(input.next_observation().unwrap().is_none());
    }
}
