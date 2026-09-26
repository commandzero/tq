//! Markdown pages for the Stack Overflow benchmark campaign.

use std::{
    collections::BTreeSet,
    fmt::Write as _,
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use thiserror::Error;
use tq_test_support::benchmark::{
    BenchmarkCampaignReport, BenchmarkOutcome, BenchmarkRow, BenchmarkTool, RssProvenance,
};
use tq_test_support::compatibility::ToolKind;

pub const RESULTS_START_MARKER: &str = "<!-- STACK_OVERFLOW_RESULTS_START -->";
pub const RESULTS_END_MARKER: &str = "<!-- STACK_OVERFLOW_RESULTS_END -->";

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputMode {
    #[default]
    Structured,
    Raw,
    Color,
}

#[derive(Debug, Deserialize)]
pub struct Scenario {
    pub id: String,
    pub rank: u32,
    pub source: Source,
    #[serde(default)]
    pub answer: Option<Answer>,
    pub benchmark: BenchmarkInput,
}

#[derive(Debug, Deserialize)]
pub struct Source {
    pub title: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub score: Option<i64>,
    #[serde(default)]
    pub retrieved_from: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Answer {
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BenchmarkInput {
    #[serde(default)]
    pub output_mode: OutputMode,
    pub query: String,
    #[serde(default)]
    pub yq: Option<QueryOverride>,
    pub input: Value,
}

#[derive(Debug, Deserialize)]
pub struct QueryOverride {
    pub query: String,
    pub note: String,
}

impl BenchmarkInput {
    pub fn query_for(&self, tool: BenchmarkTool) -> &str {
        if tool == BenchmarkTool::Yq {
            self.yq.as_ref().map_or(&self.query, |yq| &yq.query)
        } else {
            &self.query
        }
    }
}

#[derive(Debug)]
pub struct ScenarioRecord {
    pub scenario: Scenario,
    pub path: PathBuf,
    pub input: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum ReportError {
    #[error("Stack Overflow report I/O failed: {0}")]
    Io(#[from] io::Error),
    #[error("invalid Stack Overflow report: {0}")]
    Invalid(String),
}

pub fn load_scenarios(directory: &Path) -> Result<Vec<ScenarioRecord>, ReportError> {
    let mut paths = fs::read_dir(directory)?
        .map(|entry| entry.map(|value| value.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| {
        path.extension()
            .is_some_and(|extension| extension == "toon")
    });
    paths.sort();

    let mut records = Vec::with_capacity(paths.len());
    for path in paths {
        let scenario: Scenario = tq_test_support::fixture_data::read(&path).map_err(|error| {
            ReportError::Invalid(format!("{} could not be read: {error}", path.display()))
        })?;
        let input = serde_json::to_vec(&scenario.benchmark.input)
            .map_err(|error| ReportError::Invalid(format!("{} input: {error}", path.display())))?;
        records.push(ScenarioRecord {
            scenario,
            path,
            input,
        });
    }
    records.sort_by_key(|record| record.scenario.rank);
    for (index, record) in records.iter().enumerate() {
        let expected = u32::try_from(index + 1)
            .map_err(|_| ReportError::Invalid("scenario rank overflow".to_owned()))?;
        if record.scenario.rank != expected {
            return Err(ReportError::Invalid(format!(
                "scenario {} has rank {}, expected {}",
                record.scenario.id, record.scenario.rank, expected
            )));
        }
    }
    Ok(records)
}

#[allow(
    clippy::too_many_lines,
    reason = "report validation keeps corpus, row, and provenance checks in one transaction"
)]
pub fn validate_report(
    report: &BenchmarkCampaignReport,
    scenarios: &[ScenarioRecord],
) -> Result<(), ReportError> {
    if report.suite != "stack-overflow" {
        return Err(ReportError::Invalid(format!(
            "expected suite `stack-overflow`, found `{}`",
            report.suite
        )));
    }
    if !matches!(report.profile.as_str(), "quick" | "standard" | "extended") {
        return Err(ReportError::Invalid(format!(
            "expected profile `quick`, `standard`, or `extended`, found `{}`",
            report.profile
        )));
    }

    let expected = scenarios
        .iter()
        .map(|record| record.scenario.id.as_str())
        .collect::<BTreeSet<_>>();
    if expected.len() != scenarios.len() {
        return Err(ReportError::Invalid(
            "checked-in scenarios do not have unique IDs".to_owned(),
        ));
    }
    let adapters = ["jq-json", "yq-json", "tq-json"];
    let expected_rows = expected.len() * adapters.len();
    let incomplete = report.profile == "quick"
        && report.final_status == tq_test_support::benchmark::BenchmarkFinalStatus::Incomplete
        && report
            .execution
            .as_ref()
            .is_some_and(|execution| !execution.complete);
    if report.cases.len() != expected_rows && !incomplete {
        return Err(ReportError::Invalid(format!(
            "expected {expected_rows} rows for {} scenarios, found {}",
            expected.len(),
            report.cases.len()
        )));
    }

    let mut seen = BTreeSet::new();
    for row in &report.cases {
        if !expected.contains(row.case_id.as_str()) {
            return Err(ReportError::Invalid(format!(
                "report contains unknown scenario `{}`",
                row.case_id
            )));
        }
        if !adapters.contains(&row.adapter_id.as_str()) {
            return Err(ReportError::Invalid(format!(
                "scenario {} contains unknown adapter `{}`",
                row.case_id, row.adapter_id
            )));
        }
        if row.source_id != row.case_id {
            return Err(ReportError::Invalid(format!(
                "scenario {} is attached to source `{}`",
                row.case_id, row.source_id
            )));
        }
        if row.input_format != tq_test_support::benchmark::InputFormat::Json {
            return Err(ReportError::Invalid(format!(
                "scenario {} adapter {} is not a JSON-input observation",
                row.case_id, row.adapter_id
            )));
        }
        let scenario = scenarios
            .iter()
            .find(|record| record.scenario.id == row.case_id)
            .expect("validated report case ID is present in scenarios");
        let tool = adapter_tool(&row.adapter_id).expect("validated adapter ID has a tool");
        let query = scenario.scenario.benchmark.query_for(tool);
        if row.command.last().map(String::as_str) != Some(query) {
            return Err(ReportError::Invalid(format!(
                "scenario {} adapter {} has a command/query mismatch",
                row.case_id, row.adapter_id
            )));
        }
        let expected_args =
            super::outputs::profile_args(scenario.scenario.benchmark.output_mode, tool, query);
        let legacy_structured_toon = scenario.scenario.benchmark.output_mode
            == OutputMode::Structured
            && tool == BenchmarkTool::Tq
            && row_command_matches(
                report,
                row,
                tool,
                &super::outputs::legacy_toon_args(&scenario.scenario.benchmark.query),
            );
        if !row_command_matches(report, row, tool, &expected_args) && !legacy_structured_toon {
            return Err(ReportError::Invalid(format!(
                "scenario {} adapter {} has stale command/options",
                row.case_id, row.adapter_id
            )));
        }
        let key = (row.case_id.as_str(), row.adapter_id.as_str());
        if !seen.insert(key) {
            return Err(ReportError::Invalid(format!(
                "duplicate observation for {} {}",
                row.case_id, row.adapter_id
            )));
        }
    }
    let corpus = report
        .corpus
        .iter()
        .map(|entry| entry.source_id.as_str())
        .collect::<BTreeSet<_>>();
    if (!incomplete && (report.corpus.len() != expected.len() || corpus != expected))
        || (incomplete && (!corpus.is_subset(&expected) || corpus.len() != report.corpus.len()))
    {
        return Err(ReportError::Invalid(
            "report corpus does not match the checked-in scenario set".to_owned(),
        ));
    }
    for scenario in scenarios {
        let Some(entry) = report
            .corpus
            .iter()
            .find(|entry| entry.source_id == scenario.scenario.id)
        else {
            continue;
        };
        let digest = sha256_hex(&scenario.input);
        let bytes = u64::try_from(scenario.input.len()).unwrap_or(u64::MAX);
        if entry.artifact.sha256 != digest
            || entry.artifact.bytes != bytes
            || entry.manifest_sha256 != digest
        {
            return Err(ReportError::Invalid(format!(
                "scenario {} input digest does not match its fixture",
                scenario.scenario.id
            )));
        }
    }
    Ok(())
}

/// Renders a non-publishable, session-only summary of every checked scenario.
pub fn render_quick_table(
    report: &BenchmarkCampaignReport,
    scenarios: &[ScenarioRecord],
) -> String {
    let mut output =
        String::from("| Scenario | jq (ms) | yq (ms) | tq (ms) |\n| --- | ---: | ---: | ---: |\n");
    for scenario in scenarios {
        let [jq, yq, tq] = ["jq-json", "yq-json", "tq-json"].map(|adapter| {
            report
                .cases
                .iter()
                .find(|row| row.case_id == scenario.scenario.id && row.adapter_id == adapter)
                .map_or_else(|| "not run".to_owned(), quick_cell)
        });
        let _ = writeln!(output, "| {} | {jq} | {yq} | {tq} |", scenario.scenario.id);
    }
    output
}

fn quick_cell(row: &BenchmarkRow) -> String {
    match (&row.outcome, &row.summary) {
        (BenchmarkOutcome::Timed, Some(summary)) => {
            format!("{:.3}", summary.wall_time_micros.median / 1_000.0)
        }
        _ => outcome_name(&row.outcome).to_owned(),
    }
}

/// Renders the index and one page per fixture.
///
/// The returned string is the rendered index.
#[cfg(test)]
pub fn render_report(
    report: &BenchmarkCampaignReport,
    scenarios: &[ScenarioRecord],
    report_dir: &Path,
    repository_root: &Path,
) -> Result<String, ReportError> {
    render_report_with_outputs(report, scenarios, report_dir, repository_root, None)
}

pub fn render_report_with_outputs(
    report: &BenchmarkCampaignReport,
    scenarios: &[ScenarioRecord],
    report_dir: &Path,
    repository_root: &Path,
    outputs: Option<&super::outputs::OutputCampaign>,
) -> Result<String, ReportError> {
    validate_report(report, scenarios)?;
    if report.profile == "quick" {
        return Err(ReportError::Invalid(
            "quick Stack Overflow reports are table-only and cannot be published".to_owned(),
        ));
    }
    if let Some(outputs) = outputs {
        super::outputs::validate(outputs, scenarios).map_err(ReportError::Invalid)?;
    }
    let tokenizers = super::outputs::Tokenizers::new()
        .map_err(|error| ReportError::Invalid(error.to_string()))?;
    fs::create_dir_all(report_dir)?;

    let mut pending = Vec::with_capacity(scenarios.len() + 1);
    let mut migrations = Vec::new();
    let mut page_paths = BTreeSet::from([report_dir.join("index.md")]);
    for record in scenarios {
        let path = report_dir.join(page_filename(record));
        if !page_paths.insert(path.clone()) {
            return Err(ReportError::Invalid(format!(
                "duplicate report path: {}",
                path.display()
            )));
        }
        let legacy = report_dir.join(
            record
                .path
                .with_extension("md")
                .file_name()
                .ok_or_else(|| ReportError::Invalid("scenario path has no filename".to_owned()))?,
        );
        let source_path = if legacy != path && legacy.try_exists()? {
            if path.try_exists()? {
                return Err(ReportError::Invalid(format!(
                    "both legacy and normalized report pages exist; reconcile {} and {} before rendering",
                    legacy.display(),
                    path.display()
                )));
            }
            migrations.push((legacy.clone(), path.clone()));
            legacy
        } else {
            path.clone()
        };
        let source = match fs::read_to_string(&source_path) {
            Ok(source) => source,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                scenario_preamble(record, repository_root)
            }
            Err(error) => return Err(error.into()),
        };
        let source = if source.starts_with("---\n") {
            source
        } else {
            format!(
                "---\ntype: Report\ntitle: {}\ndescription: \"Benchmark reproduction and measured results for Stack Overflow scenario {:02}.\"\ngenerated: {{ by: tq-stack-overflow, at: {} }}\n---\n\n{source}",
                serde_json::to_string(&decode_html_entities(&record.scenario.source.title))
                    .expect("title serializes"),
                record.scenario.rank,
                report.environment.collected_at,
            )
        };
        let mut generated = render_scenario_results(
            report,
            record.scenario.id.as_str(),
            record.scenario.benchmark.output_mode,
        );
        generated.push_str(&super::outputs::render_case(
            outputs,
            &record.scenario.id,
            &tokenizers,
        ));
        let rendered = replace_results_region(&path, &source, &generated)?;
        pending.push((path, rendered));
    }

    let index_path = report_dir.join("index.md");
    let mut source = match fs::read_to_string(&index_path) {
        Ok(source) => source,
        Err(error) if error.kind() == io::ErrorKind::NotFound => index_preamble(scenarios),
        Err(error) => return Err(error.into()),
    };
    for (legacy, normalized) in &migrations {
        let old = legacy.file_name().expect("page filename").to_string_lossy();
        let new = normalized
            .file_name()
            .expect("page filename")
            .to_string_lossy();
        source = source.replace(&format!("(./{old})"), &format!("(./{new})"));
        source = source.replace(&format!("({old})"), &format!("({new})"));
    }
    let mut generated = render_index_results(report, scenarios);
    generated.push_str(&super::outputs::render_summary(outputs));
    let index = replace_results_region(&index_path, &source, &generated)?;
    pending.push((index_path, index.clone()));

    for (path, contents) in pending {
        fs::write(path, contents)?;
    }
    for (legacy, _) in migrations {
        fs::remove_file(legacy)?;
    }
    Ok(index)
}

fn page_filename(record: &ScenarioRecord) -> String {
    record.path.file_stem().map_or_else(
        || format!("{:02}-scenario.md", record.scenario.rank),
        |stem| format!("{}.md", stem.to_string_lossy().trim_end_matches('-')),
    )
}

fn scenario_preamble(record: &ScenarioRecord, repository_root: &Path) -> String {
    let scenario = &record.scenario;
    let title = decode_html_entities(&scenario.source.title);
    let fixture = relative_fixture_path(&record.path, repository_root);
    let source_link = scenario.source.url.as_deref().map_or_else(
        || "Stack Overflow question".to_owned(),
        |url| format!("[Stack Overflow question]({url})"),
    );
    let answer_link = scenario
        .answer
        .as_ref()
        .and_then(|answer| answer.url.as_deref())
        .map_or_else(String::new, |url| format!(" · [selected answer]({url})"));
    let score = scenario
        .source
        .score
        .map_or_else(|| "unknown".to_owned(), |value| value.to_string());

    let mut output = format!(
        "# {:02}: {title}\n\n\
         {source_link}{answer_link}\n\n\
         Rank: {} · Question score: {score}\n\n\
         Fixture: [{fixture}]({fixture})\n\n\
         This page records the checked-in benchmark reproduction for the question. \
         The benchmark query and input can be smaller or derived from the original \
         answer examples.\n\n\
         ## Benchmark input\n\n\
         Query: `{}`\n\n\
         Output profile: `{}`\n\n\
         ```json\n{}\n```\n",
        scenario.rank,
        scenario.rank,
        scenario.benchmark.query,
        output_mode_name(scenario.benchmark.output_mode),
        serde_json::to_string_pretty(&scenario.benchmark.input)
            .expect("benchmark input is serializable"),
    );
    if let Some(yq) = &scenario.benchmark.yq {
        let _ = write!(output, "\nyq equivalent: `{}`\n\n{}\n", yq.query, yq.note);
    }
    if let Some(retrieved_from) = &scenario.source.retrieved_from {
        let _ = write!(
            output,
            "\nScenario source registry: [{retrieved_from}]({retrieved_from})\n"
        );
    }
    output
}

fn output_mode_name(mode: OutputMode) -> &'static str {
    match mode {
        OutputMode::Structured => "structured",
        OutputMode::Raw => "raw",
        OutputMode::Color => "color",
    }
}

fn index_preamble(scenarios: &[ScenarioRecord]) -> String {
    let mut output = String::from(
        "# Stack Overflow jq top 50\n\nThe 50 checked-in scenarios reproduce selected queries from the highest-voted jq questions in the saved source collection. Each page links its source question, selected answer, and fixture. Some queries simplify the original question; results cover the exact fixture query and input.\n\n## Scenarios\n\n",
    );
    for record in scenarios {
        let title = decode_html_entities(&record.scenario.source.title);
        let _ = writeln!(
            output,
            "{}. [{}](./{})",
            record.scenario.rank,
            title,
            page_filename(record)
        );
    }
    output
}

fn relative_fixture_path(path: &Path, repository_root: &Path) -> String {
    let relative = path
        .strip_prefix(repository_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    format!("../../../{relative}")
}

fn replace_results_region(
    path: &Path,
    source: &str,
    generated: &str,
) -> Result<String, ReportError> {
    let heading_positions = source
        .split_inclusive('\n')
        .scan(0, |offset, line| {
            let current = *offset;
            *offset += line.len();
            Some((current, line))
        })
        .filter_map(|(offset, line)| {
            (line.trim_end_matches(['\r', '\n']) == "## Results").then_some(offset)
        })
        .collect::<Vec<_>>();
    if heading_positions.len() > 1 {
        return Err(ReportError::Invalid(format!(
            "{} has more than one `## Results` heading",
            path.display()
        )));
    }
    let starts = all_positions(source, RESULTS_START_MARKER);
    let ends = all_positions(source, RESULTS_END_MARKER);
    match heading_positions.first().copied() {
        None if starts.is_empty() && ends.is_empty() => {
            let mut output = source.trim_end_matches(['\r', '\n']).to_owned();
            output.push_str("\n\n## Results\n");
            output.push_str(RESULTS_START_MARKER);
            output.push('\n');
            output.push_str(generated.trim_end());
            output.push('\n');
            output.push_str(RESULTS_END_MARKER);
            output.push('\n');
            Ok(output)
        }
        None => Err(ReportError::Invalid(format!(
            "{} has results markers without a `## Results` heading",
            path.display()
        ))),
        Some(_heading) if starts.len() != 1 || ends.len() != 1 => {
            Err(ReportError::Invalid(format!(
                "{} has an unmarked or malformed `## Results` section",
                path.display()
            )))
        }
        Some(heading) => {
            let start = starts[0];
            let end = ends[0];
            if start <= heading || end <= start {
                return Err(ReportError::Invalid(format!(
                    "{} has results markers before or out of order with its heading",
                    path.display()
                )));
            }
            let mut output = String::with_capacity(source.len() + generated.len());
            output.push_str(&source[..start]);
            output.push_str(RESULTS_START_MARKER);
            output.push('\n');
            output.push_str(generated.trim_end());
            output.push('\n');
            output.push_str(RESULTS_END_MARKER);
            output.push_str(&source[end + RESULTS_END_MARKER.len()..]);
            Ok(output)
        }
    }
}

fn all_positions(source: &str, needle: &str) -> Vec<usize> {
    source
        .match_indices(needle)
        .map(|(position, _)| position)
        .collect()
}

fn render_scenario_results(
    report: &BenchmarkCampaignReport,
    case_id: &str,
    output_mode: OutputMode,
) -> String {
    let rows = ["jq-json", "yq-json", "tq-json"]
        .into_iter()
        .map(|adapter| find_row(report, case_id, adapter))
        .collect::<Vec<_>>();
    let mut output = campaign_metadata(report, false);
    let _ = writeln!(
        output,
        "\nOutput profile: `{}`.\n",
        output_mode_name(output_mode)
    );
    output.push_str(
        "\n| Tool | Output | Status | Wall (ms) | Δ vs jq | Peak RSS (MiB, source) | RSS Δ vs jq | Samples |\n",
    );
    output.push_str("| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |\n");
    let jq = rows[0];
    for row in rows {
        let _ = writeln!(
            output,
            "| {} | {} | {} | {} | {} | {} | {} | {} |",
            tool_name(row),
            output_format(row, output_mode),
            outcome_name(&row.outcome),
            wall_ms(row),
            wall_delta(row, jq),
            peak_rss(row),
            rss_delta(row, jq),
            sample_count(row),
        );
    }
    if report
        .cases
        .iter()
        .any(|row| row.case_id == case_id && row.diagnostic.is_some())
    {
        output.push_str("\nDetails:\n\n");
    }
    for row in [
        jq,
        find_row(report, case_id, "yq-json"),
        find_row(report, case_id, "tq-json"),
    ] {
        if let Some(diagnostic) = &row.diagnostic {
            let _ = writeln!(
                output,
                "- `{}`: {}",
                tool_name(row),
                single_line(diagnostic)
            );
        }
    }
    output.trim_end().to_owned()
}

fn render_index_results(report: &BenchmarkCampaignReport, scenarios: &[ScenarioRecord]) -> String {
    let rows = report.cases.iter().collect::<Vec<_>>();
    let mut output = campaign_metadata(report, true);
    let _ = writeln!(
        output,
        "\nCoverage: {} scenarios, {} observations.\n",
        scenarios.len(),
        rows.len()
    );
    output.push_str("| Outcome | jq | yq | tq | Total |\n| --- | ---: | ---: | ---: | ---: |\n");
    for outcome in [
        BenchmarkOutcome::Timed,
        BenchmarkOutcome::Incorrect,
        BenchmarkOutcome::Unsupported,
        BenchmarkOutcome::Timeout,
        BenchmarkOutcome::ResourceLimit,
        BenchmarkOutcome::OomOrSignal,
    ] {
        let counts = ["jq-json", "yq-json", "tq-json"]
            .into_iter()
            .map(|adapter| {
                rows.iter()
                    .filter(|row| row.adapter_id == adapter && row.outcome == outcome)
                    .count()
            })
            .collect::<Vec<_>>();
        let _ = writeln!(
            output,
            "| {} | {} | {} | {} | {} |",
            outcome_name(&outcome),
            counts[0],
            counts[1],
            counts[2],
            counts.iter().sum::<usize>(),
        );
    }

    let matched = scenarios
        .iter()
        .filter(|scenario| {
            ["jq-json", "yq-json", "tq-json"]
                .into_iter()
                .all(|adapter| {
                    let row = find_row(report, &scenario.scenario.id, adapter);
                    row.outcome == BenchmarkOutcome::Timed
                        && row.samples.len() == row.requested_samples
                        && !row.samples.is_empty()
                        && row.summary.as_ref().is_some_and(|summary| {
                            summary.peak_rss_bytes.is_some_and(|bytes| bytes > 0)
                        })
                        && row.samples.iter().all(|sample| {
                            sample.rss_provenance.is_some()
                                && sample.peak_rss_bytes.is_some_and(|bytes| bytes > 0)
                        })
                })
        })
        .collect::<Vec<_>>();
    let _ = writeln!(
        output,
        "\n### Matched successful scenario comparison\n\n\
         Aggregate values use the same {} complete, RSS-verified scenarios for all three tools. \
         They are medians of per-scenario medians and per-scenario authoritative \
         peak RSS values; incomplete or failed scenarios are excluded from every column.\n",
        matched.len()
    );
    output.push_str(
        "| Metric | jq | yq | yq Δ vs jq | tq | tq Δ vs jq |\n| --- | ---: | ---: | ---: | ---: | ---: |\n",
    );
    let aggregate_rows = [
        (
            "Median wall (ms)",
            aggregate_wall(report, &matched, "jq-json"),
            aggregate_wall(report, &matched, "yq-json"),
            aggregate_wall(report, &matched, "tq-json"),
        ),
        (
            "Median peak RSS (MiB)",
            aggregate_rss(report, &matched, "jq-json"),
            aggregate_rss(report, &matched, "yq-json"),
            aggregate_rss(report, &matched, "tq-json"),
        ),
    ];
    for (metric, jq, yq, tq) in aggregate_rows {
        let _ = writeln!(
            output,
            "| {metric} | {jq} | {yq} | {} | {tq} | {} |",
            aggregate_delta(&yq, &jq),
            aggregate_delta(&tq, &jq),
        );
    }
    output.trim_end().to_owned()
}

fn campaign_metadata(report: &BenchmarkCampaignReport, include_last_updated: bool) -> String {
    let mut output = String::new();
    if include_last_updated {
        let _ = writeln!(
            output,
            "Last updated: {}",
            report
                .environment
                .collected_at
                .split('T')
                .next()
                .unwrap_or(&report.environment.collected_at),
        );
    }
    let _ = writeln!(output, "Status: `{}`", status_name(report.final_status));
    output.push_str("Environment: ");
    output.push_str(&environment(report));
    output.push_str("  \nTools: ");
    let tools = ["jq", "yq", "tq"]
        .into_iter()
        .filter_map(|name| {
            let tool = report
                .tools
                .iter()
                .find(|tool| tool_name_kind(tool.tool) == name)?;
            Some(format!("`{name}` {}", single_line(&tool.version)))
        })
        .collect::<Vec<_>>();
    output.push_str(&tools.join("; "));
    output.push_str("  \n");
    if report_is_legacy_unverified(report) {
        output.push_str(
            "Memory evidence: **unverified legacy report**; authoritative peak RSS provenance is missing, and sampled-group RSS is not used as a substitute.  \n",
        );
    } else {
        output.push_str(
            "Memory evidence: authoritative peak RSS from the native collector; sampled-group RSS is not used as a substitute.  \n",
        );
    }
    output
}

fn environment(report: &BenchmarkCampaignReport) -> String {
    let environment = &report.environment;
    let cpu = environment
        .cpu_model
        .as_deref()
        .map_or_else(|| "unknown CPU".to_owned(), single_line);
    format!(
        "`{}` / `{}` / `{}` / {} logical CPUs",
        single_line(&environment.os),
        single_line(&environment.architecture),
        cpu,
        environment
            .logical_cpus
            .map_or_else(|| "unknown".to_owned(), |value| value.to_string()),
    )
}

fn report_is_legacy_unverified(report: &BenchmarkCampaignReport) -> bool {
    report.cases.iter().any(|row| {
        row.samples.iter().any(|sample| {
            sample.peak_rss_bytes.is_none_or(|bytes| bytes == 0) || sample.rss_provenance.is_none()
        }) || (row.outcome == BenchmarkOutcome::Timed
            && row
                .summary
                .as_ref()
                .is_some_and(|summary| summary.peak_rss_bytes.is_none()))
    })
}

fn find_row<'a>(
    report: &'a BenchmarkCampaignReport,
    case_id: &str,
    adapter: &str,
) -> &'a BenchmarkRow {
    report
        .cases
        .iter()
        .find(|row| row.case_id == case_id && row.adapter_id == adapter)
        .expect("validated Stack Overflow report has every adapter row")
}

fn tool_name(row: &BenchmarkRow) -> &'static str {
    match row.adapter_id.as_str() {
        "jq-json" => "jq",
        "yq-json" => "yq",
        "tq-json" => "tq",
        _ => "unknown",
    }
}

