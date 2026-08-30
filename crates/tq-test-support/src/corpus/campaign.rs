//! Refreshed and frozen corpus campaign selection.

use std::{
    collections::BTreeMap,
    fs,
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
    time::UNIX_EPOCH,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;
use thiserror::Error;

use super::{ArtifactIdentity, SnapshotManifest, SnapshotState, encode_hex};

/// Corpus acquisition mode for a benchmark or compatibility campaign.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum CampaignMode {
    /// Reuse or prepare the machine-local admitted snapshot.
    #[default]
    Prepared,
    /// Recollect every configured natural source before running.
    Refreshed,
    /// Replay only the artifacts identified by an existing manifest.
    Frozen(PathBuf),
}

impl CampaignMode {
    /// Selects frozen replay when a manifest is supplied, otherwise prepared mode.
    #[must_use]
    pub fn from_frozen_manifest(manifest: Option<PathBuf>) -> Self {
        manifest.map_or(Self::Prepared, Self::Frozen)
    }
}

/// A frozen snapshot whose complete artifact set matches its manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenSnapshot {
    /// Parsed snapshot manifest.
    pub manifest: SnapshotManifest,
    /// Manifest path used for the replay identity.
    pub manifest_path: PathBuf,
}

/// Stable frozen-replay failures.
#[derive(Debug, Error)]
pub enum CampaignError {
    /// Manifest or artifact I/O failed.
    #[error("frozen campaign I/O failed: {0}")]
    Io(#[from] io::Error),
    /// The manifest was not valid JSON for its versioned structure.
    #[error("invalid frozen manifest: {0}")]
    Json(#[from] serde_json::Error),
    /// A recorded artifact path was unsafe.
    #[error("unsafe frozen artifact path: {0}")]
    UnsafePath(String),
    /// A required artifact was absent.
    #[error("frozen artifact is missing: {0}")]
    MissingArtifact(String),
    /// Artifact bytes no longer match their snapshot identity.
    #[error("frozen artifact digest mismatch for {path}: expected {expected}, got {actual}")]
    DigestMismatch {
        /// Cache-relative artifact path.
        path: String,
        /// Manifest digest.
        expected: String,
        /// Observed digest.
        actual: String,
    },
    /// Artifact byte length no longer matches its snapshot identity.
    #[error("frozen artifact size mismatch for {path}: expected {expected}, got {actual}")]
    SizeMismatch {
        /// Cache-relative artifact path.
        path: String,
        /// Manifest byte length.
        expected: u64,
        /// Observed byte length.
        actual: u64,
    },
}

/// Loads a frozen manifest and verifies every recorded artifact before replay.
///
/// # Errors
///
/// Returns an I/O, JSON, unsafe-path, missing-artifact, size, or digest error.
pub fn load_frozen_snapshot(
    manifest_path: &Path,
    cache_root: &Path,
) -> Result<FrozenSnapshot, CampaignError> {
    load_snapshot(manifest_path, cache_root, false)
}

/// Loads a frozen snapshot and forces a complete streaming SHA-256 audit.
///
/// # Errors
///
/// Returns an I/O, JSON, path, missing-artifact, size, or digest error.
pub fn verify_frozen_snapshot(
    manifest_path: &Path,
    cache_root: &Path,
) -> Result<FrozenSnapshot, CampaignError> {
    load_snapshot(manifest_path, cache_root, true)
}

/// Finds the newest admitted manifest for every source in a machine-local cache.
///
/// # Errors
///
/// Returns an I/O or manifest JSON error while scanning the cache.
pub fn discover_latest_validated_manifests(
    cache_root: &Path,
) -> Result<Vec<PathBuf>, CampaignError> {
    let campaigns = cache_root.join("campaigns");
    if !campaigns.is_dir() {
        return Ok(Vec::new());
    }
    let mut latest = BTreeMap::<String, (String, PathBuf)>::new();
    for campaign in directories(&campaigns)? {
        for source in directories(&campaign)? {
            let path = source.join("manifest.json");
            if !path.is_file() {
                continue;
            }
            let manifest: SnapshotManifest = serde_json::from_reader(fs::File::open(&path)?)?;
            if manifest.state != SnapshotState::CrossFormatValidated
                || manifest.artifacts.generated.is_none()
            {
                continue;
            }
            let key = manifest.retrieved_at.clone();
            let replace = latest
                .get(&manifest.source_id)
                .is_none_or(|(current, _)| key > *current);
            if replace {
                latest.insert(manifest.source_id, (key, path));
            }
        }
    }
    Ok(latest.into_values().map(|(_, path)| path).collect())
}

/// Records that preparation just produced and verified every manifest artifact.
/// Later benchmark runs can use metadata fingerprints instead of rehashing them.
///
/// # Errors
///
/// Returns an I/O error when artifact metadata or the cache cannot be written.
pub fn remember_verified_snapshot(
    cache_root: &Path,
    manifest: &SnapshotManifest,
) -> Result<(), CampaignError> {
    let mut cache = read_verification_cache(cache_root);
    remember(cache_root, &manifest.artifacts.download, &mut cache)?;
    remember(cache_root, &manifest.artifacts.source_json, &mut cache)?;
    if let Some(generated) = &manifest.artifacts.generated {
        remember(cache_root, &generated.yaml, &mut cache)?;
        remember(cache_root, &generated.toon, &mut cache)?;
    }
    write_verification_cache(cache_root, &cache)
}

fn load_snapshot(
    manifest_path: &Path,
    cache_root: &Path,
    force: bool,
) -> Result<FrozenSnapshot, CampaignError> {
    let manifest: SnapshotManifest = serde_json::from_reader(fs::File::open(manifest_path)?)?;
    let mut cache = read_verification_cache(cache_root);
    let mut changed = false;
    changed |= verify(cache_root, &manifest.artifacts.download, &mut cache, force)?;
    changed |= verify(
        cache_root,
        &manifest.artifacts.source_json,
        &mut cache,
        force,
    )?;
    if let Some(generated) = &manifest.artifacts.generated {
        changed |= verify(cache_root, &generated.yaml, &mut cache, force)?;
        changed |= verify(cache_root, &generated.toon, &mut cache, force)?;
    }
    if changed {
        write_verification_cache(cache_root, &cache)?;
    }
    Ok(FrozenSnapshot {
        manifest,
        manifest_path: manifest_path.to_owned(),
    })
}

fn verify(
    root: &Path,
    identity: &ArtifactIdentity,
    cache: &mut VerificationCache,
    force: bool,
) -> Result<bool, CampaignError> {
    let path = artifact_path(root, identity)?;
    if !path.is_file() {
        return Err(CampaignError::MissingArtifact(identity.path.clone()));
    }
    let metadata = path.metadata()?;
    let actual_size = metadata.len();
    if actual_size != identity.bytes {
        return Err(CampaignError::SizeMismatch {
            path: identity.path.clone(),
            expected: identity.bytes,
            actual: actual_size,
        });
    }
    let fingerprint = fingerprint(&metadata);
    if !force
        && cache.entries.get(&identity.path).is_some_and(|entry| {
            entry.bytes == identity.bytes
                && entry.sha256 == identity.sha256
                && entry.modified_seconds == fingerprint.0
                && entry.modified_nanos == fingerprint.1
        })
    {
        return Ok(false);
    }
    let actual_digest = hash_file(&path)?;
    if actual_digest != identity.sha256 {
        return Err(CampaignError::DigestMismatch {
            path: identity.path.clone(),
            expected: identity.sha256.clone(),
            actual: actual_digest,
        });
    }
    cache.entries.insert(
        identity.path.clone(),
        VerificationEntry {
            bytes: identity.bytes,
            sha256: identity.sha256.clone(),
            modified_seconds: fingerprint.0,
            modified_nanos: fingerprint.1,
        },
    );
    Ok(true)
}

fn remember(
    root: &Path,
    identity: &ArtifactIdentity,
    cache: &mut VerificationCache,
) -> Result<(), CampaignError> {
    let path = artifact_path(root, identity)?;
    if !path.is_file() {
        return Err(CampaignError::MissingArtifact(identity.path.clone()));
    }
    let metadata = path.metadata()?;
    if metadata.len() != identity.bytes {
        return Err(CampaignError::SizeMismatch {
            path: identity.path.clone(),
            expected: identity.bytes,
            actual: metadata.len(),
        });
    }
    let fingerprint = fingerprint(&metadata);
    cache.entries.insert(
        identity.path.clone(),
        VerificationEntry {
            bytes: identity.bytes,
            sha256: identity.sha256.clone(),
            modified_seconds: fingerprint.0,
            modified_nanos: fingerprint.1,
        },
    );
    Ok(())
}

fn artifact_path(root: &Path, identity: &ArtifactIdentity) -> Result<PathBuf, CampaignError> {
    if identity.path.is_empty()
        || !Path::new(&identity.path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(CampaignError::UnsafePath(identity.path.clone()));
    }
    Ok(root.join(&identity.path))
}

fn hash_file(path: &Path) -> Result<String, io::Error> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 1024 * 1024].into_boxed_slice();
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(encode_hex(&hasher.finalize()))
}

