//! Benchmark statistics, failure rows, comparability, and regression gates.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{
    BenchmarkLimits, ComparisonFamily, EnvironmentManifest, ExecutionClass, InputFormat,
    MeasuredOutcome, RssProvenance,
};
use crate::{compatibility::ToolIdentity, corpus::ArtifactIdentity};

/// Versioned performance campaign report.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BenchmarkCampaignReport {
    /// Schema version.
    pub schema_version: u32,
    /// Unique local campaign ID.
    pub campaign_id: String,
    /// smoke, standard, or large.
    pub profile: String,
    /// Host/compiler identity.
    pub environment: EnvironmentManifest,
    /// Exact natural corpus artifacts.
    pub corpus: Vec<BenchmarkCorpusIdentity>,
    /// Exact executable identities.
    pub tools: Vec<ToolIdentity>,
    /// Every applicable and inapplicable benchmark row.
    pub cases: Vec<BenchmarkRow>,
    /// Comparison status against another report, when requested.
    pub comparability: Comparability,
    /// tq self-regression result.
    pub regression_gate: RegressionGate,
    /// Overall status.
    pub final_status: BenchmarkFinalStatus,
}

/// Exact benchmark input identity and logical shape.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BenchmarkCorpusIdentity {
    /// smoke, refreshed, or frozen origin.
    pub origin: String,
    /// Source snapshot ID.
    pub source_id: String,
    /// Natural tier.
    pub tier: String,
    /// Input format.
    pub format: InputFormat,
    /// Exact artifact.
    pub artifact: ArtifactIdentity,
    /// Logical record count.
    pub logical_records: u64,
    /// Snapshot/campaign manifest digest.
    pub manifest_sha256: String,
}

/// One tool/workload/dataset/format row.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BenchmarkRow {
    /// Workload ID.
    pub case_id: String,
    /// Adapter ID.
    pub adapter_id: String,
    /// Dataset source ID.
    pub source_id: String,
    /// Natural tier.
    pub tier: String,
    /// Input format.
    pub input_format: InputFormat,
    /// Blocking/streaming classification.
    pub execution_class: ExecutionClass,
    /// Same-format/native/parser report groups.
    pub comparison_families: Vec<ComparisonFamily>,
    /// Secret-safe exact command.
    pub command: Vec<String>,
    /// Correctness/failure disposition.
    pub outcome: BenchmarkOutcome,
    /// Warmup count performed.
    pub warmups: usize,
    /// Required measured count.
    pub requested_samples: usize,
    /// Per-invocation timeout.
    pub timeout_seconds: u64,
    /// Output and RSS limits.
    pub limits: BenchmarkLimits,
    /// Actual measurements; empty for correctness failures.
    pub samples: Vec<BenchmarkSample>,
    /// Separate sampled RSS-limit repetitions, never pooled into primary timing summaries.
    #[serde(default)]
    pub instrumented_samples: Vec<BenchmarkSample>,
    /// Aggregate metrics for valid samples.
    pub summary: Option<RowSummary>,
    /// Ratios to named reference rows; informational only.
    pub reference_ratios: BTreeMap<String, f64>,
    /// Peak-RSS ratios to named reference rows; informational only.
    #[serde(default)]
    pub reference_peak_rss_ratios: BTreeMap<String, f64>,
    /// Soft jq-relative targets for tq JSON rows.
    #[serde(default)]
    pub soft_performance_objective: Option<SoftPerformanceObjective>,
    /// Bounded correctness-gate detail for an incorrect or unnormalized row.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<String>,
}

/// Informational jq-relative time and memory assessment.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SoftPerformanceObjective {
    /// tq median wall time divided by jq median wall time.
    pub wall_time_ratio: Option<f64>,
    /// tq maximum observed peak RSS divided by jq's value.
    pub peak_rss_ratio: Option<f64>,
    /// Assessment against the 2.0 wall-time target.
    pub wall_time: SoftObjectiveStatus,
    /// Assessment against the 1.5 peak-RSS target.
    pub peak_rss: SoftObjectiveStatus,
}

/// Result of one non-blocking comparative target.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SoftObjectiveStatus {
    /// The comparable ratio is within the target.
    Met,
    /// The comparable ratio exceeds the target.
    Missed,
    /// One of the required comparable metrics is absent.
    NotComparable,
}

/// First-class row outcome.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BenchmarkOutcome {
    /// Correctness passed and timing completed.
    Timed,
    /// Correctness gate disagreed.
    Incorrect,
    /// Adapter or executable unavailable.
    Unsupported,
    /// Process timed out.
    Timeout,
    /// OOM or signal termination.
    OomOrSignal,
    /// Configured output/RSS/engine limit exceeded.
    ResourceLimit,
}

/// Collection protocol used to determine whether measurements are comparable.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MeasurementProtocol {
    /// Clock boundaries and observation method.
    pub timing_method: String,
    /// How input bytes reach the tool.
    pub input_delivery: String,
    /// Accounting scope, including any documented inherited usage.
    pub rss_scope: String,
    /// Requested exit-observation polling interval, not an accuracy guarantee.
    pub exit_poll_interval_micros: u64,
    /// Optional process-group RSS enforcement sampling interval.
    pub rss_poll_interval_micros: Option<u64>,
    /// Legacy wire field containing observed known-duration control excess;
    /// the field name is retained for compatibility and does not claim timer
    /// error or universal accuracy.
    pub validated_accuracy_micros: Option<u64>,
    /// Identity of the worker and collector that launched the target.
    #[serde(default)]
    pub worker: Option<WorkerIdentity>,
    /// Evidence that the worker's launch RSS floor was independently checked.
    #[serde(default)]
    pub isolation_evidence: Option<LaunchIsolationEvidence>,
}

