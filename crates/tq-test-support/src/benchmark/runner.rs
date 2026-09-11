//! Correctness-first warmup and sampling loop.

use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead as _, BufReader, Read as _, Seek as _},
    path::Path,
};

use thiserror::Error;

use super::correctness::value_digest;
use super::{
    BenchmarkAdapter, BenchmarkCase, BenchmarkCorpusIdentity, BenchmarkInvocation,
    BenchmarkOutcome, BenchmarkRow, BenchmarkSample, CorrectnessDecision, CorrectnessObservation,
    CorrectnessPayload, DatasetTier, MeasuredOutcome, MeasuredStatus, OutputContractKind,
    SemanticDigest, SemanticDigester, correctness_gate, measure_process,
    measure_process_uninstrumented, summarize_samples,
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

type CandidateResult = Result<(CorrectnessObservation, MeasuredOutcome), BenchmarkRunnerError>;

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
        return Err(BenchmarkRunnerError::CorrectnessOutputLimit {
            limit: correctness.output_limit,
            stdout: retained_stdout,
            stderr: retained_stderr,
        });
    }
    let normalized = correctness_observation(
        &stdout_path,
        &stderr_path,
        tool,
        contract,
        &measured,
        &invocation.args,
        expected,
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
            return Ok(Some(row_with_diagnostic(
                case,
                adapter,
                corpus,
                tier,
                invocation,
                outcome,
                vec![BenchmarkSample::from(measured)],
                Some(process_diagnostic(observation, measured)),
            )));
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
    if let Some(row) = correctness_gate_row(case, adapter, corpus, tier, invocation, reference)? {
        return Ok(row);
    }

    for _ in 0..case.sampling.warmups {
        let warmup = measure_process(invocation)?;
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
        // First enforce the live limit in a separate repetition. Only then
        // time the equivalent command without sampler interference. Native
        // peak RSS still checks the limit at the end of the timing repetition.
        if invocation.rss_limit.is_some() {
            let instrumented = measure_process(invocation)?;
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
        let measured = measure_process_uninstrumented(invocation)?;
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
    let (instrumented_samples, samples): (Vec<_>, Vec<_>) =
        samples.into_iter().partition(|sample| {
            sample
                .measurement_protocol
                .as_ref()
                .is_some_and(|protocol| protocol.rss_poll_interval_micros.is_some())
        });
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
        instrumented_samples,
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
    stdout_path: &Path,
    stderr_path: &Path,
    tool: ToolKind,
    contract: OutputContractKind,
    measured: &MeasuredOutcome,
    args: &[String],
    expected: Option<&SemanticDigest>,
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
    let error_class = classify_process(tool, &metadata);
    let payload = match contract {
        _ if error_class.is_some() => empty_payload(contract),
        OutputContractKind::SemanticSequence => CorrectnessPayload::SemanticSequence(
            digest_semantic_sequence(stdout_path, tool, args, expected)?,
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
        OutputContractKind::SemanticSequence => {
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
) -> Result<SemanticDigest, NormalizationError> {
    match tool {
        ToolKind::Jq | ToolKind::Yq => digest_json_sequence(path, tool, has_sequence_flag(args)),
        ToolKind::Tq => match tq_output_mode(args) {
            TqOutputMode::Json { sequence } => digest_json_sequence(path, tool, sequence),
            TqOutputMode::ToonSequence => digest_toon_sequence(path),
            TqOutputMode::ToonValues => digest_toon_values(path, expected),
            TqOutputMode::ToonUnframed => digest_toon_document(path),
            TqOutputMode::Raw => Err(NormalizationError::Toon(
                "raw output cannot satisfy a semantic sequence contract".to_owned(),
            )),
        },
    }
}

fn digest_json_sequence(
    path: &Path,
    tool: ToolKind,
    sequence: bool,
) -> Result<SemanticDigest, NormalizationError> {
    if sequence {
        return digest_json_text_sequence(path, tool);
    }
    let file = File::open(path).map_err(|error| normalization_error(tool, error.to_string()))?;
    let mut digest = matches!(tool, ToolKind::Jq | ToolKind::Yq)
        .then(SemanticDigester::with_value_witness)
        .unwrap_or_default();
    for value in serde_json::Deserializer::from_reader(BufReader::new(file)).into_iter() {
        let value = value.map_err(|error| normalization_error(tool, error.to_string()))?;
        digest
            .push(&value)
            .map_err(|error| normalization_error(tool, error.to_string()))?;
    }
    Ok(digest.finish())
}

fn digest_json_text_sequence(
    path: &Path,
    tool: ToolKind,
) -> Result<SemanticDigest, NormalizationError> {
    let file = File::open(path).map_err(|error| normalization_error(tool, error.to_string()))?;
    let mut reader = BufReader::new(file);
    let mut marker = [0_u8; 1];
    let mut digest = if matches!(tool, ToolKind::Jq | ToolKind::Yq) {
        SemanticDigester::with_value_witness()
    } else {
        SemanticDigester::default()
    };
    loop {
        let read = reader
            .read(&mut marker)
            .map_err(|error| normalization_error(tool, error.to_string()))?;
        if read == 0 {
            break;
        }
        if marker[0] != 0x1e {
            return Err(normalization_error(
                tool,
                "JSON sequence record does not begin with RS".to_owned(),
            ));
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
            return Err(normalization_error(
                tool,
                "JSON sequence record does not end with LF".to_owned(),
            ));
        }
        if bytes == 0 || record.is_empty() {
            return Err(normalization_error(
                tool,
                "JSON sequence record is empty".to_owned(),
            ));
        }
        let value = serde_json::from_slice::<serde_json::Value>(&record)
            .map_err(|error| normalization_error(tool, error.to_string()))?;
        digest
            .push(&value)
            .map_err(|error| normalization_error(tool, error.to_string()))?;
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
    let mut arguments = args.iter();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--seq" => framing = Some(ToonFraming::Sequence),
            "--unframed" => framing = Some(ToonFraming::Unframed),
            "-r" | "--raw-output" | "--raw-output0" | "-j" | "--join-output" => raw = true,
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
    if raw {
        return TqOutputMode::Raw;
    }
    if output == Some("json") {
        return TqOutputMode::Json {
            sequence: framing == Some(ToonFraming::Sequence),
        };
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

fn digest_toon_document(path: &Path) -> Result<SemanticDigest, NormalizationError> {
    let stdout =
        std::fs::read(path).map_err(|error| NormalizationError::Toon(error.to_string()))?;
    if stdout.is_empty() {
        return Ok(SemanticDigester::default().finish());
    }
    let value = decode_toon_record(&stdout, NormalizationError::Toon)?;
    let mut digest = SemanticDigester::default();
    digest
        .push(&value)
        .map_err(|error| NormalizationError::Toon(error.to_string()))?;
    Ok(digest.finish())
}

#[allow(
    clippy::too_many_lines,
    reason = "the bounded streaming fallback is kept together with its invariants"
)]
fn digest_toon_values(
    path: &Path,
    expected: Option<&SemanticDigest>,
) -> Result<SemanticDigest, NormalizationError> {
    let Some(expected) = expected else {
        return digest_toon_document(path);
    };
    let Some(expected_values) = expected.value_digests.as_deref() else {
        return Err(NormalizationError::Toon(
            "default TOON values require a reference boundary witness".to_owned(),
        ));
    };
    if expected_values.len() <= 1 {
        if !path_is_empty(path)? && !file_ends_with_lf(path)? {
            return Err(NormalizationError::Toon(
                "default TOON values require a terminal LF".to_owned(),
            ));
        }
        let actual = digest_toon_document_with_witness(path)?;
        if actual.result_count != expected.result_count
            || actual
                .value_digests
                .as_deref()
                .and_then(|values| values.first())
                != expected_values.first()
        {
            return Err(NormalizationError::Toon(
                "default TOON value does not match the reference sequence".to_owned(),
            ));
        }
        return Ok(actual);
    }

    let line_counts = expected.toon_line_counts.as_deref().ok_or_else(|| {
        NormalizationError::Toon(
            "default TOON values require a reference line-count witness".to_owned(),
        )
    })?;
    if line_counts.len() != expected_values.len() {
        return Err(NormalizationError::Toon(
            "default TOON reference witnesses have different lengths".to_owned(),
        ));
    }
    let file = File::open(path).map_err(|error| NormalizationError::Toon(error.to_string()))?;
    let mut reader = BufReader::new(file);
    let mut digest = SemanticDigester::default();
    for (expected_value, line_count) in expected_values.iter().zip(line_counts) {
        let mut record = Vec::new();
        let hinted = read_lines(&mut reader, *line_count, &mut record)?;
        let hinted_value = hinted
            .then(|| decode_toon_record(&record, NormalizationError::Toon).ok())
            .flatten()
            .filter(|value| value_digest(value).is_ok_and(|digest| digest == *expected_value));
        if let Some(value) = hinted_value {
            digest
                .push(&value)
                .map_err(|error| NormalizationError::Toon(error.to_string()))?;
            continue;
        }
        if record.len() > MAX_TOON_BOUNDARY_FALLBACK_BYTES {
            return Err(NormalizationError::Toon(
                "default TOON boundary hint did not match within fallback bound".to_owned(),
            ));
        }
        let mut matched = false;
        let mut scanned_len = 0;
        let mut decode_attempts = 0;
        loop {
            if let Some((value, consumed_len)) = matching_toon_prefix(
                &record,
                *expected_value,
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
                digest
                    .push(&value)
                    .map_err(|error| NormalizationError::Toon(error.to_string()))?;
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
            return Err(NormalizationError::Toon(
                "default TOON values do not match the reference sequence".to_owned(),
            ));
        }
    }
    let mut trailing = [0_u8; 1];
    if reader
        .read(&mut trailing)
        .map_err(|error| NormalizationError::Toon(error.to_string()))?
        != 0
    {
        return Err(NormalizationError::Toon(
            "default TOON output contains extra bytes after the reference sequence".to_owned(),
        ));
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

fn digest_toon_document_with_witness(path: &Path) -> Result<SemanticDigest, NormalizationError> {
    let stdout =
        std::fs::read(path).map_err(|error| NormalizationError::Toon(error.to_string()))?;
    if stdout.is_empty() {
        return Ok(SemanticDigester::with_value_witness().finish());
    }
    let value = decode_toon_record(&stdout, NormalizationError::Toon)?;
    let mut digest = SemanticDigester::with_value_witness();
    digest
        .push(&value)
        .map_err(|error| NormalizationError::Toon(error.to_string()))?;
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
    use std::fs;

    use serde_json::json;

    use super::digest_toon_values;
    use crate::benchmark::correctness::semantic_digest;

    #[test]
    fn fallback_rewinds_unread_bytes_before_next_result() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("output.toon");
        fs::write(&path, b"a: 1\n2\n").expect("TOON output");

        let values = [json!({"a": 1}), json!(2)];
        let mut expected = semantic_digest(values.iter()).expect("semantic digest");
        expected.toon_line_counts = Some(vec![2, 1]);

        let actual = digest_toon_values(&path, Some(&expected)).expect("normalized values");

        assert_eq!(actual, expected);
        assert_eq!(actual.result_count, 2);
    }
}
