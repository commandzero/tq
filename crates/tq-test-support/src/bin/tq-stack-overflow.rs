//! Correctness-gated Stack Overflow jq benchmark campaign.

#[path = "tq-bench/deadline.rs"]
mod deadline;
#[path = "tq_stack_overflow/outputs.rs"]
mod outputs;
#[path = "tq_stack_overflow/report.rs"]
mod report;
use std::{
    collections::BTreeMap,
    env, fs, io,
    path::{Path, PathBuf},
    process::ExitCode,
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, Instant},
};

use report::{
    BenchmarkInput, load_scenarios, render_report_with_outputs, validate_publishable_report,
    validate_report,
};
use sha2::{Digest as _, Sha256};
use tq_test_support::{
    benchmark::{
        BenchmarkAdapter, BenchmarkCampaignReport, BenchmarkCase, BenchmarkCorpusIdentity,
        BenchmarkFinalStatus, BenchmarkInvocation, BenchmarkLimits, BenchmarkOutcome,
        BenchmarkSampling, BenchmarkTool, CampaignExecution, Comparability, ComparisonFamily,
        DatasetFamily, DatasetSelector, DatasetTier, ExecutionClass, InputFormat, OutputContract,
        OutputContractKind, RegressionGate, collect_environment, normalize_correctness_run,
        populate_reference_ratios, preflight_rss, run_gated_row, unsupported_row,
    },
    compatibility::{ExecutableConfig, ProcessStatus, ToolIdentity, ToolKind, discover_tool},
    corpus::ArtifactIdentity,
};

const SUITE: &str = "stack-overflow";
const DEFAULT_PROFILE: Profile = Profile::Standard;
const TIMEOUT_SECONDS: u64 = 10;
const OUTPUT_LIMIT: u64 = 32 * 1024 * 1024;

fn main() -> ExitCode {
    if matches!(
        env::args().nth(1).as_deref(),
        Some("--internal-rss-control" | "--internal-rss-probe")
    ) {
        return finish_run(run_internal());
    }
    let mut options = match options() {
        Ok(options) => options,
        Err(error) => {
            eprintln!("tq-stack-overflow: {error}");
            return ExitCode::from(2);
        }
    };
    let quick = options.command == Command::Run && options.profile == Profile::Quick;
    if quick && options.output.is_none() {
        options.output = Some(env::var_os("TQ_QUICK_DEFAULT_OUTPUT").map_or_else(
            || {
                env::temp_dir().join(format!(
                    "tq-stack-overflow-{}-{}.json",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .expect("Unix epoch")
                        .as_nanos(),
                ))
            },
            PathBuf::from,
        ));
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let report = options
        .output
        .as_deref()
        .map(|path| path_from_root(&root, path));
    match tq_test_support::benchmark::quick::supervise_current_if_quick(
        quick,
        Duration::from_secs(tq_test_support::benchmark::quick::QUICK_WORK_BUDGET_SECONDS),
        report.as_deref(),
    ) {
        Ok(Some(code)) => return ExitCode::from(u8::try_from(code).unwrap_or(2)),
        Err(error) => {
            eprintln!("tq-stack-overflow: quick supervisor: {error}");
            return ExitCode::from(2);
        }
        Ok(None) => {}
    }
    finish_run(run(&options))
}

fn finish_run(result: Result<(), Box<dyn std::error::Error>>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tq-stack-overflow: {error}");
            ExitCode::from(2)
        }
    }
}

