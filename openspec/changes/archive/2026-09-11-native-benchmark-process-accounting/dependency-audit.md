# Native child-accounting dependency audit

## Approved decision

The user approved raising MSRV to Rust 1.95 and selecting `wait4 = 0.2.0`
with `rustix = 1.1.4`. This supersedes the earlier Rust 1.88 and wait4 0.1.3
selection. The new call consumes its Child and retries EINTR internally.
RSS conversion clamps negative counters and saturates multiplication; CPU
conversion remains unchecked. The approved valid-OS-counter assumption and
checked first-party validation remain necessary. The upgrade does not resolve
Linux pre-exec RSS inheritance. Fresh calibration is required for the changed
collector before publication.

The remaining audit records the original selection context and version-specific
source inspection. Rust 1.88 restrictions and wait4 0.1.3 smoke runs below are
historical evidence, not the current dependency decision or validation of 0.2.0.

Status: dependency spike and source audit complete. The two-phase smoke proof passed on this macOS host and on Ironhide Linux. The macOS-only `libproc 0.14.11` adjunct is selected for verified process-group cleanup; workload-specific descendant and unit controls are still required.

## Scope and baseline

The audited repository baseline is proposal revision `6e357f7`. The current
workspace advertises Rust `1.88` and currently has these relevant entries:

| crate | current direct/transitive status | exact version/source evidence |
| --- | --- | --- |
| `nix` | direct in `tq-test-support`, features `process`, `signal` | `crates/tq-test-support/Cargo.toml`; lockfile `0.29.0` |
| `wait-timeout` | direct in `tq-test-support` | `crates/tq-test-support/Cargo.toml`; lockfile `0.2.1` |
| `rustix` | direct in `tq-test-support`, feature `process` | lockfile `1.1.4` |
| `wait4` | direct in `tq-test-support`, pinned for MSRV | lockfile `0.1.3` |
| `libproc` | macOS-targeted direct dependency in `tq-test-support` | lockfile `0.14.11`; local registry source and docs audited |

The exact local source paths below are under
`/Users/reno/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`.
The lockfile currently resolves `libc 0.2.189`, `bitflags 2.13.1`,
`cfg-if 1.0.4`, `cfg_aliases 0.2.2`, `errno 0.3.14`, and
`linux-raw-sys 0.12.1`.

## Decision summary

`wait4 0.2.0` is the strongest single safe call-site API for this task. Its
public `Wait4::try_wait4` performs an exact-PID, nonblocking **resource-aware
reap** on Unix, while `Wait4::wait4` consumes the `Child` by value. Its Unix
implementation retries `EINTR`, converts status and `rusage`, and explicitly
normalizes Linux KiB versus Apple bytes. However, `wait4 0.2.0` declares
`rust-version = 1.95`, while this repository promises Rust 1.88. Adding it
without a separate MSRV decision would violate the workspace contract. The
release was published 2026-08-22 after the 2021 `0.1.x` line, so its current
API is new but its maintenance history is short.

If the advertised MSRV must remain 1.88, the only audited composition that
meets the lifecycle shape is a single owner using safe `rustix 1.1.4`
`waitid(Pid, EXITED | NOWAIT | NOHANG)` for nonblocking observation, followed
by `wait4 0.1.3`'s blocking resource-aware reap (or an equivalent version
selected after a lockfile/MSRV decision). `WNOWAIT` is important: it leaves
the leader waitable while the owner terminates the private process group, so a
descendant cannot be left behind and the resource-bearing reap remains the
only consuming wait. This composition needs a direct `rustix` dependency with
the `process` feature; the existing transitive `tempfile` edge enables only
`fs`. There is a separate strictness caveat: `wait4 0.1.3` performs unchecked
RSS multiplication and signed-to-unsigned time conversions internally. If the
spec requires the dependency itself to prove checked conversion, this version
is blocked even though its valid-kernel-value path works on both tested hosts.

No audited API turns arbitrary process-tree accounting into a pure
process-only metric. Both OS `wait4` manuals describe the returned usage as a
summary for the terminated process and its waited-for children. jq/yq/tq must
be checked for child-spawning behavior, and any descendant topology must be
reported or excluded rather than relabeled as process-only RSS.

