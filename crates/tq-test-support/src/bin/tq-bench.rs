//! Local correctness-gated benchmark campaign driver.

#[path = "tq-bench/calibration.rs"]
mod calibration;

use std::{
    collections::BTreeMap,
    env, fs,
    io::Write as _,
    path::{Path, PathBuf},
    process::ExitCode,
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};

use sha2::{Digest, Sha256};
use tempfile::TempDir;
use tq_test_support::{
    benchmark::{
        BenchmarkAdapter, BenchmarkCampaignReport, BenchmarkCase, BenchmarkCorpusIdentity,
        BenchmarkFinalStatus, BenchmarkInvocation, BenchmarkOutcome, BenchmarkRow,
        BenchmarkRunnerError, BenchmarkSampling, BenchmarkTool, Comparability, DatasetTier,
        InputFormat, RegressionGate, RegressionThresholds, collect_environment, compare_reports,
        correctness_witness_limit_row, evaluate_regression, is_correctness_output_limit,
        load_benchmark_catalog, normalize_correctness_run, populate_reference_ratios,
        preflight_rss, render_markdown_campaigns, render_markdown_pages,
        run_correctness_limit_probe, run_gated_row, unsupported_row,
    },
    compatibility::{ExecutableConfig, ToolIdentity, ToolKind, discover_tool},
    corpus::{
        ArtifactIdentity, SmokeSnapshot, SnapshotManifest, discover_latest_validated_manifests,
        discover_smoke_corpus, generate_representations, load_frozen_snapshot,
    },
};

const RAPID_CASES: &[&str] = &[
    "benchmark.identity-reencode",
    "benchmark.parse-discard",
    "benchmark.scalar-extraction",
    "benchmark.path-update",
    "benchmark.event-stream",
];
const RAPID_SOURCE: &str = "usgs-all-month";
const DEEP_MERGE_ENTRIES: usize = 10_000;

#[path = "tq-bench/native_rows.rs"]
mod native_rows;

fn main() -> ExitCode {
    match run() {
        Ok(status) => status,
        Err(error) => {
            eprintln!("tq-bench: {error}");
            ExitCode::from(2)
        }
    }
}

struct Options {
    profile: String,
    output: PathBuf,
    manifests: Vec<PathBuf>,
    cache_root: PathBuf,
    origin: String,
    max_samples: Option<usize>,
    timeout_seconds: Option<u64>,
    rss_limit_bytes: Option<u64>,
    selected_cases: Vec<String>,
    baseline: Option<PathBuf>,
    markdown_dir: Option<PathBuf>,
    render_only: Vec<PathBuf>,
    preflight_only: bool,
    timing_calibrations: Vec<PathBuf>,
    regression_thresholds: RegressionThresholds,
}

struct PreparedDataset {
    source_id: String,
    tier: DatasetTier,
    logical_records: u64,
    manifest_sha256: String,
    origin: String,
    formats: BTreeMap<&'static str, (PathBuf, ArtifactIdentity)>,
}

struct PreparedCampaign {
    temporary: Option<TempDir>,
    datasets: Vec<PreparedDataset>,
}

