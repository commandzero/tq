//! Mandatory correctness decision before timing.

use serde::{Serialize, Serializer, ser::SerializeMap as _, ser::SerializeSeq as _};
use serde_json::Value;
use sha2::{Digest as _, Sha256};

use super::OutputContractKind;
use crate::compatibility::{ErrorClass, ProcessStatus};

/// Bounded identity of an ordered structured-result sequence.
#[derive(Clone, Debug)]
pub struct SemanticDigest {
    /// Number of result values included in the digest.
    pub result_count: u64,
    /// SHA-256 over canonical JSON values separated as JSON Text Sequences.
    pub sha256: [u8; 32],
    /// Per-result hashes retained only for file-backed TOON boundary checks.
    pub(crate) value_digests: Option<Vec<[u8; 32]>>,
    /// Canonical LF line counts used as a linear-time TOON boundary hint.
    pub(crate) toon_line_counts: Option<Vec<u64>>,
}

impl PartialEq for SemanticDigest {
    fn eq(&self, other: &Self) -> bool {
        self.result_count == other.result_count && self.sha256 == other.sha256
    }
}

impl Eq for SemanticDigest {}

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
    let mut digest = SemanticDigester::with_value_witness();
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
    value_digests: Option<Vec<[u8; 32]>>,
    toon_line_counts: Option<Vec<u64>>,
}

impl SemanticDigester {
    pub(crate) fn with_value_witness() -> Self {
        Self {
            value_digests: Some(Vec::new()),
            toon_line_counts: Some(Vec::new()),
            ..Self::default()
        }
    }

    pub(crate) fn push(&mut self, value: &Value) -> Result<(), serde_json::Error> {
        self.hasher.update([0x1e]);
        serde_json::to_writer(HashWriter(&mut self.hasher), &CanonicalValue(value))?;
        self.hasher.update(b"\n");
        if let Some(value_digests) = &mut self.value_digests {
            value_digests.push(value_digest(value)?);
        }
        if let Some(line_counts) = &mut self.toon_line_counts {
            line_counts.push(toon_line_count(value)?);
        }
        self.result_count = self.result_count.saturating_add(1);
        Ok(())
    }

    pub(crate) fn finish(mut self) -> SemanticDigest {
        self.hasher.update([0xff]);
        self.hasher.update(self.result_count.to_be_bytes());
        SemanticDigest {
            result_count: self.result_count,
            sha256: self.hasher.finalize().into(),
            value_digests: self.value_digests,
            toon_line_counts: self.toon_line_counts,
        }
    }
}

pub(crate) fn value_digest(value: &Value) -> Result<[u8; 32], serde_json::Error> {
    let mut hasher = Sha256::new();
    serde_json::to_writer(HashWriter(&mut hasher), &CanonicalValue(value))?;
    Ok(hasher.finalize().into())
}

fn toon_line_count(value: &Value) -> Result<u64, serde_json::Error> {
    let value = tq_core::Value::from_json(value.clone())
        .map_err(|error| serde_json::Error::io(std::io::Error::other(error.to_string())))?;
    let encoded = tq_toon::encode(&value, tq_toon::WriterConfig::default());
    Ok(encoded.bytes().filter(|byte| *byte == b'\n').count() as u64 + 1)
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