fn run_internal() -> Result<(), Box<dyn std::error::Error>> {
    if env::args().nth(1).as_deref() == Some("--internal-rss-probe") {
        tq_test_support::benchmark::run_allocation_probe(64 * 1024 * 1024, 1)?;
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Command {
    Run,
    Render,
    Outputs,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Profile {
    Quick,
    Standard,
    Extended,
}

impl Profile {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "quick" => Ok(Self::Quick),
            "standard" => Ok(Self::Standard),
            "extended" => Ok(Self::Extended),
            _ => Err(format!(
                "unknown profile `{value}` (expected quick, standard, or extended)"
            )),
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Quick => "quick",
            Self::Standard => "standard",
            Self::Extended => "extended",
        }
    }

    const fn sampling(self) -> BenchmarkSampling {
        let (warmups, samples) = match self {
            Self::Quick => (0, 1),
            Self::Standard => (1, 3),
            Self::Extended => (1, 5),
        };
        BenchmarkSampling {
            warmups,
            small: samples,
            medium: samples,
            large: samples,
        }
    }
}

struct Options {
    command: Command,
    profile: Profile,
    scenario_dir: PathBuf,
    output: Option<PathBuf>,
    report_dir: Option<PathBuf>,
    input: Option<PathBuf>,
}

#[allow(
    clippy::too_many_lines,
    reason = "campaign orchestration is intentionally linear and delegates measurement details"
)]
fn run(options: &Options) -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let quick_start = Instant::now();
    let quick_output = if options.command == Command::Run && options.profile == Profile::Quick {
        Some(if let Some(path) = options.output.as_deref() {
            path_from_root(&root, path)
        } else {
            tempfile::Builder::new()
                .prefix("tq-stack-overflow-")
                .suffix(".json")
                .tempfile()?
                .keep()?
                .1
        })
    } else {
        None
    };
    let quick_deadline = if let Some(output) = &quick_output {
        let cancellation = Arc::new(AtomicBool::new(false));
        for signal in [signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM] {
            signal_hook::flag::register(signal, Arc::clone(&cancellation))?;
        }
        let work =
            Duration::from_secs(tq_test_support::benchmark::quick::QUICK_WORK_BUDGET_SECONDS);
        let remaining = tq_test_support::benchmark::quick::remaining_work_budget().unwrap_or(work);
        let deadline = deadline::Deadline::start(cancellation, None, work.min(remaining))?;
        let provisional = quick_report(quick_start);
        write_quick_json(
            output,
            &QuickCheckpoint {
                benchmark: &provisional,
            },
        )?;
        eprintln!("tq-stack-overflow: session report: {}", output.display());
        Some(deadline)
    } else {
        None
    };
    if options.command == Command::Run {
        let rss_preflight = match preflight_rss(
            quick_deadline
                .as_ref()
                .map(|deadline| Arc::clone(deadline.flag())),
        ) {
            Ok(preflight) => preflight,
            Err(error) => {
                if let (Some(output), Some(deadline)) = (&quick_output, &quick_deadline) {
                    finish_quick_setup(output, deadline, quick_start, &error)?;
                }
                return Err(error.into());
            }
        };
        if options.profile != Profile::Quick {
            eprintln!(
                "tq-stack-overflow: RSS preflight passed ({})",
                rss_preflight.provenance.label()
            );
        }
    }
    let scenarios = match load_scenarios(&root.join(&options.scenario_dir)) {
        Ok(scenarios) => scenarios,
        Err(error) => {
            if let (Some(output), Some(deadline)) = (&quick_output, &quick_deadline) {
                finish_quick_setup(output, deadline, quick_start, &error)?;
            }
            return Err(error.into());
        }
    };
    if scenarios.len() != 50 {
        return Err(format!(
            "expected exactly 50 Stack Overflow scenarios, found {}",
            scenarios.len()
        )
        .into());
    }

    if options.command != Command::Run {
        return render_saved_report(&root, options, &scenarios);
    }
    if let (Some(output), Some(deadline)) = (&quick_output, &quick_deadline) {
        return run_quick(&root, &scenarios, output, deadline, quick_start);
    }

    let tools = discover_tools(&root)?;
    let mut report = BenchmarkCampaignReport {
        execution: None,
        schema_version: 1,
        campaign_id: jiff::Timestamp::now().to_string(),
        suite: SUITE.to_owned(),
        profile: options.profile.name().to_owned(),
        environment: collect_environment("release-benchmark"),
        corpus: Vec::with_capacity(scenarios.len()),
        tools: tools.values().cloned().collect(),
        cases: Vec::with_capacity(scenarios.len() * 3),
        comparability: Comparability::default(),
        regression_gate: RegressionGate::default(),
        final_status: BenchmarkFinalStatus::Incomplete,
    };
    collect_scenario_rows(
        &root,
        &scenarios,
        options.profile,
        &tools,
        &mut report,
        None,
        |_| Ok(()),
    )?;
    populate_reference_ratios(&mut report.cases, &["jq-json"]);
    report.final_status = if report.cases.iter().any(|row| {
        !matches!(
            row.outcome,
            BenchmarkOutcome::Timed | BenchmarkOutcome::Unsupported
        )
    }) {
        BenchmarkFinalStatus::ObservedFailures
    } else {
        BenchmarkFinalStatus::Passed
    };
    report
        .validate_authoritative_rss()
        .map_err(|error| format!("benchmark report RSS validation failed: {error}"))?;
    validate_report(&report, &scenarios)?;

    let captured = outputs::capture(&scenarios, &tools, &root)?;
    outputs::validate(&captured, &scenarios)
        .map_err(|error| format!("output comparison failed: {error}"))?;
    let saved = outputs::SavedReport {
        benchmark: report,
        output_comparison: Some(captured),
    };
    let output = if let Some(path) = options.output.as_deref() {
        path_from_root(&root, path)
    } else {
        let (_, path) = tempfile::Builder::new()
            .prefix("tq-stack-overflow-")
            .suffix(".json")
            .tempfile()?
            .keep()?;
        path
    };
    write_json(&output, &saved)?;
    if options.profile == Profile::Extended {
        let report_dir = path_from_root(
            &root,
            options
                .report_dir
                .as_deref()
                .unwrap_or(Path::new("docs/tests/stack-overflow")),
        );
        render_report_with_outputs(
            &saved.benchmark,
            &scenarios,
            &report_dir,
            &root,
            saved.output_comparison.as_ref(),
        )?;
        println!(
            "benchmarked {} scenarios with {} samples; JSON report written to {}; pages written to {}",
            scenarios.len(),
            options.profile.sampling().small,
            output.display(),
            report_dir.display()
        );
    } else {
        eprintln!("tq-stack-overflow: session report: {}", output.display());
        println!(
            "benchmarked {} scenarios with {} samples",
            scenarios.len(),
            options.profile.sampling().small,
        );
    }
    if saved.benchmark.final_status == BenchmarkFinalStatus::ObservedFailures {
        return Err("one or more Stack Overflow benchmark rows failed".into());
    }
    Ok(())
}

