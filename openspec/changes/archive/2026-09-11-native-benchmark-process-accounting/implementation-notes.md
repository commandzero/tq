# Implementation evidence

## Resumed native worker testing

The repeated controls are recorded in [native worker proof results](worker-proof.md).
Across two unchanged release runs per host, all 480 RSS comparisons passed
but 11 CPU comparisons failed. The failed evidence remains retained and
cannot authorize worker adoption or campaign publication.

The user granted standing permission to copy repositories to Ironhide for
testing. The 2026-09-11 source refresh succeeded at the existing isolated
scratch destination. The native Linux release build used the pinned nightly
compiler explicitly and the locked `wait4 0.2.0` dependency.

Linux passed 12 measurement tests, 18 probe tests, one waited-descendant scope
test, eight native-owner tests, and five worker tests. Ironhide's Cargo output
layout places tests under `release/build/.../out`; three direct worker tests
initially failed executable discovery. Supplying the supported explicit
`TQ_BENCH_WORKER` release path made all five pass. The macOS release worker
suite also passed all five tests.

Initial independent-time spot checks are retained in the external archive at
`.work/worker-proof-linux.L4EUbX/` and `.work/worker-proof-macos.vQNGzk/`.
Linux no-op target RSS remained between 2,527,232 and 2,584,576 bytes with
0, 32 MiB, and 128 MiB retained in the coordinator. GNU time reported
2,461,696 to 2,576,384 bytes. The 32 MiB stdin payload round-tripped with a
2,580,480-byte target peak. macOS no-op target RSS was 6,094,848 bytes for
all three coordinator allocations and matched BSD time exactly; the large
stdin target peak was 6,144,000 bytes. These are spot checks, not repeated
proof or campaign calibration.

## Isolated-worker implementation checkpoint

Commit `d8398ca` records the native accounting implementation and approved
Rust 1.95 upgrade. Subsequent work adds an explicit, diagnostic
`measure_process_worker` entry point. The default collector has not switched
to this prototype. Worker adoption still requires the two-host proof in
task 1.4, followed by the lifecycle and limit integration in task 2.7.

Elevated macOS tests passed coordinator allocations of 0, 32 MiB, and 128 MiB,
plus a 32 MiB prepared stdin payload. These are coarse regression checks, not
the required repeated, independent BSD-time calibration. Literal arguments,
binary stdin and EOF, nonzero target exits, and blocked-input timeout checks
also passed. The first test run found a serde reply-decoding failure for
`u128`; externally tagged replies corrected that failure.

Report/schema tests passed 36 checks and Markdown tests passed 16. New report
gates require worker identity and launch-isolation evidence for publication,
while retaining uncalibrated native records as diagnostics. Stable comparison
Results blocks have not been regenerated.

The permissions reviewer rejected the attempted refresh of private source to
`ironhide.local:/var/tmp/tq-native-accounting-bre76E/src/`. Earlier transfers
documented below do not constitute execution evidence for this worker build.
No alternative transfer was attempted after this rejection. A newly approved
source refresh is required before the native Linux proof can proceed.

No completed-worker calibration, paired matrix, or issue #30 acceptance is
claimed. The outstanding task checkboxes remain open.

The final local prototype preflight passed with exit 0: formatting,
all-target checking, strict all-feature Clippy, workspace tests and doc-tests,
documentation validation with zero errors and warnings, and all 20 strict
OpenSpec items. The workspace also passed all-target checking with the actual
Rust 1.95.0 compiler selected explicitly. Five worker integration tests,
16 calibration-loader tests, and the u128 reply and shared-group-drop
regressions passed. This completes local checkpoint validation, not task 5.1's
post-adoption verification.

## Approved MSRV and wait4 upgrade

The user approved Rust 1.95 as the workspace minimum and `wait4 = 0.2.0`.
The workspace manifest, lockfile, README, migration guidance, and changelog
now reflect that requirement. The final reap moves the owned Child into
`wait4`; the dependency handles EINTR retries internally. WNOWAIT observation
and group cleanup still precede that consuming call. Measurement metadata
identifies 0.2.0, invalidating earlier collector calibration.

Upgrading the dependency first reproduced the expected E0507 compile error
at the old borrowed-child retry helper. Removing that helper and transferring
ownership made the all-target workspace check pass. The same locked workspace
also passed all-target checking with the actual Rust 1.95.0 compiler selected
explicitly. Focused native lifecycle, measurement, probe, reporting, and schema
tests passed 115 checks. Formatting, strict change validation, and documentation
validation passed. Clippy reported no errors and the existing unknown-lint
warning. These checks do not validate Linux RSS or the proposed worker.

