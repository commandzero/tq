//! Cross-tool result-sequence and failure normalization.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use super::{ProcessOutcome, ProcessStatus, ToolKind};

/// Stable compatibility error taxonomy.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorClass {
    /// Invalid command-line usage or option combination.
    CliUsage,
    /// Query parsing, resolution, analysis, or compilation failed.
    QueryCompile,
    /// Structured input could not be decoded.
    InputParse,
    /// Evaluation encountered an invalid type, path, or explicit error.
    RuntimeTypePath,
    /// Evaluation rejected a numeric value outside its portable range.
    RuntimeRange,
    /// Evaluation denied ambient access under the active capability policy.
    RuntimePolicy,
    /// A configured resource limit was exceeded.
    Resource,
    /// Harness wall-time limit was exceeded.
    Timeout,
    /// Process terminated from a signal.
    Signal,
    /// Recognized capability is intentionally unsupported.
    UnsupportedCapability,
    /// Tool output could not satisfy its declared result contract.
    MalformedOutput,
}

/// Metadata retained for intentional normalization boundaries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NormalizationNote {
    /// YAML presentation details are outside the ordered JSON-shaped model.
    YamlPresentationNotRetained,
    /// stderr was retained independently and not folded into result data.
    StderrCaptured,
}

/// Normalized cross-tool observation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedObservation {
    /// Ordered zero-or-more structured results.
    pub results: Vec<Value>,
    /// Exact raw bytes for raw-output contracts.
    pub raw_bytes: Option<Vec<u8>>,
    /// Exact stderr bytes.
    pub stderr: Vec<u8>,
    /// Process completion class.
    pub process_status: ProcessStatus,
    /// Exit code, when available.
    pub exit_code: Option<i32>,
    /// Stable classified failure, when one occurred.
    pub error_class: Option<ErrorClass>,
    /// Explicit normalization-boundary notes.
    pub notes: Vec<NormalizationNote>,
}

/// Stable structured-output normalization failures.
#[derive(Debug, Error)]
pub enum NormalizationError {
    /// jq emitted bytes that were not a sequence of JSON texts.
    #[error("malformed jq JSON result sequence: {0}")]
    Jq(String),
    /// yq emitted bytes that were not a sequence of YAML documents.
    #[error("malformed yq YAML result sequence: {0}")]
    Yq(String),
    /// tq output was not a valid TOON Text Sequence.
    #[error("malformed TOON Text Sequence: {0}")]
    ToonSequence(String),
}

/// Normalizes zero-or-more jq JSON result texts without sorting.
///
/// # Errors
///
/// Returns a jq normalization error when stdout contains malformed JSON.
pub fn normalize_jq(outcome: &ProcessOutcome) -> Result<NormalizedObservation, NormalizationError> {
    let results = serde_json::Deserializer::from_slice(&outcome.stdout)
        .into_iter::<Value>()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| NormalizationError::Jq(error.to_string()))?
        .into_iter()
        .map(canonicalize_numbers)
        .collect();
    Ok(observation(
        ToolKind::Jq,
        outcome,
        results,
        None,
        Vec::new(),
    ))
}

/// Normalizes zero-or-more yq YAML documents into ordered JSON-model values.
///
/// # Errors
///
/// Returns a yq normalization error when stdout contains malformed YAML.
pub fn normalize_yq(outcome: &ProcessOutcome) -> Result<NormalizedObservation, NormalizationError> {
    if outcome.stdout.is_empty() {
        return Ok(observation(
            ToolKind::Yq,
            outcome,
            Vec::new(),
            None,
            vec![NormalizationNote::YamlPresentationNotRetained],
        ));
    }
    // Compatibility and benchmark adapters request compact JSON from yq.
    // Its stdout is a JSON text sequence, just like jq's. Prefer that strict
    // framing so adjacent scalar results cannot collapse into one YAML plain
    // scalar (for example `1\n2\n`). Retain YAML-document decoding for the
    // explicitly exercised YAML presentation boundary.
    if let Ok(results) = serde_json::Deserializer::from_slice(&outcome.stdout)
        .into_iter::<Value>()
        .collect::<Result<Vec<_>, _>>()
    {
        return Ok(observation(
            ToolKind::Yq,
            outcome,
            results.into_iter().map(canonicalize_numbers).collect(),
            None,
            vec![NormalizationNote::YamlPresentationNotRetained],
        ));
    }
    let text = String::from_utf8_lossy(&outcome.stdout);
    let results = yaml_serde::Deserializer::from_str(&text)
        .map(Value::deserialize)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| NormalizationError::Yq(error.to_string()))?
        .into_iter()
        .map(canonicalize_numbers)
        .collect();
    Ok(observation(
        ToolKind::Yq,
        outcome,
        results,
        None,
        vec![NormalizationNote::YamlPresentationNotRetained],
    ))
}

