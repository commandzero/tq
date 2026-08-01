//! Streaming `GeoJSON` structural-validation contract tests.

use std::io::Cursor;

use tq_test_support::corpus::{GeoJsonError, validate_geojson};

#[test]
fn feature_collection_is_validated_and_counted_without_assumed_size() {
    let input = br#"{
        "type": "FeatureCollection",
        "metadata": {"count": 2},
        "features": [
            {"type": "Feature", "properties": {"id": 1}, "geometry": null},
            {"type": "Feature", "properties": {"id": 2}, "geometry": null}
        ]
    }"#;

    let metadata = validate_geojson(Cursor::new(input)).expect("valid GeoJSON");
    assert_eq!(metadata.root_type, "FeatureCollection");
    assert_eq!(metadata.logical_records, 2);
}

#[test]
fn wrong_geojson_root_type_is_rejected() {
    let input = br#"{"type":"Feature","features":[]}"#;

    assert!(matches!(
        validate_geojson(Cursor::new(input)),
        Err(GeoJsonError::WrongRootType(actual)) if actual == "Feature"
    ));
}

#[test]
fn missing_or_non_array_features_are_rejected_as_unexpected_shape() {
    let missing = br#"{"type":"FeatureCollection"}"#;
    let wrong_type = br#"{"type":"FeatureCollection","features":{}}"#;

    assert!(matches!(
        validate_geojson(Cursor::new(missing)),
        Err(GeoJsonError::UnexpectedShape(_))
    ));
    assert!(matches!(
        validate_geojson(Cursor::new(wrong_type)),
        Err(GeoJsonError::UnexpectedShape(_))
    ));
}

#[test]
fn non_object_document_is_rejected_as_unexpected_shape() {
    assert!(matches!(
        validate_geojson(Cursor::new(br"[]")),
        Err(GeoJsonError::UnexpectedShape(_))
    ));
}

#[test]
fn invalid_json_and_trailing_data_are_rejected_as_syntax() {
    assert!(matches!(
        validate_geojson(Cursor::new(br#"{"type":"FeatureCollection","features":["#)),
        Err(GeoJsonError::InvalidJson(_))
    ));
    assert!(matches!(
        validate_geojson(Cursor::new(
            br#"{"type":"FeatureCollection","features":[]} false"#
        )),
        Err(GeoJsonError::InvalidJson(_))
    ));
}
