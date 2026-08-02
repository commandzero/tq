#![no_main]

use libfuzzer_sys::fuzz_target;
use tq_core::{ResolveOptions, analyze, parse_bytes, resolve};

fuzz_target!(|data: &[u8]| {
    if data.len() > 64 * 1024 {
        return;
    }
    if let Ok(parsed) = parse_bytes("<fuzz-query>", data)
        && let Ok(resolved) = resolve(parsed, &ResolveOptions::default())
    {
        let _ = analyze(resolved).compile();
    }
});
