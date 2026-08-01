//! Cross-tool compatibility harness primitives.

mod discovery;
mod normalization;
mod process;

pub use discovery::{ExecutableConfig, ToolDiscoveryError, ToolIdentity, ToolKind, discover_tool};
pub use normalization::{
    ErrorClass, NormalizationError, NormalizationNote, NormalizedObservation, classify_process,
    normalize_jq, normalize_raw, normalize_toon_sequence, normalize_yq,
};
pub use process::{Invocation, ProcessError, ProcessOutcome, ProcessStatus, run_process};