## Candidate audit

### `wait4 0.2.0`

Local manifest: `wait4-0.2.0/Cargo.toml`.

* **Lifecycle/API.** `Wait4::wait4(self)` consumes the `Child`, closes stdin,
  and calls native `wait4(pid, 0)`. `Wait4::try_wait4(&mut self)` calls native
  `wait4(pid, WNOHANG)` and returns `Ok(None)` while running. The API docs
  explicitly say `Ok(Some(_))` has reaped the child; no later `wait`,
  `try_wait4`, or signal may be sent through that `Child` because the PID may
  already have been recycled. `try_wait4_pid(u32)` is also exact-PID and
  rejects PID zero, avoiding `wait4(0)`'s process-group meaning.
* **Source evidence.** See `src/lib.rs:47-79` for ownership guarantees and
  `src/unix.rs:61-115` for the native implementation. `src/unix.rs:61-78`
  retries `EINTR`; `:73-77` distinguishes WNOHANG's zero return from a
  resource-bearing reap. `ExitStatus::from_raw` is used at `:46-58`.
* **Targets.** The native wait4 branch is compiled for Linux, Android, macOS,
  iOS/tvOS/watchOS, and the BSDs (`src/unix.rs:31-43`). Other Unix targets
  fall back to `getrusage(RUSAGE_CHILDREN)`, which the crate itself documents
  as process-wide and unsuitable for concurrent per-child accounting
  (`src/unix.rs:118-167`). The change must gate authoritative rows to the
  native Linux/macOS branch and fail unsupported targets clearly.
* **Units and CPU.** `maxrss_bytes` treats Apple kernels as bytes and Linux
  and BSD kernels as KiB times 1024 (`src/unix.rs:14-29`). `ru_utime` and
  `ru_stime` are converted from `timeval` to microsecond `Duration`
  (`src/unix.rs:8-12`). The multiplication is saturating and negative RSS is
  clamped to zero, so the caller must still reject zero/invalid/overflow
  values for authoritative preflight. The `timeval` arithmetic uses signed
  `i64` before casting to `u64`; OS values should be nonnegative, but an
  additional checked validation at the report boundary is prudent.
* **Scope.** The crate documents native wait4 rusage as specific to the
  waited child but including descendants the child itself waited for
  (`src/lib.rs:34-43`). Linux `getrusage(2)` says `RUSAGE_CHILDREN` is not a
  process-tree peak and that `ru_maxrss` is the largest child; Apple's wait4
  manual says "terminated process and all its children." This is not a
  simultaneous process-tree peak and can be accepted only with an explicit
  process/descendant scope in provenance.
* **Timeout integration.** There is no timeout argument. A controller may
  poll `try_wait4`, but it must be the same owner that decides deadline,
  termination, and reap. It must never use `Child::try_wait`,
  `wait-timeout`, or `Child::wait` as a shortcut. For a process-group
  workload, a direct `try_wait4` can reap a naturally exiting leader before
  descendants are killed; use the `rustix waitid(... WNOWAIT ...)` phase below
  or prove that the measured executable never leaves descendants.
* **License/transitives.** MIT. On Unix, the normal dependency is only
  `libc ^0.2`; Windows uses `windows-sys ^0.61` and is outside this change.
  The crate has internal unsafe libc calls, as do all audited syscall crates,
  but a call site can remain within the repository's first-party unsafe/FFI
  prohibition. No first-party wrapper or local libc call is needed.
* **Maintenance/MSRV.** `0.2.0` is latest and was published 2026-08-22;
  `0.1.3` was published 2021-09-02. The new version's MSRV is 1.95 because
  it uses `std::cfg_select!`; this is the material adoption blocker for the
  current workspace's declared 1.88.

Minimal call-site shape (the result branch owns the one consuming wait):

```rust
use std::process::Child;
use wait4::Wait4;

fn poll_once(child: &mut Child) -> std::io::Result<Option<wait4::ResUse>> {
    // None: still running; Some: status and rusage are already collected and
    // the Child must never be waited/signalled again.
    child.try_wait4()
}
```

