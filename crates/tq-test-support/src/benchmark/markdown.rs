//! Stable, user-facing benchmark result pages.

use std::{
    collections::BTreeMap,
    fmt::Write as _,
    fs, io,
    path::{Path, PathBuf},
};

use thiserror::Error;

use super::{
    BenchmarkCampaignReport, BenchmarkFinalStatus, BenchmarkOutcome, BenchmarkRow,
    ComparisonFamily, MetricSummary, RowSummary, RssProvenance,
};

/// The marker that starts the harness-owned part of a workload page.
pub const RESULTS_START_MARKER: &str = "<!-- benchmark-results:start -->";
/// The marker that ends the harness-owned part of a workload page.
pub const RESULTS_END_MARKER: &str = "<!-- benchmark-results:end -->";

/// Errors returned while updating benchmark result pages.
#[derive(Debug, Error)]
pub enum MarkdownRenderError {
    /// A page or output directory could not be read or written.
    #[error("benchmark Markdown I/O failed: {0}")]
    Io(#[from] io::Error),
    /// A report workload ID cannot be mapped to one safe Markdown filename.
    #[error("unsafe benchmark workload ID: {0}")]
    UnsafeWorkloadId(String),
    /// An authored page does not have exactly one valid results region.
    #[error("invalid benchmark results markers in {path}: {reason}")]
    InvalidMarkers {
        /// Page containing the malformed region.
        path: PathBuf,
        /// Short validation failure.
        reason: &'static str,
    },
}

/// Updates one stable Markdown page for every workload in `report`.
///
/// Pages are authored outside the harness. The renderer only replaces the
/// text between [`RESULTS_START_MARKER`] and [`RESULTS_END_MARKER`]. Every
/// page is read, validated, and rendered before any page is changed, so a
/// malformed page cannot cause an earlier page in the same campaign to be
/// clobbered.
///
/// # Errors
///
/// Returns an I/O, unsafe workload ID, or malformed-marker error.
pub fn render_markdown_pages(
    markdown_dir: &Path,
    report: &BenchmarkCampaignReport,
) -> Result<(), MarkdownRenderError> {
    let mut workloads = BTreeMap::<String, Vec<&BenchmarkRow>>::new();
    for row in &report.cases {
        workloads.entry(row.case_id.clone()).or_default().push(row);
    }

    let mut pending = Vec::with_capacity(workloads.len() + 1);
    for (case_id, rows) in workloads {
        let filename = workload_filename(&case_id)?;
        let path = markdown_dir.join(filename);
        let source = fs::read_to_string(&path)?;
        let rendered = replace_results_region(&path, &source, &render_results(report, &rows))?;
        pending.push((path, rendered));
    }

    let index_path = markdown_dir.join("index.md");
    let index = match fs::read_to_string(&index_path) {
        Ok(source) => source,
        Err(error) if error.kind() == io::ErrorKind::NotFound => format!(
            "# Benchmark comparisons\n\n## Results\n{RESULTS_START_MARKER}\n{RESULTS_END_MARKER}\n"
        ),
        Err(error) => return Err(error.into()),
    };
    pending.push((
        index_path.clone(),
        replace_results_region(&index_path, &index, &render_overview(report))?,
    ));

    // Marker validation and rendering above happen before this loop.
    for (path, contents) in pending {
        fs::write(path, contents)?;
    }
    Ok(())
}

/// Maps a stable `benchmark.*` workload ID to a safe page filename.
///
/// Only lower-case kebab IDs are accepted. This keeps filename mapping
/// one-to-one and prevents path syntax from reaching the filesystem.
///
/// # Errors
///
/// Returns [`MarkdownRenderError::UnsafeWorkloadId`] when the ID is not a
/// lower-case kebab workload ID.
pub fn workload_filename(case_id: &str) -> Result<String, MarkdownRenderError> {
    let Some(raw) = case_id.strip_prefix("benchmark.") else {
        return Err(MarkdownRenderError::UnsafeWorkloadId(case_id.to_owned()));
    };
    if raw.is_empty() || raw.starts_with('-') || raw.ends_with('-') {
        return Err(MarkdownRenderError::UnsafeWorkloadId(case_id.to_owned()));
    }

    if !raw.bytes().all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == b'-'
    }) {
        return Err(MarkdownRenderError::UnsafeWorkloadId(case_id.to_owned()));
    }
    Ok(format!("{raw}.md"))
}