fn adapter_tool(adapter: &str) -> Option<BenchmarkTool> {
    match adapter {
        "jq-json" => Some(BenchmarkTool::Jq),
        "yq-json" => Some(BenchmarkTool::Yq),
        "tq-json" => Some(BenchmarkTool::Tq),
        _ => None,
    }
}

fn row_command_matches(
    report: &BenchmarkCampaignReport,
    row: &BenchmarkRow,
    tool: BenchmarkTool,
    args: &[String],
) -> bool {
    let Some(identity) = report.tools.iter().find(|identity| {
        matches!(
            (tool, identity.tool),
            (BenchmarkTool::Jq, ToolKind::Jq)
                | (BenchmarkTool::Yq, ToolKind::Yq)
                | (BenchmarkTool::Tq, ToolKind::Tq)
        )
    }) else {
        if row.outcome != BenchmarkOutcome::Unsupported {
            return false;
        }
        let mut expected = vec![
            tool_name_kind(match tool {
                BenchmarkTool::Jq => ToolKind::Jq,
                BenchmarkTool::Yq => ToolKind::Yq,
                BenchmarkTool::Tq => ToolKind::Tq,
            })
            .to_owned(),
        ];
        expected.extend(args.iter().cloned());
        return row.command == expected;
    };
    let mut expected = vec![identity.path.display().to_string()];
    expected.extend(args.iter().cloned());
    row.command == expected
}

