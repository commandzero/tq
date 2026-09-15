//! Data-driven compatibility campaign execution.

use std::{collections::BTreeMap, fs, io, io::Write, path::Path, time::Duration};

use thiserror::Error;

mod comparison;
pub use comparison::{
    compare_manual, compare_manual_with_disparities, summarize_manual_comparison,
};

use super::normalization::toon_unframed_value_match;
use super::{
    CapabilityCounts, CapabilityDisposition, CaseAdapter, CaseClassification, CaseReport,
    CaseStatus, CompatibilityCase, CompatibilityCatalog, CompatibilityReport, ContractKind,
    CoverageCount, ExecutableConfig, FinalStatus, FixtureFormat, Invocation, InvocationMode,
    NormalizationError, ObservationState, ProcessError, SemanticDiff, ToolIdentity, ToolKind,
    ToolObservation, discover_tool, encode_hex, normalize_jq, normalize_raw,
    normalize_toon_document, normalize_toon_sequence, normalize_yq, run_process_with_environment,
    toon_values_match,
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
            let structured = matches!(
                case.expected.contract,
                ContractKind::ResultSequence | ContractKind::Error
            );
            let output_mode = output_mode_for_args(&adapter.args, structured);
            let observation = if tool == ToolKind::Tq
                && structured
                && output_mode.toon_output
                && !output_mode.toon_sequence
            {
                execute_toon_with_json_companion(
                    case,
                    adapter,
                    identity,
                    repository_root,
                    timeout,
                    fixture,
                    output_mode,
                )?
            } else {
                execute(
                    case,
                    adapter,
                    identity,
                    repository_root,
                    timeout,
                    fixture,
                    output_mode,
                )?
            };
            observations.push(observation);
        }
    }
    let semantic_diffs = semantic_diffs(&observations, case.expected.compare_stderr);
    Ok(CaseReport {
        id: case.id.clone(),
        capabilities: case.capabilities.clone(),
        observations,
        semantic_diffs,
    })
}

// One execution owns fixture materialization, process invocation, and output
// normalization so temporary files and subprocess observations stay paired.
#[allow(clippy::too_many_lines)]
fn execute(
    case: &CompatibilityCase,
    adapter: &CaseAdapter,
    identity: &ToolIdentity,
    repository_root: &Path,
    timeout: Duration,
    fixture: ExecutionFixture,
    output_mode: OutputMode,
) -> Result<ToolObservation, io::Error> {
    let resources = ExecutionResources::new(case, adapter, repository_root, &fixture)?;
    execute_with_resources(
        case,
        adapter,
        identity,
        repository_root,
        timeout,
        fixture,
        output_mode,
        &resources,
    )
    .map(|result| result.observation)
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::unnecessary_wraps,
    reason = "paired compatibility invocations share one normalization path"
)]
fn execute_with_resources(
    case: &CompatibilityCase,
    adapter: &CaseAdapter,
    identity: &ToolIdentity,
    repository_root: &Path,
    timeout: Duration,
    fixture: ExecutionFixture,
    output_mode: OutputMode,
    resources: &ExecutionResources,
) -> Result<ExecutionResult, io::Error> {
    let ExecutionFixture {
        format: input_format,
        bytes,
        pin_format: pin_input_format,
    } = fixture;
    let mut args = adapter.args.clone();
    rewrite_json_module_args(resources.module_directory.as_ref(), &mut args);
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
    if !adapter.omit_query {
        args.push(adapter.query.clone().unwrap_or_else(|| case.query.clone()));
    }
    args.extend(adapter.trailing_args.iter().cloned());
    let stdin = match case.invocation_mode {
        InvocationMode::Stdin => bytes,
        InvocationMode::NullInput => Vec::new(),
        InvocationMode::File => {
            let file = resources
                .input_file
                .as_ref()
                .expect("file invocation has a shared temporary input");
            args.push(file.path().display().to_string());
            Vec::new()
        }
    };
    let process = run_process_with_environment(
        &Invocation {
            executable: identity.path.clone(),
            args,
            stdin,
            timeout,
            current_dir: Some(repository_root.to_owned()),
            environment: BTreeMap::from([("TQ_COMPAT_SENTINEL".to_owned(), "present".to_owned())]),
        },
        &resources.environment,
    );
    let outcome = match process {
        Ok(outcome) => outcome,
        Err(error) => {
            return Ok(ExecutionResult {
                observation: harness_error(identity.tool, input_format, &error),
                outcome: None,
            });
        }
    };
    let normalized = match case.expected.contract {
        ContractKind::RawBytes | ContractKind::ExitStatus => {
            Ok(normalize_raw(identity.tool, &outcome))
        }
        ContractKind::ResultSequence | ContractKind::Error => match identity.tool {
            ToolKind::Jq if sequence_flag_for_args(&adapter.args) => {
                normalize_json_sequence(&outcome)
            }
            ToolKind::Jq => normalize_jq(&outcome),
            ToolKind::Yq => normalize_yq(&outcome),
            ToolKind::Tq if output_mode.json_output => {
                let normalized = if output_mode.json_sequence {
                    normalize_json_sequence(&outcome)
                } else {
                    normalize_jq(&outcome)
                };
                normalized.map(|mut value| {
                    value.error_class = super::classify_process(ToolKind::Tq, &outcome);
                    value
                })
            }
            ToolKind::Tq if output_mode.toon_sequence => normalize_toon_sequence(&outcome),
            ToolKind::Tq => normalize_toon_document(&outcome),
        },
    };
    match normalized {
        Ok(value) => Ok(ExecutionResult {
            observation: ToolObservation {
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
            },
            outcome: Some(outcome),
        }),
        Err(error) => Ok(ExecutionResult {
            observation: normalization_error(identity.tool, input_format, &error, &outcome),
            outcome: Some(outcome),
        }),
    }
}

