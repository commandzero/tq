//! Owns one child until its resource-aware reap. No ordinary wait may touch it.

use std::{
    io,
    process::Child,
    thread,
    time::{Duration, Instant},
};

use nix::{
    errno::Errno,
    sys::signal::{Signal, kill, killpg},
    unistd::Pid,
};
use rustix::process::{WaitId, WaitIdOptions, waitid};
use wait4::{ResUse, Wait4 as _};

/// Polling requests 100 microseconds; host scheduling accuracy is measured separately.
pub(super) const EXIT_POLL: Duration = Duration::from_micros(100);
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Eq, PartialEq)]
enum ProcessGroupOwnership {
    /// This owner may signal and verify the target's dedicated process group.
    Dedicated,
    /// The worker/coordinator owns group cleanup; this owner only handles its PID.
    Shared,
}

pub(super) struct NativeChild {
    child: Option<Child>,
    pid: Pid,
    process_group: Pid,
    group_ownership: ProcessGroupOwnership,
    exited: bool,
    termination_requested: bool,
    group_cleanup_verified: bool,
    observed_exit_code: Option<i32>,
    observed_signal: Option<i32>,
}

impl NativeChild {
    pub(super) fn new(child: Child) -> Self {
        Self::new_with_group(child, None)
    }

    /// Owns a target that shares a process group with an external lifecycle
    /// owner. The target is still reaped by its exact PID, but this owner must
    /// not signal or enumerate the shared group (which also contains the
    /// worker that will perform final cleanup).
    pub(super) fn new_in_shared_group(child: Child, group: Pid) -> Self {
        Self::new_with_group(child, Some(group))
    }

    fn new_with_group(child: Child, shared_group: Option<Pid>) -> Self {
        // All supported kernels use signed positive process identifiers.
        let pid = Pid::from_raw(i32::try_from(child.id()).expect("native child PID fits i32"));
        Self {
            child: Some(child),
            pid,
            process_group: shared_group.unwrap_or(pid),
            group_ownership: shared_group.map_or(ProcessGroupOwnership::Dedicated, |_| {
                ProcessGroupOwnership::Shared
            }),
            exited: false,
            termination_requested: false,
            group_cleanup_verified: false,
            observed_exit_code: None,
            observed_signal: None,
        }
    }

    pub(super) fn id(&self) -> u32 {
        u32::try_from(self.pid.as_raw()).expect("positive child PID")
    }

    /// Observe completion without reaping. The zombie reserves the PID/PGID
    /// until descendants have been signaled and wait4 collects resources.
    pub(super) fn observe_exit(&mut self) -> io::Result<bool> {
        if self.exited {
            return Ok(true);
        }
        let pid = rustix::process::Pid::from_raw(self.pid.as_raw())
            .ok_or_else(|| io::Error::other("invalid child PID"))?;
        loop {
            match waitid(
                WaitId::Pid(pid),
                WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
            ) {
                Ok(status) => {
                    self.exited = status.is_some();
                    if let Some(status) = status {
                        self.observed_exit_code = status.exit_status();
                        self.observed_signal = status.terminating_signal();
                    }
                    return Ok(self.exited);
                }
                Err(rustix::io::Errno::INTR) => {}
                Err(error) => return Err(error.into()),
            }
        }
    }

    #[allow(dead_code)]
    pub(super) fn observed_status(&self) -> (Option<i32>, Option<i32>) {
        (self.observed_exit_code, self.observed_signal)
    }