impl MeasurementProtocol {
    fn is_valid(&self) -> bool {
        !self.timing_method.trim().is_empty()
            && !self.input_delivery.trim().is_empty()
            && !self.rss_scope.trim().is_empty()
            && self.exit_poll_interval_micros > 0
            && self.rss_poll_interval_micros != Some(0)
            && self.validated_accuracy_micros != Some(0)
            && self.worker.as_ref().is_none_or(WorkerIdentity::is_valid)
            && self
                .isolation_evidence
                .as_ref()
                .is_none_or(LaunchIsolationEvidence::is_valid)
    }

    fn is_calibrated(&self) -> bool {
        self.validated_accuracy_micros
            .is_some_and(|accuracy| accuracy > 0)
    }

    fn is_valid_for_comparison(&self) -> bool {
        self.is_valid()
            && self.is_calibrated()
            && self.worker.as_ref().is_some_and(WorkerIdentity::is_valid)
            && self
                .isolation_evidence
                .as_ref()
                .is_some_and(LaunchIsolationEvidence::is_valid)
            && self.has_explicit_lifetime_scope()
    }

    fn has_explicit_lifetime_scope(&self) -> bool {
        let scope = self.rss_scope.to_ascii_lowercase().replace(['-', '_'], " ");
        scope.contains("pre exec") && scope.contains("waited") && scope.contains("descendant")
    }
}

/// Immutable identity of the executable that owns target launch and waiting.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkerIdentity {
    /// SHA-256 of the worker executable bytes.
    pub executable_sha256: String,
    /// Versioned protocol used to transfer a prepared invocation.
    pub launch_protocol: String,
    /// SHA-256 of the compiled native collector sources.
    pub collector_source_sha256: String,
}

impl WorkerIdentity {
    fn is_valid(&self) -> bool {
        !self.executable_sha256.trim().is_empty()
            && !self.launch_protocol.trim().is_empty()
            && !self.collector_source_sha256.trim().is_empty()
    }
}

/// Retained proof that worker launch memory does not determine target RSS.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LaunchIsolationEvidence {
    /// SHA-256 of the native validation summary containing the controls.
    pub summary_sha256: String,
    /// Matched-control peak RSS in bytes.
    pub control_peak_rss_bytes: u64,
    /// Maximum observed parent-allocation delta in bytes.
    pub max_parent_delta_bytes: u64,
    /// Declared page-aware tolerance in bytes.
    pub tolerance_bytes: u64,
}

impl LaunchIsolationEvidence {
    fn is_valid(&self) -> bool {
        !self.summary_sha256.trim().is_empty()
            && self.control_peak_rss_bytes > 0
            && self.tolerance_bytes > 0
            && self.max_parent_delta_bytes <= self.tolerance_bytes
    }
}

/// One measured fresh-process invocation.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BenchmarkSample {
    /// Explicit protocol; absent only for historical reports.
    #[serde(default)]
    pub measurement_protocol: Option<MeasurementProtocol>,
    /// Wall duration.
    pub wall_time_micros: u128,
    /// User CPU duration.
    pub user_cpu_micros: Option<u128>,
    /// System CPU duration.
    pub system_cpu_micros: Option<u128>,
    /// Peak resident bytes.
    pub peak_rss_bytes: Option<u64>,
    /// Explicit source of the authoritative peak RSS value. `None` is only
    /// valid for legacy reports that predate the provenance field.
    #[serde(default)]
    pub rss_provenance: Option<RssProvenance>,
    /// Peak resident bytes observed separately by process-group sampling.
    #[serde(default)]
    pub process_group_peak_rss_bytes: Option<u64>,
    /// Time to first stdout byte.
    pub first_result_micros: Option<u128>,
    /// Exact output bytes.
    pub output_bytes: u64,
}

impl From<&MeasuredOutcome> for BenchmarkSample {
    fn from(value: &MeasuredOutcome) -> Self {
        Self {
            measurement_protocol: Some(value.measurement_protocol.clone()),
            wall_time_micros: value.wall_time_micros,
            user_cpu_micros: value.user_cpu_micros,
            system_cpu_micros: value.system_cpu_micros,
            peak_rss_bytes: value.peak_rss_bytes,
            rss_provenance: Some(value.rss_provenance),
            process_group_peak_rss_bytes: value.process_group_peak_rss_bytes,
            first_result_micros: value.first_result_micros,
            output_bytes: value.output_bytes,
        }
    }
}

impl BenchmarkCampaignReport {
    /// Validates the RSS contract for a newly collected campaign report.
    ///
    /// Historical reports may omit provenance and remain readable, but a new
    /// report must never publish a measured sample without positive RSS and an
    /// explicit authoritative source.
    ///
    /// # Errors
    ///
    /// Returns an error when a measured sample lacks positive authoritative RSS
    /// or explicit provenance, when a timed row has no samples, or when a
    /// native RSS-limited timed row lacks valid separate enforcement evidence.
    pub fn validate_authoritative_rss(&self) -> Result<(), String> {
        for row in &self.cases {
            for (index, sample) in row
                .samples
                .iter()
                .chain(&row.instrumented_samples)
                .enumerate()
            {
                if sample.peak_rss_bytes.is_none_or(|bytes| bytes == 0) {
                    return Err(format!(
                        "{} {} sample {} has no positive authoritative RSS",
                        row.case_id, row.adapter_id, index
                    ));
                }
                if sample.rss_provenance.is_none() {
                    return Err(format!(
                        "{} {} sample {} has no RSS provenance",
                        row.case_id, row.adapter_id, index
                    ));
                }
                if matches!(
                    sample.rss_provenance,
                    Some(RssProvenance::DarwinWait4 | RssProvenance::LinuxWait4)
                ) && (sample
                    .measurement_protocol
                    .as_ref()
                    .is_none_or(|protocol| !protocol.is_valid())
                    || sample.user_cpu_micros.is_none()
                    || sample.system_cpu_micros.is_none())
                {
                    return Err(format!(
                        "{} {} sample {} lacks native collection evidence",
                        row.case_id, row.adapter_id, index
                    ));
                }
            }
            if !row.samples.is_empty() && row_rss_source(row).is_none() {
                return Err(format!(
                    "{} {} mixes collection methods",
                    row.case_id, row.adapter_id
                ));
            }
            if let Some(reason) = validate_instrumented_samples(row) {
                return Err(format!("{} {} {reason}", row.case_id, row.adapter_id));
            }
            if row.outcome == BenchmarkOutcome::Timed && row.samples.is_empty() {
                return Err(format!(
                    "{} {} is timed without a measured sample",
                    row.case_id, row.adapter_id
                ));
            }
        }
        Ok(())
    }

