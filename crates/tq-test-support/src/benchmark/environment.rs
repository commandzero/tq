//! Local benchmark host identity collection.

use std::process::Command;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Host and compiler identity attached to every campaign.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct EnvironmentManifest {
    /// UTC collection instant.
    pub collected_at: String,
    /// Operating system family.
    pub os: String,
    /// Kernel release/version.
    pub kernel: Option<String>,
    /// CPU architecture.
    pub architecture: String,
    /// Logical CPU count.
    pub logical_cpus: Option<usize>,
    /// Physical CPU count when observable.
    pub physical_cpus: Option<u64>,
    /// CPU model when observable.
    pub cpu_model: Option<String>,
    /// Total physical memory when observable.
    pub memory_bytes: Option<u64>,
    /// Filesystem identity for the repository volume.
    pub filesystem: Option<String>,
    /// Power/performance settings when observable.
    pub power_settings: Option<String>,
    /// tq compiler profile.
    pub compiler_profile: String,
    /// Stable hash excluding collection time.
    pub machine_identity: String,
}

/// Collects host fields and leaves unsupported fields as `None`.
#[must_use]
pub fn collect_environment(compiler_profile: &str) -> EnvironmentManifest {
    let mut manifest = EnvironmentManifest {
        collected_at: jiff::Timestamp::now().to_string(),
        os: std::env::consts::OS.to_owned(),
        kernel: command_output("uname", &["-srv"]),
        architecture: std::env::consts::ARCH.to_owned(),
        logical_cpus: std::thread::available_parallelism().ok().map(usize::from),
        physical_cpus: platform_number("hw.physicalcpu"),
        cpu_model: platform_text("machdep.cpu.brand_string")
            .or_else(|| linux_cpu_field("model name")),
        memory_bytes: platform_number("hw.memsize").or_else(linux_memory_bytes),
        filesystem: command_output("df", &["-T", "."]),
        power_settings: command_output("pmset", &["-g", "custom"]),
        compiler_profile: compiler_profile.to_owned(),
        machine_identity: String::new(),
    };
    manifest.machine_identity = identity(&manifest);
    manifest
}

fn command_output(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn platform_text(key: &str) -> Option<String> {
    command_output("sysctl", &["-n", key]).filter(|value| !value.is_empty())
}

fn platform_number(key: &str) -> Option<u64> {
    platform_text(key)?.parse().ok()
}

fn linux_cpu_field(field: &str) -> Option<String> {
    let contents = std::fs::read_to_string("/proc/cpuinfo").ok()?;
    contents.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        (name.trim() == field).then(|| value.trim().to_owned())
    })
}

fn linux_memory_bytes() -> Option<u64> {
    let contents = std::fs::read_to_string("/proc/meminfo").ok()?;
    let kibibytes = contents.lines().find_map(|line| {
        let suffix = line.strip_prefix("MemTotal:")?;
        suffix.split_whitespace().next()?.parse::<u64>().ok()
    })?;
    kibibytes.checked_mul(1024)
}

fn identity(manifest: &EnvironmentManifest) -> String {
    let stable = serde_json::json!({
        "os": manifest.os,
        "kernel": manifest.kernel,
        "architecture": manifest.architecture,
        "logical_cpus": manifest.logical_cpus,
        "physical_cpus": manifest.physical_cpus,
        "cpu_model": manifest.cpu_model,
        "memory_bytes": manifest.memory_bytes,
        "filesystem": manifest.filesystem,
        "power_settings": manifest.power_settings,
        "compiler_profile": manifest.compiler_profile,
    });
    let digest = Sha256::digest(serde_json::to_vec(&stable).expect("serialize machine identity"));
    digest
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            use std::fmt::Write as _;
            write!(hex, "{byte:02x}").expect("write digest to string");
            hex
        })
}
