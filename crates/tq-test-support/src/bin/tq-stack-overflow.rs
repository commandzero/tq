//! Correctness-gated Stack Overflow jq benchmark campaign.

use std::{
    collections::BTreeMap,
    env,
    fmt::Write as _,
    fs, io,
    path::{Path, PathBuf},
    process::ExitCode,
    time::Duration,
};

use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use tq_test_support::{
    benchmark::{
        BenchmarkAdapter, BenchmarkCampaignReport, BenchmarkCase, BenchmarkCorpusIdentity,
        BenchmarkFinalStatus, BenchmarkInvocation, BenchmarkLimits, BenchmarkOutcome,
        BenchmarkSampling, BenchmarkTool, Comparability, ComparisonFamily, DatasetFamily,
        DatasetSelector, DatasetTier, ExecutionClass, InputFormat, OutputContract,
        OutputContractKind, RegressionGate, collect_environment, normalize_correctness_run,
        populate_reference_ratios, run_gated_row, unsupported_row,
    },
    compatibility::{ExecutableConfig, ProcessStatus, ToolIdentity, ToolKind, discover_tool},
    corpus::ArtifactIdentity,
};

const SAMPLE_COUNT: usize = 5;
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

#[derive(Debug, Deserialize)]
struct Scenario {
    id: String,
    rank: u32,
    source: Source,
    benchmark: BenchmarkInput,
}

#[derive(Debug, Deserialize)]
struct Source {
    title: String,
}

#[derive(Debug, Deserialize)]
struct BenchmarkInput {
    query: String,
    input: Value,
}

#[derive(Debug)]
struct ScenarioRecord {
    scenario: Scenario,
    path: PathBuf,
    input: Vec<u8>,
}

struct Options {
    scenario_dir: PathBuf,
    output: PathBuf,
    report: PathBuf,
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
    let scenarios = load_scenarios(&root.join(&options.scenario_dir))?;
    if scenarios.len() != 50 {
        return Err(format!(
            "expected exactly 50 Stack Overflow scenarios, found {}",
            scenarios.len()
        )
        .into());
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
                    executable: PathBuf::from(tool_name(tool)),
                    args: Vec::new(),
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
    write_json(&options.output, &report)?;
    write_report(&options.report, &report, &scenarios, &root)?;
    println!(
        "benchmarked {} scenarios with {} samples; report written to {}",
        scenarios.len(),
        SAMPLE_COUNT,
        options.report.display()
    );
    if has_failure {
        return Err("one or more Stack Overflow benchmark rows failed".into());
    }
    Ok(())
}

fn options() -> Result<Options, Box<dyn std::error::Error>> {
    let mut scenario_dir = PathBuf::from("tests/stack-overflow");
    let archive_root = env::var_os("TQ_BENCHMARK_ARCHIVE_ROOT")
        .map_or_else(|| PathBuf::from("benchmarks"), PathBuf::from);
    let mut output = archive_root.join(".work/stack-overflow.json");
    let mut report = archive_root.join("stack-overflow.md");
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "run" => {}
            "--scenario-dir" => {
                scenario_dir =
                    PathBuf::from(arguments.next().ok_or("--scenario-dir requires a path")?);
            }
            "--output" => {
                output = PathBuf::from(arguments.next().ok_or("--output requires a path")?);
            }
            "--report" => {
                report = PathBuf::from(arguments.next().ok_or("--report requires a path")?);
            }
            "-h" | "--help" => {
                println!(
                    "Usage: tq-stack-overflow run [--scenario-dir PATH] [--output PATH] [--report PATH]"
                );
                std::process::exit(0);
            }
            value => return Err(format!("unknown argument: {value}").into()),
        }
    }
    Ok(Options {
        scenario_dir,
        output,
        report,
    })
}

