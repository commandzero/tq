//! Real embedded API subprocess observations through the shared campaign seam.

use std::{collections::BTreeSet, path::Path, time::Duration};

use tq_test_support::{
    compatibility::{
        CampaignProfile, CompatibilityCatalog, ErrorClass, ExecutableConfig, ExecutionMode,
        FinalStatus, FixtureFormat, ObservationState, ProcessStatus, ToolKind,
        discover_embedded_host, load_catalog, run_campaign,
    },
    corpus::ArtifactIdentity,
};

#[test]
fn committed_denial_cases_execute_the_real_embedded_api_and_record_its_identity() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut catalog = load_catalog(&root.join("tests/compatibility/cases")).unwrap();
    catalog
        .cases
        .retain(|case| matches!(case.id.as_str(), "environment.denied" | "platform.denied"));
    assert_eq!(catalog.cases.len(), 2);
    for case in &catalog.cases {
        assert_eq!(
            case.adapters.tq.execution_mode,
            if case.query == "env" {
                ExecutionMode::EmbeddedDenyEnvironment
            } else {
                ExecutionMode::EmbeddedDenyPlatform
            }
        );
    }
    let config = ExecutableConfig {
        embedded_host: Some(env!("CARGO_BIN_EXE_tq-compat-embedded").into()),
        ..Default::default()
    };
    let expected_identity = discover_embedded_host(&config, &root).unwrap().unwrap();
    let report = run_campaign(
        &catalog,
        CampaignProfile::Full,
        &config,
        &root,
        Duration::from_secs(3),
    )
    .unwrap();
    assert_eq!(
        report.final_status,
        FinalStatus::Passed,
        "{}",
        report.render_human()
    );
    for case in &report.cases {
        assert!(case.contract_failures.is_empty());
        assert!(case.semantic_diffs.is_empty());
        let provenance = case.tq_execution.as_ref().unwrap();
        assert_eq!(provenance.host, expected_identity);
        assert!(provenance.host.version.starts_with("tq-compat-embedded "));
        assert_eq!(provenance.host.executable.sha256.len(), 64);
        let mut formats = BTreeSet::new();
        for observation in case.observations.iter().filter(|o| o.tool == ToolKind::Tq) {
            assert_eq!(observation.state, ObservationState::Executed);
            assert_eq!(observation.process_status, Some(ProcessStatus::Exited));
            assert_eq!(observation.exit_code, Some(5));
            assert_eq!(observation.error_class, Some(ErrorClass::RuntimePolicy));
            assert_eq!(observation.results, [] as [serde_json::Value; 0]);
            assert_eq!(observation.stdout_hex.as_deref(), Some(""));
            assert!(observation.stderr_hex.is_some());
            formats.insert(serde_json::to_string(&observation.input_format).unwrap());
        }
        assert_eq!(
            formats,
            [
                FixtureFormat::Json,
                FixtureFormat::Yaml,
                FixtureFormat::Toon
            ]
            .map(|f| serde_json::to_string(&f).unwrap())
            .into_iter()
            .collect()
        );
    }
    let round_trip: tq_test_support::compatibility::CompatibilityReport =
        serde_json::from_value(serde_json::to_value(&report).unwrap()).unwrap();
    assert_eq!(
        round_trip.cases[0].tq_execution.as_ref().unwrap().host,
        expected_identity
    );
}

#[test]
fn selected_authority_is_denied_while_the_other_authority_is_admitted() {
    use std::collections::BTreeMap;
    use tq_test_support::compatibility::{Invocation, run_process};
    for (mode, query, expected) in [
        (
            "embedded-deny-environment",
            "now | type",
            b"\"number\"\n".as_slice(),
        ),
        (
            "embedded-deny-platform",
            "env.TQ_COMPAT_HOST_SENTINEL",
            b"\"controlled\"\n".as_slice(),
        ),
    ] {
        let outcome = run_process(&Invocation {
            executable: env!("CARGO_BIN_EXE_tq-compat-embedded").into(),
            args: [
                "--execution-mode",
                mode,
                "--",
                "-n",
                "-o",
                "json",
                "-c",
                query,
            ]
            .map(str::to_owned)
            .into(),
            stdin: Vec::new(),
            timeout: Duration::from_secs(3),
            current_dir: None,
            environment: BTreeMap::from([("TQ_COMPAT_HOST_SENTINEL".into(), "controlled".into())]),
        })
        .unwrap();
        assert_eq!(outcome.exit_code, Some(0));
        assert_eq!(outcome.stdout, expected);
        assert_eq!(outcome.stderr, [] as [u8; 0]);
    }
    for (mode, flag) in [
        ("embedded-deny-environment", "--allow-environment"),
        ("embedded-deny-platform", "--allow-platform"),
    ] {
        let outcome = run_process(&Invocation {
            executable: env!("CARGO_BIN_EXE_tq-compat-embedded").into(),
            args: ["--execution-mode", mode, "--", flag, "-n", "1"]
                .map(str::to_owned)
                .into(),
            stdin: Vec::new(),
            timeout: Duration::from_secs(3),
            current_dir: None,
            environment: BTreeMap::new(),
        })
        .unwrap();
        assert_eq!(outcome.exit_code, Some(2));
        assert_eq!(outcome.stdout, [] as [u8; 0]);
    }
}

#[test]
fn embedded_routing_is_explicit_and_independent_of_case_identifiers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let case = serde_json::from_value(serde_json::json!({
        "schema_version": 1, "id": "arbitrary.policy", "title": "No ID routing",
        "classification": "jq-target", "capabilities": ["policy.denied"], "status": "mvp",
        "fixture": {"format": "json", "inline": "null"}, "query": "env",
        "adapters": {"tq": {"supported": true, "execution_mode": "embedded-deny-environment",
            "env": {"TQ_COMPAT_REDACTION_TEST": "private-sentinel-value"}}},
        "invocation_mode": "stdin",
        "expected": {"contract": "error", "baseline": "not-applicable", "error_class": "runtime-policy"}
    })).unwrap();
    let catalog = CompatibilityCatalog {
        cases: vec![case],
        identity: ArtifactIdentity {
            path: "test".into(),
            bytes: 0,
            sha256: String::new(),
        },
    };
    let config = ExecutableConfig {
        embedded_host: Some(env!("CARGO_BIN_EXE_tq-compat-embedded").into()),
        ..Default::default()
    };
    let report = run_campaign(
        &catalog,
        CampaignProfile::Full,
        &config,
        &root,
        Duration::from_secs(3),
    )
    .unwrap();
    assert_eq!(
        report.final_status,
        FinalStatus::Passed,
        "{}",
        report.render_human()
    );
    let json = serde_json::to_string(&report).unwrap();
    assert!(!json.contains("private-sentinel-value"));
    for observation in &report.cases[0].observations {
        assert!(
            !observation
                .stderr_hex
                .as_deref()
                .unwrap_or_default()
                .contains(&tq_test_support::compatibility::encode_hex(
                    b"private-sentinel-value"
                ))
        );
    }
}
