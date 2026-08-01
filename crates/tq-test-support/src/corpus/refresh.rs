//! End-to-end natural source refresh and cross-format preparation.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use thiserror::Error;

use super::{
    ArchiveIdentity, ArtifactIdentity, FetchError, FetchOutcome, FetchRequest, ManifestError,
    Provenance, RequestIdentity, ReqwestTransport, SnapshotState, SourceSnapshotInput,
    build_source_snapshot, extract_zip_member, fetch, generate_representations,
    validate_generated_representations, write_snapshot_manifest,
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
        )?);
    }
    Ok(RefreshCampaign {
        campaign_id,
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
    let generated = generate_representations(
        &source_file,
        &cache_root.join(&yaml_relative),
        &cache_root.join(&toon_relative),
        &relative_string(&yaml_relative)?,
        &relative_string(&toon_relative)?,
    )?;
    validate_generated_representations(
        &source_file,
        &cache_root.join(&yaml_relative),
        &cache_root.join(&toon_relative),
    )?;
    manifest.artifacts.generated = Some(generated);
    manifest.validation.yaml_equivalent = Some(true);
    manifest.validation.toon_equivalent = Some(true);
    manifest.state = SnapshotState::CrossFormatValidated;
    write_snapshot_manifest(&manifest_path, &manifest)?;
    Ok(manifest_path)
}

fn relative_string(path: &Path) -> Result<String, io::Error> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| io::Error::other(format!("non-UTF-8 cache path: {}", path.display())))
}
