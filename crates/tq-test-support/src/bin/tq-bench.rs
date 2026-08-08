//! Local correctness-gated benchmark campaign driver.

use std::{
    collections::BTreeMap,
    env, fs,
    io::Write as _,
    path::{Path, PathBuf},
    process::ExitCode,
    time::Duration,
};

use sha2::{Digest, Sha256};
use tempfile::TempDir;
use tq_test_support::{
    benchmark::{
        BenchmarkCampaignReport, BenchmarkCorpusIdentity, BenchmarkFinalStatus,
        BenchmarkInvocation, BenchmarkOutcome, BenchmarkSampling, BenchmarkTool, Comparability,
        DatasetTier, InputFormat, RegressionGate, RegressionThresholds, collect_environment,
        compare_reports, evaluate_regression, is_correctness_output_limit, load_benchmark_catalog,
        normalize_correctness_run, populate_reference_ratios, run_correctness_limit_probe,
        run_gated_row, unsupported_row,
    },
    compatibility::{ExecutableConfig, ToolIdentity, ToolKind, discover_tool},
    corpus::{
        ArtifactIdentity, SmokeSnapshot, discover_smoke_corpus, generate_representations,
        load_frozen_snapshot,
    },
};

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
    _temporary: Option<TempDir>,
    datasets: Vec<PreparedDataset>,
}

#[allow(
    clippy::too_many_lines,
    reason = "campaign orchestration is intentionally linear and delegates measurement details"
)]
fn run() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let options = options()?;
    let prepared = if options.profile == "smoke" {
        prepare_smoke(&root.join("examples"))?
    } else {
        prepare_manifests(&options)?
    };
    let catalog = load_benchmark_catalog(&root.join("benchmarks/cases"))?;
    let tools = discover_tools(&root)?;
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
                .ok_or("reference jq executable is unavailable")?;
            let reference_invocation =
                invocation(&case, reference_adapter, dataset, reference_identity)?;
            let reference = match normalize_correctness_run(
                &reference_invocation,
                ToolKind::Jq,
                case.output_contract.kind,
            ) {
                Ok(reference) => Some(reference),
                Err(error) if is_correctness_output_limit(&error) => None,
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
                    rows.push(unsupported_row(
                        &case,
                        adapter,
                        &corpus_identity,
                        dataset.tier,
                        &placeholder,
                    ));
                    continue;
                };
                let invocation = invocation(&case, adapter, dataset, identity)?;
                if !adapter.applicable {
                    rows.push(unsupported_row(
                        &case,
                        adapter,
                        &corpus_identity,
                        dataset.tier,
                        &invocation,
                    ));
                    continue;
                }
                rows.push(if let Some(reference) = &reference {
                    run_gated_row(
                        &case,
                        adapter,
                        &corpus_identity,
                        dataset.tier,
                        &invocation,
                        reference,
                    )?
                } else {
                    run_correctness_limit_probe(
                        &case,
                        adapter,
                        &corpus_identity,
                        dataset.tier,
                        &invocation,
                    )?
                });
            }
        }
    }
    populate_reference_ratios(&mut rows, &["jq-json", "yq-json", "yq-yaml"]);
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
        environment: collect_environment("release-benchmark"),
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
        if report.regression_gate.evaluated && !report.regression_gate.failures.is_empty() {
            report.final_status = BenchmarkFinalStatus::Regression;
        }
    }
    write_report(&options.output, &report)?;
    print!("{}", report.render_human());
    Ok(ExitCode::SUCCESS)
}

#[allow(
    clippy::too_many_lines,
    reason = "the command-line grammar stays intentionally explicit and dependency-free"
)]
fn options() -> Result<Options, Box<dyn std::error::Error>> {
    let mut profile = "smoke".to_owned();
    let mut output = PathBuf::from("benchmarks/.work/smoke.json");
    let mut manifests = Vec::new();
    let mut cache_root = PathBuf::from("benchmarks/.work/corpus");
    let mut origin = "frozen".to_owned();
    let mut max_samples = None;
    let mut timeout_seconds = None;
    let mut rss_limit_bytes = None;
    let mut selected_cases = Vec::new();
    let mut baseline = None;
    let mut wall_time_percent: f64 = 50.0;
    let mut peak_rss_percent: f64 = 20.0;
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
                    "Usage: tq-bench run --profile smoke|standard|large --output PATH [--manifest PATH --cache-root PATH --origin refreshed|frozen] [--max-samples N] [--timeout-seconds N] [--rss-limit-bytes N] [--case ID] [--baseline PATH --wall-regression-percent N --rss-regression-percent N --minimum-regression-samples N]"
                );
                std::process::exit(0);
            }
            value => return Err(format!("unknown argument: {value}").into()),
        }
    }
    if !matches!(profile.as_str(), "smoke" | "standard" | "large") {
        return Err(format!("invalid profile: {profile}").into());
    }
    if profile != "smoke" && manifests.is_empty() {
        if let Some(paths) = env::var_os("TQ_BENCH_MANIFESTS") {
            manifests.extend(env::split_paths(&paths));
        } else {
            let directory = cache_root.join("manifests");
            if directory.is_dir() {
                manifests.extend(
                    fs::read_dir(directory)?
                        .filter_map(Result::ok)
                        .map(|entry| entry.path())
                        .filter(|path| path.extension().is_some_and(|value| value == "json")),
                );
                manifests.sort();
            }
        }
    }
    if profile != "smoke" && manifests.is_empty() {
        return Err("standard/large campaigns require prepared snapshot manifests via --manifest, TQ_BENCH_MANIFESTS, or CACHE_ROOT/manifests".into());
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
    Ok(PreparedCampaign {
        _temporary: Some(temporary),
        datasets,
    })
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
    formats.insert("json", (snapshot.file.clone(), snapshot.artifact.clone()));
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
    let mut datasets = Vec::new();
    for path in &options.manifests {
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
        let tier = source_tier(&frozen.manifest.source_id);
        if (options.profile == "standard" && tier == DatasetTier::Large)
            || (options.profile == "large" && tier != DatasetTier::Large)
        {
            continue;
        }
        datasets.push(PreparedDataset {
            source_id: frozen.manifest.source_id.clone(),
            tier,
            logical_records: frozen.manifest.document.logical_records,
            manifest_sha256: hex(&Sha256::digest(&fs::read(path)?)),
            origin: options.origin.clone(),
            formats,
        });
    }
    Ok(PreparedCampaign {
        _temporary: None,
        datasets,
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

fn invocation(
    case: &tq_test_support::benchmark::BenchmarkCase,
    adapter: &tq_test_support::benchmark::BenchmarkAdapter,
    dataset: &PreparedDataset,
    identity: &ToolIdentity,
) -> Result<BenchmarkInvocation, Box<dyn std::error::Error>> {
    let path = &dataset
        .formats
        .get(format_name(adapter.input_format))
        .ok_or("missing format")?
        .0;
    let mut args = adapter.args.clone();
    args.push(adapter.query.clone().unwrap_or_else(|| case.query.clone()));
    args.push(path.display().to_string());
    Ok(BenchmarkInvocation {
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
            _ => InputFormat::Toon,
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
    match family {
        tq_test_support::benchmark::DatasetFamily::Natural => dataset.tier != DatasetTier::Startup,
        tq_test_support::benchmark::DatasetFamily::SyntheticHelper => {
            dataset.tier == DatasetTier::Startup
        }
        tq_test_support::benchmark::DatasetFamily::Usgs => {
            matches!(dataset.tier, DatasetTier::Small | DatasetTier::Medium)
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
