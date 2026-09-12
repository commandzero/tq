//! Release-gate checks for deterministic jq reference provisioning.

#[cfg(unix)]
mod unix {
    use sha2::{Digest, Sha256};
    use std::{fmt::Write as _, fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

    fn root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn digest(bytes: &[u8]) -> String {
        let mut output = String::with_capacity(64);
        for byte in Sha256::digest(bytes) {
            write!(output, "{byte:02x}").expect("write digest");
        }
        output
    }

    fn run(script: &Path, reference: &Path, expected_digest: Option<&str>) -> std::process::Output {
        let mut command = Command::new("sh");
        command
            .arg(script)
            .arg("sh")
            .arg("-c")
            .arg("test \"$TQ_JQ\" = \"$EXPECTED_JQ\"")
            .env("TQ_REFERENCE_JQ", reference)
            .env("EXPECTED_JQ", reference)
            .env_remove("TQ_JQ");
        if let Some(expected_digest) = expected_digest {
            command.env("TQ_REFERENCE_JQ_SHA256", expected_digest);
        } else {
            command.env_remove("TQ_REFERENCE_JQ_SHA256");
        }
        command.output().expect("run reference provisioning helper")
    }

    fn run_download(
        script: &Path,
        source: &Path,
        output_path: &Path,
        expected_digest: Option<&str>,
    ) -> std::process::Output {
        let mut command = Command::new("sh");
        command
            .arg(script)
            .arg("sh")
            .arg("-c")
            .arg("test \"$TQ_JQ\" = \"$EXPECTED_JQ\"")
            .env(
                "TQ_REFERENCE_JQ_URL",
                format!("file://{}", source.display()),
            )
            .env("TQ_REFERENCE_JQ_OUTPUT", output_path)
            .env("EXPECTED_JQ", output_path)
            .env_remove("TQ_JQ")
            .env_remove("TQ_REFERENCE_JQ");
        if let Some(expected_digest) = expected_digest {
            command.env("TQ_REFERENCE_JQ_SHA256", expected_digest);
        } else {
            command.env_remove("TQ_REFERENCE_JQ_SHA256");
        }
        command.output().expect("run reference download helper")
    }

    fn run_signed_url_download(
        script: &Path,
        source: &Path,
        output_path: &Path,
        curl: &Path,
    ) -> std::process::Output {
        let mut command = Command::new("sh");
        command
            .arg(script)
            .env(
                "TQ_REFERENCE_JQ_URL",
                "https://example.invalid/jq?token=super-secret",
            )
            .env("TQ_REFERENCE_JQ_SHA256", "00".repeat(32))
            .env("TQ_REFERENCE_JQ_OUTPUT", output_path)
            .env("SOURCE", source)
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", curl.parent().unwrap().display()),
            )
            .env_remove("TQ_JQ")
            .env_remove("TQ_REFERENCE_JQ");
        command
            .output()
            .expect("run signed URL provisioning helper")
    }

    #[test]
    fn explicit_reference_is_forwarded_with_verified_digest() {
        let directory = tempfile::tempdir().expect("reference fixture directory");
        let reference = directory.path().join("jq with spaces");
        let bytes = b"#!/bin/sh\nexit 0\n";
        fs::write(&reference, bytes).expect("reference executable writes");
        fs::set_permissions(&reference, fs::Permissions::from_mode(0o755))
            .expect("reference executable permissions");
        let output = run(
            &root().join("scripts/reference-jq-provision.sh"),
            &reference,
            Some(&digest(bytes)),
        );
        assert!(
            output.status.success(),
            "status={:?}\nstdout={}\nstderr={}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn digest_mismatch_is_a_release_gate_failure() {
        let directory = tempfile::tempdir().expect("reference fixture directory");
        let reference = directory.path().join("jq");
        let bytes = b"#!/bin/sh\nexit 0\n";
        fs::write(&reference, bytes).expect("reference executable writes");
        fs::set_permissions(&reference, fs::Permissions::from_mode(0o755))
            .expect("reference executable permissions");
        let output = run(
            &root().join("scripts/reference-jq-provision.sh"),
            &reference,
            Some(&"00".repeat(32)),
        );
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("SHA-256"));
    }

    #[test]
    fn file_artifact_is_provisioned_only_with_a_matching_digest() {
        let directory = tempfile::tempdir().expect("reference fixture directory");
        let source = directory.path().join("source-jq");
        let output_path = directory.path().join("target/jq");
        let bytes = b"#!/bin/sh\nexit 0\n";
        fs::write(&source, bytes).expect("reference artifact writes");
        let output = run_download(
            &root().join("scripts/reference-jq-provision.sh"),
            &source,
            &output_path,
            Some(&digest(bytes)),
        );
        assert!(
            output.status.success(),
            "status={:?}\nstdout={}\nstderr={}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output_path.is_file());

        let wrong_digest_output = directory.path().join("target/wrong-digest-jq");
        let output = run_download(
            &root().join("scripts/reference-jq-provision.sh"),
            &source,
            &wrong_digest_output,
            Some(&"00".repeat(32)),
        );
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("SHA-256 mismatch"));
        assert!(!wrong_digest_output.exists());

        let missing_digest_output = directory.path().join("target/missing-digest-jq");
        let output = run_download(
            &root().join("scripts/reference-jq-provision.sh"),
            &source,
            &missing_digest_output,
            None,
        );
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("requires TQ_REFERENCE_JQ_SHA256")
        );
        assert!(!missing_digest_output.exists());
    }

    #[test]
    fn signed_artifact_url_is_not_echoed_on_digest_failure() {
        let directory = tempfile::tempdir().expect("reference fixture directory");
        let source = directory.path().join("source-jq");
        let curl = directory.path().join("curl");
        let output_path = directory.path().join("target/jq");
        let bytes = b"#!/bin/sh\nexit 0\n";
        fs::write(&source, bytes).expect("reference artifact writes");
        fs::write(
            &curl,
            b"#!/bin/sh\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = -o ]; then output=$2; shift 2; else shift; fi\ndone\ncp \"$SOURCE\" \"$output\"\n",
        )
        .expect("curl stub writes");
        fs::set_permissions(&curl, fs::Permissions::from_mode(0o755))
            .expect("curl stub permissions");
        let output = run_signed_url_download(
            &root().join("scripts/reference-jq-provision.sh"),
            &source,
            &output_path,
            &curl,
        );
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("downloaded jq artifact SHA-256 mismatch"));
        assert!(!stderr.contains("super-secret"));
        assert!(!output_path.exists());
    }

    #[test]
    fn missing_explicit_reference_is_a_release_gate_failure() {
        let directory = tempfile::tempdir().expect("reference fixture directory");
        let output = run(
            &root().join("scripts/reference-jq-provision.sh"),
            &directory.path().join("missing-jq"),
            None,
        );
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("not usable"));
    }
}
