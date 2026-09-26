//! Correctness-first warmup and sampling loop.

use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead as _, BufReader, Read as _, Seek as _},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

use thiserror::Error;

use super::correctness::{SemanticDigestError, SemanticWitness, value_digest};
use super::{
    BenchmarkAdapter, BenchmarkCase, BenchmarkCorpusIdentity, BenchmarkInvocation,
    BenchmarkOutcome, BenchmarkRow, BenchmarkSample, CorrectnessDecision, CorrectnessObservation,
    CorrectnessPayload, DatasetTier, MeasureError, MeasuredOutcome, MeasuredStatus,
    OutputContractKind, SemanticDigest, SemanticDigester, correctness_gate, measure_process,
    measure_process_worker, measure_process_worker_uninstrumented, summarize_samples,
};
use crate::compatibility::{
    ErrorClass, NormalizationError, ProcessError, ProcessOutcome, ProcessStatus, ToolKind,
    classify_process,
};

const MAX_CORRECTNESS_OUTPUT_BYTES: u64 = 32 * 1024 * 1024;

/// Returns whether a runner error represents the bounded correctness capture
/// limit rather than an infrastructure failure.
#[must_use]
pub const fn is_correctness_output_limit(error: &BenchmarkRunnerError) -> bool {
    matches!(error, BenchmarkRunnerError::CorrectnessOutputLimit { .. })
}

/// Returns whether a runner error represents a bounded correctness capture
/// limit rather than an infrastructure failure.
#[must_use]
pub const fn is_correctness_resource_limit(error: &BenchmarkRunnerError) -> bool {
    matches!(error, BenchmarkRunnerError::CorrectnessOutputLimit { .. })
}

