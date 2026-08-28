//! Command parsing and execution boundary for `tq`.

mod args;
mod runner;

pub use args::{
    CapabilityPolicy, CliError, ColorMode, Command, ExecutionOverride, ExplainFormat,
    ExternalArgument, ExternalArgumentKind, FilterSource, PositionalArgumentKind, ResourceLimits,
    RunOptions, generated_help, parse_args, parse_args_with_policy,
};
pub use runner::{RunError, run, run_with_io};
pub use tq_formats::JsonIndent;

/// Stable process exit categories. Exact jq-aligned status selection is
/// performed by the command runner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
    Runtime,
    /// Configured resource limit failure.
    Resource,
    /// Recognized but intentionally unsupported capability.
    Unsupported,
    /// Execution interrupted by the user.
    Interrupted,
}

impl ExitStatus {
    /// Numeric status code.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::FalseOrNull => 1,
            Self::Usage | Self::Unsupported => 2,
            Self::Compile => 3,
            Self::NoResult => 4,
            Self::Input | Self::Runtime | Self::Resource => 5,
            Self::Interrupted => 130,
        }
    }
}
