//! Native Windows reference streams retain their actual byte contracts.

#[cfg(windows)]
mod native {
    use std::{fs, path::Path, time::Duration};
    use tq_test_support::{
        compatibility::{
            CampaignProfile, CompatibilityCatalog, ExecutableConfig, ToolKind, compare_manual,
            discover_tool, manual_host_target, run_campaign, validate_manual_reference,
        },
        corpus::ArtifactIdentity,
    };

    fn reviewed_config(root: &Path) -> ExecutableConfig {
        let config = ExecutableConfig {
            jq: Some(root.join("target/reference-build/jq/jq.exe")),
            tq: Some(root.join("target/debug/tq.exe")),
            yq: None,
        };
        let pin = tq_test_support::fixture_data::read(
            &root.join("tests/compatibility/reviews/jq-manual/reference-pin.toon"),
        )
        .unwrap();
        let reference = discover_tool(ToolKind::Jq, &config, root).unwrap().unwrap();
        validate_manual_reference(&pin, &reference, &manual_host_target()).unwrap();
        config
    }

    fn stream_catalog() -> CompatibilityCatalog {
        let cases = [
            ("semantic", "json", "true", "result-sequence", vec![], vec![]),
            ("raw-cli", "json", "true", "raw-bytes", vec!["-c"], vec!["-c", "-o", "json"]),
            ("raw-input", "raw", "one\r\ntwo\n", "result-sequence", vec!["-R"], vec!["-R"]),
            ("raw-output", "json", "\"one\\ntwo\"", "raw-bytes", vec!["-r"], vec!["-r"]),
            ("raw-exit", "json", "true", "exit-status", vec!["-e"], vec!["-e", "-o", "json"]),
        ].into_iter().map(|(name, format, input, contract, jq_args, tq_args)| {
            serde_json::from_value(serde_json::json!({
                "schema_version": 1, "id": format!("manual.windows.{name}"), "title": name,
                "classification": "jq-target", "capabilities": ["windows.reference-streams"],
                "status": "mvp", "fixture": {"format": format, "inline": input}, "query": ".",
                "adapters": {"jq": {"supported": true, "args": jq_args}, "tq": {"supported": true, "args": tq_args}},
                "invocation_mode": "stdin", "expected": {"contract": contract, "baseline": "required"}
            })).unwrap()
        }).collect();
        CompatibilityCatalog {
            cases,
            identity: ArtifactIdentity {
                path: "windows-stream-regression".into(),
                bytes: 0,
                sha256: String::new(),
            },
        }
    }

    #[test]
    #[ignore = "requires built tq and the reviewed native Windows jq"]
    fn semantic_binary_mode_does_not_modify_raw_cli_or_raw_input_streams() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let config = reviewed_config(&root);
        let fixtures = tempfile::tempdir().unwrap();
        fs::create_dir_all(
            fixtures
                .path()
                .join("tests/compatibility/reviews/jq-manual"),
        )
        .unwrap();
        let catalog = stream_catalog();
        let report =
            compare_manual(&catalog, &config, fixtures.path(), Duration::from_secs(5)).unwrap();
        assert_eq!(report["reference_execution"]["target"], "x86_64-windows");
        assert_eq!(
            report["reference_execution"]["jq_structured_program_prefix_args"],
            serde_json::json!(["--binary"])
        );
        let row = |id: &str| {
            report["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["id"] == format!("manual.windows.{id}"))
                .unwrap()
        };
        let semantic = row("semantic");
        assert_eq!(semantic["jq"]["stdout_hex"], "747275650a");
        assert_eq!(semantic["compact"]["exact"], true);
        assert_eq!(semantic["compact"]["jq"]["stdout_hex"], "747275650a");
        let raw_cli = row("raw-cli");
        assert_eq!(raw_cli["jq"]["stdout_hex"], "747275650d0a");
        assert_eq!(raw_cli["tq"]["stdout_hex"], "747275650a");
        assert_eq!(raw_cli["verdict"], "failure");
        assert!(raw_cli["compact"].is_null());
        let raw_output = row("raw-output");
        assert_eq!(raw_output["jq"]["stdout_hex"], "6f6e650d0a74776f0d0a");
        assert_eq!(raw_output["tq"]["stdout_hex"], "6f6e650a74776f0a");
        let raw_exit = row("raw-exit");
        assert_eq!(raw_exit["jq"]["stdout_hex"], "747275650d0a");
        assert_eq!(raw_exit["tq"]["stdout_hex"], "747275650a");
        let raw_input = row("raw-input");
        assert_eq!(
            raw_input["jq"]["results"],
            serde_json::json!(["one", "two"])
        );
        assert_eq!(
            raw_input["jq"]["stdout_hex"],
            "226f6e65220d0a2274776f220d0a"
        );
        assert_eq!(
            raw_input["compact"]["jq"]["stdout_hex"],
            raw_input["jq"]["stdout_hex"]
        );
        let campaign = run_campaign(
            &catalog,
            CampaignProfile::Full,
            &config,
            fixtures.path(),
            Duration::from_secs(5),
        )
        .unwrap();
        for (id, stdout) in [
            ("semantic", "747275650a"),
            ("raw-cli", "747275650d0a"),
            ("raw-output", "6f6e650d0a74776f0d0a"),
            ("raw-exit", "747275650d0a"),
            ("raw-input", "226f6e65220d0a2274776f220d0a"),
        ] {
            let case = campaign
                .cases
                .iter()
                .find(|case| case.id == format!("manual.windows.{id}"))
                .unwrap();
            let jq = case
                .observations
                .iter()
                .find(|value| value.tool == ToolKind::Jq)
                .unwrap();
            assert_eq!(jq.stdout_hex.as_deref(), Some(stdout), "{id}");
        }
        if let Some(path) = std::env::var_os("TQ_COMPAT_FOCUSED_REPORT") {
            fs::write(
                path,
                serde_json::to_vec_pretty(
                    &serde_json::json!({"manual": report, "campaign": campaign}),
                )
                .unwrap(),
            )
            .unwrap();
        }
    }
}