fn replace_results_region(
    path: &Path,
    source: &str,
    generated: &str,
) -> Result<String, MarkdownRenderError> {
    let start_positions = all_positions(source, RESULTS_START_MARKER);
    let end_positions = all_positions(source, RESULTS_END_MARKER);
    if start_positions.len() != 1 || end_positions.len() != 1 {
        return Err(MarkdownRenderError::InvalidMarkers {
            path: path.to_owned(),
            reason: "expected exactly one start marker and one end marker",
        });
    }
    let start = start_positions[0];
    let end = end_positions[0];
    let results_heading = standalone_results_heading(source);
    if results_heading.is_none_or(|heading| heading >= start) {
        return Err(MarkdownRenderError::InvalidMarkers {
            path: path.to_owned(),
            reason: "results markers must follow a ## Results heading",
        });
    }
    if end <= start + RESULTS_START_MARKER.len() {
        return Err(MarkdownRenderError::InvalidMarkers {
            path: path.to_owned(),
            reason: "end marker must follow the start marker",
        });
    }

    let marker_end = end + RESULTS_END_MARKER.len();
    let mut output = String::with_capacity(source.len() + generated.len());
    output.push_str(&source[..start]);
    output.push_str(RESULTS_START_MARKER);
    output.push('\n');
    output.push_str(generated);
    output.push('\n');
    output.push_str(&source[end..marker_end]);
    output.push_str(&source[marker_end..]);
    Ok(output)
}

fn all_positions(source: &str, needle: &str) -> Vec<usize> {
    source
        .match_indices(needle)
        .map(|(position, _)| position)
        .collect()
}

fn standalone_results_heading(source: &str) -> Option<usize> {
    let mut offset = 0;
    for line in source.split_inclusive('\n') {
        if line.trim_end_matches(['\r', '\n']) == "## Results" {
            return Some(offset);
        }
        offset += line.len();
    }
    None
}

fn render_results(report: &BenchmarkCampaignReport, rows: &[&BenchmarkRow]) -> String {
    let mut output = String::new();
    output.push_str("Last updated: ");
    output.push_str(&last_updated(&report.campaign_id));
    output.push('\n');
    output.push_str("Profile: `");
    output.push_str(&escape_cell(&report.profile));
    output.push_str("` | Status: `");
    output.push_str(&escape_cell(status_name(report.final_status)));
    output.push_str("`\n\n");

    output.push_str("Tools: ");
    if report.tools.is_empty() {
        output.push_str("no tool versions recorded");
    } else {
        let mut tools = report
            .tools
            .iter()
            .map(|tool| {
                format!(
                    "`{}` ({})",
                    tool_name(tool.tool),
                    escape_cell(&single_line(&tool.version))
                )
            })
            .collect::<Vec<_>>();
        tools.sort();
        output.push_str(&tools.join(", "));
    }
    output.push('\n');
    output.push_str("Environment: ");
    output.push_str(&environment_summary(report));
    output.push_str("\n\n");
    output.push_str(concat!(
        "Peak RSS is authoritative only after the campaign RSS preflight passes. ",
            "Captured values show their source when present; missing values are `not captured`, never estimates.\n\n",
    ));

    output.push_str("Compare columns with the same input format to isolate tool differences. Missing adapters are marked `not recorded`.\n\n");
    output.push_str(
        "Timing rows show numeric medians followed by compact MAD / p95 / range rows. CPU, throughput, and output cells use the captured summary values.\n\n",
    );
    append_metric_tables(&mut output, rows);
    output.trim_end_matches('\n').to_owned()
}

