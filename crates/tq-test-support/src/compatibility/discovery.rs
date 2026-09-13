//! Configurable executable discovery and identity capture.

use std::{
    collections::BTreeMap,
    env, fs, io,
    path::{Path, PathBuf},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::process::{Invocation, ProcessError, ProcessStatus, run_process};
use crate::corpus::ArtifactIdentity;

/// Tool role in the compatibility matrix.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolKind {
    /// jq semantic reference.
    Jq,
    /// yq compatibility peer.
    Yq,
    /// tq implementation under test.
    Tq,
}

impl ToolKind {
    fn executable_name(self) -> &'static str {
        match self {
            Self::Jq => "jq",
            Self::Yq => "yq",
            Self::Tq => "tq",
        }
    }
}

/// Explicit executable overrides, normally populated from environment variables.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExecutableConfig {
    /// jq executable override.
    pub jq: Option<PathBuf>,
    /// yq executable override.
    pub yq: Option<PathBuf>,
    /// tq executable override.
    pub tq: Option<PathBuf>,
}

impl ExecutableConfig {
    /// Reads `TQ_JQ`, `TQ_YQ`, and `TQ_BIN` overrides.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            jq: env::var_os("TQ_JQ").map(PathBuf::from),
            yq: env::var_os("TQ_YQ").map(PathBuf::from),
            tq: env::var_os("TQ_BIN").map(PathBuf::from),
        }
    }

    fn explicit(&self, kind: ToolKind) -> Option<&Path> {
        match kind {
            ToolKind::Jq => self.jq.as_deref(),
            ToolKind::Yq => self.yq.as_deref(),
            ToolKind::Tq => self.tq.as_deref(),
        }
    }
}

/// Immutable executable identity recorded in baselines and reports.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolIdentity {
    /// Tool role.
    pub tool: ToolKind,
    /// Canonical executable path.
    pub path: PathBuf,
    /// Trimmed version output.
    pub version: String,
    /// Executable byte identity.
    pub executable: ArtifactIdentity,
    /// Build-feature observations; empty until tool-specific probes add them.
    pub build_features: Vec<String>,
    /// Hashes of dynamically linked runtime libraries, when the platform exposes them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runtime_libraries: Vec<ArtifactIdentity>,
}

/// Stable discovery and identity failures.
#[derive(Debug, Error)]
pub enum ToolDiscoveryError {
    /// Explicit override does not identify an executable file.
    #[error("configured {tool:?} executable is not usable: {path}")]
    InvalidOverride {
        /// Tool role.
        tool: ToolKind,
        /// Rejected path.
        path: PathBuf,
    },
    /// Filesystem identity failed.
    #[error("tool identity I/O failed: {0}")]
    Io(#[from] io::Error),
    /// Version subprocess failed at the harness boundary.
    #[error(transparent)]
    Process(#[from] ProcessError),
    /// Version command timed out, was signaled, or exited nonzero.
    #[error("{tool:?} version command failed: {status:?} exit={exit_code:?}")]
    VersionFailed {
        /// Tool role.
        tool: ToolKind,
        /// Completion status.
        status: ProcessStatus,
        /// Exit code, when available.
        exit_code: Option<i32>,
    },
}

/// Discovers and identifies a tool, returning `None` when no candidate exists.
///
/// Explicit configuration is authoritative and invalid overrides return an
/// error. Otherwise local sibling builds and repository-local reference builds
/// are preferred before `PATH`.
///
/// # Errors
///
/// Returns an override, filesystem, or version-capture error.
pub fn discover_tool(
    kind: ToolKind,
    config: &ExecutableConfig,
    repository_root: &Path,
) -> Result<Option<ToolIdentity>, ToolDiscoveryError> {
    let path = if let Some(explicit) = config.explicit(kind) {
        if !is_executable(explicit) {
            return Err(ToolDiscoveryError::InvalidOverride {
                tool: kind,
                path: explicit.to_owned(),
            });
        }
        explicit.to_owned()
    } else {
        candidates(kind, repository_root)
            .into_iter()
            .find(|candidate| is_executable(candidate))
            .or_else(|| find_in_path(kind.executable_name()))
            .unwrap_or_default()
    };
    if path.as_os_str().is_empty() {
        return Ok(None);
    }
    let path = fs::canonicalize(path)?;
    let bytes = fs::read(&path)?;
    let digest = Sha256::digest(&bytes);
    let sha256 = digest
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            use std::fmt::Write as _;
            write!(hex, "{byte:02x}").expect("write digest to string");
            hex
        });
    let outcome = run_process(&Invocation {
        executable: path.clone(),
        args: vec!["--version".to_owned()],
        stdin: Vec::new(),
        timeout: Duration::from_secs(5),
        current_dir: Some(repository_root.to_owned()),
        environment: BTreeMap::new(),
    })?;
    if outcome.status != ProcessStatus::Exited || outcome.exit_code != Some(0) {
        return Err(ToolDiscoveryError::VersionFailed {
            tool: kind,
            status: outcome.status,
            exit_code: outcome.exit_code,
        });
    }
    let version_bytes = if outcome.stdout.is_empty() {
        &outcome.stderr
    } else {
        &outcome.stdout
    };
    let version = String::from_utf8_lossy(version_bytes).trim().to_owned();
    let build_features = capture_build_features(kind, &path, repository_root);
    let runtime_libraries = capture_runtime_libraries(kind, &path, repository_root);
    Ok(Some(ToolIdentity {
        tool: kind,
        path: path.clone(),
        version,
        executable: ArtifactIdentity {
            path: path.display().to_string(),
            bytes: u64::try_from(bytes.len())
                .map_err(|_| io::Error::other("executable length does not fit in u64"))?,
            sha256,
        },
        build_features,
        runtime_libraries,
    }))
}

