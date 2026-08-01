//! Explicitly labeled local smoke-corpus discovery.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::{ArtifactIdentity, DocumentIdentity, GeoJsonError, encode_hex, validate_geojson};

/// Provenance class attached to every corpus-backed report.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CorpusOrigin {
    /// Local development snapshots from `examples/`.
    Smoke,
    /// Naturally recollected public source snapshots.
    Refreshed,
    /// Explicit digest-verified replay of a prior snapshot.
    Frozen,
}

/// One validated local smoke snapshot.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SmokeSnapshot {
    /// Stable identifier derived from the file stem.
    pub source_id: String,
    /// Local source path.
    pub file: PathBuf,
    /// Exact byte identity.
    pub artifact: ArtifactIdentity,
    /// Validated natural shape and feature count.
    pub document: DocumentIdentity,
}

/// A smoke corpus that cannot be mistaken for a refreshed campaign.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SmokeCorpus {
    /// Required report label.
    pub origin: CorpusOrigin,
    /// Validated snapshots in lexical path order.
    pub snapshots: Vec<SmokeSnapshot>,
}

/// Stable smoke discovery failures.
#[derive(Debug, Error)]
pub enum SmokeError {
    /// Directory or file I/O failed.
    #[error("smoke corpus I/O failed: {0}")]
    Io(#[from] io::Error),
    /// A smoke snapshot was not valid `GeoJSON`.
    #[error("invalid smoke snapshot {path}: {source}")]
    GeoJson {
        /// Snapshot path.
        path: PathBuf,
        /// Structural validation failure.
        source: GeoJsonError,
    },
    /// A snapshot file name was not valid UTF-8.
    #[error("smoke snapshot has no UTF-8 file stem: {0}")]
    InvalidName(PathBuf),
}

/// Discovers and validates local `.geojson` example snapshots.
///
/// # Errors
///
/// Returns an I/O, naming, or structural-validation error. Non-GeoJSON files
/// are ignored rather than treated as generated benchmark inputs.
pub fn discover_smoke_corpus(examples_dir: &Path) -> Result<SmokeCorpus, SmokeError> {
    let mut paths = fs::read_dir(examples_dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| {
        path.extension()
            .is_some_and(|extension| extension == "geojson")
    });
    paths.sort();

    let mut snapshots = Vec::with_capacity(paths.len());
    for path in paths {
        let bytes = fs::read(&path)?;
        let document =
            validate_geojson(bytes.as_slice()).map_err(|source| SmokeError::GeoJson {
                path: path.clone(),
                source,
            })?;
        let source_id = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| SmokeError::InvalidName(path.clone()))?
            .replace('_', "-");
        snapshots.push(SmokeSnapshot {
            source_id,
            file: path.clone(),
            artifact: ArtifactIdentity {
                path: path.to_string_lossy().into_owned(),
                bytes: u64::try_from(bytes.len())
                    .map_err(|_| io::Error::other("smoke length does not fit in u64"))?,
                sha256: encode_hex(&Sha256::digest(&bytes)),
            },
            document: DocumentIdentity {
                root_type: document.root_type,
                logical_records: document.logical_records,
            },
        });
    }
    Ok(SmokeCorpus {
        origin: CorpusOrigin::Smoke,
        snapshots,
    })
}