    /// Validates the stricter evidence contract required for stable publication.
    ///
    /// Reports without native samples remain publishable for historical pages.
    /// Native samples may still be retained as diagnostic JSON until calibration
    /// and launch-isolation evidence are attached.
    ///
    /// # Errors
    ///
    /// Returns an error when native samples lack calibrated timing, explicit
    /// lifetime scope, worker identity, or validated launch-isolation evidence.
    pub fn validate_for_publication(&self) -> Result<(), String> {
        let has_native_samples = self.cases.iter().any(|row| {
            row.samples
                .iter()
                .chain(&row.instrumented_samples)
                .any(|sample| sample.rss_provenance.is_some_and(is_native_rss_provenance))
        });
        if !has_native_samples {
            return Ok(());
        }

        for row in &self.cases {
            for (family, samples) in [
                ("primary", &row.samples),
                ("instrumented", &row.instrumented_samples),
            ] {
                for sample in samples {
                    if !sample.rss_provenance.is_some_and(is_native_rss_provenance) {
                        continue;
                    }
                    let Some(protocol) = sample.measurement_protocol.as_ref() else {
                        return Err(format!(
                            "{} {} {family} sample requires a measurement protocol",
                            row.case_id, row.adapter_id
                        ));
                    };
                    if !protocol.is_valid_for_comparison() {
                        return Err(format!(
                            "{} {} {family} sample requires positive observed control excess plus calibrated worker, lifetime-scope, and launch-isolation evidence",
                            row.case_id, row.adapter_id
                        ));
                    }
                }
            }
        }
        self.validate_authoritative_rss()?;
        Ok(())
    }
}

/// Aggregate row metrics without a composite score.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RowSummary {
    /// Wall timing distribution.
    pub wall_time_micros: MetricSummary,
    /// Time-to-first-result distribution when available.
    pub first_result_micros: Option<MetricSummary>,
    /// Maximum observed RSS.
    pub peak_rss_bytes: Option<u64>,
    /// Median user CPU.
    pub user_cpu_micros: Option<f64>,
    /// Median system CPU.
    pub system_cpu_micros: Option<f64>,
    /// Stable output byte count, or max if samples vary.
    pub output_bytes: u64,
    /// Logical records per second using median wall time.
    pub logical_records_per_second: f64,
    /// Physical input MiB per second using median wall time.
    pub physical_mib_per_second: f64,
}

/// Median and robust dispersion.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MetricSummary {
    /// Number of observations.
    pub samples: usize,
    /// Median.
    pub median: f64,
    /// Median absolute deviation.
    pub median_absolute_deviation: f64,
    /// 95th percentile using nearest rank.
    pub p95: f64,
    /// Minimum.
    pub minimum: f64,
    /// Maximum.
    pub maximum: f64,
}

/// Report comparability result.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Comparability {
    /// Whether direct comparisons are safe.
    pub comparable: bool,
    /// Explicit mismatch reasons.
    pub reasons: Vec<String>,
}

impl Default for Comparability {
    fn default() -> Self {
        Self {
            comparable: true,
            reasons: Vec::new(),
        }
    }
}

/// Configurable tq-only regression policy.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RegressionThresholds {
    /// Allowed median wall-time increase percent.
    pub wall_time_percent: f64,
    /// Allowed peak-RSS increase percent.
    pub peak_rss_percent: f64,
    /// Minimum measured samples for a gate.
    pub minimum_samples: usize,
}

/// Regression evaluation outcome.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RegressionGate {
    /// Whether a comparable accepted baseline was supplied.
    pub evaluated: bool,
    /// Thresholds used.
    pub thresholds: Option<RegressionThresholds>,
    /// Row-specific failures.
    pub failures: Vec<String>,
    /// Independent metric increases above the issue #30 disclosure threshold.
    #[serde(default)]
    pub disclosures: Vec<String>,
    /// Rows without sufficient comparable evidence; never counted as passes.
    #[serde(default)]
    pub unavailable: Vec<String>,
}

/// Campaign final status.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BenchmarkFinalStatus {
    /// All eligible rows timed and no tq regression occurred.
    Passed,
    /// Failures were preserved but no tq regression gate failed.
    ObservedFailures,
    /// tq exceeded its own comparable baseline threshold.
    Regression,
}

