//! jq-compatible non-finite numbers at the public JSON format boundary.

use tq_core::{SourceId, Value};
use tq_formats::{JsonEventOptions, decode_json, decode_json_event_stream};
use tq_toon::{Event, EventConsumer, Scalar};

fn assert_runtime_number(value: &Value, expected: f64) {
    let Value::Number(number) = value else {
        panic!("expected a number, got {value:?}");
    };

    let actual = number.as_f64();
    if expected.is_nan() {
        assert!(actual.is_nan(), "expected NaN, got {actual:?}");
    } else {
        assert_eq!(actual.to_bits(), expected.to_bits());
    }
}

#[test]
fn document_api_accepts_root_and_nested_nonfinite_numbers() {
    let documents = decode_json(b"NaN Infinity -Infinity", "nonfinite.json")
        .expect("strict jq JSON accepts non-finite numeric roots");
    assert_eq!(documents.len(), 3);
    assert_runtime_number(&documents[0].value, f64::NAN);
    assert_runtime_number(&documents[1].value, f64::INFINITY);
    assert_runtime_number(&documents[2].value, f64::NEG_INFINITY);

    let documents = decode_json(
        br#"{"root":NaN,"nested":[Infinity,{"negative":-Infinity}]}"#,
        "nested-nonfinite.json",
    )
    .expect("strict jq JSON accepts non-finite numbers in containers");
    let Value::Object(object) = &documents[0].value else {
        panic!("expected an object document");
    };
    assert_runtime_number(object.get("root").expect("root key"), f64::NAN);
    let Value::Array(nested) = object.get("nested").expect("nested key") else {
        panic!("expected an array");
    };
    assert_runtime_number(&nested[0], f64::INFINITY);
    let Value::Object(negative) = &nested[1] else {
        panic!("expected nested object");
    };
    assert_runtime_number(
        negative.get("negative").expect("negative key"),
        f64::NEG_INFINITY,
    );
}

#[derive(Default)]
struct NumberEvents(Vec<f64>);

impl EventConsumer for NumberEvents {
    type Error = std::convert::Infallible;

    fn consume(&mut self, event: Event) -> Result<(), Self::Error> {
        if let Event::Scalar {
            value: Scalar::Number(number),
            ..
        } = event
        {
            self.0.push(number.as_f64());
        }
        Ok(())
    }
}

#[test]
fn event_api_emits_typed_nonfinite_root_and_nested_scalars() {
    let mut events = NumberEvents::default();
    let documents = decode_json_event_stream(
        br#"NaN {"root":Infinity,"nested":[-Infinity]}"#.as_slice(),
        SourceId::new(41),
        &mut events,
        JsonEventOptions::default(),
    )
    .expect("strict jq JSON event decoding accepts non-finite numbers");

    assert_eq!(documents, 2);
    assert_eq!(events.0.len(), 3);
    assert!(events.0[0].is_nan());
    assert_eq!(events.0[1].to_bits(), f64::INFINITY.to_bits());
    assert_eq!(events.0[2].to_bits(), f64::NEG_INFINITY.to_bits());
}