/// Benchmark row construction failures at the harness boundary.
#[derive(Debug, Error)]
pub enum BenchmarkRunnerError {
    /// Correctness output file I/O failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Correctness subprocess failed to launch or capture.
    #[error(transparent)]
    Process(#[from] ProcessError),
    /// Correctness output could not be normalized.
    #[error(transparent)]
    Normalize(#[from] NormalizationError),
    /// Timed process could not be measured.
    #[error(transparent)]
    Measure(#[from] super::MeasureError),
    /// Correctness output exceeded the bounded file-backed capture limit.
    #[error("correctness output exceeded {limit} bytes; capture files were removed")]
    CorrectnessOutputLimit {
        /// Maximum allowed stdout bytes.
        limit: u64,
        /// Deleted stdout capture path, retained for diagnostic identity.
        stdout: std::path::PathBuf,
        /// Deleted stderr capture path, retained for diagnostic identity.
        stderr: std::path::PathBuf,
    },
}

type CandidateResult = Result<(CorrectnessObservation, MeasuredOutcome), BenchmarkRunnerError>;

#[derive(Debug, Error)]
enum DigestError {
    #[error(transparent)]
    Normalize(#[from] NormalizationError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("benchmark measurement cancelled")]
    Cancelled,
}
impl From<DigestError> for BenchmarkRunnerError {
    fn from(error: DigestError) -> Self {
        match error {
            DigestError::Normalize(error) => Self::Normalize(error),
            DigestError::Io(error) => Self::Io(error),
            DigestError::Cancelled => Self::Measure(MeasureError::Cancelled),
        }
    }
}

fn check_cancelled(cancellation: Option<&AtomicBool>) -> Result<(), DigestError> {
    if cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
        Err(DigestError::Cancelled)
    } else {
        Ok(())
    }
}

fn push_digest(
    digest: &mut SemanticDigester,
    value: &serde_json::Value,
    tool: ToolKind,
) -> Result<(), DigestError> {
    match digest.push(value) {
        Ok(()) => Ok(()),
        Err(SemanticDigestError::Json(error)) => Err(DigestError::Normalize(normalization_error(
            tool,
            error.to_string(),
        ))),
        Err(SemanticDigestError::Io(error)) => Err(DigestError::Io(error)),
    }
}

fn propagate_measure_error(
    candidate: CandidateResult,
) -> Result<CandidateResult, BenchmarkRunnerError> {
    match candidate {
        Err(error @ BenchmarkRunnerError::Measure(_)) => Err(error),
        result => Ok(result),
    }
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
) -> Result<CorrectnessObservation, BenchmarkRunnerError> {
    measured_correctness_run(invocation, tool, contract, None).map(|(observation, _)| observation)
}

fn measured_correctness_run(
    invocation: &BenchmarkInvocation,
    tool: ToolKind,
    contract: OutputContractKind,
    expected: Option<&SemanticDigest>,
) -> Result<(CorrectnessObservation, MeasuredOutcome), BenchmarkRunnerError> {
    let mut correctness = invocation.clone();
    correctness.output_limit = correctness.output_limit.min(MAX_CORRECTNESS_OUTPUT_BYTES);
    correctness.retain_output = true;
    let measured = measure_process(&correctness)?;
    let stdout_path = measured
        .stdout_path
        .clone()
        .ok_or_else(|| std::io::Error::other("missing correctness stdout file"))?;
    let stderr_path = measured
        .stderr_path
        .clone()
        .ok_or_else(|| std::io::Error::other("missing correctness stderr file"))?;
    if measured.status == MeasuredStatus::OutputLimit {
        let retained_stdout = stdout_path.clone();
        let retained_stderr = stderr_path.clone();
        let _ = std::fs::remove_file(&stdout_path);
        let _ = std::fs::remove_file(&stderr_path);
        return Err(BenchmarkRunnerError::CorrectnessOutputLimit {
            limit: correctness.output_limit,
            stdout: retained_stdout,
            stderr: retained_stderr,
        });
    }
    let normalized = correctness_observation(
        (&stdout_path, &stderr_path),
        tool,
        contract,
        &measured,
        &invocation.args,
        expected,
        invocation.cancellation.as_deref(),
    );
    let _ = std::fs::remove_file(stdout_path);
    let _ = std::fs::remove_file(stderr_path);
    Ok((normalized?, measured))
}

fn correctness_gate_row(
    case: &BenchmarkCase,
    adapter: &BenchmarkAdapter,
    corpus: &BenchmarkCorpusIdentity,
    tier: DatasetTier,
    invocation: &BenchmarkInvocation,
    reference: &CorrectnessObservation,
) -> Result<Option<BenchmarkRow>, BenchmarkRunnerError> {
    if is_unverified_resource_observation(reference) {
        return Ok(Some(reference_resource_limit_row(
            case, adapter, corpus, tier, invocation,
        )));
    }
    let tool = tool_kind(adapter);
    let expected = match &reference.payload {
        CorrectnessPayload::SemanticSequence(digest) => Some(digest),
        _ => None,
    };
    let candidate = propagate_measure_error(measured_correctness_run(
        invocation,
        tool,
        case.output_contract.kind,
        expected,
    ))?;
    if let Err(BenchmarkRunnerError::CorrectnessOutputLimit { limit, .. }) = &candidate {
        return Ok(Some(correctness_limit_row(
            case, adapter, corpus, tier, invocation, *limit,
        )));
    }
    if let Ok((observation, measured)) = &candidate {
        let process_failure = if measured.status == MeasuredStatus::RssLimit {
            Some(BenchmarkOutcome::ResourceLimit)
        } else {
            correctness_process_failure(observation.process_status, observation.error_class)
                .or_else(|| {
                    (observation.exit_code != Some(0)).then_some(BenchmarkOutcome::Incorrect)
                })
        };
        if let Some(outcome) = process_failure {
            let mut row = row_with_diagnostic(
                case,
                adapter,
                corpus,
                tier,
                invocation,
                outcome,
                vec![BenchmarkSample::from(measured)],
                Some(process_diagnostic(observation, measured)),
            );
            if measured
                .measurement_protocol
                .rss_poll_interval_micros
                .is_some()
            {
                row.instrumented_samples = std::mem::take(&mut row.samples);
            }
            return Ok(Some(row));
        }
    }
    let decision = match &candidate {
        Ok((observation, _)) => {
            correctness_gate(case.output_contract.kind, reference, Ok(observation))
        }
        Err(error) => correctness_gate(
            case.output_contract.kind,
            reference,
            Err(&error.to_string()),
        ),
    };
    if decision != CorrectnessDecision::Passed {
        return Ok(Some(row_with_diagnostic(
            case,
            adapter,
            corpus,
            tier,
            invocation,
            BenchmarkOutcome::Incorrect,
            Vec::new(),
            Some(bounded_diagnostic(&decision)),
        )));
    }
    Ok(None)
}

fn is_unverified_resource_observation(observation: &CorrectnessObservation) -> bool {
    observation.process_status == ProcessStatus::Signaled
        && observation.error_class == Some(ErrorClass::Resource)
}

fn reference_resource_limit_row(
    case: &BenchmarkCase,
    adapter: &BenchmarkAdapter,
    corpus: &BenchmarkCorpusIdentity,
    tier: DatasetTier,
    invocation: &BenchmarkInvocation,
) -> BenchmarkRow {
    row_with_diagnostic(
        case,
        adapter,
        corpus,
        tier,
        invocation,
        BenchmarkOutcome::ResourceLimit,
        Vec::new(),
        Some(
            "reference correctness run exceeded an RSS limit; candidate remains unverified"
                .to_owned(),
        ),
    )
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
    reference: &CorrectnessObservation,
    instrument_rss: bool,
) -> Result<BenchmarkRow, BenchmarkRunnerError> {
    if invocation
        .cancellation
        .as_ref()
        .is_some_and(|flag| flag.load(Ordering::Relaxed))
    {
        return Err(BenchmarkRunnerError::Measure(MeasureError::Cancelled));
    }
    if let Some(row) = correctness_gate_row(case, adapter, corpus, tier, invocation, reference)? {
        return Ok(row);
    }
    if invocation
        .cancellation
        .as_ref()
        .is_some_and(|flag| flag.load(Ordering::Relaxed))
    {
        return Err(BenchmarkRunnerError::Measure(MeasureError::Cancelled));
    }

    for _ in 0..case.sampling.warmups {
        let warmup = measure_process_worker(invocation)?;
        if let Some(outcome) = failed_outcome(case, &warmup) {
            let diagnostic = measured_diagnostic(&outcome, &warmup);
            return Ok(row_with_diagnostic(
                case,
                adapter,
                corpus,
                tier,
                invocation,
                outcome,
                Vec::new(),
                Some(diagnostic),
            ));
        }
    }
    let mut samples = Vec::with_capacity(case.sampling.measured(tier));
    let mut instrumented_samples = Vec::new();
    for _ in 0..case.sampling.measured(tier) {
        // Explicit comparison mode can retain a separate sampler repetition;
        // the default uses one native wait4/sampler measurement per sample.
        if instrument_rss && invocation.rss_limit.is_some() {
            let instrumented = measure_process_worker(invocation)?;
            instrumented_samples.push(BenchmarkSample::from(&instrumented));
            if let Some(outcome) = failed_outcome(case, &instrumented) {
                let diagnostic = measured_diagnostic(&outcome, &instrumented);
                let mut row = row_with_diagnostic(
                    case,
                    adapter,
                    corpus,
                    tier,
                    invocation,
                    outcome,
                    samples,
                    Some(diagnostic),
                );
                row.instrumented_samples = instrumented_samples;
                return Ok(row);
            }
        }
        let measured = if !instrument_rss && invocation.rss_limit.is_some() {
            measure_process_worker(invocation)?
        } else {
            measure_process_worker_uninstrumented(invocation)?
        };
        if let Some(outcome) = failed_outcome(case, &measured) {
            samples.push(BenchmarkSample::from(&measured));
            let diagnostic = measured_diagnostic(&outcome, &measured);
            let mut row = row_with_diagnostic(
                case,
                adapter,
                corpus,
                tier,
                invocation,
                outcome,
                samples,
                Some(diagnostic),
            );
            row.instrumented_samples = instrumented_samples;
            return Ok(row);
        }
        samples.push(BenchmarkSample::from(&measured));
    }
    let mut row = row(
        case,
        adapter,
        corpus,
        tier,
        invocation,
        BenchmarkOutcome::Timed,
        samples,
    );
    row.instrumented_samples = instrumented_samples;
    Ok(row)
}

fn correctness_limit_row(
    case: &BenchmarkCase,
    adapter: &BenchmarkAdapter,
    corpus: &BenchmarkCorpusIdentity,
    tier: DatasetTier,
    invocation: &BenchmarkInvocation,
    limit: u64,
) -> BenchmarkRow {
    let mut bounded_case = case.clone();
    bounded_case.limits.output_bytes = limit;
    row_with_diagnostic(
        &bounded_case,
        adapter,
        corpus,
        tier,
        invocation,
        BenchmarkOutcome::ResourceLimit,
        Vec::new(),
        Some(format!(
            "candidate correctness output exceeded {limit} bytes"
        )),
    )
}

/// Runs one bounded process probe when the reference output itself is too
/// large to normalize safely. This preserves an explicit row for every tool
/// without timing an unverified result or loading gigabytes into the harness.
///
/// # Errors
///
/// Returns process measurement failures.
pub fn run_correctness_limit_probe(
    case: &BenchmarkCase,
    adapter: &BenchmarkAdapter,
    corpus: &BenchmarkCorpusIdentity,
    tier: DatasetTier,
    invocation: &BenchmarkInvocation,
) -> Result<BenchmarkRow, BenchmarkRunnerError> {
    let mut bounded = invocation.clone();
    bounded.output_limit = bounded.output_limit.min(MAX_CORRECTNESS_OUTPUT_BYTES);
    let measured = measure_process(&bounded)?;
    let outcome = match measured.status {
        MeasuredStatus::OutputLimit | MeasuredStatus::RssLimit => BenchmarkOutcome::ResourceLimit,
        MeasuredStatus::Timeout => BenchmarkOutcome::Timeout,
        MeasuredStatus::Signaled => BenchmarkOutcome::OomOrSignal,
        MeasuredStatus::Exited if measured.exit_code == Some(0) => BenchmarkOutcome::ResourceLimit,
        MeasuredStatus::Exited => BenchmarkOutcome::Incorrect,
    };
    let mut bounded_case = case.clone();
    bounded_case.limits.output_bytes = bounded.output_limit;
    let diagnostic = if measured.status == MeasuredStatus::Exited && measured.exit_code == Some(0) {
        format!(
            "reference correctness output exceeded {} bytes; candidate remains unverified",
            bounded.output_limit
        )
    } else {
        measured_diagnostic(&outcome, &measured)
    };
    Ok(row_with_diagnostic(
        &bounded_case,
        adapter,
        corpus,
        tier,
        &bounded,
        outcome,
        vec![BenchmarkSample::from(&measured)],
        Some(diagnostic),
    ))
}

const fn correctness_process_failure(
    status: ProcessStatus,
    error_class: Option<ErrorClass>,
) -> Option<BenchmarkOutcome> {
    match status {
        ProcessStatus::TimedOut => Some(BenchmarkOutcome::Timeout),
        ProcessStatus::Signaled => Some(BenchmarkOutcome::OomOrSignal),
        ProcessStatus::Exited => match error_class {
            Some(ErrorClass::Resource) => Some(BenchmarkOutcome::ResourceLimit),
            Some(ErrorClass::UnsupportedCapability) => Some(BenchmarkOutcome::Unsupported),
            _ => None,
        },
    }
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
    row_with_diagnostic(
        case,
        adapter,
        corpus,
        tier,
        invocation,
        BenchmarkOutcome::Unsupported,
        Vec::new(),
        Some(bound_message(
            adapter
                .unsupported_reason
                .clone()
                .unwrap_or_else(|| "adapter marked unsupported".to_owned()),
        )),
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
    row_with_diagnostic(
        case, adapter, corpus, tier, invocation, outcome, samples, None,
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "row construction keeps the serialized benchmark fields explicit"
)]
fn row_with_diagnostic(
    case: &BenchmarkCase,
    adapter: &BenchmarkAdapter,
    corpus: &BenchmarkCorpusIdentity,
    tier: DatasetTier,
    invocation: &BenchmarkInvocation,
    outcome: BenchmarkOutcome,
    samples: Vec<BenchmarkSample>,
    diagnostic: Option<String>,
) -> BenchmarkRow {
    let summary = (outcome == BenchmarkOutcome::Timed && !samples.is_empty())
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
        instrumented_samples: Vec::new(),
        summary,
        reference_ratios: BTreeMap::new(),
        reference_peak_rss_ratios: BTreeMap::new(),
        soft_performance_objective: None,
        diagnostic,
    }
}

const MAX_DIAGNOSTIC_BYTES: usize = 1024;

fn bounded_diagnostic(decision: &CorrectnessDecision) -> String {
    let message = match decision {
        CorrectnessDecision::Passed => "correctness passed".to_owned(),
        CorrectnessDecision::Incorrect(message) | CorrectnessDecision::Unnormalized(message) => {
            message.clone()
        }
    };
    bound_message(message)
}

fn bound_message(message: String) -> String {
    if message.len() <= MAX_DIAGNOSTIC_BYTES {
        return message;
    }
    let mut end = MAX_DIAGNOSTIC_BYTES - '…'.len_utf8();
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &message[..end])
}

fn process_diagnostic(observation: &CorrectnessObservation, measured: &MeasuredOutcome) -> String {
    let message = match measured.status {
        MeasuredStatus::Timeout => "process exceeded the benchmark timeout".to_owned(),
        MeasuredStatus::Signaled => measured.signal.map_or_else(
            || "process terminated by a signal".to_owned(),
            |signal| format!("process terminated by signal {signal}"),
        ),
        MeasuredStatus::OutputLimit => "process exceeded the benchmark output limit".to_owned(),
        MeasuredStatus::RssLimit => "process exceeded the benchmark RSS limit".to_owned(),
        MeasuredStatus::Exited => {
            let exit_status = measured
                .exit_code
                .map_or_else(|| "unknown".to_owned(), |code| code.to_string());
            match observation.error_class {
                Some(error_class) => format!(
                    "process exited with classified error {error_class:?} (exit status {exit_status})"
                ),
                None => format!("process exited with status {exit_status}"),
            }
        }
    };
    bound_message(message)
}

fn measured_diagnostic(outcome: &BenchmarkOutcome, measured: &MeasuredOutcome) -> String {
    let message = match outcome {
        BenchmarkOutcome::ResourceLimit
            if measured.output_bytes > 0 && measured.first_result_micros.is_none() =>
        {
            "stdout was produced but no first output timestamp was recorded".to_owned()
        }
        BenchmarkOutcome::ResourceLimit => "benchmark resource limit reached".to_owned(),
        BenchmarkOutcome::Timeout => "process exceeded the benchmark timeout".to_owned(),
        BenchmarkOutcome::OomOrSignal => "process terminated by a signal".to_owned(),
        BenchmarkOutcome::Incorrect => format!(
            "measured process exited with status {}",
            measured
                .exit_code
                .map_or_else(|| "unknown".to_owned(), |code| code.to_string())
        ),
        BenchmarkOutcome::Unsupported => "measured process outcome is unsupported".to_owned(),
        BenchmarkOutcome::Timed => "process completed".to_owned(),
    };
    bound_message(message)
}

fn failed_outcome(case: &BenchmarkCase, measured: &MeasuredOutcome) -> Option<BenchmarkOutcome> {
    let outcome = match measured.status {
        MeasuredStatus::Timeout => Some(BenchmarkOutcome::Timeout),
        MeasuredStatus::Signaled => Some(BenchmarkOutcome::OomOrSignal),
        MeasuredStatus::OutputLimit | MeasuredStatus::RssLimit => {
            Some(BenchmarkOutcome::ResourceLimit)
        }
        MeasuredStatus::Exited => {
            if measured.exit_code != Some(0) {
                return Some(BenchmarkOutcome::Incorrect);
            }
            if case.measure_first_result
                && measured.output_bytes > 0
                && measured.first_result_micros.is_none()
            {
                return Some(BenchmarkOutcome::ResourceLimit);
            }
            if let (Some(limit), Some(actual)) = (case.limits.rss_bytes, measured.peak_rss_bytes)
                && actual > limit
            {
                return Some(BenchmarkOutcome::ResourceLimit);
            }
            None
        }
    };
    if outcome.is_some() {
        if let Some(path) = &measured.stdout_path {
            eprintln!("benchmark stdout retained at {}", path.display());
        }
        if let Some(path) = &measured.stderr_path {
            eprintln!("benchmark stderr retained at {}", path.display());
        }
    }
    outcome
}

const fn tool_kind(adapter: &BenchmarkAdapter) -> ToolKind {
    match adapter.tool {
        super::BenchmarkTool::Jq => ToolKind::Jq,
        super::BenchmarkTool::Yq => ToolKind::Yq,
        super::BenchmarkTool::Tq => ToolKind::Tq,
    }
}

fn correctness_observation(
    captures: (&Path, &Path),
    tool: ToolKind,
    contract: OutputContractKind,
    measured: &MeasuredOutcome,
    args: &[String],
    expected: Option<&SemanticDigest>,
    cancellation: Option<&AtomicBool>,
) -> Result<CorrectnessObservation, BenchmarkRunnerError> {
    let (stdout_path, stderr_path) = captures;
    let stderr = std::fs::read(stderr_path)?;
    let process_status = process_status(measured.status);
    let metadata = ProcessOutcome {
        status: process_status,
        exit_code: measured.exit_code,
        signal: measured.signal,
        stdout: Vec::new(),
        stderr,
        wall_time_micros: measured.wall_time_micros,
        recorded_command: Vec::new(),
    };
    let error_class = (measured.status == MeasuredStatus::RssLimit)
        .then_some(ErrorClass::Resource)
        .or_else(|| classify_process(tool, &metadata));
    if cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
        return Err(BenchmarkRunnerError::Measure(MeasureError::Cancelled));
    }
    let payload = match contract {
        _ if measured.status == MeasuredStatus::RssLimit => empty_payload(contract),
        _ if error_class.is_some() => empty_payload(contract),
        OutputContractKind::SemanticSequence => CorrectnessPayload::SemanticSequence(
            digest_semantic_sequence(stdout_path, tool, args, expected, cancellation)?,
        ),
        OutputContractKind::ColoredSemanticSequence => CorrectnessPayload::SemanticSequence(
            digest_colored_json_sequence(stdout_path, tool, cancellation)?,
        ),
        OutputContractKind::RawBytes => CorrectnessPayload::RawBytes(std::fs::read(stdout_path)?),
        OutputContractKind::ExitOnly => CorrectnessPayload::ExitOnly,
    };
    Ok(CorrectnessObservation {
        payload,
        process_status,
        exit_code: measured.exit_code,
        error_class,
    })
}

fn empty_payload(contract: OutputContractKind) -> CorrectnessPayload {
    match contract {
        OutputContractKind::SemanticSequence | OutputContractKind::ColoredSemanticSequence => {
            CorrectnessPayload::SemanticSequence(SemanticDigester::default().finish())
        }
        OutputContractKind::RawBytes => CorrectnessPayload::RawBytes(Vec::new()),
        OutputContractKind::ExitOnly => CorrectnessPayload::ExitOnly,
    }
}

fn process_status(status: MeasuredStatus) -> ProcessStatus {
    match status {
        MeasuredStatus::Exited => ProcessStatus::Exited,
        MeasuredStatus::Timeout => ProcessStatus::TimedOut,
        MeasuredStatus::Signaled | MeasuredStatus::RssLimit => ProcessStatus::Signaled,
        MeasuredStatus::OutputLimit => unreachable!("output-limit returns before normalization"),
    }
}

fn digest_semantic_sequence(
    path: &Path,
    tool: ToolKind,
    args: &[String],
    expected: Option<&SemanticDigest>,
    cancellation: Option<&AtomicBool>,
) -> Result<SemanticDigest, DigestError> {
    match tool {
        ToolKind::Jq | ToolKind::Yq => {
            digest_json_sequence(path, tool, has_sequence_flag(args), cancellation)
        }
        ToolKind::Tq => match tq_output_mode(args) {
            TqOutputMode::Json { sequence } => {
                digest_json_sequence(path, tool, sequence, cancellation)
            }
            TqOutputMode::ToonSequence => digest_toon_sequence(path, cancellation),
            TqOutputMode::ToonValues => digest_toon_values(path, expected, cancellation),
            TqOutputMode::ToonUnframed => digest_toon_unframed_document(path, expected),
            TqOutputMode::Raw => Err(DigestError::Normalize(NormalizationError::Toon(
                "raw output cannot satisfy a semantic sequence contract".to_owned(),
            ))),
        },
    }
}

fn digest_colored_json_sequence(
    path: &Path,
    tool: ToolKind,
    cancellation: Option<&AtomicBool>,
) -> Result<SemanticDigest, DigestError> {
    check_cancelled(cancellation)?;
    let bytes =
        std::fs::read(path).map_err(|error| normalization_error(tool, error.to_string()))?;
    let bytes = strip_sgr(&bytes, tool)?;
    check_cancelled(cancellation)?;
    let mut digest = SemanticDigester::with_value_witness();
    let mut values = 0_u64;
    for value in serde_json::Deserializer::from_slice(&bytes).into_iter::<serde_json::Value>() {
        check_cancelled(cancellation)?;
        let value = value.map_err(|error| normalization_error(tool, error.to_string()))?;
        values += 1;
        push_digest(&mut digest, &value, tool)?;
    }
    if values == 0 {
        return Err(DigestError::Normalize(normalization_error(
            tool,
            "color output contains no JSON value".to_owned(),
        )));
    }
    Ok(digest.finish())
}

fn strip_sgr(bytes: &[u8], tool: ToolKind) -> Result<Vec<u8>, NormalizationError> {
    let mut stripped = Vec::with_capacity(bytes.len());
    let mut saw_sgr = false;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != 0x1b {
            stripped.push(bytes[index]);
            index += 1;
            continue;
        }
        if bytes.get(index + 1) != Some(&b'[') {
            return Err(normalization_error(
                tool,
                "color output contains a non-SGR ANSI escape".to_owned(),
            ));
        }
        let mut end = index + 2;
        while let Some(byte) = bytes.get(end) {
            if *byte == b'm' {
                break;
            }
            if !byte.is_ascii_digit() && *byte != b';' {
                return Err(normalization_error(
                    tool,
                    "color output contains a malformed SGR escape".to_owned(),
                ));
            }
            end += 1;
        }
        if bytes.get(end) != Some(&b'm') {
            return Err(normalization_error(
                tool,
                "color output contains an unterminated SGR escape".to_owned(),
            ));
        }
        saw_sgr = true;
        index = end + 1;
    }
    if !saw_sgr {
        return Err(normalization_error(
            tool,
            "color output contains no SGR escape".to_owned(),
        ));
    }
    Ok(stripped)
}

