---
type: Review
title: Benchmark harness repair
description: Tasks and verification for correctness checks in the rapid, standard, and smoke benchmark campaigns.
status: stable
generated: { by: codex/gpt-6, at: 2026-09-10T22:20:03Z }
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
7. [x] Run the accepted Linux rapid, standard, and smoke review on Ironhide with the final reports.
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

An accepted campaign must first prove that the platform collector returns numeric
RSS for the selected time implementation, process-group inspection, and one
measured child. If that preflight fails, abandon the campaign immediately and
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

The earlier macOS captures used freshly built tq, jq 1.8.1, and yq 4.53.2 on the
frozen corpus. Each timed row had one measured sample, but every measured row
lacked authoritative RSS. They remain historical correctness and execution
evidence only; they are not an accepted performance benchmark and must not
support memory or speed findings. The accepted review is the Linux run below.

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
