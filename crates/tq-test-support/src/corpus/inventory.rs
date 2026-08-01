//! Machine-readable corpus inventory reporting.

use std::{fs, io, path::PathBuf};

use serde::Serialize;
use thiserror::Error;

use super::{
    ArtifactIdentity, CorpusOrigin, GeneratedArtifacts, SnapshotManifest, SnapshotState,
    ValidationIdentity,
};

/// One source snapshot shown by the inventory command.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SnapshotInventory {
    /// Reviewed source identifier.
    pub source_id: String,
    /// Source URL used for this snapshot.
    pub source_url: String,
    /// Campaign identifier.
    pub campaign_id: String,
    /// Manifest lifecycle state.
    pub state: SnapshotState,
    /// Exact download identity.
    pub download: ArtifactIdentity,
    /// Exact source JSON identity.
    pub source_json: ArtifactIdentity,
    /// Generated YAML and TOON identities, when present.
    pub generated: Option<GeneratedArtifacts>,
    /// Natural logical feature count.
    pub logical_records: u64,
    /// Source and generated validation status.
    pub validation: ValidationIdentity,
    /// Manifest path supplying this row.
    pub manifest: PathBuf,
}

/// Corpus inventory with a mandatory campaign-origin label.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CorpusInventory {
    /// Smoke, refreshed, or frozen provenance class.
    pub origin: CorpusOrigin,
    /// Snapshot rows in requested manifest order.
    pub snapshots: Vec<SnapshotInventory>,
}

/// Stable inventory failures.
#[derive(Debug, Error)]
pub enum InventoryError {
    /// Manifest I/O failed.
    #[error("inventory I/O failed: {0}")]
    Io(#[from] io::Error),
    /// Manifest JSON was invalid.
    #[error("inventory manifest JSON failed: {0}")]
    Json(#[from] serde_json::Error),
}

/// Builds an inventory from versioned snapshot manifests.
///
/// # Errors
///
/// Returns an I/O or manifest decoding error.
pub fn inventory_snapshots(
    origin: CorpusOrigin,
    manifests: &[PathBuf],
) -> Result<CorpusInventory, InventoryError> {
    let mut snapshots = Vec::with_capacity(manifests.len());
    for path in manifests {
        let manifest: SnapshotManifest = serde_json::from_reader(fs::File::open(path)?)?;
        snapshots.push(SnapshotInventory {
            source_id: manifest.source_id,
            source_url: manifest.request.requested_url,
            campaign_id: manifest.campaign_id,
            state: manifest.state,
            download: manifest.artifacts.download,
            source_json: manifest.artifacts.source_json,
            generated: manifest.artifacts.generated,
            logical_records: manifest.document.logical_records,
            validation: manifest.validation,
            manifest: path.clone(),
        });
    }
    Ok(CorpusInventory { origin, snapshots })
}
