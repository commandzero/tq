#![no_main]

use libfuzzer_sys::fuzz_target;
use tq_core::Bytecode;

fuzz_target!(|data: &[u8]| {
    let words = data
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect::<Vec<_>>();
    if let Ok(bytecode) = Bytecode::decode(&words) {
        assert!(bytecode.validate().is_ok());
        let _ = bytecode.disassemble();
        let _ = bytecode.encode_kernel();
    }
});
