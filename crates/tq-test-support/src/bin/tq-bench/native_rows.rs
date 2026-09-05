//! Deterministic, correctness-checked native row fixtures, generated before timing.

use super::{ArtifactIdentity, DatasetTier, PreparedDataset, hex};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Write as _, path::Path};
use tq_formats::{DecodeOptions, InputRepresentation, NativeFormat, NativeInputObservation};

pub(super) fn prepare(output: &Path) -> Result<Vec<PreparedDataset>, Box<dyn std::error::Error>> {
    [(DatasetTier::Small, 8), (DatasetTier::Large, 131_072)]
        .into_iter()
        .map(|(tier, count)| prepare_rows(output, tier, count))
        .collect()
}

fn prepare_rows(
    output: &Path,
    tier: DatasetTier,
    count: u64,
) -> Result<PreparedDataset, Box<dyn std::error::Error>> {
    let mut json = Vec::new();
    let mut csv = b"id,active,label\n".to_vec();
    let mut tsv = b"id\tactive\tlabel\n".to_vec();
    for id in 0..count {
        let active = id % 2 == 0;
        writeln!(
            json,
            "\x1e{{\"id\":{id},\"active\":{active},\"label\":\"native-row-{id}\"}}"
        )?;
        writeln!(csv, "{id},{active},native-row-{id}")?;
        writeln!(tsv, "{id}\t{active}\tnative-row-{id}")?;
    }
    let source_id = format!("native-rows-{count}");
    let manifest_sha256 = hex(&Sha256::digest(&json));
    let mut formats = BTreeMap::new();
    for (format, bytes) in [
        (NativeFormat::JsonSequence, json),
        (NativeFormat::Csv, csv),
        (NativeFormat::Tsv, tsv),
    ] {
        validate_rows(format, &bytes, count)?;
        let name = format.descriptor().name;
        let path = output.join(format!("{source_id}.{name}"));
        fs::write(&path, &bytes)?;
        let artifact = ArtifactIdentity {
            path: path.display().to_string(),
            bytes: bytes.len() as u64,
            sha256: hex(&Sha256::digest(&bytes)),
        };
        formats.insert(name, (path, artifact));
    }
    Ok(PreparedDataset {
        source_id,
        tier,
        logical_records: count,
        manifest_sha256,
        origin: "synthetic-reviewed".to_owned(),
        formats,
    })
}

fn validate_rows(
    format: NativeFormat,
    bytes: &[u8],
    count: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut input = format
        .select_input(DecodeOptions::default(), InputRepresentation::Documents)?
        .open(bytes, "native benchmark fixture");
    for id in 0..count {
        let Some(NativeInputObservation::Document(document)) = input.next_observation()? else {
            return Err("native fixture lost a row or published a parse failure".into());
        };
        let expected = serde_json::json!({"id": id, "active": id % 2 == 0, "label": format!("native-row-{id}")});
        if document.value.to_json()? != expected {
            return Err(format!("native fixture changed row {id}").into());
        }
        let tq_core::Value::Object(actual) = &document.value else {
            return Err("native row is not an object".into());
        };
        if actual.keys().map(AsRef::as_ref).collect::<Vec<&str>>() != ["id", "active", "label"] {
            return Err(format!("native fixture reordered row {id}").into());
        }
    }
    if input.next_observation()?.is_some() {
        return Err("native fixture added a row".into());
    }
    Ok(())
}
