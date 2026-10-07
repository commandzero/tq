//! Subprocess host for explicit compatibility checks of the real embedded API.

use std::{
    io::{self, Write},
    process::ExitCode,
};

use tq_cli::{
    CapabilityPolicy, Command, ExitStatus, RunError, parse_args_with_policy, run_with_io,
};
use tq_test_support::compatibility::ExecutionMode;

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let first = args.next();
    if first.as_deref() == Some(std::ffi::OsStr::new("--version")) {
        println!(
            "tq-compat-embedded {} ({}-{}; embedded API v1)",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::ARCH,
            std::env::consts::OS
        );
        return ExitCode::SUCCESS;
    }
    let mode = args
        .next()
        .and_then(|value| value.into_string().ok())
        .and_then(|value| {
            serde_json::from_value::<ExecutionMode>(serde_json::Value::String(value)).ok()
        });
    let separator = args.next();
    if first.as_deref() != Some(std::ffi::OsStr::new("--execution-mode"))
        || separator.as_deref() != Some(std::ffi::OsStr::new("--"))
    {
        eprintln!("tq-compat-embedded: expected --execution-mode MODE -- QUERY-ARGS");
        return ExitCode::from(ExitStatus::Usage.code());
    }
    let policy = match mode {
        Some(ExecutionMode::EmbeddedDenyEnvironment) => CapabilityPolicy {
            environment: false,
            ..CapabilityPolicy::default()
        },
        Some(ExecutionMode::EmbeddedDenyPlatform) => CapabilityPolicy {
            platform: false,
            ..CapabilityPolicy::default()
        },
        Some(ExecutionMode::Process) | None => {
            eprintln!("tq-compat-embedded: an explicit embedded denial mode is required");
            return ExitCode::from(ExitStatus::Usage.code());
        }
    };
    let mut command = match parse_args_with_policy(args, policy) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("tq: {error}");
            return ExitCode::from(ExitStatus::Usage.code());
        }
    };
    let Command::Run(options) = &mut command else {
        eprintln!("tq-compat-embedded: only query execution is supported");
        return ExitCode::from(ExitStatus::Usage.code());
    };
    // Admit the other authority so a denial proves the selected policy, not
    // the parser's default-disabled ambient options.
    options.allow_environment = policy.environment;
    options.allow_platform = policy.platform;
    let mut stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();
    let status = match run_with_io(command, &mut stdin, &mut stdout, &mut stderr) {
        Ok(status) => status,
        Err(error) => {
            if !matches!(error, RunError::ReportedRuntime(_)) {
                let _ = writeln!(stderr, "tq: {error}");
            }
            error.status()
        }
    };
    ExitCode::from(status.code())
}