fn append_metric_tables(output: &mut String, rows: &[&BenchmarkRow]) {
    append_sampling_note(output, rows);
    let columns = adapter_columns(rows);
    let mut datasets = BTreeMap::<&str, Vec<&BenchmarkRow>>::new();
    for row in rows {
        datasets.entry(&row.source_id).or_default().push(row);
    }
    for (dataset, rows) in datasets {
        let _ = write!(output, "### {}\n\n", escape_cell(dataset));
        table_header(output, &["Metric"], &columns);
        for metric in [
            "Wall (ms)",
            "Wall dispersion (MAD / p95 / range)",
            "First output (ms)",
            "First output dispersion (MAD / p95 / range)",
            "User CPU (ms)",
            "System CPU (ms)",
            "Peak RSS (MiB, source)",
            "Logical throughput (records/s)",
            "Physical throughput (MiB/s)",
            "Output bytes",
            "Outcome",
            "Details",
            "Samples (warmups)",
            "Comparison view",
        ] {
            table_row(output, &[metric.to_owned()], &columns, |column| {
                matching_row(&rows, column).map_or_else(
                    || "not recorded".to_owned(),
                    |row| match metric {
                        "Wall (ms)" => wall_time(row),
                        "Wall dispersion (MAD / p95 / range)" => wall_dispersion(row),
                        "First output (ms)" => first_output(row),
                        "First output dispersion (MAD / p95 / range)" => {
                            first_output_dispersion(row)
                        }
                        "User CPU (ms)" => cpu_time(row, |summary| summary.user_cpu_micros),
                        "System CPU (ms)" => cpu_time(row, |summary| summary.system_cpu_micros),
                        "Peak RSS (MiB, source)" => memory(row),
                        "Logical throughput (records/s)" => {
                            throughput(row, |summary| summary.logical_records_per_second)
                        }
                        "Physical throughput (MiB/s)" => {
                            throughput(row, |summary| summary.physical_mib_per_second)
                        }
                        "Output bytes" => output_bytes(row),
                        "Outcome" => outcome_name(&row.outcome).to_owned(),
                        "Details" => row
                            .diagnostic
                            .as_deref()
                            .unwrap_or_else(|| {
                                if row.outcome == BenchmarkOutcome::Timed {
                                    "none"
                                } else {
                                    "not recorded"
                                }
                            })
                            .to_owned(),
                        "Samples (warmups)" => sample_count(row),
                        _ => view(row).to_owned(),
                    },
                )
            });
        }
        output.push('\n');
    }
}

fn append_sampling_note(output: &mut String, rows: &[&BenchmarkRow]) {
    if rows
        .iter()
        .any(|row| row.outcome == BenchmarkOutcome::Timed)
        && rows
            .iter()
            .filter(|row| row.outcome == BenchmarkOutcome::Timed)
            .all(|row| row.samples.len() == 1)
    {
        output.push_str("Each timed row has 1 measured sample. This run verifies correctness and execution; repeated samples are needed for a stable performance comparison.\n\n");
    }
}

type AdapterColumn<'a> = (&'a str, super::InputFormat);

fn adapter_columns<'a>(rows: &[&'a BenchmarkRow]) -> Vec<AdapterColumn<'a>> {
    let mut columns = rows
        .iter()
        .map(|row| (row.adapter_id.as_str(), row.input_format))
        .collect::<Vec<_>>();
    columns.sort_by_key(|(adapter, format)| {
        let tool = match adapter_tool_name(adapter) {
            "jq" => 0,
            "yq" => 1,
            "tq" => 2,
            _ => 3,
        };
        let format = match format {
            super::InputFormat::Json => 0,
            super::InputFormat::Yaml => 1,
            super::InputFormat::Toon => 2,
        };
        (tool, format, *adapter)
    });
    columns.dedup();
    columns
}

fn matching_row<'a>(
    rows: &[&'a BenchmarkRow],
    column: &AdapterColumn<'_>,
) -> Option<&'a BenchmarkRow> {
    rows.iter()
        .copied()
        .find(|row| row.adapter_id == column.0 && row.input_format == column.1)
}