fn load_scenarios(directory: &Path) -> Result<Vec<ScenarioRecord>, Box<dyn std::error::Error>> {
    let mut paths = fs::read_dir(directory)?
        .map(|entry| entry.map(|value| value.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| {
        path.extension()
            .is_some_and(|extension| extension == "json")
    });
    paths.sort();
    let mut records = Vec::with_capacity(paths.len());
    for path in paths {
        let bytes = fs::read(&path)?;
        let scenario: Scenario = serde_json::from_slice(&bytes)?;
        let input = serde_json::to_vec(&scenario.benchmark.input)?;
        records.push(ScenarioRecord {
            scenario,
            path,
            input,
        });
    }
    records.sort_by_key(|record| record.scenario.rank);
    for (index, record) in records.iter().enumerate() {
        let expected = u32::try_from(index + 1).map_err(|_| "scenario rank overflow")?;
        if record.scenario.rank != expected {
            return Err(format!(
                "scenario {} has rank {}, expected {}",
                record.scenario.id, record.scenario.rank, expected
            )
            .into());
        }
    }
    Ok(records)
}

fn benchmark_case(scenario: &Scenario) -> BenchmarkCase {
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
    let mut args = match adapter.tool {
        BenchmarkTool::Jq => vec!["-c".to_owned()],
        BenchmarkTool::Yq => vec![
            "-p=json".to_owned(),
            "-o=json".to_owned(),
            "-I=0".to_owned(),
        ],
        BenchmarkTool::Tq => vec!["--input-format".to_owned(), "json".to_owned()],
    };
    args.push(benchmark.query.clone());
    BenchmarkInvocation {
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

fn write_json(path: &Path, report: &BenchmarkCampaignReport) -> Result<(), io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut bytes = serde_json::to_vec_pretty(report).map_err(io::Error::other)?;
    bytes.push(b'\n');
    fs::write(path, bytes)
}

fn write_report(
    path: &Path,
    report: &BenchmarkCampaignReport,
    scenarios: &[ScenarioRecord],
    root: &Path,
) -> Result<(), io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut output = String::new();
    writeln!(output, "# Stack Overflow jq gauntlet").expect("write report");
    writeln!(output).expect("write report");
    writeln!(
        output,
        "This report benchmarks the top 50 `jq`-tagged Stack Overflow questions sorted by votes: [questions tagged jq](https://stackoverflow.com/questions/tagged/jq?tab=votes&pagesize=50)."
    )
    .expect("write report");
    writeln!(
        output,
        "The checked-in scenarios retain each question and selected answer; accepted answers were preferred, with the highest-voted answer used when no accepted answer was available."
    )
    .expect("write report");
    writeln!(output).expect("write report");
    writeln!(output, "## Method\n").expect("write report");
    writeln!(
        output,
        "- One warmup followed by five measured samples per tool and scenario."
    )
    .expect("write report");
    writeln!(
        output,
        "- Time is the median wall-clock duration per scenario, reported in milliseconds."
    )
    .expect("write report");
    writeln!(output, "- Memory is the maximum observed resident set size across measured samples, reported in MiB.").expect("write report");
    writeln!(
        output,
        "- Deltas are percentage changes relative to `jq`: positive means slower or more memory."
    )
    .expect("write report");
    writeln!(
        output,
        "- Correctness is checked before timing using the shared semantic-sequence gate."
    )
    .expect("write report");
    writeln!(output).expect("write report");
    writeln!(output, "## Execution time (ms)\n").expect("write report");
    writeln!(output, "| # | Scenario | jq | yq | yq Δ | tq | tq Δ |").expect("write report");
    writeln!(output, "| ---: | --- | ---: | ---: | ---: | ---: | ---: |").expect("write report");
    for scenario in scenarios {
        let jq = row(report, &scenario.scenario.id, "jq-json");
        let yq = row(report, &scenario.scenario.id, "yq-json");
        let tq = row(report, &scenario.scenario.id, "tq-json");
        let relative_path = scenario
            .path
            .strip_prefix(root)
            .unwrap_or(&scenario.path)
            .display();
        let jq_ms = median_ms(jq);
        let yq_ms = median_ms(yq);
        let tq_ms = median_ms(tq);
        writeln!(
            output,
            "| {:02} | [{} — {}](../{}) · `{}` | {} | {} | {} | {} | {} |",
            scenario.scenario.rank,
            scenario.scenario.rank,
            escape_table(&scenario.scenario.source.title),
            relative_path,
            escape_table(&scenario.scenario.benchmark.query),
            jq_ms,
            yq_ms,
            delta(yq, jq),
            tq_ms,
            delta(tq, jq),
        )
        .expect("write report");
    }
    writeln!(output, "\n## Peak memory (MiB)\n").expect("write report");
    writeln!(output, "| # | Scenario | jq | yq | yq Δ | tq | tq Δ |").expect("write report");
    writeln!(output, "| ---: | --- | ---: | ---: | ---: | ---: | ---: |").expect("write report");
    for scenario in scenarios {
        let jq = row(report, &scenario.scenario.id, "jq-json");
        let yq = row(report, &scenario.scenario.id, "yq-json");
        let tq = row(report, &scenario.scenario.id, "tq-json");
        let relative_path = scenario
            .path
            .strip_prefix(root)
            .unwrap_or(&scenario.path)
            .display();
        writeln!(
            output,
            "| {:02} | [{} — {}](../{}) | {} | {} | {} | {} | {} |",
            scenario.scenario.rank,
            scenario.scenario.rank,
            escape_table(&scenario.scenario.source.title),
            relative_path,
            memory_mib(jq),
            memory_mib(yq),
            delta_memory(yq, jq),
            memory_mib(tq),
            delta_memory(tq, jq),
        )
        .expect("write report");
    }
    write_findings(&mut output, report);
    fs::write(path, output)
}

fn row<'a>(
    report: &'a BenchmarkCampaignReport,
    case_id: &str,
    adapter_id: &str,
) -> Option<&'a tq_test_support::benchmark::BenchmarkRow> {
    report
        .cases
        .iter()
        .find(|row| row.case_id == case_id && row.adapter_id == adapter_id)
}

