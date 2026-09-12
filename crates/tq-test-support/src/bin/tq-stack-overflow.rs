//! Correctness-gated Stack Overflow jq benchmark campaign.

#[path = "tq_stack_overflow/outputs.rs"]
mod outputs;
#[path = "tq_stack_overflow/report.rs"]
mod report;

use std::{
    collections::BTreeMap,
    env, fs, io,
    path::{Path, PathBuf},
    process::ExitCode,
    time::Duration,
};

use report::{BenchmarkInput, load_scenarios, render_report_with_outputs, validate_report};
use sha2::{Digest as _, Sha256};
use tq_test_support::{
    benchmark::{
        BenchmarkAdapter, BenchmarkCampaignReport, BenchmarkCase, BenchmarkCorpusIdentity,
        BenchmarkFinalStatus, BenchmarkInvocation, BenchmarkLimits, BenchmarkOutcome,
        BenchmarkSampling, BenchmarkTool, Comparability, ComparisonFamily, DatasetFamily,
        DatasetSelector, DatasetTier, ExecutionClass, InputFormat, OutputContract,
        OutputContractKind, RegressionGate, collect_environment, normalize_correctness_run,
        populate_reference_ratios, preflight_rss, run_gated_row, unsupported_row,
    },
    compatibility::{ExecutableConfig, ProcessStatus, ToolIdentity, ToolKind, discover_tool},
    corpus::ArtifactIdentity,
};

const SAMPLE_COUNT: usize = 30;
const WARMUP_COUNT: usize = 1;
const TIMEOUT_SECONDS: u64 = 10;
const OUTPUT_LIMIT: u64 = 32 * 1024 * 1024;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tq-stack-overflow: {error}");
            ExitCode::from(2)
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Command {
    Run,
    Render,
    Outputs,
}

struct Options {
    command: Command,
    scenario_dir: PathBuf,
    output: PathBuf,
    report_dir: PathBuf,
    input: Option<PathBuf>,
}

#[allow(
    clippy::too_many_lines,
    reason = "campaign orchestration is intentionally linear and delegates measurement details"
)]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let options = options()?;
    if options.command == Command::Run {
        // RSS is an infrastructure prerequisite. Abandon before loading tools or
        // allocating campaign work when the host cannot provide authoritative RSS.
        let rss_preflight = preflight_rss(None)?;
        eprintln!(
            "tq-stack-overflow: RSS preflight passed ({})",
            rss_preflight.provenance.label()
        );
    }
    let scenarios = load_scenarios(&root.join(&options.scenario_dir))?;
    if scenarios.len() != 50 {
        return Err(format!(
            "expected exactly 50 Stack Overflow scenarios, found {}",
            scenarios.len()
        )
        .into());
    }

    if options.command != Command::Run {
        let input = options.input.as_ref().expect("saved report input required");
        let source_bytes = fs::read(path_from_root(&root, input))?;
        let mut saved = outputs::SavedReport::from_slice(&source_bytes)?;
        validate_report(&saved.benchmark, &scenarios)?;
        if options.command == Command::Outputs {
            let tools = discover_tools(&root)?;
            saved.output_comparison = Some(outputs::capture(&scenarios, &tools, &root)?);
            let enriched = outputs::attach_captures(
                &source_bytes,
                saved.output_comparison.as_ref().expect("captured outputs"),
            )?;
            write_json(&path_from_root(&root, &options.output), &enriched)?;
        }
        render_report_with_outputs(
            &saved.benchmark,
            &scenarios,
            &path_from_root(&root, &options.report_dir),
            &root,
            saved.output_comparison.as_ref(),
        )?;
        println!(
            "rendered {} Stack Overflow scenario pages to {}",
            scenarios.len(),
            path_from_root(&root, &options.report_dir).display()
        );
        return Ok(());
    }

    let tools = discover_tools(&root)?;
    let jq = tools
        .get(&BenchmarkTool::Jq)
        .ok_or("jq executable is required as the correctness reference")?;
    let mut rows = Vec::with_capacity(scenarios.len() * 3);
    let mut corpus = Vec::with_capacity(scenarios.len());
    for record in &scenarios {
        let case = benchmark_case(&record.scenario);
        let artifact = artifact_identity(&record.path, &record.input);
        let corpus_identity = BenchmarkCorpusIdentity {
            origin: "checked-in".to_owned(),
            source_id: record.scenario.id.clone(),
            tier: "small".to_owned(),
            format: InputFormat::Json,
            artifact,
            logical_records: 1,
            manifest_sha256: sha256_hex(&record.input),
        };
        corpus.push(corpus_identity.clone());
        let jq_adapter = adapter(BenchmarkTool::Jq);
        let jq_invocation = invocation(
            &record.scenario.benchmark,
            &jq_adapter,
            jq,
            &record.input,
            &root,
        );
        let reference = normalize_correctness_run(
            &jq_invocation,
            ToolKind::Jq,
            OutputContractKind::SemanticSequence,
        )?;
        if reference.process_status != ProcessStatus::Exited || reference.exit_code != Some(0) {
            return Err(
                format!("jq correctness reference failed for {}", record.scenario.id).into(),
            );
        }

        for tool in [BenchmarkTool::Jq, BenchmarkTool::Yq, BenchmarkTool::Tq] {
            let adapter = adapter(tool);
            let Some(identity) = tools.get(&tool) else {
                let placeholder = BenchmarkInvocation {
                    cancellation: None,
                    executable: PathBuf::from(tool_name(tool)),
                    args: tool_args(tool, &record.scenario.benchmark.query),
                    stdin: record.input.clone(),
                    current_dir: Some(root.clone()),
                    timeout: Duration::from_secs(TIMEOUT_SECONDS),
                    output_limit: OUTPUT_LIMIT,
                    rss_limit: None,
                    retain_output: false,
                };
                rows.push(unsupported_row(
                    &case,
                    &adapter,
                    &corpus_identity,
                    DatasetTier::Small,
                    &placeholder,
                ));
                continue;
            };
            let invocation = invocation(
                &record.scenario.benchmark,
                &adapter,
                identity,
                &record.input,
                &root,
            );
            rows.push(run_gated_row(
                &case,
                &adapter,
                &corpus_identity,
                DatasetTier::Small,
                &invocation,
                &reference,
            )?);
        }
    }
    populate_reference_ratios(&mut rows, &["jq-json"]);
    let has_failure = rows.iter().any(|row| {
        !matches!(
            row.outcome,
            BenchmarkOutcome::Timed | BenchmarkOutcome::Unsupported
        )
    });
    let report = BenchmarkCampaignReport {
        schema_version: 1,
        campaign_id: jiff::Timestamp::now().to_string(),
        profile: "stack-overflow".to_owned(),
        environment: collect_environment("release-benchmark"),
        corpus,
        tools: tools.values().cloned().collect(),
        cases: rows,
        comparability: Comparability::default(),
        regression_gate: RegressionGate::default(),
        final_status: if has_failure {
            BenchmarkFinalStatus::ObservedFailures
        } else {
            BenchmarkFinalStatus::Passed
        },
    };
    report
        .validate_authoritative_rss()
        .map_err(|error| format!("benchmark report RSS validation failed: {error}"))?;
    validate_report(&report, &scenarios)?;

    let output = path_from_root(&root, &options.output);
    let saved = outputs::SavedReport {
        benchmark: report,
        output_comparison: Some(outputs::capture(&scenarios, &tools, &root)?),
    };
    write_json(&output, &saved)?;
    render_report_with_outputs(
        &saved.benchmark,
        &scenarios,
        &path_from_root(&root, &options.report_dir),
        &root,
        saved.output_comparison.as_ref(),
    )?;
    println!(
        "benchmarked {} scenarios with {} samples; JSON report written to {}; pages written to {}",
        scenarios.len(),
        SAMPLE_COUNT,
        output.display(),
        path_from_root(&root, &options.report_dir).display()
    );
    if has_failure {
        return Err("one or more Stack Overflow benchmark rows failed".into());
    }
    Ok(())
}

