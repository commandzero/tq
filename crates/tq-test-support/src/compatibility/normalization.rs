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
    /// tq output was not a valid standalone TOON document.
    #[error("malformed standalone TOON document: {0}")]
    Toon(String),
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
    let complete_stdout =
        if outcome.exit_code.is_some_and(|code| code != 0) && !outcome.stdout.ends_with(b"\n") {
            outcome
                .stdout
                .iter()
                .rposition(|byte| *byte == b'\n')
                .map_or(&[][..], |index| &outcome.stdout[..=index])
        } else {
            outcome.stdout.as_slice()
        };
    let results = tq_formats::decode_toon_sequence(
        complete_stdout,
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

/// Normalizes one standalone TOON document, or zero output from a failed tq
/// process. A successful empty document is the canonical empty object (`{}`),
/// while a failed process with no output has no results.
///
/// # Errors
///
/// Returns a TOON decoding error when non-empty stdout is not one document.
pub fn normalize_toon_document(
    outcome: &ProcessOutcome,
) -> Result<NormalizedObservation, NormalizationError> {
    if outcome.stdout.is_empty() {
        let results = (outcome.exit_code == Some(0)).then(|| serde_json::json!({}));
        return Ok(observation(
            ToolKind::Tq,
            outcome,
            results.into_iter().collect(),
            None,
            Vec::new(),
        ));
    }
    let results = tq_formats::decode_toon(
        &outcome.stdout,
        "<tq-compatibility-output>",
        tq_toon::DecoderConfig::default(),
    )
    .map_err(|error| NormalizationError::Toon(error.to_string()))?
    .into_iter()
    .map(|document| {
        document
            .value
            .to_json()
            .map(canonicalize_numbers)
            .map_err(|error| NormalizationError::Toon(error.to_string()))
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

/// Uses independently observed JSON values to resolve TOON result boundaries.
/// Every result must end with LF and every captured byte must be consumed.
#[must_use]
pub fn toon_values_match(stdout: &[u8], expected: &[Value]) -> bool {
    let mut start = 0;
    for expected_value in expected {
        let end = stdout[start..]
            .iter()
            .enumerate()
            .filter(|(_, byte)| **byte == b'\n')
            .map(|(offset, _)| start + offset + 1)
            .find(|end| {
                tq_formats::decode_toon(
                    &stdout[start..*end],
                    "<tq-result>",
                    tq_toon::DecoderConfig::default(),
                )
                .is_ok_and(|documents| {
                    documents.len() == 1
                        && documents[0]
                            .value
                            .to_json()
                            .ok()
                            .map(canonicalize_numbers)
                            .as_ref()
                            == Some(expected_value)
                })
            });
        let Some(end) = end else {
            return false;
        };
        start = end;
    }
    start == stdout.len()
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
        || tool == ToolKind::Jq
            && stderr
                .lines()
                .any(|line| line.starts_with("jq: error (at "))
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
        Value::Number(number) => tq_core::Number::canonicalize_literal_numeric(&number.to_string())
            .ok()
            .and_then(|number| number.parse().ok())
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

#[cfg(test)]
mod tests {
    use super::{ProcessOutcome, ProcessStatus, normalize_toon_document, normalize_toon_sequence};

    #[test]
    fn ordinary_toon_results_use_expected_boundaries_and_preserve_all_bytes() {
        let values = vec![serde_json::json!({"a": 1}), serde_json::json!({"b": 2})];
        assert!(super::toon_values_match(b"a: 1\nb: 2\n", &values));
        assert!(!super::toon_values_match(b"a: 1\nb: 2", &values));
        assert!(!super::toon_values_match(b"a: 1\nb: 2\n3\n", &values));
        assert!(super::toon_values_match(b"", &[]));
        assert!(!super::toon_values_match(b"\n", &[]));
        assert!(super::toon_values_match(b"\n", &[serde_json::json!({})]));
        assert!(super::toon_values_match(
            b"0\n1e3\n",
            &[serde_json::json!(0), serde_json::json!(1000)]
        ));
    }

    #[test]
    fn failed_tq_process_ignores_only_its_incomplete_final_frame() {
        let outcome = ProcessOutcome {
            status: ProcessStatus::Exited,
            exit_code: Some(5),
            signal: None,
            stdout: b"\x1e1\n\x1e".to_vec(),
            stderr: b"tq: JSON input rejected".to_vec(),
            wall_time_micros: 1,
            recorded_command: Vec::new(),
        };

        let normalized = normalize_toon_sequence(&outcome).unwrap();
        assert_eq!(normalized.results, [serde_json::json!(1)]);
        assert!(normalized.error_class.is_some());
    }

    #[test]
    fn successful_tq_process_rejects_an_incomplete_final_frame() {
        let outcome = ProcessOutcome {
            status: ProcessStatus::Exited,
            exit_code: Some(0),
            signal: None,
            stdout: b"\x1e".to_vec(),
            stderr: Vec::new(),
            wall_time_micros: 1,
            recorded_command: Vec::new(),
        };

        assert!(normalize_toon_sequence(&outcome).is_err());
    }

    #[test]
    fn successful_empty_toon_sequence_has_no_results() {
        let outcome = ProcessOutcome {
            status: ProcessStatus::Exited,
            exit_code: Some(0),
            signal: None,
            stdout: Vec::new(),
            stderr: Vec::new(),
            wall_time_micros: 1,
            recorded_command: Vec::new(),
        };

        let normalized = normalize_toon_sequence(&outcome).unwrap();
        assert!(normalized.results.is_empty());
    }

    #[test]
    fn standalone_toon_output_is_normalized_without_sequence_framing() {
        let outcome = ProcessOutcome {
            status: ProcessStatus::Exited,
            exit_code: Some(0),
            signal: None,
            stdout: b"a: 1".to_vec(),
            stderr: Vec::new(),
            wall_time_micros: 1,
            recorded_command: Vec::new(),
        };

        let normalized = normalize_toon_document(&outcome).unwrap();
        assert_eq!(normalized.results, [serde_json::json!({"a": 1})]);
        assert_eq!(normalized.error_class, None);
    }

    #[test]
    fn successful_empty_standalone_toon_document_is_an_empty_object() {
        let outcome = ProcessOutcome {
            status: ProcessStatus::Exited,
            exit_code: Some(0),
            signal: None,
            stdout: Vec::new(),
            stderr: Vec::new(),
            wall_time_micros: 1,
            recorded_command: Vec::new(),
        };

        let normalized = normalize_toon_document(&outcome).unwrap();
        assert_eq!(normalized.results, [serde_json::json!({})]);
    }

    #[test]
    fn failed_empty_standalone_toon_output_has_no_results() {
        let outcome = ProcessOutcome {
            status: ProcessStatus::Exited,
            exit_code: Some(5),
            signal: None,
            stdout: Vec::new(),
            stderr: b"runtime error".to_vec(),
            wall_time_micros: 1,
            recorded_command: Vec::new(),
        };

        let normalized = normalize_toon_document(&outcome).unwrap();
        assert!(normalized.results.is_empty());
        assert!(normalized.error_class.is_some());
    }
}
