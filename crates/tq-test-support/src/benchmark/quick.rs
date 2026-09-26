//! A whole-stage wall-clock guard for quick benchmark runs.
//!
//! Effective options are parsed before supervision without discovery or
//! filesystem work. The guard owns termination and bounded cleanup while
//! sharing one absolute deadline with nested workers.

use std::{
    env, io,
    path::{Path, PathBuf},
    process::{Child, Command},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Sender},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::collections::{HashMap, HashSet};

#[cfg(unix)]
use nix::{
    errno::Errno,
    sys::signal::{Signal, killpg},
    unistd::Pid,
};

/// Default work phase in seconds; cleanup has a separate bounded allowance.
pub const QUICK_WORK_BUDGET_SECONDS: u64 = 50;
const DEFAULT_CLEANUP: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(20);
const TERM_GRACE: Duration = Duration::from_millis(150);
const FORCE_MARGIN: Duration = Duration::from_millis(400);
const DEADLINE_ENV: &str = "TQ_QUICK_WORK_DEADLINE_UNIX_MS";
const SUPERVISED_ENV: &str = "TQ_QUICK_SUPERVISED";
const COMPLETION_ENV: &str = "TQ_QUICK_COMPLETION_STAGE";

fn epoch_millis() -> io::Result<u128> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|time| time.as_millis())
        .map_err(|error| io::Error::other(format!("system clock is before Unix epoch: {error}")))
}

fn inherited_deadline() -> Option<u128> {
    env::var(DEADLINE_ENV).ok()?.parse::<u128>().ok()
}

/// Time left in the inherited, stage-wide work budget, if guarded.
/// A zero duration means cleanup must already be under way.
#[must_use]
pub fn remaining_work_budget() -> Option<Duration> {
    let deadline = inherited_deadline()?;
    let now = epoch_millis().ok()?;
    let millis = u64::try_from(deadline.saturating_sub(now)).unwrap_or(u64::MAX);
    Some(Duration::from_millis(millis))
}

/// The coordinator writes a completed report only to the supervisor-owned
/// staging path. A failed or interrupted run leaves its durable checkpoint
/// incomplete; only the owning guard can accept completion.
pub fn completion_stage() -> Option<PathBuf> {
    env::var_os(COMPLETION_ENV).map(PathBuf::from)
}

/// Called after side-effect-free option parsing, before any discovery or I/O.
/// The parsed effective policy, not an independent argument scan, determines
/// whether this process must own a quick coordinator.
///
/// # Errors
///
/// Returns an error if the coordinator cannot be launched or supervised.
pub fn supervise_current_if_quick(
    quick: bool,
    budget: Duration,
    report: Option<&Path>,
) -> io::Result<Option<i32>> {
    if env::var_os(SUPERVISED_ENV).is_some() || !quick {
        return Ok(None);
    }
    let mut command = Command::new(env::current_exe()?);
    command.args(env::args_os().skip(1));
    if let Some(report) = report {
        command.env("TQ_QUICK_DEFAULT_OUTPUT", report);
    }
    supervise_command_with_report(&mut command, budget, report).map(Some)
}

/// Guard a post-compilation command, inheriting its standard streams.
/// Nested guards share the earlier absolute deadline instead of resetting it.
///
/// # Errors
///
/// Returns an error if the command cannot be launched or its lifecycle cannot be observed.
pub fn supervise_command(command: &mut Command, budget: Duration) -> io::Result<i32> {
    let report = env::var_os("TQ_QUICK_REPORT_PATH").map(PathBuf::from);
    supervise_command_with_report(command, budget, report.as_deref())
}

