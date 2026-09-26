//! Prepare a caller-supplied `GeoJSON` large input without generating other representations.

use super::{PreparedCampaign, PreparedDataset, hex};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, BufReader, Read},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tq_test_support::{
    benchmark::DatasetTier,
    corpus::{ArtifactIdentity, validate_geojson},
};

struct InputReader<'a> {
    file: fs::File,
    digest: Sha256,
    bytes: u64,
    cancellation: &'a Arc<AtomicBool>,
}

impl Read for InputReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.cancellation.load(Ordering::Acquire) {
            return Err(io::Error::other("large-input preparation cancelled"));
        }
        let count = self.file.read(buffer)?;
        self.digest.update(&buffer[..count]);
        self.bytes += count as u64;
        Ok(count)
    }
}

pub(super) fn prepare(
    path: &Path,
    cancellation: &Arc<AtomicBool>,
) -> Result<PreparedCampaign, Box<dyn std::error::Error>> {
    let path = fs::canonicalize(path)?;
    let mut input = InputReader {
        file: fs::File::open(&path)?,
        digest: Sha256::new(),
        bytes: 0,
        cancellation,
    };
    let metadata = validate_geojson(BufReader::with_capacity(64 * 1024, &mut input))?;
    let artifact = ArtifactIdentity {
        path: path.display().to_string(),
        bytes: input.bytes,
        sha256: hex(&input.digest.finalize()),
    };
    let descriptor = serde_json::to_vec(&(&artifact, metadata.logical_records))?;
    Ok(PreparedCampaign {
        temporary: Some(tempfile::tempdir()?),
        datasets: vec![PreparedDataset {
            source_id: "large-input-selected-json".to_owned(),
            tier: DatasetTier::Large,
            logical_records: metadata.logical_records,
            manifest_sha256: hex(&Sha256::digest(descriptor)),
            origin: "provided-input".to_owned(),
            formats: BTreeMap::from([("json", (path, artifact))]),
        }],
    })
}