fn render_saved_report(
    root: &Path,
    options: &Options,
    scenarios: &[report::ScenarioRecord],
) -> Result<(), Box<dyn std::error::Error>> {
    let input = options.input.as_ref().expect("saved report input required");
    let source_bytes = fs::read(path_from_root(root, input))?;
    let mut saved = outputs::SavedReport::from_slice(&source_bytes)?;
    validate_report(&saved.benchmark, scenarios)?;
    validate_publishable_report(&saved.benchmark)?;
    if options.command == Command::Outputs {
        let tools = discover_tools(root)?;
        let captured = outputs::capture(scenarios, &tools, root)?;
        outputs::validate(&captured, scenarios)
            .map_err(|error| format!("output comparison failed: {error}"))?;
        saved.output_comparison = Some(captured);
        let enriched = outputs::attach_captures(
            &source_bytes,
            saved.output_comparison.as_ref().expect("captured outputs"),
        )?;
        let output = options.output.clone().unwrap_or_else(|| {
            env::var_os("TQ_BENCHMARK_ARCHIVE_ROOT")
                .map_or_else(|| PathBuf::from("benchmarks"), PathBuf::from)
                .join(".work/stack-overflow.json")
        });
        write_json(&path_from_root(root, &output), &enriched)?;
    }
    let report_dir = path_from_root(
        root,
        options
            .report_dir
            .as_deref()
            .unwrap_or(Path::new("docs/tests/stack-overflow")),
    );
    render_report_with_outputs(
        &saved.benchmark,
        scenarios,
        &report_dir,
        root,
        saved.output_comparison.as_ref(),
    )?;
    println!(
        "rendered {} Stack Overflow scenario pages to {}",
        scenarios.len(),
        report_dir.display()
    );
    Ok(())
}