The full repository preflight then passed with exit 0, including workspace
tests, doc-tests, formatting, compilation, Clippy, documentation validation,
and all 20 strict OpenSpec items. Task 5.1 still requires another run after
the pending worker implementation.

The earlier dependency selection and native runs below remain historical
evidence for 0.1.3. No prior calibration or campaign is relabeled as 0.2.0.

## Resolved dependency pause

The user approved the documented valid-OS-counter assumption and instructed
implementation to continue. The design, requirement, and task 2.4 now reflect
that decision. The draft still requires lifecycle completion and native
validation before it can support published campaigns.

The dependency audit found no audited API satisfying the strict checked-conversion
contract. `wait4 0.1.3` works with the workspace MSRV and passed a two-phase
lifecycle smoke proof on both native hosts, but internally converts RSS and CPU
with unchecked arithmetic. Its safe result does not expose raw fields or overflow
errors. `wait4 0.2.0` requires Rust 1.95 and still does not provide a checked CPU
conversion/error boundary. Raising MSRV alone would not resolve the requirement.

Dependency selection paused until the user approved the assumption above.
The uncommitted native implementation still needs the full native validation
suite before acceptance. No native matrix results have been
published. Report formatting, schema migration, and helper work can be reviewed
independently. See [dependency-audit.md](dependency-audit.md) for source and host
evidence. Task 1.3 remains incomplete because its full unit/scope proof is pending.

## Baseline and affected files

The baseline is `6e357f7`, delta's committed jq parity work. Planning was committed separately as `b975de1`. The baseline measurement contract launches `/usr/bin/time`, starts the clock before capture preparation, samples process-group RSS with recurring `ps`, and ends timing after sampler/input-worker joins and resource parsing.

1. `crates/tq-test-support/src/benchmark/measure.rs`: invocation, wait/kill ownership, timing, CPU/RSS, preflight, output capture.
2. `crates/tq-test-support/src/benchmark/runner.rs`: correctness invocations, warmups, samples, infrastructure-error propagation.
3. `crates/tq-test-support/src/benchmark/report.rs`: samples, provenance, summaries, comparability, regression gates.
4. `crates/tq-test-support/src/benchmark/markdown.rs`: generated Results regions and metric presentation.
5. `crates/tq-test-support/src/bin/tq-bench.rs`: campaign preflight, report publication, baseline CLI controls.
6. `schemas/benchmark-campaign-v1.schema.json`: persisted metric/provenance shape.
7. `crates/tq-test-support/src/bin/tq-bench-helper.rs` and benchmark integration tests: measurement behavior controls.
8. `CONTRIBUTING.md`, `benchmarks/README.md`, `scripts/run-campaign.sh`, and current benchmark method guidance: mandatory wrapper/sampler policy and invocation.
9. `docs/tests/comparison/`: stable generated Results regions; historical authored findings must remain identifiable.

## Native hosts

The local worktree is on macOS. A read-only SSH check confirmed Ironhide runs Linux and has Cargo and `/usr/bin/jq`; yq was not on the noninteractive PATH. This is host discovery, not native measurement verification.

The local system jq is Apple 1.7.1, so it is not the pinned matrix executable.
The official `jq-macos-arm64` asset from release `jq-1.8.1` was downloaded
into the external archive. Its SHA-256 matches the release's `sha256sum.txt`:
`a9fe3ea2f86dfc72f6728417521ec9067b343277152b114f4e98d8cb0e263603`.
The downloaded binary reports `jq-1.8.1`; local Brew yq reports `v4.53.2`.
No system executable was replaced.

A separate source export of baseline commit
`6e357f77d5166c7b0c153b6e8e23175fddb0c1ea` is being built with
`cargo build --release --locked -p tq-cli --bin tq`. Baseline and candidate
must both be measured with the new collector. Old wrapper timing cannot
establish a tq self-regression or improvement.

## Focused checks before the design pause

