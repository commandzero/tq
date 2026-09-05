//! Reproducible Unicode and tiny-chunk profile round trips.

use std::io::{self, Read};
use tq_core::{Object, Value};
use tq_formats::{
    DecodeOptions, InputRepresentation, NativeFormat, NativeInputObservation, NativeOutputSequence,
    OutputOptions,
};

struct Chunks<'a> {
    bytes: &'a [u8],
    size: usize,
}
impl Read for Chunks<'_> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let count = self.bytes.len().min(output.len()).min(self.size);
        output[..count].copy_from_slice(&self.bytes[..count]);
        self.bytes = &self.bytes[count..];
        Ok(count)
    }
}

#[test]
fn delimited_property_unicode_strings_round_trip_under_chunking() {
    let mut seed = 0x6a09_e667_f3bc_c909_u64;
    for iteration in 0..200 {
        let mut text = String::from("42,true,false,null,\"\t\n\r\0");
        for _ in 0..iteration % 32 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            if let Some(character) = char::from_u32((seed % 0x11_0000) as u32) {
                text.push(character);
            }
        }
        let mut object = Object::new();
        object.insert(text.clone().into(), Value::string(text));
        object.insert("numeric".into(), Value::string("42"));
        object.insert("empty".into(), Value::string(""));
        object.insert("optional".into(), Value::Null);
        let value = Value::object(object);
        for format in [NativeFormat::Csv, NativeFormat::Tsv] {
            let mut output =
                NativeOutputSequence::new(format.select_output(OutputOptions::default()).unwrap());
            let mut bytes = Vec::new();
            output.write_result(&mut bytes, &value).unwrap();
            output.finish(&mut bytes).unwrap();
            for size in 1..=7 {
                let mut input = format
                    .select_input(DecodeOptions::default(), InputRepresentation::Documents)
                    .unwrap()
                    .open(
                        Chunks {
                            bytes: &bytes,
                            size,
                        },
                        "property",
                    );
                let NativeInputObservation::Document(document) =
                    input.next_observation().unwrap().unwrap()
                else {
                    panic!("row")
                };
                assert_eq!(document.value, value, "iteration {iteration}, chunk {size}");
                assert!(input.next_observation().unwrap().is_none());
            }
        }
    }
}