fn table_header(output: &mut String, labels: &[&str], columns: &[AdapterColumn<'_>]) {
    output.push_str("| ");
    output.push_str(&labels.join(" | "));
    for (adapter, format) in columns {
        let _ = write!(
            output,
            " | {} {}",
            escape_cell(adapter_tool_name(adapter)),
            format_name(*format).to_uppercase()
        );
    }
    output.push_str(" |\n|");
    for _ in labels {
        output.push_str(" --- |");
    }
    for _ in columns {
        output.push_str(" ---: |");
    }
    output.push('\n');
}

fn table_row(
    output: &mut String,
    labels: &[String],
    columns: &[AdapterColumn<'_>],
    cell: impl Fn(&AdapterColumn<'_>) -> String,
) {
    output.push_str("| ");
    output.push_str(
        &labels
            .iter()
            .map(|label| escape_cell(label))
            .collect::<Vec<_>>()
            .join(" | "),
    );
    for column in columns {
        output.push_str(" | ");
        output.push_str(&escape_cell(&cell(column)));
    }
    output.push_str(" |\n");
}

fn render_overview(report: &BenchmarkCampaignReport) -> String {
    let rows = report.cases.iter().collect::<Vec<_>>();
    let columns = adapter_columns(&rows);
    let mut output = format!(
        "Last updated: {}\nProfile: `{}` | Status: `{}`\n\n{} adapter observations across {} workloads. Only correctness-checked outputs are timed; failed rows cannot support a speed ranking.\n\n### Campaign coverage\n\n",
        last_updated(&report.campaign_id),
        escape_cell(&report.profile),
        status_name(report.final_status),
        rows.len(),
        rows.iter()
            .map(|row| &row.case_id)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    );
    append_sampling_note(&mut output, &rows);
    table_header(&mut output, &["Outcome"], &columns);
    for outcome in [
        BenchmarkOutcome::Timed,
        BenchmarkOutcome::Incorrect,
        BenchmarkOutcome::Unsupported,
        BenchmarkOutcome::Timeout,
        BenchmarkOutcome::ResourceLimit,
        BenchmarkOutcome::OomOrSignal,
    ] {
        table_row(
            &mut output,
            &[outcome_name(&outcome).to_owned()],
            &columns,
            |column| {
                rows.iter()
                    .filter(|row| {
                        row.adapter_id == column.0
                            && row.input_format == column.1
                            && row.outcome == outcome
                    })
                    .count()
                    .to_string()
            },
        );
    }
    let largest = report
        .corpus
        .iter()
        .filter(|corpus| {
            corpus.format == super::InputFormat::Json
                && rows.iter().any(|row| row.source_id == corpus.source_id)
        })
        .max_by_key(|corpus| corpus.artifact.bytes);
    if let Some(dataset) = largest {
        let _ = write!(
            output,
            "\n### {}\n\nLargest recorded JSON input: {} bytes, {} logical records. Each row compares the same workload across tools. Wall time is in milliseconds and captured peak RSS is in MiB. Lower is better. Missing RSS is not captured; compare matching formats and check each workload page for outcomes and sample counts.\n\n",
            escape_cell(&dataset.source_id),
            dataset.artifact.bytes,
            dataset.logical_records
        );
        table_header(&mut output, &["Workload", "Metric"], &columns);
        let mut workloads = BTreeMap::<&str, Vec<&BenchmarkRow>>::new();
        for row in rows.iter().filter(|row| row.source_id == dataset.source_id) {
            workloads.entry(&row.case_id).or_default().push(row);
        }
        for (workload, rows) in workloads {
            for metric in ["Wall (ms)", "Peak RSS (MiB, source)"] {
                table_row(
                    &mut output,
                    &[
                        workload
                            .strip_prefix("benchmark.")
                            .unwrap_or(workload)
                            .to_owned(),
                        metric.to_owned(),
                    ],
                    &columns,
                    |column| {
                        matching_row(&rows, column).map_or_else(
                            || "not recorded".to_owned(),
                            |row| {
                                if row.outcome != BenchmarkOutcome::Timed {
                                    outcome_name(&row.outcome).to_owned()
                                } else if metric == "Wall (ms)" {
                                    wall_time(row)
                                } else {
                                    memory(row)
                                }
                            },
                        )
                    },
                );
            }
        }
    }
    output.trim_end().to_owned()
}

fn environment_summary(report: &BenchmarkCampaignReport) -> String {
    let environment = &report.environment;
    let cpus = environment
        .logical_cpus
        .map_or_else(|| "unknown".to_owned(), |count| count.to_string());
    format!(
        "`{}` / `{}`, {cpus} logical CPUs, compiler profile `{}`",
        escape_cell(&single_line(&environment.os)),
        escape_cell(&single_line(&environment.architecture)),
        escape_cell(&single_line(&environment.compiler_profile)),
    )
}

fn last_updated(campaign_id: &str) -> String {
    if campaign_id.parse::<jiff::Timestamp>().is_ok()
        && campaign_id.len() >= 10
        && campaign_id.as_bytes()[4] == b'-'
        && campaign_id.as_bytes()[7] == b'-'
    {
        escape_cell(&campaign_id[..10])
    } else {
        "unavailable (campaign ID is not a timestamp)".to_owned()
    }
}

fn view(row: &BenchmarkRow) -> &'static str {
    if row
        .comparison_families
        .contains(&ComparisonFamily::SameFormat)
    {
        "same input"
    } else if row
        .comparison_families
        .contains(&ComparisonFamily::NativeFormat)
    {
        "native input"
    } else if row
        .comparison_families
        .contains(&ComparisonFamily::ParserSpecific)
    {
        "parser-specific"
    } else {
        "unclassified"
    }
}

fn wall_time(row: &BenchmarkRow) -> String {
    timed_summary(row).map_or_else(
        || "not measured".to_owned(),
        |summary| format_millis(summary.wall_time_micros.median),
    )
}

fn wall_dispersion(row: &BenchmarkRow) -> String {
    timed_summary(row).map_or_else(
        || "not measured".to_owned(),
        |summary| format_dispersion(&summary.wall_time_micros, format_millis),
    )
}

fn first_output(row: &BenchmarkRow) -> String {
    let Some(summary) = timed_summary(row) else {
        return "not measured".to_owned();
    };
    summary.first_result_micros.as_ref().map_or_else(
        || "no output".to_owned(),
        |metric| format_millis(metric.median),
    )
}

fn first_output_dispersion(row: &BenchmarkRow) -> String {
    let Some(summary) = timed_summary(row) else {
        return "not measured".to_owned();
    };
    summary.first_result_micros.as_ref().map_or_else(
        || "no output".to_owned(),
        |metric| format_dispersion(metric, format_millis),
    )
}

#[allow(
    clippy::cast_precision_loss,
    reason = "memory is rendered as a human-readable MiB value"
)]
fn memory(row: &BenchmarkRow) -> String {
    let Some(summary) = timed_summary(row) else {
        return "not measured".to_owned();
    };
    summary.peak_rss_bytes.map_or_else(
        || "not captured".to_owned(),
        |bytes| format!("{} ({})", format_mib(bytes as f64), rss_source(row)),
    )
}