- `cargo test -p tq-test-support --test benchmark_reporting --test benchmark_report_schema`: 17 passed. Covers schema history/native requirements, collection-method mismatch, unavailable baselines, independent metrics, and unrounded 20%/50% boundaries.
- `cargo test -p tq-test-support --lib benchmark::markdown`: 9 passed. Includes units, placeholders, source placement, and authored-text preservation.
- `cargo test -p tq-test-support --test benchmark_probe`: 10 passed. Validates Rust helper behavior without treating helpers as calibrated RSS or timing evidence.
- `cargo check -p tq-test-support --all-targets`: passed for the current macOS draft.
- Strict OpenSpec validation and `git diff --check`: passed.
- Clippy identified unfinished native draft lint cleanup. The full repository preflight and lifecycle/calibration suites have not passed or been claimed.
- The exploratory release build was stopped when the dependency decision blocked campaigns. It is not build verification.

All implementation edits remain uncommitted. No Python source or generated
Python cache was added. Stable comparison pages have not been regenerated from
unverified native measurements.

Since that pause, focused tests cover bounded sampler subprocess/worker cleanup,
sampler failure propagation, cancellation, retained accounting-error diagnostics,
and repeated deadline races. Native unit/precision calibration and platform
lifecycle verification remain required before campaigns. Task completion is
tracked in `tasks.md`; the test counts above describe the earlier checkpoint.

## Current implementation checkpoint

- macOS native-owner tests: 8 passed, including externally consumed wait status
  and injected cleanup failure followed by the exact-child reap.
- macOS measurement integration tests: 11 passed, including repeated deadline
  races, independent high/low children, cancellation, blocked input, and limits.
- macOS waited-descendant allocation scope: passed across three runs. A waited
  allocation child materially raises the parent's returned wait4 RSS; the
  protocol retains this scope rather than claiming a pure process-tree sum.
- Report/schema/CLI checks: 34 passed. Temporary corpus relocation can compare
  by immutable content identity, while changed commands and inputs remain
  non-comparable. Collector calibration rejects foreign hosts, different
  collector sources, debug controls, missing samples, and unsupported precision.
- Renderer tests: 10 passed. Sources remain outside tables, measurement cells
  use one decimal and units, and authored text survives regeneration.
- Ironhide scratch `/var/tmp/tq-native-accounting-bre76E` passed a release build,
  11 measurement tests, 13 probe tests, and the earlier five native-owner tests.
  Updated source and the final allocation/calibration suite still need reruns.
- Final release controls and equivalent-method baseline/candidate matrices are
  pending. Builds must finish before authoritative timing controls run.

The native control runner retains raw independent BSD/GNU time output,
RSS/CPU comparisons, multiple allocation sizes, thread allocations, and
sampler-on controls in the external archive. A compile/test pass is not a
passing native validation run. No new native comparison Results blocks have
been published at this checkpoint.

## macOS release counter validation

The elevated release control run at external archive path
`.work/native-accounting-validation/1789094807093492-304/` passed all 280 RSS
and CPU comparisons with BSD `/usr/bin/time -l`. It recorded 20 repetitions
for each of 14 controls. Allocation-delta, no-op isolation, and released-burst
checks passed, with no entries in `errors.jsonl`.

Summary SHA-256:
`b06fdef7751cede587eb9f8cbf7d2b098edd71a7c6d98e3fb4184afbc674fd08`.
Compiled collector-source identity:
`ee998c2040d2ddb833962ac68eff1dbf9a3822ffa64b17a49fe445c8dbe8797e`.

The largest observed known-duration excess was 39,720 microseconds. This
includes startup and sleep scheduling and is not a universal timing-error
guarantee. Sampler-on wall median differences were at most 594 microseconds,
below the corresponding no-sampler median absolute deviations; no timing
distortion was resolved above that dispersion. RSS medians were unchanged.
Separate sampler calibration linkage and an additional sampled 250 ms control
are being added before matrix publication. Linux independent controls and
both full matrices remain pending.

The expanded macOS control run
`.work/native-accounting-validation/1789095183265637-6221/` passed RSS/CPU
validation but resolved a sampler-on wall-time difference for the 20 ms
control: 3,698 microseconds versus a no-sampler MAD of 1,077 microseconds.
This supersedes the earlier "not resolved" observation for that control.
The implementation therefore separates RSS-limit enforcement repetitions
from primary timing repetitions as the spec requires. An instrumented
repetition must succeed before the matching timing repetition runs. Timing
still collects native RSS and checks its limit at exit, but has no in-flight
RSS sampler; timeout and output enforcement remain active. Native controls
must be rerun after the collector-source fingerprint changes.

## Separated-repetition checkpoint

The public runner tests first demonstrated that sampled RSS enforcement leaked
into primary timing samples, then passed after separating those collections.
A second test demonstrated that a failed sampled enforcement attempt could be
misclassified as a timing sample; failed attempts now retain their instrumented
classification. All 20 runner gate tests passed.