/// Normalizes an RS-prefixed, LF-suffixed TOON Text Sequence.
///
/// # Errors
///
/// Returns a framing or TOON decoding error. Empty stdout is zero results.
pub fn normalize_toon_sequence(
    outcome: &ProcessOutcome,
) -> Result<NormalizedObservation, NormalizationError> {
    let results = tq_formats::decode_toon_sequence(
        &outcome.stdout,
        "<tq-compatibility-output>",
        tq_toon::DecoderConfig::default(),
    )
    .map_err(|error| NormalizationError::ToonSequence(error.to_string()))?
    .into_iter()
    .map(|document| {
        document
            .value
            .to_json()
            .map(canonicalize_numbers)
            .map_err(|error| NormalizationError::ToonSequence(error.to_string()))
    })
    .collect::<Result<Vec<_>, _>>()?;
    Ok(observation(
        ToolKind::Tq,
        outcome,
        results,
        None,
        Vec::new(),
    ))
}

/// Preserves exact stdout bytes for a raw-output contract.
#[must_use]
pub fn normalize_raw(tool: ToolKind, outcome: &ProcessOutcome) -> NormalizedObservation {
    observation(
        tool,
        outcome,
        Vec::new(),
        Some(outcome.stdout.clone()),
        Vec::new(),
    )
}

/// Classifies process failures without changing their captured bytes.
#[must_use]
pub fn classify_process(tool: ToolKind, outcome: &ProcessOutcome) -> Option<ErrorClass> {
    match outcome.status {
        ProcessStatus::TimedOut => return Some(ErrorClass::Timeout),
        ProcessStatus::Signaled => return Some(ErrorClass::Signal),
        ProcessStatus::Exited => {}
    }
    let code = outcome.exit_code.unwrap_or(1);
    if code == 0 {
        return None;
    }
    let stderr = String::from_utf8_lossy(&outcome.stderr).to_ascii_lowercase();
    let unsupported = stderr.contains("unsupported")
        || stderr.contains("not supported")
        || stderr.contains("not implemented");
    if code == 2 && matches!(tool, ToolKind::Jq | ToolKind::Yq | ToolKind::Tq) {
        if tool == ToolKind::Tq
            && unsupported
            && (stderr.contains("bytecode operation is not executable")
                || stderr.contains("unsupported mode:"))
        {
            return Some(ErrorClass::UnsupportedCapability);
        }
        return Some(ErrorClass::CliUsage);
    }
    if unsupported {
        return Some(ErrorClass::UnsupportedCapability);
    }
    if stderr.contains("resource")
        || stderr.contains("memory limit")
        || stderr.contains("step limit")
    {
        return Some(ErrorClass::Resource);
    }
    if stderr.contains("numeric range error") {
        return Some(ErrorClass::RuntimeRange);
    }
    if stderr.contains("capability policy") {
        return Some(ErrorClass::RuntimePolicy);
    }
    if stderr.contains("parse error") && (tool == ToolKind::Jq || stderr.contains("input")) {
        return Some(ErrorClass::InputParse);
    }
    if tool == ToolKind::Tq
        && (stderr.contains(" input rejected")
            || stderr.contains("input rejected")
            || stderr.contains("input i/o")
            || stderr.contains("input resource"))
    {
        return Some(ErrorClass::InputParse);
    }
    if stderr.contains("compile error") || stderr.contains("syntax error") {
        return Some(ErrorClass::QueryCompile);
    }
    if stderr.contains("cannot index")
        || stderr.contains("type error")
        || stderr.contains("runtime error")
    {
        return Some(ErrorClass::RuntimeTypePath);
    }
    match (tool, code) {
        (ToolKind::Jq, 2 | 4) | (ToolKind::Yq | ToolKind::Tq, 2) | (ToolKind::Tq, 4) => {
            Some(ErrorClass::CliUsage)
        }
        (ToolKind::Jq | ToolKind::Tq, 3) => Some(ErrorClass::QueryCompile),
        _ => Some(ErrorClass::RuntimeTypePath),
    }
}

fn canonicalize_numbers(value: Value) -> Value {
    match value {
        Value::Number(number) => tq_core::Number::parse(&number.to_string())
            .ok()
            .and_then(|number| number.to_string().parse().ok())
            .map(Value::Number)
            .unwrap_or(Value::Number(number)),
        Value::Array(values) => {
            Value::Array(values.into_iter().map(canonicalize_numbers).collect())
        }
        Value::Object(values) => Value::Object(
            values
                .into_iter()
                .map(|(key, value)| (key, canonicalize_numbers(value)))
                .collect(),
        ),
        other => other,
    }
}

fn observation(
    tool: ToolKind,
    outcome: &ProcessOutcome,
    results: Vec<Value>,
    raw_bytes: Option<Vec<u8>>,
    mut notes: Vec<NormalizationNote>,
) -> NormalizedObservation {
    if !outcome.stderr.is_empty() {
        notes.push(NormalizationNote::StderrCaptured);
    }
    NormalizedObservation {
        results,
        raw_bytes,
        stderr: outcome.stderr.clone(),
        process_status: outcome.status,
        exit_code: outcome.exit_code,
        error_class: classify_process(tool, outcome),
        notes,
    }
}
