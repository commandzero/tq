#![no_main]

use libfuzzer_sys::fuzz_target;
use tq_core::{ResolveOptions, Value, Vm, VmLimits, analyze, parse, resolve};

fuzz_target!(|data: &[u8]| {
    let depth = data.first().copied().map_or(0, u32::from).min(48);
    let query = match data.get(1).copied().unwrap_or_default() % 5 {
        0 => format!(
            "def step(f; $n): if $n == 0 then . else f | step(f; $n - 1) end; step(. + 1; {depth})"
        ),
        1 => format!(
            "def step(f; $n): if $n == 0 then . else f | step(f; $n - 1) end; [0,1,2,3] | map(step(. + 1; {depth}))"
        ),
        2 => format!(
            "def step(f; $n): if $n == 0 then . else f | step(f; $n - 1) end; {{a:0,b:1}} | map_values(step(. + 1; {depth}))"
        ),
        3 => format!(
            "def predicate: . < {depth}; [0,1,2,3] | [.[] | select(predicate)]"
        ),
        _ => "def key: .; [3,1,2] | sort_by(key)".to_owned(),
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
        Value::Null,
        VmLimits {
            steps: 4096,
            value_stack: 64,
            call_stack: 64,
            path_stack: 64,
            fork_stack: 64,
            output_bytes: 4096,
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