The latest focused macOS run passed 29 measurement, probe, and waited-descendant
tests. All eight CLI/calibration tests and the all-target compilation check
passed. These checks do not replace the pending final native controls or
baseline/candidate matrices.

The campaign script forwards `TQ_TIMING_CALIBRATION`. Standard publication
without it exits with status 64 before corpus work; shell syntax validation
passed. Direct CLI standard runs remain available for diagnostic recording
without publication.

ShellCheck passed for `scripts/run-campaign.sh`. `okf validate docs` reported
75 concepts, zero errors, zero warnings, and five informational links to
non-concept artifacts. Strict validation of this OpenSpec change passed.

The full preflight passed compilation, Clippy, and the benchmark test suites,
then the existing fixture-storage test rejected generated JSON under ignored
`tests/fuzz/target`. Git confirmed that directory contained no tracked files.
The build cache was preserved, not deleted, under the external archive's
`.work/fuzz-cache-preserved.lJ6rO9/target`, and preflight was restarted without
changing the fixture policy. Clippy still reports the repository's pre-existing
unknown `clippy::assert_is_empty` lint warning on this toolchain.

The rerun of `./scripts/preflight.sh` completed successfully with exit status 0.
Formatting, all-target compilation, Clippy, the full workspace test suite,
doc-tests, and all 20 strict OpenSpec validations passed.

The permission reviewer initially blocked Linux source refresh pending explicit
authorization to transmit private repository source to `ironhide.local`. The
user authorized the exact scratch destination, and the subsequent source
transfer succeeded. No alternate transfer bypassed the rejection. Existing
Linux scratch evidence is not evidence for the final collector revision.

## Final macOS collector controls

The elevated release run at
`.work/native-accounting-validation/1789097046410547-79973/` passed all 300
RSS and CPU comparisons against independent BSD `/usr/bin/time -l`, with
20 repetitions for each of 15 controls. Allocation deltas, released bursts
under 25 ms, and no-op isolation passed. No-op RSS before and after allocation
controls was identical at 6,127,616 bytes. No validation failures were recorded.

Compiled collector-source SHA-256:
`d480d1ef72759eba4dba4e44cf42eed66758809811bb16594296135fdb198fd2`.
Summary SHA-256:
`7d50d8fbd8d47bd5a1d29d0c8266c528b4f5c1e493b61bf4de6ef3430cf0f199`.

Observed known-duration excess bounds were 17,895 microseconds without sampling
and 20,569 microseconds with sampling. These include process startup and sleep
scheduling, not a universal accuracy guarantee. This run resolved no sampler
wall-time difference above the paired control's MAD, but earlier controls did;
separate timing and enforcement repetitions remain required. Native Linux
controls and both equivalent-method campaign reviews remain incomplete.

## Review corrections

The spec review found that missing live process-group observations could pass
requested RSS enforcement. A public probe test reproduced this and passed
after the lifecycle owner rejected missing observations while its exact child
was still alive. A child that exits before inspection still uses native
exit-time RSS enforcement; its missing sampled observation remains explicit.
The Rust inspection fixture avoids shell-startup timeout noise. All 18 probe
tests passed, and Clippy reported no code errors.

Uncalibrated native reports remain diagnostic JSON but no longer support
comparisons, regression evaluation, or reference ratios. Two unknown-provenance
reports are explicitly non-comparable. The reporting and schema tests passed
32 checks. Short-burst calibration gating is being corrected before controls
are rerun. These collector changes supersede the earlier final-control source
fingerprint; those retained records are not relabeled.

## Updated native Linux lifecycle checks

After the authorized refresh, the release build completed with exit 0 in
1 minute 3 seconds on Ironhide using the recorded nightly toolchain. The
measurement suite passed 12 tests, the probe suite 18, and waited-descendant
scope one. The native-process unit suite passed seven tests with one test
thread. Native independent GNU-time controls remain separate and pending.

The Linux measurement unit suite also passed six tests, including detectable
invalid/overflowed counter rejection, sampler cleanup and error propagation,
and timing isolation from injected setup/cleanup delays. Together with the
macOS tests, these complete the lifecycle and first-party conversion checks;
independent platform counter validation remains a separate acceptance task.

The source lives at `/var/tmp/tq-native-accounting-bre76E/src`, with the
task-owned `target` and `tmp` directories beside it. These current file hashes
match the local collector sources; the initial scratch `SNAPSHOT.txt` is stale
after the refresh and must not identify these results.

