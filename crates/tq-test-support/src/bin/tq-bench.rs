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
    time::{Duration, Instant},
};

use sha2::{Digest, Sha256};
use tempfile::TempDir;
use tq_test_support::{
    benchmark::{
        BenchmarkCampaignReport, BenchmarkCase, BenchmarkCorpusIdentity, BenchmarkFinalStatus,
        BenchmarkInvocation, BenchmarkRow, BenchmarkTool, DatasetTier, InputFormat, RegressionGate,
        RegressionThresholds, compare_reports, evaluate_regression, preflight_rss,
        render_markdown_campaigns, render_markdown_pages,
    },
    compatibility::{ExecutableConfig, ToolIdentity, ToolKind, discover_tool},
    corpus::{
        ArtifactIdentity, SmokeSnapshot, SnapshotManifest, discover_latest_validated_manifests,
        discover_smoke_corpus, generate_representations, load_frozen_snapshot,
    },
};

const DEEP_MERGE_ENTRIES: usize = 10_000;

#[path = "tq-bench/native_rows.rs"]
mod native_rows;

#[path = "tq-bench/campaign.rs"]
mod campaign;
#[path = "tq-bench/deadline.rs"]
mod deadline;
#[path = "tq-bench/large_input.rs"]
mod large_input;
#[path = "tq-bench/policy.rs"]
mod policy;

use policy::{Mode, Sampling};

fn main() -> ExitCode {
    if matches!(
        env::args().nth(1).as_deref(),
        Some("--internal-rss-control" | "--internal-rss-probe")
    ) {
        return finish_run(run_internal());
    }
    let options = match options() {
        Ok(options) => options,
        Err(error) => {
            eprintln!("tq-bench: {error}");
            return ExitCode::from(2);
        }
    };
    match tq_test_support::benchmark::quick::supervise_current_if_quick(
        options.is_quick(),
        Duration::from_secs(options.campaign_budget_seconds),
        (!options.preflight_only && options.render_only.is_empty())
            .then_some(options.output.as_path()),
    ) {
        Ok(Some(code)) => return ExitCode::from(u8::try_from(code).unwrap_or(2)),
        Err(error) => {
            eprintln!("tq-bench: quick supervisor: {error}");
            return ExitCode::from(2);
        }
        Ok(None) => {}
    }
    finish_run(run(options))
}

fn finish_run(result: Result<ExitCode, Box<dyn std::error::Error>>) -> ExitCode {
    match result {
        Ok(status) => status,
        Err(error) => {
            eprintln!("tq-bench: {error}");
            ExitCode::from(2)
        }
    }
}

fn run_internal() -> Result<ExitCode, Box<dyn std::error::Error>> {
    if env::args().nth(1).as_deref() == Some("--internal-rss-probe") {
        tq_test_support::benchmark::run_allocation_probe(64 * 1024 * 1024, 1)?;
    }
    Ok(ExitCode::SUCCESS)
}

struct Options {
    suite: String,
    profile: String,
    input: Option<PathBuf>,
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
    mode: Mode,
    sampling: Sampling,
    selected_adapters: Vec<String>,
    instrument_rss: bool,
    campaign_budget_seconds: u64,
    case_budget_seconds: u64,
}

