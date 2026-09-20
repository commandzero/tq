//! Public format operations shared by format and runner correctness checks.
#![allow(
    dead_code,
    reason = "each target uses a subset of these format helpers"
)]
use std::io::{Cursor, Write};
use tq_core::Value;
use tq_formats::{
    NativeFormat, NativeOutputSequence, OutputFormat, OutputOptions, StreamOptions, ToonFraming,
    stream_json, stream_toon,
};
use tq_toon::DecoderConfig;

pub fn stream(format: &str, bytes: &[u8], emit: impl FnMut(Value) -> Result<(), String>) {
    match format {
        "json" => stream_json(Cursor::new(bytes), StreamOptions::default(), emit).unwrap(),
        "toon" => stream_toon(
            Cursor::new(bytes),
            DecoderConfig::default(),
            StreamOptions::default(),
            emit,
        )
        .unwrap(),
        _ => panic!("unknown input format"),
    }
}

pub fn encode(format: &str, values: &[Value], writer: &mut impl Write) {
    let output = if format == "json" {
        OutputFormat::Json
    } else {
        OutputFormat::Toon
    };
    let options = OutputOptions {
        format: output,
        toon_framing: if format == "toon" {
            ToonFraming::Unframed
        } else {
            OutputOptions::default().toon_framing
        },
        ..OutputOptions::default()
    };
    let mut sequence = NativeOutputSequence::new(
        NativeFormat::from_output(output)
            .select_output(options)
            .unwrap(),
    );
    for value in values {
        sequence.write_result(writer, value).unwrap();
    }
    sequence.finish(writer).unwrap();
}