/// Supervise a coordinator and accept its completed report only after it has
/// exited and all reporting has finished within the work deadline.
///
/// # Errors
///
/// Returns an error on spawn, wait, or report-publication failure.
pub fn supervise_command_with_report(
    command: &mut Command,
    budget: Duration,
    report: Option<&Path>,
) -> io::Result<i32> {
    if !cfg!(any(target_os = "macos", target_os = "linux")) {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "quick process supervisor supports macOS and Linux",
        ));
    }
    let started = Instant::now();
    let inherited = remaining_work_budget();
    let work = inherited
        .map_or(budget, |remaining| remaining.min(budget))
        .min(Duration::from_secs(QUICK_WORK_BUDGET_SECONDS));
    let work_deadline = started + work;
    let cleanup = DEFAULT_CLEANUP.min(if work < Duration::from_secs(5) {
        Duration::from_secs(1)
    } else {
        DEFAULT_CLEANUP
    });
    let hard_deadline = work_deadline + cleanup;
    let absolute = epoch_millis()?
        .checked_add(work.as_millis())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "quick deadline overflow"))?;
    // Preserve the precise inherited epoch, not a fresh millisecond-rounded
    // approximation that could move a nested worker past its parent's budget.
    let absolute = inherited_deadline().map_or(absolute, |parent| parent.min(absolute));
    command.env(DEADLINE_ENV, absolute.to_string());
    command.env(SUPERVISED_ENV, "1");
    let stage = report.map(|path| {
        let mut name = path.as_os_str().to_os_string();
        name.push(format!(".quick-{}-{absolute}.complete", std::process::id()));
        PathBuf::from(name)
    });
    if let Some(path) = &stage {
        command.env(COMPLETION_ENV, path);
    }
    let interrupt = Arc::new(AtomicBool::new(false));
    let terminate = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&interrupt))?;
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&terminate))?;
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        command.process_group(0);
    }
    // Reserve the eventual exact-child reaper before launch. Thread creation
    // failure cannot leave a spawned child without a bounded cleanup owner.
    let reaper = reserve_reaper()?;
    let child = command.spawn()?;
    let result = supervise_child(
        child,
        work_deadline,
        hard_deadline,
        absolute,
        &interrupt,
        &terminate,
        &reaper,
    );
    let accepted = !interrupt.load(Ordering::Relaxed)
        && !terminate.load(Ordering::Relaxed)
        && Instant::now() < work_deadline;
    if let Some((stage, report)) = stage.as_ref().zip(report) {
        if accepted && result.as_ref().is_ok_and(|code| matches!(*code, 0..=2)) && stage.exists() {
            std::fs::rename(stage, report)?;
        } else {
            let _ = std::fs::remove_file(stage);
        }
    }
    if !accepted
        && result
            .as_ref()
            .is_ok_and(|code| *code != 124 && *code != 130 && *code != 143)
    {
        return Ok(if interrupt.load(Ordering::Relaxed) {
            130
        } else if terminate.load(Ordering::Relaxed) {
            143
        } else {
            124
        });
    }
    result
}

fn supervise_child(
    mut child: Child,
    work_deadline: Instant,
    hard_deadline: Instant,
    absolute: u128,
    interrupt: &AtomicBool,
    terminate: &AtomicBool,
    reaper: &Sender<Child>,
) -> io::Result<i32> {
    let root = child.id();
    let result = supervise_child_until_deadline(
        &mut child,
        work_deadline,
        hard_deadline,
        absolute,
        interrupt,
        terminate,
    );
    if result.is_err() && child_exited(root).is_ok() {
        // Signal only while the unreaped exact child still anchors its PGID.
        #[cfg(unix)]
        signal_root_group(root, Signal::SIGKILL);
        let _ = child.kill();
    }
    match child.try_wait() {
        Ok(Some(_)) => result,
        Ok(None) => {
            let _ = child.kill();
            defer_reap(child, reaper)?;
            result.map(|_| 124)
        }
        Err(error) => {
            #[cfg(unix)]
            let lost_owner = error.raw_os_error() == Some(nix::libc::ECHILD);
            #[cfg(not(unix))]
            let lost_owner = false;
            if !lost_owner {
                let _ = child.kill();
                defer_reap(child, reaper)?;
            }
            Err(error)
        }
    }
}

