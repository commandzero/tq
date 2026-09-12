//! Cross-tool compatibility harness primitives.

mod baseline;
mod case;
mod discovery;
mod manual;
mod manual_pin;
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
    InvocationMode, ToolAdapters, TqContract, load_catalog,
};
pub use discovery::{ExecutableConfig, ToolDiscoveryError, ToolIdentity, ToolKind, discover_tool};
pub use manual::{
    DisparityEvidence, DisparityValidationError, GapBaselineVerdict, GapInventorySummary,
    ManualGapEntry, ManualGapInventory, ManualInventoryError, ManualVerdictCounts,
    ReviewedDisparity, ReviewedObservation, StrictCampaignError, apply_reviewed_disparities,
    case_fingerprint, manual_case_ids, manual_verdict_counts, read_gap_inventory,
    read_manual_ledger, read_manual_review_case_ids, validate_completion_manual_report,
    validate_gap_inventory, validate_strict_manual_report,
};
pub use manual_pin::{
    ManualBaselineCase, ManualBuildPin, ManualPinError, ManualReferencePin, ManualSourcePin,
    manual_host_target, validate_manual_reference, validate_manual_source_checkout,
    validate_manual_source_inventory, validate_pinned_manual_coverage,
};
pub use normalization::{
    ErrorClass, NormalizationError, NormalizationNote, NormalizedObservation, classify_process,
    normalize_jq, normalize_raw, normalize_toon_document, normalize_toon_sequence, normalize_yq,
    toon_values_match,
};
pub use process::{
    Invocation, ProcessError, ProcessOutcome, ProcessStatus, run_process,
    run_process_with_environment,
};
pub use report::{
    CapabilityCounts, CapabilityDisposition, CaseReport, CompatibilityReport, CoverageCount,
    FinalStatus, ObservationState, REPORT_SCHEMA_VERSION, SemanticDiff, ToolObservation,
    encode_hex, tq_contract_matches,
};
pub use runner::{
    CampaignProfile, RunnerError, compare_manual, compare_manual_with_disparities, run_campaign,
    summarize_manual_comparison,
};