#[allow(
    clippy::too_many_lines,
    reason = "campaign orchestration is intentionally linear and delegates measurement details"
)]
fn run() -> Result<ExitCode, Box<dyn std::error::Error>> {
    if env::args().nth(1).as_deref() == Some("--internal-rss-control") {
        return Ok(ExitCode::SUCCESS);
    }
    if env::args().nth(1).as_deref() == Some("--internal-rss-probe") {
        tq_test_support::benchmark::run_allocation_probe(64 * 1024 * 1024, 1)?;
        return Ok(ExitCode::SUCCESS);
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let options = options()?;
    if options.preflight_only && !options.render_only.is_empty() {
        return Err("--preflight-only cannot be combined with --render-only".into());
    }
    if !options.render_only.is_empty() {
        let markdown_dir = options
            .markdown_dir
            .as_deref()
            .ok_or("--render-only requires --markdown-dir")?;
        let reports = options
            .render_only
            .iter()
            .map(|path| {
                serde_json::from_reader(fs::File::open(path)?)
                    .map_err(|error| -> Box<dyn std::error::Error> { Box::new(error) })
            })
            .collect::<Result<Vec<BenchmarkCampaignReport>, _>>()?;
        render_markdown_campaigns(markdown_dir, &reports)?;
        for (index, report) in reports.iter().enumerate() {
            if index > 0 {
                println!();
            }
            print!("{}", report.render_human());
        }
        return Ok(ExitCode::SUCCESS);
    }
    let cancellation = Arc::new(AtomicBool::new(false));
    // The CLI owns these process-lifetime handlers. Measurements only inspect
    // the shared flag; their single owner still performs termination and reap.
    for signal in [signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM] {
        signal_hook::flag::register(signal, Arc::clone(&cancellation))?;
    }
    let rss_preflight = preflight_rss(Some(Arc::clone(&cancellation)))?;
    eprintln!(
        "tq-bench: RSS preflight passed ({})",
        rss_preflight.provenance.label()
    );
    if options.preflight_only {
        return Ok(ExitCode::SUCCESS);
    }
    let calibrations = options
        .timing_calibrations
        .iter()
        .map(|path| calibration::TimingCalibration::load(path))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let tools = discover_tools(&root)?;
    require_campaign_tools(&tools)?;
    let mut prepared = if options.profile == "smoke" {
        prepare_smoke(&root.join("examples"))?
    } else {
        prepare_manifests(&options)?
    };
    if options
        .selected_cases
        .iter()
        .any(|id| id.starts_with("benchmark.native-"))
        || options.selected_cases.is_empty()
    {
        let directory = prepared
            .temporary
            .as_ref()
            .ok_or("missing fixture directory")?;
        prepared
            .datasets
            .extend(native_rows::prepare(directory.path())?);
    }
    let catalog = load_benchmark_catalog(&root.join("benchmarks/cases"))?;
    let planned_rows = plan_rows(&catalog.cases, &options.selected_cases, &prepared.datasets)?;
    let mut corpus = Vec::new();
    for dataset in &prepared.datasets {
        for (format, (_, artifact)) in &dataset.formats {
            corpus.push(corpus_identity(dataset, format, artifact));
        }
    }

    let mut rows = Vec::new();
    for original_case in &catalog.cases {
        if !options.selected_cases.is_empty() && !options.selected_cases.contains(&original_case.id)
        {
            continue;
        }
        let mut case = original_case.clone();
        if let Some(samples) = options.max_samples {
            case.sampling = BenchmarkSampling {
                warmups: usize::from(samples > 1),
                small: samples,
                medium: samples,
                large: samples,
            };
        }
        if let Some(timeout_seconds) = options.timeout_seconds {
            case.timeout_seconds = timeout_seconds;
        }
        if let Some(rss_limit_bytes) = options.rss_limit_bytes {
            case.limits.rss_bytes = Some(
                case.limits
                    .rss_bytes
                    .map_or(rss_limit_bytes, |limit| limit.min(rss_limit_bytes)),
            );
        }
        for dataset in prepared.datasets.iter().filter(|dataset| {
            case.dataset_selector.tiers.contains(&dataset.tier)
                && family_matches(case.dataset_selector.family, dataset)
        }) {
            let reference_adapter = case
                .adapters
                .iter()
                .find(|adapter| adapter.id == case.output_contract.reference_adapter)
                .ok_or_else(|| format!("{} has no reference adapter", case.id))?;
            let reference_identity = tools
                .get(&reference_adapter.tool)
                .ok_or("reference executable is unavailable")?;
            let reference_invocation = invocation(
                &case,
                reference_adapter,
                dataset,
                reference_identity,
                &cancellation,
            )?;
            let mut reference_witness_limit = None;
            let reference = match normalize_correctness_run(
                &reference_invocation,
                match reference_adapter.tool {
                    BenchmarkTool::Jq => ToolKind::Jq,
                    BenchmarkTool::Yq => ToolKind::Yq,
                    BenchmarkTool::Tq => ToolKind::Tq,
                },
                case.output_contract.kind,
            ) {
                Ok(reference) => Some(reference),
                Err(error) if is_correctness_output_limit(&error) => None,
                Err(BenchmarkRunnerError::CorrectnessWitnessLimit { limit }) => {
                    reference_witness_limit = Some(limit);
                    None
                }
                Err(error) => return Err(error.into()),
            };
            for adapter in &case.adapters {
                let corpus_identity = corpus_identity(
                    dataset,
                    format_name(adapter.input_format),
                    &dataset
                        .formats
                        .get(format_name(adapter.input_format))
                        .ok_or("missing prepared representation")?
                        .1,
                );
                let placeholder = BenchmarkInvocation {
                    cancellation: Some(Arc::clone(&cancellation)),
                    executable: PathBuf::from(tool_name(adapter.tool)),
                    args: Vec::new(),
                    stdin: Vec::new(),
                    current_dir: Some(root.clone()),
                    timeout: Duration::from_secs(case.timeout_seconds),
                    output_limit: case.limits.output_bytes,
                    rss_limit: case.limits.rss_bytes,
                    retain_output: false,
                };
                let Some(identity) = tools.get(&adapter.tool) else {
                    record_row(
                        &mut rows,
                        &case,
                        dataset,
                        adapter,
                        unsupported_row(
                            &case,
                            adapter,
                            &corpus_identity,
                            dataset.tier,
                            &placeholder,
                        ),
                    );
                    continue;
                };
                let invocation = invocation(&case, adapter, dataset, identity, &cancellation)?;
                if !adapter.applicable {
                    record_row(
                        &mut rows,
                        &case,
                        dataset,
                        adapter,
                        unsupported_row(
                            &case,
                            adapter,
                            &corpus_identity,
                            dataset.tier,
                            &invocation,
                        ),
                    );
                    continue;
                }
                let row = if let Some(reference) = &reference {
                    run_gated_row(
                        &case,
                        adapter,
                        &corpus_identity,
                        dataset.tier,
                        &invocation,
                        reference,
                    )?
                } else if let Some(limit) = reference_witness_limit {
                    correctness_witness_limit_row(
                        &case,
                        adapter,
                        &corpus_identity,
                        dataset.tier,
                        &invocation,
                        limit,
                    )
                } else {
                    run_correctness_limit_probe(
                        &case,
                        adapter,
                        &corpus_identity,
                        dataset.tier,
                        &invocation,
                    )?
                };
                record_row(&mut rows, &case, dataset, adapter, row);
            }
        }
    }
    validate_rows_against_plan(&planned_rows, &rows)?;
    for row in &mut rows {
        for sample in row
            .samples
            .iter_mut()
            .chain(row.instrumented_samples.iter_mut())
        {
            if !calibrations.is_empty() {
                let protocol = sample
                    .measurement_protocol
                    .as_mut()
                    .ok_or("measured sample has no protocol for timing calibration")?;
                calibrations
                    .iter()
                    .find(|calibration| calibration.matches(protocol))
                    .ok_or("no matching timing calibration for sample instrumentation")?
                    .apply(protocol)?;
            }
        }
    }
    populate_reference_ratios(
        &mut rows,
        &[
            "jq-json",
            "yq-json",
            "yq-yaml",
            "jq-json-seq",
            "yq-csv",
            "yq-tsv",
        ],
    );
    let has_failure = rows.iter().any(|row| {
        !matches!(
            row.outcome,
            BenchmarkOutcome::Timed | BenchmarkOutcome::Unsupported
        )
    });
    let mut report = BenchmarkCampaignReport {
        schema_version: 1,
        campaign_id: jiff::Timestamp::now().to_string(),
        profile: options.profile,
        environment: collect_environment(if cfg!(debug_assertions) {
            "debug-benchmark"
        } else {
            "release-benchmark"
        }),
        corpus,
        tools: tools.into_values().collect(),
        cases: rows,
        comparability: Comparability::default(),
        regression_gate: RegressionGate::default(),
        final_status: if has_failure {
            BenchmarkFinalStatus::ObservedFailures
        } else {
            BenchmarkFinalStatus::Passed
        },
    };
    if let Some(path) = &options.baseline {
        let baseline: BenchmarkCampaignReport = serde_json::from_reader(fs::File::open(path)?)?;
        report.comparability = compare_reports(&baseline, &report);
        report.regression_gate =
            evaluate_regression(&baseline, &report, options.regression_thresholds.clone());
        if regression_gate_failed(&report.regression_gate) {
            report.final_status = BenchmarkFinalStatus::Regression;
        }
    }
    report
        .validate_authoritative_rss()
        .map_err(|error| format!("benchmark report RSS validation failed: {error}"))?;
    // A report without linked native controls is useful diagnostic JSON, but
    // it must not reach the stable Results renderer. Validate before writing
    // either artifact so a failed publication cannot leave a misleading report
    // beside the campaign output.
    if options.markdown_dir.is_some() {
        report
            .validate_for_publication()
            .map_err(|error| format!("benchmark report publication validation failed: {error}"))?;
    }
    if cancellation.load(std::sync::atomic::Ordering::Acquire) {
        return Err("benchmark campaign cancelled; report not published".into());
    }
    write_report(&options.output, &report)?;
    if let Some(markdown_dir) = &options.markdown_dir {
        render_markdown_pages(markdown_dir, &report)?;
    }
    print!("{}", report.render_human());
    Ok(exit_code_for_status(report.final_status))
}

fn exit_code_for_status(status: BenchmarkFinalStatus) -> ExitCode {
    match status {
        BenchmarkFinalStatus::Passed => ExitCode::SUCCESS,
        BenchmarkFinalStatus::ObservedFailures | BenchmarkFinalStatus::Regression => {
            ExitCode::from(1)
        }
    }
}

fn regression_gate_failed(gate: &RegressionGate) -> bool {
    // Structural failures (for example, a missing baseline row) are fatal even
    // when no comparable timing rows remain and `evaluated` is therefore false.
    !gate.failures.is_empty()
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PlannedRowIdentity {
    case_id: String,
    source_id: String,
    tier: String,
    adapter_id: String,
}

fn plan_rows(
    cases: &[BenchmarkCase],
    selected_cases: &[String],
    datasets: &[PreparedDataset],
) -> Result<std::collections::BTreeSet<PlannedRowIdentity>, String> {
    for selected in selected_cases {
        if !cases.iter().any(|case| case.id == *selected) {
            return Err(format!(
                "selected benchmark case {selected} is not present in the catalog"
            ));
        }
    }

    let mut planned = std::collections::BTreeSet::new();
    for case in cases {
        if !selected_cases.is_empty() && !selected_cases.contains(&case.id) {
            continue;
        }
        let matching_datasets = datasets.iter().filter(|dataset| {
            case.dataset_selector.tiers.contains(&dataset.tier)
                && family_matches(case.dataset_selector.family, dataset)
        });
        let mut matched = false;
        for dataset in matching_datasets {
            matched = true;
            for adapter in &case.adapters {
                let identity = PlannedRowIdentity {
                    case_id: case.id.clone(),
                    source_id: dataset.source_id.clone(),
                    tier: tier_name(dataset.tier).to_owned(),
                    adapter_id: adapter.id.clone(),
                };
                if !planned.insert(identity.clone()) {
                    return Err(format!(
                        "duplicate planned benchmark row identity: {}",
                        format_row_identity(&identity)
                    ));
                }
            }
        }
        if !matched && !selected_cases.is_empty() {
            return Err(format!(
                "selected benchmark case {} has no prepared datasets for this profile",
                case.id
            ));
        }
        if matched && case.adapters.is_empty() {
            return Err(format!(
                "selected benchmark case {} has no adapters in the catalog",
                case.id
            ));
        }
    }
    if planned.is_empty() {
        return Err("benchmark campaign produced no planned rows".to_owned());
    }
    Ok(planned)
}

fn validate_rows_against_plan(
    planned: &std::collections::BTreeSet<PlannedRowIdentity>,
    rows: &[BenchmarkRow],
) -> Result<(), String> {
    let mut actual = std::collections::BTreeSet::new();
    for row in rows {
        let identity = PlannedRowIdentity {
            case_id: row.case_id.clone(),
            source_id: row.source_id.clone(),
            tier: row.tier.clone(),
            adapter_id: row.adapter_id.clone(),
        };
        if !actual.insert(identity.clone()) {
            return Err(format!(
                "duplicate benchmark row identity: {}",
                format_row_identity(&identity)
            ));
        }
    }
    let missing = planned
        .difference(&actual)
        .map(format_row_identity)
        .collect::<Vec<_>>();
    let unexpected = actual
        .difference(planned)
        .map(format_row_identity)
        .collect::<Vec<_>>();
    if !missing.is_empty() || !unexpected.is_empty() {
        let mut details = Vec::new();
        if !missing.is_empty() {
            details.push(format!("missing [{}]", missing.join(", ")));
        }
        if !unexpected.is_empty() {
            details.push(format!("unexpected [{}]", unexpected.join(", ")));
        }
        return Err(format!(
            "benchmark matrix did not complete its planned rows: {}",
            details.join("; ")
        ));
    }
    Ok(())
}

fn format_row_identity(identity: &PlannedRowIdentity) -> String {
    format!(
        "{}/{}/{}/{}",
        identity.case_id, identity.source_id, identity.tier, identity.adapter_id
    )
}

fn record_row(
    rows: &mut Vec<BenchmarkRow>,
    case: &BenchmarkCase,
    dataset: &PreparedDataset,
    adapter: &BenchmarkAdapter,
    row: BenchmarkRow,
) {
    eprintln!(
        "tq-bench: workload={} dataset={} adapter={} outcome={:?}",
        case.id, dataset.source_id, adapter.id, row.outcome
    );
    rows.push(row);
}

#[allow(
    clippy::too_many_lines,
    reason = "the command-line grammar stays intentionally explicit and dependency-free"
)]
fn options() -> Result<Options, Box<dyn std::error::Error>> {
    let mut profile = "rapid".to_owned();
    let archive_root = env::var_os("TQ_BENCHMARK_ARCHIVE_ROOT")
        .map_or_else(|| PathBuf::from("benchmarks"), PathBuf::from);
    let mut output = archive_root.join(".work/rapid.json");
    let mut manifests = Vec::new();
    let mut cache_root = archive_root.join(".work/corpus");
    let mut origin = "frozen".to_owned();
    let mut max_samples = None;
    let mut timeout_seconds = None;
    let mut rss_limit_bytes = None;
    let mut selected_cases = Vec::new();
    let mut baseline = None;
    let mut markdown_dir = None;
    let mut render_only = Vec::new();
    let mut preflight_only = false;
    let mut timing_calibrations = Vec::new();
    let mut wall_time_percent: f64 = 50.0;
    let mut peak_rss_percent: f64 = 50.0;
    let mut minimum_samples = 5;
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "run" => {}
            "--profile" => profile = arguments.next().ok_or("--profile needs a value")?,
            "--output" => output = PathBuf::from(arguments.next().ok_or("--output needs a path")?),
            "--manifest" => manifests.push(PathBuf::from(
                arguments.next().ok_or("--manifest needs a path")?,
            )),
            "--cache-root" => {
                cache_root = PathBuf::from(arguments.next().ok_or("--cache-root needs a path")?);
            }
            "--origin" => origin = arguments.next().ok_or("--origin needs a value")?,
            "--max-samples" => {
                max_samples = Some(
                    arguments
                        .next()
                        .ok_or("--max-samples needs a value")?
                        .parse()?,
                );
            }
            "--timeout-seconds" => {
                timeout_seconds = Some(
                    arguments
                        .next()
                        .ok_or("--timeout-seconds needs a value")?
                        .parse()?,
                );
            }
            "--rss-limit-bytes" => {
                rss_limit_bytes = Some(
                    arguments
                        .next()
                        .ok_or("--rss-limit-bytes needs a value")?
                        .parse()?,
                );
            }
            "--case" => selected_cases.push(arguments.next().ok_or("--case needs an ID")?),
            "--baseline" => {
                baseline = Some(PathBuf::from(
                    arguments.next().ok_or("--baseline needs a path")?,
                ));
            }
            "--timing-calibration" => {
                timing_calibrations.push(PathBuf::from(
                    arguments
                        .next()
                        .ok_or("--timing-calibration needs a summary path")?,
                ));
            }
            "--markdown-dir" => {
                if markdown_dir.is_some() {
                    return Err("--markdown-dir may be supplied only once".into());
                }
                markdown_dir = Some(PathBuf::from(
                    arguments.next().ok_or("--markdown-dir needs a directory")?,
                ));
            }
            "--render-only" => {
                render_only.push(PathBuf::from(
                    arguments
                        .next()
                        .ok_or("--render-only needs a report path")?,
                ));
            }
            "--preflight-only" => preflight_only = true,
            "--wall-regression-percent" => {
                wall_time_percent = arguments
                    .next()
                    .ok_or("--wall-regression-percent needs a value")?
                    .parse()?;
            }
            "--rss-regression-percent" => {
                peak_rss_percent = arguments
                    .next()
                    .ok_or("--rss-regression-percent needs a value")?
                    .parse()?;
            }
            "--minimum-regression-samples" => {
                minimum_samples = arguments
                    .next()
                    .ok_or("--minimum-regression-samples needs a value")?
                    .parse()?;
            }
            "-h" | "--help" => {
                println!(
                    "Usage: tq-bench [--preflight-only] run --profile smoke|rapid|standard|large --output PATH [--manifest PATH --cache-root PATH --origin refreshed|frozen] [--max-samples N] [--timeout-seconds N] [--rss-limit-bytes N] [--case ID] [--timing-calibration SUMMARY ...] [--baseline PATH --wall-regression-percent N --rss-regression-percent N --minimum-regression-samples N] [--markdown-dir DIRECTORY] [--render-only REPORT... --markdown-dir DIRECTORY] [native child accounting on macOS/Linux; platform time commands are independent validation only; Windows deferred]"
                );
                std::process::exit(0);
            }
            value => return Err(format!("unknown argument: {value}").into()),
        }
    }
    if !matches!(profile.as_str(), "smoke" | "rapid" | "standard" | "large") {
        return Err(format!("invalid profile: {profile}").into());
    }
    if render_only.is_empty() && !preflight_only {
        if profile == "rapid" {
            if max_samples.is_none() {
                max_samples = Some(1);
            }
            if selected_cases.is_empty() {
                selected_cases.extend(RAPID_CASES.iter().map(|case| (*case).to_owned()));
            }
        }
        if profile != "smoke" && manifests.is_empty() {
            if let Some(paths) = env::var_os("TQ_BENCH_MANIFESTS") {
                manifests.extend(env::split_paths(&paths));
            } else {
                manifests = discover_latest_validated_manifests(&cache_root)?;
            }
        }
        if profile != "smoke" && manifests.is_empty() {
            return Err("no admitted machine-local corpus snapshots were found; run tq-corpus prepare or pass --manifest".into());
        }
    }
    if !matches!(origin.as_str(), "refreshed" | "frozen") {
        return Err(format!("invalid origin: {origin}").into());
    }
    if !wall_time_percent.is_finite() || wall_time_percent < 0.0 {
        return Err("--wall-regression-percent must be a finite non-negative number".into());
    }
    if !peak_rss_percent.is_finite() || peak_rss_percent < 0.0 {
        return Err("--rss-regression-percent must be a finite non-negative number".into());
    }
    if minimum_samples == 0 {
        return Err("--minimum-regression-samples must be at least 1".into());
    }
    if timeout_seconds == Some(0) {
        return Err("--timeout-seconds must be at least 1".into());
    }
    if rss_limit_bytes == Some(0) {
        return Err("--rss-limit-bytes must be at least 1".into());
    }
    if !render_only.is_empty() && markdown_dir.is_none() {
        return Err("--render-only requires --markdown-dir".into());
    }
    Ok(Options {
        profile,
        output,
        manifests,
        cache_root,
        origin,
        max_samples,
        timeout_seconds,
        rss_limit_bytes,
        selected_cases,
        baseline,
        markdown_dir,
        render_only,
        preflight_only,
        timing_calibrations,
        regression_thresholds: RegressionThresholds {
            wall_time_percent,
            peak_rss_percent,
            minimum_samples,
        },
    })
}

