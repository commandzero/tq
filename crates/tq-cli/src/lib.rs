//! Library boundary for tq command parsing and execution.

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
    /// Runtime or input failure.
    Runtime = 5,
    /// CLI usage failure.
    Usage = 2,
    /// Query compilation failure.
    Compile = 3,
}

impl ExitStatus {
    /// Numeric status code.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }
}
