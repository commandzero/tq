//! Correctness-gated process benchmark support.

mod correctness;
mod environment;
mod manifest;
mod measure;
mod report;
mod runner;

pub use correctness::{CorrectnessDecision, correctness_gate};
pub use environment::{EnvironmentManifest, collect_environment};
pub use manifest::{
    BenchmarkAdapter, BenchmarkCase, BenchmarkCatalog, BenchmarkCatalogError, BenchmarkLimits,
    BenchmarkSampling, BenchmarkTool, ComparisonFamily, DatasetFamily, DatasetSelector,
    DatasetTier, ExecutionClass, InputFormat, OutputContract, OutputContractKind,
    load_benchmark_catalog,
};
pub use measure::{
    BenchmarkInvocation, MeasureError, MeasuredOutcome, MeasuredStatus, measure_process,
};
pub use report::{
    BenchmarkCampaignReport, BenchmarkCorpusIdentity, BenchmarkFinalStatus, BenchmarkOutcome,
    BenchmarkRow, BenchmarkSample, Comparability, MetricSummary, RegressionGate,
    RegressionThresholds, RowSummary, compare_reports, evaluate_regression,
    populate_reference_ratios, summarize_samples,
};
pub use runner::{BenchmarkRunnerError, normalize_correctness_run, run_gated_row, unsupported_row};