fn prepare_smoke(examples: &Path) -> Result<PreparedCampaign, Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let generated_examples = temporary.path().join("smoke");
    let smoke_dir = if examples.is_dir() {
        examples
    } else {
        fs::create_dir_all(&generated_examples)?;
        fs::write(
            generated_examples.join("generated.geojson"),
            br#"{"type":"FeatureCollection","features":[{"id":"smoke","properties":{"mag":1,"place":"generated"}}]}"#,
        )?;
        &generated_examples
    };
    let smoke = discover_smoke_corpus(smoke_dir)?;
    let mut datasets = Vec::new();
    for snapshot in smoke.snapshots {
        datasets.push(prepare_smoke_snapshot(&snapshot, temporary.path())?);
    }
    let startup_path = temporary.path().join("startup.json");
    fs::write(&startup_path, b"null")?;
    let startup = SmokeSnapshot {
        source_id: "startup".to_owned(),
        file: startup_path,
        artifact: ArtifactIdentity {
            path: "startup.json".to_owned(),
            bytes: 4,
            sha256: hex(&Sha256::digest(b"null")),
        },
        document: tq_test_support::corpus::DocumentIdentity {
            root_type: "null".to_owned(),
            logical_records: 1,
        },
    };
    let mut prepared = prepare_smoke_snapshot(&startup, temporary.path())?;
    prepared.tier = DatasetTier::Startup;
    datasets.push(prepared);
    datasets.push(prepare_issue5_input_sequence(temporary.path())?);

    let deep_merge_bytes = deep_merge_fixture(DEEP_MERGE_ENTRIES)?;
    let deep_merge_path = temporary.path().join("deep-merge.json");
    fs::write(&deep_merge_path, &deep_merge_bytes)?;
    let deep_merge = SmokeSnapshot {
        source_id: "deep-merge".to_owned(),
        file: deep_merge_path,
        artifact: ArtifactIdentity {
            path: "deep-merge.json".to_owned(),
            bytes: u64::try_from(deep_merge_bytes.len())?,
            sha256: hex(&Sha256::digest(&deep_merge_bytes)),
        },
        document: tq_test_support::corpus::DocumentIdentity {
            root_type: "array".to_owned(),
            logical_records: 2,
        },
    };
    let mut prepared = prepare_smoke_snapshot(&deep_merge, temporary.path())?;
    prepared.tier = DatasetTier::Startup;
    datasets.push(prepared);
    Ok(PreparedCampaign {
        temporary: Some(temporary),
        datasets,
    })
}

