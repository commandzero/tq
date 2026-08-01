//! Data-driven compatibility campaign execution.

use std::{collections::BTreeMap, fs, io, io::Write, path::Path, time::Duration};

use thiserror::Error;

use super::{
    CaseAdapter, CaseClassification, CaseReport, CaseStatus, CompatibilityCase,
    CompatibilityCatalog, CompatibilityReport, ContractKind, CoverageCount, ExecutableConfig,
    FinalStatus, Invocation, InvocationMode, NormalizationError, ObservationState, ProcessError,
    SemanticDiff, ToolIdentity, ToolKind, ToolObservation, discover_tool, encode_hex, normalize_jq,
    normalize_raw, normalize_toon_sequence, normalize_yq, run_process,
};

/// Compatibility campaign size.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CampaignProfile {
    /// Fast common-surface check.
    Smoke,
    /// Every MVP and deferred marker.
    Full,
}

impl CampaignProfile {
    /// Stable CLI/report name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Smoke => "smoke",
            Self::Full => "full",
        }
    }

    fn includes(self, case: &CompatibilityCase) -> bool {
        match self {
            Self::Smoke => {
                case.status == CaseStatus::Mvp
                    && matches!(case.classification, CaseClassification::Common)
            }
            Self::Full => true,
        }
    }
}

