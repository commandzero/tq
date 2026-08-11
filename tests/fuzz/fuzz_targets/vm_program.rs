#![no_main]

use libfuzzer_sys::fuzz_target;
use tq_core::{Bytecode, Value, Vm, VmLimits};

fuzz_target!(|data: &[u8]| {
    let words = data
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect::<Vec<_>>();
    if let Ok(bytecode) = Bytecode::decode(&words) {
        let mut vm = Vm::from_validated_bytecode(
            bytecode,
            Value::Null,
            VmLimits {
                steps: 1024,
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
    }
});
