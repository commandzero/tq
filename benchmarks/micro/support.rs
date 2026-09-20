//! Deterministic, synthetic event-stream fixtures shared by benchmark targets.
#![allow(
    dead_code,
    reason = "each benchmark target uses a subset of the shared fixtures"
)]

use criterion::Criterion;
use serde_json::{Value as Json, json};
use std::{fmt::Write as _, hint::black_box, io, time::Duration};
use tq_core::Value;

pub const QUERY: &str = "select(length == 2 and (.[0] | length) == 1)";
pub const SIZES: [usize; 2] = [16, 1024];
pub const SHAPES: [&str; 2] = ["nested", "tabular"];

pub fn configuration() -> Criterion {
    Criterion::default()
        .sample_size(30)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(3))
        .without_plots()
}

pub struct Fixture {
    pub model: Json,
    pub json: Vec<u8>,
    pub toon: Vec<u8>,
    pub records: Vec<Value>,
    pub results: Vec<Value>,
}

impl Fixture {
    pub fn new(shape: &str, size: usize) -> Self {
        assert!(SHAPES.contains(&shape));
        assert!(size > 0);
        let nested = shape == "nested";
        let items: Vec<_> = (0..size)
            .map(|id| {
                if nested {
                    json!({"id": id, "meta": {"flag": true}})
                } else {
                    json!({"id": id, "flag": true})
                }
            })
            .collect();
        let model = json!({"items": items, "tail": 7});
        let mut toon = if nested {
            format!("items[{size}]:\n")
        } else {
            format!("items[{size}]{{id,flag}}:\n")
        };
        for id in 0..size {
            if nested {
                writeln!(toon, "  - id: {id}\n    meta:\n      flag: true").unwrap();
            } else {
                writeln!(toon, "  {id},true").unwrap();
            }
        }
        toon.push_str("tail: 7\n");
        let mut records = Vec::new();
        expected_records(&model, &mut Vec::new(), &mut records);
        Self {
            json: serde_json::to_vec(&model).unwrap(),
            model,
            toon: toon.into_bytes(),
            records,
            results: vec![value(json!([["tail"], 7]))],
        }
    }

    pub fn output(format: &str) -> &'static [u8] {
        match format {
            "json" => b"[[\"tail\"],7]\n",
            "toon" => b"[2]:\n  - [1]: tail\n  - 7",
            _ => panic!("unknown fixture output"),
        }
    }
}

pub fn value(json: Json) -> Value {
    Value::from_json(json).unwrap()
}

// Independent jq stream contract over the fixture model, not a decoder call.
fn expected_records(model: &Json, path: &mut Vec<Json>, records: &mut Vec<Value>) {
    let children: Vec<(Json, &Json)> = match model {
        Json::Object(object) => object
            .iter()
            .map(|(key, value)| (json!(key), value))
            .collect(),
        Json::Array(array) => array
            .iter()
            .enumerate()
            .map(|(index, value)| (json!(index), value))
            .collect(),
        _ => Vec::new(),
    };
    if children.is_empty() {
        records.push(value(json!([path, model])));
    } else {
        for (key, child) in &children {
            path.push(key.clone());
            expected_records(child, path, records);
            path.pop();
        }
        path.push(children.last().unwrap().0.clone());
        records.push(value(json!([path])));
        path.pop();
    }
}

pub fn validate_small_contract() {
    let fixture = Fixture::new("tabular", 1);
    let expected = [
        json!([["items", 0, "id"], 0]),
        json!([["items", 0, "flag"], true]),
        json!([["items", 0, "flag"]]),
        json!([["items", 0]]),
        json!([["tail"], 7]),
        json!([["tail"]]),
    ]
    .into_iter()
    .map(value)
    .collect::<Vec<_>>();
    assert_eq!(fixture.records, expected);
}

/// Constant-space writer: includes byte consumption in the timed operation.
#[derive(Default)]
pub struct ByteSink {
    pub bytes: u64,
    pub checksum: u64,
}
impl io::Write for ByteSink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        for byte in black_box(bytes) {
            self.checksum = self.checksum.wrapping_add(u64::from(*byte));
        }
        self.bytes += bytes.len() as u64;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
