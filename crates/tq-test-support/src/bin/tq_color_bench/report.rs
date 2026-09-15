//! Validated color reports and stable, authored-page rendering.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    error::Error,
    fmt::Write as _,
    fs,
    io::Read,
    path::{Path, PathBuf},
};
use tq_test_support::benchmark::{
    EnvironmentManifest, MeasuredOutcome, MeasuredStatus, replace_results_region,
};

pub const CASES: [(&str, &str, &str); 4] = [
    ("document-json-identity", "json", "."),
    (
        "json-projection",
        "json",
        ".features[] | {id, mag: .properties.mag, place: .properties.place}",
    ),
    ("identity-transcode", "toon", "."),
    ("sequence-transcode", "toon-seq", "."),
];

#[derive(Deserialize, Serialize)]
pub struct ColorReport {
    pub source: PathBuf,
    pub source_bytes: u64,
    pub binary: PathBuf,
    #[serde(default)]
    pub source_sha256: Option<String>,
    #[serde(default)]
    pub binary_sha256: Option<String>,
    #[serde(default)]
    pub environment: Option<EnvironmentManifest>,
    pub preflight: String,
    pub warmups: usize,
    pub measured_samples: usize,
    pub rows: Vec<ColorRow>,
}

#[derive(Deserialize, Serialize)]
pub struct ColorRow {
    pub case: String,
    pub flag: String,
    pub stripped_correctness: String,
    /// Whether the correctness capture contained generated SGR. Missing evidence
    /// in older reports is rejected rather than inferred from the requested flag.
    #[serde(default)]
    pub color_present: Option<bool>,
    pub samples: Vec<MeasuredOutcome>,
}

pub fn sha256(path: &Path) -> Result<String, Box<dyn Error>> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    let mut digest = String::with_capacity(64);
    for byte in hasher.finalize() {
        write!(digest, "{byte:02x}")?;
    }
    Ok(digest)
}

pub fn strip_sgr(bytes: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut result = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == 0x1b {
            if bytes.get(index + 1) != Some(&b'[') {
                return Err("unexpected escape in structured output".into());
            }
            index += 2;
            while bytes
                .get(index)
                .is_some_and(|b| b.is_ascii_digit() || *b == b';')
            {
                index += 1;
            }
            if bytes.get(index) != Some(&b'm') {
                return Err("unterminated or non-SGR escape".into());
            }
        } else {
            result.push(bytes[index]);
        }
        index += 1;
    }
    Ok(result)
}

pub fn checked_sample(sample: &MeasuredOutcome) -> Result<(), Box<dyn Error>> {
    if sample.status != MeasuredStatus::Exited
        || sample.exit_code != Some(0)
        || sample.wall_time_micros == 0
        || sample.user_cpu_micros.is_none()
        || sample.system_cpu_micros.is_none()
        || sample.peak_rss_bytes.is_none_or(|n| n == 0)
        || !matches!(
            sample.rss_provenance.label(),
            "darwin-wait4" | "linux-wait4"
        )
        || sample.measurement_protocol.worker.is_none()
    {
        return Err("color report requires successful timed samples with native worker RSS".into());
    }
    Ok(())
}

fn validate(report: &ColorReport) -> Result<(), Box<dyn Error>> {
    if report.rows.len() != 8 || report.measured_samples == 0 || report.source_bytes == 0 {
        return Err("incomplete color report".into());
    }
    let mut seen = BTreeSet::new();
    for row in &report.rows {
        if row.color_present != Some(row.flag == "-C") {
            return Err(
                "missing or inconsistent color-presence evidence; remeasure the report".into(),
            );
        }
        if !CASES.iter().any(|(case, _, _)| *case == row.case)
            || !matches!(row.flag.as_str(), "-M" | "-C")
            || !seen.insert((&row.case, &row.flag))
            || row.stripped_correctness != "byte-identical"
            || row.samples.len() != report.measured_samples
        {
            return Err("invalid or duplicate color row".into());
        }
        for sample in &row.samples {
            checked_sample(sample)?;
            if sample.rss_provenance.label() != report.preflight
                || sample.output_bytes != row.samples[0].output_bytes
                || sample.measurement_protocol != report.rows[0].samples[0].measurement_protocol
            {
                return Err("inconsistent color sample provenance or output size".into());
            }
        }
    }
    Ok(())
}

