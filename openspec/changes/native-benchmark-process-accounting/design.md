## Context

See [proposal.md](proposal.md) for motivation and scope. The starting revision is `6e357f7`, copied from delta after its work was committed.

`benchmark/measure.rs` starts `Instant` before temporary-file preparation, launches `/usr/bin/time`, starts a recurring `ps` process-group sampler, uses `wait_timeout` plus ordinary child waits, and computes final wall time after joins and resource parsing. Preflight requires the sampler to observe a Python allocation held for 400 ms. These responsibilities must change together because a separate ordinary wait can consume the resource collector's child status.

`benchmark/{runner,report,markdown}.rs`, the campaign schema, and tests already carry RSS provenance and stable Results-block publication. Extend those existing consumers. The active `achieve-jq-manual-parity` change is separate; this proposal does not alter its artifacts or task completion.

## Goals / Non-Goals

Keep one public measurement interface for native macOS and Linux. Encapsulate command preparation, child ownership, accounting, and cleanup behind it. Use a private platform module within `tq-test-support`; a new workspace crate is unnecessary for this single consumer.

An internal Rust worker executable within the existing package is permitted for the approved launcher-isolation investigation. Keep the public measurement interface unchanged. Do not add a general-purpose process service or new workspace crate for the worker.

Implement measurement helpers and allocation preflight in Rust. Do not add or commit Python source, Python bytecode, or `__pycache__` directories. Replace the current Python-based allocation probe as part of this change.

Do not generalize this into a process profiler. The accepted tool workloads must have verified process accounting semantics; arbitrary executables that fork require a documented scope assessment before their memory can enter comparisons. Native Windows remains unsupported in this implementation.

## Decisions

### One native process owner

Prepare command configuration and capture destinations first. Prefer a prepared seekable stdin file for the existing finite `BenchmarkInvocation.stdin` payload so input delivery does not require a potentially blocked writer thread. Preserve the exact supplied bytes and EOF behavior; keep any intentionally piped input workload explicit in metadata and comparisons.

Capture the start timestamp immediately before direct `Command::spawn`. Transfer completion ownership into a private running-process object whose only reap operation also retrieves resource usage for that PID. Its terminal result contains exit status, the observation timestamp, CPU usage, and raw/native-normalized memory evidence. The timer ends at exit observation before joins, conversion, file persistence, or reporting.

Use a safe resource-aware wait API with either bounded nonblocking waits or a dedicated resource-aware waiter and coordinated deadline control. Choose the concrete mechanism in the dependency spike after measuring exit-observation latency. A fixed multi-millisecond polling loop cannot substantiate sub-millisecond precision. Timeout and limit controllers may request termination but cannot call `wait`, `try_wait`, or a timeout helper that independently reaps the child. Handle interrupted waits and exit/kill races explicitly. Prevent signals to reused process identities through ownership and synchronization verified in the spike.

Track the dedicated process group for bounded descendant cleanup. The old policy of preserving the `/usr/bin/time` supervisor no longer applies. Keep cleanup failures visible and stop reusing an execution path that could leave live processes. Freeze duration when exit is observed even if later cleanup fails. Do not depend on sampler shutdown to discover completion.

Alternatives rejected: retaining the wrapper measures a different launched executable; a second wait path races with resource collection; cumulative `RUSAGE_CHILDREN` mixes sample accounting.

### Isolated measurement worker investigation

Native Linux controls exposed a launcher-memory dependency. The same no-op target reported roughly 2.6 MB from an empty launcher and 36.0 MB when its launcher retained a page-touched 32 MiB allocation. Linux preserves pre-exec high-water RSS. Exact-child waiting therefore does not establish target-only post-exec memory. See the retained reproduction in [implementation-notes.md](implementation-notes.md).

The user approved investigating an isolated, low-memory Rust measurement worker. Initialize the worker in its own executable image before it launches the target. Its own inherited lifetime RSS must never substitute for the target's resource result. The worker directly spawns the selected executable, owns its exact-child wait and cleanup, and measures the interval using its own monotonic clock. Worker startup, request transfer, reply transfer, and worker teardown stay outside that interval. This is a specific exception permitting an internal measurement process, not permission to time a shell, `time`, or the worker in place of the tool.

Prepare bulk stdin and capture files in the coordinator. Send bounded command metadata and references to prepared files, not corpus contents or accumulated reports. The worker must not load or retain payload-sized buffers before spawning the target. Preserve argument bytes, input bytes, environment, working directory, output contracts, and requested limits through the existing public measurement interface. Select fresh-worker or reuse behavior only after proving that earlier requests and worker allocations cannot contaminate later samples.

