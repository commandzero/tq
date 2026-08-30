//! End-to-end natural source refresh and cross-format preparation.

use std::{
    fs,
    io::{self, Read as _},
    path::{Path, PathBuf},
};

use serde::Deserialize;
use thiserror::Error;

use super::{
    ArchiveIdentity, ArtifactIdentity, FetchError, FetchOutcome, FetchRequest, ManifestError,
    Provenance, RequestIdentity, ReqwestTransport, SnapshotState, SourceSnapshotInput,
    build_source_snapshot, extract_zip_member, fetch, finalize_generated_representations_with_tq,
    generate_representations_with_tq, remember_verified_snapshot, write_snapshot_manifest,
};

/// Completed refreshed campaign.
#[derive(Clone, Debug)]
pub struct RefreshCampaign {
    /// Filesystem-safe UTC campaign ID.
    pub campaign_id: String,
    /// Generated snapshot manifests.
    pub manifests: Vec<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct SourceDefinition {
    id: String,
    campaigns: Vec<String>,
    fetch: SourceFetch,
    archive: Option<SourceArchive>,
    provenance: Provenance,
}

#[derive(Debug, Deserialize)]
struct SourceFetch {
    url: String,
    redirect_limit: usize,
    expected_content_types: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SourceArchive {
    format: String,
    member: String,
}

/// Refresh orchestration failures.
#[derive(Debug, Error)]
pub enum RefreshError {
    /// Registry or artifact I/O.
    #[error("refresh I/O failed: {0}")]
    Io(#[from] io::Error),
    /// Invalid source registry JSON.
    #[error("invalid source registry: {0}")]
    Json(#[from] serde_json::Error),
    /// Network/protocol/integrity failure.
    #[error(transparent)]
    Fetch(#[from] FetchError),
    /// Archive validation/decompression failure.
    #[error(transparent)]
    Archive(#[from] super::ArchiveError),
    /// Source validation or manifest failure.
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    /// Cross-format preparation failure.
    #[error(transparent)]
    Conversion(#[from] super::ConversionError),
    /// A source registry uses an unsupported archive format.
    #[error("unsupported archive format for {source_id}: {format}")]
    UnsupportedArchive {
        /// Source ID.
        source_id: String,
        /// Archive format.
        format: String,
    },
    /// Conditional response cannot occur for a unique campaign destination.
    #[error("unexpected not-modified response for fresh campaign source {0}")]
    UnexpectedNotModified(String),
}

/// Recollects every registry source assigned to `campaign`, preserving natural
/// bytes beneath a unique cache directory and producing JSON/YAML/TOON.
///
/// # Errors
///
/// Returns registry, network, archive, validation, conversion, or manifest
/// errors. Source validation is published before cross-format preparation, so
/// an expensive conversion failure retains a resumable provenance record.
pub fn refresh_campaign(
    source_directory: &Path,
    cache_root: &Path,
    campaign: &str,
    tq: &Path,
) -> Result<RefreshCampaign, RefreshError> {
    let timestamp = jiff::Timestamp::now();
    let campaign_id = timestamp.to_string().replace(':', "-");
    let mut definitions = source_definitions(source_directory)?;
    definitions.retain(|definition| definition.campaigns.iter().any(|value| value == campaign));
    let mut manifests = Vec::new();
    for definition in definitions {
        manifests.push(refresh_source(
            cache_root,
            &campaign_id,
            timestamp.to_string(),
            &definition,
            tq,
        )?);
    }
    Ok(RefreshCampaign {
        campaign_id,
        manifests,
    })
}

/// Reuses the newest admitted snapshot for every configured source, resumes an
/// interrupted local snapshot, and downloads only sources absent from the cache.
///
/// # Errors
///
/// Returns registry, network, archive, validation, conversion, or manifest
/// errors while preparing a missing source.
pub fn prepare_campaign(
    source_directory: &Path,
    cache_root: &Path,
    campaign: &str,
    tq: &Path,
) -> Result<RefreshCampaign, RefreshError> {
    let mut definitions = source_definitions(source_directory)?;
    definitions.retain(|definition| definition.campaigns.iter().any(|value| value == campaign));
    let timestamp = jiff::Timestamp::now();
    let campaign_id = timestamp.to_string().replace(':', "-");
    let mut manifests = Vec::new();
    for definition in definitions {
        let manifest = match latest_manifest(cache_root, &definition.id)? {
            Some(path) => {
                let existing: super::SnapshotManifest =
                    serde_json::from_reader(fs::File::open(&path)?)?;
                if existing.state == SnapshotState::CrossFormatValidated
                    && existing.artifacts.generated.is_some()
                {
                    path
                } else {
                    resume_snapshot(cache_root, &path, existing, tq)?
                }
            }
            None => refresh_source(
                cache_root,
                &campaign_id,
                timestamp.to_string(),
                &definition,
                tq,
            )?,
        };
        manifests.push(manifest);
    }
    Ok(RefreshCampaign {
        campaign_id: "machine-cache".to_owned(),
        manifests,
    })
}

fn source_definitions(directory: &Path) -> Result<Vec<SourceDefinition>, RefreshError> {
    let mut paths = fs::read_dir(directory)?
        .map(|entry| entry.map(|value| value.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| path.extension().is_some_and(|value| value == "json"));
    paths.sort();
    paths
        .into_iter()
        .map(|path| Ok(serde_json::from_reader(fs::File::open(path)?)?))
        .collect()
}

fn refresh_source(
    cache_root: &Path,
    campaign_id: &str,
    retrieved_at: String,
    definition: &SourceDefinition,
    tq: &Path,
) -> Result<PathBuf, RefreshError> {
    let relative_root = PathBuf::from("campaigns")
        .join(campaign_id)
        .join(&definition.id);
    let source_root = cache_root.join(&relative_root);
    fs::create_dir_all(&source_root)?;
    let download_name = if definition.archive.is_some() {
        "source.zip"
    } else {
        "source.geojson"
    };
    let download_relative = relative_root.join(download_name);
    let download_file = cache_root.join(&download_relative);
    let outcome = fetch(
        &ReqwestTransport,
        &FetchRequest {
            url: definition.fetch.url.clone(),
            expected_content_types: definition.fetch.expected_content_types.clone(),
            redirect_limit: definition.fetch.redirect_limit,
            if_none_match: None,
            if_modified_since: None,
            expected_sha256: None,
        },
        &download_file,
    )?;
    let FetchOutcome::Downloaded(metadata) = outcome else {
        return Err(RefreshError::UnexpectedNotModified(definition.id.clone()));
    };
    let download = ArtifactIdentity {
        path: relative_string(&download_relative)?,
        bytes: metadata.bytes,
        sha256: metadata.sha256,
    };
    let (source_file, source_relative, archive) = if let Some(archive) = &definition.archive {
        if archive.format != "zip" {
            return Err(RefreshError::UnsupportedArchive {
                source_id: definition.id.clone(),
                format: archive.format.clone(),
            });
        }
        let source_relative = relative_root.join("source.geojson");
        let source_file = cache_root.join(&source_relative);
        extract_zip_member(
            &download_file,
            &archive.member,
            &source_file,
            16 * 1024 * 1024 * 1024,
            None,
        )?;
        (
            source_file,
            source_relative,
            Some(ArchiveIdentity {
                format: archive.format.clone(),
                member: archive.member.clone(),
            }),
        )
    } else {
        (download_file, download_relative.clone(), None)
    };
    let mut manifest = build_source_snapshot(SourceSnapshotInput {
        campaign_id: campaign_id.to_owned(),
        source_id: definition.id.clone(),
        retrieved_at,
        request: RequestIdentity {
            requested_url: definition.fetch.url.clone(),
            final_url: metadata.final_url,
            status: metadata.status,
            content_type: metadata.content_type,
            etag: metadata.etag,
            last_modified: metadata.last_modified,
        },
        archive,
        download,
        source_json_file: source_file.clone(),
        source_json_path: relative_string(&source_relative)?,
        provenance: definition.provenance.clone(),
    })?;
    let manifest_path = source_root.join("manifest.json");
    write_snapshot_manifest(&manifest_path, &manifest)?;
    let yaml_relative = relative_root.join("source.yaml");
    let toon_relative = relative_root.join("source.toon");
    let generated = generate_representations_with_tq(
        tq,
        &source_file,
        &cache_root.join(&yaml_relative),
        &cache_root.join(&toon_relative),
        &relative_string(&yaml_relative)?,
        &relative_string(&toon_relative)?,
    )?;
    manifest.artifacts.generated = Some(generated);
    manifest.validation.yaml_equivalent = Some(true);
    manifest.validation.toon_equivalent = Some(true);
    manifest.state = SnapshotState::CrossFormatValidated;
    write_snapshot_manifest(&manifest_path, &manifest)?;
    remember_verified_snapshot(cache_root, &manifest)
        .map_err(|error| io::Error::other(error.to_string()))?;
    Ok(manifest_path)
}

fn latest_manifest(cache_root: &Path, source_id: &str) -> Result<Option<PathBuf>, RefreshError> {
    let campaigns = cache_root.join("campaigns");
    if !campaigns.is_dir() {
        return Ok(None);
    }
    let mut latest = None::<(String, PathBuf)>;
    for campaign in fs::read_dir(campaigns)? {
        let path = campaign?.path().join(source_id).join("manifest.json");
        if !path.is_file() {
            continue;
        }
        let manifest: super::SnapshotManifest = serde_json::from_reader(fs::File::open(&path)?)?;
        let replace = latest
            .as_ref()
            .is_none_or(|(retrieved_at, _)| manifest.retrieved_at > *retrieved_at);
        if replace {
            latest = Some((manifest.retrieved_at, path));
        }
    }
    Ok(latest.map(|(_, path)| path))
}

fn resume_snapshot(
    cache_root: &Path,
    manifest_path: &Path,
    mut manifest: super::SnapshotManifest,
    tq: &Path,
) -> Result<PathBuf, RefreshError> {
    let source_relative = safe_relative(&manifest.artifacts.source_json.path)?;
    let artifact_root = source_relative
        .parent()
        .ok_or_else(|| io::Error::other("source artifact path has no parent"))?;
    let yaml_relative = artifact_root.join("source.yaml");
    let toon_relative = artifact_root.join("source.toon");
    let yaml_path = cache_root.join(&yaml_relative);
    let toon_path = cache_root.join(&toon_relative);
    let yaml_manifest_path = relative_string(&yaml_relative)?;
    let toon_manifest_path = relative_string(&toon_relative)?;
    let source_path = cache_root.join(source_relative);
    let generated =
        if yaml_path.is_file() && toon_path.is_file() && is_lossless_yaml_profile(&yaml_path)? {
            match finalize_generated_representations_with_tq(
                tq,
                &source_path,
                &yaml_path,
                &toon_path,
                &yaml_manifest_path,
                &toon_manifest_path,
            ) {
                Ok(generated) => generated,
                Err(_) => generate_representations_with_tq(
                    tq,
                    &source_path,
                    &yaml_path,
                    &toon_path,
                    &yaml_manifest_path,
                    &toon_manifest_path,
                )?,
            }
        } else {
            generate_representations_with_tq(
                tq,
                &source_path,
                &yaml_path,
                &toon_path,
                &yaml_manifest_path,
                &toon_manifest_path,
            )?
        };
    manifest.artifacts.generated = Some(generated);
    manifest.validation.yaml_equivalent = Some(true);
    manifest.validation.toon_equivalent = Some(true);
    manifest.state = SnapshotState::CrossFormatValidated;
    write_snapshot_manifest(manifest_path, &manifest)?;
    remember_verified_snapshot(cache_root, &manifest)
        .map_err(|error| io::Error::other(error.to_string()))?;
    Ok(manifest_path.to_owned())
}

fn is_lossless_yaml_profile(path: &Path) -> Result<bool, io::Error> {
    let mut file = fs::File::open(path)?;
    let mut prefix = [0_u8; 64];
    let read = file.read(&mut prefix)?;
    Ok(prefix[..read]
        .iter()
        .copied()
        .find(|byte| !byte.is_ascii_whitespace())
        .is_some_and(|byte| matches!(byte, b'{' | b'[')))
}

fn safe_relative(path: &str) -> Result<&Path, RefreshError> {
    let path = Path::new(path);
    if path.as_os_str().is_empty()
        || !path
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
    {
        return Err(io::Error::other(format!(
            "unsafe cache-relative artifact path: {}",
            path.display()
        ))
        .into());
    }
    Ok(path)
}

fn relative_string(path: &Path) -> Result<String, io::Error> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| io::Error::other(format!("non-UTF-8 cache path: {}", path.display())))
}