fn digest_json_sequence(
    path: &Path,
    tool: ToolKind,
    sequence: bool,
    cancellation: Option<&AtomicBool>,
) -> Result<SemanticDigest, DigestError> {
    if sequence {
        return digest_json_text_sequence(path, tool, cancellation);
    }
    let file = File::open(path).map_err(|error| normalization_error(tool, error.to_string()))?;
    let mut digest = SemanticDigester::with_value_witness();
    for value in serde_json::Deserializer::from_reader(BufReader::new(file)).into_iter() {
        check_cancelled(cancellation)?;
        let value = value.map_err(|error| normalization_error(tool, error.to_string()))?;
        push_digest(&mut digest, &value, tool)?;
    }
    Ok(digest.finish())
}

fn digest_json_text_sequence(
    path: &Path,
    tool: ToolKind,
    cancellation: Option<&AtomicBool>,
) -> Result<SemanticDigest, DigestError> {
    let file = File::open(path).map_err(|error| normalization_error(tool, error.to_string()))?;
    let mut reader = BufReader::new(file);
    let mut marker = [0_u8; 1];
    let mut digest = SemanticDigester::with_value_witness();
    loop {
        check_cancelled(cancellation)?;
        let read = reader
            .read(&mut marker)
            .map_err(|error| normalization_error(tool, error.to_string()))?;
        if read == 0 {
            break;
        }
        if marker[0] != 0x1e {
            return Err(DigestError::Normalize(normalization_error(
                tool,
                "JSON sequence record does not begin with RS".to_owned(),
            )));
        }
        let mut record = Vec::new();
        let bytes = reader
            .read_until(0x1e, &mut record)
            .map_err(|error| normalization_error(tool, error.to_string()))?;
        let has_next = record.last() == Some(&0x1e);
        if has_next {
            record.pop();
        }
        if record.pop() != Some(b'\n') {
            return Err(DigestError::Normalize(normalization_error(
                tool,
                "JSON sequence record does not end with LF".to_owned(),
            )));
        }
        if bytes == 0 || record.is_empty() {
            return Err(DigestError::Normalize(normalization_error(
                tool,
                "JSON sequence record is empty".to_owned(),
            )));
        }
        let value = serde_json::from_slice::<serde_json::Value>(&record)
            .map_err(|error| normalization_error(tool, error.to_string()))?;
        push_digest(&mut digest, &value, tool)?;
        if !has_next {
            break;
        }
    }
    Ok(digest.finish())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TqOutputMode {
    Json { sequence: bool },
    ToonSequence,
    ToonValues,
    ToonUnframed,
    Raw,
}

fn tq_output_mode(args: &[String]) -> TqOutputMode {
    let mut output = None;
    let mut framing = None;
    let mut raw = false;
    let mut compact = false;
    let mut arguments = args.iter();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--seq" => framing = Some(ToonFraming::Sequence),
            "--unframed" => framing = Some(ToonFraming::Unframed),
            "-r" | "--raw-output" | "--raw-output0" | "-j" | "--join-output" => raw = true,
            "-c" | "--compact-output" => compact = true,
            "-o" | "--output-format" => output = arguments.next().map(String::as_str),
            value if value.starts_with("--output-format=") => {
                output = value.split_once('=').map(|(_, value)| value);
            }
            value if value.starts_with("-o=") => {
                output = Some(&value[3..]);
            }
            value if value.starts_with("-o") && value.len() > 2 => {
                output = Some(&value[2..]);
            }
            _ => {}
        }
    }
    let output = output.map(|name| {
        tq_formats::NativeFormat::from_name(name).map_or(name, |format| format.descriptor().name)
    });
    if raw {
        return TqOutputMode::Raw;
    }
    if output == Some("json") {
        return TqOutputMode::Json {
            sequence: framing == Some(ToonFraming::Sequence),
        };
    }
    if matches!(output, Some("json-seq")) {
        return TqOutputMode::Json { sequence: true };
    }
    if matches!(output, Some("jsonl" | "ndjson")) {
        return TqOutputMode::Json { sequence: false };
    }
    if output.is_none() && compact {
        return TqOutputMode::Json {
            sequence: framing == Some(ToonFraming::Sequence),
        };
    }
    if output == Some("toon-seq") {
        return TqOutputMode::ToonSequence;
    }
    match framing {
        Some(ToonFraming::Sequence) => TqOutputMode::ToonSequence,
        Some(ToonFraming::Unframed) => TqOutputMode::ToonUnframed,
        None => TqOutputMode::ToonValues,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ToonFraming {
    Sequence,
    Unframed,
}

fn has_sequence_flag(args: &[String]) -> bool {
    args.iter().any(|argument| argument == "--seq")
}

fn digest_toon_sequence(
    path: &Path,
    cancellation: Option<&AtomicBool>,
) -> Result<SemanticDigest, DigestError> {
    check_cancelled(cancellation)?;
    let file =
        File::open(path).map_err(|error| NormalizationError::ToonSequence(error.to_string()))?;
    let mut reader = BufReader::new(file);
    let mut marker = [0_u8; 1];
    let mut digest = SemanticDigester::default();
    if reader
        .read(&mut marker)
        .map_err(|error| NormalizationError::ToonSequence(error.to_string()))?
        == 0
    {
        return Ok(digest.finish());
    }
    if marker[0] != 0x1e {
        return Err(DigestError::Normalize(NormalizationError::ToonSequence(
            "record does not begin with RS".to_owned(),
        )));
    }

    loop {
        check_cancelled(cancellation)?;
        let mut record = Vec::new();
        let bytes = reader
            .read_until(0x1e, &mut record)
            .map_err(|error| NormalizationError::ToonSequence(error.to_string()))?;
        if bytes == 0 {
            return Err(DigestError::Normalize(NormalizationError::ToonSequence(
                "sequence ends after RS without a record".to_owned(),
            )));
        }
        let has_next = record.last() == Some(&0x1e);
        if has_next {
            record.pop();
        }
        if record.pop() != Some(b'\n') {
            return Err(DigestError::Normalize(NormalizationError::ToonSequence(
                "record does not end with LF".to_owned(),
            )));
        }
        let mut documents = tq_formats::decode_toon(
            &record,
            "<tq-benchmark-output>",
            tq_toon::DecoderConfig::default(),
        )
        .map_err(|error| NormalizationError::ToonSequence(error.to_string()))?;
        if documents.len() != 1 {
            return Err(DigestError::Normalize(NormalizationError::ToonSequence(
                format!(
                    "record contains {} documents; expected exactly one",
                    documents.len()
                ),
            )));
        }
        let document = documents.pop().ok_or_else(|| {
            NormalizationError::ToonSequence("record contains no document".to_owned())
        })?;
        let value = document
            .value
            .to_json()
            .map_err(|error| NormalizationError::ToonSequence(error.to_string()))?;
        push_digest(&mut digest, &value, ToolKind::Tq)?;
        if !has_next {
            break;
        }
    }
    Ok(digest.finish())
}

fn digest_toon_unframed_document(
    path: &Path,
    expected: Option<&SemanticDigest>,
) -> Result<SemanticDigest, DigestError> {
    let stdout =
        std::fs::read(path).map_err(|error| NormalizationError::Toon(error.to_string()))?;
    if stdout.is_empty() {
        // Unframed TOON encodes one empty object as zero bytes. Accept it only
        // when the independent reference digest proves that exact one value.
        let mut empty_object = SemanticDigester::with_value_witness();
        push_digest(&mut empty_object, &serde_json::json!({}), ToolKind::Tq)?;
        let empty_object = empty_object.finish();
        if expected.is_some_and(|expected| expected == &empty_object) {
            return Ok(empty_object);
        }
        return Err(DigestError::Normalize(NormalizationError::Toon(
            "unframed TOON output requires exactly one result".to_owned(),
        )));
    }
    digest_toon_document_bytes(&stdout)
}

fn digest_toon_document_bytes(stdout: &[u8]) -> Result<SemanticDigest, DigestError> {
    if stdout.is_empty() {
        return Ok(SemanticDigester::default().finish());
    }
    let value = decode_toon_record(stdout, NormalizationError::Toon)?;
    let mut digest = SemanticDigester::default();
    push_digest(&mut digest, &value, ToolKind::Tq)?;
    Ok(digest.finish())
}

#[allow(
    clippy::too_many_lines,
    reason = "the bounded streaming fallback is kept together with its invariants"
)]
fn digest_toon_values(
    path: &Path,
    expected: Option<&SemanticDigest>,
    cancellation: Option<&AtomicBool>,
) -> Result<SemanticDigest, DigestError> {
    check_cancelled(cancellation)?;
    let Some(expected) = expected else {
        return digest_toon_document_with_witness(path);
    };
    if expected.result_count > 1 && expected.witness.is_none() {
        return Err(DigestError::Normalize(NormalizationError::Toon(
            "default TOON values require a reference boundary witness".to_owned(),
        )));
    }
    if expected.result_count <= 1 {
        if !path_is_empty(path)? && !file_ends_with_lf(path)? {
            return Err(DigestError::Normalize(NormalizationError::Toon(
                "default TOON values require a terminal LF".to_owned(),
            )));
        }
        let actual = digest_toon_document_with_witness(path)?;
        let expected_first = if expected.result_count == 1 {
            let witness = expected.witness.as_ref().ok_or_else(|| {
                DigestError::Normalize(NormalizationError::Toon(
                    "default TOON values require a reference boundary witness".to_owned(),
                ))
            })?;
            let mut reader = witness.reader()?;
            Some(SemanticWitness::read_record(&mut reader)?.0)
        } else {
            None
        };
        let actual_first = if let Some(witness) = actual.witness.as_ref() {
            let mut reader = witness.reader()?;
            Some(SemanticWitness::read_record(&mut reader)?.0)
        } else {
            None
        };
        if actual.result_count != expected.result_count || actual_first != expected_first {
            return Err(DigestError::Normalize(NormalizationError::Toon(
                "default TOON value does not match the reference sequence".to_owned(),
            )));
        }
        return Ok(actual);
    }

    let expected_witness = expected.witness.as_ref().expect("checked above");
    let mut expected_reader = expected_witness.reader()?;
    let file = File::open(path).map_err(|error| NormalizationError::Toon(error.to_string()))?;
    let mut reader = BufReader::new(file);
    let mut digest = SemanticDigester::default();
    for _ in 0..expected.result_count {
        check_cancelled(cancellation)?;
        let (expected_value, line_count) = SemanticWitness::read_record(&mut expected_reader)?;
        let mut record = Vec::new();
        let hinted = read_lines(&mut reader, line_count, &mut record)?;
        let hinted_value = hinted
            .then(|| decode_toon_record(&record, NormalizationError::Toon).ok())
            .flatten()
            .filter(|value| value_digest(value).is_ok_and(|digest| digest == expected_value));
        if let Some(value) = hinted_value {
            push_digest(&mut digest, &value, ToolKind::Tq)?;
            continue;
        }
        if record.len() > MAX_TOON_BOUNDARY_FALLBACK_BYTES {
            return Err(DigestError::Normalize(NormalizationError::Toon(
                "default TOON boundary hint did not match within fallback bound".to_owned(),
            )));
        }
        let mut matched = false;
        let mut scanned_len = 0;
        let mut decode_attempts = 0;
        loop {
            check_cancelled(cancellation)?;
            if let Some((value, consumed_len)) = matching_toon_prefix(
                &record,
                expected_value,
                &mut scanned_len,
                &mut decode_attempts,
            )? {
                let unread = i64::try_from(record.len().saturating_sub(consumed_len))
                    .map_err(|error| NormalizationError::Toon(error.to_string()))?;
                if unread > 0 {
                    reader
                        .seek_relative(-unread)
                        .map_err(|error| NormalizationError::Toon(error.to_string()))?;
                }
                push_digest(&mut digest, &value, ToolKind::Tq)?;
                matched = true;
                break;
            }
            let bytes = reader
                .read_until(b'\n', &mut record)
                .map_err(|error| NormalizationError::Toon(error.to_string()))?;
            if bytes == 0 {
                break;
            }
            if record.len() > MAX_TOON_BOUNDARY_FALLBACK_BYTES {
                break;
            }
        }
        if !matched {
            return Err(DigestError::Normalize(NormalizationError::Toon(
                "default TOON values do not match the reference sequence".to_owned(),
            )));
        }
    }
    let mut trailing = [0_u8; 1];
    if reader
        .read(&mut trailing)
        .map_err(|error| NormalizationError::Toon(error.to_string()))?
        != 0
    {
        return Err(DigestError::Normalize(NormalizationError::Toon(
            "default TOON output contains extra bytes after the reference sequence".to_owned(),
        )));
    }
    Ok(digest.finish())
}

