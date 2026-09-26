//! Correctness-gated process benchmark support.

mod correctness;
mod environment;
mod manifest;
mod markdown;
mod measure;
#[cfg(any(target_os = "macos", target_os = "linux"))]
mod native_process;
mod probe;
pub mod quick;
mod report;
mod runner;
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub mod worker;

pub use probe::{AllocationProbeError, run_allocation_probe};

pub use correctness::{
    CorrectnessDecision, CorrectnessObservation, CorrectnessPayload, SemanticDigest,
    correctness_gate, semantic_digest,
};
pub(crate) use correctness::{SemanticDigester, canonical_number};
pub use environment::{EnvironmentManifest, collect_environment};
pub use manifest::{
    BenchmarkAdapter, BenchmarkCase, BenchmarkCatalog, BenchmarkCatalogError, BenchmarkLimits,
    BenchmarkSampling, BenchmarkTool, ComparisonFamily, DatasetFamily, DatasetSelector,
    DatasetTier, ExecutionClass, InputFormat, OutputContract, OutputContractKind,
    load_benchmark_catalog,
};
pub use markdown::{
    MarkdownRenderError, RESULTS_END_MARKER, RESULTS_START_MARKER, render_markdown_campaigns,
    render_markdown_pages, replace_results_region, workload_filename,
};
pub use measure::{
    BenchmarkInvocation, MeasureError, MeasuredOutcome, MeasuredStatus, RssPreflight,
    RssProvenance, collector_source_sha256, measure_process, measure_process_uninstrumented,
    measure_process_worker, measure_process_worker_uninstrumented, preflight_rss,
};
pub use report::{
    BenchmarkCampaignReport, BenchmarkCorpusIdentity, BenchmarkFinalStatus, BenchmarkOutcome,
    BenchmarkRow, BenchmarkSample, CampaignExecution, Comparability, LaunchIsolationEvidence,
    MeasurementProtocol, MetricSummary, RegressionGate, RegressionThresholds, RowSummary,
    SoftObjectiveStatus, SoftPerformanceObjective, WorkerIdentity, compare_reports,
    evaluate_regression, populate_reference_ratios, summarize_samples,
};
pub use runner::{
    BenchmarkRunnerError, is_correctness_output_limit, is_correctness_resource_limit,
    normalize_correctness_run, run_correctness_limit_probe, run_gated_row, unsupported_row,
};
