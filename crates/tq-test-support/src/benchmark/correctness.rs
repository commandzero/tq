//! Mandatory correctness decision before timing.

use std::{
    fs::File,
    io::{self, BufReader, BufWriter, Read as _, Seek as _, SeekFrom, Write as _},
    sync::{Arc, Mutex},
};

use super::OutputContractKind;
use crate::compatibility::{ErrorClass, ProcessStatus};
use serde::{Serialize, Serializer, ser::SerializeMap as _, ser::SerializeSeq as _};
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use tempfile::NamedTempFile;
use thiserror::Error;

/// Bounded identity of an ordered structured-result sequence.
#[derive(Clone, Debug)]
pub struct SemanticDigest {
    /// Number of result values included in the digest.
    pub result_count: u64,
    /// SHA-256 over canonical JSON values separated as JSON Text Sequences.
    pub sha256: [u8; 32],
    /// File-backed per-result hashes and line counts for ambiguous TOON values.
    ///
    /// The witness is intentionally outside the digest hash so a semantic
    /// comparison stays cheap while TOON boundary recovery can still inspect
    /// one expected result at a time.
    pub(crate) witness: Option<Arc<SemanticWitness>>,
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
            OutputContractKind::SemanticSequence | OutputContractKind::ColoredSemanticSequence,
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
            OutputContractKind::ColoredSemanticSequence => "ordered colored JSON result sequence",
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
        digest
            .push(value)
            .map_err(SemanticDigestError::into_json_error)?;
    }
    Ok(digest.finish())
}

/// Incremental canonical semantic-sequence digester.
#[derive(Default)]
pub(crate) struct SemanticDigester {
    hasher: Sha256,
    result_count: u64,
    witness_enabled: bool,
    witness: Option<WitnessWriter>,
}

#[derive(Debug, Error)]
pub(crate) enum SemanticDigestError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
}

impl SemanticDigestError {
    fn into_json_error(self) -> serde_json::Error {
        match self {
            Self::Json(error) => error,
            Self::Io(error) => serde_json::Error::io(error),
        }
    }
}

impl SemanticDigester {
    pub(crate) fn with_value_witness() -> Self {
        Self {
            witness_enabled: true,
            ..Self::default()
        }
    }

    pub(crate) fn push(&mut self, value: &Value) -> Result<(), SemanticDigestError> {
        self.hasher.update([0x1e]);
        serde_json::to_writer(HashWriter(&mut self.hasher), &CanonicalValue(value))?;
        self.hasher.update(b"\n");
        if self.witness_enabled {
            let value_digest = value_digest(value)?;
            let line_count = toon_line_count(value)?;
            let witness = if let Some(witness) = &mut self.witness {
                witness
            } else {
                self.witness = Some(WitnessWriter::new()?);
                self.witness.as_mut().expect("witness was just created")
            };
            witness.push(value_digest, line_count)?;
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
            witness: self
                .witness
                .take()
                .map(|witness| Arc::new(SemanticWitness::from_writer(witness))),
        }
    }
}

struct WitnessWriter {
    file: BufWriter<NamedTempFile>,
}

impl WitnessWriter {
    fn new() -> io::Result<Self> {
        Ok(Self {
            file: BufWriter::new(NamedTempFile::new()?),
        })
    }

    fn push(&mut self, value_digest: [u8; 32], line_count: u64) -> io::Result<()> {
        self.file.write_all(&value_digest)?;
        self.file.write_all(&line_count.to_be_bytes())
    }
}

/// File-backed ordered witness used only when an ambiguous output format
/// needs expected per-result boundaries. Each record is a fixed 40 bytes, so
/// the in-memory state remains constant regardless of result count.
#[derive(Debug)]
pub(crate) struct SemanticWitness {
    file: Mutex<BufWriter<NamedTempFile>>,
}

impl SemanticWitness {
    fn from_writer(writer: WitnessWriter) -> Self {
        Self {
            file: Mutex::new(writer.file),
        }
    }

    pub(crate) fn reader(&self) -> io::Result<BufReader<File>> {
        let mut writer = self
            .file
            .lock()
            .map_err(|_| io::Error::other("semantic witness lock poisoned"))?;
        writer.flush()?;
        let mut file = writer.get_ref().reopen()?;
        file.seek(SeekFrom::Start(0))?;
        Ok(BufReader::new(file))
    }

    pub(crate) fn read_record(reader: &mut BufReader<File>) -> io::Result<([u8; 32], u64)> {
        let mut value_digest = [0_u8; 32];
        reader.read_exact(&mut value_digest)?;
        let mut line_count = [0_u8; std::mem::size_of::<u64>()];
        reader.read_exact(&mut line_count)?;
        Ok((value_digest, u64::from_be_bytes(line_count)))
    }
}

/// Canonicalizes a finite tq number for semantic comparisons shared by
/// benchmark and corpus validation code.
pub(crate) fn canonical_number(input: &str) -> Result<String, String> {
    tq_core::Number::canonicalize_literal_numeric(input).map_err(|error| error.to_string())
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
                let canonical = canonical_number(&number.to_string())
                    .map_err(serde::ser::Error::custom)?
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
                let mut entries = values.iter().collect::<Vec<_>>();
                entries.sort_unstable_by_key(|(key, _)| *key);
                for (key, value) in entries {
                    map.serialize_entry(key, &CanonicalValue(value))?;
                }
                map.end()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SemanticDigester;

    #[test]
    fn semantic_digest_ignores_object_member_order_but_raw_bytes_do_not() {
        let first: serde_json::Value =
            serde_json::from_str(r#"{"z":1,"a":2}"#).expect("first object");
        let second: serde_json::Value =
            serde_json::from_str(r#"{"a":2,"z":1}"#).expect("second object");
        assert_eq!(
            super::semantic_digest([&first]).expect("first digest"),
            super::semantic_digest([&second]).expect("second digest")
        );
        assert_ne!(
            serde_json::to_vec(&first).expect("first bytes"),
            serde_json::to_vec(&second).expect("second bytes")
        );
    }

    #[test]
    fn disk_witness_accepts_more_results_than_the_old_memory_ceiling() {
        let mut digest = SemanticDigester::with_value_witness();
        for value in 0..220_000_u64 {
            digest
                .push(&serde_json::json!(value))
                .expect("disk-backed witness accepts every result");
        }
        let digest = digest.finish();
        assert_eq!(digest.result_count, 220_000);
        let witness = digest.witness.expect("witness file");
        let mut reader = witness.reader().expect("witness reader");
        let (first, first_lines) =
            super::SemanticWitness::read_record(&mut reader).expect("first witness record");
        assert_eq!(first_lines, 1);
        assert_eq!(
            first,
            super::value_digest(&serde_json::json!(0)).expect("first digest")
        );
    }

    #[test]
    fn numeric_spelling_is_canonicalized_alongside_object_order() {
        let first: serde_json::Value =
            serde_json::from_str(r#"{"z":1.0,"a":2}"#).expect("first object");
        let second: serde_json::Value =
            serde_json::from_str(r#"{"a":2.0,"z":1}"#).expect("second object");
        assert_eq!(
            super::semantic_digest([&first]).expect("first digest"),
            super::semantic_digest([&second]).expect("second digest")
        );
    }
}
