//! Review-only compatibility baseline workflow.

use std::{collections::BTreeSet, env, fs, path::PathBuf, process::ExitCode};

use tq_test_support::compatibility::{
    CompatibilityBaseline, CompatibilityReport, accept_reviewed_candidate, diff_baselines,
    read_baseline, write_baseline_atomic,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tq-baseline: {error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = env::args().skip(1);
    let command = arguments.next().ok_or("expected diff or review")?;
    let current_path = arguments
        .next()
        .ok_or("expected current baseline path or -")?;
    let candidate_path = PathBuf::from(arguments.next().ok_or("expected candidate report path")?);
    let current = if current_path == "-" {
        None
    } else {
        Some(read_baseline(&PathBuf::from(current_path))?)
    };
    let report: CompatibilityReport = serde_json::from_slice(&fs::read(candidate_path)?)?;
    let candidate = CompatibilityBaseline::from(&report);
    let changes = diff_baselines(current.as_ref(), &candidate);
    for change in &changes {
        println!(
            "{} {}: {} -> {}",
            change.case_id,
            change.tool,
            state(change.before.as_ref()),
            state(change.after.as_ref())
        );
    }
    match command.as_str() {
        "diff" => {
            if arguments.next().is_some() {
                return Err("diff accepts no additional arguments".into());
            }
        }
        "review" => {
            let output = PathBuf::from(arguments.next().ok_or("review requires output path")?);
            let mut reviewed = BTreeSet::new();
            while let Some(flag) = arguments.next() {
                if flag != "--case" {
                    return Err(format!("expected --case, found {flag}").into());
                }
                reviewed.insert(arguments.next().ok_or("--case requires an ID")?);
            }
            let accepted = accept_reviewed_candidate(current.as_ref(), candidate, &reviewed)?;
            write_baseline_atomic(&output, &accepted)?;
            println!("wrote reviewed baseline to {}", output.display());
        }
        _ => return Err(format!("unknown command: {command}").into()),
    }
    Ok(())
}

fn state(observation: Option<&tq_test_support::compatibility::BaselineObservation>) -> String {
    observation.map_or_else(
        || "<absent>".to_owned(),
        |value| {
            format!(
                "{:?}/exit={:?}/error={:?}/results={}",
                value.state,
                value.exit_code,
                value.error_class,
                value.results.len()
            )
        },
    )
}