### `wait4 0.1.3`

Local source: `wait4-0.1.3/src/lib.rs` and `src/unix.rs`.

This is the old API in the same project. `Wait4::wait4(&mut Child)` is
blocking-only (`src/unix.rs:18-49`) and does not expose WNOHANG, timeout, or a
raw-PID nonblocking operation. It calls native wait4 once and does not retry
`EINTR` (`:26-30`). Its direct branch supports macOS, Linux, and FreeBSD;
other targets use `Child::wait` and process-wide `RUSAGE_CHILDREN` (`:50-100`),
which is expressly unsuitable for independent/concurrent samples. It
normalizes macOS as bytes and Linux as KiB times 1024, but uses unchecked
arithmetic and casts (`:33-45`). The manifest declares no MSRV. MIT license,
with `cfg-if` and `libc` on Unix. It can be used as the final consuming reap
after a safe rustix WNOWAIT observation, but it is not sufficient by itself
for bounded nonblocking lifecycle control. The owner must wrap the blocking
call in an `Interrupted` retry loop.

The manifest has no `rust-version` field. The scratch proof declares
`rust-version = 1.88` and runs with the installed Rust 1.88.0 toolchain on
macOS. `rustix 1.1.4` declares MSRV 1.63. This is evidence that the audited
composition builds at the workspace MSRV, not an upstream MSRV guarantee for
wait4 0.1.3.

The unchecked conversions are a real adoption limit. On Linux the source
computes `rusage.ru_maxrss * 1024` in the signed libc field type and then casts
the result to `u64`. `timeval_to_duration` multiplies signed seconds and adds
microseconds before casting to `u64` for `Duration::from_micros`. A negative or
overflowed intermediate therefore becomes a large or wrapped value in a
release build. The API does not return the raw field or an overflow indicator,
so a caller cannot establish strict checked conversion by inspecting only the
returned `ResourceUsage`. Actual kernel values are expected to be
nonnegative and far below these limits, and both host runs followed that path,
but that is an OS-value assumption, not a guarantee of this crate's API.

If checked conversion is a hard requirement, treat `wait4 0.1.3` as blocked
unless the design records and accepts those OS bounds. Raising the workspace
MSRV to use `wait4 0.2.0` removes the unchecked RSS multiplication through
saturating conversion and clamps negative RSS, but 0.2.0 still casts its
signed `timeval` arithmetic and does not report saturation. Neither release
alone provides a checked-conversion error boundary.

### `rustix 1.1.4` (`process` feature)

Local manifest and source: `rustix-1.1.4/Cargo.toml`,
`src/process/wait.rs`, `src/backend/libc/process/syscalls.rs`, and
`src/process/kill.rs`.

* **Safe observation API.** `rustix::process::waitid` takes
  `WaitId::Pid(Pid)` for a specific PID and returns `io::Result<Option<WaitIdStatus>>`
  (`src/process/wait.rs:379-404,478-493`). `WaitIdOptions` exposes `EXITED`,
  `NOHANG`, and `NOWAIT` (`:50-74`). `WaitIdStatus` exposes `exited`, `killed`,
  `dumped`, `exit_status`, and `terminating_signal` (`:206-240`), which is
  enough to classify the result without raw status decoding. It does not
  expose rusage, so it cannot replace wait4 as the consuming collector.
* **macOS/Linux support.** The public waitid cfg excludes Cygwin, Horizon,
  OpenBSD, Redox, and WASI, not macOS (`:478-486`). The libc backend exports
  `WNOWAIT`/`WEXITED` on both relevant targets (`src/backend/libc/process/wait.rs:10-17`).
  The pinned libc source has Apple `WEXITED`, `WNOWAIT`, `P_PID`, and
  `waitid` declarations (`libc-0.2.189/src/unix/bsd/apple/mod.rs:3670-3677,
  4779-4788`), and Linux has the corresponding constants and declaration.
  The Linux raw backend also implements P_PID waitid, so Linux behavior must
  be tested with the selected backend and kernel.
