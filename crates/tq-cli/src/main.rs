//! Command-line entry point for `tq`.

fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(tq_cli::ExitStatus::Success.code())
}