fn deep_merge_fixture(entries: usize) -> Result<Vec<u8>, serde_json::Error> {
    let mut left = serde_json::Map::new();
    let mut right = serde_json::Map::new();
    for index in 0..entries {
        let key = format!("key_{index:05}");
        left.insert(
            key.clone(),
            serde_json::json!({
                "shared": {"left": index, "replace": "left"},
                "left_only": index
            }),
        );
        right.insert(
            key,
            serde_json::json!({
                "shared": {"right": index, "replace": "right"},
                "right_only": index
            }),
        );
    }
    serde_json::to_vec(&[
        serde_json::Value::Object(left),
        serde_json::Value::Object(right),
    ])
}

fn prepare_smoke_snapshot(
    snapshot: &SmokeSnapshot,
    output: &Path,
) -> Result<PreparedDataset, Box<dyn std::error::Error>> {
    let yaml = output.join(format!("{}.yaml", snapshot.source_id));
    let toon = output.join(format!("{}.toon", snapshot.source_id));
    let generated = generate_representations(
        &snapshot.file,
        &yaml,
        &toon,
        &format!("{}.yaml", snapshot.source_id),
        &format!("{}.toon", snapshot.source_id),
    )?;
    let mut formats = BTreeMap::new();
    // The command records the concrete temporary/source path it launches. Keep
    // the artifact identity aligned with that path even for synthetic smoke
    // fixtures whose seed metadata uses a cache-relative name.
    formats.insert(
        "json",
        (
            snapshot.file.clone(),
            absolute_identity(&snapshot.file, snapshot.artifact.clone()),
        ),
    );
    formats.insert(
        "yaml",
        (yaml.clone(), absolute_identity(&yaml, generated.yaml)),
    );
    formats.insert(
        "toon",
        (toon.clone(), absolute_identity(&toon, generated.toon)),
    );
    Ok(PreparedDataset {
        source_id: snapshot.source_id.clone(),
        tier: if snapshot.source_id == "all-hour" {
            DatasetTier::Small
        } else {
            DatasetTier::Medium
        },
        logical_records: snapshot.document.logical_records,
        manifest_sha256: snapshot.artifact.sha256.clone(),
        origin: "smoke".to_owned(),
        formats,
    })
}

