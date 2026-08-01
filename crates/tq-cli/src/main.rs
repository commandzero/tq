//! Command-line entry point for `tq`.

fn main() -> std::process::ExitCode {
    let status = match tq_cli::parse_args(std::env::args_os().skip(1)) {
        Ok(command) => tq_cli::run(command),
        Err(error) => {
            eprintln!("tq: {error}");
            match error {
                tq_cli::CliError::Unsupported(_) => tq_cli::ExitStatus::Unsupported,
                _ => tq_cli::ExitStatus::Usage,
            }
        }
    };
    std::process::ExitCode::from(status.code())
}