| File | SHA-256 |
| --- | --- |
| `measure.rs` | `2a14ca6449e82422352d82c393bab702a1cd369acc59a9644cce8b7df2376f9d` |
| `native_process.rs` | `639e4683bd47cfd22b8a05f2eb577ed80112b2e51c3252f57702997d19bebe8a` |
| `probe.rs` | `e309bce2c85b2a29858221be2da75d49e5da2778560747bc5a469cfd3db746b9` |
| `tq-bench-probe.rs` | `e1f5e87c8c1f21a6ab73df9194cbebb666f37102f8c249ee052d05ac91d2d925` |
| `Cargo.lock` | `ba3097a9f924c972d4a5b5aca701808c9fedfc5d195ddba30a8706e14c03a022` |

Full preflight passed again after review corrections, including the new pinned
OKF 0.2.7 documentation gate and OpenSpec 1.11.0 prerequisite check. All workspace
tests passed, including nine CLI/calibration tests; documentation validation
reported zero errors and warnings, and all 20 strict OpenSpec items passed.

## Post-review native controls

The current collector fingerprint is
`84d22031d93785c02b261674dbb2c3891b0f878395afe854534edcdb19518872`.
The macOS release run retained at
`.work/native-accounting-validation/1789099631201642-23350/` passed all
300 RSS and 300 CPU comparisons, all required allocation deltas, no-op
isolation, and released-burst checks. Its summary SHA-256 is
`f4f96a7a3395263e28f888c0a3da2cb1c27f319a75c703a506b2044e3841c991`.
Observed known-duration excess bounds were 22,353 microseconds without
sampling and 22,487 microseconds with sampling. These are observed startup
and scheduling bounds, not universal accuracy guarantees.

Linux independent validation did not pass. The retained run at
`/var/tmp/tq-native-accounting-bre76E/tmp/native-validation/1789099700071176-1965243/`
has the same collector fingerprint and 300 paired records. CPU comparisons
passed, but the small controls returned a fixed native RSS of 4,694,016 bytes
against roughly 2.4 MB under independent GNU time. Larger allocations agreed.
The 4 MiB allocation delta also failed because of the elevated control floor.
Both calibration linkages correctly remain
`unverified-validation-failures`. Linux matrices and publication are paused
pending diagnosis; no tolerance has been weakened or counter offset subtracted.

The macOS baseline smoke campaign completed all 18 rows with outcome `Timed`
and overall status `Passed`. Its report remains in the external archive at
`.work/native-accounting-macos.JppnvV/baseline-smoke.json`. It is baseline
evidence only, not a completed self-regression comparison or full matrix.

## Linux launcher-memory reproduction and design pause

A separate safe-Rust scratch executable called the unchanged
`measure_process_uninstrumented` interface on the same release no-op probe,
varying only a retained, page-touched allocation in the launcher. Four native
elevated runs returned:

1. Parent allocation 0 bytes: child peak 2,576,384 bytes.
2. Parent allocation 33,554,432 bytes: child peak 36,016,128 bytes.
3. Parent allocation 0 bytes: child peak 2,564,096 bytes.
4. Parent allocation 33,554,432 bytes: child peak 35,753,984 bytes.

All children exited successfully. This reproduces launcher-memory dependence
through the actual collector, without changing the target workload. The scratch
build used its own dependency lock and is diagnostic evidence, not a campaign
calibration or a replacement for the workspace-locked validation run.

The Linux [exec implementation](https://raw.githubusercontent.com/torvalds/linux/master/fs/exec.c)
preserves the old address space's high-water RSS in the task's resource usage.
The [resource-usage manual](https://man7.org/linux/man-pages/man2/getrusage.2.html)
also documents preservation across exec. Exact-PID waiting does not remove
this pre-exec history. Subtracting parent RSS would be invalid because the
counter represents a maximum, not additive memory usage.

Scratch sources are preserved outside the repository under
`.work/native-accounting-macos.JppnvV/parent-rss-repro-source`; the remote
diagnostic is under the task's `tmp/parent-rss-repro`. No scratch source is
included in the implementation. No diagnostic process remains running.

The current direct-launch design cannot substantiate target-only Linux memory
comparisons when launcher memory exceeds the target peak. Acceptance remains
paused for a design decision. A separately initialized, low-memory measurement
worker is a candidate to investigate, not a verified fix. It would need explicit
launch-inclusive scope and independent controls before any campaign resumes.
