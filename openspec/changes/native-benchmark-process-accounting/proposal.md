## Why

[Issue #30](https://github.com/commandzero/tq/issues/30) requires benchmark results that describe the executed tool. The current measurement includes setup and cleanup in wall time, wraps tools with `/usr/bin/time`, and requires recurring `ps` sampling even when no sampled limit is requested.

## What Changes

- Launch jq, yq, and tq directly with argument vectors and measure monotonic spawn-to-exit duration after preparing input and capture files.
- Investigate an isolated, low-memory Rust measurement worker to prevent campaign-runner memory from determining Linux child RSS. The worker directly launches and reaps the selected tool; worker startup and communication stay outside the measured interval. Adoption requires parent-memory isolation and independent native validation, not merely a successful prototype.
- Collect the specific child's exit status, CPU usage, and OS-recorded peak RSS through one owner of the wait lifecycle on macOS and Linux. Keep first-party Rust free of unsafe code and FFI bridges.
- Raise the minimum Rust version to 1.95 and use the audited `wait4 0.2.0` dependency for consuming child ownership, interrupted-call retries, and defensive RSS conversion. Retain the approved valid-OS-counter assumption, checked first-party conversions, and independent validation.
- Remove mandatory RSS polling and `time` wrappers from primary measurements. Keep explicitly requested diagnostic sampling or limit enforcement separately labeled and bounded.
- **BREAKING**: Replace the benchmark policy requiring `time` and `ps` with verified native accounting. Add explicit method, scope, availability, and timing precision to reports; preserve old provenance and reject incompatible comparisons.
- Validate short allocation bursts, units, independent children, threads, termination, and timing controls on both native platforms. Compare native counters independently with platform `time`.
- Rerun the supported jq/yq/tq matrix and update stable comparison Results blocks while preserving authored explanations.
- Format comparison tables with units in each measurement cell and one decimal place. Use `-` for missing or untimed measurements, explain placeholders before each table, and keep collector source labels out of the tables while retaining provenance in report metadata and method descriptions.
- Apply issue #30's self-regression acceptance policy: disclose each comparable increase above 20%; documented increases through 50% are acceptable; above 50% blocks acceptance.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `performance-benchmarks`: Direct execution, timing boundaries, native resource accounting, lifecycle ownership, provenance, verification, and published comparison policy.

## Impact

The implementation affects `crates/tq-test-support/src/benchmark/measure.rs`, benchmark orchestration, schemas, report generation, measurement tests, contributor benchmark guidance, and `docs/tests/comparison/`. Audit safe Rust accounting libraries before choosing a dependency. The baseline for this proposal is delta's committed revision `6e357f7`; its jq parity change remains separate.

## Non-goals

Native Windows benchmarking remains deferred to issue #31. This change does not implement a heap profiler, physical-footprint metric, simultaneous process-tree peak, or jq engine optimization. Historical measurements will not be relabeled, and no benchmark is run during this planning change.
