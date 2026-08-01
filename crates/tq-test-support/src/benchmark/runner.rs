//! Correctness-first warmup and sampling loop.

use std::collections::BTreeMap;

use thiserror::Error;

use super::{
    BenchmarkAdapter, BenchmarkCase, BenchmarkCorpusIdentity, BenchmarkInvocation,
    BenchmarkOutcome, BenchmarkRow, BenchmarkSample, CorrectnessDecision, DatasetTier,
    MeasuredOutcome, MeasuredStatus, OutputContractKind, correctness_gate, measure_process,
    summarize_samples,
};
use crate::compatibility::{
    Invocation, NormalizationError, NormalizedObservation, ProcessError, ProcessOutcome, ToolKind,
    normalize_jq, normalize_raw, normalize_toon_sequence, normalize_yq, run_process,
};

/// Benchmark row construction failures at the harness boundary.
#[derive(Debug, Error)]
pub enum BenchmarkRunnerError {
    /// Correctness subprocess failed to launch or capture.
    #[error(transparent)]
    Process(#[from] ProcessError),
    /// Correctness output could not be normalized.
    #[error(transparent)]
    Normalize(#[from] NormalizationError),
    /// Timed process could not be measured.
    #[error(transparent)]
    Measure(#[from] super::MeasureError),
}

/// Runs one unmeasured correctness invocation and normalizes its contract.
///
/// # Errors
///
/// Returns process and normalization failures.
pub fn normalize_correctness_run(
    invocation: &BenchmarkInvocation,
    tool: ToolKind,
    contract: OutputContractKind,
) -> Result<NormalizedObservation, BenchmarkRunnerError> {
    let outcome = run_process(&Invocation {
        executable: invocation.executable.clone(),
        args: invocation.args.clone(),
        stdin: invocation.stdin.clone(),
        timeout: invocation.timeout,
        current_dir: invocation.current_dir.clone(),
    })?;
    Ok(match contract {
        OutputContractKind::RawBytes | OutputContractKind::ExitOnly => {
            normalize_raw(tool, &outcome)
        }
        OutputContractKind::SemanticSequence => normalize(tool, &outcome)?,
    })
}

/// Executes correctness, then warmups and samples only on a passing gate.
///
/// # Errors
///
/// Returns harness lifecycle failures. Tool failures are first-class row
/// outcomes and stop further samples for that row.
pub fn run_gated_row(
    case: &BenchmarkCase,
    adapter: &BenchmarkAdapter,
    corpus: &BenchmarkCorpusIdentity,
    tier: DatasetTier,
    invocation: &BenchmarkInvocation,
    reference: &NormalizedObservation,
) -> Result<BenchmarkRow, BenchmarkRunnerError> {
    let tool = tool_kind(adapter);
    let candidate = normalize_correctness_run(invocation, tool, case.output_contract.kind);
    let decision = match &candidate {
        Ok(observation) => correctness_gate(case.output_contract.kind, reference, Ok(observation)),
        Err(error) => correctness_gate(
            case.output_contract.kind,
            reference,
            Err(&error.to_string()),
        ),
    };
    if decision != CorrectnessDecision::Passed {
        return Ok(row(
            case,
            adapter,
            corpus,
            tier,
            invocation,
            BenchmarkOutcome::Incorrect,
            Vec::new(),
        ));
    }

    for _ in 0..case.sampling.warmups {
        let warmup = measure_process(invocation)?;
        if let Some(outcome) = failed_outcome(case, &warmup) {
            return Ok(row(
                case,
                adapter,
                corpus,
                tier,
                invocation,
                outcome,
                Vec::new(),
            ));
        }
    }
    let mut samples = Vec::with_capacity(case.sampling.measured(tier));
    for _ in 0..case.sampling.measured(tier) {
        let measured = measure_process(invocation)?;
        if let Some(outcome) = failed_outcome(case, &measured) {
            return Ok(row(
                case, adapter, corpus, tier, invocation, outcome, samples,
            ));
        }
        samples.push(BenchmarkSample::from(&measured));
    }
    Ok(row(
        case,
        adapter,
        corpus,
        tier,
        invocation,
        BenchmarkOutcome::Timed,
        samples,
    ))
}

/// Constructs an explicit not-applicable row without invoking a process.
#[must_use]
pub fn unsupported_row(
    case: &BenchmarkCase,
    adapter: &BenchmarkAdapter,
    corpus: &BenchmarkCorpusIdentity,
    tier: DatasetTier,
    invocation: &BenchmarkInvocation,
) -> BenchmarkRow {
    row(
        case,
        adapter,
        corpus,
        tier,
        invocation,
        BenchmarkOutcome::Unsupported,
        Vec::new(),
    )
}

fn row(
    case: &BenchmarkCase,
    adapter: &BenchmarkAdapter,
    corpus: &BenchmarkCorpusIdentity,
    tier: DatasetTier,
    invocation: &BenchmarkInvocation,
    outcome: BenchmarkOutcome,
    samples: Vec<BenchmarkSample>,
) -> BenchmarkRow {
    let summary = (outcome == BenchmarkOutcome::Timed)
        .then(|| summarize_samples(&samples, corpus.artifact.bytes, corpus.logical_records))
        .flatten();
    let mut command = vec![invocation.executable.display().to_string()];
    command.extend(invocation.args.clone());
    BenchmarkRow {
        case_id: case.id.clone(),
        adapter_id: adapter.id.clone(),
        source_id: corpus.source_id.clone(),
        tier: tier_name(tier).to_owned(),
        input_format: adapter.input_format,
        execution_class: case.execution_class,
        comparison_families: adapter.comparison_families.clone(),
        command,
        outcome,
        warmups: case.sampling.warmups,
        requested_samples: case.sampling.measured(tier),
        timeout_seconds: case.timeout_seconds,
        limits: case.limits.clone(),
        samples,
        summary,
        reference_ratios: BTreeMap::new(),
    }
}

fn failed_outcome(case: &BenchmarkCase, measured: &MeasuredOutcome) -> Option<BenchmarkOutcome> {
    match measured.status {
        MeasuredStatus::Timeout => Some(BenchmarkOutcome::Timeout),
        MeasuredStatus::Signaled => Some(BenchmarkOutcome::OomOrSignal),
        MeasuredStatus::OutputLimit => Some(BenchmarkOutcome::ResourceLimit),
        MeasuredStatus::Exited => {
            if case.measure_first_result && measured.first_result_micros.is_none() {
                return Some(BenchmarkOutcome::ResourceLimit);
            }
            if let (Some(limit), Some(actual)) = (case.limits.rss_bytes, measured.peak_rss_bytes)
                && actual > limit
            {
                return Some(BenchmarkOutcome::ResourceLimit);
            }
            None
        }
    }
}

fn normalize(
    tool: ToolKind,
    outcome: &ProcessOutcome,
) -> Result<NormalizedObservation, NormalizationError> {
    match tool {
        ToolKind::Jq => normalize_jq(outcome),
        ToolKind::Yq => normalize_yq(outcome),
        ToolKind::Tq => normalize_toon_sequence(outcome),
    }
}

const fn tool_kind(adapter: &BenchmarkAdapter) -> ToolKind {
    match adapter.tool {
        super::BenchmarkTool::Jq => ToolKind::Jq,
        super::BenchmarkTool::Yq => ToolKind::Yq,
        super::BenchmarkTool::Tq => ToolKind::Tq,
    }
}

const fn tier_name(tier: DatasetTier) -> &'static str {
    match tier {
        DatasetTier::Small => "small",
        DatasetTier::Medium => "medium",
        DatasetTier::Large => "large",
        DatasetTier::Startup => "startup",
    }
}
