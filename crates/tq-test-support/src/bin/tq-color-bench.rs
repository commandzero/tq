//! Correctness-gated output color measurements and reproducible Markdown.

#[path = "tq_color_bench/report.rs"]
mod report;

use std::{env, error::Error, fs, path::PathBuf, time::Duration};
use tq_test_support::benchmark::{
    BenchmarkInvocation, collect_environment, measure_process, preflight_rss, run_allocation_probe,
};

use report::{
    CASES, ColorReport, ColorRow, check_colored_output, checked_sample, render_reports, sha256,
};

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.first().map(String::as_str) {
        Some("--internal-rss-control") => return Ok(()),
        Some("--internal-rss-probe") => {
            run_allocation_probe(64 * 1024 * 1024, 1)?;
            return Ok(());
        }
        Some("--render-only") if args.len() >= 4 => {
            let reports = args[2..].iter().map(|path| {
                Ok(serde_json::from_slice(&fs::read(path)?)?)
            }).collect::<Result<Vec<ColorReport>, Box<dyn Error>>>()?;
            return render_reports(&PathBuf::from(&args[1]), &reports);
        }
        Some("run") if args.len() == 6 => (),
        _ => return Err("Usage: tq-color-bench run TQ_BIN WEEK_JSON MONTH_JSON ARCHIVE_DIR MARKDOWN_PAGE\n       tq-color-bench --render-only MARKDOWN_PAGE WEEK_REPORT MONTH_REPORT".into()),
    }
    if cfg!(debug_assertions) {
        return Err(
            "color measurements require a release build; render-only works in debug builds".into(),
        );
    }
    if env::var_os("JQ_COLORS").is_some() || env::var_os("NO_COLOR").is_some() {
        return Err("unset JQ_COLORS and NO_COLOR before measuring the built-in palette".into());
    }
    let binary = fs::canonicalize(&args[1])?;
    let archive = PathBuf::from(&args[4]);
    let page = PathBuf::from(&args[5]);
    // Check the authored page before spending time measuring anything.
    tq_test_support::benchmark::replace_results_region(&page, &fs::read_to_string(&page)?, "")?;
    fs::create_dir_all(&archive)?;
    for name in ["week.json", "month.json"] {
        if archive.join(name).exists() {
            return Err(
                "archive already contains color reports; choose a new run directory".into(),
            );
        }
    }
    let preflight = preflight_rss(None)?;
    let environment = collect_environment("release");
    let mut reports = Vec::new();
    for (label, source) in [("week", &args[2]), ("month", &args[3])] {
        let source = fs::canonicalize(source)?;
        let mut report = ColorReport {
            source_bytes: fs::metadata(&source)?.len(),
            source_sha256: Some(sha256(&source)?),
            binary_sha256: Some(sha256(&binary)?),
            source: source.clone(),
            binary: binary.clone(),
            environment: Some(environment.clone()),
            preflight: preflight.provenance.label().into(),
            warmups: 2,
            measured_samples: 5,
            rows: Vec::new(),
        };
        measure_rows(&mut report, label)?;
        if sha256(&source)? != report.source_sha256.clone().unwrap_or_default()
            || sha256(&binary)? != report.binary_sha256.clone().unwrap_or_default()
        {
            return Err("input or binary changed during measurement".into());
        }
        serde_json::to_writer_pretty(
            fs::File::create_new(archive.join(format!("{label}.json")))?,
            &report,
        )?;
        reports.push(report);
    }
    render_reports(&page, &reports)
}

fn measure_rows(report: &mut ColorReport, label: &str) -> Result<(), Box<dyn Error>> {
    let source = report.source.clone();
    let binary = report.binary.clone();
    for (case, format, query) in CASES {
        let mut plain = Vec::new();
        for flag in ["-M", "-C"] {
            let request = BenchmarkInvocation {
                executable: binary.clone(),
                args: vec![
                    "-i".into(),
                    "json".into(),
                    "-o".into(),
                    format.into(),
                    flag.into(),
                    query.into(),
                    source.to_string_lossy().into_owned(),
                ],
                stdin: vec![],
                current_dir: None,
                timeout: Duration::from_secs(120),
                output_limit: 512 * 1024 * 1024,
                rss_limit: None,
                cancellation: None,
                retain_output: true,
            };
            let check = measure_process(&request)?;
            checked_sample(&check)?;
            let output = fs::read(
                check
                    .stdout_path
                    .as_ref()
                    .ok_or("missing retained output")?,
            )?;
            let color_present = if flag == "-M" {
                plain = output;
                false
            } else {
                if !check_colored_output(&plain, &output)? {
                    return Err(format!("no generated color for {case}; captures retained").into());
                }
                true
            };
            let expected_bytes = check.output_bytes;
            for path in [check.stdout_path, check.stderr_path].into_iter().flatten() {
                fs::remove_file(path)?;
            }
            let timed = BenchmarkInvocation {
                retain_output: false,
                ..request
            };
            let mut samples = Vec::new();
            for index in 0..report.warmups + report.measured_samples {
                let sample = measure_process(&timed)?;
                checked_sample(&sample)?;
                if sample.output_bytes != expected_bytes {
                    return Err("output size changed after correctness gate".into());
                }
                if index >= report.warmups {
                    samples.push(sample);
                }
            }
            report.rows.push(ColorRow {
                case: case.into(),
                flag: flag.into(),
                stripped_correctness: "byte-identical".into(),
                color_present: Some(color_present),
                samples,
            });
            eprintln!("completed {label} {case} {flag}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires TQ_COLOR_TEST_BIN and native child-accounting permissions"]
    fn measurement_rows_feed_the_same_renderer() {
        let binary =
            PathBuf::from(env::var_os("TQ_COLOR_TEST_BIN").expect("set TQ_COLOR_TEST_BIN"));
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("synthetic.json");
        fs::write(
            &source,
            br#"{"features":[{"id":"a","properties":{"mag":1,"place":"test\u001bX"}}]}"#,
        )
        .unwrap();
        let mut report = ColorReport {
            source: source.clone(),
            source_bytes: fs::metadata(source).unwrap().len(),
            binary,
            source_sha256: None,
            binary_sha256: None,
            environment: None,
            preflight: if cfg!(target_os = "macos") {
                "darwin-wait4"
            } else {
                "linux-wait4"
            }
            .into(),
            warmups: 0,
            measured_samples: 1,
            rows: Vec::new(),
        };
        measure_rows(&mut report, "synthetic integration fixture").unwrap();
        assert_eq!(report.rows.len(), 8);
        let second: ColorReport =
            serde_json::from_value(serde_json::to_value(&report).unwrap()).unwrap();
        assert!(
            second
                .rows
                .iter()
                .all(|row| row.color_present == Some(row.flag == "-C"))
        );
        let page = directory.path().join("report.md");
        fs::write(&page, "# Synthetic test\n\n## Results\n<!-- benchmark-results:start -->\n<!-- benchmark-results:end -->\n").unwrap();
        render_reports(&page, &[report, second]).unwrap();
        assert!(
            fs::read_to_string(page)
                .unwrap()
                .contains("| sequence-transcode |")
        );
    }
}
