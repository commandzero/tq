//! Atomic acquisition of natural corpus artifacts.

use std::{
    fs,
    io::{self, Read, Write},
    path::{Component, Path},
};

use reqwest::header::{CONTENT_TYPE, ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};
use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;
use thiserror::Error;
use zip::ZipArchive;

mod campaign;
mod conversion;
mod geojson;
mod inventory;
mod manifest;
mod smoke;

pub use campaign::{CampaignError, CampaignMode, FrozenSnapshot, load_frozen_snapshot};
pub use conversion::{
    ConversionError, DifferenceKind, SemanticDifference, compare_ordered, generate_representations,
    validate_generated_representations,
};
pub use geojson::{GeoJsonError, GeoJsonMetadata, validate_geojson};
pub use inventory::{CorpusInventory, InventoryError, SnapshotInventory, inventory_snapshots};
pub use manifest::{
    ArchiveIdentity, ArtifactIdentity, ArtifactSet, DocumentIdentity, GeneratedArtifacts,
    LicenseIdentity, ManifestError, Provenance, RequestIdentity, SnapshotManifest, SnapshotState,
    SourceSnapshotInput, ValidationIdentity, build_source_snapshot, write_snapshot_manifest,
};
pub use smoke::{CorpusOrigin, SmokeCorpus, SmokeError, SmokeSnapshot, discover_smoke_corpus};

/// A source download request, including cache validators and integrity policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FetchRequest {
    /// Public source URL.
    pub url: String,
    /// Accepted media types, without parameters.
    pub expected_content_types: Vec<String>,
    /// Maximum redirects followed by the concrete transport.
    pub redirect_limit: usize,
    /// Optional `ETag` from an existing snapshot.
    pub if_none_match: Option<String>,
    /// Optional `Last-Modified` value from an existing snapshot.
    pub if_modified_since: Option<String>,
    /// Optional lowercase SHA-256 required for the downloaded body.
    pub expected_sha256: Option<String>,
}

/// Transport-level request passed to an HTTP implementation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpRequest {
    /// Public source URL.
    pub url: String,
    /// Maximum redirects to follow.
    pub redirect_limit: usize,
    /// Optional `If-None-Match` request value.
    pub if_none_match: Option<String>,
    /// Optional `If-Modified-Since` request value.
    pub if_modified_since: Option<String>,
}

/// Streaming HTTP response used by the atomic fetcher.
pub struct HttpResponse {
    /// Final HTTP status.
    pub status: u16,
    /// URL after redirects.
    pub final_url: String,
    /// `Content-Type` header, when supplied.
    pub content_type: Option<String>,
    /// `ETag` response validator, when supplied.
    pub etag: Option<String>,
    /// `Last-Modified` response validator, when supplied.
    pub last_modified: Option<String>,
    /// Streaming response body.
    pub body: Box<dyn Read + Send>,
}

/// Pluggable synchronous transport used by corpus acquisition.
pub trait Transport {
    /// Executes one conditional GET.
    ///
    /// # Errors
    ///
    /// Returns a classified transport error when a response cannot be obtained.
    fn get(&self, request: &HttpRequest) -> Result<HttpResponse, FetchError>;
}

/// Metadata calculated before a completed artifact is atomically admitted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DownloadMetadata {
    /// UTC instant at which acquisition completed.
    pub retrieved_at: String,
    /// URL after redirects.
    pub final_url: String,
    /// Successful HTTP status.
    pub status: u16,
    /// Normalized media type without parameters.
    pub content_type: String,
    /// Response `ETag`, when supplied.
    pub etag: Option<String>,
    /// Response `Last-Modified` value, when supplied.
    pub last_modified: Option<String>,
    /// Exact number of downloaded bytes.
    pub bytes: u64,
    /// Lowercase SHA-256 of the downloaded bytes.
    pub sha256: String,
}

/// Result of a conditional corpus fetch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FetchOutcome {
    /// A new body was verified and atomically installed.
    Downloaded(DownloadMetadata),
    /// The source reported that the existing snapshot remains current.
    NotModified {
        /// URL after redirects.
        final_url: String,
        /// Response `ETag`, when supplied.
        etag: Option<String>,
        /// Response `Last-Modified` value, when supplied.
        last_modified: Option<String>,
    },
}