fn rss_source(row: &BenchmarkRow) -> &str {
    let mut source = None;
    for sample in &row.samples {
        if sample.peak_rss_bytes.is_none() {
            return "RSS incomplete";
        }
        let Some(provenance) = sample.rss_provenance else {
            return "provenance missing";
        };
        if source.is_some_and(|known| known != provenance) {
            return "mixed sources";
        }
        source = Some(provenance);
    }
    source.map_or("provenance missing", RssProvenance::label)
}

fn cpu_time(row: &BenchmarkRow, value: impl Fn(&RowSummary) -> Option<f64>) -> String {
    timed_summary(row)
        .and_then(value)
        .map_or_else(|| "not captured".to_owned(), format_millis)
}

fn throughput(row: &BenchmarkRow, value: impl Fn(&RowSummary) -> f64) -> String {
    timed_summary(row).map_or_else(
        || "not measured".to_owned(),
        |summary| format!("{:.3}", value(summary)),
    )
}

fn output_bytes(row: &BenchmarkRow) -> String {
    timed_summary(row).map_or_else(
        || "not measured".to_owned(),
        |summary| summary.output_bytes.to_string(),
    )
}

fn timed_summary(row: &BenchmarkRow) -> Option<&RowSummary> {
    (row.outcome == BenchmarkOutcome::Timed)
        .then_some(row.summary.as_ref())
        .flatten()
}

fn format_dispersion(metric: &MetricSummary, format: fn(f64) -> String) -> String {
    format!(
        "{} / {} / {}-{}",
        format(metric.median_absolute_deviation),
        format(metric.p95),
        format(metric.minimum),
        format(metric.maximum),
    )
}

fn sample_count(row: &BenchmarkRow) -> String {
    let Some(summary) = timed_summary(row) else {
        return "not timed".to_owned();
    };
    format!(
        "{} measured ({} warmup)",
        summary.wall_time_micros.samples, row.warmups
    )
}

fn format_millis(micros: f64) -> String {
    format!("{:.3}", micros / 1_000.0)
}

fn format_mib(bytes: f64) -> String {
    format!("{:.2}", bytes / (1024.0 * 1024.0))
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

fn status_name(status: BenchmarkFinalStatus) -> &'static str {
    match status {
        BenchmarkFinalStatus::Passed => "passed",
        BenchmarkFinalStatus::ObservedFailures => "observed-failures",
        BenchmarkFinalStatus::Regression => "regression",
    }
}