fn capture_runtime_libraries(
    kind: ToolKind,
    path: &Path,
    repository_root: &Path,
) -> Vec<ArtifactIdentity> {
    if kind != ToolKind::Jq {
        return Vec::new();
    }
    #[cfg(target_os = "linux")]
    let probe = ("ldd", vec![path.display().to_string()]);
    #[cfg(target_os = "macos")]
    let probe = ("otool", vec!["-L".to_owned(), path.display().to_string()]);
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    let _ = (path, repository_root);
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    return Vec::new();

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let Ok(outcome) = run_process(&Invocation {
        executable: PathBuf::from(probe.0),
        args: probe.1,
        stdin: Vec::new(),
        timeout: Duration::from_secs(5),
        current_dir: Some(repository_root.to_owned()),
        environment: BTreeMap::new(),
    }) else {
        return Vec::new();
    };
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    if outcome.status != ProcessStatus::Exited || outcome.exit_code != Some(0) {
        return Vec::new();
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let paths = runtime_paths(&outcome.stdout);
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    paths
        .into_iter()
        .filter_map(|path| artifact_identity(&path))
        .collect()
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn runtime_paths(output: &[u8]) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for (line_number, line) in String::from_utf8_lossy(output).lines().enumerate() {
        #[cfg(target_os = "linux")]
        let _ = line_number;
        #[cfg(target_os = "linux")]
        let candidate = line
            .split_once("=>")
            .map_or(line, |(_, rest)| rest)
            .split_whitespace()
            .next();
        #[cfg(target_os = "macos")]
        let candidate = (line_number > 0)
            .then(|| line.split_whitespace().next())
            .flatten();
        if let Some(candidate) = candidate.filter(|path| path.starts_with('/')) {
            let path = PathBuf::from(candidate);
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
    }
    paths
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn artifact_identity(path: &Path) -> Option<ArtifactIdentity> {
    let canonical = fs::canonicalize(path).ok()?;
    let bytes = fs::read(&canonical).ok()?;
    Some(ArtifactIdentity {
        path: canonical.display().to_string(),
        bytes: u64::try_from(bytes.len()).ok()?,
        sha256: crate::compatibility::encode_hex(&Sha256::digest(&bytes)),
    })
}

fn capture_build_features(kind: ToolKind, path: &Path, repository_root: &Path) -> Vec<String> {
    let args = match kind {
        ToolKind::Jq => vec!["--build-configuration".to_owned()],
        ToolKind::Yq => vec!["--help".to_owned()],
        ToolKind::Tq => return Vec::new(),
    };
    let Ok(outcome) = run_process(&Invocation {
        executable: path.to_owned(),
        args,
        stdin: Vec::new(),
        timeout: Duration::from_secs(5),
        current_dir: Some(repository_root.to_owned()),
        environment: BTreeMap::new(),
    }) else {
        return Vec::new();
    };
    if outcome.status != ProcessStatus::Exited || outcome.exit_code != Some(0) {
        return Vec::new();
    }
    let output = String::from_utf8_lossy(&outcome.stdout);
    match kind {
        ToolKind::Jq => output
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect(),
        ToolKind::Yq => output
            .lines()
            .map(str::trim)
            .filter(|line| {
                line.contains("--input-format")
                    || line.contains("--output-format")
                    || line.contains("--yaml-")
            })
            .map(str::to_owned)
            .collect(),
        ToolKind::Tq => Vec::new(),
    }
}

fn candidates(kind: ToolKind, root: &Path) -> Vec<PathBuf> {
    let sibling = root.parent().unwrap_or(root);
    match kind {
        ToolKind::Jq => vec![
            sibling.join("jq/jq"),
            root.join("target/reference-build/jq/jq"),
        ],
        ToolKind::Yq => vec![
            sibling.join("yq/yq"),
            root.join("target/reference-build/yq/yq"),
        ],
        ToolKind::Tq => vec![root.join("target/release/tq"), root.join("target/debug/tq")],
    }
}

fn find_in_path(name: &str) -> Option<PathBuf> {
    env::var_os("PATH")
        .into_iter()
        .flat_map(|path| env::split_paths(&path).collect::<Vec<_>>())
        .map(|directory| directory.join(name))
        .find(|candidate| is_executable(candidate))
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    metadata.is_file() && executable_permissions(&metadata)
}

#[cfg(unix)]
fn executable_permissions(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn executable_permissions(_metadata: &fs::Metadata) -> bool {
    true
}