    pub(super) fn terminate(&mut self) -> io::Result<()> {
        // This is only called while this owner still holds the unreaped child.
        // Once a group signal succeeds, never issue it again: the child PID is
        // still reserved until wait4, but its group may otherwise be reused
        // after an unexpected external reap.
        if self.child.is_none() || self.termination_requested {
            return Ok(());
        }
        let deadline = Instant::now() + CLEANUP_TIMEOUT;
        loop {
            let signal_result = if self.group_ownership == ProcessGroupOwnership::Dedicated {
                killpg(self.process_group, Signal::SIGKILL)
            } else {
                kill(self.pid, Signal::SIGKILL)
            };
            match signal_result {
                Ok(()) | Err(Errno::ESRCH) => {
                    self.termination_requested = true;
                    return Ok(());
                }
                // macOS reports EPERM when WNOWAIT has left only the zombie
                // group leader, but it can also report EPERM for a live member
                // whose credentials deny the signal. Refresh the exact child
                // observation for a deadline/exit race. The post-exit cleanup
                // path verifies the group with libproc before allowing the reap.
                Err(Errno::EPERM) if cfg!(target_os = "macos") => {
                    if !self.exited && !self.observe_exit()? {
                        return Err(signal_error(self.process_group, Errno::EPERM));
                    }
                    self.termination_requested = true;
                    return Ok(());
                }
                Err(Errno::EINTR) if Instant::now() < deadline => {}
                Err(Errno::EINTR) => {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "process-group termination remained interrupted",
                    ));
                }
                Err(error) => return Err(signal_error(self.process_group, error)),
            }
        }
    }

    pub(super) fn finish(self) -> io::Result<ResUse> {
        if !self.exited {
            return Err(io::Error::other(
                "child must exit before resource collection",
            ));
        }
        if self.group_ownership == ProcessGroupOwnership::Dedicated {
            self.finish_after_cleanup(Self::ensure_group_cleanup)
        } else {
            self.finish_after_cleanup(|_| Ok(()))
        }
    }

    fn finish_after_cleanup<F>(mut self, cleanup: F) -> io::Result<ResUse>
    where
        F: FnOnce(&mut Self) -> io::Result<()>,
    {
        // A naturally exiting child may leave descendants in its dedicated
        // process group. Verify that group (and signal it when needed) while
        // the leader's PID is still reserved, then reap the exact leader once.
        let cleanup_result = cleanup(&mut self);
        // Move the handle out before the final wait so Drop cannot try to
        // inspect a process after its resource-bearing reap has completed.
        let child = self.child.take().expect("owned child");
        let reap_result = child.wait4();
        match (cleanup_result, reap_result) {
            (Ok(()), Ok(resources)) => Ok(resources),
            (Err(cleanup_error), Ok(_resources)) => Err(io::Error::other(format!(
                "process-group cleanup failed before exact-child reap: {cleanup_error}"
            ))),
            (Ok(()), Err(reap_error)) => Err(reap_error),
            (Err(cleanup_error), Err(reap_error)) => Err(io::Error::other(format!(
                "process-group cleanup failed: {cleanup_error}; exact-child reap failed: {reap_error}"
            ))),
        }
    }

    #[cfg(test)]
    fn finish_with_injected_cleanup_failure(self) -> io::Result<ResUse> {
        assert!(self.exited, "injected cleanup test requires observed exit");
        self.finish_after_cleanup(|_| Err(io::Error::other("injected cleanup failure")))
    }

    fn abort(&mut self) -> io::Result<()> {
        // Resolve a natural exit before signalling. This avoids treating a
        // macOS zombie-only process group as a live target and preserves the
        // exact PID until the owner performs its one resource-aware reap.
        self.observe_exit()?;
        let mut termination_error = None;
        if !self.exited {
            if let Err(error) = self.terminate() {
                // A signal failure does not by itself prove that the child is
                // still running. Refresh the exact PID and wait briefly for a
                // race with natural exit; if it did exit, retain the signal
                // diagnostic but continue to the one consuming reap below.
                termination_error = Some(error);
            }
            let deadline = Instant::now() + CLEANUP_TIMEOUT;
            while !self.observe_exit()? {
                if Instant::now() >= deadline {
                    return Err(termination_error.unwrap_or_else(|| {
                        io::Error::new(io::ErrorKind::TimedOut, "child did not exit after SIGKILL")
                    }));
                }
                thread::sleep(EXIT_POLL);
            }
        }
        let cleanup_result = self.ensure_group_cleanup();
        let child = self.child.take().expect("owned child");
        let reap_result = child.wait4().map(|_| ());
        match (termination_error, cleanup_result, reap_result) {
            (None, Ok(()), Ok(())) => Ok(()),
            (Some(termination_error), Ok(()), Ok(())) => Err(io::Error::other(format!(
                "process-group termination failed before exact-child reap: {termination_error}"
            ))),
            (None, Err(cleanup_error), Ok(())) => Err(io::Error::other(format!(
                "process-group cleanup failed before exact-child reap: {cleanup_error}"
            ))),
            (Some(termination_error), Err(cleanup_error), Ok(())) => {
                Err(io::Error::other(format!(
                    "process-group termination failed: {termination_error}; cleanup failed: {cleanup_error}"
                )))
            }
            (None, Ok(()), Err(reap_error)) => Err(reap_error),
            (Some(termination_error), Ok(()), Err(reap_error)) => Err(io::Error::other(format!(
                "process-group termination failed: {termination_error}; exact-child reap failed: {reap_error}"
            ))),
            (None, Err(cleanup_error), Err(reap_error)) => Err(io::Error::other(format!(
                "process-group cleanup failed: {cleanup_error}; exact-child reap failed: {reap_error}"
            ))),
            (Some(termination_error), Err(cleanup_error), Err(reap_error)) => {
                Err(io::Error::other(format!(
                    "process-group termination failed: {termination_error}; cleanup failed: {cleanup_error}; exact-child reap failed: {reap_error}"
                )))
            }
        }
    }

    fn ensure_group_cleanup(&mut self) -> io::Result<()> {
        if self.child.is_none()
            || self.group_cleanup_verified
            || self.group_ownership == ProcessGroupOwnership::Shared
        {
            // A shared-group target is owned only for exact-PID observation
            // and reap. Its worker owns group cleanup; signaling or querying
            // this group here could terminate the coordinator/worker itself.
            self.group_cleanup_verified = true;
            return Ok(());
        }

        #[cfg(target_os = "macos")]
        {
            self.ensure_macos_group_cleanup()
        }

        #[cfg(not(target_os = "macos"))]
        {
            // Linux has no supported safe process-group enumeration in this
            // helper. Preserve the killpg result as the cleanup observation;
            // macOS uses the stronger enumeration path below because XNU's
            // EPERM is ambiguous after WNOWAIT observes a zombie leader.
            if !self.termination_requested {
                self.terminate()?;
            }
            self.group_cleanup_verified = true;
            Ok(())
        }
    }

    #[cfg(target_os = "macos")]
    fn ensure_macos_group_cleanup(&mut self) -> io::Result<()> {
        let deadline = Instant::now() + CLEANUP_TIMEOUT;
        loop {
            match self.live_group_members() {
                Ok(live_members) if live_members.is_empty() => {
                    self.group_cleanup_verified = true;
                    return Ok(());
                }
                Ok(live_members) => {
                    if !self.termination_requested {
                        self.terminate()?;
                    }
                    if Instant::now() >= deadline {
                        return Err(live_members_error(self.pid, &live_members));
                    }
                }
                Err(error) => {
                    // The PID list may race with a descendant's transition
                    // to zombie, or BSDInfo may reject a credential-changed
                    // member. The group is still anchored by our unreaped
                    // leader, so make the bounded cleanup attempt even when
                    // inspection itself is temporarily unavailable. A
                    // persistent permission error remains visible below.
                    if !self.termination_requested {
                        self.terminate()?;
                    }
                    if Instant::now() >= deadline {
                        return Err(error);
                    }
                }
            }
            thread::sleep(EXIT_POLL);
        }
    }

    /// Return non-zombie members of this still-reserved process group.
    ///
    /// `proc_listpids(PROC_PGRP_ONLY)` includes the zombie list on Darwin, so
    /// each PID is inspected with `BSDInfo` (argument 1 requests zombie lookup)
    /// and `SZOMB` is excluded. A `BSDInfo` permission/error result is returned
    /// instead of being treated as an empty group: an unsignalable live
    /// descendant must remain a visible collection error.
    #[cfg(target_os = "macos")]
    fn live_group_members(&self) -> io::Result<Vec<u32>> {
        use libproc::{
            bsd_info::BSDInfo,
            proc_pid::pidinfo,
            processes::{ProcFilter, pids_by_type},
        };

        let group = self.id();
        let pids = pids_by_type(ProcFilter::ByProgramGroup { pgrpid: group }).map_err(|error| {
            io::Error::other(format!(
                "libproc process-group listing for pgid {group} failed: {error}"
            ))
        })?;
        let mut live = Vec::new();
        for pid in pids {
            // The leader is known to be exited from waitid and is allowed to
            // remain a zombie until the exact wait4 below. Never inspect a
            // potentially stale/reused leader PID after it leaves this owner.
            if pid == group {
                continue;
            }
            let pid_i32 = i32::try_from(pid).map_err(|_| {
                io::Error::other(format!("libproc returned out-of-range process PID {pid}"))
            })?;
            let info = pidinfo::<BSDInfo>(pid_i32, 1).map_err(|error| {
                io::Error::other(format!(
                    "libproc BSD info for process {pid} in pgid {group} failed: {error}"
                ))
            })?;
            // A PID can exit or change groups between listpids and pidinfo.
            // Ignore that snapshot entry and retry the group query instead of
            // signalling a potentially unrelated process.
            if info.pbi_pid != pid || info.pbi_pgid != group {
                continue;
            }
            if info.pbi_status != nix::libc::SZOMB {
                live.push(pid);
            }
        }
        Ok(live)
    }
}

