//! Command parsing and execution boundary for `tq`.

mod args;
mod runner;

pub use args::{
    CliError, Command, ExplainFormat, ExternalArgument, ExternalArgumentKind, FilterSource,
    RunOptions, parse_args,
};
pub use runner::{RunError, run, run_with_io};

/// Stable process exit categories. Exact jq-aligned status selection is
/// performed by the command runner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ExitStatus {
    /// Successful execution.
    Success = 0,
    /// False/null last result under `--exit-status`.
    FalseOrNull = 1,
    /// No result under `--exit-status`.
    NoResult = 4,
    /// CLI usage failure.
    Usage = 2,
    /// Query compilation failure.
    Compile = 3,
    /// Structured or raw input failure.
    Input = 5,
    /// Query runtime failure.
    Runtime = 6,
    /// Configured resource limit failure.
    Resource = 7,
    /// Recognized but intentionally unsupported capability.
    Unsupported = 8,
}

impl ExitStatus {
    /// Numeric status code.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }
}