fn adapter_tool_name(adapter_id: &str) -> &str {
    adapter_id
        .split_once('-')
        .map_or(adapter_id, |(tool, _)| tool)
}

fn tool_name(tool: super::super::compatibility::ToolKind) -> &'static str {
    match tool {
        super::super::compatibility::ToolKind::Jq => "jq",
        super::super::compatibility::ToolKind::Yq => "yq",
        super::super::compatibility::ToolKind::Tq => "tq",
    }
}

fn format_name(format: super::InputFormat) -> &'static str {
    match format {
        super::InputFormat::Json => "json",
        super::InputFormat::Yaml => "yaml",
        super::InputFormat::Toon => "toon",
    }
}

fn single_line(value: &str) -> String {
    value.replace(['\r', '\n'], " ")
}

fn escape_cell(value: &str) -> String {
    single_line(value).replace('\\', "\\\\").replace('|', "\\|")
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, fs};

    use super::super::{
        BenchmarkCampaignReport, BenchmarkFinalStatus, BenchmarkLimits, BenchmarkOutcome,
        BenchmarkRow, BenchmarkSample, Comparability, ComparisonFamily, ExecutionClass,
        InputFormat, RegressionGate, RssProvenance, summarize_samples,
    };
    use super::{
        RESULTS_END_MARKER, RESULTS_START_MARKER, render_markdown_pages, workload_filename,
    };

    #[test]
    fn workload_filenames_are_stable_and_safe() {
        assert_eq!(
            workload_filename("benchmark.blocking-sort").unwrap(),
            "blocking-sort.md"
        );
        assert!(workload_filename("benchmark.issue5-input_sequence").is_err());
        assert!(workload_filename("benchmark../escape").is_err());
        assert!(workload_filename("benchmark.foo/bar").is_err());
        assert!(workload_filename("benchmark.Foo").is_err());
        assert!(workload_filename("other.foo").is_err());
    }

    #[test]
    fn rendering_preserves_authored_text_and_is_idempotent() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let page = directory.path().join("blocking-sort.md");
        fs::write(
            &page,
            format!(
                "# Blocking sort\n\nAuthored introduction.\n\n## Results\n{RESULTS_START_MARKER}\nold generated text\n{RESULTS_END_MARKER}\n\nAuthored trailing note.\n"
            ),
        )
        .expect("authored page");
        let report = report(vec![row(
            "benchmark.blocking-sort",
            BenchmarkOutcome::Timed,
        )]);

        render_markdown_pages(directory.path(), &report).expect("render page");
        let first = fs::read_to_string(&page).expect("rendered page");
        assert!(first.contains("Authored introduction."));
        assert!(first.contains("Authored trailing note."));
        assert!(!first.contains("old generated text"));
        assert!(first.contains("Last updated: 2026-09-10"));
        assert!(first.contains("| Metric | tq JSON |"));
        assert!(first.contains("| Wall (ms) | 1.234 |"));
        assert!(
            first.contains("| Wall dispersion (MAD / p95 / range) | 0.000 / 1.234 / 1.234-1.234 |")
        );
        assert!(first.contains("| Outcome | timed |"));
        assert!(first.contains("Each timed row has 1 measured sample."));
        assert!(!first.contains("/private/") && !first.contains("target/release"));
        assert!(
            first.contains(
                "Peak RSS is authoritative only after the campaign RSS preflight passes."
            )
        );

        render_markdown_pages(directory.path(), &report).expect("render page again");
        let second = fs::read_to_string(&page).expect("rendered page again");
        assert_eq!(first, second);
    }

    #[test]
    fn failed_rows_keep_outcomes_and_do_not_report_zero_timings() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let page = directory.path().join("blocking-sort.md");
        fs::write(
            &page,
            format!("# Workload\n\n## Results\n{RESULTS_START_MARKER}\n{RESULTS_END_MARKER}\n"),
        )
        .expect("authored page");
        let mut failed = row("benchmark.blocking-sort", BenchmarkOutcome::Incorrect);
        failed.diagnostic = Some("ordered result sequence differs".to_owned());
        let report = report(vec![failed]);

        render_markdown_pages(directory.path(), &report).expect("render page");
        let rendered = fs::read_to_string(page).expect("rendered page");
        assert!(rendered.contains("| incorrect |"));
        assert!(rendered.contains("| Details | ordered result sequence differs |"));
        assert!(rendered.contains("| Wall (ms) | not measured |"));
        assert!(!rendered.contains("0.000"));
        assert!(rendered.contains("| Samples (warmups) | not timed |"));
    }

    #[test]
    fn renders_available_metrics_and_distinguishes_missing_rss() {
        let mut measured = row("benchmark.blocking-sort", BenchmarkOutcome::Timed);
        measured.samples = vec![BenchmarkSample {
            wall_time_micros: 1_234,
            user_cpu_micros: Some(200),
            system_cpu_micros: Some(34),
            peak_rss_bytes: Some(2 * 1024 * 1024),
            rss_provenance: Some(RssProvenance::BsdTimeL),
            process_group_peak_rss_bytes: None,
            first_result_micros: Some(567),
            output_bytes: 42,
        }];
        measured.summary = summarize_samples(&measured.samples, 100, 1);
        let mut missing_rss = measured.clone();
        missing_rss.adapter_id = "jq-json".to_owned();
        missing_rss.samples[0].peak_rss_bytes = None;
        missing_rss.summary = summarize_samples(&missing_rss.samples, 100, 1);
        let campaign = report(vec![measured, missing_rss]);

        let rendered = super::render_results(&campaign, &campaign.cases.iter().collect::<Vec<_>>());
        assert!(rendered.contains("| First output (ms) | 0.567 |"));
        assert!(rendered.contains(
            "| First output dispersion (MAD / p95 / range) | 0.000 / 0.567 / 0.567-0.567 |"
        ));
        assert!(rendered.contains("| User CPU (ms) | 0.200 | 0.200 |"));
        assert!(rendered.contains("| System CPU (ms) | 0.034 | 0.034 |"));
        assert!(rendered.contains("| Peak RSS (MiB, source) | not captured | 2.00 (bsd-time-l) |"));
        assert!(rendered.contains("| Output bytes | 42 | 42 |"));
        assert!(
            rendered
                .contains("| Samples (warmups) | 1 measured (1 warmup) | 1 measured (1 warmup) |")
        );
        assert!(!rendered.contains("sampled/unverified"));
        assert!(!rendered.contains("unverified"));
    }

    #[test]
    fn malformed_page_is_rejected_before_another_page_is_changed() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let good = directory.path().join("a.md");
        let bad = directory.path().join("b.md");
        let good_source =
            format!("# A\n\n## Results\n{RESULTS_START_MARKER}\nold\n{RESULTS_END_MARKER}\n");
        fs::write(&good, &good_source).expect("good page");
        fs::write(&bad, "# B\n\n## Results\nmissing markers\n").expect("bad page");
        let report = report(vec![
            row("benchmark.a", BenchmarkOutcome::Timed),
            row("benchmark.b", BenchmarkOutcome::Timed),
        ]);

        assert!(render_markdown_pages(directory.path(), &report).is_err());
        assert_eq!(fs::read_to_string(good).expect("good page"), good_source);
    }

    #[test]
    fn table_cells_escape_pipes_and_newlines() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let page = directory.path().join("blocking-sort.md");
        fs::write(
            &page,
            format!("# Workload\n\n## Results\n{RESULTS_START_MARKER}\n{RESULTS_END_MARKER}\n"),
        )
        .expect("authored page");
        let mut escaped = row("benchmark.blocking-sort", BenchmarkOutcome::Timed);
        escaped.source_id = "dataset|row\nnext".to_owned();
        escaped.adapter_id = "tq|adapter".to_owned();
        let report = report(vec![escaped]);

        render_markdown_pages(directory.path(), &report).expect("render page");
        let rendered = fs::read_to_string(page).expect("rendered page");
        assert!(rendered.contains("dataset\\|row next"));
        assert!(rendered.contains("tq\\|adapter"));
    }

    #[test]
    fn dataset_tables_align_adapters_and_keep_missing_and_failed_results_distinct() {
        let mut tq = row("benchmark.blocking-sort", BenchmarkOutcome::Timed);
        tq.source_id = "small".to_owned();
        let mut jq = tq.clone();
        jq.adapter_id = "jq-json".to_owned();
        jq.summary.as_mut().unwrap().wall_time_micros.median = 9_000.0;
        let mut failed = tq.clone();
        failed.source_id = "large".to_owned();
        failed.outcome = BenchmarkOutcome::Incorrect;
        let campaign = report(vec![tq, failed, jq]);
        let rows = campaign.cases.iter().collect::<Vec<_>>();
        let rendered = super::render_results(&campaign, &rows);
        assert_eq!(
            rendered.matches("| Metric | jq JSON | tq JSON |").count(),
            2
        );
        let (large, small) = rendered.split_once("### small").unwrap();
        assert!(large.contains("| Outcome | not recorded | incorrect |"));
        assert!(large.contains("| Wall (ms) | not recorded | not measured |"));
        assert!(small.contains("| Wall (ms) | 9.000 | 1.234 |"));
        assert!(small.contains("| Wall dispersion (MAD / p95 / range) | 0.000 / 1.234 / 1.234-1.234 | 0.000 / 1.234 / 1.234-1.234 |"));
        assert!(small.contains("| Outcome | timed | timed |"));
        let overview = super::render_overview(&campaign);
        assert!(overview.contains("| timed | 1 | 1 |"));
        assert!(overview.contains("| incorrect | 0 | 1 |"));
    }

    #[test]
    fn overview_uses_largest_json_dataset_without_averaging_or_ranking_failures() {
        let mut timed = row("benchmark.blocking-sort", BenchmarkOutcome::Timed);
        timed.source_id = "large".to_owned();
        let mut failed = timed.clone();
        failed.adapter_id = "jq-json".to_owned();
        failed.outcome = BenchmarkOutcome::Incorrect;
        let mut campaign = report(vec![timed, failed]);
        campaign.corpus = serde_json::from_value(serde_json::json!([
            {"origin":"frozen", "source_id":"large", "tier":"medium", "format":"json",
             "artifact":{"path":"input.json", "bytes":1000, "sha256":"digest"},
             "logical_records":10, "manifest_sha256":"manifest"}
        ]))
        .unwrap();
        let overview = super::render_overview(&campaign);
        assert!(overview.contains("### large"));
        assert!(overview.contains("| Workload | Metric | jq JSON | tq JSON |"));
        assert!(overview.contains("| blocking-sort | Wall (ms) | incorrect | 1.234 |"));
        assert!(overview.contains(
            "| blocking-sort | Peak RSS (MiB, source) | incorrect | 2.00 (bsd-time-l) |"
        ));
    }

    fn report(cases: Vec<BenchmarkRow>) -> BenchmarkCampaignReport {
        BenchmarkCampaignReport {
            schema_version: 1,
            campaign_id: "2026-09-10T04:26:43.169253Z".to_owned(),
            profile: "standard".to_owned(),
            environment: crate::benchmark::collect_environment("release-benchmark"),
            corpus: Vec::new(),
            tools: Vec::new(),
            cases,
            comparability: Comparability::default(),
            regression_gate: RegressionGate::default(),
            final_status: BenchmarkFinalStatus::Passed,
        }
    }

    fn row(case_id: &str, outcome: BenchmarkOutcome) -> BenchmarkRow {
        let samples = vec![BenchmarkSample {
            wall_time_micros: 1_234,
            user_cpu_micros: None,
            system_cpu_micros: None,
            peak_rss_bytes: Some(2 * 1024 * 1024),
            rss_provenance: Some(RssProvenance::BsdTimeL),
            process_group_peak_rss_bytes: None,
            first_result_micros: None,
            output_bytes: 0,
        }];
        BenchmarkRow {
            case_id: case_id.to_owned(),
            adapter_id: "tq-json".to_owned(),
            source_id: "dataset".to_owned(),
            tier: "small".to_owned(),
            input_format: InputFormat::Json,
            execution_class: ExecutionClass::Document,
            comparison_families: vec![ComparisonFamily::SameFormat],
            command: vec!["/private/tmp/tq".to_owned()],
            outcome,
            diagnostic: None,
            warmups: 1,
            requested_samples: 3,
            timeout_seconds: 10,
            limits: BenchmarkLimits {
                output_bytes: 1024,
                rss_bytes: None,
            },
            samples: samples.clone(),
            summary: summarize_samples(&samples, 100, 1),
            reference_ratios: BTreeMap::new(),
            reference_peak_rss_ratios: BTreeMap::new(),
            soft_performance_objective: None,
        }
    }
}