fn signal_error(pid: Pid, error: Errno) -> io::Error {
    let source = io::Error::from_raw_os_error(error as i32);
    io::Error::new(
        source.kind(),
        format!("killpg(SIGKILL, pgid={}) failed: {source}", pid.as_raw()),
    )
}

#[cfg(target_os = "macos")]
fn live_members_error(pid: Pid, members: &[u32]) -> io::Error {
    io::Error::other(format!(
        "process-group cleanup for pgid {} left live members before exact-child reap: {members:?}",
        pid.as_raw()
    ))
}

impl Drop for NativeChild {
    fn drop(&mut self) {
        if self.child.is_some()
            && let Err(error) = self.abort()
        {
            eprintln!("tq-bench: child {} cleanup failed: {error}", self.pid);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{BufRead as _, BufReader},
        process::{Command, Stdio},
        thread,
        time::{Duration, Instant},
    };

    use super::NativeChild;

    fn spawn_grouped(script: &str) -> NativeChild {
        use std::os::unix::process::CommandExt as _;

        let mut command = Command::new("/bin/sh");
        command.args(["-c", script]).process_group(0);
        NativeChild::new(command.spawn().expect("spawn native-process fixture"))
    }

    fn spawn_grouped_with_stdout(script: &str) -> NativeChild {
        use std::os::unix::process::CommandExt as _;

        let mut command = Command::new("/bin/sh");
        command
            .args(["-c", script])
            .stdout(Stdio::piped())
            .process_group(0);
        NativeChild::new(command.spawn().expect("spawn native-process fixture"))
    }

    fn wait_for_exit(owner: &mut NativeChild) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !owner.observe_exit().expect("observe fixture exit") {
            assert!(
                Instant::now() < deadline,
                "fixture did not exit before deadline"
            );
            thread::sleep(super::EXIT_POLL);
        }
    }

