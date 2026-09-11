//! Stable, user-facing benchmark result pages.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs, io,
    path::{Path, PathBuf},
};

use thiserror::Error;

use super::{
    BenchmarkCampaignReport, BenchmarkFinalStatus, BenchmarkOutcome, BenchmarkRow, BenchmarkSample,
    ComparisonFamily, MetricSummary, RowSummary,
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
    /// No host report was supplied for rendering.
    #[error("benchmark Markdown rendering requires at least one report")]
    NoReports,
    /// A native report is not ready for stable publication.
    #[error("benchmark Markdown native publication rejected: {0}")]
    InvalidNativeReport(String),
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
/// Returns an I/O, unsafe workload ID, malformed-marker, or native-publication
/// error.
pub fn render_markdown_pages(
    markdown_dir: &Path,
    report: &BenchmarkCampaignReport,
) -> Result<(), MarkdownRenderError> {
    render_markdown_campaigns(markdown_dir, std::slice::from_ref(report))
}

/// Updates stable Markdown pages from one or more independent host campaigns.
///
/// When multiple reports are supplied, every host gets a separately labelled
/// section. Rows, samples, ratios, provenance, and placeholders never cross a
/// report boundary. All pages are read and rendered before any page is changed.
///
/// # Errors
///
/// Returns an I/O, unsafe workload ID, malformed-marker, native-publication, or
/// empty-report error.
pub fn render_markdown_campaigns(
    markdown_dir: &Path,
    reports: &[BenchmarkCampaignReport],
) -> Result<(), MarkdownRenderError> {
    if reports.is_empty() {
        return Err(MarkdownRenderError::NoReports);
    }
    for report in reports {
        validate_native_publication(report)?;
    }
    if reports.len() == 1 {
        return render_markdown_pages_single(markdown_dir, &reports[0]);
    }

    let mut workload_ids = BTreeSet::new();
    for report in reports {
        workload_ids.extend(report.cases.iter().map(|row| row.case_id.as_str()));
    }

    let mut pending = Vec::with_capacity(workload_ids.len() + 1);
    for case_id in workload_ids {
        let filename = workload_filename(case_id)?;
        let path = markdown_dir.join(filename);
        let source = fs::read_to_string(&path)?;
        let rendered =
            replace_results_region(&path, &source, &render_multi_host_results(reports, case_id))?;
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
        replace_results_region(&index_path, &index, &render_multi_host_overview(reports))?,
    ));

    // Marker validation and rendering above happen before this loop.
    for (path, contents) in pending {
        fs::write(path, contents)?;
    }
    Ok(())
}

fn validate_native_publication(
    report: &BenchmarkCampaignReport,
) -> Result<(), MarkdownRenderError> {
    report
        .validate_for_publication()
        .map_err(MarkdownRenderError::InvalidNativeReport)
}