struct ExecutionResult {
    observation: ToolObservation,
    outcome: Option<super::ProcessOutcome>,
}

// TOON document output has no framing marker between adjacent results. Run
// the same tq invocation once with JSON output to obtain result boundaries,
// then verify the unchanged TOON bytes against those values. A failed
// companion is never used to manufacture a TOON result sequence.
#[allow(
    clippy::too_many_arguments,
    reason = "paired output observations share one invocation"
)]
fn execute_toon_with_json_companion(
    case: &CompatibilityCase,
    adapter: &CaseAdapter,
    identity: &ToolIdentity,
    repository_root: &Path,
    timeout: Duration,
    fixture: ExecutionFixture,
    output_mode: OutputMode,
) -> Result<ToolObservation, io::Error> {
    let resources = ExecutionResources::new(case, adapter, repository_root, &fixture)?;
    let toon_result = execute_with_resources(
        case,
        adapter,
        identity,
        repository_root,
        timeout,
        fixture.clone(),
        output_mode,
        &resources,
    )?;
    let mut toon = toon_result.observation;
    let Some(toon_outcome) = toon_result.outcome else {
        return Ok(toon);
    };
    if toon_outcome.status != super::ProcessStatus::Exited {
        return Ok(toon);
    }

    let mut json_adapter = adapter.clone();
    comparison::force_output_format_args(&mut json_adapter.args, "json");
    let json_result = execute_with_resources(
        case,
        &json_adapter,
        identity,
        repository_root,
        timeout,
        fixture,
        output_mode_for_args(&json_adapter.args, true),
        &resources,
    )?;
    let Some(json_outcome) = json_result.outcome.as_ref() else {
        return Ok(companion_error(
            toon,
            "companion tq JSON execution did not produce a result sequence",
        ));
    };
    let json = json_result.observation;
    let process_contract_matches = json.state == ObservationState::Executed
        && json_outcome.status == super::ProcessStatus::Exited
        && json_outcome.status == toon_outcome.status
        && json_outcome.exit_code == toon_outcome.exit_code
        && super::classify_process(ToolKind::Tq, json_outcome)
            == super::classify_process(ToolKind::Tq, &toon_outcome);
    if !process_contract_matches {
        return Ok(companion_error(
            toon,
            "companion tq JSON execution did not produce a successful result sequence",
        ));
    }
    let output_matches = if output_mode.toon_unframed {
        toon_unframed_value_match(&toon_outcome.stdout, &json.results)
    } else {
        toon_values_match(&toon_outcome.stdout, &json.results)
    };
    let matching_empty_failure = output_mode.toon_unframed
        && toon_outcome.exit_code.is_some_and(|code| code != 0)
        && toon_outcome.stdout.is_empty()
        && json_outcome.stdout.is_empty()
        && json.results.is_empty();
    if !output_matches && !matching_empty_failure {
        return Ok(companion_error(
            toon,
            "TOON stdout does not match companion tq JSON results",
        ));
    }

    toon.results = json.results;
    toon.state = ObservationState::Executed;
    toon.raw_stdout_hex = None;
    toon.error_class = super::classify_process(ToolKind::Tq, &toon_outcome);
    toon.note = None;
    Ok(toon)
}

fn companion_error(mut observation: ToolObservation, note: &str) -> ToolObservation {
    observation.state = ObservationState::HarnessError;
    observation.results.clear();
    if observation.stdout_hex.is_some() {
        observation
            .raw_stdout_hex
            .clone_from(&observation.stdout_hex);
    }
    observation.error_class = Some(super::ErrorClass::MalformedOutput);
    observation.note = Some(note.to_owned());
    observation
}

fn fixture_environment(
    adapter: &CaseAdapter,
    repository_root: &Path,
) -> Result<BTreeMap<String, String>, io::Error> {
    let mut environment = adapter.env.clone();
    let root = repository_root.canonicalize()?;
    for (name, relative) in &adapter.env_paths {
        if environment.contains_key(name)
            || relative.as_os_str().is_empty()
            || relative
                .components()
                .any(|part| !matches!(part, std::path::Component::Normal(_)))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid or conflicting fixture environment path: {name}"),
            ));
        }
        let path = root.join(relative).canonicalize()?;
        if !path.starts_with(&root) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("fixture environment path escapes repository: {name}"),
            ));
        }
        environment.insert(
            name.clone(),
            path.to_str()
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "fixture environment path is not UTF-8",
                    )
                })?
                .to_owned(),
        );
    }
    Ok(environment)
}