Keep one target completion owner in the worker. The coordinator may manage the worker's lifecycle, but may not race it to reap the target. Bound control messages and startup, communication, cancellation, and teardown waits. Missing workers, malformed or truncated replies, worker failures, and lost control channels must produce infrastructure failures with retained diagnostics, never successful zero samples or fallback to the contaminated path. Verify cleanup of the target and its process group when the coordinator or worker fails. If safe APIs cannot substantiate that lifecycle, pause for another design decision.

The result remains native child lifetime RSS, including validated pre-exec launch and waited-descendant effects. Do not claim a pure post-exec metric, subtract an estimated floor, or reset a live target's peak. Record the worker executable identity, launch protocol, resource scope, residual floor evidence, and collector source identity. A changed worker or protocol invalidates old calibration and incompatible comparisons.

Before adopting the worker, repeat the same small target with coordinator allocations of 0, 32 MiB, and a larger corpus-representative size, plus high/low request sequences. Test large prepared input delivery separately so the worker cannot reintroduce the floor by copying stdin. Require independence within declared page-aware tolerances and agreement with independent GNU time on Linux and BSD time on macOS. Existing tolerances must not be loosened merely to accommodate the failed collector. A residual floor that obscures the supported small-tool workloads blocks adoption. Fresh controls and calibration are required for the final measurement path on both hosts before matrices or publication resume.

### Safe API audit before dependency selection

Audit existing safe Rust candidates, starting with the `wait4` crate and supported APIs in established Unix libraries. Inspect source and exact versions for PID targeting, reaping, timeout integration, status conversion, error behavior, supported targets, CPU/RSS units, licensing, maintenance, and transitive dependencies. A crate name is not evidence that its interface can meet this lifecycle.

Pin the selected dependency only after a small native lifecycle proof on macOS and Linux. Keep all first-party code under the existing unsafe-code prohibition. If no safe library can provide the required lifecycle, record the evidence and request a separate design decision. Do not add local libc calls or hide them behind a first-party wrapper.

The OS manuals support targeted resource-aware waiting, but returned accounting can contain inherited descendant usage depending on platform and field. Audit that detail against actual jq/yq/tq process behavior and native controls. A result whose scope cannot be established is unavailable for process-only comparison; do not rename descendant-inclusive data to satisfy a label.

### Explicit timing and memory evidence

Retain existing duration storage where adequate and add timing-method and validated-precision metadata. Persist the captured interval; never call `elapsed()` again to manufacture the final duration. Precision documentation must include scheduling and wait observation overhead, not just clock resolution. No-op and known-duration helpers quantify it; do not subtract an estimated constant from individual samples.

Extend RSS provenance with explicit native methods and scope while preserving old `bsd-time-l` and `gnu-time-v` records as historical methods. Normalize platform units once: Linux `ru_maxrss` is KiB; verify macOS units against its native API and independent controls before publishing byte values. If a dependency already normalizes units, do not multiply twice. First-party conversions use checked arithmetic and reject detectable invalid or overflowed values.

The user approved raising MSRV to Rust 1.95 to select `wait4 = 0.2.0` with `rustix = 1.1.4`. The consuming `wait4` call owns the child and retries interrupted calls internally. Keep WNOWAIT observation and process-group cleanup before that consuming call. RSS conversion clamps negative values to zero and saturates multiplication; CPU conversion still uses unchecked signed arithmetic. The approved valid, nonnegative OS-counter assumption therefore remains, together with checked first-party conversions and rejection of detectable invalid results. The API does not expose raw values or a saturation indicator. This upgrade does not provide pure post-exec RSS or resolve the Linux launcher-memory floor. Require positive RSS, identify the selected version in accounting metadata, and repeat independent native validation. Do not introduce first-party unsafe code.

Continue to require authoritative evidence before valid campaign publication. A process may have a useful exit diagnostic even if memory collection fails; retain that diagnostic separately. Decode old schemas with their existing provenance or unknown evidence, and prevent aggregation across incompatible measurement contracts. Update Rust types, schema enumerations, validation, summary logic, and Markdown together.

### Optional instrumentation is separate

Remove unconditional `ps` invocation and sampler prerequisites. If configured RSS limits still need a sampler, opt into it explicitly, report the sampled process-group scope, interval, and missed-peak limitation, and retain it only for enforcement or diagnostics. Do not sum separately observed per-process high-water marks.

