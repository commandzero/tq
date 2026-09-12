---
type: Report
title: "User function calls"
description: "Measures passing a value through a user-defined filter function."
workload: benchmark.user-filter-call
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# User function calls

## What this measures

The query defines `magnitude(f)` and calls it for each feature's magnitude.
This isolates function definition, argument passing, and repeated invocation.

## Why it matters

Reusable filters make production queries easier to maintain. A function call
should keep the same stream behavior as the equivalent inline expression.

## Input and output

The input is a natural feature snapshot. The output is one magnitude per
feature in input order, emitted as a sequence after passing through the user
function.

## Results

<!-- benchmark-results:start -->
Last updated: 2026-09-11
Profile: `standard` | Status: `observed-failures`

Tools: `jq` (jq-1.8.1), `tq` (tq 0.1.0 (TOON v3; jq target 1.8.x; revision a4b4d916-worktree)), `yq` (yq (https://github.com/mikefarah/yq/) version v4.53.2)
Environment: `linux` / `x86_64`, AMD Ryzen 7 7700 8-Core Processor, 16 logical CPUs, 61.9 GiB RAM; kernel `Linux 7.2.0-ogc4.1.fc44.x86_64 #1 SMP PREEMPT_DYNAMIC Thu Aug 20 16:15:37 UTC 2026`; compiler profile `release-benchmark`

Peak RSS is authoritative only after the campaign RSS preflight passes. Missing or invalid values are `-`, never estimates. RSS collector provenance (outside measurement tables): `linux-wait4`.
Measurement method (outside measurement tables): `tq-bench` native measurement: RSS scope `wait4 child lifetime including pre exec waited descendants and threads`; residual RSS floor `2.5 MiB` retained, not subtracted; observed control excess `1.1 ms`; primary timing is sampler-free.

Compare columns with the same input format to isolate tool differences. Missing adapters have `-` measurement cells and `not recorded` outcome/detail cells.

Timing rows show numeric medians followed by compact MAD / p95 / range rows, with one decimal place and a unit in each measurement cell. CPU, throughput, and output cells use the captured summary values.

### usgs-all-day

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 2.8 ms | - | - | 4.8 ms | 5.5 ms | 5.3 ms |
| Wall time dispersion | 0.1 ms / 3.1 ms / 2.6-3.1 ms | - | - | 0.2 ms / 5.1 ms / 4.6-5.1 ms | 0.0 ms / 5.9 ms / 5.4-5.9 ms | 0.1 ms / 5.7 ms / 5.1-5.7 ms |
| First output | 2.7 ms | - | - | 4.5 ms | 5.2 ms | 5.1 ms |
| First output dispersion | 0.1 ms / 2.9 ms / 2.6-2.9 ms | - | - | 0.1 ms / 4.8 ms / 4.5-4.8 ms | 0.1 ms / 5.6 ms / 5.1-5.6 ms | 0.1 ms / 5.3 ms / 4.8-5.3 ms |
| User CPU | 1.6 ms | - | - | 2.6 ms | 3.4 ms | 2.7 ms |
| System CPU | 1.1 ms | - | - | 2.2 ms | 2.0 ms | 2.6 ms |
| Peak RSS | 4.4 MiB | - | - | 9.0 MiB | 9.6 MiB | 9.1 MiB |
| Logical throughput | 69897.3 records/s | - | - | 40095.1 records/s | 35482.4 records/s | 36770.3 records/s |
| Physical throughput | 47.9 MiB/s | - | - | 27.4 MiB/s | 24.3 MiB/s | 29.8 MiB/s |
| Output bytes | 839.0 B | - | - | 839.0 B | 839.0 B | 839.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | - | - | 1.3 ms | 1.3 ms | 1.3 ms |
| Wall time dispersion | 0.1 ms / 1.8 ms / 1.3-1.8 ms | - | - | 0.1 ms / 1.5 ms / 1.2-1.5 ms | 0.1 ms / 1.6 ms / 1.1-1.6 ms | 0.1 ms / 1.6 ms / 1.1-1.6 ms |
| First output | 1.3 ms | - | - | 1.0 ms | 1.0 ms | 1.0 ms |
| First output dispersion | 0.0 ms / 1.7 ms / 1.2-1.7 ms | - | - | 0.0 ms / 1.3 ms / 1.0-1.4 ms | 0.0 ms / 1.4 ms / 1.0-1.4 ms | 0.0 ms / 1.4 ms / 1.0-1.5 ms |
| User CPU | 0.7 ms | - | - | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.7 ms | - | - | 1.1 ms | 1.1 ms | 1.1 ms |
| Peak RSS | 4.1 MiB | - | - | 8.0 MiB | 8.4 MiB | 7.9 MiB |
| Logical throughput | 1343.6 records/s | - | - | 1531.4 records/s | 1516.3 records/s | 1559.5 records/s |
| Physical throughput | 1.1 MiB/s | - | - | 1.3 MiB/s | 1.3 MiB/s | 1.5 MiB/s |
| Output bytes | 10.0 B | - | - | 10.0 B | 10.0 B | 10.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 99.3 ms | - | - | 195.2 ms | 238.2 ms | 213.8 ms |
| Wall time dispersion | 1.4 ms / 103.8 ms / 97.6-103.8 ms | - | - | 2.2 ms / 201.2 ms / 192.1-201.2 ms | 2.1 ms / 241.7 ms / 232.2-241.7 ms | 1.7 ms / 223.4 ms / 210.4-223.4 ms |
| First output | 79.8 ms | - | - | 191.6 ms | 232.8 ms | 209.6 ms |
| First output dispersion | 0.8 ms / 82.8 ms / 78.2-82.8 ms | - | - | 2.2 ms / 195.9 ms / 188.7-195.9 ms | 1.9 ms / 236.0 ms / 227.8-236.0 ms | 1.7 ms / 218.5 ms / 206.8-218.5 ms |
| User CPU | 74.9 ms | - | - | 140.1 ms | 177.2 ms | 156.6 ms |
| System CPU | 23.9 ms | - | - | 54.2 ms | 61.4 ms | 57.6 ms |
| Peak RSS | 60.7 MiB | - | - | 69.2 MiB | 89.4 MiB | 78.1 MiB |
| Logical throughput | 113556.5 records/s | - | - | 57755.4 records/s | 47337.0 records/s | 52732.6 records/s |
| Physical throughput | 77.0 MiB/s | - | - | 39.1 MiB/s | 32.0 MiB/s | 42.3 MiB/s |
| Output bytes | 50801.0 B | - | - | 50801.0 B | 50801.0 B | 50801.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 20.4 ms | - | - | 39.8 ms | 47.1 ms | 42.8 ms |
| Wall time dispersion | 0.3 ms / 22.1 ms / 18.8-22.1 ms | - | - | 0.7 ms / 40.9 ms / 38.0-40.9 ms | 0.7 ms / 48.0 ms / 44.9-48.0 ms | 0.8 ms / 45.2 ms / 41.1-45.2 ms |
| First output | 17.5 ms | - | - | 38.9 ms | 46.0 ms | 42.0 ms |
| First output dispersion | 0.2 ms / 18.8 ms / 16.4-18.8 ms | - | - | 0.7 ms / 40.0 ms / 37.0-40.0 ms | 0.6 ms / 46.9 ms / 43.8-46.9 ms | 0.9 ms / 44.3 ms / 40.4-44.3 ms |
| User CPU | 13.9 ms | - | - | 27.9 ms | 34.2 ms | 28.9 ms |
| System CPU | 6.5 ms | - | - | 11.3 ms | 12.7 ms | 12.9 ms |
| Peak RSS | 14.7 MiB | - | - | 20.0 MiB | 24.4 MiB | 21.6 MiB |
| Logical throughput | 109154.2 records/s | - | - | 55939.0 records/s | 47289.6 records/s | 51931.4 records/s |
| Physical throughput | 74.0 MiB/s | - | - | 37.9 MiB/s | 32.0 MiB/s | 41.7 MiB/s |
| Output bytes | 9903.0 B | - | - | 9903.0 B | 9903.0 B | 9903.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
