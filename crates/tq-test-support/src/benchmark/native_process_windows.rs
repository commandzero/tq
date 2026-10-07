//! Retained-handle Windows accounting and suspended Job Object containment.

use std::{io, process::ExitStatus, sync::Arc, time::Duration};

use process_wrap::tokio::{ChildWrapper, CommandWrap, CommandWrapper, JobObject, KillOnDrop};
use win32job::{ExtendedLimitInfo, Job};
use winsafe::{HPROCESS, co, guard::CloseHandleGuard};

pub(super) const EXIT_POLL: Duration = Duration::from_micros(100);
pub(super) const RSS_SAMPLE_INTERVAL: Duration = Duration::from_millis(25);

#[cfg(test)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum JobFault {
    Query,
    Terminate,
    ExitObservation,
    ExitObservationWithoutAccounting,
}

#[cfg(test)]
thread_local! {
    pub(super) static JOB_FAULT: std::cell::Cell<Option<JobFault>> = const { std::cell::Cell::new(None) };
}

#[derive(Debug)]
struct InspectionJob(Arc<Job>);

impl CommandWrapper for InspectionJob {
    fn post_spawn(
        &mut self,
        _command: &mut tokio::process::Command,
        child: &mut tokio::process::Child,
        _core: &CommandWrap,
    ) -> io::Result<()> {
        // JobObject's pre_spawn suspends the initial thread. All post_spawn
        // hooks precede its wrap_child assignment/resume, so no target code
        // can create a descendant outside this inspection job.
        let handle = child
            .raw_handle()
            .ok_or_else(|| io::Error::other("spawned child has no process handle"))?;
        self.0.assign_process(handle as isize).map_err(io_error)
    }
}

pub(super) struct PreparedChild {
    command: CommandWrap,
    job: Arc<Job>,
}

impl PreparedChild {
    pub(super) fn spawn(mut self) -> io::Result<NativeChild> {
        let child = self.command.spawn()?;
        let pid = child
            .id()
            .ok_or_else(|| io::Error::other("spawned child has no process identity"))?;
        // Acquire before try_wait can release Tokio's original PID anchor.
        let accounting = open_process(pid);
        #[cfg(test)]
        let accounting = if JOB_FAULT.with(std::cell::Cell::get)
            == Some(JobFault::ExitObservationWithoutAccounting)
        {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "injected accounting access failure",
            ))
        } else {
            accounting
        };
        // Release the wrapper's InspectionJob clone explicitly: NativeChild
        // must own the last inspection handle for the close fallback to work.
        drop(self.command);
        Ok(NativeChild {
            child,
            accounting,
            job: Some(self.job),
            status: None,
            #[cfg(test)]
            fault: JOB_FAULT.with(std::cell::Cell::get),
        })
    }
}

pub(super) struct NativeChild {
    child: Box<dyn ChildWrapper>,
    accounting: io::Result<CloseHandleGuard<HPROCESS>>,
    job: Option<Arc<Job>>,
    status: Option<ExitStatus>,
    #[cfg(test)]
    fault: Option<JobFault>,
}

pub(super) struct Resources {
    pub user: Duration,
    pub system: Duration,
    pub peak_working_set: u64,
}

impl NativeChild {
    pub(super) fn prepare(command: tokio::process::Command) -> io::Result<PreparedChild> {
        let mut limits = ExtendedLimitInfo::new();
        limits.limit_kill_on_job_close();
        let job = Arc::new(Job::create_with_limit_info(&limits).map_err(io_error)?);
        let mut command = CommandWrap::from(command);
        command
            .wrap(KillOnDrop)
            .wrap(JobObject)
            .wrap(InspectionJob(Arc::clone(&job)));
        Ok(PreparedChild { command, job })
    }