fn quick_report(started: Instant) -> BenchmarkCampaignReport {
    BenchmarkCampaignReport {
        execution: Some(CampaignExecution {
            mode: "exhaustive".to_owned(),
            sampling: "quick".to_owned(),
            instrument_rss: false,
            campaign_budget_seconds: tq_test_support::benchmark::quick::QUICK_WORK_BUDGET_SECONDS,
            case_budget_seconds: tq_test_support::benchmark::quick::QUICK_WORK_BUDGET_SECONDS,
            planned_rows: 50 * 3,
            elapsed_seconds: started.elapsed().as_secs_f64(),
            complete: false,
            interruptions: Vec::new(),
        }),
        schema_version: 1,
        campaign_id: jiff::Timestamp::now().to_string(),
        suite: SUITE.to_owned(),
        profile: "quick".to_owned(),
        environment: collect_environment("release-benchmark"),
        corpus: Vec::new(),
        tools: Vec::new(),
        cases: Vec::new(),
        comparability: Comparability::default(),
        regression_gate: RegressionGate::default(),
        final_status: BenchmarkFinalStatus::Incomplete,
    }
}

#[derive(serde::Serialize)]
struct QuickCheckpoint<'a> {
    #[serde(flatten)]
    benchmark: &'a BenchmarkCampaignReport,
}

fn checkpoint_quick(
    path: &Path,
    report: &mut BenchmarkCampaignReport,
    started: Instant,
) -> Result<(), io::Error> {
    report
        .execution
        .as_mut()
        .expect("quick execution")
        .elapsed_seconds = started.elapsed().as_secs_f64();
    write_quick_json(path, &QuickCheckpoint { benchmark: report })
}

