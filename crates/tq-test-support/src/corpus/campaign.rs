//! Refreshed and frozen corpus campaign selection.

use std::{
    fs, io,
    path::{Component, Path, PathBuf},
};

use sha2::{Digest, Sha256};
use thiserror::Error;

use super::{ArtifactIdentity, SnapshotManifest, encode_hex};

/// Corpus acquisition mode for a benchmark or compatibility campaign.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum CampaignMode {
    /// Recollect every configured natural source before running.
    #[default]
    Refreshed,
    /// Replay only the artifacts identified by an existing manifest.
    Frozen(PathBuf),
}

impl CampaignMode {
    /// Selects frozen replay when a manifest is supplied, otherwise refreshed mode.
    #[must_use]
    pub fn from_frozen_manifest(manifest: Option<PathBuf>) -> Self {
        manifest.map_or(Self::Refreshed, Self::Frozen)
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
    let manifest: SnapshotManifest = serde_json::from_reader(fs::File::open(manifest_path)?)?;
    verify(cache_root, &manifest.artifacts.download)?;
    verify(cache_root, &manifest.artifacts.source_json)?;
    if let Some(generated) = &manifest.artifacts.generated {
        verify(cache_root, &generated.yaml)?;
        verify(cache_root, &generated.toon)?;
    }
    Ok(FrozenSnapshot {
        manifest,
        manifest_path: manifest_path.to_owned(),
    })
}

fn verify(root: &Path, identity: &ArtifactIdentity) -> Result<(), CampaignError> {
    if identity.path.is_empty()
        || !Path::new(&identity.path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(CampaignError::UnsafePath(identity.path.clone()));
    }
    let path = root.join(&identity.path);
    if !path.is_file() {
        return Err(CampaignError::MissingArtifact(identity.path.clone()));
    }
    let bytes = fs::read(&path)?;
    let actual_size = u64::try_from(bytes.len())
        .map_err(|_| io::Error::other("artifact length does not fit in u64"))?;
    if actual_size != identity.bytes {
        return Err(CampaignError::SizeMismatch {
            path: identity.path.clone(),
            expected: identity.bytes,
            actual: actual_size,
        });
    }
    let actual_digest = encode_hex(&Sha256::digest(&bytes));
    if actual_digest != identity.sha256 {
        return Err(CampaignError::DigestMismatch {
            path: identity.path.clone(),
            expected: identity.sha256.clone(),
            actual: actual_digest,
        });
    }
    Ok(())
}
