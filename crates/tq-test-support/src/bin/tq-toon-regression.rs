//! Prepare immutable TOON migration inputs/baseline, then compare a final candidate.

use std::{collections::BTreeMap, env, fs, path::PathBuf, process::ExitCode};
use tq_test_support::benchmark::migration_guard::{self, BuildManifest};

const USAGE: &str = "Usage:\n  tq-toon-regression prepare --baseline PATH --directory DIR --build-manifest JSON --deadline-seconds N\n  tq-toon-regression run --candidate PATH --directory DIR --build-manifest JSON --report-file JSON --deadline-seconds N\n\nPrepare creates immutable baseline.json, a frozen executable and inputs.json.\nRun preserves every selected row, correctness/plan/spool evidence and native paired samples.\nConfirmed wall-time regressions above 20% fail; observed median regressions above 10%\nreceive a nonblocking review note. Exactly 20% passes the hard threshold.\nExit 0: every row passes; 1: observed regression/incorrect/incompatible evidence;\n2: invalid invocation or incomplete/inconclusive evidence. This is not calibrated approval.\nBuild JSON requires source_revision, source_sha256, toolchain, target, profile=release,\nfeatures=[default], allocator, threads (runtime policy string), optimization.\nHost CPU count is collected separately; threads is not that count.\nThe tq-bench-worker binary must be beside this helper or set by TQ_BENCH_WORKER.";

struct Options {
    prepare: bool,
    binary: PathBuf,
    directory: PathBuf,
    build_manifest: PathBuf,
    report: Option<PathBuf>,
    deadline_seconds: u64,
}

fn parse() -> Result<Option<Options>, String> {
    let mut args = env::args().skip(1);
    let Some(phase) = args.next() else {
        return Err(USAGE.into());
    };
    if matches!(phase.as_str(), "--help" | "-h") {
        return Ok(None);
    }
    let prepare = match phase.as_str() {
        "prepare" => true,
        "run" => false,
        _ => return Err("expected prepare or run phase".into()),
    };
    let mut flags = BTreeMap::new();
    while let Some(flag) = args.next() {
        if !matches!(
            flag.as_str(),
            "--baseline"
                | "--candidate"
                | "--directory"
                | "--build-manifest"
                | "--report-file"
                | "--deadline-seconds"
        ) {
            return Err(format!("unknown option: {flag}"));
        }
        let value = args
            .next()
            .filter(|value| !value.starts_with("--"))
            .ok_or_else(|| format!("missing value for {flag}"))?;
        if flags.insert(flag.clone(), value).is_some() {
            return Err(format!("repeated option: {flag}"));
        }
    }
    let mut required = |key: &str| {
        flags
            .remove(key)
            .ok_or_else(|| format!("required option: {key}"))
    };
    let binary = PathBuf::from(required(if prepare {
        "--baseline"
    } else {
        "--candidate"
    })?);
    let directory = PathBuf::from(required("--directory")?);
    let build_manifest = PathBuf::from(required("--build-manifest")?);
    let deadline_seconds = required("--deadline-seconds")?
        .parse::<u64>()
        .map_err(|_| "deadline must be a positive integer")?;
    if deadline_seconds == 0 {
        return Err("deadline must be positive".into());
    }
    let report = if prepare {
        None
    } else {
        Some(PathBuf::from(required("--report-file")?))
    };
    if !flags.is_empty() {
        return Err("option is not valid for this phase".into());
    }
    Ok(Some(Options {
        prepare,
        binary,
        directory,
        build_manifest,
        report,
        deadline_seconds,
    }))
}

fn execute(options: &Options) -> migration_guard::Result<ExitCode> {
    let build: BuildManifest = serde_json::from_slice(&fs::read(&options.build_manifest)?)?;
    if options.prepare {
        let frozen = migration_guard::prepare(
            &options.binary,
            &options.directory,
            build,
            options.deadline_seconds,
        )?;
        println!(
            "Frozen {} workloads at {} (baseline {}).",
            frozen.workloads.len(),
            options.directory.display(),
            frozen.binary.executable.sha256
        );
        Ok(ExitCode::SUCCESS)
    } else {
        let report = migration_guard::run(
            &options.binary,
            &options.directory,
            build,
            options.report.as_deref().expect("run requires report"),
            options.deadline_seconds,
        )?;
        for row in &report.rows {
            println!(
                "{}: {:?}; initial ratio {:?}; confirmation ratio {:?}",
                row.workload.id,
                row.decision,
                row.initial_summary.as_ref().map(|s| s.ratio),
                row.confirmation_summary.as_ref().map(|s| s.ratio)
            );
            for (round, summary) in [
                ("initial", row.initial_summary.as_ref()),
                ("confirmation", row.confirmation_summary.as_ref()),
            ] {
                if let Some(note) = summary.and_then(|summary| summary.review_note.as_deref()) {
                    println!("  Note ({round}): {note}");
                }
            }
        }
        if report.diagnostic.as_deref() == Some(migration_guard::REPORT_PATH_CONFLICT) {
            eprintln!("{}", migration_guard::REPORT_PATH_CONFLICT);
        } else {
            if let Some(diagnostic) = &report.diagnostic {
                eprintln!("{diagnostic}");
            }
            println!(
                "Report: {} (exit {})",
                options.report.as_ref().expect("run report").display(),
                report.exit_code
            );
        }
        Ok(ExitCode::from(u8::try_from(report.exit_code)?))
    }
}

fn main() -> ExitCode {
    let options = match parse() {
        Ok(Some(options)) => options,
        Ok(None) => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            eprintln!("tq-toon-regression: {error}\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    if !options.prepare
        && options
            .report
            .as_ref()
            .and_then(|report| fs::canonicalize(report).ok())
            .is_some_and(|report| {
                fs::canonicalize(&options.build_manifest).ok().as_ref() == Some(&report)
            })
    {
        eprintln!("report file must differ from the build manifest");
        return ExitCode::from(2);
    }
    match execute(&options) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("tq-toon-regression: {error}");
            // Missing/invalid campaign inputs must leave a durable non-passing
            // report, without discarding an existing checkpoint's raw samples.
            let path = options
                .report
                .unwrap_or_else(|| options.directory.join("prepare-report.json"));
            let previous: Option<serde_json::Value> = fs::read(&path)
                .ok()
                .and_then(|bytes| serde_json::from_slice(&bytes).ok());
            let is_checkpoint = previous.as_ref().is_some_and(|report| {
                if options.prepare {
                    report["completed_correctness"].is_array() && report["current_row"].is_string()
                } else {
                    report["profile"] == "focused-migration-repeated" && report["rows"].is_array()
                }
            });
            let path_entry = fs::symlink_metadata(&path).ok();
            if path_entry
                .as_ref()
                .is_some_and(|entry| entry.file_type().is_symlink() || !is_checkpoint)
            {
                eprintln!(
                    "refusing to overwrite non-checkpoint report path {}",
                    path.display()
                );
                return ExitCode::from(2);
            }
            let mut report = if is_checkpoint {
                previous.expect("recognized checkpoint exists")
            } else {
                serde_json::json!({"schema_version":1,"profile":"focused-migration-repeated","calibrated":false})
            };
            report["exit_code"] = 2.into();
            report["fatal_error"] = error.to_string().into();
            match serde_json::to_vec_pretty(&report)
                .map_err(std::io::Error::other)
                .and_then(|bytes| fs::write(&path, bytes))
            {
                Ok(()) => {}
                Err(report_error) => {
                    eprintln!("could not retain report {}: {report_error}", path.display());
                }
            }
            ExitCode::from(2)
        }
    }
}