fn tool_name_kind(tool: ToolKind) -> &'static str {
    match tool {
        ToolKind::Jq => "jq",
        ToolKind::Yq => "yq",
        ToolKind::Tq => "tq",
    }
}

fn output_format(row: &BenchmarkRow, output_mode: OutputMode) -> &'static str {
    match output_mode {
        OutputMode::Raw => "raw",
        OutputMode::Color => "colored JSON",
        OutputMode::Structured => match row.adapter_id.as_str() {
            "tq-json" => "TOON",
            _ => "JSON",
        },
    }
}

fn outcome_name(outcome: &BenchmarkOutcome) -> &'static str {
    match outcome {
        BenchmarkOutcome::Timed => "timed",
        BenchmarkOutcome::Incorrect => "incorrect",
        BenchmarkOutcome::Unsupported => "unsupported",
        BenchmarkOutcome::Timeout => "timeout",
        BenchmarkOutcome::OomOrSignal => "oom-or-signal",
        BenchmarkOutcome::ResourceLimit => "resource-limit",
    }
}

fn status_name(status: tq_test_support::benchmark::BenchmarkFinalStatus) -> &'static str {
    match status {
        tq_test_support::benchmark::BenchmarkFinalStatus::Passed => "passed",
        tq_test_support::benchmark::BenchmarkFinalStatus::ObservedFailures => "observed-failures",
        tq_test_support::benchmark::BenchmarkFinalStatus::Regression => "regression",
        tq_test_support::benchmark::BenchmarkFinalStatus::Incomplete => "incomplete",
    }
}