impl Options {
    fn is_quick(&self) -> bool {
        self.sampling == Sampling::Quick
    }
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
fn run(mut options: Options) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    if options.render_only.is_empty()
        && !options.preflight_only
        && options.suite != "smoke"
        && options.input.is_none()
        && options.manifests.is_empty()
    {
        if let Some(paths) = env::var_os("TQ_BENCH_MANIFESTS") {
            options.manifests.extend(env::split_paths(&paths));
        } else {
            options.manifests = discover_latest_validated_manifests(&options.cache_root)?;
        }
        if options.manifests.is_empty() {
            return Err("no admitted machine-local corpus snapshots were found; run tq-corpus prepare or pass --manifest".into());
        }
    }
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
            print_report(report)?;
        }
        return Ok(ExitCode::SUCCESS);
    }
    let cancellation = Arc::new(AtomicBool::new(false));
    // The CLI owns these process-lifetime handlers. Measurements only inspect
    // the shared flag; their single owner still performs termination and reap.
    for signal in [signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM] {
        signal_hook::flag::register(signal, Arc::clone(&cancellation))?;
    }
    let started = Instant::now();
    if options.is_quick() && !options.preflight_only {
        let provisional = campaign::initial_report(
            &options,
            &PreparedCampaign {
                temporary: None,
                datasets: Vec::new(),
            },
            &BTreeMap::new(),
            0,
        );
        write_report(&options.output, &provisional)?;
        eprintln!("tq-bench: report checkpoint: {}", options.output.display());
    }
    let work_budget = Duration::from_secs(options.campaign_budget_seconds);
    let work_budget = if options.is_quick() {
        work_budget
            .min(tq_test_support::benchmark::quick::remaining_work_budget().unwrap_or(work_budget))
    } else {
        work_budget
    };
    let deadline = deadline::Deadline::start(Arc::clone(&cancellation), None, work_budget)?;
    let result = run_campaign(&options, &root, &deadline, &cancellation, started);
    if options.is_quick() && (deadline.cancelled() || result.is_err()) && !options.preflight_only {
        let mut report: BenchmarkCampaignReport =
            serde_json::from_reader(fs::File::open(&options.output)?)?;
        if report.final_status == BenchmarkFinalStatus::Incomplete || deadline.cancelled() {
            report.final_status = BenchmarkFinalStatus::Incomplete;
            if let Some(execution) = &mut report.execution {
                execution.complete = false;
                execution.elapsed_seconds = started.elapsed().as_secs_f64();
                execution.interruptions.push(
                    if deadline.expired()
                        || tq_test_support::benchmark::quick::remaining_work_budget()
                            .is_some_and(|duration| duration.is_zero())
                    {
                        "campaign work budget exhausted".to_owned()
                    } else if deadline.cancelled() {
                        "cancelled by signal".to_owned()
                    } else {
                        format!(
                            "campaign preparation failed: {}",
                            result.as_ref().unwrap_err()
                        )
                    },
                );
            }
            write_report(&options.output, &report)?;
            print_report(&report)?;
            return Ok(ExitCode::from(2));
        }
    }
    result
}

#[allow(
    clippy::too_many_lines,
    reason = "campaign orchestration is intentionally linear and delegates measurement details"
)]
fn run_campaign(
    options: &Options,
    root: &Path,
    deadline: &deadline::Deadline,
    cancellation: &Arc<AtomicBool>,
    started: Instant,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    if deadline.cancelled() {
        return Err("campaign work budget exhausted before preflight".into());
    }
    let rss_preflight = preflight_rss(Some(Arc::clone(cancellation)))?;
    if !options.is_quick() {
        eprintln!(
            "tq-bench: RSS preflight passed ({})",
            rss_preflight.provenance.label()
        );
    }
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
    let tools = discover_tools(root)?;
    if deadline.cancelled() {
        return Err("campaign work budget exhausted during tool discovery".into());
    }
    if options.suite == "large-input" {
        for tool in [BenchmarkTool::Jq, BenchmarkTool::Tq, BenchmarkTool::Yq] {
            if tool == BenchmarkTool::Yq
                && !options
                    .selected_adapters
                    .iter()
                    .any(|adapter| adapter == "yq-json")
            {
                continue;
            }
            if !tools.contains_key(&tool) {
                return Err(format!(
                    "required large-input executable unavailable: {}",
                    tool_name(tool)
                )
                .into());
            }
        }
    } else {
        require_campaign_tools(&tools)?;
    }
    let mut prepared = if options.suite == "smoke" {
        prepare_smoke(&root.join("examples"))?
    } else if let Some(input) = options.input.as_deref() {
        large_input::prepare(input, cancellation)?
    } else {
        prepare_manifests(options)?
    };
    if options.suite != "large-input"
        && (options
            .selected_cases
            .iter()
            .any(|id| id.starts_with("benchmark.native-"))
            || options.selected_cases.is_empty())
    {
        let directory = prepared
            .temporary
            .as_ref()
            .ok_or("missing fixture directory")?;
        prepared
            .datasets
            .extend(native_rows::prepare(directory.path())?);
    }
    if deadline.cancelled() {
        return Err("campaign work budget exhausted during preparation".into());
    }
    let mut report = campaign::execute(
        options,
        &prepared,
        &tools,
        &calibrations,
        deadline,
        started,
        root,
    )?;
    if let Some(path) = &options.baseline {
        let baseline: BenchmarkCampaignReport = serde_json::from_reader(fs::File::open(path)?)?;
        report.comparability = compare_reports(&baseline, &report);
        report.regression_gate =
            evaluate_regression(&baseline, &report, options.regression_thresholds.clone());
        if report.final_status != BenchmarkFinalStatus::Incomplete
            && regression_gate_failed(&report.regression_gate)
        {
            report.final_status = BenchmarkFinalStatus::Regression;
        }
    }
    report
        .validate_authoritative_rss()
        .map_err(|error| format!("benchmark report RSS validation failed: {error}"))?;
    if !options.is_quick() || report.final_status == BenchmarkFinalStatus::Incomplete {
        write_report(&options.output, &report)?;
    }
    if let Some(markdown_dir) = &options.markdown_dir {
        report
            .validate_for_publication()
            .map_err(|error| format!("benchmark report publication validation failed: {error}"))?;
        render_markdown_pages(markdown_dir, &report)?;
    }
    let completion_stage =
        if options.is_quick() && report.final_status != BenchmarkFinalStatus::Incomplete {
            Some(
                tq_test_support::benchmark::quick::completion_stage()
                    .ok_or("quick completion requires a supervisor-owned report stage")?,
            )
        } else {
            None
        };
    print_report(&report)?;
    if let Some(stage) = completion_stage {
        write_report(&stage, &report)?;
    }
    Ok(exit_code_for_status(report.final_status))
}