fn prepare_manifests(options: &Options) -> Result<PreparedCampaign, Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let mut datasets = Vec::new();
    for path in &options.manifests {
        let manifest: SnapshotManifest = serde_json::from_reader(fs::File::open(path)?)?;
        let tier = source_tier(&manifest.source_id);
        if (options.profile == "rapid" && manifest.source_id != RAPID_SOURCE)
            || ((options.profile == "standard" || options.profile == "rapid")
                && tier == DatasetTier::Large)
            || (options.profile == "large" && tier != DatasetTier::Large)
        {
            continue;
        }
        let frozen = load_frozen_snapshot(path, &options.cache_root)?;
        let generated = frozen
            .manifest
            .artifacts
            .generated
            .as_ref()
            .ok_or("snapshot has no generated representations")?;
        let mut formats = BTreeMap::new();
        formats.insert(
            "json",
            (
                options
                    .cache_root
                    .join(&frozen.manifest.artifacts.source_json.path),
                frozen.manifest.artifacts.source_json.clone(),
            ),
        );
        formats.insert(
            "yaml",
            (
                options.cache_root.join(&generated.yaml.path),
                generated.yaml.clone(),
            ),
        );
        formats.insert(
            "toon",
            (
                options.cache_root.join(&generated.toon.path),
                generated.toon.clone(),
            ),
        );
        datasets.push(PreparedDataset {
            source_id: frozen.manifest.source_id.clone(),
            tier,
            logical_records: frozen.manifest.document.logical_records,
            manifest_sha256: hex(&Sha256::digest(&fs::read(path)?)),
            origin: options.origin.clone(),
            formats,
        });
    }
    if options.profile == "rapid" && datasets.is_empty() {
        return Err(format!("rapid profile requires a {RAPID_SOURCE} snapshot").into());
    }
    datasets.push(prepare_issue5_input_sequence(temporary.path())?);
    Ok(PreparedCampaign {
        temporary: Some(temporary),
        datasets,
    })
}