impl BenchmarkCampaignReport {
    /// Renders separate same-format, native-format, and parser-specific views.
    #[must_use]
    pub fn render_human(&self) -> String {
        use std::fmt::Write as _;
        let mut output = format!(
            "benchmark {}: {:?}\nmachine: {}\n",
            self.profile, self.final_status, self.environment.machine_identity
        );
        for (title, family) in [
            ("same-format", ComparisonFamily::SameFormat),
            ("native-format", ComparisonFamily::NativeFormat),
            ("parser-specific", ComparisonFamily::ParserSpecific),
        ] {
            writeln!(output, "\n{title}:").expect("write benchmark report");
            for row in self
                .cases
                .iter()
                .filter(|row| row.comparison_families.contains(&family))
            {
                let median = row.summary.as_ref().map_or_else(
                    || "-".to_owned(),
                    |summary| format!("{:.0} us", summary.wall_time_micros.median),
                );
                let objective =
                    row.soft_performance_objective
                        .as_ref()
                        .map_or_else(String::new, |objective| {
                            format!(
                                ", soft jq target: time {:?}, rss {:?}",
                                objective.wall_time, objective.peak_rss
                            )
                        });
                writeln!(
                    output,
                    "- {} {} {}: {:?}, {}{}",
                    row.case_id, row.adapter_id, row.source_id, row.outcome, median, objective
                )
                .expect("write benchmark report");
            }
        }
        for disclosure in &self.regression_gate.disclosures {
            writeln!(output, "disclosure: {disclosure}").expect("write regression disclosure");
        }
        for reason in &self.regression_gate.unavailable {
            writeln!(output, "not comparable: {reason}").expect("write unavailable comparison");
        }
        output
    }
}

/// Summarizes valid samples into independent metrics.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    reason = "reported statistical rates intentionally use floating-point summaries"
)]
pub fn summarize_samples(
    samples: &[BenchmarkSample],
    input_bytes: u64,
    logical_records: u64,
) -> Option<RowSummary> {
    if samples.is_empty() {
        return None;
    }
    if samples.iter().any(|sample| {
        sample.rss_provenance != samples[0].rss_provenance
            || sample.measurement_protocol != samples[0].measurement_protocol
    }) {
        return None;
    }
    let wall = metric(samples.iter().map(|sample| sample.wall_time_micros as f64))?;
    let first_values = samples
        .iter()
        .filter_map(|sample| sample.first_result_micros)
        .map(|value| value as f64)
        .collect::<Vec<_>>();
    let first_result_micros = (!first_values.is_empty())
        .then(|| metric(first_values.into_iter()))
        .flatten();
    let peak_rss_bytes = samples
        .iter()
        .filter_map(|sample| sample.peak_rss_bytes)
        .max();
    let user_cpu_micros =
        optional_median(samples.iter().filter_map(|sample| sample.user_cpu_micros));
    let system_cpu_micros =
        optional_median(samples.iter().filter_map(|sample| sample.system_cpu_micros));
    let output_bytes = samples
        .iter()
        .map(|sample| sample.output_bytes)
        .max()
        .unwrap_or(0);
    let seconds = wall.median / 1_000_000.0;
    Some(RowSummary {
        wall_time_micros: wall,
        first_result_micros,
        peak_rss_bytes,
        user_cpu_micros,
        system_cpu_micros,
        output_bytes,
        logical_records_per_second: logical_records as f64 / seconds,
        physical_mib_per_second: input_bytes as f64 / (1024.0 * 1024.0) / seconds,
    })
}

/// Compares machine, corpus, and tool identities explicitly.
#[must_use]
pub fn compare_reports(
    left: &BenchmarkCampaignReport,
    right: &BenchmarkCampaignReport,
) -> Comparability {
    report_comparability(left, right, false)
}

fn report_comparability(
    left: &BenchmarkCampaignReport,
    right: &BenchmarkCampaignReport,
    self_regression: bool,
) -> Comparability {
    let mut reasons = Vec::new();
    if left.profile != right.profile {
        reasons.push("campaign profile differs".to_owned());
    }
    if left.environment.machine_identity != right.environment.machine_identity {
        reasons.push("machine identity differs".to_owned());
    }
    if left.environment.os != right.environment.os {
        reasons.push("operating system differs".to_owned());
    }
    if !corpus_identities_match(&left.corpus, &right.corpus) {
        reasons.push("corpus identity differs".to_owned());
    }
    if !self_regression && left.tools != right.tools {
        reasons.push("tool identity differs".to_owned());
    }
    if self_regression {
        let build = |report: &BenchmarkCampaignReport| {
            report
                .tools
                .iter()
                .find(|tool| tool.tool == crate::compatibility::ToolKind::Tq)
                .map(|tool| (tool.build_features.clone(), tool.runtime_libraries.clone()))
        };
        if build(left) != build(right) {
            reasons.push("tq build configuration differs".to_owned());
        }
    }
    if left.environment.compiler_profile != right.environment.compiler_profile {
        reasons.push("compiler profile differs".to_owned());
    }
    if !measurement_contracts_match(left, right) {
        reasons.push("measurement protocol or RSS provenance differs".to_owned());
    }
    if [left, right]
        .iter()
        .any(|report| has_unvalidated_native_timing(report))
    {
        reasons.push("native timing controls are unvalidated".to_owned());
    }
    if [left, right]
        .iter()
        .any(|report| has_unverified_rss_provenance(report))
    {
        reasons.push("unverified RSS provenance is present".to_owned());
    }
    for report in [left, right] {
        for row in &report.cases {
            if let Some(reason) = validate_instrumented_samples(row) {
                reasons.push(format!(
                    "{} {} invalid instrumented evidence: {reason}",
                    row.case_id, row.adapter_id
                ));
            }
        }
    }
    Comparability {
        comparable: reasons.is_empty(),
        reasons,
    }
}