const MAX_TOON_BOUNDARY_FALLBACK_BYTES: usize = 64 * 1024;
const MAX_TOON_BOUNDARY_DECODE_ATTEMPTS: usize = 256;

fn path_is_empty(path: &Path) -> Result<bool, NormalizationError> {
    Ok(std::fs::metadata(path)
        .map_err(|error| NormalizationError::Toon(error.to_string()))?
        .len()
        == 0)
}

fn file_ends_with_lf(path: &Path) -> Result<bool, NormalizationError> {
    let mut file = File::open(path).map_err(|error| NormalizationError::Toon(error.to_string()))?;
    let length = file
        .metadata()
        .map_err(|error| NormalizationError::Toon(error.to_string()))?
        .len();
    if length == 0 {
        return Ok(false);
    }
    file.seek(std::io::SeekFrom::End(-1))
        .map_err(|error| NormalizationError::Toon(error.to_string()))?;
    let mut last = [0_u8; 1];
    file.read_exact(&mut last)
        .map_err(|error| NormalizationError::Toon(error.to_string()))?;
    Ok(last[0] == b'\n')
}

fn read_lines(
    reader: &mut BufReader<File>,
    line_count: u64,
    record: &mut Vec<u8>,
) -> Result<bool, NormalizationError> {
    for _ in 0..line_count {
        let before = record.len();
        reader
            .read_until(b'\n', record)
            .map_err(|error| NormalizationError::Toon(error.to_string()))?;
        if record.len() == before || record.last() != Some(&b'\n') {
            return Ok(false);
        }
    }
    Ok(true)
}