fn prepare_issue5_input_sequence(
    output: &Path,
) -> Result<PreparedDataset, Box<dyn std::error::Error>> {
    const RECORDS: u64 = 65_536;
    let mut json = Vec::new();
    let mut yaml = Vec::new();
    let mut toon = Vec::new();
    for id in 0..RECORDS {
        let value = id % 997;
        writeln!(
            json,
            "{{\"id\":{id},\"value\":{value},\"label\":\"issue5-input-{id}\"}}"
        )?;
        writeln!(
            yaml,
            "---\nid: {id}\nvalue: {value}\nlabel: issue5-input-{id}"
        )?;
        writeln!(
            toon,
            "\x1eid: {id}\nvalue: {value}\nlabel: issue5-input-{id}"
        )?;
    }
    let paths = [
        ("json", output.join("issue5-input-sequence.json"), json),
        ("yaml", output.join("issue5-input-sequence.yaml"), yaml),
        ("toon", output.join("issue5-input-sequence.toonseq"), toon),
    ];
    let mut formats = BTreeMap::new();
    for (format, path, bytes) in paths {
        fs::write(&path, &bytes)?;
        formats.insert(
            format,
            (
                path.clone(),
                ArtifactIdentity {
                    path: path.display().to_string(),
                    bytes: bytes.len() as u64,
                    sha256: hex(&Sha256::digest(&bytes)),
                },
            ),
        );
    }
    let manifest_sha256 = formats
        .get("json")
        .ok_or("missing generated JSON input sequence")?
        .1
        .sha256
        .clone();
    Ok(PreparedDataset {
        source_id: "issue5-input-sequence".to_owned(),
        tier: DatasetTier::Medium,
        logical_records: RECORDS,
        manifest_sha256,
        origin: "synthetic-reviewed".to_owned(),
        formats,
    })
}

fn discover_tools(
    root: &Path,
) -> Result<BTreeMap<BenchmarkTool, ToolIdentity>, Box<dyn std::error::Error>> {
    let config = ExecutableConfig::from_env();
    let mut tools = BTreeMap::new();
    for (benchmark, kind) in [
        (BenchmarkTool::Jq, ToolKind::Jq),
        (BenchmarkTool::Yq, ToolKind::Yq),
        (BenchmarkTool::Tq, ToolKind::Tq),
    ] {
        if let Some(identity) = discover_tool(kind, &config, root)? {
            tools.insert(benchmark, identity);
        }
    }
    Ok(tools)
}

fn require_campaign_tools(tools: &BTreeMap<BenchmarkTool, ToolIdentity>) -> Result<(), String> {
    let missing = [BenchmarkTool::Jq, BenchmarkTool::Yq, BenchmarkTool::Tq]
        .into_iter()
        .filter(|tool| !tools.contains_key(tool))
        .map(tool_name)
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "required campaign executables unavailable: {}; repair discovery before running the matrix",
            missing.join(", ")
        ))
    }
}

