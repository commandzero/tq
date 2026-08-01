#![no_main]

use std::io::{BufReader, Cursor};

use libfuzzer_sys::fuzz_target;
use tq_core::SourceId;
use tq_toon::{Decoder, DecoderConfig};

fuzz_target!(|data: &[u8]| {
    let mut config = DecoderConfig::default();
    config.maximum_depth = 64;
    config.maximum_token_bytes = 64 * 1024;
    config.maximum_line_bytes = 128 * 1024;
    config.maximum_lookahead_bytes = 4 * 1024;
    let reader = BufReader::with_capacity(7, Cursor::new(data));
    let mut decoder = Decoder::new(reader, SourceId::new(0), config);
    while matches!(decoder.next_event(), Ok(Some(_))) {}
});
