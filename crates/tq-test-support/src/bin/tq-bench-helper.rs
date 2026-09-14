//! Deterministic subprocess used only by benchmark-harness self-tests.

use std::{env, hint::black_box, io::Write as _, process::ExitCode, time::Duration};

fn main() -> ExitCode {
    let mut arguments = env::args().skip(1);
    let Some(mode) = arguments.next() else {
        return ExitCode::from(64);
    };
    match mode.as_str() {
        "sleep" => {
            let millis = number(arguments.next());
            std::thread::sleep(Duration::from_millis(millis));
        }
        "output" => {
            let bytes = usize::try_from(number(arguments.next())).unwrap_or(usize::MAX);
            let chunk = vec![b'x'; bytes.min(64 * 1024)];
            let mut remaining = bytes;
            let mut stdout = std::io::stdout().lock();
            while remaining > 0 {
                let length = remaining.min(chunk.len());
                if stdout.write_all(&chunk[..length]).is_err() {
                    return ExitCode::FAILURE;
                }
                remaining -= length;
            }
        }
        "streams" => {
            let stdout_bytes = usize::try_from(number(arguments.next())).unwrap_or(usize::MAX);
            let stderr_bytes = usize::try_from(number(arguments.next())).unwrap_or(usize::MAX);
            write_bytes(std::io::stdout(), stdout_bytes);
            write_bytes(std::io::stderr(), stderr_bytes);
        }
        "first" => {
            println!("first");
            let _ = std::io::stdout().flush();
            std::thread::sleep(Duration::from_millis(number(arguments.next())));
            println!("last");
        }
        "memory" => {
            let bytes = usize::try_from(number(arguments.next())).unwrap_or(usize::MAX);
            let mut allocation = vec![0_u8; bytes];
            for byte in allocation.iter_mut().step_by(4096) {
                *byte = 1;
            }
            black_box(&allocation);
            std::thread::sleep(Duration::from_millis(200));
        }
        _ => return ExitCode::from(64),
    }
    ExitCode::SUCCESS
}

fn write_bytes(mut output: impl std::io::Write, bytes: usize) {
    let chunk = vec![b'x'; bytes.min(64 * 1024)];
    let mut remaining = bytes;
    while remaining > 0 {
        let length = remaining.min(chunk.len());
        if output.write_all(&chunk[..length]).is_err() {
            return;
        }
        remaining -= length;
    }
}

fn number(value: Option<String>) -> u64 {
    value.and_then(|item| item.parse().ok()).unwrap_or_default()
}