fn invocation(
    case: &tq_test_support::benchmark::BenchmarkCase,
    adapter: &tq_test_support::benchmark::BenchmarkAdapter,
    dataset: &PreparedDataset,
    identity: &ToolIdentity,
    cancellation: &Arc<AtomicBool>,
) -> Result<BenchmarkInvocation, Box<dyn std::error::Error>> {
    let path = &dataset
        .formats
        .get(format_name(adapter.input_format))
        .ok_or("missing format")?
        .0;
    let mut args = adapter.args.clone();
    if adapter.tool == BenchmarkTool::Tq {
        args.push("--max-input-bytes".to_owned());
        args.push(fs::metadata(path)?.len().to_string());
    }
    args.push(adapter.query.clone().unwrap_or_else(|| case.query.clone()));
    args.push(path.display().to_string());
    Ok(BenchmarkInvocation {
        cancellation: Some(Arc::clone(cancellation)),
        executable: identity.path.clone(),
        args,
        stdin: Vec::new(),
        current_dir: None,
        timeout: Duration::from_secs(case.timeout_seconds),
        output_limit: case.limits.output_bytes,
        rss_limit: case.limits.rss_bytes,
        retain_output: false,
    })
}

fn corpus_identity(
    dataset: &PreparedDataset,
    format: &str,
    artifact: &ArtifactIdentity,
) -> BenchmarkCorpusIdentity {
    BenchmarkCorpusIdentity {
        origin: dataset.origin.clone(),
        source_id: dataset.source_id.clone(),
        tier: tier_name(dataset.tier).to_owned(),
        format: match format {
            "json" => InputFormat::Json,
            "yaml" => InputFormat::Yaml,
            "toon" => InputFormat::Toon,
            "json-seq" => InputFormat::JsonSequence,
            "csv" => InputFormat::Csv,
            "tsv" => InputFormat::Tsv,
            _ => unreachable!("prepared format is registered"),
        },
        artifact: artifact.clone(),
        logical_records: dataset.logical_records,
        manifest_sha256: dataset.manifest_sha256.clone(),
    }
}

fn family_matches(
    family: tq_test_support::benchmark::DatasetFamily,
    dataset: &PreparedDataset,
) -> bool {
    if dataset.source_id.starts_with("native-rows-") {
        return family == tq_test_support::benchmark::DatasetFamily::NativeRowSequence;
    }
    match family {
        tq_test_support::benchmark::DatasetFamily::NativeRowSequence => false,
        tq_test_support::benchmark::DatasetFamily::Natural => {
            dataset.tier != DatasetTier::Startup && dataset.source_id != "issue5-input-sequence"
        }
        tq_test_support::benchmark::DatasetFamily::SyntheticHelper => {
            dataset.source_id == "startup"
        }
        tq_test_support::benchmark::DatasetFamily::DeepMergeHelper => {
            dataset.source_id == "deep-merge"
        }
        tq_test_support::benchmark::DatasetFamily::Issue5InputSequence => {
            dataset.source_id == "issue5-input-sequence"
        }
        tq_test_support::benchmark::DatasetFamily::Usgs => {
            dataset.source_id != "issue5-input-sequence"
                && matches!(dataset.tier, DatasetTier::Small | DatasetTier::Medium)
        }
        tq_test_support::benchmark::DatasetFamily::LargeNatural => {
            dataset.tier == DatasetTier::Large
        }
    }
}

fn source_tier(source: &str) -> DatasetTier {
    if source.contains("microsoft") {
        DatasetTier::Large
    } else if source.ends_with("all-hour") || source == "usgs-all-hour" {
        DatasetTier::Small
    } else {
        DatasetTier::Medium
    }
}
fn tier_name(tier: DatasetTier) -> &'static str {
    match tier {
        DatasetTier::Small => "small",
        DatasetTier::Medium => "medium",
        DatasetTier::Large => "large",
        DatasetTier::Startup => "startup",
    }
}
fn format_name(format: InputFormat) -> &'static str {
    match format {
        InputFormat::Json => "json",
        InputFormat::Yaml => "yaml",
        InputFormat::Toon => "toon",
        InputFormat::JsonSequence => "json-seq",
        InputFormat::Csv => "csv",
        InputFormat::Tsv => "tsv",
    }
}
fn tool_name(tool: BenchmarkTool) -> &'static str {
    match tool {
        BenchmarkTool::Jq => "jq",
        BenchmarkTool::Yq => "yq",
        BenchmarkTool::Tq => "tq",
    }
}
fn absolute_identity(path: &Path, mut artifact: ArtifactIdentity) -> ArtifactIdentity {
    artifact.path = path.display().to_string();
    artifact
}
fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, byte| {
            use std::fmt::Write as _;
            write!(out, "{byte:02x}").expect("hex");
            out
        })
}