fn median_ms(row: Option<&tq_test_support::benchmark::BenchmarkRow>) -> String {
    row.and_then(|row| row.summary.as_ref()).map_or_else(
        || "—".to_owned(),
        |summary| format!("{:.3}", summary.wall_time_micros.median / 1_000.0),
    )
}

#[allow(
    clippy::cast_precision_loss,
    reason = "RSS is formatted for human-readable MiB output"
)]
fn memory_mib(row: Option<&tq_test_support::benchmark::BenchmarkRow>) -> String {
    row.and_then(|row| row.summary.as_ref())
        .and_then(|summary| summary.peak_rss_bytes)
        .map_or_else(
            || "—".to_owned(),
            |bytes| format!("{:.2}", bytes as f64 / 1024.0 / 1024.0),
        )
}

fn delta(
    candidate: Option<&tq_test_support::benchmark::BenchmarkRow>,
    baseline: Option<&tq_test_support::benchmark::BenchmarkRow>,
) -> String {
    let candidate = candidate
        .and_then(|row| row.summary.as_ref())
        .map(|summary| summary.wall_time_micros.median);
    let baseline = baseline
        .and_then(|row| row.summary.as_ref())
        .map(|summary| summary.wall_time_micros.median);
    percentage(candidate, baseline)
}

#[allow(
    clippy::cast_precision_loss,
    reason = "RSS deltas are informational report values"
)]
fn delta_memory(
    candidate: Option<&tq_test_support::benchmark::BenchmarkRow>,
    baseline: Option<&tq_test_support::benchmark::BenchmarkRow>,
) -> String {
    let candidate = candidate
        .and_then(|row| row.summary.as_ref())
        .and_then(|summary| summary.peak_rss_bytes)
        .map(|bytes| bytes as f64);
    let baseline = baseline
        .and_then(|row| row.summary.as_ref())
        .and_then(|summary| summary.peak_rss_bytes)
        .map(|bytes| bytes as f64);
    percentage(candidate, baseline)
}