fn print_report(report: &BenchmarkCampaignReport) -> std::io::Result<()> {
    use std::io::Write as _;

    let mut output = std::io::stdout().lock();
    let quick = report.profile == "quick"
        || report
            .execution
            .as_ref()
            .is_some_and(|execution| execution.sampling == "quick");
    if !quick {
        return write!(output, "{}", report.render_human());
    }
    let planned = report
        .execution
        .as_ref()
        .map_or(report.cases.len(), |execution| execution.planned_rows);
    let progress = if planned == 0 && report.final_status == BenchmarkFinalStatus::Incomplete {
        format!("{} / plan pending", report.cases.len())
    } else {
        format!("{} / {planned}", report.cases.len())
    };
    writeln!(
        output,
        "| Suite | Profile | Status | Completed / planned | Evidence |"
    )?;
    writeln!(output, "| --- | --- | --- | ---: | --- |")?;
    writeln!(
        output,
        "| {} | {} | {:?} | {progress} | Single-sample diagnostic; no warmup |",
        report.suite, report.profile, report.final_status,
    )?;
    writeln!(output)?;
    writeln!(
        output,
        "| Scenario | Input | Adapter | Outcome | Wall (us) | Peak RSS (bytes) |"
    )?;
    writeln!(output, "| --- | --- | --- | --- | ---: | ---: |")?;
    for row in &report.cases {
        write!(
            output,
            "| {} | {} | {} | {:?} | ",
            row.case_id, row.source_id, row.adapter_id, row.outcome
        )?;
        if let Some(summary) = &row.summary {
            write!(output, "{:.0} | ", summary.wall_time_micros.median)?;
            if let Some(rss) = summary.peak_rss_bytes {
                writeln!(output, "{rss} |")?;
            } else {
                writeln!(output, "unavailable |")?;
            }
        } else {
            writeln!(output, "unavailable | unavailable |")?;
        }
    }
    Ok(())
}

