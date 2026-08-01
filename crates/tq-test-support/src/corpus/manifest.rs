//! Versioned corpus snapshot manifests.

use std::{
    fs,
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;
use thiserror::Error;

use super::{GeoJsonError, encode_hex, validate_geojson};

/// Lifecycle state of a snapshot manifest.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SnapshotState {
    /// Source JSON passed byte and structural validation.
    SourceValidated,
    /// Generated YAML and TOON also passed ordered semantic validation.
    CrossFormatValidated,
}

/// Exact identity of one stored artifact.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArtifactIdentity {
    /// Cache-relative artifact path.
    pub path: String,
    /// Exact byte length.
    pub bytes: u64,
    /// Lowercase SHA-256 digest.
    pub sha256: String,
}

/// HTTP request and response identity for a snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RequestIdentity {
    /// Configured source URL.
    pub requested_url: String,
    /// Final URL after redirects.
    pub final_url: String,
    /// Final HTTP status.
    pub status: u16,
    /// Normalized response media type.
    pub content_type: String,
    /// Response `ETag`, when supplied.
    pub etag: Option<String>,
    /// Response `Last-Modified`, when supplied.
    pub last_modified: Option<String>,
}

/// Archive member identity for compressed sources.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArchiveIdentity {
    /// Archive container format.
    pub format: String,
    /// Exact extracted member name.
    pub member: String,
}

/// Dataset license identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LicenseIdentity {
    /// Human-readable license name.
    pub name: String,
    /// Authoritative license reference.
    pub url: String,
}

/// Snapshot provenance copied from the reviewed source registry.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Provenance {
    /// Dataset title.
    pub title: String,
    /// Dataset publisher.
    pub publisher: String,
    /// Dataset landing page.
    pub landing_page: String,
    /// Dataset license.
    pub license: LicenseIdentity,
}

/// Generated cross-format artifact identities.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GeneratedArtifacts {
    /// Generated YAML document.
    pub yaml: ArtifactIdentity,
    /// Generated TOON document.
    pub toon: ArtifactIdentity,
}

/// Artifact identities attached to one snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArtifactSet {
    /// Exact downloaded bytes, compressed when the source is an archive.
    pub download: ArtifactIdentity,
    /// Validated natural JSON document.
    pub source_json: ArtifactIdentity,
    /// Generated representations after cross-format validation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated: Option<GeneratedArtifacts>,
}

/// Validated logical document shape.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DocumentIdentity {
    /// Root `GeoJSON` type.
    pub root_type: String,
    /// Natural count of root features.
    pub logical_records: u64,
}

/// Validation state for source and generated representations.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ValidationIdentity {
    /// Source JSON validation status.
    pub source_json: String,
    /// YAML ordered-semantic equivalence, when generated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yaml_equivalent: Option<bool>,
    /// TOON ordered-semantic equivalence, when generated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toon_equivalent: Option<bool>,
}

/// Machine-readable identity of one natural corpus snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SnapshotManifest {
    /// Manifest schema version.
    pub schema_version: u32,
    /// Current manifest lifecycle state.
    pub state: SnapshotState,
    /// Campaign that acquired the snapshot.
    pub campaign_id: String,
    /// Reviewed source registry identifier.
    pub source_id: String,
    /// UTC RFC 3339 retrieval instant.
    pub retrieved_at: String,
    /// HTTP acquisition identity.
    pub request: RequestIdentity,
    /// Archive identity, or null for direct JSON.
    pub archive: Option<ArchiveIdentity>,
    /// Exact artifact identities.
    pub artifacts: ArtifactSet,
    /// Validated document shape and record count.
    pub document: DocumentIdentity,
    /// Validation state.
    pub validation: ValidationIdentity,
    /// Reviewed dataset provenance.
    pub provenance: Provenance,
}

/// Inputs needed to validate source JSON and populate its snapshot manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceSnapshotInput {
    /// Campaign identifier.
    pub campaign_id: String,
    /// Reviewed source identifier.
    pub source_id: String,
    /// UTC RFC 3339 retrieval instant.
    pub retrieved_at: String,
    /// HTTP identity.
    pub request: RequestIdentity,
    /// Optional source archive identity.
    pub archive: Option<ArchiveIdentity>,
    /// Downloaded artifact identity.
    pub download: ArtifactIdentity,
    /// Filesystem path of the natural source JSON.
    pub source_json_file: PathBuf,
    /// Cache-relative path recorded in the manifest.
    pub source_json_path: String,
    /// Reviewed provenance.
    pub provenance: Provenance,
}

