---
type: Report
title: "User function mapping"
description: "Measures mapping a named user filter across a feature collection."
workload: benchmark.user-filter-map
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# User function mapping

## What this measures

The query defines a no-argument `magnitude` filter and applies it with `map`.
It combines user-function lookup with array construction over all features.

## Why it matters

Named mapping functions are common in reusable transformations. The case tests
the cost and semantics of invoking a filter once for every array member.

## Input and output

The input is a natural snapshot with a feature array. The output is one array
of magnitudes, in feature order, rather than a stream of separate numbers.

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
| Wall time | 2.9 ms | - | - | 3.6 ms | 4.3 ms | 3.9 ms |
| Wall time dispersion | 0.1 ms / 3.4 ms / 2.7-3.4 ms | - | - | 0.1 ms / 3.9 ms / 3.4-3.9 ms | 0.1 ms / 4.7 ms / 4.0-4.7 ms | 0.1 ms / 4.3 ms / 3.7-4.3 ms |
| First output | 2.8 ms | - | - | 3.4 ms | 4.0 ms | 3.6 ms |
| First output dispersion | 0.1 ms / 3.2 ms / 2.5-3.2 ms | - | - | 0.2 ms / 3.7 ms / 3.1-3.7 ms | 0.1 ms / 4.3 ms / 3.7-4.3 ms | 0.0 ms / 4.0 ms / 3.5-4.0 ms |
| User CPU | 2.0 ms | - | - | 2.7 ms | 2.6 ms | 2.8 ms |
| System CPU | 0.9 ms | - | - | 0.9 ms | 1.5 ms | 0.9 ms |
| Peak RSS | 4.4 MiB | - | - | 9.1 MiB | 9.7 MiB | 9.1 MiB |
| Logical throughput | 65863.2 records/s | - | - | 54571.0 records/s | 44726.2 records/s | 50161.6 records/s |
| Physical throughput | 45.1 MiB/s | - | - | 37.4 MiB/s | 30.6 MiB/s | 40.6 MiB/s |
| Output bytes | 1424.0 B | - | - | 846.0 B | 846.0 B | 846.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | - | - | 1.2 ms | 1.2 ms | 1.2 ms |
| Wall time dispersion | 0.2 ms / 1.7 ms / 1.3-1.7 ms | - | - | 0.0 ms / 1.5 ms / 1.2-1.5 ms | 0.0 ms / 1.5 ms / 1.2-1.5 ms | 0.0 ms / 1.5 ms / 1.1-1.5 ms |
| First output | 1.3 ms | - | - | 1.0 ms | 1.0 ms | 1.0 ms |
| First output dispersion | 0.1 ms / 1.6 ms / 1.2-1.7 ms | - | - | 0.0 ms / 1.4 ms / 1.0-1.4 ms | 0.0 ms / 1.3 ms / 1.0-1.4 ms | 0.0 ms / 1.4 ms / 1.0-1.4 ms |
| User CPU | 1.3 ms | - | - | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.0 ms | - | - | 1.1 ms | 1.1 ms | 1.1 ms |
| Peak RSS | 4.1 MiB | - | - | 8.0 MiB | 8.3 MiB | 7.9 MiB |
| Logical throughput | 1337.8 records/s | - | - | 1682.8 records/s | 1699.2 records/s | 1696.4 records/s |
| Physical throughput | 1.1 MiB/s | - | - | 1.4 MiB/s | 1.4 MiB/s | 1.6 MiB/s |
| Output bytes | 19.0 B | - | - | 15.0 B | 15.0 B | 15.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 101.2 ms | - | - | 135.3 ms | 178.7 ms | 153.7 ms |
| Wall time dispersion | 0.8 ms / 104.2 ms / 99.2-104.2 ms | - | - | 1.5 ms / 137.1 ms / 133.2-137.1 ms | 1.3 ms / 181.4 ms / 176.7-181.4 ms | 0.5 ms / 156.8 ms / 152.2-156.8 ms |
| First output | 96.8 ms | - | - | 131.6 ms | 174.0 ms | 149.7 ms |
| First output dispersion | 1.0 ms / 99.6 ms / 94.9-99.6 ms | - | - | 1.1 ms / 133.1 ms / 129.4-133.1 ms | 1.2 ms / 175.7 ms / 172.0-175.7 ms | 0.7 ms / 153.3 ms / 148.6-153.3 ms |
| User CPU | 76.9 ms | - | - | 108.0 ms | 144.1 ms | 125.3 ms |
| System CPU | 23.4 ms | - | - | 27.2 ms | 33.9 ms | 27.4 ms |
| Peak RSS | 60.5 MiB | - | - | 69.9 MiB | 90.0 MiB | 79.1 MiB |
| Logical throughput | 111453.3 records/s | - | - | 83300.1 records/s | 63101.9 records/s | 73345.7 records/s |
| Physical throughput | 75.5 MiB/s | - | - | 56.5 MiB/s | 42.7 MiB/s | 58.9 MiB/s |
| Output bytes | 84626.0 B | - | - | 50810.0 B | 50810.0 B | 50810.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 20.2 ms | - | - | 28.4 ms | 34.4 ms | 31.0 ms |
| Wall time dispersion | 0.3 ms / 21.6 ms / 19.3-21.6 ms | - | - | 0.8 ms / 29.4 ms / 25.9-29.4 ms | 0.6 ms / 35.6 ms / 33.1-35.6 ms | 0.3 ms / 32.6 ms / 29.5-32.6 ms |
| First output | 19.2 ms | - | - | 27.4 ms | 33.3 ms | 30.0 ms |
| First output dispersion | 0.3 ms / 20.8 ms / 18.4-20.8 ms | - | - | 0.8 ms / 28.4 ms / 25.1-28.4 ms | 0.6 ms / 34.5 ms / 31.8-34.5 ms | 0.4 ms / 31.7 ms / 28.7-31.7 ms |
| User CPU | 15.3 ms | - | - | 21.6 ms | 26.1 ms | 24.1 ms |
| System CPU | 5.1 ms | - | - | 6.0 ms | 8.3 ms | 6.4 ms |
| Peak RSS | 14.8 MiB | - | - | 20.2 MiB | 24.3 MiB | 21.8 MiB |
| Logical throughput | 110419.1 records/s | - | - | 78422.4 records/s | 64626.7 records/s | 71858.8 records/s |
| Physical throughput | 74.8 MiB/s | - | - | 53.1 MiB/s | 43.7 MiB/s | 57.7 MiB/s |
| Output bytes | 16581.0 B | - | - | 9911.0 B | 9911.0 B | 9911.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