fn supervise_child_until_deadline(
    child: &mut Child,
    work_deadline: Instant,
    hard_deadline: Instant,
    absolute: u128,
    interrupt: &AtomicBool,
    terminate: &AtomicBool,
) -> io::Result<i32> {
    let root = child.id();
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    let mut known = HashMap::new();
    let interruption = loop {
        let now = Instant::now();
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        if let Ok(snapshot) = process_snapshot() {
            track_descendants(root, &snapshot, &mut known);
        }
        let signal = interrupt.load(Ordering::Relaxed) || terminate.load(Ordering::Relaxed);
        let expired = Instant::now() >= work_deadline || epoch_millis()? >= absolute;
        if child_exited(root)? || expired || signal {
            break (expired, signal);
        }
        std::thread::sleep(POLL.min(work_deadline.saturating_duration_since(now)));
    };
    let timed_out = interruption.0;
    let interrupted = interruption.1;

    // Notify the coordinator first. It owns checkpoint/report writes and can
    // cancel its own workers; leave the rest of the cleanup allowance for that
    // work instead of killing it after an arbitrary short signal grace.
    #[cfg(unix)]
    if timed_out || interrupted {
        signal_root_group(
            root,
            if !timed_out && interrupt.load(Ordering::Relaxed) {
                Signal::SIGINT
            } else {
                Signal::SIGTERM
            },
        );
    }
    let force_at = if interrupted && !timed_out {
        (Instant::now() + DEFAULT_CLEANUP).min(hard_deadline)
    } else {
        hard_deadline
    }
    .checked_sub(FORCE_MARGIN)
    .unwrap_or(work_deadline);
    while (timed_out || interrupted) && Instant::now() < force_at {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        if let Ok(snapshot) = process_snapshot() {
            track_descendants(root, &snapshot, &mut known);
        }
        if child_exited(root)? {
            break;
        }
        std::thread::sleep(POLL.min(force_at.saturating_duration_since(Instant::now())));
    }

    // Clean up *before* reaping the direct child: while it is still our child,
    // its PID and process-group identifier cannot be recycled. This also
    // handles workers which started separate process groups.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    signal_owned_groups(root, &known, Signal::SIGTERM);
    let grace = (Instant::now() + TERM_GRACE).min(hard_deadline);
    while Instant::now() < grace {
        std::thread::sleep(POLL.min(grace.saturating_duration_since(Instant::now())));
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    signal_owned_groups(root, &known, Signal::SIGKILL);
    // Force the exact child as well as the observed groups. An exited
    // coordinator retains its PID until we reap it below.
    if !child_exited(root)? {
        let _ = child.kill();
    }
    loop {
        if child_exited(root)? {
            // A later group signal must never follow this reap.
            let status = child.wait()?;
            if timed_out {
                return Ok(124);
            }
            if interrupted {
                return Ok(if interrupt.load(Ordering::Relaxed) {
                    130
                } else {
                    143
                });
            }
            return Ok(status.code().unwrap_or_else(|| {
                #[cfg(unix)]
                {
                    use std::os::unix::process::ExitStatusExt as _;
                    status.signal().map_or(1, |signal| 128 + signal)
                }
                #[cfg(not(unix))]
                {
                    1
                }
            }));
        }
        if Instant::now() >= hard_deadline {
            // The outer owner still holds the Child and handles deferred reap
            // if the exact child remains alive despite direct SIGKILL.
            let _ = child.kill();
            return Ok(124);
        }
        std::thread::sleep(POLL.min(hard_deadline.saturating_duration_since(Instant::now())));
    }
}

fn reserve_reaper() -> io::Result<Sender<Child>> {
    let (sender, receiver) = mpsc::channel::<Child>();
    std::thread::Builder::new()
        .name("quick-child-reaper".to_owned())
        .spawn(move || {
            if let Ok(mut child) = receiver.recv() {
                let _ = child.kill();
                // Exceptional waits stay off the bounded caller. Keep the
                // exact owner alive until reaped or another waiter owns it.
                loop {
                    match child.wait() {
                        Ok(_) => break,
                        #[cfg(unix)]
                        Err(error) if error.raw_os_error() == Some(nix::libc::ECHILD) => break,
                        Err(_) => {
                            let _ = child.kill();
                            std::thread::sleep(POLL);
                        }
                    }
                }
            }
        })?;
    Ok(sender)
}

fn defer_reap(child: Child, reaper: &Sender<Child>) -> io::Result<()> {
    reaper.send(child).map_err(|error| {
        let mut child = error.0;
        let _ = child.kill();
        io::Error::other("quick child reaper exited before taking ownership")
    })
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[derive(Clone, Copy)]
struct ProcessInfo {
    parent: u32,
    group: u32,
    // A process identity, not just a PID: stale entries must not become new
    // owners when the operating system recycles a descendant PID.
    birth: u64,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn track_descendants(
    root: u32,
    snapshot: &HashMap<u32, ProcessInfo>,
    known: &mut HashMap<u32, ProcessInfo>,
) {
    known.retain(|pid, info| {
        snapshot
            .get(pid)
            .is_some_and(|current| current.birth == info.birth)
    });
    let mut attached = HashSet::from([root]);
    loop {
        let before = attached.len();
        for (&pid, info) in snapshot {
            if attached.contains(&info.parent)
                || known.get(&info.parent).is_some_and(|parent| {
                    snapshot
                        .get(&info.parent)
                        .is_some_and(|current| current.birth == parent.birth)
                })
            {
                attached.insert(pid);
            }
        }
        if attached.len() == before {
            break;
        }
    }
    for pid in attached {
        if pid != root
            && let Some(info) = snapshot.get(&pid)
        {
            known.insert(pid, *info);
        }
    }
}

#[cfg(unix)]
fn signal_root_group(root: u32, signal: Signal) {
    if let Ok(group) = i32::try_from(root) {
        signal_group(group, signal);
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn signal_owned_groups(root: u32, known: &HashMap<u32, ProcessInfo>, signal: Signal) {
    let mut groups = HashSet::from([root]);
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    if let Ok(snapshot) = process_snapshot() {
        // Recheck an identity and group against the current process table; a
        // stale PID alone is never authorization to signal a process group.
        for (pid, info) in known {
            if snapshot
                .get(pid)
                .is_some_and(|current| current.birth == info.birth && current.group == info.group)
            {
                groups.insert(info.group);
            }
        }
    }
    // A process group is signalled only while its observed member exists.
    // Root's group is anchored by our unreaped direct child.
    let own_group = nix::unistd::getpgrp().as_raw();
    for group in groups {
        if let Some(group) = i32::try_from(group)
            .ok()
            .filter(|group| *group != own_group)
        {
            signal_group(group, signal);
        }
    }
}

#[cfg(unix)]
fn signal_group(group: i32, signal: Signal) {
    match killpg(Pid::from_raw(group), signal) {
        Ok(()) | Err(Errno::ESRCH) => {}
        // XNU reports EPERM for a group containing only an unreaped zombie
        // leader. A failed live-member query is *not* treated as success.
        #[cfg(target_os = "macos")]
        Err(Errno::EPERM)
            if process_snapshot().is_ok_and(|snapshot| {
                !snapshot
                    .values()
                    .any(|info| info.group == group.cast_unsigned())
            }) => {}
        // Diagnostics must not block the owner when stderr is a full pipe.
        Err(_) => {}
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn child_exited(pid: u32) -> io::Result<bool> {
    use rustix::process::{WaitId, WaitIdOptions, waitid};
    let pid = i32::try_from(pid).map_err(|_| io::Error::other("child PID overflow"))?;
    let pid =
        rustix::process::Pid::from_raw(pid).ok_or_else(|| io::Error::other("invalid child PID"))?;
    loop {
        match waitid(
            WaitId::Pid(pid),
            WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
        ) {
            Ok(status) => return Ok(status.is_some()),
            Err(rustix::io::Errno::INTR) => {}
            Err(error) => return Err(error.into()),
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn child_exited(_pid: u32) -> io::Result<bool> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "quick process supervisor supports macOS and Linux",
    ))
}

#[cfg(target_os = "macos")]
fn process_snapshot() -> io::Result<HashMap<u32, ProcessInfo>> {
    use libproc::{
        bsd_info::BSDInfo,
        proc_pid::pidinfo,
        processes::{ProcFilter, pids_by_type},
    };
    let mut snapshot = HashMap::new();
    for pid in pids_by_type(ProcFilter::All)? {
        let Ok(raw) = i32::try_from(pid) else {
            continue;
        };
        let Ok(info) = pidinfo::<BSDInfo>(raw, 0) else {
            continue;
        };
        if info.pbi_pid == pid && info.pbi_status != nix::libc::SZOMB {
            snapshot.insert(
                pid,
                ProcessInfo {
                    parent: info.pbi_ppid,
                    group: info.pbi_pgid,
                    birth: (info.pbi_start_tvsec << 20) | info.pbi_start_tvusec,
                },
            );
        }
    }
    Ok(snapshot)
}

#[cfg(target_os = "linux")]
fn process_snapshot() -> io::Result<HashMap<u32, ProcessInfo>> {
    let mut snapshot = HashMap::new();
    for entry in std::fs::read_dir("/proc")? {
        let entry = entry?;
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|pid| pid.parse::<u32>().ok())
        else {
            continue;
        };
        let Ok(stat) = std::fs::read_to_string(entry.path().join("stat")) else {
            continue;
        };
        let Some((_, fields)) = stat.rsplit_once(") ") else {
            continue;
        };
        let mut fields = fields.split_whitespace();
        let state = fields.next();
        let parent = fields.next().and_then(|field| field.parse::<u32>().ok());
        let group = fields.next().and_then(|field| field.parse::<u32>().ok());
        let birth = fields.nth(16).and_then(|field| field.parse::<u64>().ok());
        if let (Some(parent), Some(group), Some(birth)) = (parent, group, birth)
            && state != Some("Z")
        {
            snapshot.insert(
                pid,
                ProcessInfo {
                    parent,
                    group,
                    birth,
                },
            );
        }
    }
    Ok(snapshot)
}