* **No-reap semantics.** `NOWAIT` is passed through to the OS. With
  `EXITED|NOWAIT|NOHANG`, `Ok(None)` means not yet waitable and `Ok(Some(s))`
  observes exit without consuming the child's wait state. The same owner can
  then signal the private PGID and invoke wait4 exactly once. Retry only
  `ErrorKind::Interrupted`; an ordinary `waitid` error is infrastructure
  failure. Do not call `Child::try_wait` after the observation.
* **Signal/cleanup API.** `rustix::process::kill_process_group(Pid, Signal::KILL)`
  is a safe wrapper (`src/process/kill.rs:20-37`). With `CommandExt::process_group(0)`,
  use the spawned child's PID as the private PGID. Signal while the leader is
  still unreaped; then consume it with wait4. Treat `ESRCH`/`NotFound` as "the
  group has already disappeared" only after verifying that the leader's PID
  remains the exact owned child. Never kill a PGID after a resource-aware reap
  has returned, because the numeric identity may be reused.
* **License/transitives/MSRV.** Apache-2.0 WITH LLVM-exception OR Apache-2.0
  OR MIT; MSRV 1.63. In this lockfile it resolves to `bitflags 2.13.1`,
  `errno 0.3.14`, `libc 0.2.189`, `linux-raw-sys 0.12.1`, and
  `windows-sys 0.61.2` (target/feature dependent). `rustix` is already in the
  lock through `tempfile 3.27.0`, but only its `fs` feature is unified today;
  a direct dependency should request `process` explicitly. Its own source
  uses internal unsafe syscall/FFI blocks, but the public call site is safe.
  The project is actively maintained relative to the old wait4 crate; the
  pinned 1.1.4 release is dated 2026-02-22 in the local sparse index.

Recommended two-phase shape when process-group cleanup is in scope:

```rust
use std::io;
use rustix::process::{waitid, Pid, WaitId, WaitIdOptions};

fn observe_exit(pid: Pid) -> io::Result<bool> {
    let options = WaitIdOptions::EXITED | WaitIdOptions::NOWAIT | WaitIdOptions::NOHANG;
    loop {
        match waitid(WaitId::Pid(pid), options) {
            Ok(Some(status)) => return Ok(status.exited() || status.killed() || status.dumped()),
            Ok(None) => return Ok(false),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.into()),
        }
    }
}
```

The `waitid` status is only an observation. After it returns true, the owner
must perform process-group termination (if descendants/forced cleanup are in
scope), freeze the monotonic timestamp at that observation/cleanup decision as
specified by the implementation, and then use `wait4`/`try_wait4` once for the
resource-bearing reap. If a deadline fires while `observe_exit` is false,
terminate first, keep polling WNOWAIT until the leader is waitable, and then
reap. A separate ordinary wait is never allowed.

### `nix 0.29.0` (`process`, `signal`, optional `resource`)

Local source: `nix-0.29.0/src/sys/wait.rs` and `src/sys/resource.rs`.

`nix::sys::wait::waitpid(Pid, WNOHANG)` is safe, exact-PID, and converts status
to `WaitStatus` (`src/sys/wait.rs:291-314`), but it has no rusage output. Nix's
`waitid` wrapper is cfg-gated to Android, FreeBSD, Haiku, and Linux and is not
available for macOS in this release (`:360-380`); it therefore cannot be the
shared macOS/Linux observation interface. `nix::sys::resource::getrusage`
returns current process, process-wide waited children, or (on some targets)
current-thread usage (`src/sys/resource.rs:250-398`). Its `Usage::max_rss` is
documented as bytes on Apple targets and kilobytes elsewhere (`:300-322`),
but `RUSAGE_CHILDREN` is cumulative/process-wide, not target-child scoped.
Combining nix waitpid with getrusage would reintroduce exactly the accounting
mixing and reaping race this change rejects. Nix does provide safe signals,
but that does not repair the missing resource-aware reap.

The pinned crate is MSRV 1.69, MIT licensed, and has normal dependencies
`bitflags`, `cfg-if`, `cfg_aliases` (build), and `libc`; its docs identify
macOS and Linux as supported active targets. It remains useful for signal
operations already used by the current code, but it is not a candidate
collector. Newer Nix releases were not selected because the current 0.29.0
lock entry is the exact crate being audited and newer releases still expose
waitpid/waitid/getrusage as separate APIs rather than a wait4 rusage result.