fn write_quick_json(path: &Path, report: &QuickCheckpoint<'_>) -> Result<(), io::Error> {
    use io::Write as _;

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer_pretty(temporary.as_file_mut(), report).map_err(io::Error::other)?;
    temporary.as_file_mut().write_all(b"\n")?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

fn finish_quick_setup(
    output: &Path,
    deadline: &deadline::Deadline,
    started: Instant,
    error: &dyn std::fmt::Display,
) -> Result<(), io::Error> {
    let mut report = quick_report(started);
    report
        .execution
        .as_mut()
        .expect("quick execution")
        .interruptions
        .push(if deadline.expired() {
            "campaign work budget exhausted during preparation".to_owned()
        } else {
            format!("campaign preparation interrupted: {error}")
        });
    checkpoint_quick(output, &mut report, started)?;
    println!(
        "| Suite | Profile | Status | Completed / planned |\n| --- | --- | --- | ---: |\n| {SUITE} | quick | Incomplete | 0 / 150 |"
    );
    eprintln!("tq-stack-overflow: session report: {}", output.display());
    Ok(())
}

fn run_quick(
    root: &Path,
    scenarios: &[report::ScenarioRecord],
    output: &Path,
    deadline: &deadline::Deadline,
    started: Instant,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut report = quick_report(started);
    let outcome = collect_quick_rows(root, scenarios, output, deadline, started, &mut report);
    if let Err(error) = &outcome {
        let message = if deadline.expired() {
            "campaign work budget exhausted".to_owned()
        } else if deadline.cancelled() {
            "campaign cancelled by signal".to_owned()
        } else {
            format!("campaign interrupted: {error}")
        };
        report
            .execution
            .as_mut()
            .expect("quick execution")
            .interruptions
            .push(message);
    }
    if outcome.is_ok() && !deadline.cancelled() && report.cases.len() == scenarios.len() * 3 {
        validate_report(&report, scenarios)?;
        if report.cases.iter().any(|row| {
            !matches!(
                row.outcome,
                BenchmarkOutcome::Timed | BenchmarkOutcome::Unsupported
            )
        }) {
            report.final_status = BenchmarkFinalStatus::ObservedFailures;
        } else {
            report.final_status = BenchmarkFinalStatus::Passed;
        }
        report.execution.as_mut().expect("quick execution").complete = true;
    }
    populate_reference_ratios(&mut report.cases, &["jq-json"]);
    report.validate_authoritative_rss()?;
    if deadline.cancelled() || deadline.remaining().is_zero() {
        report.final_status = BenchmarkFinalStatus::Incomplete;
        let execution = report.execution.as_mut().expect("quick execution");
        execution.complete = false;
        if execution.interruptions.is_empty() {
            execution
                .interruptions
                .push("campaign work budget exhausted during reporting".to_owned());
        }
    }
    if report.final_status == BenchmarkFinalStatus::Incomplete {
        checkpoint_quick(output, &mut report, started)?;
    } else {
        let status = report.final_status;
        report.final_status = BenchmarkFinalStatus::Incomplete;
        report.execution.as_mut().expect("quick execution").complete = false;
        checkpoint_quick(output, &mut report, started)?;
        report.final_status = status;
        report.execution.as_mut().expect("quick execution").complete = true;
    }
    let completion_stage = if report.final_status == BenchmarkFinalStatus::Incomplete {
        None
    } else {
        Some(
            tq_test_support::benchmark::quick::completion_stage()
                .ok_or("quick completion requires a supervisor-owned report stage")?,
        )
    };
    println!(
        "| Suite | Profile | Status | Completed / planned |\n| --- | --- | --- | ---: |\n| {} | quick | {:?} | {} / {} |\n",
        SUITE,
        report.final_status,
        report.cases.len(),
        scenarios.len() * 3
    );
    println!("{}", report::render_quick_table(&report, scenarios));
    eprintln!("tq-stack-overflow: session report: {}", output.display());
    if let Some(stage) = completion_stage {
        checkpoint_quick(&stage, &mut report, started)?;
    }
    outcome?;
    if report.final_status != BenchmarkFinalStatus::Passed {
        return Err("Stack Overflow benchmark was incomplete or had failing rows".into());
    }
    Ok(())
}

fn scenario_corpus_identity(record: &report::ScenarioRecord) -> BenchmarkCorpusIdentity {
    BenchmarkCorpusIdentity {
        origin: "checked-in".to_owned(),
        source_id: record.scenario.id.clone(),
        tier: "small".to_owned(),
        format: InputFormat::Json,
        artifact: artifact_identity(&record.path, &record.input),
        logical_records: 1,
        manifest_sha256: sha256_hex(&record.input),
    }
}

fn collect_quick_rows(
    root: &Path,
    scenarios: &[report::ScenarioRecord],
    output: &Path,
    deadline: &deadline::Deadline,
    started: Instant,
    report: &mut BenchmarkCampaignReport,
) -> Result<(), Box<dyn std::error::Error>> {
    if deadline.cancelled() || deadline.remaining().is_zero() {
        return Err("campaign work budget exhausted before tool discovery".into());
    }
    let tools = discover_tools(root)?;
    report.tools = tools.values().cloned().collect();
    checkpoint_quick(output, report, started)?;
    collect_scenario_rows(
        root,
        scenarios,
        Profile::Quick,
        &tools,
        report,
        Some(deadline),
        |report| checkpoint_quick(output, report, started),
    )
}

fn collect_scenario_rows(
    root: &Path,
    scenarios: &[report::ScenarioRecord],
    profile: Profile,
    tools: &BTreeMap<BenchmarkTool, ToolIdentity>,
    report: &mut BenchmarkCampaignReport,
    deadline: Option<&deadline::Deadline>,
    mut record_progress: impl FnMut(&mut BenchmarkCampaignReport) -> io::Result<()>,
) -> Result<(), Box<dyn std::error::Error>> {
    let jq = tools
        .get(&BenchmarkTool::Jq)
        .ok_or("jq executable is required as the correctness reference")?;
    for record in scenarios {
        if deadline.is_some_and(|deadline| deadline.cancelled() || deadline.remaining().is_zero()) {
            return Err("campaign work budget exhausted".into());
        }
        let case = benchmark_case(&record.scenario, profile);
        report.corpus.push(scenario_corpus_identity(record));
        let jq_adapter = adapter(BenchmarkTool::Jq);
        let mut jq_invocation = invocation(
            &record.scenario.benchmark,
            &jq_adapter,
            jq,
            &record.input,
            root,
        );
        jq_invocation.cancellation = deadline.map(|deadline| Arc::clone(deadline.flag()));
        let reference = normalize_correctness_run(
            &jq_invocation,
            ToolKind::Jq,
            output_contract_kind(record.scenario.benchmark.output_mode),
        )?;
        if reference.process_status != ProcessStatus::Exited || reference.exit_code != Some(0) {
            return Err(
                format!("jq correctness reference failed for {}", record.scenario.id).into(),
            );
        }
        for tool in [BenchmarkTool::Jq, BenchmarkTool::Yq, BenchmarkTool::Tq] {
            if deadline
                .is_some_and(|deadline| deadline.cancelled() || deadline.remaining().is_zero())
            {
                return Err("campaign work budget exhausted".into());
            }
            let adapter = adapter(tool);
            let corpus_identity = report.corpus.last().expect("current corpus input");
            let row = if let Some(identity) = tools.get(&tool) {
                let mut invocation = invocation(
                    &record.scenario.benchmark,
                    &adapter,
                    identity,
                    &record.input,
                    root,
                );
                invocation.cancellation = deadline.map(|deadline| Arc::clone(deadline.flag()));
                run_gated_row(
                    &case,
                    &adapter,
                    corpus_identity,
                    DatasetTier::Small,
                    &invocation,
                    &reference,
                    false,
                )?
            } else {
                let placeholder = BenchmarkInvocation {
                    cancellation: deadline.map(|deadline| Arc::clone(deadline.flag())),
                    executable: PathBuf::from(tool_name(tool)),
                    args: tool_args(
                        record.scenario.benchmark.output_mode,
                        tool,
                        record.scenario.benchmark.query_for(tool),
                    ),
                    stdin: record.input.clone(),
                    current_dir: Some(root.to_owned()),
                    timeout: Duration::from_secs(TIMEOUT_SECONDS),
                    output_limit: OUTPUT_LIMIT,
                    rss_limit: None,
                    retain_output: false,
                };
                unsupported_row(
                    &case,
                    &adapter,
                    corpus_identity,
                    DatasetTier::Small,
                    &placeholder,
                )
            };
            report.cases.push(row);
            record_progress(report)?;
        }
    }
    Ok(())
}

fn options() -> Result<Options, Box<dyn std::error::Error>> {
    let mut command = Command::Run;
    let mut profile = DEFAULT_PROFILE;
    let mut profile_was_specified = false;
    let mut scenario_dir = PathBuf::from("tests/stack-overflow");
    let mut output = None;
    let mut report_dir = None;
    let mut input = None;
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "run" => {
                if input.is_some() || command != Command::Run {
                    return Err("run cannot be combined with render arguments".into());
                }
            }
            "render" => command = Command::Render,
            "outputs" => command = Command::Outputs,
            "--profile" => {
                profile_was_specified = true;
                profile = Profile::parse(&arguments.next().ok_or("--profile requires a value")?)?;
            }
            "--scenario-dir" => {
                scenario_dir =
                    PathBuf::from(arguments.next().ok_or("--scenario-dir requires a path")?);
            }
            "--output" => {
                output = Some(PathBuf::from(
                    arguments.next().ok_or("--output requires a path")?,
                ));
            }
            "--report-dir" => {
                report_dir = Some(PathBuf::from(
                    arguments
                        .next()
                        .ok_or("--report-dir requires a directory")?,
                ));
            }
            "--input" => {
                input = Some(PathBuf::from(
                    arguments.next().ok_or("--input requires a path")?,
                ));
                if command == Command::Run {
                    command = Command::Render;
                }
            }
            "--render-only" => {
                input = Some(PathBuf::from(
                    arguments
                        .next()
                        .ok_or("--render-only requires a report path")?,
                ));
                command = Command::Render;
            }
            "-h" | "--help" => {
                println!(
                    "Usage: tq-stack-overflow run [--profile quick|standard|extended] [--scenario-dir PATH] [--output PATH] [--report-dir DIR]\n       tq-stack-overflow render --input REPORT.json [--scenario-dir PATH] [--report-dir DIR]\n       tq-stack-overflow outputs --input REPORT.json --output ENRICHED.json [--report-dir DIR]\n\nThe default run profile is standard. Quick and standard keep inspectable session JSON in a temporary file by default; quick prints only a table to stdout. Only extended generates pages."
                );
                std::process::exit(0);
            }
            value => return Err(format!("unknown argument: {value}").into()),
        }
    }
    if command != Command::Run && input.is_none() {
        return Err("render and outputs require --input REPORT.json".into());
    }
    if command != Command::Run && profile_was_specified {
        return Err(
            "--profile is only valid for run; render and outputs use the report profile".into(),
        );
    }
    if command == Command::Run && profile != Profile::Extended && report_dir.is_some() {
        return Err("--report-dir is only valid for extended runs or render/outputs".into());
    }
    Ok(Options {
        command,
        profile,
        scenario_dir,
        output,
        report_dir,
        input,
    })
}

