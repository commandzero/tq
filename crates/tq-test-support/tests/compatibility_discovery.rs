//! Executable discovery and identity tests.

use std::path::PathBuf;

use tq_test_support::compatibility::{
    ExecutableConfig, ToolDiscoveryError, ToolKind, discover_tool,
};

#[cfg(unix)]
#[test]
fn explicit_executable_is_canonicalized_hashed_and_versioned() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir().expect("temporary directory");
    let executable = temp.path().join("fake-jq");
    std::fs::write(&executable, "#!/bin/sh\nprintf 'jq-test-1.8.0\\n'\n").expect("fake executable");
    let mut permissions = std::fs::metadata(&executable)
        .expect("fake metadata")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&executable, permissions).expect("executable mode");
    let config = ExecutableConfig {
        jq: Some(executable.clone()),
        ..ExecutableConfig::default()
    };

    let identity = discover_tool(ToolKind::Jq, &config, temp.path())
        .expect("discovery")
        .expect("identity");
    assert_eq!(identity.path, std::fs::canonicalize(executable).unwrap());
    assert_eq!(identity.version, "jq-test-1.8.0");
    assert_eq!(identity.executable.sha256.len(), 64);
}

#[test]
fn invalid_explicit_override_is_an_error_and_optional_tq_discovery_is_best_effort() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let config = ExecutableConfig {
        jq: Some(PathBuf::from("does-not-exist")),
        tq: Some(PathBuf::from("also-missing")),
        ..ExecutableConfig::default()
    };
    assert!(matches!(
        discover_tool(ToolKind::Jq, &config, temp.path()),
        Err(ToolDiscoveryError::InvalidOverride { .. })
    ));

    let no_override = ExecutableConfig::default();
    let isolated_root = temp.path().join("isolated/repository");
    std::fs::create_dir_all(&isolated_root).expect("isolated root");
    if let Some(identity) =
        discover_tool(ToolKind::Tq, &no_override, &isolated_root).expect("optional discovery")
    {
        assert_eq!(identity.tool, ToolKind::Tq);
        assert!(identity.path.is_absolute());
        assert_eq!(identity.executable.sha256.len(), 64);
    }
}
