#![no_main]

use libfuzzer_sys::fuzz_target;
use tq_core::{ResolveOptions, Value, Vm, VmLimits, analyze, parse, resolve};

fuzz_target!(|data: &[u8]| {
    if data.len() > 16 * 1024 {
        return;
    }

    let depth = data.first().copied().map_or(0, usize::from).min(64);
    let mut input = Value::Null;
    for _ in 0..depth {
        input = Value::array([input]);
    }

    let source = std::str::from_utf8(data.get(1..).unwrap_or_default()).unwrap_or(".");
    let query = if data.get(1).is_some_and(|byte| byte & 1 == 0) {
        "..".to_owned()
    } else {
        format!("\"value=\\({source})\"")
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
        input,
        VmLimits {
            steps: 2048,
            value_stack: 64,
            call_stack: 64,
            path_stack: 64,
            fork_stack: 64,
            output_bytes: 4096,
            ..VmLimits::default()
        },
    );
    for _ in 0..128 {
        match vm.next_result() {
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => break,
        }
    }
});