fn path_from_root(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        root.join(path)
    }
}

fn benchmark_case(scenario: &report::Scenario, profile: Profile) -> BenchmarkCase {
    BenchmarkCase {
        schema_version: 1,
        id: scenario.id.clone(),
        compatibility_gate: "stack-overflow".to_owned(),
        dataset_selector: DatasetSelector {
            family: DatasetFamily::SyntheticHelper,
            tiers: vec![DatasetTier::Small],
        },
        query: scenario.benchmark.query.clone(),
        execution_class: ExecutionClass::Document,
        measure_first_result: false,
        sampling: profile.sampling(),
        timeout_seconds: TIMEOUT_SECONDS,
        limits: BenchmarkLimits {
            output_bytes: OUTPUT_LIMIT,
            rss_bytes: None,
        },
        output_contract: OutputContract {
            kind: output_contract_kind(scenario.benchmark.output_mode),
            reference_adapter: "jq-json".to_owned(),
        },
        adapters: vec![
            adapter(BenchmarkTool::Jq),
            adapter(BenchmarkTool::Yq),
            adapter(BenchmarkTool::Tq),
        ],
    }
}

fn adapter(tool: BenchmarkTool) -> BenchmarkAdapter {
    BenchmarkAdapter {
        id: format!("{}-json", tool_name(tool)),
        tool,
        input_format: InputFormat::Json,
        applicable: true,
        unsupported_reason: None,
        args: Vec::new(),
        query: None,
        comparison_families: vec![ComparisonFamily::SameFormat],
    }
}