fn matching_toon_prefix(
    record: &[u8],
    expected: [u8; 32],
    scanned_len: &mut usize,
    decode_attempts: &mut usize,
) -> Result<Option<(serde_json::Value, usize)>, NormalizationError> {
    for index in *scanned_len..record.len() {
        if record[index] != b'\n' {
            continue;
        }
        *decode_attempts = (*decode_attempts).saturating_add(1);
        if *decode_attempts > MAX_TOON_BOUNDARY_DECODE_ATTEMPTS {
            return Err(NormalizationError::Toon(
                "default TOON boundary fallback exceeded decode-attempt bound".to_owned(),
            ));
        }
        let Ok(value) = decode_toon_record(&record[..=index], NormalizationError::Toon) else {
            continue;
        };
        if value_digest(&value).map_err(|error| NormalizationError::Toon(error.to_string()))?
            == expected
        {
            return Ok(Some((value, index + 1)));
        }
    }
    *scanned_len = record.len();
    Ok(None)
}

fn digest_toon_document_with_witness(path: &Path) -> Result<SemanticDigest, DigestError> {
    let stdout =
        std::fs::read(path).map_err(|error| NormalizationError::Toon(error.to_string()))?;
    if stdout.is_empty() {
        return Ok(SemanticDigester::with_value_witness().finish());
    }
    let value = decode_toon_record(&stdout, NormalizationError::Toon)?;
    let mut digest = SemanticDigester::with_value_witness();
    push_digest(&mut digest, &value, ToolKind::Tq)?;
    Ok(digest.finish())
}

