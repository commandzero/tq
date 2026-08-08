//! Mandatory correctness decision before timing.

use serde::{Serialize, Serializer, ser::SerializeMap as _, ser::SerializeSeq as _};
use serde_json::Value;
use sha2::{Digest as _, Sha256};

use super::OutputContractKind;
use crate::compatibility::{ErrorClass, ProcessStatus};

/// Bounded identity of an ordered structured-result sequence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticDigest {
    /// Number of result values included in the digest.
    pub result_count: u64,
    /// SHA-256 over canonical JSON values separated as JSON Text Sequences.
    pub sha256: [u8; 32],
}

/// Contract-specific correctness payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CorrectnessPayload {
    /// Ordered semantic results represented without retaining the sequence.
    SemanticSequence(SemanticDigest),
    /// Exact bytes for a raw-output contract.
    RawBytes(Vec<u8>),
    /// Exit metadata alone.
    ExitOnly,
}

/// Bounded observation used by the benchmark correctness gate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CorrectnessObservation {
    /// Output payload appropriate to the case contract.
    pub payload: CorrectnessPayload,
    /// Process completion class.
    pub process_status: ProcessStatus,
    /// Exit code, when available.
    pub exit_code: Option<i32>,
    /// Stable classified failure, when one occurred.
    pub error_class: Option<ErrorClass>,
}

/// Correctness-gate result retained in a report row.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CorrectnessDecision {
    /// Candidate is eligible for timing.
    Passed,
    /// Candidate normalized successfully but differs semantically.
    Incorrect(String),
    /// Candidate output could not be normalized.
    Unnormalized(String),
}

/// Compares a candidate to the reviewed/reference observation contract.
/// Callers must not measure samples unless this returns `Passed`.
#[must_use]
pub fn correctness_gate(
    contract: OutputContractKind,
    reference: &CorrectnessObservation,
    candidate: Result<&CorrectnessObservation, &str>,
) -> CorrectnessDecision {
    let candidate = match candidate {
        Ok(candidate) => candidate,
        Err(error) => return CorrectnessDecision::Unnormalized(error.to_owned()),
    };
    let mut differences = Vec::new();
    let payload_matches = match (contract, &reference.payload, &candidate.payload) {
        (
            OutputContractKind::SemanticSequence,
            CorrectnessPayload::SemanticSequence(left),
            CorrectnessPayload::SemanticSequence(right),
        ) => left == right,
        (
            OutputContractKind::RawBytes,
            CorrectnessPayload::RawBytes(left),
            CorrectnessPayload::RawBytes(right),
        ) => left == right,
        (
            OutputContractKind::ExitOnly,
            CorrectnessPayload::ExitOnly,
            CorrectnessPayload::ExitOnly,
        ) => true,
        _ => {
            differences.push("output contract");
            true
        }
    };
    if !payload_matches {
        differences.push(match contract {
            OutputContractKind::SemanticSequence => "ordered result sequence",
            OutputContractKind::RawBytes => "raw output bytes",
            OutputContractKind::ExitOnly => "output contract",
        });
    }
    if reference.process_status != candidate.process_status {
        differences.push("process status");
    }
    if reference.exit_code != candidate.exit_code {
        differences.push("exit code");
    }
    if reference.error_class != candidate.error_class {
        differences.push("error class");
    }
    if differences.is_empty() {
        CorrectnessDecision::Passed
    } else {
        CorrectnessDecision::Incorrect(differences.join(", "))
    }
}

/// Builds the same canonical digest used by the streaming benchmark gate.
///
/// This helper is primarily useful for small in-memory test expectations.
///
/// # Errors
///
/// Returns a JSON serialization error if a value cannot be serialized.
pub fn semantic_digest<'a>(
    values: impl IntoIterator<Item = &'a Value>,
) -> Result<SemanticDigest, serde_json::Error> {
    let mut digest = SemanticDigester::default();
    for value in values {
        digest.push(value)?;
    }
    Ok(digest.finish())
}

/// Incremental canonical semantic-sequence digester.
#[derive(Default)]
pub(crate) struct SemanticDigester {
    hasher: Sha256,
    result_count: u64,
}

impl SemanticDigester {
    pub(crate) fn push(&mut self, value: &Value) -> Result<(), serde_json::Error> {
        self.hasher.update([0x1e]);
        serde_json::to_writer(HashWriter(&mut self.hasher), &CanonicalValue(value))?;
        self.hasher.update(b"\n");
        self.result_count = self.result_count.saturating_add(1);
        Ok(())
    }

    pub(crate) fn finish(mut self) -> SemanticDigest {
        self.hasher.update([0xff]);
        self.hasher.update(self.result_count.to_be_bytes());
        SemanticDigest {
            result_count: self.result_count,
            sha256: self.hasher.finalize().into(),
        }
    }
}

struct HashWriter<'a>(&'a mut Sha256);

impl std::io::Write for HashWriter<'_> {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.0.update(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct CanonicalValue<'a>(&'a Value);

impl Serialize for CanonicalValue<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0 {
            Value::Null => serializer.serialize_unit(),
            Value::Bool(value) => serializer.serialize_bool(*value),
            Value::Number(number) => {
                let canonical = tq_core::Number::parse(&number.to_string())
                    .map_err(serde::ser::Error::custom)?
                    .to_string()
                    .parse::<serde_json::Number>()
                    .map_err(serde::ser::Error::custom)?;
                canonical.serialize(serializer)
            }
            Value::String(value) => serializer.serialize_str(value),
            Value::Array(values) => {
                let mut sequence = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    sequence.serialize_element(&CanonicalValue(value))?;
                }
                sequence.end()
            }
            Value::Object(values) => {
                let mut map = serializer.serialize_map(Some(values.len()))?;
                for (key, value) in values {
                    map.serialize_entry(key, &CanonicalValue(value))?;
                }
                map.end()
            }
        }
    }
}