fn wall_ms(row: &BenchmarkRow) -> String {
    row.summary.as_ref().map_or_else(
        || "n/a".to_owned(),
        |summary| format!("{:.3}", summary.wall_time_micros.median / 1_000.0),
    )
}

#[allow(
    clippy::cast_precision_loss,
    reason = "RSS is rendered as a human-readable MiB value"
)]
fn peak_rss(row: &BenchmarkRow) -> String {
    let Some(summary) = row.summary.as_ref() else {
        return "n/a".to_owned();
    };
    let Some(bytes) = summary.peak_rss_bytes else {
        return "not captured".to_owned();
    };
    let source = rss_source(row);
    format!("{:.2} ({source})", bytes as f64 / 1024.0 / 1024.0)
}

fn rss_source(row: &BenchmarkRow) -> &'static str {
    let sources = row
        .samples
        .iter()
        .filter_map(|sample| sample.rss_provenance)
        .collect::<Vec<_>>();
    let Some(first) = sources.first().copied() else {
        return "mixed/unknown";
    };
    if sources.iter().any(|source| *source != first) {
        return "mixed/unknown";
    }
    match first {
        RssProvenance::DarwinWait4 => "darwin-wait4",
        RssProvenance::LinuxWait4 => "linux-wait4",
        RssProvenance::GnuTimeV => "gnu-time-v",
        RssProvenance::BsdTimeL => "bsd-time-l",
    }
}