struct ExecutionResources {
    module_directory: Option<tempfile::TempDir>,
    input_file: Option<tempfile::NamedTempFile>,
    _empty_home: tempfile::TempDir,
    environment: BTreeMap<String, String>,
}

impl ExecutionResources {
    fn new(
        case: &CompatibilityCase,
        adapter: &CaseAdapter,
        repository_root: &Path,
        fixture: &ExecutionFixture,
    ) -> io::Result<Self> {
        let module_directory = materialize_json_module(case, repository_root)?;
        let input_file = if matches!(case.invocation_mode, InvocationMode::File) {
            let mut file = tempfile::NamedTempFile::new()?;
            file.write_all(&fixture.bytes)?;
            Some(file)
        } else {
            None
        };
        let empty_home = tempfile::tempdir()?;
        let mut environment = fixture_environment(adapter, repository_root)?;
        if !environment.contains_key("HOME") {
            environment.insert(
                "HOME".to_owned(),
                empty_home.path().to_string_lossy().into_owned(),
            );
        }
        Ok(Self {
            module_directory,
            input_file,
            _empty_home: empty_home,
            environment,
        })
    }
}

fn materialize_json_module(
    case: &CompatibilityCase,
    repository_root: &Path,
) -> io::Result<Option<tempfile::TempDir>> {
    if case.id != "manual.modules.import-json" {
        return Ok(None);
    }
    let directory = tempfile::tempdir()?;
    let value: serde_json::Value = crate::fixture_data::read(
        &repository_root.join("tests/fixtures/manual-modules/data.toon"),
    )?;
    fs::write(
        directory.path().join("data.json"),
        serde_json::to_vec(&value)?,
    )?;
    Ok(Some(directory))
}

