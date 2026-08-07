#![no_main]

use libfuzzer_sys::fuzz_target;
use tq_cli::parse_args;

fuzz_target!(|data: &[u8]| {
    if data.len() > 64 * 1024 {
        return;
    }
    let arguments = data
        .split(|byte| *byte == 0)
        .filter_map(|bytes| std::str::from_utf8(bytes).ok())
        .take(128)
        .collect::<Vec<_>>();
    let _ = parse_args(arguments);
});
