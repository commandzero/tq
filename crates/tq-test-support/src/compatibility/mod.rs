//! Cross-tool compatibility harness primitives.

mod baseline;
mod case;
mod discovery;
mod normalization;
mod process;
mod report;
mod runner;

pub use baseline::{
    BASELINE_SCHEMA_VERSION, BaselineChange, BaselineError, BaselineObservation,
    CompatibilityBaseline, accept_reviewed_candidate, diff_baselines, read_baseline,
    write_baseline_atomic,
};
pub use case::{
    BaselinePolicy, CaseAdapter, CaseClassification, CaseFixture, CaseStatus, CatalogError,
    CompatibilityCase, CompatibilityCatalog, ContractKind, ExpectedContract, FixtureFormat,
    InvocationMode, ToolAdapters, load_catalog,
};
pub use discovery::{ExecutableConfig, ToolDiscoveryError, ToolIdentity, ToolKind, discover_tool};
pub use normalization::{
    ErrorClass, NormalizationError, NormalizationNote, NormalizedObservation, classify_process,
    normalize_jq, normalize_raw, normalize_toon_sequence, normalize_yq,
};
pub use process::{Invocation, ProcessError, ProcessOutcome, ProcessStatus, run_process};
pub use report::{
    CapabilityCounts, CapabilityDisposition, CaseReport, CompatibilityReport, CoverageCount,
    FinalStatus, ObservationState, REPORT_SCHEMA_VERSION, SemanticDiff, ToolObservation,
    encode_hex,
};
pub use runner::{CampaignProfile, RunnerError, run_campaign};
