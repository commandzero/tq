//! Archive integrity and extraction contract tests.

use std::{
    io::{Cursor, Write},
    path::Path,
};

use tq_test_support::corpus::{ArchiveError, extract_zip_member};
use zip::{ZipWriter, write::SimpleFileOptions};

fn zip_bytes(member: &str, body: &[u8]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file(member, SimpleFileOptions::default())
        .expect("start zip member");
    writer.write_all(body).expect("write zip member");
    writer.finish().expect("finish zip").into_inner()
}

#[test]
fn valid_expected_member_is_atomically_extracted_and_identified() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let archive = temp.path().join("Georgia.geojson.zip");
    let destination = temp.path().join("Georgia.geojson");
    std::fs::write(
        &archive,
        zip_bytes("Georgia.geojson", b"{\"type\":\"FeatureCollection\"}"),
    )
    .expect("archive");

    let metadata = extract_zip_member(&archive, "Georgia.geojson", &destination, 1024, None)
        .expect("valid archive");

    assert_eq!(metadata.member, "Georgia.geojson");
    assert_eq!(metadata.compressed_bytes, file_len(&archive));
    assert_eq!(metadata.uncompressed_bytes, 28);
    assert_eq!(
        metadata.sha256,
        "3ca92cc8e99803203d90f8c34844412c177f52cf24154fb0088671bd0e514309"
    );
    assert_eq!(
        std::fs::read(destination).expect("extracted member"),
        b"{\"type\":\"FeatureCollection\"}"
    );
}

#[test]
fn corrupt_archive_is_rejected_without_output() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let archive = temp.path().join("corrupt.zip");
    let destination = temp.path().join("Georgia.geojson");
    std::fs::write(&archive, b"not a zip archive").expect("corrupt archive");

    assert!(matches!(
        extract_zip_member(&archive, "Georgia.geojson", &destination, 1024, None),
        Err(ArchiveError::Corrupt(_))
    ));
    assert!(!destination.exists());
}

#[test]
fn missing_or_unsafe_archive_member_is_rejected() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let archive = temp.path().join("wrong.zip");
    let destination = temp.path().join("Georgia.geojson");
    std::fs::write(&archive, zip_bytes("../Georgia.geojson", b"unexpected")).expect("archive");

    assert!(matches!(
        extract_zip_member(
            &archive,
            "Georgia.geojson",
            &destination,
            1024,
            None
        ),
        Err(ArchiveError::MissingMember(member)) if member == "Georgia.geojson"
    ));
    assert!(!destination.exists());
}

#[test]
fn declared_or_actual_oversize_member_is_rejected() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let archive = temp.path().join("large.zip");
    let destination = temp.path().join("Georgia.geojson");
    std::fs::write(&archive, zip_bytes("Georgia.geojson", b"too large")).expect("archive");

    assert!(matches!(
        extract_zip_member(&archive, "Georgia.geojson", &destination, 4, None),
        Err(ArchiveError::SizeLimit { limit: 4, .. })
    ));
    assert!(!destination.exists());
}

#[test]
fn extracted_digest_mismatch_preserves_existing_destination() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let archive = temp.path().join("Georgia.geojson.zip");
    let destination = temp.path().join("Georgia.geojson");
    std::fs::write(&archive, zip_bytes("Georgia.geojson", b"new bytes")).expect("archive");
    std::fs::write(&destination, b"old bytes").expect("old destination");

    assert!(matches!(
        extract_zip_member(
            &archive,
            "Georgia.geojson",
            &destination,
            1024,
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        ),
        Err(ArchiveError::DigestMismatch { .. })
    ));
    assert_eq!(
        std::fs::read(destination).expect("old destination"),
        b"old bytes"
    );
}

fn file_len(path: &Path) -> u64 {
    std::fs::metadata(path).expect("file metadata").len()
}
