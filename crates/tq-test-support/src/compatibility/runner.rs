//! Data-driven compatibility campaign execution.

use std::{collections::BTreeMap, fs, io, io::Write, path::Path, time::Duration};

use thiserror::Error;

use super::{
    CapabilityCounts, CapabilityDisposition, CaseAdapter, CaseClassification, CaseReport,
    CaseStatus, CompatibilityCase, CompatibilityCatalog, CompatibilityReport, ContractKind,
    CoverageCount, ExecutableConfig, FinalStatus, FixtureFormat, Invocation, InvocationMode,
    NormalizationError, ObservationState, ProcessError, SemanticDiff, ToolIdentity, ToolKind,
    ToolObservation, discover_tool, encode_hex, normalize_jq, normalize_raw,
    normalize_toon_sequence, normalize_yq, run_process,
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
    let (capability_matrix, capability_counts) = capability_matrix(catalog, &reports);
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
        capability_matrix,
        capability_counts,
        final_status,
    })
}

fn capability_matrix(
    catalog: &CompatibilityCatalog,
    reports: &[CaseReport],
) -> (BTreeMap<String, CapabilityDisposition>, CapabilityCounts) {
    let report_by_id = reports
        .iter()
        .map(|report| (report.id.as_str(), report))
        .collect::<BTreeMap<_, _>>();
    let mut cases_by_capability = BTreeMap::<String, Vec<&CompatibilityCase>>::new();
    for case in &catalog.cases {
        if !report_by_id.contains_key(case.id.as_str()) {
            continue;
        }
        for capability in &case.capabilities {
            cases_by_capability
                .entry(capability.clone())
                .or_default()
                .push(case);
        }
    }

    let mut matrix = BTreeMap::new();
    let mut counts = CapabilityCounts::default();
    for (capability, cases) in cases_by_capability {
        let disposition = if cases.iter().all(|case| case.status == CaseStatus::Deferred) {
            CapabilityDisposition::Deferred
        } else {
            let mut executed = 0_usize;
            let mut skipped = 0_usize;
            let mut unavailable = 0_usize;
            let mut divergent = false;
            for case in cases
                .into_iter()
                .filter(|case| case.status == CaseStatus::Mvp)
            {
                let report = report_by_id[case.id.as_str()];
                let tq = report
                    .observations
                    .iter()
                    .find(|observation| observation.tool == ToolKind::Tq)
                    .expect("runner always records tq disposition");
                match tq.state {
                    ObservationState::Executed => executed += 1,
                    ObservationState::Unavailable => unavailable += 1,
                    ObservationState::Unsupported | ObservationState::HarnessError => skipped += 1,
                }
                divergent |= report.semantic_diffs.iter().any(|difference| {
                    matches!(
                        (difference.left, difference.right),
                        (ToolKind::Jq, ToolKind::Tq) | (ToolKind::Tq, ToolKind::Jq)
                    )
                });
            }
            if divergent {
                CapabilityDisposition::Divergent
            } else if executed > 0 && (skipped > 0 || unavailable > 0) {
                CapabilityDisposition::Partial
            } else if executed > 0 {
                CapabilityDisposition::Supported
            } else if unavailable > 0 {
                CapabilityDisposition::Untested
            } else {
                CapabilityDisposition::Unsupported
            }
        };
        counts.record(disposition);
        matrix.insert(capability, disposition);
    }
    (matrix, counts)
}

