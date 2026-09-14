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
use wait4::ResUse;

/// Polling requests 100 microseconds; host scheduling accuracy is measured separately.
pub(super) const EXIT_POLL: Duration = Duration::from_micros(100);
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(5);

/// Wait for a bounded polling interval without using macOS's relative
/// `nanosleep` path, which asserted on `EINVAL` during a campaign run.
///
/// Poll callers use fixed, short durations that fit the monotonic clock. The
/// deadline check also prevents a preexisting `unpark` token from shortening
/// the requested interval.
pub(super) fn poll_delay(delay: Duration) {
    #[cfg(target_os = "macos")]
    {
        if delay.is_zero() {
            return;
        }
        let deadline = Instant::now()
            .checked_add(delay)
            .expect("bounded polling delay fits monotonic clock");
        loop {
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return;
            };
            if remaining.is_zero() {
                return;
            }
            thread::park_timeout(remaining);
        }
    }

    #[cfg(not(target_os = "macos"))]
    thread::sleep(delay);
}

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
        if self.child.is_none() {
            return Err(io::Error::from_raw_os_error(nix::libc::ECHILD));
        }
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
                Err(error) if error == rustix::io::Errno::CHILD => {
                    // Another owner consumed the child. Remove this handle
                    // before returning so callers cannot signal a reused PID.
                    self.child.take();
                    return Err(io::Error::from_raw_os_error(nix::libc::ECHILD));
                }
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

    pub(super) fn finish(&mut self) -> io::Result<ResUse> {
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

    /// Verify group cleanup, then attempt one nonblocking resource-aware reap.
    ///
    /// Group cleanup may wait to its own bound on macOS, so callers should use
    /// this only on the cleanup executor rather than bounded caller polling.
    /// `try_wait4` is the only nonblocking operation permitted after the
    /// non-consuming `waitid` observation. A `None` result keeps the child in
    /// this owner for a later poll; a successful result removes it before any
    /// further signal can be attempted. `ECHILD` is likewise terminal:
    /// another owner consumed the child and this owner must not fall back to
    /// an ordinary wait or signal a potentially reused PID.
    pub(super) fn try_finish4(&mut self) -> io::Result<Option<ResUse>> {
        if !self.exited {
            return Err(io::Error::other(
                "child must exit before resource collection",
            ));
        }
        self.ensure_group_cleanup()?;
        self.try_reap4()
    }

    fn try_reap4(&mut self) -> io::Result<Option<ResUse>> {
        self.try_reap4_with(wait4::Wait4::try_wait4)
    }

    fn try_reap4_with<F>(&mut self, mut try_wait4: F) -> io::Result<Option<ResUse>>
    where
        F: FnMut(&mut Child) -> io::Result<Option<ResUse>>,
    {
        let Some(child) = self.child.as_mut() else {
            return Err(io::Error::from_raw_os_error(nix::libc::ECHILD));
        };
        let result = try_wait4(child);
        match result {
            Ok(Some(resources)) => {
                self.child.take();
                Ok(Some(resources))
            }
            Ok(None) => Ok(None),
            Err(error) if error.raw_os_error() == Some(nix::libc::ECHILD) => {
                self.child.take();
                Err(error)
            }
            Err(error) => Err(error),
        }
    }

    fn reap_until_deadline(&mut self) -> io::Result<ResUse> {
        self.reap_until_deadline_with(wait4::Wait4::try_wait4)
    }

    fn reap_until_deadline_with<F>(&mut self, mut try_wait4: F) -> io::Result<ResUse>
    where
        F: FnMut(&mut Child) -> io::Result<Option<ResUse>>,
    {
        self.reap_until_deadline_at(Instant::now() + CLEANUP_TIMEOUT, &mut try_wait4)
    }

    fn reap_until_deadline_at<F>(
        &mut self,
        deadline: Instant,
        mut try_wait4: F,
    ) -> io::Result<ResUse>
    where
        F: FnMut(&mut Child) -> io::Result<Option<ResUse>>,
    {
        let mut recoverable_error = None;
        loop {
            match self.try_reap4_with(&mut try_wait4) {
                Ok(Some(resources)) => return Ok(resources),
                Ok(None) => {}
                Err(error) if error.raw_os_error() == Some(nix::libc::ECHILD) => {
                    return Err(error);
                }
                Err(error) => recoverable_error = Some(error),
            }
            if Instant::now() >= deadline {
                return Err(recoverable_error.unwrap_or_else(|| {
                    io::Error::new(io::ErrorKind::TimedOut, "exact-child reap remained pending")
                }));
            }
            poll_delay(EXIT_POLL);
        }
    }

    fn finish_after_cleanup<F>(&mut self, cleanup: F) -> io::Result<ResUse>
    where
        F: FnOnce(&mut Self) -> io::Result<()>,
    {
        // A naturally exiting child may leave descendants in its dedicated
        // process group. Verify that group (and signal it when needed) while
        // the leader's PID is still reserved, then reap the exact leader once.
        let cleanup_result = cleanup(self);
        let Err(cleanup_error) = cleanup_result else {
            return self.reap_until_deadline();
        };
        // Do not consume the exact-child wait owner when group cleanup did not
        // verify. The caller (or its deferred owner) must be able to retry
        // cleanup before the leader is reaped and its process-group anchor is
        // lost.
        Err(io::Error::other(format!(
            "process-group cleanup failed before exact-child reap: {cleanup_error}"
        )))
    }

    #[cfg(test)]
    fn finish_with_injected_cleanup_failure(&mut self) -> io::Result<ResUse> {
        assert!(self.exited, "injected cleanup test requires observed exit");
        self.finish_after_cleanup(|_| Err(io::Error::other("injected cleanup failure")))
    }

    fn abort(&mut self) -> io::Result<()> {
        // Resolve a natural exit before signalling. This avoids treating a
        // macOS zombie-only process group as a live target and preserves the
        // exact PID until the owner performs its one resource-aware reap.
        if let Err(error) = self.observe_exit() {
            if error.raw_os_error() == Some(nix::libc::ECHILD) {
                // Another owner consumed the child. Drop only the local
                // handle; never signal a PID after ownership was lost.
                self.child.take();
            }
            return Err(error);
        }
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
            loop {
                match self.observe_exit() {
                    Ok(true) => break,
                    Ok(false) => {}
                    Err(error) => {
                        return Err(match termination_error.take() {
                            Some(termination_error) => io::Error::other(format!(
                                "process-group termination failed: {termination_error}; observe child exit: {error}"
                            )),
                            None => error,
                        });
                    }
                }
                if Instant::now() >= deadline {
                    return Err(termination_error.take().unwrap_or_else(|| {
                        io::Error::new(io::ErrorKind::TimedOut, "child did not exit after SIGKILL")
                    }));
                }
                poll_delay(EXIT_POLL);
            }
        }
        let cleanup_result = self.ensure_group_cleanup();
        if let Err(cleanup_error) = cleanup_result {
            return Err(match termination_error {
                Some(termination_error) => io::Error::other(format!(
                    "process-group termination failed: {termination_error}; cleanup failed: {cleanup_error}"
                )),
                None => io::Error::other(format!(
                    "process-group cleanup failed before exact-child reap: {cleanup_error}"
                )),
            });
        }

        let reap_result = self.reap_until_deadline().map(|_| ());
        match (termination_error, reap_result) {
            (None, Ok(())) => Ok(()),
            (Some(termination_error), Ok(())) => Err(io::Error::other(format!(
                "process-group termination failed before exact-child reap: {termination_error}"
            ))),
            (None, Err(reap_error)) => Err(reap_error),
            (Some(termination_error), Err(reap_error)) => Err(io::Error::other(format!(
                "process-group termination failed: {termination_error}; exact-child reap failed: {reap_error}"
            ))),
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
            poll_delay(EXIT_POLL);
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
        // Retain the exact wait owner until it has reaped the child. This can
        // extend a worker's lifetime after its caller's bounded deadline, but
        // dropping the handle after a recoverable wait error would abandon
        // the only owner that can collect exact resources. ECHILD is terminal
        // and abort() clears the local handle before returning it.
        let mut reported = false;
        while self.child.is_some() {
            if let Err(error) = self.abort() {
                if !reported {
                    eprintln!("tq-bench: child {} cleanup pending: {error}", self.pid);
                    reported = true;
                }
                if self.child.is_some() {
                    poll_delay(EXIT_POLL);
                }
            }
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
    use wait4::Wait4 as _;

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

    fn assert_reaped(pid: u32) {
        let pid = rustix::process::Pid::from_raw(
            i32::try_from(pid).expect("fixture PID fits process identifier"),
        )
        .expect("fixture PID is positive");
        let status = rustix::process::waitid(
            rustix::process::WaitId::Pid(pid),
            rustix::process::WaitIdOptions::EXITED
                | rustix::process::WaitIdOptions::NOHANG
                | rustix::process::WaitIdOptions::NOWAIT,
        );
        assert!(
            matches!(status, Err(rustix::io::Errno::CHILD)),
            "exact child remained waitable after reap: {status:?}"
        );
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn poll_delay_reaches_deadline_after_preexisting_unpark() {
        thread::current().unpark();
        let delay = Duration::from_millis(20);
        let started = Instant::now();

        super::poll_delay(delay);

        let elapsed = started.elapsed();
        assert!(elapsed >= delay, "poll delay returned early: {elapsed:?}");
        assert!(
            elapsed < Duration::from_secs(1),
            "poll delay exceeded bound: {elapsed:?}"
        );
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn poll_delay_zero_returns_immediately() {
        let started = Instant::now();
        super::poll_delay(Duration::ZERO);
        assert!(started.elapsed() < Duration::from_millis(100));
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn poll_delay_completes_within_bounded_interval() {
        let delay = Duration::from_millis(20);
        let started = Instant::now();

        super::poll_delay(delay);

        let elapsed = started.elapsed();
        assert!(elapsed >= delay, "poll delay returned early: {elapsed:?}");
        assert!(
            elapsed < Duration::from_secs(1),
            "poll delay exceeded bound: {elapsed:?}"
        );
    }

    #[test]
    fn normal_exit_is_observed_and_reaped_once() {
        let mut owner = spawn_grouped("exit 7");
        let pid = owner.id();
        wait_for_exit(&mut owner);
        assert_eq!(owner.observed_status(), (Some(7), None));
        let resources = owner.finish().expect("resource-aware reap");

        assert_eq!(resources.status.code(), Some(7));
        assert_reaped(pid);
    }

    #[test]
    fn recoverable_reap_error_retains_owner_for_later_exact_reap() {
        let mut owner = spawn_grouped("exit 17");
        let pid = owner.id();
        wait_for_exit(&mut owner);

        let error = owner
            .try_reap4_with(|_| Err(std::io::Error::from(std::io::ErrorKind::WouldBlock)))
            .expect_err("injected recoverable reap error");
        assert_eq!(error.kind(), std::io::ErrorKind::WouldBlock);
        assert!(owner.child.is_some(), "recoverable error lost wait owner");

        let resources = owner.finish().expect("retry exact-child reap");
        assert_eq!(resources.status.code(), Some(17));
        assert!(
            owner.child.is_none(),
            "successful reap retained child handle"
        );
        assert_reaped(pid);
    }

    #[test]
    fn delayed_reap_none_retains_owner_until_exact_reap() {
        let mut owner = spawn_grouped("exit 18");
        let pid = owner.id();
        wait_for_exit(&mut owner);
        let mut attempts = 0;

        let resources = owner
            .reap_until_deadline_with(|child| {
                attempts += 1;
                if attempts == 1 {
                    Ok(None)
                } else {
                    child.try_wait4()
                }
            })
            .expect("delayed exact-child reap");

        assert!(attempts >= 2, "reap did not retry after a pending result");
        assert_eq!(resources.status.code(), Some(18));
        assert!(
            owner.child.is_none(),
            "successful reap retained child handle"
        );
        assert_reaped(pid);
    }

    #[test]
    fn reap_deadline_expiry_retains_owner_for_later_exact_reap() {
        let mut owner = spawn_grouped("exit 20");
        let pid = owner.id();
        wait_for_exit(&mut owner);

        let error = owner
            .reap_until_deadline_at(Instant::now(), |_| Ok(None))
            .expect_err("expired reap deadline must remain an error");
        assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
        assert!(owner.child.is_some(), "deadline expiry lost wait owner");

        owner
            .finish()
            .expect("retry exact-child reap after timeout");
        assert_reaped(pid);
    }

    #[test]
    fn echild_reap_clears_owner_without_signalling() {
        let mut owner = spawn_grouped("exit 19");
        let pid = owner.id();
        wait_for_exit(&mut owner);

        let error = owner
            .try_reap4_with(|child| {
                // Simulate another owner winning the consuming wait. This is
                // test-only injection; production always calls try_wait4.
                child.wait().expect("external test reap");
                Err(std::io::Error::from_raw_os_error(nix::libc::ECHILD))
            })
            .expect_err("external reap must remain an ownership error");
        assert_eq!(error.raw_os_error(), Some(nix::libc::ECHILD));
        assert!(
            owner.child.is_none(),
            "ECHILD retained a stale child handle"
        );
        assert!(!owner.termination_requested);
        assert!(process_is_gone(pid));
        assert_reaped(pid);
        assert_eq!(
            owner
                .observe_exit()
                .expect_err("empty owner must remain terminal")
                .raw_os_error(),
            Some(nix::libc::ECHILD)
        );
        assert_eq!(
            owner
                .try_finish4()
                .expect_err("empty owner must not attempt another reap")
                .raw_os_error(),
            Some(nix::libc::ECHILD)
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_zombie_leader_is_not_a_live_group_member() {
        let mut owner = spawn_grouped("exit 0");
        wait_for_exit(&mut owner);

        assert_eq!(
            owner
                .live_group_members()
                .expect("inspect reserved process group"),
            [] as [u32; 0]
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
        let mut owner = spawn_grouped("sleep 30");
        let pid = owner.id();
        let error = owner
            .finish()
            .expect_err("unobserved child must not be reaped");

        assert!(
            error
                .to_string()
                .contains("must exit before resource collection")
        );
        assert!(owner.child.is_some(), "failed finish dropped wait owner");
        drop(owner);
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
    fn cleanup_failure_retains_group_anchor_until_retry() {
        let mut owner =
            spawn_grouped_with_stdout("sleep 30 & child=$!; printf '%s\\n' \"$child\"; exit 23");
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
        let pid = owner.id();

        let error = owner
            .finish_with_injected_cleanup_failure()
            .expect_err("injected cleanup failure must be visible");
        assert!(error.to_string().contains("injected cleanup failure"));
        assert!(
            owner.child.is_some(),
            "cleanup error must retain the exact-child wait owner"
        );
        assert!(
            !process_is_gone(descendant),
            "cleanup error must retain the live process-group anchor"
        );

        owner
            .finish()
            .expect("retry group cleanup and exact-child reap");
        drop(stdout);
        assert_reaped(pid);
        wait_until_gone(descendant);
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
