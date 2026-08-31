//! Correctness-first warmup and sampling loop.

use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead as _, BufReader, Read as _},
    path::Path,
};

use thiserror::Error;

use super::{
    BenchmarkAdapter, BenchmarkCase, BenchmarkCorpusIdentity, BenchmarkInvocation,
    BenchmarkOutcome, BenchmarkRow, BenchmarkSample, CorrectnessDecision, CorrectnessObservation,
    CorrectnessPayload, DatasetTier, MeasuredOutcome, MeasuredStatus, OutputContractKind,
    SemanticDigest, SemanticDigester, correctness_gate, measure_process, summarize_samples,
};
use crate::compatibility::{
    NormalizationError, ProcessError, ProcessOutcome, ProcessStatus, ToolKind, classify_process,
};

const MAX_CORRECTNESS_OUTPUT_BYTES: u64 = 32 * 1024 * 1024;

/// Returns whether a runner error represents the bounded correctness capture
/// limit rather than an infrastructure failure.
#[must_use]
pub const fn is_correctness_output_limit(error: &BenchmarkRunnerError) -> bool {
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
    #[error("correctness output exceeded {limit} bytes; stdout: {stdout}; stderr: {stderr}")]
    CorrectnessOutputLimit {
        /// Maximum allowed stdout bytes.
        limit: u64,
        /// Retained stdout file.
        stdout: std::path::PathBuf,
        /// Retained stderr file.
        stderr: std::path::PathBuf,
    },
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
    measured_correctness_run(invocation, tool, contract).map(|(observation, _)| observation)
}

fn measured_correctness_run(
    invocation: &BenchmarkInvocation,
    tool: ToolKind,
    contract: OutputContractKind,
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
    let normalized = correctness_observation(&stdout_path, &stderr_path, tool, contract, &measured);
    let _ = std::fs::remove_file(stdout_path);
    let _ = std::fs::remove_file(stderr_path);
    Ok((normalized?, measured))
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
) -> Result<BenchmarkRow, BenchmarkRunnerError> {
    let tool = tool_kind(adapter);
    let candidate = measured_correctness_run(invocation, tool, case.output_contract.kind);
    if let Ok((observation, measured)) = &candidate {
        let process_failure = if measured.status == MeasuredStatus::RssLimit {
            Some(BenchmarkOutcome::ResourceLimit)
        } else {
            correctness_process_failure(observation.process_status)
        };
        if let Some(outcome) = process_failure {
            return Ok(row(
                case,
                adapter,
                corpus,
                tier,
                invocation,
                outcome,
                vec![BenchmarkSample::from(measured)],
            ));
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
            samples.push(BenchmarkSample::from(&measured));
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
        MeasuredStatus::Exited => BenchmarkOutcome::Incorrect,
    };
    let mut bounded_case = case.clone();
    bounded_case.limits.output_bytes = bounded.output_limit;
    Ok(row(
        &bounded_case,
        adapter,
        corpus,
        tier,
        &bounded,
        outcome,
        vec![BenchmarkSample::from(&measured)],
    ))
}

const fn correctness_process_failure(status: ProcessStatus) -> Option<BenchmarkOutcome> {
    match status {
        ProcessStatus::Exited => None,
        ProcessStatus::TimedOut => Some(BenchmarkOutcome::Timeout),
        ProcessStatus::Signaled => Some(BenchmarkOutcome::OomOrSignal),
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
    let summary = (!samples.is_empty())
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
        reference_peak_rss_ratios: BTreeMap::new(),
        soft_performance_objective: None,
    }
}

fn failed_outcome(case: &BenchmarkCase, measured: &MeasuredOutcome) -> Option<BenchmarkOutcome> {
    let outcome = match measured.status {
        MeasuredStatus::Timeout => Some(BenchmarkOutcome::Timeout),
        MeasuredStatus::Signaled => Some(BenchmarkOutcome::OomOrSignal),
        MeasuredStatus::OutputLimit | MeasuredStatus::RssLimit => {
            Some(BenchmarkOutcome::ResourceLimit)
        }
        MeasuredStatus::Exited => {
            if case.measure_first_result && measured.first_result_micros.is_none() {
                return Some(BenchmarkOutcome::ResourceLimit);
            }
            if let (Some(limit), Some(actual)) = (case.limits.rss_bytes, measured.peak_rss_bytes) {
                if actual > limit {
                    return Some(BenchmarkOutcome::ResourceLimit);
                }
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
    stdout_path: &Path,
    stderr_path: &Path,
    tool: ToolKind,
    contract: OutputContractKind,
    measured: &MeasuredOutcome,
) -> Result<CorrectnessObservation, BenchmarkRunnerError> {
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
    let payload = match contract {
        OutputContractKind::SemanticSequence => {
            CorrectnessPayload::SemanticSequence(digest_semantic_sequence(stdout_path, tool)?)
        }
        OutputContractKind::RawBytes => CorrectnessPayload::RawBytes(std::fs::read(stdout_path)?),
        OutputContractKind::ExitOnly => CorrectnessPayload::ExitOnly,
    };
    Ok(CorrectnessObservation {
        payload,
        process_status,
        exit_code: measured.exit_code,
        error_class: classify_process(tool, &metadata),
    })
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
) -> Result<SemanticDigest, NormalizationError> {
    match tool {
        ToolKind::Jq | ToolKind::Yq => digest_json_sequence(path, tool),
        ToolKind::Tq => digest_toon_sequence(path),
    }
}

fn digest_json_sequence(path: &Path, tool: ToolKind) -> Result<SemanticDigest, NormalizationError> {
    let file = File::open(path).map_err(|error| normalization_error(tool, error.to_string()))?;
    let mut digest = SemanticDigester::default();
    for value in serde_json::Deserializer::from_reader(BufReader::new(file)).into_iter() {
        let value = value.map_err(|error| normalization_error(tool, error.to_string()))?;
        digest
            .push(&value)
            .map_err(|error| normalization_error(tool, error.to_string()))?;
    }
    Ok(digest.finish())
}

fn digest_toon_sequence(path: &Path) -> Result<SemanticDigest, NormalizationError> {
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
        return Err(NormalizationError::ToonSequence(
            "record does not begin with RS".to_owned(),
        ));
    }

    loop {
        let mut record = Vec::new();
        let bytes = reader
            .read_until(0x1e, &mut record)
            .map_err(|error| NormalizationError::ToonSequence(error.to_string()))?;
        if bytes == 0 {
            return Err(NormalizationError::ToonSequence(
                "sequence ends after RS without a record".to_owned(),
            ));
        }
        let has_next = record.last() == Some(&0x1e);
        if has_next {
            record.pop();
        }
        if record.pop() != Some(b'\n') {
            return Err(NormalizationError::ToonSequence(
                "record does not end with LF".to_owned(),
            ));
        }
        let mut documents = tq_formats::decode_toon(
            &record,
            "<tq-benchmark-output>",
            tq_toon::DecoderConfig::default(),
        )
        .map_err(|error| NormalizationError::ToonSequence(error.to_string()))?;
        let document = documents.pop().ok_or_else(|| {
            NormalizationError::ToonSequence("record contains no document".to_owned())
        })?;
        let value = document
            .value
            .to_json()
            .map_err(|error| NormalizationError::ToonSequence(error.to_string()))?;
        digest
            .push(&value)
            .map_err(|error| NormalizationError::ToonSequence(error.to_string()))?;
        if !has_next {
            break;
        }
    }
    Ok(digest.finish())
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