fn measurement_contracts_match(
    left: &BenchmarkCampaignReport,
    right: &BenchmarkCampaignReport,
) -> bool {
    left.cases.iter().all(|left_row| {
        right
            .cases
            .iter()
            .find(|right_row| same_row_identity(left_row, right_row))
            .is_none_or(|right_row| row_measurement_contracts_match(left_row, right_row))
    }) && right.cases.iter().all(|right_row| {
        left.cases
            .iter()
            .find(|left_row| same_row_identity(left_row, right_row))
            .is_none_or(|left_row| row_measurement_contracts_match(left_row, right_row))
    })
}

fn same_row_identity(left: &BenchmarkRow, right: &BenchmarkRow) -> bool {
    left.case_id == right.case_id
        && left.adapter_id == right.adapter_id
        && left.source_id == right.source_id
        && left.tier == right.tier
        && left.input_format == right.input_format
        && left.execution_class == right.execution_class
}

fn row_measurement_contracts_match(left: &BenchmarkRow, right: &BenchmarkRow) -> bool {
    let left_contracts = row_measurement_contracts(left);
    let right_contracts = row_measurement_contracts(right);
    left_contracts
        .iter()
        .all(|contract| right_contracts.contains(contract))
        && right_contracts
            .iter()
            .all(|contract| left_contracts.contains(contract))
}

fn row_measurement_contracts(
    row: &BenchmarkRow,
) -> Vec<(Option<RssProvenance>, Option<MeasurementProtocol>)> {
    row.samples
        .iter()
        .chain(row.instrumented_samples.iter())
        .map(|sample| (sample.rss_provenance, sample.measurement_protocol.clone()))
        .collect()
}

/// Adds independent wall-time ratios to matching named reference adapters.
///
/// Ratios are informational and are never combined into a winner score.
#[allow(
    clippy::too_many_lines,
    reason = "keep reference contract checks beside ratio construction"
)]
#[allow(
    clippy::cast_precision_loss,
    reason = "peak-memory reference ratios intentionally use floating-point reporting"
)]
pub fn populate_reference_ratios(rows: &mut [BenchmarkRow], reference_adapters: &[&str]) {
    let summaries = rows
        .iter()
        .filter_map(|row| {
            if row.outcome != BenchmarkOutcome::Timed
                || !row
                    .comparison_families
                    .contains(&ComparisonFamily::SameFormat)
                || validate_instrumented_samples(row).is_some()
                || !row_supports_comparison(row)
            {
                return None;
            }
            let summary = row.summary.as_ref()?;
            Some((
                (
                    row.case_id.clone(),
                    row.source_id.clone(),
                    row.tier.clone(),
                    row.adapter_id.clone(),
                ),
                (
                    summary.wall_time_micros.median,
                    summary.peak_rss_bytes,
                    row.input_format,
                    row.execution_class,
                    row.warmups,
                    row.requested_samples,
                    row.timeout_seconds,
                    row.limits.output_bytes,
                    row.limits.rss_bytes,
                    row_rss_source(row),
                    row.samples
                        .first()
                        .and_then(|sample| sample.measurement_protocol.clone()),
                ),
            ))
        })
        .collect::<BTreeMap<_, _>>();
    for row in rows {
        row.reference_ratios.clear();
        row.reference_peak_rss_ratios.clear();
        row.soft_performance_objective = None;
        let comparable_candidate = row.outcome == BenchmarkOutcome::Timed
            && row
                .comparison_families
                .contains(&ComparisonFamily::SameFormat)
            && validate_instrumented_samples(row).is_none();
        let comparable_candidate = comparable_candidate && row_supports_comparison(row);
        let Some(own) = row
            .summary
            .as_ref()
            .filter(|_| comparable_candidate)
            .map(|summary| summary.wall_time_micros.median)
        else {
            continue;
        };
        for reference in reference_adapters {
            let key = (
                row.case_id.clone(),
                row.source_id.clone(),
                row.tier.clone(),
                (*reference).to_owned(),
            );
            if let Some((
                reference_median,
                reference_rss,
                reference_format,
                reference_execution_class,
                reference_warmups,
                reference_samples,
                reference_timeout,
                reference_output_limit,
                reference_rss_limit,
                reference_source,
                reference_protocol,
            )) = summaries.get(&key)
                && row.input_format == *reference_format
                && row.execution_class == *reference_execution_class
                && row.warmups == *reference_warmups
                && row.requested_samples == *reference_samples
                && row.timeout_seconds == *reference_timeout
                && row.limits.output_bytes == *reference_output_limit
                && row.limits.rss_bytes == *reference_rss_limit
                && row_supports_comparison(row)
                && row_rss_source(row) == *reference_source
                && row
                    .samples
                    .first()
                    .and_then(|sample| sample.measurement_protocol.as_ref())
                    == reference_protocol.as_ref()
            {
                if *reference_median > 0.0 {
                    row.reference_ratios
                        .insert((*reference).to_owned(), own / reference_median);
                }
                if let (Some(own_rss), Some(reference_rss)) = (
                    row.summary
                        .as_ref()
                        .and_then(|summary| summary.peak_rss_bytes),
                    reference_rss,
                ) && *reference_rss > 0
                {
                    row.reference_peak_rss_ratios.insert(
                        (*reference).to_owned(),
                        own_rss as f64 / *reference_rss as f64,
                    );
                }
            }
        }
        if has_jq_soft_performance_objective(&row.case_id)
            && row.adapter_id == "tq-json"
            && reference_adapters.contains(&"jq-json")
        {
            let wall_time_ratio = row.reference_ratios.get("jq-json").copied();
            let peak_rss_ratio = row.reference_peak_rss_ratios.get("jq-json").copied();
            row.soft_performance_objective = Some(SoftPerformanceObjective {
                wall_time_ratio,
                peak_rss_ratio,
                wall_time: soft_status(wall_time_ratio, 2.0),
                peak_rss: soft_status(peak_rss_ratio, 1.5),
            });
        }
    }
}