### `wait-timeout 0.2.1`

Local source: `wait-timeout-0.2.1/src/unix.rs`.

This crate is a timeout helper, not a resource collector. Its Unix
documentation says it uses `waitpid(WNOHANG)` for probes and `waitpid` to
actually reap (`src/unix.rs:9-12`). More importantly, its implementation
calls `child.try_wait()` before registration (`:84-97`) and from its SIGCHLD
processing path (`:162-182`). That can consume the exact child status before a
later wait4 collector, recreating the current race. It also installs a global
SIGCHLD self-pipe handler through mutable static state (`:32-74`) and uses
process-wide bookkeeping keyed by raw `Child` pointers. Do not combine it with
wait4/rustix lifecycle ownership, and remove it from primary measurement
control when implementing this change.

The pinned release is MIT/Apache-2.0, MSRV unspecified, normal Unix
dependency `libc ^0.2.56`, and was published 2025-02-03 after a long period
without releases. Its maintenance status is not a reason to retain it: the
semantic mismatch is fundamental.

## Exact lifecycle recommendation and pitfalls

1. Prepare stdin/capture files and configure creation of a private process group
   before the timing boundary. Spawn directly after recording the start timestamp,
   then record the returned child's PID/PGID. No `Child::try_wait`,
   `wait_timeout`, or wrapper process may run in this lifecycle.
2. For the latest API, poll `Child::try_wait4` as the sole owner. If any
   process-group descendant can survive a normal leader exit, first use
   rustix `waitid(Pid, EXITED|NOWAIT|NOHANG)`; this is an observation, not a
   reap. This two-phase route also works with wait4 0.1.3 if MSRV 1.88 must be
   preserved.
3. On deadline/output/memory-limit, signal the owned private PGID before the
   leader is reaped. Continue the same waitid/try_wait4 loop and classify
   timeout/limit separately from natural nonzero/signaled exit. With
   `wait4 0.1.3`, retry `wait4` on `Interrupted` because that release does not
   do so itself. Never signal after the resource-aware wait has returned.
4. Freeze duration at the agreed exit-observation point before joins, capture
   persistence, conversion, or reporting. Handle `EINTR` explicitly and treat
   `ECHILD`, invalid PID, unit overflow, zero RSS, or unavailable provenance as
   collection failure.
5. Record scope as "specific child plus descendants waited by that child" when
   using OS wait4 rusage. For direct jq/yq/tq workloads, verify whether the
   executable forks; an absence of observed descendants is not a license to
   rename the metric.

Critical race: `try_wait4` returns a fully reaped result, and `wait4 0.1.3`
returns the same kind of resource-bearing reap. Calling
`kill_process_group`, `Child::kill`, `Child::wait`, or ordinary `try_wait` after
that point may target a recycled PID. Conversely, `waitid(... WNOWAIT ...)`
does not provide rusage and must always be followed by exactly one resource-aware
reap. A process group must be signalled before that reap if descendants need
cleanup. The owner must serialize these transitions; no worker or timeout
thread may independently wait.

## Catalog invocation and descendant scope

The campaign catalog currently describes direct executable invocations. The
driver resolves the selected `ToolIdentity.path`, appends the catalog query and
fixture path, and constructs one `BenchmarkInvocation` with an exact argument
vector (`crates/tq-test-support/src/bin/tq-bench.rs:848-877`). The compatibility
runner executes that vector with `std::process::Command::new(...).args(...)`; it
does not insert a shell, pipeline, or command-string wrapper
(`crates/tq-test-support/src/compatibility/process.rs:93-99`). Tool overrides and
path discovery are still external inputs, so the measured binary's path,
version, and digest must remain in campaign provenance.