fn invocation(
    benchmark: &BenchmarkInput,
    adapter: &BenchmarkAdapter,
    identity: &ToolIdentity,
    input: &[u8],
    root: &Path,
) -> BenchmarkInvocation {
    let args = tool_args(
        benchmark.output_mode,
        adapter.tool,
        benchmark.query_for(adapter.tool),
    );
    BenchmarkInvocation {
        cancellation: None,
        executable: identity.path.clone(),
        args,
        stdin: input.to_vec(),
        current_dir: Some(root.to_owned()),
        timeout: Duration::from_secs(TIMEOUT_SECONDS),
        output_limit: OUTPUT_LIMIT,
        rss_limit: None,
        retain_output: false,
    }
}

fn tool_args(mode: report::OutputMode, tool: BenchmarkTool, query: &str) -> Vec<String> {
    outputs::profile_args(mode, tool, query)
}

fn output_contract_kind(mode: report::OutputMode) -> OutputContractKind {
    match mode {
        report::OutputMode::Structured => OutputContractKind::SemanticSequence,
        report::OutputMode::Raw => OutputContractKind::RawBytes,
        report::OutputMode::Color => OutputContractKind::ColoredSemanticSequence,
    }
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

fn artifact_identity(path: &Path, bytes: &[u8]) -> ArtifactIdentity {
    ArtifactIdentity {
        path: path.display().to_string(),
        bytes: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        sha256: sha256_hex(bytes),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            use std::fmt::Write as _;
            write!(output, "{byte:02x}").expect("write digest");
            output
        })
}

fn tool_name(tool: BenchmarkTool) -> &'static str {
    match tool {
        BenchmarkTool::Jq => "jq",
        BenchmarkTool::Yq => "yq",
        BenchmarkTool::Tq => "tq",
    }
}

fn write_json(path: &Path, report: &impl serde::Serialize) -> Result<(), io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut bytes = serde_json::to_vec_pretty(report).map_err(io::Error::other)?;
    bytes.push(b'\n');
    fs::write(path, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sampling_tracks_run_profile() {
        for (profile, warmups, samples) in [
            (Profile::Quick, 0, 1),
            (Profile::Standard, 1, 3),
            (Profile::Extended, 1, 5),
        ] {
            let sampling = profile.sampling();
            assert_eq!(sampling.warmups, warmups);
            assert_eq!(sampling.small, samples);
            assert_eq!(sampling.medium, samples);
            assert_eq!(sampling.large, samples);
        }
    }
}