#[allow(clippy::cast_precision_loss)]
fn median(row: &ColorRow, metric: fn(&MeasuredOutcome) -> f64) -> f64 {
    let mut samples = row.samples.iter().map(metric).collect::<Vec<_>>();
    samples.sort_by(f64::total_cmp);
    let middle = samples.len() / 2;
    if samples.len() % 2 == 0 {
        samples[middle - 1].midpoint(samples[middle])
    } else {
        samples[middle]
    }
}

fn cell(value: &str) -> String {
    value.replace('|', "\\|").replace(['\n', '\r'], " ")
}

pub fn render_reports(page: &Path, reports: &[ColorReport]) -> Result<(), Box<dyn Error>> {
    if reports.len() != 2 {
        return Err("supply week and month reports, in that order".into());
    }
    for report in reports {
        validate(report)?;
    }
    let generated = render(reports)?;
    let source = fs::read_to_string(page)?;
    let updated = replace_results_region(page, &source, &generated)?;
    // All reports and markers are validated before replacing the page.
    fs::write(page, updated)?;
    Ok(())
}

#[allow(clippy::cast_precision_loss)]
fn render(reports: &[ColorReport]) -> Result<String, std::fmt::Error> {
    let mut output = String::from(
        "Generated from correctness-gated color reports. Lower time and RSS are better.\n\n",
    );
    for (label, report) in ["Week", "Month"].into_iter().zip(reports) {
        writeln!(
            output,
            "### {label}\n\nInput: {} bytes. Warmups: {}. Measured samples per row: {}. RSS collector: `{}`.\n",
            report.source_bytes,
            report.warmups,
            report.measured_samples,
            cell(&report.preflight)
        )?;
        if let Some(host) = &report.environment {
            writeln!(
                output,
                "Host: {} / {} / {}; kernel: {}; logical CPUs: {}; RAM bytes: {}; build: {}. Recorded: {}.\n",
                cell(host.cpu_model.as_deref().unwrap_or("not recorded")),
                cell(&host.os),
                cell(&host.architecture),
                cell(host.kernel.as_deref().unwrap_or("not recorded")),
                host.logical_cpus
                    .map_or_else(|| "not recorded".into(), |n| n.to_string()),
                host.memory_bytes
                    .map_or_else(|| "not recorded".into(), |n| n.to_string()),
                cell(&host.compiler_profile),
                cell(&host.collected_at)
            )?;
        } else {
            output.push_str("Host/build metadata was not recorded in this legacy raw report. No current-host metadata is substituted.\n\n");
        }
        output.push_str("| Case | Plain ms | Color ms | Time ratio | Plain RSS MiB | Color RSS MiB | Plain bytes | Color bytes |\n| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n");
        for (case, _, _) in CASES {
            let plain = report
                .rows
                .iter()
                .find(|r| r.case == case && r.flag == "-M")
                .expect("validated pair");
            let color = report
                .rows
                .iter()
                .find(|r| r.case == case && r.flag == "-C")
                .expect("validated pair");
            let time = |s: &MeasuredOutcome| s.wall_time_micros as f64 / 1000.0;
            let rss =
                |s: &MeasuredOutcome| s.peak_rss_bytes.unwrap_or_default() as f64 / 1_048_576.0;
            let plain_ms = median(plain, time);
            let color_ms = median(color, time);
            writeln!(
                output,
                "| {case} | {plain_ms:.1} | {color_ms:.1} | {:.2}× | {:.1} | {:.1} | {} | {} |",
                color_ms / plain_ms,
                median(plain, rss),
                median(color, rss),
                plain.samples[0].output_bytes,
                color.samples[0].output_bytes
            )?;
        }
        output.push('\n');
    }
    Ok(output.trim_end().into())
}

#[cfg(test)]
mod tests;