fn directories(root: &Path) -> Result<Vec<PathBuf>, io::Error> {
    let mut paths = fs::read_dir(root)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}

fn fingerprint(metadata: &fs::Metadata) -> (u64, u32) {
    metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map_or((0, 0), |duration| {
            (duration.as_secs(), duration.subsec_nanos())
        })
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct VerificationCache {
    schema_version: u32,
    entries: BTreeMap<String, VerificationEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
struct VerificationEntry {
    bytes: u64,
    sha256: String,
    modified_seconds: u64,
    modified_nanos: u32,
}

fn verification_cache_path(root: &Path) -> PathBuf {
    root.join("verification-cache-v1.json")
}

fn read_verification_cache(root: &Path) -> VerificationCache {
    let path = verification_cache_path(root);
    let Ok(file) = fs::File::open(path) else {
        return VerificationCache {
            schema_version: 1,
            entries: BTreeMap::new(),
        };
    };
    serde_json::from_reader(file).unwrap_or_else(|_| VerificationCache {
        schema_version: 1,
        entries: BTreeMap::new(),
    })
}

fn write_verification_cache(root: &Path, cache: &VerificationCache) -> Result<(), CampaignError> {
    fs::create_dir_all(root)?;
    let mut temporary = NamedTempFile::new_in(root)?;
    serde_json::to_writer_pretty(&mut temporary, cache)?;
    temporary.write_all(b"\n")?;
    temporary.flush()?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(verification_cache_path(root))
        .map_err(|error| error.error)?;
    Ok(())
}
