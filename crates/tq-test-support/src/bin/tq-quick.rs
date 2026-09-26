//! Bound an entire post-compilation benchmark stage, including preparation.

use std::{
    env, io,
    process::{Command, ExitCode},
    time::Duration,
};
use tq_test_support::benchmark::quick::{QUICK_WORK_BUDGET_SECONDS, supervise_command};

fn run() -> io::Result<i32> {
    let mut args = env::args_os().skip(1);
    let mut seconds = QUICK_WORK_BUDGET_SECONDS;
    let mut first = args.next();
    if first.as_deref() == Some(std::ffi::OsStr::new("--budget-seconds")) {
        let value = args.next().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "--budget-seconds requires an integer from 1 to 50",
            )
        })?;
        seconds = value
            .to_str()
            .and_then(|value| value.parse().ok())
            .filter(|seconds| (1..=QUICK_WORK_BUDGET_SECONDS).contains(seconds))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "--budget-seconds requires an integer from 1 to 50",
                )
            })?;
        first = args.next();
    }
    if first.as_deref() != Some(std::ffi::OsStr::new("--")) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: tq-quick [--budget-seconds 1..50] -- COMMAND [ARGS...]",
        ));
    }
    let executable = args.next().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "tq-quick requires COMMAND after --",
        )
    })?;
    let mut command = Command::new(executable);
    command.args(args);
    supervise_command(&mut command, Duration::from_secs(seconds))
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
        Err(error) => {
            eprintln!("tq-quick: {error}");
            ExitCode::from(2)
        }
    }
}