fn options() -> Result<Options, Box<dyn std::error::Error>> {
    let mut command = Command::Run;
    let mut scenario_dir = PathBuf::from("tests/stack-overflow");
    let archive_root = env::var_os("TQ_BENCHMARK_ARCHIVE_ROOT")
        .map_or_else(|| PathBuf::from("benchmarks"), PathBuf::from);
    let mut output = archive_root.join(".work/stack-overflow.json");
    let mut report_dir = PathBuf::from("docs/tests/stack-overflow");
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
            "--scenario-dir" => {
                scenario_dir =
                    PathBuf::from(arguments.next().ok_or("--scenario-dir requires a path")?);
            }
            "--output" => {
                output = PathBuf::from(arguments.next().ok_or("--output requires a path")?);
            }
            "--report-dir" => {
                report_dir = PathBuf::from(
                    arguments
                        .next()
                        .ok_or("--report-dir requires a directory")?,
                );
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
                    "Usage: tq-stack-overflow run [--scenario-dir PATH] [--output PATH] [--report-dir DIR]\n       tq-stack-overflow render --input REPORT.json [--scenario-dir PATH] [--report-dir DIR]\n       tq-stack-overflow outputs --input REPORT.json --output ENRICHED.json [--report-dir DIR]\n\n--render-only PATH is an alias for render --input PATH."
                );
                std::process::exit(0);
            }
            value => return Err(format!("unknown argument: {value}").into()),
        }
    }
    if command != Command::Run && input.is_none() {
        return Err("render and outputs require --input REPORT.json".into());
    }
    Ok(Options {
        command,
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

fn benchmark_case(scenario: &report::Scenario) -> BenchmarkCase {
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
        sampling: BenchmarkSampling {
            warmups: WARMUP_COUNT,
            small: SAMPLE_COUNT,
            medium: SAMPLE_COUNT,
            large: SAMPLE_COUNT,
        },
        timeout_seconds: TIMEOUT_SECONDS,
        limits: BenchmarkLimits {
            output_bytes: OUTPUT_LIMIT,
            rss_bytes: None,
        },
        output_contract: OutputContract {
            kind: OutputContractKind::SemanticSequence,
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
    let args = tool_args(adapter.tool, &benchmark.query);
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

fn tool_args(tool: BenchmarkTool, query: &str) -> Vec<String> {
    let mut args = match tool {
        BenchmarkTool::Jq => vec!["-c".to_owned()],
        BenchmarkTool::Yq => vec![
            "-p=json".to_owned(),
            "-o=json".to_owned(),
            "-I=0".to_owned(),
        ],
        BenchmarkTool::Tq => vec!["--input-format".to_owned(), "json".to_owned()],
    };
    args.push(query.to_owned());
    args
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