Output-limit and first-byte observation also introduce work. Retain bounded output enforcement, label first-byte observation precision, and validate overhead. When instrumentation measurably distorts duration, run equivalent timing and memory/diagnostic repetitions separately and associate them by workload, executable, input, and host identity. Never silently disable a requested limit.

### Policy and report migration

Issue #30 supersedes the current mandatory `time`/`ps` guidance for these campaigns. Update `CONTRIBUTING.md`, benchmark guidance, CLI help, preflight diagnostics, and current operational documents during implementation. Keep historical archived specs and old campaign evidence intact. Use BSD `/usr/bin/time -l` and GNU `/usr/bin/time -v` only for independent validation runs with appropriate elevated permissions.

Generate only the bounded Results regions in stable `docs/tests/comparison/` pages. Preserve explanations outside them byte-for-byte and avoid dated report filenames or local archive paths. Maintain raw campaign evidence in the existing external benchmark storage and use stable repository-facing references where needed.

Use plain metric labels such as `Wall time` and `Peak RSS`. Each measurement cell includes its own unit and exactly one decimal place, for example `138.6 ms`, `64.7 MiB`, or `42.0 MiB/s`. Keep counts as integers. Round only for presentation; preserve full stored precision for calculations and acceptance checks. State validated timing accuracy in the method description so display precision does not imply measurement accuracy.

Do not put source labels such as `gnu-time-v`, `bsd-time-l`, or native collector identifiers in table cells or headers. Keep collection provenance in machine-readable metadata and describe the measurement method outside the tables. Render unavailable, unsupported, failed, unmeasured, and non-comparable measurement cells as `-`. Before every affected table, explain that `-` means no valid comparable measurement and is not zero; describe applicable exclusions and failures there so their meaning remains visible. Outcome coverage tables can retain outcome names and integer counts because those rows describe coverage rather than measurements.

Apply the new 20%/50% policy to comparable tq self-regressions for this issue. Compare baseline and candidate tq binaries under the same new measurement implementation, host, fixtures, build settings, query, and output contract. Comparing old-wrapper duration with direct duration measures the harness change, not a tq performance gain. Report missing comparable baselines explicitly; never count them as passing evidence.

## Risks / Trade-offs

- Resource-aware wait support may differ across libraries and targets. Resolve this with the first dependency/lifecycle spike; keep native verification incomplete until both hosts pass.
- OS accounting can include descendant effects. Verify the metric's scope on each platform and exclude unsupported process topologies from process-only comparisons.
- Linux lifetime RSS can retain pre-exec memory. A low-memory worker may reduce that floor but is not a proven post-exec accounting API; failed isolation or independent controls keep acceptance blocked.
- A reaped child and a deadline signal can race. Central ownership, synchronized termination, and race tests must prevent double reaping and stale-PID signaling.
- Preloaded stdin changes a pipe-based benchmark contract. Record the input delivery method and rerun comparable baselines; preserve explicit pipe workloads where that behavior is under test.
- RSS varies with runtime baseline, allocator retention, and page size. Use several allocation sizes, matched controls, documented tolerances, and repeated samples.
- Native Linux host access may be unavailable during implementation. Complete local work, but report Linux evidence and matrix acceptance as pending until a native host runs them.

## Migration Plan

1. Record the baseline and audit the native waiting dependency; prove status, termination, units, and ownership on both platforms.
2. Replace the measurement lifecycle and add behavior tests before integrating new provenance into reports.
3. Update schema and historical-reader behavior, preflight, documentation, and stable Results rendering.
4. Run native controls, independent `time` comparisons, and correctness-gated jq/yq/tq campaigns. Re-measure comparable tq baselines with the same method and apply issue #30 thresholds.
5. Validate and review the change before publication. If the collector fails, stop authoritative campaigns; rollback must restore the old provenance and policy, never relabel old data as native accounting.

## References

1. [Issue #30](https://github.com/commandzero/tq/issues/30).
2. [Linux wait4](https://man7.org/linux/man-pages/man2/wait3.2.html).
3. [Linux resource usage](https://man7.org/linux/man-pages/man2/getrusage.2.html).
4. [Apple wait4 manual](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/wait4.2.html).
5. [wait4 0.2.0 documentation](https://docs.rs/wait4/0.2.0/wait4/), the selected version. See [dependency-audit.md](dependency-audit.md) for source inspection and constraints.