fn rewrite_json_module_args(module_directory: Option<&tempfile::TempDir>, args: &mut [String]) {
    let Some(module_directory) = module_directory else {
        return;
    };
    for argument in args {
        if argument == "tests/fixtures/manual-modules" {
            *argument = module_directory.path().display().to_string();
        }
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

fn semantic_diffs(observations: &[ToolObservation], compare_stderr: bool) -> Vec<SemanticDiff> {
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
            if compare_stderr && left.stderr_hex != right.stderr_hex {
                fields.push("raw stderr");
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

#[derive(Clone)]
struct ExecutionFixture {
    format: FixtureFormat,
    bytes: Vec<u8>,
    pin_format: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "the CLI has independent JSON, TOON, raw, and sequence switches"
)]
struct OutputMode {
    json_output: bool,
    json_lines: bool,
    json_sequence: bool,
    toon_output: bool,
    toon_sequence: bool,
    toon_unframed: bool,
}

fn option_value_count(argument: &str) -> usize {
    match argument {
        "-f"
        | "--from-file"
        | "-i"
        | "--input-format"
        | "-o"
        | "--output-format"
        | "--indent"
        | "--delimiter"
        | "--flatten-depth"
        | "-L"
        | "--library-path"
        | "--trace-limit"
        | "--report-file"
        | "--max-input-bytes"
        | "--max-depth"
        | "--max-token-bytes"
        | "--max-line-bytes"
        | "--max-frame-bytes"
        | "--max-fields"
        | "--max-lookahead-bytes"
        | "--max-vm-steps"
        | "--max-results"
        | "--max-output-bytes"
        | "--prepare-memory-bytes"
        | "--hybrid-batch-values"
        | "--hybrid-in-flight-batches"
        | "--hybrid-in-flight-bytes"
        | "--decode-batch-values"
        | "--decode-batch-bytes"
        | "--decode-in-flight-batches"
        | "--decode-in-flight-bytes"
        | "--max-spool-bytes"
        | "--run-tests" => 1,
        "--arg" | "--argjson" | "--argtoon" | "--slurpfile" | "--rawfile" => 2,
        _ => 0,
    }
}

fn sequence_flag_for_args(args: &[String]) -> bool {
    let expanded_args = comparison::expand_short_comparison_options(args);
    let mut arguments = expanded_args.iter();
    while let Some(argument) = arguments.next() {
        if argument == "--" || !argument.starts_with('-') {
            break;
        }
        if argument == "--seq" {
            return true;
        }
        for _ in 0..option_value_count(argument) {
            arguments.next();
        }
    }
    false
}

fn output_mode_for_args(args: &[String], structured: bool) -> OutputMode {
    let mut output = None;
    let mut compact = false;
    let mut sequence = false;
    let mut raw = false;
    let mut unframed = false;
    let expanded_args = comparison::expand_short_comparison_options(args);
    let mut arguments = expanded_args.iter();
    while let Some(argument) = arguments.next() {
        if argument == "--" || !argument.starts_with('-') {
            break;
        }
        if matches!(argument.as_str(), "-o" | "--output-format") {
            output = arguments.next().map(String::as_str);
            continue;
        }
        let value_count = option_value_count(argument);
        if value_count > 0 {
            for _ in 0..value_count {
                arguments.next();
            }
            continue;
        }
        match argument.as_str() {
            "--seq" => sequence = true,
            "--unframed" => unframed = true,
            "-c" | "--compact-output" => compact = true,
            "-r" | "--raw-output" | "--raw-output0" | "-j" | "--join-output" => raw = true,
            value if value.starts_with("--output-format=") => {
                output = value.split_once('=').map(|(_, value)| value);
            }
            value if value.starts_with("-o=") => output = Some(&value[3..]),
            value if value.starts_with("-o") && value.len() > 2 => output = Some(&value[2..]),
            value if value.starts_with('-') && !value.starts_with("--") => {
                for short in value[1..].chars() {
                    match short {
                        'c' => compact = true,
                        'r' | 'j' => raw = true,
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    let output = output.map(|name| {
        tq_formats::NativeFormat::from_name(name).map_or(name, |format| format.descriptor().name)
    });
    let json_output = !raw
        && (matches!(output, Some("json" | "jsonl" | "ndjson" | "json-seq"))
            || output.is_none() && compact);
    let json_lines = json_output && output == Some("jsonl");
    let json_sequence = json_output
        && (matches!(output, Some("json-seq"))
            || sequence && output.is_none_or(|value| value == "json"));
    let toon_output = structured
        && !raw
        && !json_output
        && (output.is_none() || matches!(output, Some("toon" | "toon-seq")));
    let toon_sequence =
        structured && !raw && toon_output && (sequence || matches!(output, Some("toon-seq")));
    let toon_unframed = structured && toon_output && unframed;
    OutputMode {
        json_output,
        json_lines,
        json_sequence,
        toon_output,
        toon_sequence,
        toon_unframed,
    }
}

fn normalize_json_sequence(
    outcome: &super::ProcessOutcome,
) -> Result<super::NormalizedObservation, NormalizationError> {
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
    let documents = tq_formats::decode_json_sequence(complete_stdout, "<tq-compatibility-output>")
        .map_err(|error| NormalizationError::Jq(error.to_string()))?;
    let mut unframed_stdout = Vec::with_capacity(complete_stdout.len());
    for document in documents {
        let value = document
            .value
            .to_json()
            .map_err(|error| NormalizationError::Jq(error.to_string()))?;
        serde_json::to_writer(&mut unframed_stdout, &value)
            .map_err(|error| NormalizationError::Jq(error.to_string()))?;
        unframed_stdout.push(b'\n');
    }
    let mut unframed = outcome.clone();
    unframed.stdout = unframed_stdout;
    normalize_jq(&unframed)
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

    #[cfg(unix)]
    use super::format_name;
    use super::{
        FixtureFormat, ToolKind, cross_format_variants, fixture_bytes, formats_for,
        normalize_json_sequence, output_mode_for_args, sequence_flag_for_args,
    };
    use crate::compatibility::load_catalog;
    #[cfg(unix)]
    use crate::compatibility::{
        CaseAdapter, CaseClassification, CaseFixture, CaseStatus, CompatibilityCase, ContractKind,
        ExpectedContract, InvocationMode, ToolAdapters, ToolIdentity,
    };
    use crate::compatibility::{ProcessOutcome, ProcessStatus};
    #[cfg(unix)]
    use crate::corpus::ArtifactIdentity;

    #[cfg(unix)]
    use std::{collections::BTreeMap, fs, os::unix::fs::PermissionsExt, time::Duration};

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

    #[test]
    fn result_contract_normalization_follows_explicit_json_selection() {
        let mode = output_mode_for_args(
            &[
                "--input-format".to_owned(),
                "json".to_owned(),
                "--output-format".to_owned(),
                "json".to_owned(),
            ],
            true,
        );

        assert!(mode.json_output);
        assert!(!mode.json_sequence);
        assert!(!mode.toon_sequence);

        let sequence = output_mode_for_args(
            &["--seq".to_owned(), "--output-format=json".to_owned()],
            true,
        );
        assert!(sequence.json_output);
        assert!(sequence.json_sequence);
        assert!(!sequence.toon_sequence);
    }

    #[test]
    fn result_contract_normalization_keeps_default_toon_selection() {
        let mode = output_mode_for_args(&[], true);

        assert!(!mode.json_output);
        assert!(!mode.json_sequence);
        assert!(!mode.toon_sequence);
    }

    #[test]
    fn result_contract_normalization_selects_toon_sequence_only_explicitly() {
        for arguments in [
            vec!["-o".to_owned(), "toon".to_owned()],
            vec!["--output-format=toon".to_owned()],
        ] {
            let mode = output_mode_for_args(&arguments, true);
            assert!(!mode.toon_sequence, "{arguments:?}");
        }

        for arguments in [
            vec!["--seq".to_owned()],
            vec!["-o".to_owned(), "toon-seq".to_owned()],
            vec!["--output-format=toon-sequence".to_owned()],
        ] {
            let mode = output_mode_for_args(&arguments, true);
            assert!(mode.toon_sequence, "{arguments:?}");
        }
    }

    #[test]
    fn output_mode_scanner_keeps_compact_json_and_raw_output_out_of_toon_sequence() {
        let compact = output_mode_for_args(&["-c".to_owned()], true);
        assert!(compact.json_output);
        assert!(!compact.json_sequence);
        assert!(!compact.toon_sequence);

        let raw = output_mode_for_args(&["-r".to_owned()], true);
        assert!(!raw.json_output);
        assert!(!raw.json_sequence);
        assert!(!raw.toon_sequence);
    }

    #[test]
    fn json_lines_alias_does_not_claim_json_sequence_framing() {
        let mode = output_mode_for_args(
            &[
                "--seq".to_owned(),
                "--output-format".to_owned(),
                "jsonl".to_owned(),
            ],
            true,
        );

        assert!(mode.json_output);
        assert!(!mode.json_sequence);
        assert!(!mode.toon_sequence);
    }

    #[test]
    fn output_mode_scanner_accepts_catalog_sequence_aliases() {
        for (name, json) in [("jsonseq", true), ("toon-sequence", false)] {
            for arguments in [
                vec!["--output-format".to_owned(), name.to_owned()],
                vec![format!("--output-format={name}")],
                vec![format!("-o{name}")],
            ] {
                let mode = output_mode_for_args(&arguments, true);
                assert_eq!(mode.json_output, json, "{arguments:?}");
                assert_eq!(mode.json_sequence, json, "{arguments:?}");
                assert_eq!(mode.toon_sequence, !json, "{arguments:?}");
            }
        }
    }

    #[test]
    fn output_mode_scanner_skips_option_values_that_look_like_flags() {
        let mode = output_mode_for_args(
            &[
                "--arg".to_owned(),
                "name".to_owned(),
                "-c".to_owned(),
                "value".to_owned(),
            ],
            true,
        );

        assert!(!mode.json_output);
        assert!(!mode.json_sequence);
        assert!(!mode.toon_sequence);
    }

    #[test]
    fn output_mode_scanner_respects_attached_option_values_and_boundaries() {
        for arguments in [
            vec!["-ijson".to_owned()],
            vec!["-fscript".to_owned()],
            vec![".".to_owned(), "-r".to_owned()],
            vec!["--".to_owned(), "-r".to_owned()],
        ] {
            let mode = output_mode_for_args(&arguments, true);
            assert!(!mode.json_output, "{arguments:?}");
            assert!(mode.toon_output, "{arguments:?}");
            assert!(!mode.toon_sequence, "{arguments:?}");
        }

        let mode = output_mode_for_args(&["-nr".to_owned()], true);
        assert!(!mode.json_output);
        assert!(!mode.toon_output);
        assert!(!mode.toon_sequence);
    }

    #[test]
    fn output_mode_scanner_accepts_equals_on_attached_output_values() {
        let toon = output_mode_for_args(&["-o=toon".to_owned()], true);
        assert!(toon.toon_output);
        assert!(!toon.toon_sequence);

        let json_sequence = output_mode_for_args(&["-o=json-seq".to_owned()], true);
        assert!(json_sequence.json_output);
        assert!(json_sequence.json_sequence);
        assert!(!json_sequence.toon_sequence);
    }

    #[test]
    fn jq_sequence_scanner_respects_option_values_and_query_boundary() {
        assert!(sequence_flag_for_args(&["--seq".to_owned()]));
        assert!(!sequence_flag_for_args(&[
            "--arg".to_owned(),
            "name".to_owned(),
            "--seq".to_owned(),
        ]));
        assert!(!sequence_flag_for_args(&[
            ".".to_owned(),
            "--seq".to_owned()
        ]));
        assert!(!sequence_flag_for_args(&[
            "--".to_owned(),
            "--seq".to_owned()
        ]));
    }

    #[cfg(unix)]
    #[test]
    fn yaml_and_toon_inputs_are_not_reframed_as_json_sequences() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let executable = directory.path().join("tq");
        fs::write(
            &executable,
            "#!/bin/sh\njson=false\nprevious=''\nfor argument in \"$@\"; do\n  if [ \"$argument\" = --seq ]; then exit 9; fi\n  if [ \"$previous\" = -o ] && [ \"$argument\" = json ]; then json=true; fi\n  previous=\"$argument\"\ndone\ncat >/dev/null\nif [ \"$json\" = true ]; then printf '{\"a\":1}\\n'; else printf 'a: 1\\n'; fi\n",
        )
        .expect("write fake executable");
        let mut permissions = fs::metadata(&executable)
            .expect("fake executable metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&executable, permissions).expect("make fake executable");

        let identity = ToolIdentity {
            tool: ToolKind::Tq,
            path: executable,
            version: "test".to_owned(),
            executable: ArtifactIdentity {
                path: "tq".to_owned(),
                bytes: 0,
                sha256: String::new(),
            },
            build_features: Vec::new(),
            runtime_libraries: Vec::new(),
        };
        for (format, bytes) in [
            (FixtureFormat::Yaml, b"a: 1\n".to_vec()),
            (FixtureFormat::Toon, b"a: 1\n".to_vec()),
        ] {
            let case = CompatibilityCase {
                schema_version: 1,
                id: format!("runner.{format:?}"),
                title: "document input".to_owned(),
                classification: CaseClassification::Common,
                capabilities: vec!["runner.output-mode".to_owned()],
                status: CaseStatus::Mvp,
                fixture: CaseFixture {
                    format,
                    inline: None,
                    path: None,
                },
                query: ".".to_owned(),
                adapters: ToolAdapters {
                    tq: CaseAdapter {
                        args: vec!["--input-format".to_owned(), format_name(format).to_owned()],
                        supported: true,
                        ..CaseAdapter::default()
                    },
                    ..ToolAdapters::default()
                },
                invocation_mode: InvocationMode::Stdin,
                expected: ExpectedContract {
                    contract: ContractKind::ResultSequence,
                    baseline: super::super::BaselinePolicy::NotApplicable,
                    error_class: None,
                    compare_stderr: false,
                    tq_contract: None,
                },
            };
            let adapter = &case.adapters.tq;
            let observation = super::execute(
                &case,
                adapter,
                &identity,
                directory.path(),
                Duration::from_secs(2),
                super::ExecutionFixture {
                    format,
                    bytes,
                    pin_format: false,
                },
                output_mode_for_args(&adapter.args, true),
            )
            .expect("runner invocation");
            assert_eq!(observation.exit_code, Some(0), "{format:?}");
            assert_eq!(observation.results, [serde_json::json!({"a": 1})]);
        }
    }

    #[cfg(unix)]
    #[allow(
        clippy::too_many_lines,
        reason = "keeps paired fake tool scripts and cardinality assertions together"
    )]
    #[test]
    fn companion_json_rejects_false_toon_cardinality() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let executable = directory.path().join("tq");
        fs::write(
            &executable,
            "#!/bin/sh\njson=false\nprevious=''\nfor argument in \"$@\"; do\n  if [ \"$previous\" = -o ] && [ \"$argument\" = json ]; then json=true; fi\n  previous=\"$argument\"\ndone\nif [ \"$json\" = true ]; then printf '{\"a\":1,\"b\":2}\\n'; else printf 'a: 1\\nb: 2\\n'; fi\n",
        )
        .expect("write fake executable");
        let mut permissions = fs::metadata(&executable)
            .expect("fake executable metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&executable, permissions).expect("make fake executable");
        let jq_executable = directory.path().join("jq");
        fs::write(
            &jq_executable,
            "#!/bin/sh\nprintf '{\"a\":1}\\n{\"b\":2}\\n'",
        )
        .expect("write fake jq executable");
        let mut permissions = fs::metadata(&jq_executable)
            .expect("fake jq executable metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&jq_executable, permissions).expect("make fake jq executable");

        let case = CompatibilityCase {
            schema_version: 1,
            id: "runner.companion.false-cardinality".to_owned(),
            title: "companion JSON cardinality".to_owned(),
            classification: CaseClassification::Common,
            capabilities: vec!["runner.output-mode".to_owned()],
            status: CaseStatus::Mvp,
            fixture: CaseFixture {
                format: FixtureFormat::Yaml,
                inline: Some("a: 1\nb: 2\n".to_owned()),
                path: None,
            },
            query: ".".to_owned(),
            adapters: ToolAdapters {
                jq: CaseAdapter {
                    supported: true,
                    ..CaseAdapter::default()
                },
                tq: CaseAdapter {
                    args: vec!["--input-format".to_owned(), "yaml".to_owned()],
                    supported: true,
                    ..CaseAdapter::default()
                },
                ..ToolAdapters::default()
            },
            invocation_mode: InvocationMode::Stdin,
            expected: ExpectedContract {
                contract: ContractKind::ResultSequence,
                baseline: super::super::BaselinePolicy::NotApplicable,
                error_class: None,
                compare_stderr: false,
                tq_contract: None,
            },
        };
        let jq_identity = ToolIdentity {
            tool: ToolKind::Jq,
            path: jq_executable,
            version: "test".to_owned(),
            executable: ArtifactIdentity {
                path: "jq".to_owned(),
                bytes: 0,
                sha256: String::new(),
            },
            build_features: Vec::new(),
            runtime_libraries: Vec::new(),
        };
        let tq_identity = ToolIdentity {
            tool: ToolKind::Tq,
            path: executable,
            version: "test".to_owned(),
            executable: ArtifactIdentity {
                path: "tq".to_owned(),
                bytes: 0,
                sha256: String::new(),
            },
            build_features: Vec::new(),
            runtime_libraries: Vec::new(),
        };

        let report = super::run_case(
            &case,
            &[jq_identity, tq_identity],
            directory.path(),
            Duration::from_secs(2),
        )
        .expect("runner case");
        let observation = report
            .observations
            .iter()
            .find(|observation| observation.tool == ToolKind::Tq)
            .expect("tq observation");
        assert_eq!(observation.state, super::ObservationState::Executed);
        assert_eq!(observation.results, [serde_json::json!({"a": 1, "b": 2})]);
        assert_eq!(
            observation.stdout_hex,
            Some(super::encode_hex(b"a: 1\nb: 2\n"))
        );
        assert_eq!(observation.error_class, None);
        assert_eq!(report.semantic_diffs.len(), 1);
        assert_eq!(report.semantic_diffs[0].summary, "result sequence");
    }

    #[cfg(unix)]
    #[test]
    fn companion_json_status_mismatch_keeps_original_toon_failure() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let executable = directory.path().join("tq");
        fs::write(
            &executable,
            "#!/bin/sh\njson=false\nprevious=''\nfor argument in \"$@\"; do\n  if [ \"$previous\" = -o ] && [ \"$argument\" = json ]; then json=true; fi\n  previous=\"$argument\"\ndone\nif [ \"$json\" = true ]; then exit 7; fi\nprintf 'a: 1\\n'\n",
        )
        .expect("write fake executable");
        let mut permissions = fs::metadata(&executable)
            .expect("fake executable metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&executable, permissions).expect("make fake executable");

        let case = CompatibilityCase {
            schema_version: 1,
            id: "runner.companion.status-mismatch".to_owned(),
            title: "companion JSON status mismatch".to_owned(),
            classification: CaseClassification::Common,
            capabilities: vec!["runner.output-mode".to_owned()],
            status: CaseStatus::Mvp,
            fixture: CaseFixture {
                format: FixtureFormat::Yaml,
                inline: Some("a: 1\n".to_owned()),
                path: None,
            },
            query: ".".to_owned(),
            adapters: ToolAdapters {
                tq: CaseAdapter {
                    args: vec!["--input-format".to_owned(), "yaml".to_owned()],
                    supported: true,
                    ..CaseAdapter::default()
                },
                ..ToolAdapters::default()
            },
            invocation_mode: InvocationMode::Stdin,
            expected: ExpectedContract {
                contract: ContractKind::ResultSequence,
                baseline: super::super::BaselinePolicy::NotApplicable,
                error_class: None,
                compare_stderr: false,
                tq_contract: None,
            },
        };
        let identity = ToolIdentity {
            tool: ToolKind::Tq,
            path: executable,
            version: "test".to_owned(),
            executable: ArtifactIdentity {
                path: "tq".to_owned(),
                bytes: 0,
                sha256: String::new(),
            },
            build_features: Vec::new(),
            runtime_libraries: Vec::new(),
        };

        let report = super::run_case(&case, &[identity], directory.path(), Duration::from_secs(2))
            .expect("runner case");
        let observation = report
            .observations
            .iter()
            .find(|observation| observation.tool == ToolKind::Tq)
            .expect("tq observation");
        assert_eq!(observation.state, super::ObservationState::HarnessError);
        assert_eq!(observation.process_status, Some(ProcessStatus::Exited));
        assert_eq!(observation.exit_code, Some(0));
        assert_eq!(observation.stdout_hex, Some(super::encode_hex(b"a: 1\n")));
        assert_eq!(
            observation.raw_stdout_hex,
            Some(super::encode_hex(b"a: 1\n"))
        );
        assert_eq!(
            observation.error_class,
            Some(super::super::ErrorClass::MalformedOutput)
        );
    }

    #[cfg(unix)]
    #[test]
    fn companion_invocations_share_home_and_file_input_path() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let executable = directory.path().join("tq");
        let probe = directory.path().join("invocations.log");
        fs::write(
            &executable,
            "#!/bin/sh\njson=false\nprevious=''\ninput=''\nfor argument in \"$@\"; do\n  if [ \"$previous\" = -o ] && [ \"$argument\" = json ]; then json=true; fi\n  input=\"$argument\"\n  previous=\"$argument\"\ndone\nprintf '%s\\t%s\\t%s\\n' \"$json\" \"$HOME\" \"$input\" >> \"$PROBE_LOG\"\nif [ \"$json\" = true ]; then printf '{\"a\":1}\\n'; else printf 'a: 1\\n'; fi\n",
        )
        .expect("write fake executable");
        let mut permissions = fs::metadata(&executable)
            .expect("fake executable metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&executable, permissions).expect("make fake executable");

        let case = CompatibilityCase {
            schema_version: 1,
            id: "runner.companion.shared-resources".to_owned(),
            title: "companion shared resources".to_owned(),
            classification: CaseClassification::Common,
            capabilities: vec!["runner.output-mode".to_owned()],
            status: CaseStatus::Mvp,
            fixture: CaseFixture {
                format: FixtureFormat::Yaml,
                inline: Some("a: 1\n".to_owned()),
                path: None,
            },
            query: ".".to_owned(),
            adapters: ToolAdapters {
                tq: CaseAdapter {
                    args: vec!["--input-format".to_owned(), "yaml".to_owned()],
                    env: BTreeMap::from([("PROBE_LOG".to_owned(), probe.display().to_string())]),
                    supported: true,
                    ..CaseAdapter::default()
                },
                ..ToolAdapters::default()
            },
            invocation_mode: InvocationMode::File,
            expected: ExpectedContract {
                contract: ContractKind::ResultSequence,
                baseline: super::super::BaselinePolicy::NotApplicable,
                error_class: None,
                compare_stderr: false,
                tq_contract: None,
            },
        };
        let identity = ToolIdentity {
            tool: ToolKind::Tq,
            path: executable,
            version: "test".to_owned(),
            executable: ArtifactIdentity {
                path: "tq".to_owned(),
                bytes: 0,
                sha256: String::new(),
            },
            build_features: Vec::new(),
            runtime_libraries: Vec::new(),
        };

        let report = super::run_case(&case, &[identity], directory.path(), Duration::from_secs(2))
            .expect("runner case");
        let observation = report
            .observations
            .iter()
            .find(|observation| observation.tool == ToolKind::Tq)
            .expect("tq observation");
        assert_eq!(observation.state, super::ObservationState::Executed);
        let entries = fs::read_to_string(&probe).expect("shared-resource probe log");
        let rows = entries
            .lines()
            .map(|line| line.split('\t').collect::<Vec<_>>())
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 2);
        assert_ne!(rows[0][0], rows[1][0]);
        assert_eq!(rows[0][1], rows[1][1]);
        assert_eq!(rows[0][2], rows[1][2]);
    }

    #[cfg(unix)]
    #[test]
    fn explicit_toon_sequence_output_normalizes_multiple_document_results() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let executable = directory.path().join("tq");
        fs::write(
            &executable,
            "#!/bin/sh\nfor argument in \"$@\"; do\n  if [ \"$argument\" = --seq ]; then exit 9; fi\ndone\ncat >/dev/null\nprintf '\\036a: 1\\n\\036b: 2\\n'\n",
        )
        .expect("write fake executable");
        let mut permissions = fs::metadata(&executable)
            .expect("fake executable metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&executable, permissions).expect("make fake executable");

        let case = CompatibilityCase {
            schema_version: 1,
            id: "runner.explicit-toon-sequence".to_owned(),
            title: "explicit TOON sequence".to_owned(),
            classification: CaseClassification::Common,
            capabilities: vec!["runner.output-mode".to_owned()],
            status: CaseStatus::Mvp,
            fixture: CaseFixture {
                format: FixtureFormat::Yaml,
                inline: None,
                path: None,
            },
            query: ".".to_owned(),
            adapters: ToolAdapters {
                tq: CaseAdapter {
                    args: vec![
                        "--input-format".to_owned(),
                        "yaml".to_owned(),
                        "--output-format".to_owned(),
                        "toon-seq".to_owned(),
                    ],
                    supported: true,
                    ..CaseAdapter::default()
                },
                ..ToolAdapters::default()
            },
            invocation_mode: InvocationMode::Stdin,
            expected: ExpectedContract {
                contract: ContractKind::ResultSequence,
                baseline: super::super::BaselinePolicy::NotApplicable,
                error_class: None,
                compare_stderr: false,
                tq_contract: None,
            },
        };
        let adapter = &case.adapters.tq;
        let identity = ToolIdentity {
            tool: ToolKind::Tq,
            path: executable,
            version: "test".to_owned(),
            executable: ArtifactIdentity {
                path: "tq".to_owned(),
                bytes: 0,
                sha256: String::new(),
            },
            build_features: Vec::new(),
            runtime_libraries: Vec::new(),
        };
        let observation = super::execute(
            &case,
            adapter,
            &identity,
            directory.path(),
            Duration::from_secs(2),
            super::ExecutionFixture {
                format: FixtureFormat::Yaml,
                bytes: b"a: 1\n".to_vec(),
                pin_format: false,
            },
            output_mode_for_args(&adapter.args, true),
        )
        .expect("runner invocation");

        assert_eq!(observation.exit_code, Some(0));
        assert_eq!(
            observation.results,
            [serde_json::json!({"a": 1}), serde_json::json!({"b": 2})]
        );
    }

    fn outcome(stdout: &[u8]) -> ProcessOutcome {
        ProcessOutcome {
            status: ProcessStatus::Exited,
            exit_code: Some(0),
            signal: None,
            stdout: stdout.to_vec(),
            stderr: Vec::new(),
            wall_time_micros: 1,
            recorded_command: Vec::new(),
        }
    }

    #[test]
    fn json_sequence_normalization_accepts_multiline_records() {
        let normalized = normalize_json_sequence(&outcome(
            b"\x1e{\n  \"name\": \"Ada\",\n  \"values\": [1, true]\n}\n",
        ))
        .expect("multiline JSON Text Sequence");

        assert_eq!(
            normalized.results,
            [serde_json::json!({"name": "Ada", "values": [1, true]})]
        );
    }

    #[test]
    fn json_sequence_normalization_accepts_empty_and_multiple_records() {
        let empty = normalize_json_sequence(&outcome(b"")).expect("empty sequence");
        assert_eq!(empty.results, [] as [serde_json::Value; 0]);

        let multiple = normalize_json_sequence(&outcome(b"\x1e1\n\x1e{\"ok\":true}\n"))
            .expect("multiple JSON Text Sequence records");
        assert_eq!(
            multiple.results,
            [serde_json::json!(1), serde_json::json!({"ok": true})]
        );
    }

    #[test]
    fn json_sequence_normalization_rejects_malformed_framing() {
        for stdout in [b"1\n".as_slice(), b"\x1e1\n2\x1e4\n".as_slice()] {
            assert!(
                normalize_json_sequence(&outcome(stdout)).is_err(),
                "malformed JSON Text Sequence should fail"
            );
        }
    }
}