    fn process_is_gone(pid: u32) -> bool {
        use nix::{errno::Errno, sys::signal::kill, unistd::Pid};

        matches!(
            kill(
                Pid::from_raw(i32::try_from(pid).expect("fixture PID fits i32")),
                None,
            ),
            Err(Errno::ESRCH)
        )
    }

    fn wait_until_gone(pid: u32) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !process_is_gone(pid) {
            assert!(Instant::now() < deadline, "process {pid} remained alive");
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn normal_exit_is_observed_and_reaped_once() {
        let mut owner = spawn_grouped("exit 7");
        wait_for_exit(&mut owner);
        assert_eq!(owner.observed_status(), (Some(7), None));
        let resources = owner.finish().expect("resource-aware reap");

        assert_eq!(resources.status.code(), Some(7));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_zombie_leader_is_not_a_live_group_member() {
        let mut owner = spawn_grouped("exit 0");
        wait_for_exit(&mut owner);

        assert!(
            owner
                .live_group_members()
                .expect("inspect reserved process group")
                .is_empty()
        );
        owner.finish().expect("resource-aware reap");
    }

    #[test]
    fn natural_exit_cleans_a_live_descendant_before_reaping_the_leader() {
        let mut owner =
            spawn_grouped_with_stdout("sleep 30 & child=$!; printf '%s\\n' \"$child\"; exit 0");
        let mut stdout = BufReader::new(
            owner
                .child
                .as_mut()
                .expect("owned fixture")
                .stdout
                .take()
                .expect("fixture stdout"),
        );
        let mut line = String::new();
        stdout.read_line(&mut line).expect("read descendant PID");
        let descendant = line.trim().parse::<u32>().expect("numeric descendant PID");
        wait_for_exit(&mut owner);
        owner
            .finish()
            .expect("resource-aware reap with descendant cleanup");

        drop(stdout);
        wait_until_gone(descendant);
    }

    #[test]
    fn forced_termination_is_idempotent_and_reaped_once() {
        use std::os::unix::process::ExitStatusExt as _;

        let mut owner = spawn_grouped("sleep 30");
        let pid = owner.id();
        owner.terminate().expect("first group termination");
        owner.terminate().expect("repeated group termination");
        wait_for_exit(&mut owner);
        assert_eq!(owner.observed_status(), (None, Some(9)));
        let resources = owner
            .finish()
            .expect("resource-aware reap after termination");

        assert_eq!(resources.status.signal(), Some(9));
        wait_until_gone(pid);
    }

    #[test]
    fn dropping_an_unobserved_child_terminates_and_reaps_within_bound() {
        let owner = spawn_grouped("sleep 30");
        let pid = owner.id();
        let started = Instant::now();
        drop(owner);

        assert!(started.elapsed() < Duration::from_secs(6));
        wait_until_gone(pid);
    }

    #[test]
    fn finish_before_observation_reports_an_owner_error_and_cleans_up() {
        let owner = spawn_grouped("sleep 30");
        let pid = owner.id();
        let error = owner
            .finish()
            .expect_err("unobserved child must not be reaped");

        assert!(
            error
                .to_string()
                .contains("must exit before resource collection")
        );
        wait_until_gone(pid);
    }

    #[test]
    fn external_reap_is_reported_as_echild_without_group_signal() {
        let mut owner = spawn_grouped("exit 19");
        let pid = owner.id();
        let status = owner
            .child
            .as_mut()
            .expect("owned fixture")
            .wait()
            .expect("external test reap");
        assert_eq!(status.code(), Some(19));

        let error = owner
            .observe_exit()
            .expect_err("an externally reaped child must be an owner error");
        assert_eq!(error.raw_os_error(), Some(nix::libc::ECHILD));
        assert!(!owner.termination_requested);
        assert!(process_is_gone(pid));
        // The handle has already consumed the child. Avoid asking Drop to
        // retry lifecycle observation after this deliberately invalid state.
        owner.child.take();
    }

    #[test]
    fn cleanup_failure_still_reaps_an_observed_leader() {
        let mut owner = spawn_grouped("exit 23");
        wait_for_exit(&mut owner);
        let pid = owner.id();

        let error = owner
            .finish_with_injected_cleanup_failure()
            .expect_err("injected cleanup failure must be visible");
        assert!(error.to_string().contains("injected cleanup failure"));
        assert!(
            process_is_gone(pid),
            "leader must be reaped after cleanup error"
        );
    }

    #[test]
    fn dropping_shared_target_does_not_signal_its_group_owner() {
        use nix::unistd::Pid;
        use std::os::unix::process::CommandExt as _;

        let mut leader_command = Command::new("/bin/sh");
        leader_command.args(["-c", "sleep 30"]).process_group(0);
        let leader_child = leader_command.spawn().expect("spawn shared-group leader");
        let leader_pid = leader_child.id();
        let group = Pid::from_raw(i32::try_from(leader_pid).expect("leader PID fits i32"));

        let mut target_command = Command::new("/bin/sh");
        target_command
            .args(["-c", "sleep 30"])
            .process_group(group.as_raw());
        let target_child = target_command.spawn().expect("spawn shared-group target");
        let target_pid = target_child.id();
        let leader = NativeChild::new(leader_child);
        let target = NativeChild::new_in_shared_group(target_child, group);

        drop(target);
        assert!(!process_is_gone(leader_pid));
        assert!(process_is_gone(target_pid));

        drop(leader);
        wait_until_gone(leader_pid);
    }
}