fn sample_count(row: &BenchmarkRow) -> String {
    if row.outcome != BenchmarkOutcome::Timed {
        return "n/a".to_owned();
    }
    format!("{} (+{} warmup)", row.samples.len(), row.warmups)
}

fn wall_delta(candidate: &BenchmarkRow, baseline: &BenchmarkRow) -> String {
    percentage(
        candidate
            .summary
            .as_ref()
            .map(|summary| summary.wall_time_micros.median),
        baseline
            .summary
            .as_ref()
            .map(|summary| summary.wall_time_micros.median),
    )
}

#[allow(
    clippy::cast_precision_loss,
    reason = "RSS deltas are informational report values"
)]
fn rss_delta(candidate: &BenchmarkRow, baseline: &BenchmarkRow) -> String {
    percentage(
        candidate
            .summary
            .as_ref()
            .and_then(|summary| summary.peak_rss_bytes)
            .map(|bytes| bytes as f64),
        baseline
            .summary
            .as_ref()
            .and_then(|summary| summary.peak_rss_bytes)
            .map(|bytes| bytes as f64),
    )
}

fn percentage(candidate: Option<f64>, baseline: Option<f64>) -> String {
    match (candidate, baseline) {
        (Some(candidate), Some(baseline)) if baseline > 0.0 => {
            format!("{:+.1}%", (candidate / baseline - 1.0) * 100.0)
        }
        _ => "n/a".to_owned(),
    }
}

fn aggregate_wall(
    report: &BenchmarkCampaignReport,
    scenarios: &[&ScenarioRecord],
    adapter: &str,
) -> String {
    let values = scenarios
        .iter()
        .filter_map(|scenario| {
            find_row(report, &scenario.scenario.id, adapter)
                .summary
                .as_ref()
                .map(|summary| summary.wall_time_micros.median / 1_000.0)
        })
        .collect::<Vec<_>>();
    median_value(&values)
}

#[allow(
    clippy::cast_precision_loss,
    reason = "RSS aggregates are human-readable report values"
)]
fn aggregate_rss(
    report: &BenchmarkCampaignReport,
    scenarios: &[&ScenarioRecord],
    adapter: &str,
) -> String {
    let values = scenarios
        .iter()
        .filter_map(|scenario| {
            find_row(report, &scenario.scenario.id, adapter)
                .summary
                .as_ref()
                .and_then(|summary| summary.peak_rss_bytes)
                .map(|bytes| bytes as f64 / 1024.0 / 1024.0)
        })
        .collect::<Vec<_>>();
    median_value(&values)
}