fn exit_code_for_status(status: BenchmarkFinalStatus) -> ExitCode {
    match status {
        BenchmarkFinalStatus::Passed => ExitCode::SUCCESS,
        BenchmarkFinalStatus::Incomplete => ExitCode::from(2),
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
                "selected benchmark case {} has no prepared datasets for this suite",
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

#[allow(
    clippy::too_many_lines,
    reason = "the command-line grammar stays intentionally explicit and dependency-free"
)]
fn options() -> Result<Options, Box<dyn std::error::Error>> {
    let mut suite = "natural-corpus".to_owned();
    let mut profile = "standard".to_owned();
    let archive_root = env::var_os("TQ_BENCHMARK_ARCHIVE_ROOT")
        .map_or_else(|| PathBuf::from("benchmarks"), PathBuf::from);
    let mut output = None;
    let mut manifests = Vec::new();
    let mut input = None;
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
    let mut mode = None;
    let mut sampling = None;
    let mut selected_adapters = Vec::new();
    let mut instrument_rss = false;
    let mut campaign_budget_seconds = None;
    let mut case_budget_seconds = None;
    let mut wall_time_percent: f64 = 50.0;
    let mut peak_rss_percent: f64 = 50.0;
    let mut minimum_samples = 5;
    let mut regression_options_supplied = false;
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "run" => {}
            "--suite" => suite = arguments.next().ok_or("--suite needs a value")?,
            "--profile" => profile = arguments.next().ok_or("--profile needs a value")?,
            "--input" => {
                input = Some(PathBuf::from(
                    arguments.next().ok_or("--input needs a path")?,
                ));
            }
            "--mode" => {
                mode = Some(Mode::parse(
                    &arguments.next().ok_or("--mode needs a value")?,
                )?);
            }
            "--sampling" => {
                sampling = Some(Sampling::parse(
                    &arguments.next().ok_or("--sampling needs a value")?,
                )?);
            }
            "--adapter" => selected_adapters.push(arguments.next().ok_or("--adapter needs an ID")?),
            "--instrument-rss" => instrument_rss = true,
            "--campaign-budget-seconds" => {
                campaign_budget_seconds = Some(
                    arguments
                        .next()
                        .ok_or("--campaign-budget-seconds needs a value")?
                        .parse::<u64>()?,
                );
            }
            "--case-budget-seconds" => {
                case_budget_seconds = Some(
                    arguments
                        .next()
                        .ok_or("--case-budget-seconds needs a value")?
                        .parse::<u64>()?,
                );
            }
            "--output" => {
                output = Some(PathBuf::from(
                    arguments.next().ok_or("--output needs a path")?,
                ));
            }
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
                regression_options_supplied = true;
                wall_time_percent = arguments
                    .next()
                    .ok_or("--wall-regression-percent needs a value")?
                    .parse()?;
            }
            "--rss-regression-percent" => {
                regression_options_supplied = true;
                peak_rss_percent = arguments
                    .next()
                    .ok_or("--rss-regression-percent needs a value")?
                    .parse()?;
            }
            "--minimum-regression-samples" => {
                regression_options_supplied = true;
                minimum_samples = arguments
                    .next()
                    .ok_or("--minimum-regression-samples needs a value")?
                    .parse()?;
            }
            "-h" | "--help" => {
                println!(
                    "Usage: tq-bench [--preflight-only] run [--suite natural-corpus|large-input|smoke] [--profile quick|standard|extended] [--output PATH] [--input GEOJSON] [--mode fast|exhaustive] [--sampling quick|screen|compare|extended|catalog] [--case ID] [--adapter ID] [--campaign-budget-seconds N] [--case-budget-seconds N] [--instrument-rss] [--manifest PATH --cache-root PATH --origin refreshed|frozen] [--max-samples N] [--timeout-seconds N] [--rss-limit-bytes N] [--timing-calibration SUMMARY ...] [--baseline PATH --wall-regression-percent N --rss-regression-percent N --minimum-regression-samples N] [--markdown-dir DIRECTORY] [--render-only REPORT... --markdown-dir DIRECTORY]\nNatural-corpus and standard are the defaults; quick and standard default to retained temporary report.json checkpoints. Large-input defaults to JSON adapters and accepts either --input GEOJSON or admitted --manifest paths. Extended large-input adds selected-sort to parse-discard and dead-sort-length."
                );
                std::process::exit(0);
            }
            value => return Err(format!("unknown argument: {value}").into()),
        }
    }
    if !matches!(suite.as_str(), "natural-corpus" | "large-input" | "smoke") {
        return Err(format!("invalid suite: {suite}").into());
    }
    if !matches!(profile.as_str(), "quick" | "standard" | "extended") {
        return Err(format!("invalid profile: {profile}").into());
    }
    let mode = mode.unwrap_or(if suite == "large-input" {
        Mode::Fast
    } else {
        Mode::Exhaustive
    });
    let sampling = sampling.unwrap_or(if profile == "quick" {
        Sampling::Quick
    } else if profile == "standard" && max_samples.is_none() {
        Sampling::Compare
    } else if profile == "extended" && max_samples.is_none() {
        Sampling::Extended
    } else if max_samples.is_some() {
        Sampling::Catalog
    } else if mode == Mode::Fast {
        Sampling::Screen
    } else {
        Sampling::Catalog
    });
    if profile == "quick" && sampling != Sampling::Quick {
        return Err("--profile quick requires --sampling quick".into());
    }
    if sampling == Sampling::Quick {
        if max_samples.is_some() {
            return Err("--sampling quick does not accept --max-samples".into());
        }
        if instrument_rss {
            return Err("--sampling quick does not accept --instrument-rss".into());
        }
        if baseline.is_some() || regression_options_supplied {
            return Err("--sampling quick does not accept baseline or regression options".into());
        }
        if markdown_dir.is_some() {
            return Err("--sampling quick cannot publish Markdown".into());
        }
    }
    if sampling == Sampling::Compare && selected_cases.is_empty() && profile != "standard" {
        return Err("--sampling compare requires at least one explicit --case".into());
    }
    if max_samples.is_some() && sampling != Sampling::Catalog {
        return Err("--max-samples requires --sampling catalog; use screen, compare or extended for fixed sampling policies".into());
    }
    if max_samples == Some(0) {
        return Err("--max-samples must be at least 1".into());
    }
    let quick_budget = tq_test_support::benchmark::quick::QUICK_WORK_BUDGET_SECONDS;
    if sampling == Sampling::Quick
        && (campaign_budget_seconds.is_some_and(|seconds| seconds > quick_budget)
            || case_budget_seconds.is_some_and(|seconds| seconds > quick_budget))
    {
        return Err(
            format!("quick campaign and case budgets cannot exceed {quick_budget}s").into(),
        );
    }
    let campaign_budget_seconds =
        campaign_budget_seconds.unwrap_or(if sampling == Sampling::Quick {
            quick_budget
        } else if mode == Mode::Fast {
            900
        } else {
            3600
        });
    let case_budget_seconds = case_budget_seconds.unwrap_or(if sampling == Sampling::Quick {
        quick_budget
    } else if mode == Mode::Fast {
        300
    } else {
        600
    });
    let case_budget_seconds = if sampling == Sampling::Quick {
        case_budget_seconds.min(campaign_budget_seconds)
    } else {
        case_budget_seconds
    };
    if campaign_budget_seconds == 0 || case_budget_seconds == 0 {
        return Err("campaign and case budgets must be at least 1 second".into());
    }
    if suite == "large-input" && mode == Mode::Fast && selected_cases.is_empty() {
        selected_cases.extend(policy::FAST_LARGE_CASES.iter().map(|id| (*id).to_owned()));
        if profile == "extended" {
            selected_cases.push("benchmark.large-selected-sort".to_owned());
        }
    }
    if suite == "large-input" && selected_adapters.is_empty() {
        selected_adapters.extend(
            policy::LARGE_JSON_ADAPTERS
                .iter()
                .map(|id| (*id).to_owned()),
        );
    }
    if input.is_some() && suite != "large-input" {
        return Err("--input is only supported by --suite large-input".into());
    }
    if input.is_some() && !manifests.is_empty() {
        return Err("--input and --manifest cannot be combined".into());
    }
    // Filesystem discovery belongs to run(), after the quick guard is active.
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
    let output = match output {
        Some(path) => path,
        None if !preflight_only
            && render_only.is_empty()
            && (matches!(profile.as_str(), "quick" | "standard")
                || sampling == Sampling::Quick) =>
        {
            env::var_os("TQ_QUICK_DEFAULT_OUTPUT").map_or_else(
                || {
                    env::temp_dir()
                        .join(format!(
                            "tq-bench-{}-{}",
                            std::process::id(),
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .expect("Unix epoch")
                                .as_nanos()
                        ))
                        .join("report.json")
                },
                PathBuf::from,
            )
        }
        None => archive_root.join(format!(".work/{suite}-{profile}.json")),
    };
    Ok(Options {
        suite,
        profile,
        output,
        input,
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
        mode,
        sampling,
        selected_adapters,
        instrument_rss,
        campaign_budget_seconds,
        case_budget_seconds,
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
        if (options.suite == "natural-corpus" && tier == DatasetTier::Large)
            || (options.suite == "large-input" && tier != DatasetTier::Large)
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
    if options.suite == "natural-corpus"
        && (options.selected_cases.is_empty()
            || options
                .selected_cases
                .iter()
                .any(|id| id == "benchmark.issue5-inputs"))
    {
        datasets.push(prepare_issue5_input_sequence(temporary.path())?);
    }
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
    {
        let mut writer = std::io::BufWriter::new(temporary.as_file_mut());
        serde_json::to_writer_pretty(&mut writer, report)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
    }
    temporary.as_file().sync_all()?;
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
    fn selected_case_without_suite_dataset_fails_closed() {
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