    pub(super) fn observe_exit(&mut self) -> io::Result<Option<ExitStatus>> {
        if self.status.is_none() {
            if self.job.is_none()
                && let Ok(process) = &self.accounting
            {
                // The fallback must not depend on the failed job API. Wait on
                // the pinned root before reading its code (259 is a valid exit).
                use std::os::windows::process::ExitStatusExt as _;
                if process_has_exited(process)? {
                    self.status = Some(ExitStatus::from_raw(
                        process.GetExitCodeProcess().map_err(io_error)?,
                    ));
                }
            } else if self.job.is_none() {
                // Even if the independent accounting handle was unavailable,
                // the retained Tokio child still owns the exact root handle.
                // NativeChild::prepare's outer JobObject wrapper adds a failing
                // completion-port query to try_wait; bypass only that query,
                // without consuming the wrapper or closing its job guard before
                // the root has been reaped and resource collection attempted.
                self.status = self.child.inner_mut().try_wait()?;
            } else {
                #[cfg(test)]
                if matches!(
                    self.fault,
                    Some(JobFault::ExitObservation | JobFault::ExitObservationWithoutAccounting)
                ) {
                    return Err(io::Error::other(
                        "injected persistent job exit observation error",
                    ));
                }
                self.status = self.child.try_wait()?;
            }
        }
        Ok(self.status)
    }

    pub(super) fn close_inspection_job(&mut self) {
        // PreparedChild::spawn has dropped CommandWrap and its InspectionJob
        // clone before returning this owner. This is the final inspection job
        // handle; closing it kills the assigned tree, independently of the
        // process-wrap termination/query APIs. Keep child and exact accounting
        // handle alive until root exit and resource collection.
        drop(self.job.take());
    }

    pub(super) fn terminate(&mut self) -> io::Result<()> {
        // This is a retained job handle, never a recycled PID. The wrapper's
        // start_kill is nonblocking and terminates the entire nested job tree.
        #[cfg(test)]
        if self.fault == Some(JobFault::Terminate) {
            return Err(io::Error::other(
                "injected persistent job termination error",
            ));
        }
        self.child.start_kill()
    }

    pub(super) fn tree_is_empty(&self) -> io::Result<bool> {
        #[cfg(test)]
        if self.fault == Some(JobFault::Query) {
            return Err(io::Error::other("injected persistent job query error"));
        }
        Ok(self
            .job
            .as_ref()
            .ok_or_else(|| io::Error::other("inspection job closed for fail-safe cleanup"))?
            .query_process_id_list()
            .map_err(io_error)?
            .is_empty())
    }

    pub(super) fn resources(&self) -> io::Result<Resources> {
        if self.status.is_none() {
            return Err(io::Error::other(
                "resource collection requires observed exit",
            ));
        }
        let process = self.accounting.as_ref().map_err(|error| {
            io::Error::other(format!("acquire exact-child accounting handle: {error}"))
        })?;
        // Both queries remain fallible. Legitimate zero CPU time is not
        // confused with an API failure, unlike wait4's Windows conversion.
        let (_, _, kernel, user) = process.GetProcessTimes().map_err(io_error)?;
        let memory = process.GetProcessMemoryInfo().map_err(io_error)?;
        let peak_working_set = u64::try_from(memory.PeakWorkingSetSize).map_err(io_error)?;
        if peak_working_set == 0 {
            return Err(io::Error::other(
                "OS peak working set is unavailable or zero",
            ));
        }
        Ok(Resources {
            user: filetime_duration(u64::from(user)),
            system: filetime_duration(u64::from(kernel)),
            peak_working_set,
        })
    }

    pub(super) fn sampled_tree_working_set(&self) -> io::Result<u64> {
        let job = self
            .job
            .as_ref()
            .ok_or_else(|| io::Error::other("inspection job closed"))?;
        let mut total = 0_u64;
        for pid in job.query_process_id_list().map_err(io_error)? {
            let pid32 = u32::try_from(pid).map_err(io_error)?;
            let process = match open_process(pid32) {
                Ok(process) => process,
                Err(error) => {
                    if job
                        .query_process_id_list()
                        .map_err(io_error)?
                        .contains(&pid)
                    {
                        return Err(error);
                    }
                    continue;
                }
            };
            // Enumeration/open races cannot authorize an unrelated process:
            // reacquire membership after opening the handle. The newly held
            // handle pins this identity while we query its working set.
            if !job
                .query_process_id_list()
                .map_err(io_error)?
                .contains(&pid)
            {
                continue;
            }
            if process.WaitForSingleObject(Some(0)).map_err(io_error)? == co::WAIT::OBJECT_0 {
                continue;
            }
            let memory = match process.GetProcessMemoryInfo() {
                Ok(memory) => memory,
                Err(_) if process_has_exited(&process)? => continue,
                Err(error) => return Err(io_error(error)),
            };
            total = total
                .checked_add(u64::try_from(memory.WorkingSetSize).map_err(io_error)?)
                .ok_or_else(|| io::Error::other("sampled job working set overflow"))?;
        }
        Ok(total)
    }
}