/// Stable failure classes for corpus fetching.
#[derive(Debug, Error)]
pub enum FetchError {
    /// The HTTP transport could not complete the request.
    #[error("transport failed: {0}")]
    Transport(String),
    /// The source returned an unusable status.
    #[error("unexpected HTTP status {0}")]
    UnexpectedStatus(u16),
    /// The response omitted its required Content-Type.
    #[error("response omitted Content-Type")]
    MissingContentType,
    /// The response media type was not allowed by the source registry.
    #[error("unexpected Content-Type {actual}; expected one of {expected:?}")]
    UnexpectedContentType {
        /// Normalized response media type.
        actual: String,
        /// Accepted normalized media types.
        expected: Vec<String>,
    },
    /// The response body or local filesystem failed.
    #[error("I/O failed: {0}")]
    Io(#[from] io::Error),
    /// The downloaded bytes did not have the pinned digest.
    #[error("SHA-256 mismatch: expected {expected}, got {actual}")]
    DigestMismatch {
        /// Required digest.
        expected: String,
        /// Calculated digest.
        actual: String,
    },
    /// The verified temporary artifact could not be installed.
    #[error("could not atomically install artifact: {0}")]
    Persist(#[from] tempfile::PersistError),
}

/// Fetches, hashes, and atomically installs one corpus artifact.
///
/// Existing destination bytes remain untouched unless a complete response has
/// the expected media type and digest.
///
/// # Errors
///
/// Returns a classified transport, protocol, integrity, or filesystem error.
pub fn fetch(
    transport: &impl Transport,
    request: &FetchRequest,
    destination: &Path,
) -> Result<FetchOutcome, FetchError> {
    let http_request = HttpRequest {
        url: request.url.clone(),
        redirect_limit: request.redirect_limit,
        if_none_match: request.if_none_match.clone(),
        if_modified_since: request.if_modified_since.clone(),
    };
    let mut response = transport.get(&http_request)?;

    if response.status == 304 {
        return Ok(FetchOutcome::NotModified {
            final_url: response.final_url,
            etag: response.etag,
            last_modified: response.last_modified,
        });
    }
    if response.status != 200 {
        return Err(FetchError::UnexpectedStatus(response.status));
    }

    let content_type = response
        .content_type
        .as_deref()
        .map(normalize_content_type)
        .ok_or(FetchError::MissingContentType)?;
    if !request
        .expected_content_types
        .iter()
        .any(|expected| expected.eq_ignore_ascii_case(&content_type))
    {
        return Err(FetchError::UnexpectedContentType {
            actual: content_type,
            expected: request.expected_content_types.clone(),
        });
    }

    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let mut temporary = NamedTempFile::new_in(parent)?;
    let mut hasher = Sha256::new();
    let mut bytes = 0_u64;
    let mut buffer = vec![0_u8; 64 * 1024].into_boxed_slice();

    loop {
        let read = response.body.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        temporary.write_all(&buffer[..read])?;
        hasher.update(&buffer[..read]);
        let read_bytes =
            u64::try_from(read).map_err(|_| io::Error::other("read length does not fit in u64"))?;
        bytes = bytes
            .checked_add(read_bytes)
            .ok_or_else(|| io::Error::other("download byte count overflow"))?;
    }

    temporary.flush()?;
    temporary.as_file().sync_all()?;
    let sha256 = encode_hex(&hasher.finalize());
    if let Some(expected) = &request.expected_sha256
        && !expected.eq_ignore_ascii_case(&sha256)
    {
        return Err(FetchError::DigestMismatch {
            expected: expected.clone(),
            actual: sha256,
        });
    }

    temporary.persist(destination)?;

    Ok(FetchOutcome::Downloaded(DownloadMetadata {
        retrieved_at: jiff::Timestamp::now().to_string(),
        final_url: response.final_url,
        status: response.status,
        content_type,
        etag: response.etag,
        last_modified: response.last_modified,
        bytes,
        sha256,
    }))
}

fn normalize_content_type(content_type: &str) -> String {
    content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase()
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

/// Identity of an extracted archive member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveMetadata {
    /// Exact configured archive member name.
    pub member: String,
    /// Size of the source archive on disk.
    pub compressed_bytes: u64,
    /// Exact bytes emitted by decompression.
    pub uncompressed_bytes: u64,
    /// Lowercase SHA-256 of the uncompressed member.
    pub sha256: String,
}

/// Stable failure classes for archive inspection and extraction.
#[derive(Debug, Error)]
pub enum ArchiveError {
    /// The archive directory or compressed stream is invalid.
    #[error("corrupt ZIP archive: {0}")]
    Corrupt(String),
    /// The configured member was not present exactly once.
    #[error("ZIP archive does not contain exactly one {0}")]
    MissingMember(String),
    /// The configured member name could escape an extraction root.
    #[error("unsafe ZIP member name: {0}")]
    UnsafeMember(String),
    /// The member exceeds the configured uncompressed byte cap.
    #[error("ZIP member size {actual} exceeds limit {limit}")]
    SizeLimit {
        /// Configured uncompressed byte cap.
        limit: u64,
        /// Declared or observed member size.
        actual: u64,
    },
    /// The uncompressed member did not have the pinned digest.
    #[error("uncompressed SHA-256 mismatch: expected {expected}, got {actual}")]
    DigestMismatch {
        /// Required digest.
        expected: String,
        /// Calculated digest.
        actual: String,
    },
    /// The archive or local filesystem failed.
    #[error("archive I/O failed: {0}")]
    Io(#[from] io::Error),
    /// The verified temporary member could not be installed.
    #[error("could not atomically install extracted member: {0}")]
    Persist(#[from] tempfile::PersistError),
}

/// Inspects and atomically extracts exactly one configured ZIP member.
///
/// The declared and observed uncompressed sizes are bounded. The destination is
/// replaced only after decompression and optional digest verification succeed.
///
/// # Errors
///
/// Returns a classified archive, integrity, resource-limit, or filesystem error.
pub fn extract_zip_member(
    archive_path: &Path,
    expected_member: &str,
    destination: &Path,
    max_uncompressed_bytes: u64,
    expected_sha256: Option<&str>,
) -> Result<ArchiveMetadata, ArchiveError> {
    if expected_member.is_empty()
        || !Path::new(expected_member)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(ArchiveError::UnsafeMember(expected_member.to_owned()));
    }

    let archive_file = fs::File::open(archive_path)?;
    let compressed_bytes = archive_file.metadata()?.len();
    let mut archive =
        ZipArchive::new(archive_file).map_err(|error| ArchiveError::Corrupt(error.to_string()))?;
    let matching_members = archive
        .file_names()
        .filter(|name| *name == expected_member)
        .count();
    if matching_members != 1 {
        return Err(ArchiveError::MissingMember(expected_member.to_owned()));
    }

    let mut member = archive
        .by_name(expected_member)
        .map_err(|error| ArchiveError::Corrupt(error.to_string()))?;
    if member.size() > max_uncompressed_bytes {
        return Err(ArchiveError::SizeLimit {
            limit: max_uncompressed_bytes,
            actual: member.size(),
        });
    }

    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let mut temporary = NamedTempFile::new_in(parent)?;
    let mut hasher = Sha256::new();
    let mut uncompressed_bytes = 0_u64;
    let mut buffer = vec![0_u8; 64 * 1024].into_boxed_slice();

    loop {
        let read = member
            .read(&mut buffer)
            .map_err(|error| ArchiveError::Corrupt(error.to_string()))?;
        if read == 0 {
            break;
        }
        let read_bytes =
            u64::try_from(read).map_err(|_| io::Error::other("read length does not fit in u64"))?;
        uncompressed_bytes = uncompressed_bytes
            .checked_add(read_bytes)
            .ok_or_else(|| io::Error::other("uncompressed byte count overflow"))?;
        if uncompressed_bytes > max_uncompressed_bytes {
            return Err(ArchiveError::SizeLimit {
                limit: max_uncompressed_bytes,
                actual: uncompressed_bytes,
            });
        }
        temporary.write_all(&buffer[..read])?;
        hasher.update(&buffer[..read]);
    }

    temporary.flush()?;
    temporary.as_file().sync_all()?;
    let sha256 = encode_hex(&hasher.finalize());
    if let Some(expected) = expected_sha256
        && !expected.eq_ignore_ascii_case(&sha256)
    {
        return Err(ArchiveError::DigestMismatch {
            expected: expected.to_owned(),
            actual: sha256,
        });
    }

    temporary.persist(destination)?;
    Ok(ArchiveMetadata {
        member: expected_member.to_owned(),
        compressed_bytes,
        uncompressed_bytes,
        sha256,
    })
}

/// Blocking HTTPS transport used by the corpus command-line harness.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReqwestTransport;

impl Transport for ReqwestTransport {
    fn get(&self, request: &HttpRequest) -> Result<HttpResponse, FetchError> {
        let client = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(request.redirect_limit))
            .build()
            .map_err(|error| FetchError::Transport(error.to_string()))?;
        let mut builder = client.get(&request.url);
        if let Some(value) = &request.if_none_match {
            builder = builder.header(IF_NONE_MATCH, value);
        }
        if let Some(value) = &request.if_modified_since {
            builder = builder.header(IF_MODIFIED_SINCE, value);
        }

        let response = builder
            .send()
            .map_err(|error| FetchError::Transport(error.to_string()))?;
        let status = response.status().as_u16();
        let final_url = response.url().to_string();
        let content_type = header(&response, CONTENT_TYPE);
        let etag = header(&response, ETAG);
        let last_modified = header(&response, LAST_MODIFIED);

        Ok(HttpResponse {
            status,
            final_url,
            content_type,
            etag,
            last_modified,
            body: Box::new(response),
        })
    }
}

fn header(
    response: &reqwest::blocking::Response,
    name: reqwest::header::HeaderName,
) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}