fn run_case(
    case: &CompatibilityCase,
    identities: &[ToolIdentity],
    repository_root: &Path,
    timeout: Duration,
) -> Result<CaseReport, io::Error> {
    let mut observations = Vec::new();
    let source = fixture_bytes(case, repository_root)?;
    let variants = cross_format_variants(case, &source);
    for tool in [ToolKind::Jq, ToolKind::Yq, ToolKind::Tq] {
        let adapter = adapter(case, tool);
        let formats = formats_for(tool, case.fixture.format, &source, variants.as_ref());
        for fixture in formats {
            let format = fixture.format;
            let Some(identity) = identities.iter().find(|identity| identity.tool == tool) else {
                observations.push(skipped(
                    tool,
                    Some(format),
                    ObservationState::Unavailable,
                    "executable not found",
                ));
                continue;
            };
            if !adapter.supported {
                observations.push(skipped(
                    tool,
                    Some(format),
                    ObservationState::Unsupported,
                    adapter
                        .note
                        .as_deref()
                        .unwrap_or("case adapter is unsupported"),
                ));
                continue;
            }
            observations.push(execute(
                case,
                adapter,
                identity,
                repository_root,
                timeout,
                fixture,
            )?);
        }
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
    fixture: ExecutionFixture,
) -> Result<ToolObservation, io::Error> {
    let ExecutionFixture {
        format: input_format,
        bytes,
        pin_format: pin_input_format,
    } = fixture;
    let mut temporary = None;
    let mut args = adapter.args.clone();
    if pin_input_format {
        match identity.tool {
            ToolKind::Jq => {}
            ToolKind::Yq => args.push(format!("--input-format={}", format_name(input_format))),
            ToolKind::Tq => args.extend([
                "--input-format".to_owned(),
                format_name(input_format).to_owned(),
            ]),
        }
    }
    if identity.tool == ToolKind::Yq
        && matches!(
            case.expected.contract,
            ContractKind::ResultSequence | ContractKind::Error
        )
        && !args.iter().any(|argument| {
            argument == "-o"
                || argument.starts_with("-o=")
                || argument == "--output-format"
                || argument.starts_with("--output-format=")
        })
    {
        args.extend(["--output-format=json".to_owned(), "--indent=0".to_owned()]);
    }
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
        environment: BTreeMap::from([("TQ_COMPAT_SENTINEL".to_owned(), "present".to_owned())]),
    });
    let outcome = match process {
        Ok(outcome) => outcome,
        Err(error) => return Ok(harness_error(identity.tool, input_format, &error)),
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
            input_format: Some(input_format),
            state: ObservationState::Executed,
            results: value.results,
            stdout_hex: Some(encode_hex(&outcome.stdout)),
            raw_stdout_hex: value.raw_bytes.as_deref().map(encode_hex),
            stderr_hex: (!value.stderr.is_empty()).then(|| encode_hex(&value.stderr)),
            process_status: Some(value.process_status),
            exit_code: value.exit_code,
            error_class: value.error_class,
            wall_time_micros: Some(outcome.wall_time_micros),
            note: None,
        }),
        Err(error) => Ok(normalization_error(
            identity.tool,
            input_format,
            &error,
            &outcome,
        )),
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

fn skipped(
    tool: ToolKind,
    input_format: Option<FixtureFormat>,
    state: ObservationState,
    note: &str,
) -> ToolObservation {
    ToolObservation {
        tool,
        input_format,
        state,
        results: Vec::new(),
        stdout_hex: None,
        raw_stdout_hex: None,
        stderr_hex: None,
        process_status: None,
        exit_code: None,
        error_class: None,
        wall_time_micros: None,
        note: Some(note.to_owned()),
    }
}

fn harness_error(
    tool: ToolKind,
    input_format: FixtureFormat,
    error: &ProcessError,
) -> ToolObservation {
    skipped(
        tool,
        Some(input_format),
        ObservationState::HarnessError,
        &error.to_string(),
    )
}

fn normalization_error(
    tool: ToolKind,
    input_format: FixtureFormat,
    error: &NormalizationError,
    outcome: &super::ProcessOutcome,
) -> ToolObservation {
    ToolObservation {
        tool,
        input_format: Some(input_format),
        state: ObservationState::HarnessError,
        results: Vec::new(),
        stdout_hex: Some(encode_hex(&outcome.stdout)),
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
            if left.input_format != right.input_format && left.tool != right.tool {
                continue;
            }
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
                    left_format: left.input_format,
                    right: right.tool,
                    right_format: right.input_format,
                    summary: fields.join(", "),
                });
            }
        }
    }
    diffs
}

#[derive(Clone)]
struct CrossFormatVariants {
    json: Vec<u8>,
    yaml: Vec<u8>,
    toon: Option<Vec<u8>>,
}

struct ExecutionFixture {
    format: FixtureFormat,
    bytes: Vec<u8>,
    pin_format: bool,
}

