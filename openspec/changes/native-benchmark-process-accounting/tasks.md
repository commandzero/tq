## 1. Dependency and lifecycle proof

- [ ] 1.1 Record the baseline revision, current measurement contract, report schema, and affected policy locations; verify the inventory includes `measure.rs`, preflight, report/Markdown consumers, contributor guidance, and stable comparison pages.
- [ ] 1.2 Audit safe Rust accounting candidates for exact-child waiting, timeout integration, status ownership, target support, units, maintenance, licensing, and dependencies; deliver a dependency decision with source references and no first-party unsafe/FFI.
- [ ] 1.3 Prove the selected API's direct spawn, resource-aware reap, termination race handling, and unit/scope behavior on native macOS and Linux; retain command and host evidence, and require a separate design decision if no safe API satisfies the contract.

## 2. Process measurement implementation

- [ ] 2.1 Add Rust behavioral measurement helpers for literal arguments, stdin bytes/EOF, no-op, known duration, nonzero exit, signals, blocked input, output flood, and descendant cleanup; verify each helper's observable behavior before using it as a measurement oracle. Do not add Python source, bytecode, or cache directories.
- [ ] 2.2 Prepare inputs and captures before direct spawning and freeze monotonic duration at resource-aware exit observation; verify argument fidelity and that injected setup/cleanup delays do not extend command duration.
- [ ] 2.3 Centralize child waiting, timeout, interruption, cancellation, termination, and cleanup under one owner; verify normal/nonzero/signal exits, deadline races, spawn/wait failures, blocked input, and absence of zombies or double reaping.
- [ ] 2.4 Normalize native RSS/CPU units with checked conversions and explicit availability/provenance; verify platform conversion boundaries, overflow, missing data, and repeated high/low children without cross-sample accounting.
- [ ] 2.5 Remove unconditional `ps` and `time` dependencies; isolate selected diagnostic sampling and limit enforcement with explicit scope/interval metadata, and verify default measurements work without those executables while requested limits remain enforced.
- [ ] 2.6 Replace the dwell-based preflight with native allocation evidence; verify it precedes corpus work, succeeds without sampling, fails clearly on unsupported platforms, and aborts publication on unavailable authoritative RSS.

## 3. Reports and migration

- [ ] 3.1 Update outcome types, campaign schema, validation, and aggregation for native provenance, process scope, timing boundaries/precision, and optional instrumentation; verify old records retain their method or unknown status and incompatible samples cannot be pooled.
- [ ] 3.2 Implement issue #30 self-regression disclosure and acceptance checks; verify boundaries at 20%, just above 20%, 50%, and above 50%, independent metrics, unavailable baselines, and exclusion of cross-tool ratios.
- [ ] 3.3 Update the stable Results renderer with plain metric names, units and one decimal in every measurement cell, integer counts, and `-` for missing/untimed/non-comparable measurements; verify explanations precede tables, source labels stay outside tables, raw provenance and outcome classifications survive, rounding does not affect acceptance, authored text outside markers remains byte-identical, and pages contain no dated report filenames or local archive paths.
- [ ] 3.4 Replace current mandatory `time`/`ps` policy in contributor and benchmark guidance, CLI help, and diagnostics with native accounting plus independent `time` validation; search current operational docs for stale requirements and preserve historical evidence.

## 4. Native verification and campaigns

- [ ] 4.1 Run multiple page-touch allocation sizes, release-before-exit bursts shorter than the former sampling interval, independent children, and concurrent thread allocations on native macOS; compare with BSD `time -l` under elevated permissions and record page-aware tolerances and provenance.
- [ ] 4.2 Run the same allocation and lifecycle suite on native Linux; compare with GNU `time -v` under required permissions and record unit, process-scope, and cleanup evidence without treating cross-compilation as execution.
- [ ] 4.3 Repeat no-op and known-duration controls on both hosts to quantify spawn/wait overhead, observation error, and dispersion; document supported precision and split timing versus instrumented repetitions when distortion is measurable.
- [ ] 4.4 Rerun the correctness-approved jq/yq/tq matrix on both native hosts with recorded identities, inputs, warmups, repetitions, and command/input-delivery contracts; retain failures and keep old-method records distinct.
- [ ] 4.5 Measure comparable baseline and candidate tq binaries with the same new measurement method; disclose each increase above 20%, document acceptable increases through 50%, and mitigate/repeat any increase above 50% before acceptance.
- [ ] 4.6 Publish verified results into stable comparison Results blocks; verify both native platforms are represented accurately, authored explanations survive, and missing evidence is not presented as a pass.

## 5. Completion checks

- [ ] 5.1 Run focused measurement/report tests during development and the repository preflight at completion, including strict OpenSpec validation and applicable documentation checks; record commands and outcomes and resolve change-related failures.
- [ ] 5.2 Review implementation against every issue #30 acceptance criterion and this spec; retain a verification report that identifies native evidence, dependency audit, precision controls, lifecycle tests, matrix results, and the explicit Windows deferral.