fn render_markdown_pages_single(
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

fn render_multi_host_results(reports: &[BenchmarkCampaignReport], case_id: &str) -> String {
    let mut output = String::new();
    for (index, report) in reports.iter().enumerate() {
        if index > 0 {
            output.push_str("\n---\n\n");
        }
        let rows = report
            .cases
            .iter()
            .filter(|row| row.case_id == case_id)
            .collect::<Vec<_>>();
        output.push_str("### Host: ");
        output.push_str(&host_label(report));
        output.push_str("\n\n");
        if rows.is_empty() {
            let _ = writeln!(
                output,
                "No row was recorded for `{}` on this host; all measurements are unmeasured (`-`).",
                escape_cell(case_id)
            );
        } else {
            output.push_str(&render_results(report, &rows));
        }
    }
    output.trim_end().to_owned()
}

fn render_multi_host_overview(reports: &[BenchmarkCampaignReport]) -> String {
    let mut output = String::new();
    for (index, report) in reports.iter().enumerate() {
        if index > 0 {
            output.push_str("\n---\n\n");
        }
        output.push_str("### Host: ");
        output.push_str(&host_label(report));
        output.push_str("\n\n");
        output.push_str(&render_overview(report));
    }
    output.trim_end().to_owned()
}

fn host_label(report: &BenchmarkCampaignReport) -> String {
    format!(
        "{} / {} / {}",
        escape_cell(&single_line(&report.environment.os)),
        escape_cell(&single_line(&report.environment.architecture)),
        escape_cell(&single_line(&report.profile)),
    )
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
    output.push_str("Peak RSS is authoritative only after the campaign RSS preflight passes. ");
    output
        .push_str("Missing or invalid values are `-`, never estimates. RSS collector provenance ");
    output.push_str("(outside measurement tables): ");
    output.push_str(&provenance_summary(rows));
    output.push_str(".\n");
    output.push_str("Measurement method (outside measurement tables): ");
    output.push_str(&measurement_method_summary(rows));
    output.push_str(".\n\n");

    output.push_str("Compare columns with the same input format to isolate tool differences. Missing adapters have `-` measurement cells and `not recorded` outcome/detail cells.\n\n");
    output.push_str(
        "Timing rows show numeric medians followed by compact MAD / p95 / range rows, with one decimal place and a unit in each measurement cell. CPU, throughput, and output cells use the captured summary values.\n\n",
    );
    if rows.iter().any(|row| !row.instrumented_samples.is_empty()) {
        output.push_str(
            "Primary measurements and instrumented RSS-limit repetitions are separate protocol families; instrumented samples are reported only in their own metrics and are never pooled with primary timing.\n\n",
        );
    }
    append_metric_tables(&mut output, report, rows);
    output.trim_end_matches('\n').to_owned()
}

fn append_metric_tables(
    output: &mut String,
    report: &BenchmarkCampaignReport,
    rows: &[&BenchmarkRow],
) {
    append_sampling_note(output, rows);
    let columns = adapter_columns(rows);
    let mut datasets = BTreeMap::<&str, Vec<&BenchmarkRow>>::new();
    for row in rows {
        datasets.entry(&row.source_id).or_default().push(row);
    }
    for (dataset, rows) in datasets {
        let _ = write!(output, "### {}\n\n", escape_cell(dataset));
        if table_has_measurement_placeholders(&rows, &columns) {
            append_placeholder_note(output, &rows, report);
        }
        table_header(output, &["Metric"], &columns);
        for metric in metric_names(&rows) {
            table_row(output, &[metric.to_owned()], &columns, |column| {
                matching_row(&rows, column).map_or_else(
                    || missing_metric_cell(metric).to_owned(),
                    |row| metric_cell(row, metric),
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

fn metric_names(rows: &[&BenchmarkRow]) -> Vec<&'static str> {
    let mut metrics = vec![
        "Wall time",
        "Wall time dispersion",
        "First output",
        "First output dispersion",
        "User CPU",
        "System CPU",
        "Peak RSS",
        "Logical throughput",
        "Physical throughput",
        "Output bytes",
        "Outcome",
        "Details",
        "Samples (warmups)",
        "Comparison view",
    ];
    if rows.iter().any(|row| !row.instrumented_samples.is_empty()) {
        let peak_position = metrics
            .iter()
            .position(|metric| *metric == "Peak RSS")
            .map_or(metrics.len(), |position| position + 1);
        metrics.insert(peak_position, "Instrumented peak RSS");
        let samples_position = metrics
            .iter()
            .position(|metric| *metric == "Samples (warmups)")
            .map_or(metrics.len(), |position| position + 1);
        metrics.insert(samples_position, "Instrumented sample count");
    }
    metrics
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

#[allow(
    clippy::too_many_lines,
    reason = "the overview keeps campaign coverage and largest-dataset tables together"
)]
fn render_overview(report: &BenchmarkCampaignReport) -> String {
    let rows = report.cases.iter().collect::<Vec<_>>();
    let columns = adapter_columns(&rows);
    let mut output = format!(
        "Last updated: {}\nProfile: `{}` | Status: `{}`\n\n{} adapter observations across {} workloads. Only correctness-checked outputs are timed; failed rows cannot support a speed ranking.\n\n",
        last_updated(&report.campaign_id),
        escape_cell(&report.profile),
        status_name(report.final_status),
        rows.len(),
        rows.iter()
            .map(|row| &row.case_id)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    );
    output.push_str("RSS collector provenance (outside measurement tables): ");
    output.push_str(&provenance_summary(&rows));
    output.push_str(".\nMeasurement method (outside measurement tables): ");
    output.push_str(&measurement_method_summary(&rows));
    output.push_str(".\n\n### Campaign coverage\n\n");
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
            "\n### {}\n\nLargest recorded JSON input: {} bytes, {} logical records. Each row compares the same workload across tools. Wall time cells use milliseconds and peak RSS cells use MiB, with one decimal place. Lower is better. A `-` cell means no valid comparable measurement, not zero; compare matching formats and check each workload page for outcomes and sample counts.\n\n",
            escape_cell(&dataset.source_id),
            dataset.artifact.bytes,
            dataset.logical_records
        );
        let largest_rows = rows
            .iter()
            .filter(|row| row.source_id == dataset.source_id)
            .copied()
            .collect::<Vec<_>>();
        if table_has_measurement_placeholders(&largest_rows, &columns) {
            append_placeholder_note(&mut output, &largest_rows, report);
        }
        table_header(&mut output, &["Workload", "Metric"], &columns);
        let mut workloads = BTreeMap::<&str, Vec<&BenchmarkRow>>::new();
        for row in largest_rows.iter().copied() {
            workloads.entry(&row.case_id).or_default().push(row);
        }
        for (workload, rows) in workloads {
            for metric in metric_names(&largest_rows).into_iter().filter(|metric| {
                matches!(
                    *metric,
                    "Wall time"
                        | "Peak RSS"
                        | "Instrumented peak RSS"
                        | "Instrumented sample count"
                )
            }) {
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
                        matching_row(&rows, column)
                            .map_or_else(|| "-".to_owned(), |row| metric_cell(row, metric))
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
        || "-".to_owned(),
        |summary| format_millis(summary.wall_time_micros.median),
    )
}

fn wall_dispersion(row: &BenchmarkRow) -> String {
    timed_summary(row).map_or_else(
        || "-".to_owned(),
        |summary| format_dispersion(&summary.wall_time_micros, "ms"),
    )
}

fn first_output(row: &BenchmarkRow) -> String {
    let Some(summary) = timed_summary(row) else {
        return "-".to_owned();
    };
    summary
        .first_result_micros
        .as_ref()
        .map_or_else(|| "-".to_owned(), |metric| format_millis(metric.median))
}

fn first_output_dispersion(row: &BenchmarkRow) -> String {
    let Some(summary) = timed_summary(row) else {
        return "-".to_owned();
    };
    summary
        .first_result_micros
        .as_ref()
        .map_or_else(|| "-".to_owned(), |metric| format_dispersion(metric, "ms"))
}

#[allow(
    clippy::cast_precision_loss,
    reason = "memory is rendered as a human-readable MiB value"
)]
fn memory(row: &BenchmarkRow) -> String {
    if row.samples.iter().any(|sample| {
        sample.peak_rss_bytes.is_none_or(|bytes| bytes == 0) || sample.rss_provenance.is_none()
    }) {
        return "-".to_owned();
    }
    let Some(summary) = timed_summary(row) else {
        return "-".to_owned();
    };
    summary.peak_rss_bytes.map_or_else(
        || "-".to_owned(),
        |bytes| {
            if bytes > 0 {
                format_mib(bytes as f64)
            } else {
                "-".to_owned()
            }
        },
    )
}

#[allow(
    clippy::cast_precision_loss,
    reason = "memory is rendered as a human-readable MiB value"
)]
fn instrumented_peak_rss(row: &BenchmarkRow) -> String {
    if row.outcome != BenchmarkOutcome::Timed || row.instrumented_samples.is_empty() {
        return "-".to_owned();
    }
    let Some(first) = row.instrumented_samples.first() else {
        return "-".to_owned();
    };
    if row.instrumented_samples.iter().any(|sample| {
        sample
            .process_group_peak_rss_bytes
            .is_none_or(|bytes| bytes == 0)
            || sample.rss_provenance.is_none()
            || sample.rss_provenance != first.rss_provenance
            || sample.measurement_protocol != first.measurement_protocol
    }) {
        return "-".to_owned();
    }
    row.instrumented_samples
        .iter()
        .filter_map(|sample| sample.process_group_peak_rss_bytes)
        .max()
        .map_or_else(|| "-".to_owned(), |bytes| format_mib(bytes as f64))
}

fn cpu_time(
    row: &BenchmarkRow,
    value: impl Fn(&RowSummary) -> Option<f64>,
    available: impl Fn(&BenchmarkSample) -> bool,
) -> String {
    if row.samples.iter().any(|sample| !available(sample)) {
        return "-".to_owned();
    }
    timed_summary(row)
        .and_then(value)
        .map_or_else(|| "-".to_owned(), format_millis)
}

fn throughput(row: &BenchmarkRow, value: impl Fn(&RowSummary) -> f64) -> String {
    timed_summary(row).map_or_else(
        || "-".to_owned(),
        |summary| format_rate(value(summary), "records/s"),
    )
}

fn output_bytes(row: &BenchmarkRow) -> String {
    timed_summary(row).map_or_else(
        || "-".to_owned(),
        |summary| summary.output_bytes.to_string(),
    )
}

fn timed_summary(row: &BenchmarkRow) -> Option<&RowSummary> {
    if row.outcome != BenchmarkOutcome::Timed {
        return None;
    }
    let summary = row.summary.as_ref()?;
    let first = row.samples.first()?;
    row.samples
        .iter()
        .all(|sample| {
            sample.rss_provenance == first.rss_provenance
                && sample.measurement_protocol == first.measurement_protocol
        })
        .then_some(summary)
}

fn format_dispersion(metric: &MetricSummary, unit: &str) -> String {
    if [
        metric.median,
        metric.median_absolute_deviation,
        metric.p95,
        metric.minimum,
        metric.maximum,
    ]
    .into_iter()
    .any(|value| !value.is_finite() || value < 0.0)
    {
        return "-".to_owned();
    }
    format!(
        "{:.1} {unit} / {:.1} {unit} / {:.1}-{:.1} {unit}",
        metric.median_absolute_deviation / 1_000.0,
        metric.p95 / 1_000.0,
        metric.minimum / 1_000.0,
        metric.maximum / 1_000.0,
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

fn instrumented_sample_count(row: &BenchmarkRow) -> String {
    if row.outcome != BenchmarkOutcome::Timed || row.instrumented_samples.is_empty() {
        return "-".to_owned();
    }
    let Some(first) = row.instrumented_samples.first() else {
        return "-".to_owned();
    };
    if row.instrumented_samples.iter().any(|sample| {
        sample.rss_provenance.is_none()
            || sample.rss_provenance != first.rss_provenance
            || sample.measurement_protocol != first.measurement_protocol
    }) {
        return "-".to_owned();
    }
    row.instrumented_samples.len().to_string()
}

fn format_millis(micros: f64) -> String {
    if !micros.is_finite() || micros < 0.0 {
        "-".to_owned()
    } else {
        format!("{:.1} ms", micros / 1_000.0)
    }
}

fn format_mib(bytes: f64) -> String {
    if !bytes.is_finite() || bytes <= 0.0 {
        "-".to_owned()
    } else {
        format!("{:.1} MiB", bytes / (1024.0 * 1024.0))
    }
}

fn format_rate(value: f64, unit: &str) -> String {
    if !value.is_finite() || value < 0.0 {
        "-".to_owned()
    } else {
        format!("{value:.1} {unit}")
    }
}

fn metric_cell(row: &BenchmarkRow, metric: &str) -> String {
    match metric {
        "Wall time" => wall_time(row),
        "Wall time dispersion" => wall_dispersion(row),
        "First output" => first_output(row),
        "First output dispersion" => first_output_dispersion(row),
        "User CPU" => cpu_time(
            row,
            |summary| summary.user_cpu_micros,
            |sample| sample.user_cpu_micros.is_some(),
        ),
        "System CPU" => cpu_time(
            row,
            |summary| summary.system_cpu_micros,
            |sample| sample.system_cpu_micros.is_some(),
        ),
        "Peak RSS" => memory(row),
        "Instrumented peak RSS" => instrumented_peak_rss(row),
        "Logical throughput" => throughput(row, |summary| summary.logical_records_per_second),
        "Physical throughput" => timed_summary(row).map_or_else(
            || "-".to_owned(),
            |summary| format_rate(summary.physical_mib_per_second, "MiB/s"),
        ),
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
        "Instrumented sample count" => instrumented_sample_count(row),
        "Comparison view" => view(row).to_owned(),
        _ => "-".to_owned(),
    }
}

fn missing_metric_cell(metric: &str) -> &str {
    if is_measurement_metric(metric) {
        "-"
    } else {
        "not recorded"
    }
}

fn is_measurement_metric(metric: &str) -> bool {
    matches!(
        metric,
        "Wall time"
            | "Wall time dispersion"
            | "First output"
            | "First output dispersion"
            | "User CPU"
            | "System CPU"
            | "Peak RSS"
            | "Instrumented peak RSS"
            | "Logical throughput"
            | "Physical throughput"
            | "Output bytes"
    )
}

fn table_has_measurement_placeholders(
    rows: &[&BenchmarkRow],
    columns: &[AdapterColumn<'_>],
) -> bool {
    columns.iter().any(|column| {
        matching_row(rows, column).is_none_or(|row| {
            metric_names(rows)
                .into_iter()
                .filter(|metric| is_measurement_metric(metric))
                .any(|metric| metric_cell(row, metric) == "-")
        })
    })
}

fn append_placeholder_note(
    output: &mut String,
    rows: &[&BenchmarkRow],
    report: &BenchmarkCampaignReport,
) {
    output.push_str(
        "`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below.",
    );
    let mut reasons = rows
        .iter()
        .filter_map(|row| row.diagnostic.as_deref())
        .map(single_line)
        .filter(|reason| !reason.is_empty())
        .collect::<Vec<_>>();
    reasons.sort();
    reasons.dedup();
    if !report.comparability.comparable {
        reasons.extend(
            report
                .comparability
                .reasons
                .iter()
                .map(|reason| single_line(reason)),
        );
        reasons.sort();
        reasons.dedup();
    }
    if !reasons.is_empty() {
        output.push_str(" Applicable exclusions or failures: ");
        output.push_str(
            &reasons
                .iter()
                .map(|reason| format!("`{}`", escape_cell(reason)))
                .collect::<Vec<_>>()
                .join(", "),
        );
        output.push('.');
    }
    output.push_str("\n\n");
}

fn provenance_summary(rows: &[&BenchmarkRow]) -> String {
    let mut labels = BTreeSet::new();
    let mut unknown = false;
    for row in rows {
        for sample in &row.samples {
            if let Some(provenance) = sample.rss_provenance {
                labels.insert(provenance.label().to_owned());
            } else if sample.peak_rss_bytes.is_some() {
                unknown = true;
            }
        }
        for sample in &row.instrumented_samples {
            if let Some(provenance) = sample.rss_provenance {
                labels.insert(format!("instrumented {}", provenance.label()));
            } else if sample.peak_rss_bytes.is_some() {
                labels.insert("instrumented unknown".to_owned());
            }
        }
    }
    let mut values = labels
        .iter()
        .map(|label| format!("`{label}`"))
        .collect::<Vec<_>>();
    if unknown {
        values.push("`unknown`".to_owned());
    }
    if values.is_empty() {
        "unavailable".to_owned()
    } else {
        values.join(", ")
    }
}

fn measurement_method_summary(rows: &[&BenchmarkRow]) -> String {
    let mut methods = BTreeSet::new();
    let mut historical_primary = false;
    let mut historical_instrumented = false;
    for row in rows {
        for sample in &row.samples {
            let Some(protocol) = &sample.measurement_protocol else {
                historical_primary = true;
                continue;
            };
            let accuracy = protocol
                .validated_accuracy_micros
                .map_or_else(|| "unknown".to_owned(), |micros| format!("{micros} us"));
            methods.insert(format!(
                "timing `{}` (observed exit-control bound `{accuracy}`, requested exit observation interval `{}` us), input `{}`, RSS scope `{}`, {}",
                escape_cell(&single_line(&protocol.timing_method)),
                protocol.exit_poll_interval_micros,
                escape_cell(&single_line(&protocol.input_delivery)),
                escape_cell(&single_line(&protocol.rss_scope)),
                protocol_identity_summary(protocol),
            ));
        }
        for sample in &row.instrumented_samples {
            let Some(protocol) = &sample.measurement_protocol else {
                historical_instrumented = true;
                continue;
            };
            let accuracy = protocol
                .validated_accuracy_micros
                .map_or_else(|| "unknown".to_owned(), |micros| format!("{micros} us"));
            methods.insert(format!(
                "instrumented timing `{}` (observed exit-control bound `{accuracy}`, requested exit observation interval `{}` us), input `{}`, RSS scope `{}`, {}",
                escape_cell(&single_line(&protocol.timing_method)),
                protocol.exit_poll_interval_micros,
                escape_cell(&single_line(&protocol.input_delivery)),
                escape_cell(&single_line(&protocol.rss_scope)),
                protocol_identity_summary(protocol),
            ));
        }
    }
    if historical_primary {
        methods.insert("historical/unspecified".to_owned());
    }
    if historical_instrumented {
        methods.insert("instrumented historical/unspecified".to_owned());
    }
    if methods.is_empty() {
        "unavailable".to_owned()
    } else {
        methods.into_iter().collect::<Vec<_>>().join("; ")
    }
}

fn protocol_identity_summary(protocol: &super::MeasurementProtocol) -> String {
    let worker = protocol.worker.as_ref().map_or_else(
        || "worker identity unavailable".to_owned(),
        |worker| {
            format!(
                "worker executable SHA-256 `{}`, launch protocol `{}`, collector sources SHA-256 `{}`",
                escape_cell(&single_line(&worker.executable_sha256)),
                escape_cell(&single_line(&worker.launch_protocol)),
                escape_cell(&single_line(&worker.collector_source_sha256)),
            )
        },
    );
    let isolation = protocol.isolation_evidence.as_ref().map_or_else(
        || "launch-floor evidence unavailable".to_owned(),
        |evidence| {
            format!(
                "launch-floor controls `{}` bytes (max parent delta `{}` bytes, tolerance `{}` bytes, summary SHA-256 `{}`)",
                evidence.control_peak_rss_bytes,
                evidence.max_parent_delta_bytes,
                evidence.tolerance_bytes,
                escape_cell(&single_line(&evidence.summary_sha256)),
            )
        },
    );
    format!("{worker}; {isolation}")
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
        InputFormat, LaunchIsolationEvidence, RegressionGate, RssProvenance, WorkerIdentity,
        summarize_samples,
    };
    use super::{
        RESULTS_END_MARKER, RESULTS_START_MARKER, render_markdown_campaigns, render_markdown_pages,
        workload_filename,
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
        assert!(first.starts_with("# Blocking sort\n\nAuthored introduction.\n\n## Results\n"));
        assert!(first.ends_with(&format!(
            "{RESULTS_END_MARKER}\n\nAuthored trailing note.\n"
        )));
        assert!(first.contains("Last updated: 2026-09-10"));
        assert!(first.contains("| Metric | tq JSON |"));
        assert!(first.contains("| Wall time | 1.2 ms |"));
        assert!(first.contains("| Wall time dispersion | 0.0 ms / 1.2 ms / 1.2-1.2 ms |"));
        assert!(first.contains("| Outcome | timed |"));
        assert!(first.contains("Each timed row has 1 measured sample."));
        assert!(first.contains("`-` denotes no valid comparable measurement, not zero."));
        assert!(!first.contains("Wall (ms)"));
        assert!(!first.contains("Peak RSS (MiB, source)"));
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
    fn multi_host_rendering_keeps_results_separate_and_preserves_authored_bytes() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let page = directory.path().join("blocking-sort.md");
        let source = format!(
            "# Blocking sort\n\nAuthored introduction.\n\n## Results\n{RESULTS_START_MARKER}\nold generated text\n{RESULTS_END_MARKER}\n\nAuthored trailing note.\n"
        );
        fs::write(&page, &source).expect("authored page");

        let mut linux = report(vec![row(
            "benchmark.blocking-sort",
            BenchmarkOutcome::Timed,
        )]);
        linux.environment.os = "linux".to_owned();
        linux.environment.architecture = "x86_64".to_owned();

        let mut macos = linux.clone();
        macos.environment.os = "macos".to_owned();
        macos.environment.architecture = "aarch64".to_owned();
        macos.cases[0].samples[0].wall_time_micros = 2_345;
        macos.cases[0].samples[0].peak_rss_bytes = Some(3 * 1024 * 1024);
        macos.cases[0].summary = summarize_samples(&macos.cases[0].samples, 100, 1);

        render_markdown_campaigns(directory.path(), &[linux, macos]).expect("render hosts");
        let rendered = fs::read_to_string(&page).expect("rendered page");
        assert!(rendered.starts_with("# Blocking sort\n\nAuthored introduction.\n\n## Results\n"));
        assert!(rendered.ends_with(&format!(
            "{RESULTS_END_MARKER}\n\nAuthored trailing note.\n"
        )));
        assert!(rendered.contains("### Host: linux / x86_64 / standard"));
        assert!(rendered.contains("### Host: macos / aarch64 / standard"));
        assert_eq!(rendered.matches("| Metric | tq JSON |").count(), 2);
        assert!(rendered.contains("| Wall time | 1.2 ms |"));
        assert!(rendered.contains("| Wall time | 2.3 ms |"));
        assert!(rendered.contains("| Peak RSS | 2.0 MiB |"));
        assert!(rendered.contains("| Peak RSS | 3.0 MiB |"));

        let index = fs::read_to_string(directory.path().join("index.md")).expect("index");
        assert_eq!(index.matches("### Host:").count(), 2);
    }

    #[test]
    fn multi_host_rendering_marks_a_missing_workload_as_unmeasured() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let page = directory.path().join("blocking-sort.md");
        fs::write(
            &page,
            format!("# Workload\n\n## Results\n{RESULTS_START_MARKER}\n{RESULTS_END_MARKER}\n"),
        )
        .expect("authored page");

        let mut present = report(vec![row(
            "benchmark.blocking-sort",
            BenchmarkOutcome::Timed,
        )]);
        present.environment.os = "linux".to_owned();
        present.environment.architecture = "x86_64".to_owned();
        let mut missing = report(Vec::new());
        missing.environment.os = "macos".to_owned();
        missing.environment.architecture = "aarch64".to_owned();

        render_markdown_campaigns(directory.path(), &[present, missing]).expect("render hosts");
        let rendered = fs::read_to_string(page).expect("rendered page");
        assert!(rendered.contains("### Host: macos / aarch64 / standard"));
        assert!(rendered.contains(
            "No row was recorded for `benchmark.blocking-sort` on this host; all measurements are unmeasured (`-`)."
        ));
    }

    #[test]
    fn multi_host_rendering_validates_all_markers_before_writing() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let good = directory.path().join("a.md");
        let bad = directory.path().join("b.md");
        let good_source =
            format!("# A\n\n## Results\n{RESULTS_START_MARKER}\nold\n{RESULTS_END_MARKER}\n");
        fs::write(&good, &good_source).expect("good page");
        fs::write(&bad, "# B\n\n## Results\nmissing markers\n").expect("bad page");
        let reports = [
            report(vec![row("benchmark.a", BenchmarkOutcome::Timed)]),
            report(vec![row("benchmark.b", BenchmarkOutcome::Timed)]),
        ];

        assert!(render_markdown_campaigns(directory.path(), &reports).is_err());
        assert_eq!(fs::read_to_string(good).expect("good page"), good_source);
    }

    #[test]
    fn native_rendering_rejects_missing_timing_calibration_before_writing() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let page = directory.path().join("blocking-sort.md");
        let source = format!(
            "# Workload\n\n## Results\n{RESULTS_START_MARKER}\nold\n{RESULTS_END_MARKER}\n"
        );
        fs::write(&page, &source).expect("authored page");

        let mut native = row("benchmark.blocking-sort", BenchmarkOutcome::Timed);
        native.samples[0].measurement_protocol = Some(super::super::MeasurementProtocol {
            timing_method: "native wait4".to_owned(),
            input_delivery: "preloaded stdin".to_owned(),
            rss_scope: "specific child lifetime including pre-exec and waited descendants"
                .to_owned(),
            exit_poll_interval_micros: 100,
            rss_poll_interval_micros: None,
            validated_accuracy_micros: None,
            worker: Some(worker_identity()),
            isolation_evidence: Some(isolation_evidence()),
        });
        native.samples[0].rss_provenance = Some(RssProvenance::LinuxWait4);
        native.samples[0].user_cpu_micros = Some(10);
        native.samples[0].system_cpu_micros = Some(10);
        let report = report(vec![native]);

        let error = render_markdown_pages(directory.path(), &report).expect_err("reject report");
        assert!(
            error
                .to_string()
                .contains("positive validated timing accuracy")
        );
        assert_eq!(fs::read_to_string(&page).expect("authored page"), source);

        let mut calibrated = report;
        calibrated.cases[0].samples[0]
            .measurement_protocol
            .as_mut()
            .expect("native protocol")
            .validated_accuracy_micros = Some(1_000);
        render_markdown_pages(directory.path(), &calibrated).expect("render calibrated report");
        let rendered = fs::read_to_string(&page).expect("rendered calibrated page");
        assert!(rendered.contains("Measurement method (outside measurement tables):"));
        assert!(rendered.contains("native wait4"));
        assert!(rendered.contains("worker executable SHA-256 `worker`"));
        assert!(rendered.contains("launch-floor controls `1` bytes"));
    }

    #[test]
    fn native_rendering_rejects_invalid_instrumented_evidence_before_writing() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let page = directory.path().join("blocking-sort.md");
        let source = format!(
            "# Workload\n\n## Results\n{RESULTS_START_MARKER}\nold\n{RESULTS_END_MARKER}\n"
        );
        fs::write(&page, &source).expect("authored page");

        let mut native = row("benchmark.blocking-sort", BenchmarkOutcome::Timed);
        native.samples[0].measurement_protocol = Some(super::super::MeasurementProtocol {
            timing_method: "native wait4".to_owned(),
            input_delivery: "preloaded stdin".to_owned(),
            rss_scope: "specific child lifetime including pre-exec and waited descendants"
                .to_owned(),
            exit_poll_interval_micros: 100,
            rss_poll_interval_micros: None,
            validated_accuracy_micros: Some(1_000),
            worker: Some(worker_identity()),
            isolation_evidence: Some(isolation_evidence()),
        });
        native.samples[0].rss_provenance = Some(RssProvenance::LinuxWait4);
        native.samples[0].user_cpu_micros = Some(10);
        native.samples[0].system_cpu_micros = Some(10);
        let mut instrumented = native.samples[0].clone();
        instrumented.measurement_protocol = Some(super::super::MeasurementProtocol {
            timing_method: "native wait4 sampled".to_owned(),
            input_delivery: "preloaded stdin".to_owned(),
            rss_scope: "sampled process-group lifetime including pre-exec and waited descendants"
                .to_owned(),
            exit_poll_interval_micros: 100,
            rss_poll_interval_micros: Some(100),
            validated_accuracy_micros: Some(1_000),
            worker: Some(worker_identity()),
            isolation_evidence: Some(isolation_evidence()),
        });
        instrumented.peak_rss_bytes = None;
        native.instrumented_samples = vec![instrumented];
        let report = report(vec![native]);

        let error = render_markdown_pages(directory.path(), &report).expect_err("reject report");
        assert!(error.to_string().contains("no positive authoritative RSS"));
        assert_eq!(fs::read_to_string(page).expect("authored page"), source);
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
        assert!(rendered.contains("| Wall time | - |"));
        assert!(!rendered.contains("0.000"));
        assert!(rendered.contains("| Samples (warmups) | not timed |"));
    }

    #[test]
    fn renders_available_metrics_and_distinguishes_missing_rss() {
        let mut measured = row("benchmark.blocking-sort", BenchmarkOutcome::Timed);
        measured.samples = vec![BenchmarkSample {
            measurement_protocol: None,
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
        measured
            .summary
            .as_mut()
            .unwrap()
            .logical_records_per_second = 42.04;
        measured.summary.as_mut().unwrap().physical_mib_per_second = 42.04;
        measured.instrumented_samples = vec![BenchmarkSample {
            measurement_protocol: Some(super::super::MeasurementProtocol {
                timing_method: "native wait4 instrumented".to_owned(),
                input_delivery: "preloaded stdin".to_owned(),
                rss_scope: "sampled process group".to_owned(),
                exit_poll_interval_micros: 100,
                rss_poll_interval_micros: Some(100),
                validated_accuracy_micros: None,
                worker: None,
                isolation_evidence: None,
            }),
            wall_time_micros: 1_345,
            user_cpu_micros: Some(210),
            system_cpu_micros: Some(35),
            peak_rss_bytes: Some(5 * 1024 * 1024),
            rss_provenance: Some(RssProvenance::BsdTimeL),
            process_group_peak_rss_bytes: Some(4 * 1024 * 1024),
            first_result_micros: None,
            output_bytes: 42,
        }];
        let mut missing_rss = measured.clone();
        missing_rss.adapter_id = "jq-json".to_owned();
        missing_rss.samples[0].peak_rss_bytes = None;
        missing_rss.summary = summarize_samples(&missing_rss.samples, 100, 1);
        let campaign = report(vec![measured, missing_rss]);

        let rendered = super::render_results(&campaign, &campaign.cases.iter().collect::<Vec<_>>());
        assert!(rendered.contains("| First output | 0.6 ms | 0.6 ms |"));
        assert!(rendered.contains(
            "| First output dispersion | 0.0 ms / 0.6 ms / 0.6-0.6 ms | 0.0 ms / 0.6 ms / 0.6-0.6 ms |"
        ));
        assert!(rendered.contains("| User CPU | 0.2 ms | 0.2 ms |"));
        assert!(rendered.contains("| System CPU | 0.0 ms | 0.0 ms |"));
        assert!(rendered.contains("| Peak RSS | - | 2.0 MiB |"));
        assert!(rendered.contains("| Instrumented peak RSS | 4.0 MiB | 4.0 MiB |"));
        assert!(rendered.contains("| Instrumented sample count | 1 | 1 |"));
        assert!(rendered.contains("| Logical throughput | 810.4 records/s | 42.0 records/s |"));
        assert!(rendered.contains("| Physical throughput | 0.1 MiB/s | 42.0 MiB/s |"));
        assert!(rendered.contains("RSS collector provenance (outside measurement tables):"));
        assert!(rendered.contains("`bsd-time-l`"));
        assert!(rendered.contains("instrumented bsd-time-l"));
        assert!(rendered.contains("instrumented timing `native wait4 instrumented`"));
        assert!(
            !rendered
                .lines()
                .any(|line| line.starts_with("| Peak RSS |") && line.contains("bsd-time-l"))
        );
        assert!(rendered.contains("| Output bytes | 42 | 42 |"));
        assert!(
            rendered
                .contains("| Samples (warmups) | 1 measured (1 warmup) | 1 measured (1 warmup) |")
        );
        assert!(!rendered.contains("sampled/unverified"));
        assert!(!rendered.contains("unverified"));

        let mut overview_campaign = campaign.clone();
        overview_campaign.corpus = serde_json::from_value(serde_json::json!([
            {"origin":"frozen", "source_id":"dataset", "tier":"small", "format":"json",
             "artifact":{"path":"input.json", "bytes":1000, "sha256":"digest"},
             "logical_records":1, "manifest_sha256":"manifest"}
        ]))
        .expect("overview corpus");
        let overview = super::render_overview(&overview_campaign);
        assert!(overview.contains("Instrumented peak RSS"));
        assert!(overview.contains("Instrumented sample count"));
    }

    #[test]
    fn instrumented_metrics_use_placeholders_for_failed_or_invalid_rows() {
        let mut failed = row("benchmark.blocking-sort", BenchmarkOutcome::Incorrect);
        failed.instrumented_samples = vec![failed.samples[0].clone()];
        let mut invalid = row("benchmark.blocking-sort", BenchmarkOutcome::Timed);
        invalid.adapter_id = "jq-json".to_owned();
        invalid.instrumented_samples = vec![BenchmarkSample {
            process_group_peak_rss_bytes: None,
            ..invalid.samples[0].clone()
        }];
        let campaign = report(vec![failed, invalid]);
        let rendered = super::render_results(&campaign, &campaign.cases.iter().collect::<Vec<_>>());

        assert!(rendered.contains("| Instrumented peak RSS | - | - |"));
        assert!(rendered.contains("| Instrumented sample count | 1 | - |"));
        assert!(rendered.contains("`-` denotes no valid comparable measurement, not zero."));
    }

    #[test]
    fn mixed_measurement_samples_are_not_rendered_from_a_stale_summary() {
        let mut mixed = row("benchmark.blocking-sort", BenchmarkOutcome::Timed);
        let mut second = mixed.samples[0].clone();
        second.rss_provenance = Some(RssProvenance::GnuTimeV);
        mixed.samples.push(second);

        let campaign = report(vec![mixed]);
        let rendered = super::render_results(&campaign, &campaign.cases.iter().collect::<Vec<_>>());

        assert!(rendered.contains("| Wall time | - |"));
        assert!(rendered.contains("| Peak RSS | - |"));
        assert!(rendered.contains("`-` denotes no valid comparable measurement, not zero."));
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
        assert!(large.contains("| Wall time | - | - |"));
        assert!(small.contains("| Wall time | 9.0 ms | 1.2 ms |"));
        assert!(small.contains(
            "| Wall time dispersion | 0.0 ms / 1.2 ms / 1.2-1.2 ms | 0.0 ms / 1.2 ms / 1.2-1.2 ms |"
        ));
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
        assert!(overview.contains("| blocking-sort | Wall time | - | 1.2 ms |"));
        assert!(overview.contains("| blocking-sort | Peak RSS | - | 2.0 MiB |"));
    }

    #[test]
    fn placeholders_explain_non_comparable_rows_and_keep_provenance_outside_cells() {
        let mut invalid = row("benchmark.blocking-sort", BenchmarkOutcome::Timed);
        invalid.summary.as_mut().unwrap().wall_time_micros.median = f64::NAN;
        invalid.summary.as_mut().unwrap().peak_rss_bytes = Some(0);
        let mut campaign = report(vec![invalid]);
        campaign.comparability = Comparability {
            comparable: false,
            reasons: vec!["measurement contract differs".to_owned()],
        };

        let rendered = super::render_results(&campaign, &campaign.cases.iter().collect::<Vec<_>>());
        assert!(rendered.contains("| Wall time | - |"));
        assert!(rendered.contains("| Peak RSS | - |"));
        assert!(rendered.contains("`-` denotes no valid comparable measurement, not zero."));
        assert!(
            rendered.contains("Applicable exclusions or failures: `measurement contract differs`.")
        );
        assert!(
            rendered
                .contains("RSS collector provenance (outside measurement tables): `bsd-time-l`.")
        );
        assert!(
            !rendered
                .lines()
                .any(|line| line.starts_with("| ") && line.contains("bsd-time-l"))
        );
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
            measurement_protocol: None,
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
            instrumented_samples: Vec::new(),
            summary: summarize_samples(&samples, 100, 1),
            reference_ratios: BTreeMap::new(),
            reference_peak_rss_ratios: BTreeMap::new(),
            soft_performance_objective: None,
        }
    }

    fn worker_identity() -> WorkerIdentity {
        WorkerIdentity {
            executable_sha256: "worker".to_owned(),
            launch_protocol: "direct-target-v1".to_owned(),
            collector_source_sha256: "collector".to_owned(),
        }
    }

    fn isolation_evidence() -> LaunchIsolationEvidence {
        LaunchIsolationEvidence {
            summary_sha256: "summary".to_owned(),
            control_peak_rss_bytes: 1,
            max_parent_delta_bytes: 0,
            tolerance_bytes: 1,
        }
    }
}