fn percentage(candidate: Option<f64>, baseline: Option<f64>) -> String {
    match (candidate, baseline) {
        (Some(candidate), Some(baseline)) if baseline > 0.0 => {
            format!("{:+.1}%", (candidate / baseline - 1.0) * 100.0)
        }
        _ => "—".to_owned(),
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "RSS averages are informational report values"
)]
fn write_findings(output: &mut String, report: &BenchmarkCampaignReport) {
    let means = ["jq-json", "yq-json", "tq-json"]
        .into_iter()
        .map(|adapter| {
            let values = report
                .cases
                .iter()
                .filter(|row| row.adapter_id == adapter)
                .filter_map(|row| {
                    row.summary
                        .as_ref()
                        .map(|summary| summary.wall_time_micros.median)
                })
                .collect::<Vec<_>>();
            (adapter, mean(&values))
        })
        .collect::<BTreeMap<_, _>>();
    let memories = ["jq-json", "yq-json", "tq-json"]
        .into_iter()
        .map(|adapter| {
            let values = report
                .cases
                .iter()
                .filter(|row| row.adapter_id == adapter)
                .filter_map(|row| {
                    row.summary
                        .as_ref()
                        .and_then(|summary| summary.peak_rss_bytes)
                })
                .map(|bytes| bytes as f64)
                .collect::<Vec<_>>();
            (adapter, mean(&values))
        })
        .collect::<BTreeMap<_, _>>();
    let tq_wins = report
        .cases
        .iter()
        .filter(|row| row.adapter_id == "tq-json")
        .filter_map(|row| {
            let tq = row.summary.as_ref()?.wall_time_micros.median;
            let jq = report
                .cases
                .iter()
                .find(|candidate| {
                    candidate.case_id == row.case_id && candidate.adapter_id == "jq-json"
                })?
                .summary
                .as_ref()?
                .wall_time_micros
                .median;
            Some(tq < jq)
        })
        .filter(|won| *won)
        .count();
    writeln!(output, "\n## Findings\n").expect("write report");
    let jq_time = means["jq-json"];
    let yq_time = means["yq-json"];
    let tq_time = means["tq-json"];
    writeln!(
        output,
        "- Mean per-scenario median time was **{:.3} ms for jq**, **{:.3} ms for yq** ({}) and **{:.3} ms for tq** ({}).",
        micros_to_ms(jq_time),
        micros_to_ms(yq_time),
        percentage(yq_time, jq_time),
        micros_to_ms(tq_time),
        percentage(tq_time, jq_time),
    )
    .expect("write report");
    writeln!(
        output,
        "- `tq` was faster than `jq` in **{tq_wins}/50** scenarios."
    )
    .expect("write report");
    let jq_memory = memories["jq-json"];
    let yq_memory = memories["yq-json"];
    let tq_memory = memories["tq-json"];
    writeln!(
        output,
        "- Mean peak RSS was **{:.2} MiB for jq**, **{:.2} MiB for yq** ({}) and **{:.2} MiB for tq** ({}).",
        bytes_to_mib(jq_memory),
        bytes_to_mib(yq_memory),
        percentage(yq_memory, jq_memory),
        bytes_to_mib(tq_memory),
        percentage(tq_memory, jq_memory),
    )
    .expect("write report");
    writeln!(output, "- These are small, single-document inputs, so process startup and fixed runtime overhead dominate; this is a compatibility-shaped smoke benchmark, not a large-data throughput claim.").expect("write report");
    writeln!(
        output,
        "\nRaw measurements are retained in the archive checkout's `.work/stack-overflow.json`."
    )
    .expect("write report");
}

#[allow(
    clippy::cast_precision_loss,
    reason = "mean denominator is a bounded report row count"
)]
fn mean(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

fn micros_to_ms(value: Option<f64>) -> f64 {
    value.unwrap_or(0.0) / 1_000.0
}

fn bytes_to_mib(value: Option<f64>) -> f64 {
    value.unwrap_or(0.0) / 1024.0 / 1024.0
}

fn escape_table(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}
