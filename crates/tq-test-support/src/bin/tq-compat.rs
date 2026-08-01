//! Compatibility campaign command-line driver.

use std::{env, fs, path::PathBuf, process::ExitCode, time::Duration};

use tq_test_support::compatibility::{
    CampaignProfile, ExecutableConfig, FinalStatus, load_catalog, run_campaign,
};

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("tq-compat: {error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut profile = CampaignProfile::Smoke;
    let mut json_path = None;
    let mut timeout = Duration::from_secs(10);
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "run" => {}
            "--profile" => {
                let value = arguments.next().ok_or("--profile requires a value")?;
                profile = match value.as_str() {
                    "smoke" => CampaignProfile::Smoke,
                    "full" => CampaignProfile::Full,
                    _ => return Err(format!("unknown profile: {value}").into()),
                };
            }
            "--json" => {
                json_path = Some(PathBuf::from(
                    arguments.next().ok_or("--json requires a path")?,
                ));
            }
            "--timeout-seconds" => {
                let seconds = arguments
                    .next()
                    .ok_or("--timeout-seconds requires a value")?
                    .parse::<u64>()?;
                timeout = Duration::from_secs(seconds);
            }
            "-h" | "--help" => {
                println!(
                    "Usage: tq-compat run [--profile smoke|full] [--json PATH] [--timeout-seconds N]"
                );
                return Ok(ExitCode::SUCCESS);
            }
            value => return Err(format!("unknown argument: {value}").into()),
        }
    }

    let catalog = load_catalog(&root.join("compatibility/cases"))?;
    let report = run_campaign(
        &catalog,
        profile,
        &ExecutableConfig::from_env(),
        &root,
        timeout,
    )?;
    if let Some(path) = json_path {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut bytes = serde_json::to_vec_pretty(&report)?;
        bytes.push(b'\n');
        fs::write(path, bytes)?;
    }
    print!("{}", report.render_human());
    Ok(if report.final_status == FinalStatus::Failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}
