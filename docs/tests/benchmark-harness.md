---
type: Review
title: Benchmark harness repair
description: Tasks and verification for correctness checks in the rapid, standard, and smoke benchmark campaigns.
status: stable
generated: { by: codex/gpt-6, at: 2026-09-11T21:00:40Z }
---

# Benchmark harness repair and review readiness

The benchmark correctness checks must follow the selected output format.
Default TOON output does not require RS framing.

## Tasks

1. [x] Decode default TOON result streams, explicit sequences, JSON, and raw output according to the invocation.
2. [x] Preserve bounded correctness diagnostics and distinguish semantic differences, decoding failures, and resource limits. Allow valid empty selections without inventing a first-output failure.
3. [x] Revalidate adapter exclusions and record a reason for each remaining exclusion.
4. [x] Return a failing campaign exit status when observations fail, while preserving the report and showing progress.
5. [x] Verify rapid and standard correctness coverage with freshly built executables.
6. [x] Require an RSS preflight and abort immediately when authoritative RSS cannot be collected.
7. [x] Run the accepted Linux rapid, standard, and smoke review with the final reports.
8. [x] Render the accepted Linux reports and write the wall/RSS findings in the comparison index.

Three Luna subagents at xhigh reasoning handled stream decoding, adapter
eligibility, and campaign reporting. Integration review added regressions for
capture limits, nonzero exits, and complete consumption of output.
The presentation changes below use only summary values already present in the
report. Report and schema provenance fields remain owned by the measurement
workstream.

## Acceptance

A passing correctness check compares ordered values and process behavior with jq.
Raw-output contracts compare exact bytes.

Keep measured invocations unchanged when repairing output decoding.
Do not force sequencing or disable a working adapter to conceal a mismatch.

Resource limits and unsupported adapters remain visible with reasons.
They do not count as successful correctness checks or timed results.

An accepted campaign must first prove that the native accounting collector
returns positive RSS for a measured allocation child. Explicit process-group
inspection is checked separately when selected for limit enforcement. If
preflight fails, abandon the campaign immediately and
repair the sandbox or host. If any later measured sample lacks authoritative
RSS, abort the campaign and discard the partial performance report.

## Evidence

The initial rapid diagnosis recorded 16 timed, 11 incorrect, and 3 unsupported rows.
The decoder rejected valid default TOON output because it required a leading RS byte.
The previous standard report had 324 incorrect rows and 262 unsupported rows.

The repaired harness accepts default TOON streams and verifies every decoded
value against the ordered jq results. It rejects extra, missing, reordered,
and malformed output. Capture limits, nonzero exits, and resource failures
cannot produce valid timing summaries.

The catalog enables the verified tq adapters across JSON, YAML, and TOON.
The yq CSV adapter now matches jq string quoting, and CSV/TSV adapters emit an
empty field for null magnitude while preserving zero. All 48 CSV/TSV rows pass.

The [comparison pages](comparison/index.md) contain generated observations.
The [catalog](../../benchmarks/cases/workloads.jsonl) defines workload commands
and adapter eligibility. Each excluded adapter has a capability reason.

The current review reruns the full 39-workload catalog on Linux using only
`tq-bench` native workload measurements. It uses jq 1.8.1, yq 4.53.2, and the
recorded release tq executable on the frozen corpus. The host has an AMD
Ryzen 7 7700 8-Core Processor, 16 logical CPUs, 61.9 GiB RAM, and x86_64 Linux
kernel `7.2.0-ogc4.1.fc44.x86_64`. No macOS workload results are included.

Inputs and captures are prepared before an isolated worker launches each
target directly. Wall time ends at native exit observation and excludes worker
startup, request delivery, and cleanup. Native waiting supplies user/system
CPU and lifetime peak RSS, including pre-exec effects and waited descendants.
The measured 2.5 MiB residual RSS floor is retained, not subtracted. Primary
measurements have no recurring RSS sampler. Separate instrumented repetitions
enforce selected process-group limits and are not pooled into primary results.
Timing controls report excess duration above their requested interval. This
includes startup and scheduling, not just exit-observation delay, and is not a
timer-error bound. Comparisons stay within the same host and OS using the
same measurement method and concessions; repeated samples retain dispersion.

The standard report has 10,579 primary samples and 180 instrumented samples.
Smoke adds 540 primary samples; rapid adds 27 primary and three instrumented
samples. Every sample has positive native peak RSS. Non-empty output always
has a first-output observation, falling back to recorded completion when a
short-lived target exits before the metadata poll.

| Suite | Rows | Timed | Incorrect | Unsupported | Resource limit |
| --- | ---: | ---: | ---: | ---: | ---: |
| Rapid | 30 | 27 | 0 | 3 | 0 |
| Full standard | 846 | 703 | 0 | 134 | 9 |
| Additional smoke workloads | 18 | 18 | 0 | 0 | 0 |

The standard suite covers 36 workloads. Three additional smoke-only workloads
complete the 39-workload catalog. All 18 additional rows pass.

The rapid exclusions are the two yq event-stream adapters and tq event streaming
from YAML. Of the 134 standard exclusions, 130 belong to yq. The remaining four
are tq YAML event-stream rows, one per dataset.

The nine resource-limit rows are classified by the harness as `Resource` and
exit with status 5. They cover all three tq input formats for these combinations:

1. Object construction on `usgs-all-month`.
2. String reduction on `usgs-all-month`.
3. String reduction on `usgs-all-week`.

The standard command returns status 1 because these resource-limit observations
remain visible. Its report contains no incorrect rows, but it does not claim
every row passed.

The file `comparison-x86-64-linux.md` is a historical jq manual compatibility
report. It is not a performance report and is outside this renderer.

Targeted renderer and harness tests cover output contracts, failed rows, missing
RSS, captured summary metrics, and authored-text preservation. The accepted
Linux reports are rendered in [comparison pages](comparison/index.md), with
standard rendered last so the index shows the full standard review. The smoke
pages cover the three additional workloads in the 39-workload catalog.
