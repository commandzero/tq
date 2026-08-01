//! Command-line entry point for corpus preparation.

use std::{
    env,
    error::Error,
    path::{Path, PathBuf},
};

use tq_test_support::corpus::{CorpusOrigin, generate_representations, inventory_snapshots};

fn main() {
    if let Err(error) = run() {
        eprintln!("tq-corpus: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let Some((command, arguments)) = arguments.split_first() else {
        return Err(usage().into());
    };
    match command.as_str() {
        "generate" => generate(arguments),
        "inventory" => inventory(arguments),
        _ => Err(format!("unsupported corpus command: {command}\n{}", usage()).into()),
    }
}

fn generate(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    let [source, yaml, toon] = arguments else {
        return Err(usage().into());
    };
    let yaml_path = PathBuf::from(yaml);
    let toon_path = PathBuf::from(toon);
    let generated = generate_representations(
        Path::new(source),
        &yaml_path,
        &toon_path,
        file_name(&yaml_path)?,
        file_name(&toon_path)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&generated)?);
    Ok(())
}

fn inventory(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    let Some((origin, manifests)) = arguments.split_first() else {
        return Err(usage().into());
    };
    if manifests.is_empty() {
        return Err("inventory requires at least one snapshot manifest".into());
    }
    let origin = match origin.as_str() {
        "smoke" => CorpusOrigin::Smoke,
        "refreshed" => CorpusOrigin::Refreshed,
        "frozen" => CorpusOrigin::Frozen,
        _ => return Err(format!("invalid inventory origin: {origin}").into()),
    };
    let manifests = manifests.iter().map(PathBuf::from).collect::<Vec<_>>();
    let inventory = inventory_snapshots(origin, &manifests)?;
    println!("{}", serde_json::to_string_pretty(&inventory)?);
    Ok(())
}

fn usage() -> &'static str {
    "usage:\n  tq-corpus generate SOURCE.json OUTPUT.yaml OUTPUT.toon\n  tq-corpus inventory smoke|refreshed|frozen MANIFEST.json..."
}

fn file_name(path: &Path) -> Result<&str, Box<dyn Error>> {
    path.file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("output path has no UTF-8 file name: {}", path.display()).into())
}