fn cross_format_variants(case: &CompatibilityCase, source: &[u8]) -> Option<CrossFormatVariants> {
    if case.fixture.format != FixtureFormat::Json
        || case.classification == CaseClassification::Cli
        || !matches!(
            case.invocation_mode,
            InvocationMode::Stdin | InvocationMode::File
        )
        || !matches!(
            case.expected.contract,
            ContractKind::ResultSequence | ContractKind::Error
        )
        || case
            .adapters
            .jq
            .args
            .iter()
            .chain(&case.adapters.yq.args)
            .chain(&case.adapters.tq.args)
            .any(|argument| {
                matches!(argument.as_str(), "-R" | "--raw-input" | "--stream")
                    || argument.starts_with("--input-format")
            })
    {
        return None;
    }
    let value: serde_json::Value = serde_json::from_slice(source).ok()?;
    let yaml = crate::corpus::json_to_yaml(&value)
        .and_then(|value| {
            yaml_serde::to_string(&value)
                .map(String::into_bytes)
                .map_err(|error| crate::corpus::ConversionError::Yaml(error.to_string()))
        })
        .unwrap_or_else(|_| source.to_vec());
    let toon = crate::corpus::encode_toon_exact(&value)
        .map(String::into_bytes)
        .ok();
    Some(CrossFormatVariants {
        json: source.to_vec(),
        yaml,
        toon,
    })
}

fn formats_for(
    tool: ToolKind,
    original: FixtureFormat,
    source: &[u8],
    variants: Option<&CrossFormatVariants>,
) -> Vec<ExecutionFixture> {
    let Some(variants) = variants else {
        return vec![ExecutionFixture {
            format: original,
            bytes: source.to_vec(),
            pin_format: false,
        }];
    };
    match tool {
        ToolKind::Jq => vec![ExecutionFixture {
            format: FixtureFormat::Json,
            bytes: variants.json.clone(),
            pin_format: false,
        }],
        ToolKind::Yq => vec![
            ExecutionFixture {
                format: FixtureFormat::Json,
                bytes: variants.json.clone(),
                pin_format: true,
            },
            ExecutionFixture {
                format: FixtureFormat::Yaml,
                bytes: variants.yaml.clone(),
                pin_format: true,
            },
        ],
        ToolKind::Tq => {
            let mut formats = vec![
                ExecutionFixture {
                    format: FixtureFormat::Json,
                    bytes: variants.json.clone(),
                    pin_format: true,
                },
                ExecutionFixture {
                    format: FixtureFormat::Yaml,
                    bytes: variants.yaml.clone(),
                    pin_format: true,
                },
            ];
            if let Some(toon) = &variants.toon {
                formats.push(ExecutionFixture {
                    format: FixtureFormat::Toon,
                    bytes: toon.clone(),
                    pin_format: true,
                });
            }
            formats
        }
    }
}

const fn format_name(format: FixtureFormat) -> &'static str {
    match format {
        FixtureFormat::Json => "json",
        FixtureFormat::Yaml => "yaml",
        FixtureFormat::Toon => "toon",
        FixtureFormat::Raw => "raw",
        FixtureFormat::None => "none",
    }
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{FixtureFormat, ToolKind, cross_format_variants, fixture_bytes, formats_for};
    use crate::compatibility::load_catalog;

    #[test]
    fn logical_json_case_expands_to_the_complete_native_input_matrix() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let catalog = load_catalog(&root.join("tests/compatibility/cases")).unwrap();
        let case = catalog
            .cases
            .iter()
            .find(|case| case.id == "common.identity.string")
            .unwrap();
        let source = fixture_bytes(case, &root).unwrap();
        let variants = cross_format_variants(case, &source).unwrap();

        let jq = formats_for(ToolKind::Jq, case.fixture.format, &source, Some(&variants));
        let yq = formats_for(ToolKind::Yq, case.fixture.format, &source, Some(&variants));
        let tq = formats_for(ToolKind::Tq, case.fixture.format, &source, Some(&variants));

        assert_eq!(
            jq.iter().map(|value| value.format).collect::<Vec<_>>(),
            [FixtureFormat::Json]
        );
        assert_eq!(
            yq.iter().map(|value| value.format).collect::<Vec<_>>(),
            [FixtureFormat::Json, FixtureFormat::Yaml]
        );
        assert_eq!(
            tq.iter().map(|value| value.format).collect::<Vec<_>>(),
            [
                FixtureFormat::Json,
                FixtureFormat::Yaml,
                FixtureFormat::Toon
            ]
        );
        assert!(!jq[0].pin_format);
        assert!(yq.iter().chain(&tq).all(|value| value.pin_format));
        assert!(
            jq.iter()
                .chain(&yq)
                .chain(&tq)
                .all(|value| !value.bytes.is_empty())
        );
    }
}
