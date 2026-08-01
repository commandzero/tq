//! Mandatory correctness decision before timing.

use serde::{Deserialize, Serialize};

use super::OutputContractKind;
use crate::compatibility::NormalizedObservation;

/// Correctness-gate result retained in a report row.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
///
/// Callers must not measure samples unless this returns `Passed`.
#[must_use]
pub fn correctness_gate(
    contract: OutputContractKind,
    reference: &NormalizedObservation,
    candidate: Result<&NormalizedObservation, &str>,
) -> CorrectnessDecision {
    let candidate = match candidate {
        Ok(candidate) => candidate,
        Err(error) => return CorrectnessDecision::Unnormalized(error.to_owned()),
    };
    let mut differences = Vec::new();
    match contract {
        OutputContractKind::SemanticSequence => {
            if reference.results != candidate.results {
                differences.push("ordered result sequence");
            }
        }
        OutputContractKind::RawBytes => {
            if reference.raw_bytes != candidate.raw_bytes {
                differences.push("raw output bytes");
            }
        }
        OutputContractKind::ExitOnly => {}
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
