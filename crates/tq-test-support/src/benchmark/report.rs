//! Benchmark statistics, failure rows, comparability, and regression gates.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{
    BenchmarkLimits, ComparisonFamily, EnvironmentManifest, ExecutionClass, InputFormat,
    MeasuredOutcome,
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
    /// Aggregate metrics for valid samples.
    pub summary: Option<RowSummary>,
    /// Ratios to named reference rows; informational only.
    pub reference_ratios: BTreeMap<String, f64>,
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

/// One measured fresh-process invocation.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BenchmarkSample {
    /// Wall duration.
    pub wall_time_micros: u128,
    /// User CPU duration.
    pub user_cpu_micros: Option<u128>,
    /// System CPU duration.
    pub system_cpu_micros: Option<u128>,
    /// Peak resident bytes.
    pub peak_rss_bytes: Option<u64>,
    /// Time to first stdout byte.
    pub first_result_micros: Option<u128>,
    /// Exact output bytes.
    pub output_bytes: u64,
}

impl From<&MeasuredOutcome> for BenchmarkSample {
    fn from(value: &MeasuredOutcome) -> Self {
        Self {
            wall_time_micros: value.wall_time_micros,
            user_cpu_micros: value.user_cpu_micros,
            system_cpu_micros: value.system_cpu_micros,
            peak_rss_bytes: value.peak_rss_bytes,
            first_result_micros: value.first_result_micros,
            output_bytes: value.output_bytes,
        }
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
                writeln!(
                    output,
                    "- {} {} {}: {:?}, {}",
                    row.case_id, row.adapter_id, row.source_id, row.outcome, median
                )
                .expect("write benchmark report");
            }
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
    let mut reasons = Vec::new();
    if left.environment.machine_identity != right.environment.machine_identity {
        reasons.push("machine identity differs".to_owned());
    }
    if left.corpus != right.corpus {
        reasons.push("corpus identity differs".to_owned());
    }
    if serde_json::to_value(&left.tools).ok() != serde_json::to_value(&right.tools).ok() {
        reasons.push("tool identity differs".to_owned());
    }
    Comparability {
        comparable: reasons.is_empty(),
        reasons,
    }
}

/// Adds independent wall-time ratios to matching named reference adapters.
///
/// Ratios are informational and are never combined into a winner score.
pub fn populate_reference_ratios(rows: &mut [BenchmarkRow], reference_adapters: &[&str]) {
    let medians = rows
        .iter()
        .filter_map(|row| {
            let median = row.summary.as_ref()?.wall_time_micros.median;
            Some((
                (
                    row.case_id.clone(),
                    row.source_id.clone(),
                    row.tier.clone(),
                    row.adapter_id.clone(),
                ),
                median,
            ))
        })
        .collect::<BTreeMap<_, _>>();
    for row in rows {
        let Some(own) = row
            .summary
            .as_ref()
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
            if let Some(reference_median) = medians.get(&key)
                && *reference_median > 0.0
            {
                row.reference_ratios
                    .insert((*reference).to_owned(), own / reference_median);
            }
        }
    }
}

/// Evaluates only tq rows against a comparable tq baseline.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    reason = "percentage thresholds intentionally compare floating-point ratios"
)]
pub fn evaluate_regression(
    baseline: &BenchmarkCampaignReport,
    candidate: &BenchmarkCampaignReport,
    thresholds: RegressionThresholds,
) -> RegressionGate {
    let comparability = compare_reports(baseline, candidate);
    if !comparability.comparable {
        return RegressionGate {
            evaluated: false,
            thresholds: Some(thresholds),
            failures: comparability.reasons,
        };
    }
    let mut failures = Vec::new();
    for candidate_row in candidate
        .cases
        .iter()
        .filter(|row| row.adapter_id.starts_with("tq-"))
    {
        let Some(candidate_summary) = &candidate_row.summary else {
            continue;
        };
        if candidate_row.samples.len() < thresholds.minimum_samples {
            continue;
        }
        let Some(baseline_row) = baseline.cases.iter().find(|row| {
            row.case_id == candidate_row.case_id
                && row.adapter_id == candidate_row.adapter_id
                && row.source_id == candidate_row.source_id
        }) else {
            continue;
        };
        let Some(baseline_summary) = &baseline_row.summary else {
            continue;
        };
        if percent_change(
            baseline_summary.wall_time_micros.median,
            candidate_summary.wall_time_micros.median,
        ) > thresholds.wall_time_percent
        {
            failures.push(format!(
                "{}/{} median wall time regressed",
                candidate_row.case_id, candidate_row.adapter_id
            ));
        }
        if let (Some(old), Some(new)) = (
            baseline_summary.peak_rss_bytes,
            candidate_summary.peak_rss_bytes,
        ) && percent_change(old as f64, new as f64) > thresholds.peak_rss_percent
        {
            failures.push(format!(
                "{}/{} peak RSS regressed",
                candidate_row.case_id, candidate_row.adapter_id
            ));
        }
    }
    RegressionGate {
        evaluated: true,
        thresholds: Some(thresholds),
        failures,
    }
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
    if sorted.len() % 2 == 0 {
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
