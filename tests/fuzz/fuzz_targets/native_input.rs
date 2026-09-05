#![no_main]

use libfuzzer_sys::fuzz_target;
use std::{io::{self, Read}, ops::ControlFlow};
use tq_formats::{DecodeOptions, InputRepresentation, NativeFormat};

struct Chunks<'a>(&'a [u8], usize);

impl Read for Chunks<'_> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let count = output.len().min(self.0.len()).min(self.1);
        output[..count].copy_from_slice(&self.0[..count]);
        self.0 = &self.0[count..];
        Ok(count)
    }
}

fuzz_target!(|data: &[u8]| {
    let Some((&selector, bytes)) = data.split_first() else { return };
    let format = [NativeFormat::JsonSequence, NativeFormat::ToonSequence, NativeFormat::Csv, NativeFormat::Tsv][usize::from(selector) % 4];
    let options = DecodeOptions {
        maximum_source_bytes: 65_536,
        maximum_frame_bytes: 4096,
        maximum_line_bytes: 4096,
        maximum_token_bytes: 1024,
        maximum_depth: 32,
        maximum_fields: 64,
        ..DecodeOptions::default()
    };
    let reader = || Chunks(bytes, 1 + usize::from(selector >> 2));
    let mut input = format.select_input(options, InputRepresentation::Documents).unwrap().open(reader(), "fuzz");
    while matches!(input.next_observation(), Ok(Some(_))) {}
    if format == NativeFormat::JsonSequence {
        let input = format.select_input(options, InputRepresentation::Events).unwrap().open(reader(), "fuzz");
        let mut remaining = 4096;
        let _ = input.consume_events(|_| {
            remaining -= 1;
            Ok::<_, io::Error>(if remaining == 0 { ControlFlow::Break(()) } else { ControlFlow::Continue(()) })
        });
    }
});
