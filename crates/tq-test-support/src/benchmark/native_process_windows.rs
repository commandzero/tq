//! Retained-handle Windows accounting and suspended Job Object containment.

use std::{io, process::ExitStatus, sync::Arc, time::Duration};

use process_wrap::tokio::{ChildWrapper, CommandWrap, CommandWrapper, JobObject, KillOnDrop};
use win32job::{ExtendedLimitInfo, Job};
use winsafe::{HPROCESS, co, guard::CloseHandleGuard};

pub(super) const EXIT_POLL: Duration = Duration::from_micros(100);
pub(super) const RSS_SAMPLE_INTERVAL: Duration = Duration::from_millis(25);

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
        Ok(NativeChild {
            child,
            accounting,
            job: self.job,
            status: None,
        })
    }
}

pub(super) struct NativeChild {
    child: Box<dyn ChildWrapper>,
    accounting: io::Result<CloseHandleGuard<HPROCESS>>,
    job: Arc<Job>,
    status: Option<ExitStatus>,
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
            self.status = self.child.try_wait()?;
        }
        Ok(self.status)
    }

    pub(super) fn terminate(&mut self) -> io::Result<()> {
        // This is a retained job handle, never a recycled PID. The wrapper's
        // start_kill is nonblocking and terminates the entire nested job tree.
        self.child.start_kill()
    }

    pub(super) fn tree_is_empty(&self) -> io::Result<bool> {
        Ok(self
            .job
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
        let mut total = 0_u64;
        for pid in self.job.query_process_id_list().map_err(io_error)? {
            let pid32 = u32::try_from(pid).map_err(io_error)?;
            let process = match open_process(pid32) {
                Ok(process) => process,
                Err(error) => {
                    if self
                        .job
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
            if !self
                .job
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
        let _ = self.child.start_kill();
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
    fn invalid_process_identity_is_an_error_not_zero_resources() {
        assert!(open_process(0).is_err());
    }
}