impl Drop for NativeChild {
    fn drop(&mut self) {
        // Normal paths explicitly verify tree cleanup before releasing the
        // owner. This is the panic/error backstop; KillOnDrop additionally
        // sets JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE in the dependency.
        let _ = self.terminate();
    }
}

pub(super) fn open_process(pid: u32) -> io::Result<CloseHandleGuard<HPROCESS>> {
    HPROCESS::OpenProcess(
        co::PROCESS::QUERY_INFORMATION | co::PROCESS::VM_READ | co::PROCESS::SYNCHRONIZE,
        false,
        pid,
    )
    .map_err(io_error)
}

pub(super) fn process_has_exited(process: &HPROCESS) -> io::Result<bool> {
    match process.WaitForSingleObject(Some(0)).map_err(io_error)? {
        co::WAIT::OBJECT_0 => Ok(true),
        co::WAIT::TIMEOUT => Ok(false),
        _ => Err(io::Error::other("unexpected process-handle wait result")),
    }
}

fn filetime_duration(ticks: u64) -> Duration {
    // Split before multiplying: even u64::MAX FILETIME ticks fit Duration,
    // whereas multiplying the whole counter by 100 would overflow.
    Duration::new(
        ticks / 10_000_000,
        u32::try_from(ticks % 10_000_000).unwrap() * 100,
    )
}

