//! Dedicated native benchmark measurement worker.

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn main() {
    if let Err(error) = tq_test_support::benchmark::worker::run() {
        eprintln!("tq-bench-worker: {error}");
        std::process::exit(1);
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn main() {
    eprintln!("tq-bench-worker: native worker unsupported on this platform");
    std::process::exit(1);
}