fn write_report(
    path: &Path,
    report: &BenchmarkCampaignReport,
) -> Result<(), Box<dyn std::error::Error>> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer_pretty(&mut temporary, report)?;
    temporary.write_all(b"\n")?;
    temporary.persist(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        BenchmarkFinalStatus, DatasetTier, PlannedRowIdentity, PreparedDataset, SmokeSnapshot,
        exit_code_for_status, family_matches, plan_rows, prepare_smoke_snapshot,
        regression_gate_failed, validate_rows_against_plan,
    };
    use std::collections::BTreeMap;
    use std::fs;
    use tempfile::tempdir;
    use tq_test_support::benchmark::{
        BenchmarkAdapter, BenchmarkCase, BenchmarkLimits, BenchmarkSampling, BenchmarkTool,
        DatasetFamily, DatasetSelector, ExecutionClass, InputFormat, OutputContract,
        OutputContractKind, RegressionGate,
    };
    use tq_test_support::corpus::DocumentIdentity;

    #[test]
    fn missing_executables_are_not_unsupported_capability_rows() {
        let error = super::require_campaign_tools(&BTreeMap::new()).unwrap_err();
        assert!(error.contains("jq, yq, tq"));
        assert!(error.contains("repair discovery"));
    }

    #[test]
    fn issue5_input_sequence_has_one_disjoint_dataset_family() {
        let dataset = PreparedDataset {
            source_id: "issue5-input-sequence".to_owned(),
            tier: DatasetTier::Medium,
            logical_records: 65_536,
            manifest_sha256: String::new(),
            origin: "synthetic-reviewed".to_owned(),
            formats: BTreeMap::new(),
        };
        assert!(family_matches(DatasetFamily::Issue5InputSequence, &dataset));
        assert!(!family_matches(DatasetFamily::Natural, &dataset));
        assert!(!family_matches(DatasetFamily::Usgs, &dataset));
        assert!(!family_matches(DatasetFamily::LargeNatural, &dataset));
        assert!(!family_matches(DatasetFamily::SyntheticHelper, &dataset));
    }

    #[test]
    fn failed_final_statuses_return_a_failure_exit_code() {
        for status in [
            BenchmarkFinalStatus::ObservedFailures,
            BenchmarkFinalStatus::Regression,
        ] {
            assert_eq!(
                exit_code_for_status(status),
                std::process::ExitCode::from(1)
            );
        }
    }

    fn selected_case(family: DatasetFamily, tiers: Vec<DatasetTier>) -> BenchmarkCase {
        BenchmarkCase {
            schema_version: 1,
            id: "benchmark.test".to_owned(),
            compatibility_gate: "test".to_owned(),
            dataset_selector: DatasetSelector { family, tiers },
            query: ".".to_owned(),
            execution_class: ExecutionClass::Startup,
            measure_first_result: false,
            sampling: BenchmarkSampling {
                warmups: 0,
                small: 1,
                medium: 1,
                large: 1,
            },
            timeout_seconds: 1,
            limits: BenchmarkLimits {
                output_bytes: 1,
                rss_bytes: None,
            },
            output_contract: OutputContract {
                kind: OutputContractKind::ExitOnly,
                reference_adapter: "tq-json".to_owned(),
            },
            adapters: vec![BenchmarkAdapter {
                id: "tq-json".to_owned(),
                tool: BenchmarkTool::Tq,
                input_format: InputFormat::Json,
                applicable: true,
                unsupported_reason: None,
                args: Vec::new(),
                query: None,
                comparison_families: Vec::new(),
            }],
        }
    }

    fn prepared_dataset(source_id: &str, tier: DatasetTier) -> PreparedDataset {
        PreparedDataset {
            source_id: source_id.to_owned(),
            tier,
            logical_records: 1,
            manifest_sha256: String::new(),
            origin: "test".to_owned(),
            formats: BTreeMap::new(),
        }
    }

    #[test]
    fn selected_unknown_case_fails_closed() {
        let error = plan_rows(
            &[selected_case(
                DatasetFamily::Natural,
                vec![DatasetTier::Small],
            )],
            &["benchmark.missing".to_owned()],
            &[prepared_dataset("small", DatasetTier::Small)],
        )
        .expect_err("unknown selected cases must not produce an empty report");
        assert!(error.contains("benchmark.missing"));
        assert!(error.contains("not present in the catalog"));
    }

    #[test]
    fn selected_case_without_profile_dataset_fails_closed() {
        let error = plan_rows(
            &[selected_case(
                DatasetFamily::LargeNatural,
                vec![DatasetTier::Large],
            )],
            &["benchmark.test".to_owned()],
            &[prepared_dataset("small", DatasetTier::Small)],
        )
        .expect_err("a selected case with no applicable dataset must fail");
        assert!(error.contains("benchmark.test"));
        assert!(error.contains("no prepared datasets"));
    }

    #[test]
    fn actual_rows_must_match_the_planned_matrix() {
        let planned = std::collections::BTreeSet::from([PlannedRowIdentity {
            case_id: "benchmark.test".to_owned(),
            source_id: "small".to_owned(),
            tier: "small".to_owned(),
            adapter_id: "tq-json".to_owned(),
        }]);
        let error = validate_rows_against_plan(&planned, &[])
            .expect_err("missing rows must not publish a passing report");
        assert!(error.contains("missing"));
        assert!(error.contains("benchmark.test/small/small/tq-json"));
    }

    #[test]
    fn structural_regression_failure_blocks_even_when_not_evaluated() {
        let gate = RegressionGate {
            evaluated: false,
            failures: vec!["missing baseline row".to_owned()],
            ..RegressionGate::default()
        };
        assert!(regression_gate_failed(&gate));
    }

    #[test]
    fn smoke_json_identity_records_the_launch_path() {
        let directory = tempdir().expect("temporary smoke directory");
        let json_path = directory.path().join("startup.json");
        let json = br#"{"type":"FeatureCollection","features":[]}"#;
        fs::write(&json_path, json).expect("write smoke fixture");
        let snapshot = SmokeSnapshot {
            source_id: "startup".to_owned(),
            file: json_path.clone(),
            artifact: tq_test_support::corpus::ArtifactIdentity {
                path: "startup.json".to_owned(),
                bytes: json.len() as u64,
                sha256: String::new(),
            },
            document: DocumentIdentity {
                root_type: "FeatureCollection".to_owned(),
                logical_records: 0,
            },
        };

        let prepared = prepare_smoke_snapshot(&snapshot, directory.path())
            .expect("prepare smoke representations");
        assert_eq!(
            prepared.formats["json"].1.path,
            json_path.display().to_string()
        );
    }
}
