//! Host-memory discovery used only while constructing CLI invocation defaults.
//!
//! Lower-level library callers continue to receive deterministic resource
//! defaults. The CLI samples available memory once at argument initialization;
//! no per-record or per-token probing occurs.

use sysinfo::{MemoryRefreshKind, System};
#[cfg(target_os = "linux")]
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, get_current_pid};

/// Returns memory currently available to the process, when the operating
/// system can provide it.
///
/// On Linux, `sysinfo`'s process cgroup view contributes the remaining memory
/// in the current cgroup (including ancestor limits) when its standard
/// cgroup mount roots are available. Other platforms use the OS
/// available-memory result directly; unavailable cgroup data does not block
/// the host-memory probe.
#[must_use]
pub(crate) fn available_memory_bytes() -> Option<u64> {
    // A zero total means the platform probe did not populate memory data;
    // zero available with a nonzero total is a real constrained observation.
    let mut system = System::new();
    system.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
    let available = system.available_memory();
    if system.total_memory() == 0 {
        return None;
    }

    #[cfg(target_os = "linux")]
    let available = {
        let process_limits = get_current_pid().ok().and_then(|pid| {
            system.refresh_processes_specifics(
                ProcessesToUpdate::Some(&[pid]),
                true,
                ProcessRefreshKind::nothing(),
            );
            system
                .process(pid)
                .and_then(|process| process.cgroup_limits())
        });
        process_limits
            .or_else(|| system.cgroup_limits())
            .map_or(available, |limits| available.min(limits.free_memory))
    };
    Some(available)
}
