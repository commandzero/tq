//! Command-line entry point for corpus preparation.

use std::{
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
};

use tq_test_support::corpus::{
    CorpusOrigin, SnapshotManifest, SnapshotState, finalize_generated_representations_with_tq,
    generate_representations_with_tq, inventory_snapshots, prepare_campaign, refresh_campaign,
    validate_generated_representations_with_tq, verify_frozen_snapshot, write_snapshot_manifest,
};

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
        "finalize" => finalize(arguments),
        "inventory" => inventory(arguments),
        "prepare" => prepare(arguments),
        "refresh" => refresh(arguments),
        "verify" => verify(arguments),
        _ => Err(format!("unsupported corpus command: {command}\n{}", usage()).into()),
    }
}

fn finalize(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    let [cache, manifest_path] = arguments else {
        return Err(usage().into());
    };
    let cache = Path::new(cache);
    let manifest_path = Path::new(manifest_path);
    let mut manifest: SnapshotManifest = serde_json::from_reader(fs::File::open(manifest_path)?)?;
    if manifest.state == SnapshotState::CrossFormatValidated {
        println!("{}", serde_json::to_string_pretty(&manifest)?);
        return Ok(());
    }
    if manifest.artifacts.generated.is_some() {
        return Err("source-validated manifest unexpectedly contains generated artifacts".into());
    }

    let source_relative = safe_relative(&manifest.artifacts.source_json.path)?;
    let artifact_root = source_relative
        .parent()
        .ok_or("source artifact path has no parent")?;
    let yaml_relative = artifact_root.join("source.yaml");
    let toon_relative = artifact_root.join("source.toon");
    let yaml_manifest_path = relative_string(&yaml_relative)?;
    let toon_manifest_path = relative_string(&toon_relative)?;
    let generated = finalize_generated_representations_with_tq(
        &tq_binary()?,
        &cache.join(source_relative),
        &cache.join(&yaml_relative),
        &cache.join(&toon_relative),
        &yaml_manifest_path,
        &toon_manifest_path,
    )?;

    manifest.artifacts.generated = Some(generated);
    manifest.validation.yaml_equivalent = Some(true);
    manifest.validation.toon_equivalent = Some(true);
    manifest.state = SnapshotState::CrossFormatValidated;
    write_snapshot_manifest(manifest_path, &manifest)?;
    println!("{}", serde_json::to_string_pretty(&manifest)?);
    Ok(())
}

fn refresh(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    let [sources, cache, campaign] = arguments else {
        return Err(usage().into());
    };
    let refreshed = refresh_campaign(
        Path::new(sources),
        Path::new(cache),
        campaign,
        &tq_binary()?,
    )?;
    print_campaign(&refreshed)
}

fn prepare(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    let [sources, cache, campaign] = arguments else {
        return Err(usage().into());
    };
    let prepared = prepare_campaign(
        Path::new(sources),
        Path::new(cache),
        campaign,
        &tq_binary()?,
    )?;
    print_campaign(&prepared)
}

fn print_campaign(
    campaign: &tq_test_support::corpus::RefreshCampaign,
) -> Result<(), Box<dyn Error>> {
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "campaign_id": campaign.campaign_id,
            "manifests": campaign.manifests,
        }))?
    );
    Ok(())
}

fn generate(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    let [source, yaml, toon] = arguments else {
        return Err(usage().into());
    };
    let yaml_path = PathBuf::from(yaml);
    let toon_path = PathBuf::from(toon);
    let generated = generate_representations_with_tq(
        &tq_binary()?,
        Path::new(source),
        &yaml_path,
        &toon_path,
        file_name(&yaml_path)?,
        file_name(&toon_path)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&generated)?);
    Ok(())
}

fn verify(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    let [cache, manifest_path] = arguments else {
        return Err(usage().into());
    };
    let cache = Path::new(cache);
    let frozen = verify_frozen_snapshot(Path::new(manifest_path), cache)?;
    let generated = frozen
        .manifest
        .artifacts
        .generated
        .as_ref()
        .ok_or("snapshot has no generated representations")?;
    validate_generated_representations_with_tq(
        &tq_binary()?,
        &cache.join(&frozen.manifest.artifacts.source_json.path),
        &cache.join(&generated.yaml.path),
        &cache.join(&generated.toon.path),
    )?;
    println!("{}", serde_json::to_string_pretty(&frozen.manifest)?);
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
    "usage:\n  tq-corpus prepare SOURCES_DIR CACHE_ROOT rapid|standard|large\n  tq-corpus refresh SOURCES_DIR CACHE_ROOT rapid|standard|large\n  tq-corpus generate SOURCE.json OUTPUT.yaml OUTPUT.toon\n  tq-corpus finalize CACHE_ROOT MANIFEST.json\n  tq-corpus verify CACHE_ROOT MANIFEST.json\n  tq-corpus inventory smoke|refreshed|frozen MANIFEST.json..."
}

fn tq_binary() -> Result<PathBuf, Box<dyn Error>> {
    let path = env::var_os("TQ_BIN").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/release/tq"),
        PathBuf::from,
    );
    if !path.is_file() {
        return Err(format!(
            "release tq corpus generator is missing at {}; build tq-cli --release or set TQ_BIN",
            path.display()
        )
        .into());
    }
    Ok(path)
}

fn safe_relative(path: &str) -> Result<&Path, Box<dyn Error>> {
    let path = Path::new(path);
    if path.as_os_str().is_empty()
        || !path
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
    {
        return Err(format!("unsafe cache-relative artifact path: {}", path.display()).into());
    }
    Ok(path)
}

fn relative_string(path: &Path) -> Result<String, Box<dyn Error>> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("artifact path is not UTF-8: {}", path.display()).into())
}

fn file_name(path: &Path) -> Result<&str, Box<dyn Error>> {
    path.file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("output path has no UTF-8 file name: {}", path.display()).into())
}
