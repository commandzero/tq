---
type: Report
title: "User function sorting"
description: "Measures sorting with a named key filter and projecting IDs."
workload: benchmark.user-filter-sort-by
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# User function sorting

## What this measures

The query defines `magnitude`, uses it as the `sort_by` key, and maps the
ordered features to IDs. It exercises a user filter in a sorting callback.

## Why it matters

Sorting by a reusable key function is a practical pattern for reports and
ranking. It also combines callback overhead with a blocking operation.

## Input and output

The input is a natural feature snapshot. The output is one array of IDs ordered
by each feature's magnitude, not the sorted feature objects themselves.

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
| Wall time | 3.2 ms | - | - | 3.9 ms | 4.5 ms | 4.2 ms |
| Wall time dispersion | 0.1 ms / 3.4 ms / 2.8-3.4 ms | - | - | 0.1 ms / 4.2 ms / 3.7-4.2 ms | 0.1 ms / 5.1 ms / 4.3-5.1 ms | 0.1 ms / 4.6 ms / 4.1-4.6 ms |
| First output | 3.1 ms | - | - | 3.6 ms | 4.4 ms | 4.0 ms |
| First output dispersion | 0.2 ms / 3.3 ms / 2.7-3.3 ms | - | - | 0.1 ms / 3.9 ms / 3.5-3.9 ms | 0.1 ms / 4.8 ms / 4.0-4.8 ms | 0.1 ms / 4.3 ms / 3.9-4.3 ms |
| User CPU | 1.7 ms | - | - | 2.9 ms | 2.6 ms | 2.1 ms |
| System CPU | 1.2 ms | - | - | 1.0 ms | 2.1 ms | 2.0 ms |
| Peak RSS | 4.5 MiB | - | - | 9.2 MiB | 9.7 MiB | 9.0 MiB |
| Logical throughput | 60127.1 records/s | - | - | 49942.1 records/s | 43029.8 records/s | 46124.6 records/s |
| Physical throughput | 41.2 MiB/s | - | - | 34.2 MiB/s | 29.4 MiB/s | 37.3 MiB/s |
| Output bytes | 3291.0 B | - | - | 2325.0 B | 2325.0 B | 2325.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | - | - | 1.3 ms | 1.3 ms | 1.2 ms |
| Wall time dispersion | 0.0 ms / 1.7 ms / 1.3-1.8 ms | - | - | 0.1 ms / 1.5 ms / 1.2-1.7 ms | 0.1 ms / 1.7 ms / 1.2-1.7 ms | 0.0 ms / 1.5 ms / 1.1-1.5 ms |
| First output | 1.4 ms | - | - | 1.1 ms | 1.0 ms | 1.0 ms |
| First output dispersion | 0.0 ms / 1.5 ms / 1.2-1.7 ms | - | - | 0.1 ms / 1.3 ms / 1.0-1.4 ms | 0.0 ms / 1.4 ms / 1.0-1.5 ms | 0.0 ms / 1.2 ms / 1.0-1.2 ms |
| User CPU | 0.7 ms | - | - | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.7 ms | - | - | 1.1 ms | 1.1 ms | 1.1 ms |
| Peak RSS | 4.0 MiB | - | - | 8.1 MiB | 8.3 MiB | 7.9 MiB |
| Logical throughput | 1336.9 records/s | - | - | 1483.1 records/s | 1499.3 records/s | 1685.6 records/s |
| Physical throughput | 1.1 MiB/s | - | - | 1.3 MiB/s | 1.3 MiB/s | 1.6 MiB/s |
| Output bytes | 35.0 B | - | - | 27.0 B | 27.0 B | 27.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 118.1 ms | - | - | 156.0 ms | 199.7 ms | 174.2 ms |
| Wall time dispersion | 0.9 ms / 120.1 ms / 116.2-120.1 ms | - | - | 1.0 ms / 158.4 ms / 154.3-158.4 ms | 0.8 ms / 202.8 ms / 196.1-202.8 ms | 1.0 ms / 176.8 ms / 172.1-176.8 ms |
| First output | 112.8 ms | - | - | 135.2 ms | 178.7 ms | 152.3 ms |
| First output dispersion | 0.7 ms / 114.7 ms / 110.7-114.7 ms | - | - | 0.9 ms / 137.6 ms / 133.5-137.6 ms | 1.0 ms / 180.4 ms / 175.5-180.4 ms | 1.1 ms / 154.4 ms / 150.4-154.4 ms |
| User CPU | 91.8 ms | - | - | 126.5 ms | 162.4 ms | 143.6 ms |
| System CPU | 25.5 ms | - | - | 29.5 ms | 36.9 ms | 31.5 ms |
| Peak RSS | 61.5 MiB | - | - | 72.8 MiB | 93.1 MiB | 81.4 MiB |
| Logical throughput | 95485.3 records/s | - | - | 72281.7 records/s | 56462.7 records/s | 64705.3 records/s |
| Physical throughput | 64.7 MiB/s | - | - | 49.0 MiB/s | 38.2 MiB/s | 51.9 MiB/s |
| Output bytes | 189277.0 B | - | - | 132913.0 B | 132913.0 B | 132913.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 22.4 ms | - | - | 30.9 ms | 37.9 ms | 33.9 ms |
| Wall time dispersion | 0.4 ms / 23.8 ms / 21.0-23.8 ms | - | - | 0.2 ms / 31.2 ms / 30.4-31.2 ms | 0.5 ms / 38.5 ms / 36.0-38.5 ms | 0.5 ms / 34.9 ms / 32.7-34.9 ms |
| First output | 21.4 ms | - | - | 30.0 ms | 36.8 ms | 32.9 ms |
| First output dispersion | 0.4 ms / 22.8 ms / 20.1-22.8 ms | - | - | 0.2 ms / 30.3 ms / 29.5-30.3 ms | 0.4 ms / 37.2 ms / 35.1-37.2 ms | 0.6 ms / 34.0 ms / 31.6-34.0 ms |
| User CPU | 17.1 ms | - | - | 23.4 ms | 29.9 ms | 25.0 ms |
| System CPU | 5.5 ms | - | - | 7.0 ms | 7.5 ms | 7.7 ms |
| Peak RSS | 14.8 MiB | - | - | 20.9 MiB | 24.8 MiB | 22.4 MiB |
| Logical throughput | 99261.7 records/s | - | - | 72118.5 records/s | 58776.9 records/s | 65691.4 records/s |
| Physical throughput | 67.3 MiB/s | - | - | 48.9 MiB/s | 39.8 MiB/s | 52.7 MiB/s |
| Output bytes | 37825.0 B | - | - | 26705.0 B | 26705.0 B | 26705.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