/// Campaign-level failures rather than tool observations.
#[derive(Debug, Error)]
pub enum RunnerError {
    /// Executable discovery failed.
    #[error(transparent)]
    Discovery(#[from] super::ToolDiscoveryError),
    /// Fixture or temporary-file I/O failed.
    #[error("compatibility fixture I/O failed: {0}")]
    Io(#[from] io::Error),
}

/// Discovers configured tools and runs a catalog profile.
///
/// # Errors
///
/// Returns discovery or fixture I/O failures. Individual subprocess and
/// normalization failures are retained in the report.
pub fn run_campaign(
    catalog: &CompatibilityCatalog,
    profile: CampaignProfile,
    config: &ExecutableConfig,
    repository_root: &Path,
    timeout: Duration,
) -> Result<CompatibilityReport, RunnerError> {
    let mut tools = Vec::new();
    for kind in [ToolKind::Jq, ToolKind::Yq, ToolKind::Tq] {
        if let Some(identity) = discover_tool(kind, config, repository_root)? {
            tools.push(identity);
        }
    }
    let reports = catalog
        .cases
        .iter()
        .filter(|case| profile.includes(case))
        .map(|case| run_case(case, &tools, repository_root, timeout))
        .collect::<Result<Vec<_>, _>>()?;
    let coverage = coverage(&reports);
    let has_harness_error = reports.iter().any(|case| {
        case.observations
            .iter()
            .any(|observation| observation.state == ObservationState::HarnessError)
    });
    let has_differences = reports.iter().any(|case| !case.semantic_diffs.is_empty());
    let final_status = if has_harness_error {
        FinalStatus::Failed
    } else if has_differences {
        FinalStatus::ObservedDifferences
    } else {
        FinalStatus::Passed
    };
    Ok(CompatibilityReport {
        schema_version: super::REPORT_SCHEMA_VERSION,
        profile: profile.name().to_owned(),
        corpus: catalog.identity.clone(),
        tools,
        cases: reports,
        coverage,
        final_status,
    })
}

fn run_case(
    case: &CompatibilityCase,
    identities: &[ToolIdentity],
    repository_root: &Path,
    timeout: Duration,
) -> Result<CaseReport, io::Error> {
    let mut observations = Vec::new();
    for tool in [ToolKind::Jq, ToolKind::Yq, ToolKind::Tq] {
        let adapter = adapter(case, tool);
        let Some(identity) = identities.iter().find(|identity| identity.tool == tool) else {
            observations.push(skipped(
                tool,
                ObservationState::Unavailable,
                "executable not found",
            ));
            continue;
        };
        if !adapter.supported {
            observations.push(skipped(
                tool,
                ObservationState::Unsupported,
                adapter
                    .note
                    .as_deref()
                    .unwrap_or("case adapter is unsupported"),
            ));
            continue;
        }
        observations.push(execute(case, adapter, identity, repository_root, timeout)?);
    }
    let semantic_diffs = semantic_diffs(&observations);
    Ok(CaseReport {
        id: case.id.clone(),
        capabilities: case.capabilities.clone(),
        observations,
        semantic_diffs,
    })
}

fn execute(
    case: &CompatibilityCase,
    adapter: &CaseAdapter,
    identity: &ToolIdentity,
    repository_root: &Path,
    timeout: Duration,
) -> Result<ToolObservation, io::Error> {
    let bytes = fixture_bytes(case, repository_root)?;
    let mut temporary = None;
    let mut args = adapter.args.clone();
    args.push(adapter.query.clone().unwrap_or_else(|| case.query.clone()));
    let stdin = match case.invocation_mode {
        InvocationMode::Stdin => bytes,
        InvocationMode::NullInput => Vec::new(),
        InvocationMode::File => {
            let mut file = tempfile::NamedTempFile::new()?;
            file.write_all(&bytes)?;
            args.push(file.path().display().to_string());
            temporary = Some(file);
            Vec::new()
        }
    };
    let process = run_process(&Invocation {
        executable: identity.path.clone(),
        args,
        stdin,
        timeout,
        current_dir: Some(repository_root.to_owned()),
    });
    let outcome = match process {
        Ok(outcome) => outcome,
        Err(error) => return Ok(harness_error(identity.tool, &error)),
    };
    drop(temporary);
    let normalized = match case.expected.contract {
        ContractKind::RawBytes | ContractKind::ExitStatus => {
            Ok(normalize_raw(identity.tool, &outcome))
        }
        ContractKind::ResultSequence | ContractKind::Error => match identity.tool {
            ToolKind::Jq => normalize_jq(&outcome),
            ToolKind::Yq => normalize_yq(&outcome),
            ToolKind::Tq => normalize_toon_sequence(&outcome),
        },
    };
    match normalized {
        Ok(value) => Ok(ToolObservation {
            tool: identity.tool,
            state: ObservationState::Executed,
            results: value.results,
            raw_stdout_hex: value.raw_bytes.as_deref().map(encode_hex),
            stderr_hex: (!value.stderr.is_empty()).then(|| encode_hex(&value.stderr)),
            process_status: Some(value.process_status),
            exit_code: value.exit_code,
            error_class: value.error_class,
            wall_time_micros: Some(outcome.wall_time_micros),
            note: None,
        }),
        Err(error) => Ok(normalization_error(identity.tool, &error, &outcome)),
    }
}

fn fixture_bytes(case: &CompatibilityCase, repository_root: &Path) -> Result<Vec<u8>, io::Error> {
    if let Some(inline) = &case.fixture.inline {
        return Ok(inline.as_bytes().to_vec());
    }
    if let Some(path) = &case.fixture.path {
        return fs::read(repository_root.join(path));
    }
    Ok(Vec::new())
}

fn adapter(case: &CompatibilityCase, tool: ToolKind) -> &CaseAdapter {
    match tool {
        ToolKind::Jq => &case.adapters.jq,
        ToolKind::Yq => &case.adapters.yq,
        ToolKind::Tq => &case.adapters.tq,
    }
}

fn skipped(tool: ToolKind, state: ObservationState, note: &str) -> ToolObservation {
    ToolObservation {
        tool,
        state,
        results: Vec::new(),
        raw_stdout_hex: None,
        stderr_hex: None,
        process_status: None,
        exit_code: None,
        error_class: None,
        wall_time_micros: None,
        note: Some(note.to_owned()),
    }
}

fn harness_error(tool: ToolKind, error: &ProcessError) -> ToolObservation {
    skipped(tool, ObservationState::HarnessError, &error.to_string())
}

fn normalization_error(
    tool: ToolKind,
    error: &NormalizationError,
    outcome: &super::ProcessOutcome,
) -> ToolObservation {
    ToolObservation {
        tool,
        state: ObservationState::HarnessError,
        results: Vec::new(),
        raw_stdout_hex: Some(encode_hex(&outcome.stdout)),
        stderr_hex: (!outcome.stderr.is_empty()).then(|| encode_hex(&outcome.stderr)),
        process_status: Some(outcome.status),
        exit_code: outcome.exit_code,
        error_class: Some(super::ErrorClass::MalformedOutput),
        wall_time_micros: Some(outcome.wall_time_micros),
        note: Some(error.to_string()),
    }
}

fn semantic_diffs(observations: &[ToolObservation]) -> Vec<SemanticDiff> {
    let executed = observations
        .iter()
        .filter(|observation| observation.state == ObservationState::Executed)
        .collect::<Vec<_>>();
    let mut diffs = Vec::new();
    for (index, left) in executed.iter().enumerate() {
        for right in executed.iter().skip(index + 1) {
            let mut fields = Vec::new();
            if left.results != right.results {
                fields.push("result sequence");
            }
            if left.raw_stdout_hex != right.raw_stdout_hex {
                fields.push("raw stdout");
            }
            if left.exit_code != right.exit_code {
                fields.push("exit code");
            }
            if left.error_class != right.error_class {
                fields.push("error class");
            }
            if !fields.is_empty() {
                diffs.push(SemanticDiff {
                    left: left.tool,
                    right: right.tool,
                    summary: fields.join(", "),
                });
            }
        }
    }
    diffs
}

fn coverage(reports: &[CaseReport]) -> BTreeMap<String, CoverageCount> {
    let mut coverage = BTreeMap::<String, CoverageCount>::new();
    for case in reports {
        for capability in &case.capabilities {
            let count = coverage.entry(capability.clone()).or_default();
            count.cases += 1;
            for observation in &case.observations {
                match observation.state {
                    ObservationState::Executed => count.executed += 1,
                    ObservationState::HarnessError => count.harness_errors += 1,
                    ObservationState::Unsupported | ObservationState::Unavailable => {
                        count.skipped += 1;
                    }
                }
            }
        }
    }
    coverage
}