fn has_jq_soft_performance_objective(case_id: &str) -> bool {
    case_id.starts_with("benchmark.issue5-")
        || matches!(
            case_id,
            "benchmark.recurse-bounded"
                | "benchmark.walk-structural"
                | "benchmark.label-early-break"
        )
}

fn soft_status(ratio: Option<f64>, target: f64) -> SoftObjectiveStatus {
    match ratio {
        Some(ratio) if ratio <= target => SoftObjectiveStatus::Met,
        Some(_) => SoftObjectiveStatus::Missed,
        None => SoftObjectiveStatus::NotComparable,
    }
}

/// Evaluates only tq rows against a comparable tq baseline.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "compare independent metrics only after all row compatibility checks"
)]
#[allow(
    clippy::cast_precision_loss,
    reason = "percentage thresholds intentionally compare floating-point ratios"
)]
pub fn evaluate_regression(
    baseline: &BenchmarkCampaignReport,
    candidate: &BenchmarkCampaignReport,
    thresholds: RegressionThresholds,
) -> RegressionGate {
    let comparability = report_comparability(baseline, candidate, true);
    if !comparability.comparable {
        let (unavailable, failures): (Vec<_>, Vec<_>) =
            comparability.reasons.into_iter().partition(|reason| {
                reason == "measurement protocol or RSS provenance differs"
                    || reason.contains("invalid instrumented evidence")
                    || reason.contains("native timing controls are unvalidated")
                    || reason.contains("unverified RSS provenance is present")
            });
        return RegressionGate {
            evaluated: false,
            thresholds: Some(thresholds),
            failures,
            unavailable,
            ..RegressionGate::default()
        };
    }
    let mut failures = Vec::new();
    let mut disclosures = Vec::new();
    let mut unavailable = Vec::new();
    let mut evaluated_rows = 0;
    for candidate_row in candidate
        .cases
        .iter()
        .filter(|row| row.adapter_id.starts_with("tq-"))
    {
        let Some(candidate_summary) = &candidate_row.summary else {
            unavailable.push(format!(
                "{} candidate has no summary",
                candidate_row.case_id
            ));
            continue;
        };
        if candidate_row.samples.len() < thresholds.minimum_samples {
            unavailable.push(format!(
                "{} candidate has insufficient samples",
                candidate_row.case_id
            ));
            continue;
        }
        let Some(baseline_row) = baseline.cases.iter().find(|row| {
            row.case_id == candidate_row.case_id
                && row.adapter_id == candidate_row.adapter_id
                && row.source_id == candidate_row.source_id
                && row.tier == candidate_row.tier
        }) else {
            unavailable.push(format!("{} has no baseline row", candidate_row.case_id));
            continue;
        };
        let Some(baseline_summary) = &baseline_row.summary else {
            unavailable.push(format!("{} baseline has no summary", candidate_row.case_id));
            continue;
        };
        let native_contracts_match = row_native_contract(baseline_row)
            .zip(row_native_contract(candidate_row))
            .is_some_and(|(baseline_contract, candidate_contract)| {
                baseline_contract == candidate_contract
            });
        if baseline_row.samples.len() < thresholds.minimum_samples
            || baseline_row.outcome != BenchmarkOutcome::Timed
            || candidate_row.outcome != BenchmarkOutcome::Timed
            || !commands_match_for_self_regression(baseline_row, candidate_row, baseline, candidate)
            || baseline_row.input_format != candidate_row.input_format
            || baseline_row.execution_class != candidate_row.execution_class
            || baseline_row.comparison_families != candidate_row.comparison_families
            || baseline_row.warmups != candidate_row.warmups
            || baseline_row.requested_samples != candidate_row.requested_samples
            || baseline_row.timeout_seconds != candidate_row.timeout_seconds
            || baseline_row.limits.output_bytes != candidate_row.limits.output_bytes
            || baseline_row.limits.rss_bytes != candidate_row.limits.rss_bytes
            || !native_contracts_match
            || row_rss_source(baseline_row) != row_rss_source(candidate_row)
            || row_rss_source(baseline_row).is_none()
            || row_rss_source(candidate_row).is_none()
        {
            unavailable.push(format!(
                "{} row contract or samples differ",
                candidate_row.case_id
            ));
            continue;
        }
        evaluated_rows += 1;
        let wall_change = percent_change(
            baseline_summary.wall_time_micros.median,
            candidate_summary.wall_time_micros.median,
        );
        if wall_change > 20.0 {
            disclosures.push(format_disclosure(
                candidate_row,
                "wall time",
                baseline_summary.wall_time_micros.median,
                candidate_summary.wall_time_micros.median,
                &baseline_summary.wall_time_micros,
                &candidate_summary.wall_time_micros,
                wall_change,
            ));
        }
        if wall_change > thresholds.wall_time_percent {
            failures.push(format!(
                "{}/{} median wall time regressed",
                candidate_row.case_id, candidate_row.adapter_id
            ));
        }
        if let (Some(old), Some(new)) = (
            baseline_summary.peak_rss_bytes,
            candidate_summary.peak_rss_bytes,
        ) {
            let rss_change = percent_change(old as f64, new as f64);
            let baseline_rss = metric(
                baseline_row
                    .samples
                    .iter()
                    .filter_map(|sample| sample.peak_rss_bytes)
                    .map(|bytes| bytes as f64),
            );
            let candidate_rss = metric(
                candidate_row
                    .samples
                    .iter()
                    .filter_map(|sample| sample.peak_rss_bytes)
                    .map(|bytes| bytes as f64),
            );
            if rss_change > 20.0
                && let (Some(baseline_rss), Some(candidate_rss)) =
                    (baseline_rss.as_ref(), candidate_rss.as_ref())
            {
                disclosures.push(format_disclosure(
                    candidate_row,
                    "peak RSS",
                    old as f64,
                    new as f64,
                    baseline_rss,
                    candidate_rss,
                    rss_change,
                ));
            }
            if rss_change > thresholds.peak_rss_percent {
                failures.push(format!(
                    "{}/{} peak RSS regressed",
                    candidate_row.case_id, candidate_row.adapter_id
                ));
            }
        } else {
            unavailable.push(format!(
                "{} lacks comparable peak RSS",
                candidate_row.case_id
            ));
        }
    }
    RegressionGate {
        evaluated: evaluated_rows > 0,
        thresholds: Some(thresholds),
        failures,
        disclosures,
        unavailable,
    }
}