pub(super) fn io_error(error: impl std::fmt::Display) -> io::Error {
    io::Error::other(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filetime_conversion_preserves_large_unsigned_values_and_submicroseconds() {
        assert_eq!(filetime_duration(1), Duration::from_nanos(100));
        assert_eq!(filetime_duration(10_000_001), Duration::new(1, 100));
        assert_eq!(
            filetime_duration(u64::MAX).as_nanos(),
            u128::from(u64::MAX) * 100
        );
    }

    #[test]
    fn unavailable_accounting_retains_exact_exit_and_tree_cleanup() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let mut command = tokio::process::Command::new("cmd.exe");
            command.args(["/D", "/C", "exit", "17"]);
            let mut owner = NativeChild::prepare(command).unwrap().spawn().unwrap();
            assert!(owner.resources().is_err());
            owner.accounting = Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "injected accounting access failure",
            ));
            let status = loop {
                if let Some(status) = owner.observe_exit().unwrap() {
                    break status;
                }
                tokio::time::sleep(EXIT_POLL).await;
            };
            assert_eq!(status.code(), Some(17));
            assert!(owner.resources().is_err());
            owner.terminate().unwrap();
            let deadline = std::time::Instant::now() + Duration::from_secs(5);
            while !owner.tree_is_empty().unwrap() {
                assert!(std::time::Instant::now() < deadline);
                tokio::time::sleep(EXIT_POLL).await;
            }
            assert_eq!(owner.observe_exit().unwrap().unwrap().code(), Some(17));
        });
    }

    #[test]
    fn unavailable_accounting_and_failed_job_observation_still_reap_exact_root() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let mut command = tokio::process::Command::new("cmd.exe");
            command.args(["/D", "/C", "ping -n 30 127.0.0.1 > nul"]);
            let mut owner = NativeChild::prepare(command).unwrap().spawn().unwrap();
            let independent = open_process(owner.child.id().unwrap()).unwrap();
            owner.accounting = Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "injected accounting access failure",
            ));
            owner.fault = Some(JobFault::ExitObservation);
            assert!(owner.observe_exit().is_err());
            owner.close_inspection_job();
            let deadline = std::time::Instant::now() + Duration::from_secs(2);
            while owner.observe_exit().unwrap().is_none() {
                assert!(std::time::Instant::now() < deadline);
                tokio::time::sleep(EXIT_POLL).await;
            }
            assert!(process_has_exited(&independent).unwrap());
            assert!(owner.status.is_some());
            let error = owner.resources().err().expect("missing counters must fail");
            assert!(
                error
                    .to_string()
                    .contains("injected accounting access failure")
            );
            drop(owner);
        });
    }

    #[test]
    fn exact_child_resources_match_independent_handle_counters_without_tolerance() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            for script in ["exit 17", "for /L %i in (1,1,20000) do @set tq_counter=%i"] {
                let mut command = tokio::process::Command::new("cmd.exe");
                command.args(["/D", "/C", script]);
                let mut owner = NativeChild::prepare(command).unwrap().spawn().unwrap();
                // A separately opened handle is pinned by the retained child,
                // before try_wait. Both queries concern this one execution,
                // not nominally equal workloads with distinct kernel counters.
                let independent = open_process(owner.child.id().unwrap()).unwrap();
                let deadline = std::time::Instant::now() + Duration::from_secs(5);
                while owner.observe_exit().unwrap().is_none() {
                    assert!(std::time::Instant::now() < deadline);
                    tokio::time::sleep(EXIT_POLL).await;
                }
                let collected = owner.resources().unwrap();
                let (_, _, kernel, user) = independent.GetProcessTimes().unwrap();
                let memory = independent.GetProcessMemoryInfo().unwrap();
                owner.terminate().unwrap();
                while !owner.tree_is_empty().unwrap() {
                    assert!(std::time::Instant::now() < deadline);
                    tokio::time::sleep(EXIT_POLL).await;
                }
                assert_eq!(collected.user.as_nanos(), u128::from(u64::from(user)) * 100);
                assert_eq!(
                    collected.system.as_nanos(),
                    u128::from(u64::from(kernel)) * 100
                );
                assert_eq!(
                    collected.peak_working_set,
                    u64::try_from(memory.PeakWorkingSetSize).unwrap()
                );
            }
        });
    }

    #[test]
    fn close_fallback_kills_descendants_and_preserves_exact_root_accounting() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            for fault in [
                JobFault::Query,
                JobFault::Terminate,
                JobFault::ExitObservation,
            ] {
                close_fallback_case(fault).await;
            }
        });
    }

    async fn close_fallback_case(fault: JobFault) {
        let directory = tempfile::tempdir().unwrap();
        let descendant_path = directory.path().join("descendant-pid");
        let escaped = descendant_path.display().to_string().replace('\'', "''");
        let mut command = tokio::process::Command::new("powershell.exe");
        command.args(["-NoProfile", "-Command", &format!(
            "$child = Start-Process powershell.exe -ArgumentList '-NoProfile','-Command','Start-Sleep -Seconds 30' -PassThru; Set-Content -LiteralPath '{escaped}' -Value $child.Id; Start-Sleep -Seconds 30"
        )]);
        let mut owner = NativeChild::prepare(command).unwrap().spawn().unwrap();
        owner.fault = Some(fault);
        let root = open_process(owner.child.id().unwrap()).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let descendant = loop {
            if let Ok(pid) = std::fs::read_to_string(&descendant_path)
                && let Ok(pid) = pid.trim().trim_start_matches('\u{feff}').parse()
            {
                break open_process(pid).unwrap();
            }
            assert!(std::time::Instant::now() < deadline);
            tokio::time::sleep(EXIT_POLL).await;
        };
        match fault {
            JobFault::Query => assert!(owner.tree_is_empty().is_err()),
            JobFault::Terminate => assert!(owner.terminate().is_err()),
            JobFault::ExitObservation | JobFault::ExitObservationWithoutAccounting => {
                assert!(owner.observe_exit().is_err());
            }
        }
        assert!(!process_has_exited(&root).unwrap());
        assert!(!process_has_exited(&descendant).unwrap());
        owner.close_inspection_job();
        assert!(owner.job.is_none());
        while owner.observe_exit().unwrap().is_none() {
            assert!(std::time::Instant::now() < deadline);
            tokio::time::sleep(EXIT_POLL).await;
        }
        // Collection occurs while the remaining child/job guard is still
        // retained, and remains exactly comparable with an independent handle.
        let resources = owner.resources().unwrap();
        let (_, _, kernel, user) = root.GetProcessTimes().unwrap();
        assert_eq!(resources.user.as_nanos(), u128::from(u64::from(user)) * 100);
        assert_eq!(
            resources.system.as_nanos(),
            u128::from(u64::from(kernel)) * 100
        );
        assert_eq!(
            resources.peak_working_set,
            u64::try_from(root.GetProcessMemoryInfo().unwrap().PeakWorkingSetSize).unwrap()
        );
        drop(owner);
        while !process_has_exited(&descendant).unwrap() {
            assert!(std::time::Instant::now() < deadline);
            tokio::time::sleep(EXIT_POLL).await;
        }
        assert!(process_has_exited(&root).unwrap());
    }

    #[test]
    fn invalid_process_identity_is_an_error_not_zero_resources() {
        assert!(open_process(0).is_err());
    }
}
