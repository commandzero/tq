#![no_main]

use libfuzzer_sys::fuzz_target;
use tq_core::{ResolveOptions, Value, Vm, VmLimits, analyze, parse, resolve};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let mut end = text.len().min(512);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let bounded = &text[..end];
    let pattern = serde_json::to_string(bounded).expect("JSON string");
    let query = match data.first().copied().unwrap_or_default() % 4 {
        0 => format!("test({pattern})"),
        1 => format!("match({pattern}; \"g\")"),
        2 => format!("scan({pattern})"),
        _ => "try fromdateiso8601 catch .".to_owned(),
    };
    let Ok(parsed) = parse(&query) else {
        return;
    };
    let Ok(resolved) = resolve(parsed, &ResolveOptions::default()) else {
        return;
    };
    let Ok(program) = analyze(resolved).compile() else {
        return;
    };
    let plan = program.document_plan();
    let mut vm = Vm::new(
        &plan,
        Value::string(bounded),
        VmLimits {
            steps: 50_000,
            output_bytes: 64 * 1024,
            regex_pattern_bytes: 512,
            regex_input_bytes: 512,
            regex_compiled_bytes: 256 * 1024,
            ..VmLimits::default()
        },
    );
    for _ in 0..64 {
        match vm.next_result() {
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => break,
        }
    }
});