fn corpus_identities_match(
    left: &[BenchmarkCorpusIdentity],
    right: &[BenchmarkCorpusIdentity],
) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| corpus_identity_matches(left, right))
}

/// Compare the immutable artifact identity while allowing cache relocation.
fn corpus_identity_matches(
    left: &BenchmarkCorpusIdentity,
    right: &BenchmarkCorpusIdentity,
) -> bool {
    left.origin == right.origin
        && left.source_id == right.source_id
        && left.tier == right.tier
        && left.format == right.format
        && left.artifact.bytes == right.artifact.bytes
        && left.artifact.sha256 == right.artifact.sha256
        && left.logical_records == right.logical_records
        && left.manifest_sha256 == right.manifest_sha256
}

fn commands_match_for_self_regression(
    baseline: &BenchmarkRow,
    candidate: &BenchmarkRow,
    baseline_report: &BenchmarkCampaignReport,
    candidate_report: &BenchmarkCampaignReport,
) -> bool {
    let Some(baseline_command) = baseline.command.get(1..) else {
        return false;
    };
    let Some(candidate_command) = candidate.command.get(1..) else {
        return false;
    };
    let baseline_path = row_artifact_path(baseline_report, baseline);
    let candidate_path = row_artifact_path(candidate_report, candidate);
    normalize_corpus_arguments(baseline_command, baseline_path)
        == normalize_corpus_arguments(candidate_command, candidate_path)
}

fn row_artifact_path<'a>(
    report: &'a BenchmarkCampaignReport,
    row: &BenchmarkRow,
) -> Option<&'a str> {
    report
        .corpus
        .iter()
        .find(|identity| {
            identity.source_id == row.source_id
                && identity.tier == row.tier
                && identity.format == row.input_format
        })
        .map(|identity| identity.artifact.path.as_str())
}

/// Replace only an exact recorded artifact-path argument; flags and queries stay literal.
fn normalize_corpus_arguments<'a>(
    command: &'a [String],
    artifact_path: Option<&str>,
) -> Vec<Option<&'a str>> {
    command
        .iter()
        .map(|argument| {
            if artifact_path.is_some_and(|path| argument == path) {
                None
            } else {
                Some(argument.as_str())
            }
        })
        .collect()
}

fn row_native_contract(row: &BenchmarkRow) -> Option<&MeasurementProtocol> {
    let source = row_rss_source(row)?;
    if !matches!(
        source,
        RssProvenance::DarwinWait4 | RssProvenance::LinuxWait4
    ) {
        return None;
    }
    row.samples
        .first()
        .and_then(|sample| sample.measurement_protocol.as_ref())
        .filter(|protocol| protocol.is_valid_for_comparison())
}

fn row_supports_comparison(row: &BenchmarkRow) -> bool {
    let Some(source) = row_rss_source(row) else {
        return false;
    };
    !is_native_rss_provenance(source) || row_native_contract(row).is_some()
}

fn has_unvalidated_native_timing(report: &BenchmarkCampaignReport) -> bool {
    report
        .cases
        .iter()
        .flat_map(|row| row.samples.iter().chain(&row.instrumented_samples))
        .any(|sample| {
            sample.rss_provenance.is_some_and(is_native_rss_provenance)
                && sample
                    .measurement_protocol
                    .as_ref()
                    .is_none_or(|protocol| !protocol.is_valid_for_comparison())
        })
}

fn has_unverified_rss_provenance(report: &BenchmarkCampaignReport) -> bool {
    report
        .cases
        .iter()
        .flat_map(|row| row.samples.iter().chain(&row.instrumented_samples))
        .any(|sample| sample.rss_provenance.is_none())
}

