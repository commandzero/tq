#![no_main]

use libfuzzer_sys::fuzz_target;
use tq_core::{ResolveOptions, analyze, parse_bytes, resolve};

fuzz_target!(|data: &[u8]| {
    let Ok(parsed) = parse_bytes("<fuzz-automatic-plan>", data) else {
        return;
    };
    let Ok(resolved) = resolve(parsed, &ResolveOptions::default()) else {
        return;
    };
    let analyzed = analyze(resolved);
    if let Ok(program) = analyzed.compile() {
        let _ = program.automatic_plan();
    }
});