/// Stable failures while populating or writing a snapshot manifest.
#[derive(Debug, Error)]
pub enum ManifestError {
    /// Source structural validation failed.
    #[error(transparent)]
    GeoJson(#[from] GeoJsonError),
    /// A local file could not be read or written.
    #[error("manifest I/O failed: {0}")]
    Io(#[from] io::Error),
    /// A manifest could not be serialized.
    #[error("manifest serialization failed: {0}")]
    Serialize(#[from] serde_json::Error),
    /// A temporary manifest could not be atomically installed.
    #[error("could not atomically install manifest: {0}")]
    Persist(#[from] tempfile::PersistError),
    /// Retrieval time was not an RFC 3339 timestamp.
    #[error("invalid retrieval timestamp: {0}")]
    InvalidRetrievedAt(String),
    /// An artifact path was absolute or contained traversal.
    #[error("artifact path must be cache-relative and traversal-free: {0}")]
    UnsafeArtifactPath(String),
}

/// Validates natural source JSON and builds a source-stage manifest.
///
/// Hashing, exact byte counting, structural validation, and natural feature
/// counting occur in a single bounded-memory pass over the source file.
///
/// # Errors
///
/// Returns a classified timestamp, path, I/O, or `GeoJSON` validation error.
pub fn build_source_snapshot(
    input: SourceSnapshotInput,
) -> Result<SnapshotManifest, ManifestError> {
    input
        .retrieved_at
        .parse::<jiff::Timestamp>()
        .map_err(|error| ManifestError::InvalidRetrievedAt(error.to_string()))?;
    validate_relative_path(&input.download.path)?;
    validate_relative_path(&input.source_json_path)?;

    let source_file = fs::File::open(&input.source_json_file)?;
    let mut hashing_reader = HashingReader::new(source_file);
    let document = validate_geojson(&mut hashing_reader)?;
    let source_json = ArtifactIdentity {
        path: input.source_json_path,
        bytes: hashing_reader.bytes,
        sha256: encode_hex(&hashing_reader.hasher.finalize()),
    };

    Ok(SnapshotManifest {
        schema_version: 1,
        state: SnapshotState::SourceValidated,
        campaign_id: input.campaign_id,
        source_id: input.source_id,
        retrieved_at: input.retrieved_at,
        request: input.request,
        archive: input.archive,
        artifacts: ArtifactSet {
            download: input.download,
            source_json,
            generated: None,
        },
        document: DocumentIdentity {
            root_type: document.root_type,
            logical_records: document.logical_records,
        },
        validation: ValidationIdentity {
            source_json: "valid".to_owned(),
            yaml_equivalent: None,
            toon_equivalent: None,
        },
        provenance: input.provenance,
    })
}

/// Writes a newline-terminated JSON manifest through an atomic rename.
///
/// # Errors
///
/// Returns a serialization or filesystem error without replacing a prior
/// manifest until the complete new manifest is durable.
pub fn write_snapshot_manifest(
    destination: &Path,
    manifest: &SnapshotManifest,
) -> Result<(), ManifestError> {
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let mut temporary = NamedTempFile::new_in(parent)?;
    serde_json::to_writer_pretty(&mut temporary, manifest)?;
    temporary.write_all(b"\n")?;
    temporary.flush()?;
    temporary.as_file().sync_all()?;
    temporary.persist(destination)?;
    Ok(())
}

fn validate_relative_path(path: &str) -> Result<(), ManifestError> {
    if path.is_empty()
        || !Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(ManifestError::UnsafeArtifactPath(path.to_owned()));
    }
    Ok(())
}

struct HashingReader<R> {
    inner: R,
    hasher: Sha256,
    bytes: u64,
}

impl<R> HashingReader<R> {
    fn new(inner: R) -> Self {
        Self {
            inner,
            hasher: Sha256::new(),
            bytes: 0,
        }
    }
}

impl<R: Read> Read for HashingReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let read = self.inner.read(buffer)?;
        self.hasher.update(&buffer[..read]);
        let read_bytes =
            u64::try_from(read).map_err(|_| io::Error::other("read length does not fit in u64"))?;
        self.bytes = self
            .bytes
            .checked_add(read_bytes)
            .ok_or_else(|| io::Error::other("artifact byte count overflow"))?;
        Ok(read)
    }
}