fn median_value(values: &[f64]) -> String {
    if values.is_empty() {
        return "n/a".to_owned();
    }
    let mut values = values.to_vec();
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    let value = if values.len().is_multiple_of(2) {
        values[middle - 1].midpoint(values[middle])
    } else {
        values[middle]
    };
    format!("{value:.3}")
}

fn aggregate_delta(candidate: &str, baseline: &str) -> String {
    percentage(candidate.parse().ok(), baseline.parse().ok())
}

fn single_line(value: &str) -> String {
    value.replace(['\r', '\n'], " ")
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

/// Decodes the HTML entities used in the checked-in Stack Overflow titles.
pub fn decode_html_entities(value: &str) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    use tq_test_support::benchmark::{
        BenchmarkCorpusIdentity, BenchmarkFinalStatus, BenchmarkLimits, BenchmarkSample,
        BenchmarkTool, Comparability, ComparisonFamily, ExecutionClass, InputFormat, MetricSummary,
        RegressionGate, RowSummary,
    };
    use tq_test_support::compatibility::{ToolIdentity, ToolKind};
    use tq_test_support::corpus::ArtifactIdentity;

    fn scenario_fixture() -> ScenarioRecord {
        ScenarioRecord {
            scenario: Scenario {
                id: "stack-overflow.01".to_owned(),
                rank: 1,
                source: Source {
                    title: "A &amp; B &#39;title&#39;".to_owned(),
                    url: Some("https://stackoverflow.com/q/1".to_owned()),
                    score: Some(10),
                    retrieved_from: None,
                },
                answer: Some(Answer {
                    url: Some("https://stackoverflow.com/a/2".to_owned()),
                }),
                benchmark: BenchmarkInput {
                    output_mode: OutputMode::Structured,
                    query: ".name".to_owned(),
                    yq: None,
                    input: serde_json::json!({"name": "lambda"}),
                },
            },
            path: PathBuf::from("tests/stack-overflow/01-a.toon"),
            input: br#"{"name":"lambda"}"#.to_vec(),
        }
    }

    fn row(adapter: &str, outcome: BenchmarkOutcome, legacy: bool) -> BenchmarkRow {
        let timed = outcome == BenchmarkOutcome::Timed;
        BenchmarkRow {
            case_id: "stack-overflow.01".to_owned(),
            adapter_id: adapter.to_owned(),
            source_id: "stack-overflow.01".to_owned(),
            tier: "small".to_owned(),
            input_format: InputFormat::Json,
            execution_class: ExecutionClass::Document,
            comparison_families: vec![ComparisonFamily::SameFormat],
            command: {
                let tool = match adapter {
                    "jq-json" => BenchmarkTool::Jq,
                    "yq-json" => BenchmarkTool::Yq,
                    "tq-json" => BenchmarkTool::Tq,
                    _ => panic!("unknown adapter"),
                };
                let mut command = vec![format!(
                    "/tools/{}",
                    tool_name_kind(match tool {
                        BenchmarkTool::Jq => ToolKind::Jq,
                        BenchmarkTool::Yq => ToolKind::Yq,
                        BenchmarkTool::Tq => ToolKind::Tq,
                    })
                )];
                command.extend(super::super::outputs::profile_args(
                    OutputMode::Structured,
                    tool,
                    ".name",
                ));
                command
            },
            outcome,
            warmups: 1,
            requested_samples: 1,
            timeout_seconds: 10,
            limits: BenchmarkLimits {
                output_bytes: 1024,
                rss_bytes: None,
            },
            samples: timed
                .then(|| BenchmarkSample {
                    wall_time_micros: 1_000,
                    user_cpu_micros: Some(500),
                    system_cpu_micros: Some(100),
                    peak_rss_bytes: Some(1024 * 1024),
                    rss_provenance: (!legacy).then_some(RssProvenance::GnuTimeV),
                    measurement_protocol: None,
                    process_group_peak_rss_bytes: Some(2 * 1024 * 1024),
                    first_result_micros: None,
                    output_bytes: 2,
                })
                .into_iter()
                .collect(),
            instrumented_samples: Vec::new(),
            summary: timed.then_some(RowSummary {
                wall_time_micros: MetricSummary {
                    samples: 1,
                    median: 1_000.0,
                    median_absolute_deviation: 0.0,
                    p95: 1_000.0,
                    minimum: 1_000.0,
                    maximum: 1_000.0,
                },
                first_result_micros: None,
                peak_rss_bytes: Some(1024 * 1024),
                user_cpu_micros: Some(500.0),
                system_cpu_micros: Some(100.0),
                output_bytes: 2,
                logical_records_per_second: 1_000.0,
                physical_mib_per_second: 1.0,
            }),
            reference_ratios: std::collections::BTreeMap::default(),
            reference_peak_rss_ratios: std::collections::BTreeMap::default(),
            soft_performance_objective: None,
            diagnostic: (!timed).then(|| "row was not timed".to_owned()),
        }
    }

    fn report(legacy: bool, outcome: &BenchmarkOutcome) -> BenchmarkCampaignReport {
        BenchmarkCampaignReport {
            execution: None,
            schema_version: 1,
            campaign_id: "2026-09-11T00:00:00Z".to_owned(),
            suite: "stack-overflow".to_owned(),
            profile: "standard".to_owned(),
            environment: tq_test_support::benchmark::collect_environment("test"),
            corpus: vec![BenchmarkCorpusIdentity {
                origin: "checked-in".to_owned(),
                source_id: "stack-overflow.01".to_owned(),
                tier: "small".to_owned(),
                format: InputFormat::Json,
                artifact: ArtifactIdentity {
                    path: "fixture".to_owned(),
                    bytes: 17,
                    sha256: sha256_hex(br#"{"name":"lambda"}"#),
                },
                logical_records: 1,
                manifest_sha256: sha256_hex(br#"{"name":"lambda"}"#),
            }],
            tools: [
                (ToolKind::Jq, "/tools/jq"),
                (ToolKind::Yq, "/tools/yq"),
                (ToolKind::Tq, "/tools/tq"),
            ]
            .into_iter()
            .map(|(tool, path)| ToolIdentity {
                tool,
                path: PathBuf::from(path),
                version: "test".to_owned(),
                executable: ArtifactIdentity {
                    path: path.to_owned(),
                    bytes: 1,
                    sha256: "x".repeat(64),
                },
                build_features: Vec::new(),
                runtime_libraries: Vec::new(),
            })
            .collect(),
            cases: ["jq-json", "yq-json", "tq-json"]
                .into_iter()
                .map(|adapter| row(adapter, outcome.clone(), legacy))
                .collect(),
            comparability: Comparability::default(),
            regression_gate: RegressionGate::default(),
            final_status: if *outcome == BenchmarkOutcome::Timed {
                BenchmarkFinalStatus::Passed
            } else {
                BenchmarkFinalStatus::ObservedFailures
            },
        }
    }

    #[test]
    fn validation_requires_the_current_tool_specific_query() {
        let mut scenario = scenario_fixture();
        let yq_query = ".name | .";
        scenario.scenario.benchmark.yq = Some(QueryOverride {
            query: yq_query.to_owned(),
            note: "Equivalent native yq query.".to_owned(),
        });
        let scenarios = [scenario];
        let mut report = report(false, &BenchmarkOutcome::Timed);

        assert!(validate_report(&report, &scenarios).is_err());
        let yq = report
            .cases
            .iter_mut()
            .find(|row| row.adapter_id == "yq-json")
            .unwrap();
        *yq.command.last_mut().unwrap() = yq_query.to_owned();
        validate_report(&report, &scenarios)
            .expect("adapted yq query, unchanged jq and tq queries");

        let jq = report
            .cases
            .iter_mut()
            .find(|row| row.adapter_id == "jq-json")
            .unwrap();
        *jq.command.last_mut().unwrap() = yq_query.to_owned();
        assert!(validate_report(&report, &scenarios).is_err());
    }

    #[test]
    fn html_entities_are_decoded_for_page_titles() {
        assert_eq!(decode_html_entities("A &amp; &#39;x&#39;"), "A & 'x'");
    }

    #[test]
    fn saved_report_roundtrip_preserves_u128_samples_and_optional_outputs() {
        let mut benchmark = report(false, &BenchmarkOutcome::Timed);
        benchmark.cases[0].samples[0].wall_time_micros = u128::from(u64::MAX) + 1;
        let bytes = serde_json::to_vec(&benchmark).unwrap();
        let legacy = super::super::outputs::SavedReport::from_slice(&bytes).unwrap();
        assert!(legacy.output_comparison.is_none());
        let saved = super::super::outputs::SavedReport {
            benchmark,
            output_comparison: Some(super::super::outputs::OutputCampaign {
                captured_at: "2026-09-11".into(),
                tools: Vec::new(),
                cases: std::collections::BTreeMap::new(),
            }),
        };
        let restored =
            super::super::outputs::SavedReport::from_slice(&serde_json::to_vec(&saved).unwrap())
                .unwrap();
        assert_eq!(
            restored.benchmark.cases[0].samples[0].wall_time_micros,
            u128::from(u64::MAX) + 1
        );
        assert!(restored.output_comparison.is_some());
    }

    #[test]
    fn rendering_preserves_authored_text_and_writes_index_links() {
        let directory = tempdir().expect("tempdir");
        let report_dir = directory.path().join("docs/tests/stack-overflow");
        let report = report(false, &BenchmarkOutcome::Timed);
        let scenario = scenario_fixture();
        render_report(&report, &[scenario], &report_dir, directory.path()).expect("render");
        let page = report_dir.join("01-a.md");
        let original = fs::read_to_string(&page).expect("page");
        fs::write(
            &page,
            original.replace("## Results", "Authored note before results.\n\n## Results"),
        )
        .expect("update page");
        render_report(
            &report,
            &[scenario_fixture()],
            &report_dir,
            directory.path(),
        )
        .expect("rerender");
        let page = fs::read_to_string(&page).expect("page");
        assert!(page.contains("Authored note before results."));
        assert!(page.starts_with("---\ntype: Report\n"));
        assert!(page.contains("[selected answer](https://stackoverflow.com/a/2)"));
        assert!(page.contains(RESULTS_START_MARKER));
        let index = fs::read_to_string(report_dir.join("index.md")).expect("index");
        assert!(index.contains("(./01-a.md)"));
        assert!(index.contains("Matched successful scenario comparison"));
        assert!(!page.contains("Last updated:"));
        assert!(index.contains("Last updated:"));
    }

    #[test]
    fn rendering_trims_truncated_title_separators_from_document_names() {
        let directory = tempdir().expect("tempdir");
        let report_dir = directory.path().join("docs");
        let report = report(false, &BenchmarkOutcome::Timed);
        let mut scenario = scenario_fixture();
        scenario.path = directory.path().join("01-a-.toon");
        render_report(&report, &[scenario], &report_dir, directory.path()).expect("render");
        assert!(report_dir.join("01-a.md").exists());
        assert!(!report_dir.join("01-a-.md").exists());
        let index = fs::read_to_string(report_dir.join("index.md")).expect("index");
        assert!(index.contains("(./01-a.md)"));
    }

    #[test]
    fn rendering_migrates_legacy_pages_and_preserves_authored_text() {
        let directory = tempdir().unwrap();
        let report = report(false, &BenchmarkOutcome::Timed);
        let mut scenario = scenario_fixture();
        scenario.path = directory.path().join("01-a-.toon");
        let scenarios = [scenario];
        render_report(&report, &scenarios, directory.path(), directory.path()).unwrap();
        let page = directory.path().join("01-a.md");
        let legacy = directory.path().join("01-a-.md");
        fs::rename(&page, &legacy).unwrap();
        let text = fs::read_to_string(&legacy)
            .unwrap()
            .replace("## Results", "Authored note.\n\n## Results");
        fs::write(&legacy, &text).unwrap();
        let index = directory.path().join("index.md");
        fs::write(
            &index,
            fs::read_to_string(&index)
                .unwrap()
                .replace("(./01-a.md)", "(./01-a-.md)"),
        )
        .unwrap();
        render_report(&report, &scenarios, directory.path(), directory.path()).unwrap();
        assert!(!legacy.exists());
        assert!(
            fs::read_to_string(&page)
                .unwrap()
                .contains("Authored note.")
        );
        assert!(!fs::read_to_string(&index).unwrap().contains("(./01-a-.md)"));
        fs::write(&legacy, "Conflicting legacy text").unwrap();
        let before = fs::read_to_string(&page).unwrap();
        assert!(render_report(&report, &scenarios, directory.path(), directory.path()).is_err());
        assert_eq!(fs::read_to_string(&page).unwrap(), before);
        assert_eq!(
            fs::read_to_string(&legacy).unwrap(),
            "Conflicting legacy text"
        );
    }

    #[test]
    fn rendering_rejects_reserved_index_filename_without_overwriting() {
        let directory = tempdir().unwrap();
        let report = report(false, &BenchmarkOutcome::Timed);
        let mut scenario = scenario_fixture();
        scenario.path = directory.path().join("index.toon");
        let index = directory.path().join("index.md");
        fs::write(&index, "Existing index").unwrap();
        let error =
            render_report(&report, &[scenario], directory.path(), directory.path()).unwrap_err();
        assert!(error.to_string().contains("duplicate report path"));
        assert_eq!(fs::read_to_string(index).unwrap(), "Existing index");
    }

    #[test]
    fn validation_rejects_a_report_for_the_wrong_suite() {
        let mut report = report(false, &BenchmarkOutcome::Timed);
        report.suite = "natural-corpus".to_owned();
        let error = validate_report(&report, &[scenario_fixture()]).expect_err("wrong suite");
        assert!(
            error
                .to_string()
                .contains("expected suite `stack-overflow`")
        );
    }

    #[test]
    fn validation_accepts_all_run_profiles_and_rejects_non_profiles() {
        let mut report = report(false, &BenchmarkOutcome::Timed);
        for profile in ["quick", "standard", "extended"] {
            report.profile = profile.to_owned();
            validate_report(&report, &[scenario_fixture()]).expect("valid run profile");
        }
        report.profile = "stack-overflow".to_owned();
        assert!(validate_report(&report, &[scenario_fixture()]).is_err());
    }

    #[test]
    fn quick_reports_cannot_publish_pages() {
        let directory = tempdir().unwrap();
        let mut report = report(false, &BenchmarkOutcome::Timed);
        report.profile = "quick".to_owned();
        let destination = directory.path().join("pages");
        let error = render_report(
            &report,
            &[scenario_fixture()],
            &destination,
            directory.path(),
        )
        .expect_err("quick reports cannot publish");
        assert!(error.to_string().contains("quick"));
        assert!(!destination.exists());
    }

    #[test]
    fn incomplete_quick_session_preserves_partial_rows_without_implying_coverage() {
        let mut report = report(false, &BenchmarkOutcome::Timed);
        report.profile = "quick".to_owned();
        report.final_status = tq_test_support::benchmark::BenchmarkFinalStatus::Incomplete;
        report.execution = Some(tq_test_support::benchmark::CampaignExecution {
            mode: "exhaustive".to_owned(),
            sampling: "quick".to_owned(),
            instrument_rss: false,
            campaign_budget_seconds: 50,
            case_budget_seconds: 50,
            planned_rows: 3,
            elapsed_seconds: 1.0,
            complete: false,
            interruptions: vec!["campaign work budget exhausted".to_owned()],
        });
        report.cases.truncate(1);
        validate_report(&report, &[scenario_fixture()]).expect("valid partial quick session");
        let table = render_quick_table(&report, &[scenario_fixture()]);
        assert!(table.contains("| stack-overflow.01 | 1.000 | not run | not run |"));
        report.execution.as_mut().unwrap().complete = true;
        assert!(validate_report(&report, &[scenario_fixture()]).is_err());
    }

    #[test]
    fn quick_table_shows_every_tool_outcome() {
        let mut report = report(false, &BenchmarkOutcome::Timed);
        report.profile = "quick".to_owned();
        report.cases[2] = row("tq-json", BenchmarkOutcome::Unsupported, false);
        let table = render_quick_table(&report, &[scenario_fixture()]);
        assert!(table.contains("| Scenario | jq (ms) | yq (ms) | tq (ms) |"));
        assert!(table.contains("| stack-overflow.01 | 1.000 | 1.000 | unsupported |"));
    }
    #[test]
    fn changed_query_or_input_is_rejected_before_writing_pages() {
        for change_query in [true, false] {
            let mut saved = report(false, &BenchmarkOutcome::Timed);
            if change_query {
                *saved.cases[0].command.last_mut().unwrap() = ".other".to_owned();
            } else {
                saved.corpus[0].artifact.sha256 = "old input".to_owned();
            }
            let directory = tempdir().unwrap();
            let destination = directory.path().join("pages");
            assert!(
                render_report(
                    &saved,
                    &[scenario_fixture()],
                    &destination,
                    directory.path()
                )
                .is_err()
            );
            assert!(!destination.exists());
        }
    }

    #[test]
    fn validation_rejects_stale_output_profile_arguments() {
        let mut scenario = scenario_fixture();
        scenario.scenario.benchmark.output_mode = OutputMode::Color;
        let mut saved = report(false, &BenchmarkOutcome::Timed);
        for row in &mut saved.cases {
            let tool = adapter_tool(&row.adapter_id).expect("tool");
            let mut command = vec![format!(
                "/tools/{}",
                tool_name_kind(match tool {
                    BenchmarkTool::Jq => ToolKind::Jq,
                    BenchmarkTool::Yq => ToolKind::Yq,
                    BenchmarkTool::Tq => ToolKind::Tq,
                })
            )];
            command.extend(super::super::outputs::profile_args(
                OutputMode::Color,
                tool,
                ".name",
            ));
            row.command = command;
        }
        saved.cases[0].command = vec!["/tools/jq".into(), "-c".into(), ".name".into()];
        let error = validate_report(&saved, &[scenario]).expect_err("stale profile");
        assert!(error.to_string().contains("stale command/options"));
    }

    #[test]
    fn result_replacement_preserves_suffix_and_rejects_unmarked_sections() {
        let source = format!(
            "# Review\n\n## Results\n{RESULTS_START_MARKER}\nold\n{RESULTS_END_MARKER}\n\nAuthored suffix.\n"
        );
        let changed = replace_results_region(Path::new("page.md"), &source, "new").unwrap();
        assert!(changed.ends_with("\n\nAuthored suffix.\n"));
        assert!(!changed.contains("\nold\n"));
        assert!(
            replace_results_region(Path::new("page.md"), "## Results\nAuthored notes\n", "new")
                .is_err()
        );
    }

    #[test]
    fn missing_rss_excludes_the_scenario_from_all_aggregate_columns() {
        let mut saved = report(false, &BenchmarkOutcome::Timed);
        saved.cases[1].samples[0].rss_provenance = None;
        let output = render_index_results(&saved, &[scenario_fixture()]);
        assert!(output.contains("same 0 complete, RSS-verified scenarios"));
        assert!(output.contains("| Median wall (ms) | n/a | n/a | n/a | n/a | n/a |"));
    }

    #[test]
    fn legacy_rss_is_explicitly_unverified_and_never_uses_group_rss() {
        let report = report(true, &BenchmarkOutcome::Timed);
        assert!(report_is_legacy_unverified(&report));
        let row = report.cases.first().expect("row");
        assert_eq!(peak_rss(row), "1.00 (mixed/unknown)");
        let metadata = campaign_metadata(&report, true);
        assert!(metadata.contains("unverified legacy report"));
        assert!(metadata.contains("not used as a substitute"));
    }

    #[test]
    fn failure_status_and_rows_are_rendered_without_being_dropped() {
        let report = report(false, &BenchmarkOutcome::Unsupported);
        let output = render_scenario_results(&report, "stack-overflow.01", OutputMode::Structured);
        assert!(output.contains("Status: `observed-failures`"));
        assert!(output.contains("unsupported"));
        assert!(output.contains("row was not timed"));
    }
}
