//! Shared test construction helpers.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tq_test_support::corpus::{
    ArtifactIdentity, LicenseIdentity, Provenance, RequestIdentity, SourceSnapshotInput,
};

#[allow(dead_code)]
pub fn artifact(path: &str, file: &Path) -> ArtifactIdentity {
    let bytes = std::fs::read(file).expect("artifact bytes");
    let digest = Sha256::digest(&bytes);
    let sha256 = digest
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            use std::fmt::Write as _;
            write!(hex, "{byte:02x}").expect("write digest to string");
            hex
        });
    ArtifactIdentity {
        path: path.to_owned(),
        bytes: bytes.len() as u64,
        sha256,
    }
}

pub fn snapshot_input(source_json_file: PathBuf) -> SourceSnapshotInput {
    SourceSnapshotInput {
        campaign_id: "2026-07-31T08-00-00Z".to_owned(),
        source_id: "usgs-all-hour".to_owned(),
        retrieved_at: "2026-07-31T08:00:00Z".to_owned(),
        request: RequestIdentity {
            requested_url: "https://example.test/source.json".to_owned(),
            final_url: "https://example.test/source.json".to_owned(),
            status: 200,
            content_type: "application/json".to_owned(),
            etag: None,
            last_modified: None,
        },
        archive: None,
        download: ArtifactIdentity {
            path: "downloads/source.json".to_owned(),
            bytes: 0,
            sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_owned(),
        },
        source_json_file,
        source_json_path: "prepared/source.json".to_owned(),
        provenance: Provenance {
            title: "Test source".to_owned(),
            publisher: "Test publisher".to_owned(),
            landing_page: "https://example.test/source".to_owned(),
            license: LicenseIdentity {
                name: "Test license".to_owned(),
                url: "https://example.test/license".to_owned(),
            },
        },
    }
}