fn decode_toon_record(
    record: &[u8],
    error: fn(String) -> NormalizationError,
) -> Result<serde_json::Value, NormalizationError> {
    let mut documents = tq_formats::decode_toon(
        record,
        "<tq-benchmark-output>",
        tq_toon::DecoderConfig::default(),
    )
    .map_err(|decode| error(decode.to_string()))?;
    if documents.len() != 1 {
        return Err(error(format!(
            "record contains {} documents; expected exactly one",
            documents.len()
        )));
    }
    let document = documents
        .pop()
        .ok_or_else(|| error("record contains no document".to_owned()))?;
    document
        .value
        .to_json()
        .map_err(|decode| error(decode.to_string()))
}

fn normalization_error(tool: ToolKind, message: String) -> NormalizationError {
    match tool {
        ToolKind::Jq => NormalizationError::Jq(message),
        ToolKind::Yq => NormalizationError::Yq(message),
        ToolKind::Tq => NormalizationError::ToonSequence(message),
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

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, time::Duration};

    use serde_json::json;

    use super::{
        TqOutputMode, correctness_observation, digest_toon_values,
        is_unverified_resource_observation, tq_output_mode,
    };
    use crate::benchmark::correctness::semantic_digest;
    use crate::benchmark::{
        BenchmarkAdapter, BenchmarkCase, BenchmarkCorpusIdentity, BenchmarkInvocation,
        BenchmarkLimits, BenchmarkSampling, BenchmarkTool, ComparisonFamily, DatasetFamily,
        DatasetSelector, DatasetTier, ExecutionClass, InputFormat, MeasuredOutcome, MeasuredStatus,
        MeasurementProtocol, OutputContract, OutputContractKind, RssProvenance,
    };
    use crate::compatibility::{NormalizationError, ProcessStatus};
    use crate::corpus::ArtifactIdentity;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn tq_output_mode_matches_json_aliases_and_compact_cli_precedence() {
        assert_eq!(
            tq_output_mode(&args(&["-c"])),
            TqOutputMode::Json { sequence: false }
        );
        assert_eq!(
            tq_output_mode(&args(&["--compact-output", "--seq"])),
            TqOutputMode::Json { sequence: true }
        );
        assert_eq!(
            tq_output_mode(&args(&["--seq", "-c"])),
            TqOutputMode::Json { sequence: true }
        );
        for arguments in [
            &["-c", "-o", "json"][..],
            &["-o", "json", "-c"][..],
            &["-o=json", "-c"][..],
        ] {
            assert_eq!(
                tq_output_mode(&args(arguments)),
                TqOutputMode::Json { sequence: false }
            );
        }
        for arguments in [
            &["-o", "json-seq"][..],
            &["--output-format=json-seq"][..],
            &["-ojson-seq"][..],
        ] {
            assert_eq!(
                tq_output_mode(&args(arguments)),
                TqOutputMode::Json { sequence: true }
            );
        }
        for arguments in [
            &["-o", "jsonseq"][..],
            &["--output-format=jsonseq"][..],
            &["-ojsonseq"][..],
        ] {
            assert_eq!(
                tq_output_mode(&args(arguments)),
                TqOutputMode::Json { sequence: true }
            );
        }
        for alias in ["jsonl", "ndjson"] {
            for arguments in [
                vec!["-o".to_owned(), alias.to_owned(), "--seq".to_owned()],
                vec![format!("--output-format={alias}"), "--seq".to_owned()],
            ] {
                assert_eq!(
                    tq_output_mode(&arguments),
                    TqOutputMode::Json { sequence: false }
                );
            }
        }
        for arguments in [&["-c", "-o", "toon"][..], &["-o", "toon", "-c"][..]] {
            assert_eq!(tq_output_mode(&args(arguments)), TqOutputMode::ToonValues);
        }
        assert_eq!(
            tq_output_mode(&args(&["-o", "toon-seq"])),
            TqOutputMode::ToonSequence
        );
        for arguments in [
            &["-o", "toon-sequence"][..],
            &["--output-format=toon-sequence"][..],
            &["-otoon-sequence"][..],
        ] {
            assert_eq!(tq_output_mode(&args(arguments)), TqOutputMode::ToonSequence);
        }
    }

    #[test]
    fn toon_frame_rejects_multiple_documents_in_one_record() {
        let error = super::decode_toon_record(b"1\n2\n", NormalizationError::Toon)
            .expect_err("one frame must contain one document");
        assert!(matches!(error, NormalizationError::Toon(_)));
    }

    #[test]
    fn rss_limited_reference_is_marked_unverified_resource() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let stdout = directory.path().join("stdout");
        let stderr = directory.path().join("stderr");
        fs::write(&stdout, b"not a complete result").expect("stdout capture");
        fs::write(&stderr, []).expect("stderr capture");
        let measured = MeasuredOutcome {
            status: MeasuredStatus::RssLimit,
            exit_code: Some(0),
            signal: None,
            wall_time_micros: 1,
            first_result_micros: None,
            user_cpu_micros: Some(1),
            system_cpu_micros: Some(1),
            peak_rss_bytes: Some(2),
            rss_provenance: RssProvenance::DarwinWait4,
            measurement_protocol: MeasurementProtocol {
                timing_method: "test".to_owned(),
                input_delivery: "test".to_owned(),
                rss_scope: "test".to_owned(),
                exit_poll_interval_micros: 1,
                rss_poll_interval_micros: None,
                validated_accuracy_micros: None,
                worker: None,
                isolation_evidence: None,
            },
            process_group_peak_rss_bytes: None,
            output_bytes: 1,
            stderr_bytes: 0,
            stdout_path: None,
            stderr_path: None,
            process_group_rss_observed: false,
        };
        let observation = correctness_observation(
            (&stdout, &stderr),
            crate::compatibility::ToolKind::Jq,
            crate::benchmark::OutputContractKind::SemanticSequence,
            &measured,
            &[],
            None,
            None,
        )
        .expect("RSS-limited observation");

        assert_eq!(observation.process_status, ProcessStatus::Signaled);
        assert_eq!(
            observation.error_class,
            Some(crate::compatibility::ErrorClass::Resource)
        );
        assert!(is_unverified_resource_observation(&observation));
        assert_eq!(
            observation.payload,
            super::empty_payload(crate::benchmark::OutputContractKind::SemanticSequence)
        );
        assert_reference_resource_gate(&observation);
    }

    fn assert_reference_resource_gate(observation: &super::CorrectnessObservation) {
        let case = rss_limit_case();
        let adapter = BenchmarkAdapter {
            id: "jq-json".to_owned(),
            tool: BenchmarkTool::Jq,
            input_format: InputFormat::Json,
            applicable: true,
            unsupported_reason: None,
            args: Vec::new(),
            query: None,
            comparison_families: vec![ComparisonFamily::SameFormat],
        };
        let corpus = BenchmarkCorpusIdentity {
            origin: "test".to_owned(),
            source_id: "source".to_owned(),
            tier: "startup".to_owned(),
            format: InputFormat::Json,
            artifact: ArtifactIdentity {
                path: "inline".to_owned(),
                bytes: 1,
                sha256: "a".repeat(64),
            },
            logical_records: 1,
            manifest_sha256: "b".repeat(64),
        };
        let invocation = BenchmarkInvocation {
            cancellation: None,
            executable: PathBuf::from("unused"),
            args: Vec::new(),
            stdin: Vec::new(),
            current_dir: None,
            timeout: Duration::from_secs(1),
            output_limit: 1024,
            rss_limit: Some(1),
            retain_output: false,
        };
        let row = super::correctness_gate_row(
            &case,
            &adapter,
            &corpus,
            DatasetTier::Startup,
            &invocation,
            observation,
        )
        .expect("reference resource row")
        .expect("reference resource row is retained");
        assert_eq!(
            row.outcome,
            crate::benchmark::BenchmarkOutcome::ResourceLimit
        );
        assert!(row.samples.is_empty());
        assert!(
            row.diagnostic
                .as_deref()
                .is_some_and(|message| message.contains("reference correctness"))
        );
    }

    fn rss_limit_case() -> BenchmarkCase {
        BenchmarkCase {
            schema_version: 1,
            id: "benchmark.test".to_owned(),
            compatibility_gate: "test".to_owned(),
            dataset_selector: DatasetSelector {
                family: DatasetFamily::SyntheticHelper,
                tiers: vec![DatasetTier::Startup],
            },
            query: ".".to_owned(),
            execution_class: ExecutionClass::Startup,
            measure_first_result: false,
            sampling: BenchmarkSampling {
                warmups: 0,
                small: 1,
                medium: 1,
                large: 1,
            },
            timeout_seconds: 1,
            limits: BenchmarkLimits {
                output_bytes: 1024,
                rss_bytes: Some(1),
            },
            output_contract: OutputContract {
                kind: OutputContractKind::SemanticSequence,
                reference_adapter: "jq-json".to_owned(),
            },
            adapters: Vec::new(),
        }
    }

    #[test]
    fn fallback_rewinds_unread_bytes_before_next_result() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("output.toon");
        fs::write(&path, b"a: 1\n2\n").expect("TOON output");

        let values = [json!({"a": 1}), json!(2)];
        let expected = semantic_digest(values.iter()).expect("semantic digest");

        let actual = digest_toon_values(&path, Some(&expected), None).expect("normalized values");

        assert_eq!(actual, expected);
        assert_eq!(actual.result_count, 2);
    }
}
