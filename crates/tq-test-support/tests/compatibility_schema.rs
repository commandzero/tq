//! Versioned compatibility-case schema tests.

use serde_json::json;

const SCHEMA: &str = include_str!("../../../schemas/compatibility-case-v1.schema.json");

fn validator() -> jsonschema::Validator {
    let schema = serde_json::from_str(SCHEMA).expect("compatibility schema JSON");
    assert!(jsonschema::meta::is_valid(&schema));
    jsonschema::Validator::new(&schema).expect("compatibility schema")
}

fn valid_case() -> serde_json::Value {
    json!({
        "schema_version": 1,
        "id": "identity.object-order",
        "title": "Identity preserves object insertion order",
        "classification": "common",
        "capabilities": ["identity", "object.order"],
        "status": "mvp",
        "fixture": {"format": "json", "inline": "{\"z\":1,\"a\":2}"},
        "query": ".",
        "adapters": {
            "jq": {"supported": true},
            "yq": {"args": ["-o=json"], "supported": true},
            "tq": {"args": ["--input-format", "json", "--output-format", "json"], "supported": false}
        },
        "invocation_mode": "stdin",
        "expected": {"contract": "result-sequence", "baseline": "required"}
    })
}

#[test]
fn complete_versioned_case_is_valid() {
    assert!(validator().is_valid(&valid_case()));
}

#[test]
fn unknown_versions_fields_and_classifications_are_rejected() {
    let validator = validator();
    let mut case = valid_case();
    case["schema_version"] = json!(2);
    assert!(!validator.is_valid(&case));

    let mut case = valid_case();
    case["classification"] = json!("intersection");
    assert!(!validator.is_valid(&case));

    let mut case = valid_case();
    case["bless"] = json!(true);
    assert!(!validator.is_valid(&case));
}

#[test]
fn case_requires_stable_id_capabilities_adapters_and_expected_contract() {
    let validator = validator();
    for field in ["id", "capabilities", "adapters", "expected"] {
        let mut case = valid_case();
        case.as_object_mut().expect("case object").remove(field);
        assert!(!validator.is_valid(&case), "missing {field}");
    }
}
