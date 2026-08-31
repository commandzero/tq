#![no_main]

use libfuzzer_sys::fuzz_target;
use tq_core::{ResolveOptions, Value, Vm, VmLimits, analyze, parse, resolve};

fuzz_target!(|data: &[u8]| {
    if data.len() > 16 * 1024 {
        return;
    }

    let selector = data.first().copied().map_or(0, usize::from);
    let depth = selector.min(64);
    let text = String::from_utf8_lossy(data.get(2..).unwrap_or_default());
    let mut input = match selector % 3 {
        0 => Value::string(text.as_ref()),
        1 => Value::array([Value::string(text.as_ref()), Value::Null, Value::Bool(true)]),
        _ => Value::Null,
    };
    for _ in 0..depth {
        input = Value::array([input]);
    }

    let source = std::str::from_utf8(data.get(1..).unwrap_or_default()).unwrap_or(".");
    let formats = [
        "@text", "@json", "@html", "@uri", "@csv", "@tsv", "@sh", "@base64",
        "@base64d",
    ];
    let format = formats[selector % formats.len()];
    let query = match data.get(1).copied().unwrap_or_default() % 9 {
        0 => "..".to_owned(),
        1 => format!("\"value=\\({source})\""),
        2 => format.to_owned(),
        3 => format!("{format} \"value=\\({source})\""),
        4 => "recurse(if type == \"array\" then .[] else empty end)".to_owned(),
        5 => format!("recurse(. + 1; . < {})", depth.min(16)),
        6 => "walk(if type == \"number\" then . + 1 else . end)".to_owned(),
        7 => "label $outer | 1, (label $inner | 2, break $inner, 3), break $outer, 4"
            .to_owned(),
        _ => "label $out | try (1, break $out, 2) catch ., 3".to_owned(),
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