#[allow(
    clippy::too_many_lines,
    reason = "keep the separated instrumentation contract checks together"
)]
fn validate_instrumented_samples(row: &BenchmarkRow) -> Option<String> {
    let primary_source = row_rss_source(row);
    let native_primary = primary_source.is_some_and(is_native_rss_provenance);
    if row.instrumented_samples.is_empty() {
        if row.outcome == BenchmarkOutcome::Timed
            && row.limits.rss_bytes.is_some()
            && native_primary
        {
            return Some("lacks matching instrumented RSS-limit repetitions".to_owned());
        }
        return None;
    }
    if row.limits.rss_bytes.is_none() {
        return Some("has instrumented samples without an RSS limit".to_owned());
    }
    if row.outcome == BenchmarkOutcome::Timed && row.instrumented_samples.len() != row.samples.len()
    {
        return Some(format!(
            "has {} instrumented RSS-limit repetitions for {} timing samples",
            row.instrumented_samples.len(),
            row.samples.len()
        ));
    }
    if row.samples.iter().any(|sample| {
        sample
            .measurement_protocol
            .as_ref()
            .is_some_and(|protocol| protocol.rss_poll_interval_micros.is_some())
    }) {
        return Some("mixes instrumented and timing samples".to_owned());
    }

    let first = &row.instrumented_samples[0];
    let Some(source) = first.rss_provenance else {
        return Some("has instrumented samples without RSS provenance".to_owned());
    };
    if !is_native_rss_provenance(source) {
        return Some(format!(
            "uses non-native instrumented RSS provenance {}",
            source.label()
        ));
    }
    if primary_source.is_some_and(|primary| primary != source) {
        return Some("mixes primary and instrumented RSS provenance".to_owned());
    }
    let Some(protocol) = first.measurement_protocol.as_ref() else {
        return Some("has instrumented samples without a measurement protocol".to_owned());
    };
    if !protocol.is_valid() || protocol.rss_poll_interval_micros.is_none() {
        return Some("has invalid instrumented RSS sampler protocol".to_owned());
    }
    if row.instrumented_samples.iter().any(|sample| {
        sample.rss_provenance != Some(source)
            || sample.measurement_protocol.as_ref() != Some(protocol)
            || !instrumented_sample_has_native_evidence(sample)
    }) {
        return Some("mixes or contains invalid instrumented RSS evidence".to_owned());
    }
    None
}

fn instrumented_sample_has_native_evidence(sample: &BenchmarkSample) -> bool {
    sample.peak_rss_bytes.is_some_and(|bytes| bytes > 0)
        && sample.user_cpu_micros.is_some()
        && sample.system_cpu_micros.is_some()
        && sample
            .measurement_protocol
            .as_ref()
            .is_some_and(MeasurementProtocol::is_valid)
}

const fn is_native_rss_provenance(source: RssProvenance) -> bool {
    matches!(
        source,
        RssProvenance::DarwinWait4 | RssProvenance::LinuxWait4
    )
}

fn format_disclosure(
    row: &BenchmarkRow,
    metric_name: &str,
    baseline: f64,
    candidate: f64,
    baseline_metrics: &MetricSummary,
    candidate_metrics: &MetricSummary,
    increase_percent: f64,
) -> String {
    format!(
        "{}/{} {metric_name} increased by {increase_percent:?}% (baseline={baseline:?}, candidate={candidate:?}, baseline_samples={}, candidate_samples={}, baseline_dispersion=mad:{:?};p95:{:?};range:{:?}-{:?}, candidate_dispersion=mad:{:?};p95:{:?};range:{:?}-{:?}, explanation=issue-30 disclosure above 20%)",
        row.case_id,
        row.adapter_id,
        baseline_metrics.samples,
        candidate_metrics.samples,
        baseline_metrics.median_absolute_deviation,
        baseline_metrics.p95,
        baseline_metrics.minimum,
        baseline_metrics.maximum,
        candidate_metrics.median_absolute_deviation,
        candidate_metrics.p95,
        candidate_metrics.minimum,
        candidate_metrics.maximum,
    )
}

fn row_rss_source(row: &BenchmarkRow) -> Option<RssProvenance> {
    let source = row.samples.first()?.rss_provenance?;
    if matches!(
        source,
        RssProvenance::DarwinWait4 | RssProvenance::LinuxWait4
    ) && row.samples[0]
        .measurement_protocol
        .as_ref()
        .is_none_or(|protocol| !protocol.is_valid())
    {
        return None;
    }
    row.samples
        .iter()
        .all(|sample| {
            sample.rss_provenance == Some(source)
                && sample.measurement_protocol == row.samples[0].measurement_protocol
                && sample.peak_rss_bytes.is_some_and(|bytes| bytes > 0)
                && (!matches!(
                    source,
                    RssProvenance::DarwinWait4 | RssProvenance::LinuxWait4
                ) || (sample.user_cpu_micros.is_some() && sample.system_cpu_micros.is_some()))
        })
        .then_some(source)
}

fn metric(values: impl Iterator<Item = f64>) -> Option<MetricSummary> {
    let mut values = values.collect::<Vec<_>>();
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let median_value = median(&values);
    let mut deviations = values
        .iter()
        .map(|value| (value - median_value).abs())
        .collect::<Vec<_>>();
    deviations.sort_by(f64::total_cmp);
    let rank = values
        .len()
        .saturating_mul(95)
        .saturating_add(99)
        .checked_div(100)
        .unwrap_or(1)
        .saturating_sub(1)
        .min(values.len() - 1);
    Some(MetricSummary {
        samples: values.len(),
        median: median_value,
        median_absolute_deviation: median(&deviations),
        p95: values[rank],
        minimum: values[0],
        maximum: values[values.len() - 1],
    })
}

#[allow(
    clippy::cast_precision_loss,
    reason = "CPU duration summaries intentionally use floating-point medians"
)]
fn optional_median(values: impl Iterator<Item = u128>) -> Option<f64> {
    let mut values = values.map(|value| value as f64).collect::<Vec<_>>();
    values.sort_by(f64::total_cmp);
    (!values.is_empty()).then(|| median(&values))
}

fn median(sorted: &[f64]) -> f64 {
    let middle = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        sorted[middle - 1].mul_add(0.5, sorted[middle] * 0.5)
    } else {
        sorted[middle]
    }
}

fn percent_change(old: f64, new: f64) -> f64 {
    if old == 0.0 {
        return if new == 0.0 { 0.0 } else { f64::INFINITY };
    }
    (new - old) / old * 100.0
}
