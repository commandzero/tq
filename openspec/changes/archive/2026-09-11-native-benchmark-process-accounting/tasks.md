## 1. Dependency and lifecycle proof

- [x] 1.1 Record the baseline revision, current measurement contract, report schema, and affected policy locations; verify the inventory includes `measure.rs`, preflight, report/Markdown consumers, contributor guidance, and stable comparison pages.
- [x] 1.2 Audit safe Rust accounting candidates for exact-child waiting, timeout integration, status ownership, target support, units, maintenance, licensing, and dependencies; deliver a dependency decision with source references and no first-party unsafe/FFI.
- [x] 1.3 Prove the selected API's direct spawn, resource-aware reap, termination race handling, and unit/scope behavior on native macOS and Linux; retain command and host evidence, and require a separate design decision if no safe API satisfies the contract.
- [x] 1.4 Investigate an isolated low-memory Rust worker through the existing measurement interface; reproduce the Linux parent-memory failure first, then verify independence at 0, 32 MiB, and larger corpus-representative coordinator allocations, large prepared stdin, and high/low request sequences. Retain independent time comparisons and residual-floor evidence; do not adopt the worker or weaken tolerances if the proof fails.

## 2. Process measurement implementation

- [x] 2.1 Add Rust behavioral measurement helpers for literal arguments, stdin bytes/EOF, no-op, known duration, nonzero exit, signals, blocked input, output flood, and descendant cleanup; verify each helper's observable behavior before using it as a measurement oracle. Do not add Python source, bytecode, or cache directories.
- [x] 2.2 Prepare inputs and captures before direct spawning and freeze monotonic duration at resource-aware exit observation; verify argument fidelity and that injected setup/cleanup delays do not extend command duration.
- [x] 2.3 Centralize child waiting, timeout, interruption, cancellation, termination, and cleanup under one owner; verify normal/nonzero/signal exits, deadline races, spawn/wait failures, blocked input, and absence of zombies or double reaping.
- [x] 2.4 Normalize native RSS/CPU units once with checked first-party conversions and explicit availability/provenance; document the approved valid-OS-counter assumption for dependency-internal conversions, test detectable boundary/overflow/missing results, and verify repeated high/low children without cross-sample accounting.
- [x] 2.5 Remove unconditional `ps` and `time` dependencies; isolate selected diagnostic sampling and limit enforcement with explicit scope/interval metadata, and verify default measurements work without those executables while requested limits remain enforced.
- [x] 2.6 Replace the dwell-based preflight with native allocation evidence; verify it precedes corpus work, succeeds without sampling, fails clearly on unsupported platforms, and aborts publication on unavailable authoritative RSS.
- [x] 2.7 After a passing worker proof, integrate bounded worker requests with prepared input/captures, direct target spawning, worker-local timing, exact-child ownership, and fail-closed diagnostics. Verify literal arguments, environment, working directory, bytes/EOF, delayed worker overhead, limits, cancellation, worker/coordinator failure, and cleanup without fallback to the contaminated path.

## 3. Reports and migration

- [x] 3.1 Update outcome types, campaign schema, validation, and aggregation for native provenance, process scope, timing boundaries/precision, and optional instrumentation; verify old records retain their method or unknown status and incompatible samples cannot be pooled.
- [x] 3.2 Implement issue #30 self-regression disclosure and acceptance checks; verify boundaries at 20%, just above 20%, 50%, and above 50%, independent metrics, unavailable baselines, and exclusion of cross-tool ratios.
- [x] 3.3 Update the stable Results renderer with plain metric names, units and one decimal in every measurement cell, integer counts, and `-` for missing/untimed/non-comparable measurements; verify explanations precede tables, source labels stay outside tables, raw provenance and outcome classifications survive, rounding does not affect acceptance, authored text outside markers remains byte-identical, and pages contain no dated report filenames or local archive paths.
- [x] 3.4 Replace current mandatory `time`/`ps` policy in contributor and benchmark guidance, CLI help, and diagnostics with native accounting plus independent `time` validation; search current operational docs for stale requirements and preserve historical evidence.
- [x] 3.5 Record worker executable identity, launch protocol, collector sources, lifetime RSS scope, and residual-floor evidence in reports and calibration. Reject stale or incompatible identities and update operational guidance without relabeling earlier evidence.

## 4. Native verification and campaigns

- [x] 4.1 Rerun the final measurement path on native macOS with multiple page-touch allocation sizes, release-before-exit bursts shorter than the former sampling interval, independent children, concurrent thread allocations, and worker-isolation controls; compare with BSD `time -l` under elevated permissions and record page-aware tolerances and provenance. Revalidate the macOS polling-wait fix; earlier passing controls remain evidence for their collector identity.
- [x] 4.2 Run the same allocation and lifecycle suite on native Linux; compare with GNU `time -v` under required permissions and record unit, process-scope, and cleanup evidence without treating cross-compilation as execution.
- [x] 4.3 Repeat no-op and known-duration controls on both hosts to quantify observed excess duration and dispersion; distinguish startup/scheduling effects from timer error, correct misleading accuracy/error-bound labels, and verify same-host/same-OS comparisons use equivalent harness settings and concessions without requiring a universal 1 ms guarantee. Keep timing and instrumented repetitions separate and retain CPU/RSS and self-regression gates.
- [x] 4.4 Rerun the correctness-approved jq/yq/tq matrix on both native hosts with recorded identities, inputs, warmups, repetitions, and command/input-delivery contracts; retain failures and keep old-method records distinct.
- [x] 4.5 Measure comparable baseline and candidate tq binaries with the same new measurement method; disclose each increase above 20%, document acceptable increases through 50%, and mitigate/repeat any increase above 50% before acceptance.
- [x] 4.6 Publish verified Linux results into stable comparison Results blocks; retain macOS verification separately as requested, verify both native platforms are represented accurately, preserve authored explanations, and never present missing evidence as a pass.

## 5. Completion checks

- [x] 5.1 Rerun focused measurement/report tests and repository preflight after worker changes, including strict OpenSpec validation and applicable documentation checks; record commands and outcomes and resolve change-related failures. Earlier preflight passes remain evidence for the pre-worker implementation.
- [x] 5.2 Review implementation against every issue #30 acceptance criterion and this spec; retain a verification report that identifies native evidence, dependency audit, precision controls, lifecycle tests, matrix results, and the explicit Windows deferral.