This establishes a no-shell harness contract, not a universal claim that every
third-party executable is non-forking. The local `tq` production crates have no
`Command`, `exec`, or process-spawn use found in the audited source; their worker
threads are part of the waited child's resource summary. The catalog's jq
adapters use ordinary filter/file arguments (the event-stream adapter adds
`--stream`); the upstream jq CLI source documents filter plus file operands and
its `main.c` has no `fork`, `exec`, `popen`, or `system` call in the inspected
command path. That is useful source evidence for this pinned jq binary, but not
a repository-wide or future-version guarantee. The yq executable is an
externally selected binary and was not proven non-forking by the local Rust
harness; do not make that claim from the catalog alone.

Accordingly, reports must retain the precise wait4 scope: **the exact waited
child plus descendants that it waits for, and the child/thread resource fields
provided by the OS**. They must not relabel rows as “process-only” merely
because no descendant was observed in a sample. If a tool or wrapper creates a
descendant, it is included only when the leader waits for it; an independently
living descendant remains a cleanup/lifecycle concern and invalidates an
unqualified process-tree interpretation. The validation runner's no-sampler
and sampler families are separately labeled because the RSS sampler changes
the invocation lifecycle and its observed timing/CPU dispersion.

Primary jq references used for this limited source check: the
[`jq` project README](https://github.com/jqlang/jq) and its
[`src/main.c`](https://raw.githubusercontent.com/jqlang/jq/master/src/main.c)
and [`src/util.c`](https://raw.githubusercontent.com/jqlang/jq/master/src/util.c)
sources. These references support the documented CLI/file-input behavior; they
are not evidence about arbitrary user-provided jq/yq builds.

### macOS process-group cleanup and `EPERM`

The Apple XNU source makes `killpg(2)`'s macOS result more subtle than a
simple live-versus-missing test. In [`killpg1_callback`][xnu-killpg], XNU
counts a zombie for the signal-permission check, while the process-group
iterator excludes `SZOMB` members from the signal pass. `killpg1` can
therefore return `EPERM` when a group has no signalable member. After
`waitid(... WNOWAIT ...)` observes the leader, `EPERM` can mean either

* the group contains only the waitable zombie leader (and possibly other
  zombies), or
* a live descendant remains but its credentials or MAC policy deny `SIGKILL`.

We do not use that `EPERM` as an empty-group proof. On macOS, the owner keeps
the leader unreaped and calls the safe `libproc` wrapper
`pids_by_type(ByProgramGroup { pgrpid })`. XNU's `proc_listpids` enumerates
both `allproc` and `zombproc` for this filter ([`proc_listpids`][xnu-listpids]),
so each returned PID is inspected with `pidinfo::<BSDInfo>(pid, 1)`. The
nonzero argument requests zombie lookup; XNU returns `pbi_status` from the
BSD-info record ([`proc_pidbsdinfo`][xnu-bsdinfo]), allowing the owner to
discard `SZOMB` entries and accept a group containing no non-zombie member.
The PID and PGID are checked again before classification, avoiding a stale
snapshot becoming a signal target.

`proc_listpids` itself does not apply same-user filtering, but BSD-info does:
XNU selects `CHECK_SAME_USER` for `PROC_PIDTBSDINFO` ([`proc_pidinfo`
security check][xnu-pidinfo]). A credential-denied or MAC-denied live member
therefore remains a visible `libproc` inspection error; it is never silently
treated as an empty group. Group enumeration and BSD-info races are retried
at the bounded cleanup interval. If a non-zombie member remains after the
bound, cleanup fails before the exact-child `wait4` result is accepted. The
leader is still reaped once so a diagnostic cannot strand a zombie, but the
measurement is reported as collection failure.

The direct Rust calls are the safe public APIs from `libproc 0.14.11`:
[`pids_by_type`][libproc-processes] and [`pidinfo`][libproc-pidinfo]. The crate
contains the platform FFI internally; the benchmark helper does not add
unsafe code. The dependency is target-gated to macOS and pinned to the
registry package's MSRV-compatible `0.14.11` release. Its exact local registry
manifest declares Rust 1.72 and MIT licensing, with runtime dependencies
`errno 0.3.14` and `libc ^0.2.176`; `bindgen 0.72.1` is a macOS build-only
dependency used to generate the system bindings. Thus Linux builds do not
compile this adjunct, while macOS builds require the normal libclang/Xcode
headers needed by that build step.

[xnu-killpg]: https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/kern_sig.c#L1549-L1624
[xnu-listpids]: https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/bsd/kern/proc_info.c#L337-L457
[xnu-bsdinfo]: https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/bsd/kern/proc_info.c#L637-L668
[xnu-pidinfo]: https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/bsd/kern/proc_info.c#L2046-L2091
[libproc-processes]: https://docs.rs/libproc/0.14.11/src/libproc/processes.rs.html
[libproc-pidinfo]: https://docs.rs/libproc/0.14.11/src/libproc/proc_pid.rs.html#265

The two-phase route has two timestamps worth naming. `waitid(... WNOWAIT ...)`
observes that the leader is waitable, while `wait4` later returns the status and
rusage. If the benchmark contract ends at the first observation, freeze the
monotonic clock there and say so in provenance. If it ends at the
resource-bearing wait, freeze it when `wait4` returns. Do not let cleanup,
capture joins, or report conversion choose the boundary implicitly.

## Native smoke proof

The exact two-phase flow was compiled and run on this macOS host in the
scratch project `/private/tmp/wait4-rustix-proof` (no repository Cargo files
were changed). It uses `rustix = 1.1.4` with `process`, `wait4 = 0.1.3`,
spawns `sh -c 'sleep 30 & exit 7'` in a private process group, observes the
leader with `EXITED|NOWAIT|NOHANG`, kills the group, and calls the blocking
`Child::wait4` once for the resource-bearing reap. The same source and lockfile
ran on Ironhide Linux from `/var/tmp/wait4-rustix-proof-013`; `/tmp` was over
the host user's quota, so the Linux scratch had to use `/var/tmp`.

Command and result:

```text
macOS (Rust 1.88.0): rustup run 1.88.0 cargo run --offline
Finished `dev` profile
pid=16935 maxrss=2064384 utime=1.05ms stime=1.837ms

Linux (Ironhide, Linux 7.2.0-ogc4.1.fc44.x86_64):
cargo run -j1 --offline --manifest-path /var/tmp/wait4-rustix-proof-013/Cargo.toml
Finished `dev` profile
pid=1835525 maxrss=3973120 utime=0ns stime=1.175ms
```

These runs validate compile-time API shape, nonblocking observation, status
classification, process-group signal call, and one consuming wait on both
targets. They do not validate RSS units against independent `time` output,
allocation-burst controls, or the exact descendant behavior of jq/yq/tq. The
shell proof confirms the lifecycle call sequence, not the numeric descendant
scope of the benchmark workloads. Those controls remain required by tasks
1.3 and 4.1-4.3.

## Primary references

1. [Linux `wait4(2)`](https://man7.org/linux/man-pages/man2/wait4.2.html):
  targeted child selection and resource-bearing wait; it calls wait4
  nonstandard and distinguishes it from waitpid.
2. [Linux `waitid(2)`](https://man7.org/linux/man-pages/man2/waitid.2.html):
  `P_PID`, `WNOHANG`, `WNOWAIT`, status fields, EINTR/ECHILD behavior, and
  zombie retention.
3. [Linux `getrusage(2)`](https://man7.org/linux/man-pages/man2/getrusage.2.html):
  `ru_maxrss` is KiB and for `RUSAGE_CHILDREN` is the largest child, not a
  process-tree peak.
4. [Apple `wait4(2)`](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/wait4.2.html):
  targeted PID, WNOHANG, status, and resource summary for the terminated
  process and its children.
5. [Apple `getrusage(2)`](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/getrusage.2.html):
  archived wording says KiB; the current local macOS `man 2 getrusage` says
  bytes. This version/documentation conflict is why the implementation must
  verify native units against current `/usr/bin/time -l` and page-aware
  allocation controls before publishing.
6. [wait4 crate docs](https://docs.rs/wait4/0.2.0/),
  [rustix process docs](https://docs.rs/rustix/1.1.4/rustix/process/), and
  [wait-timeout crate docs](https://docs.rs/wait-timeout/0.2.1/).
