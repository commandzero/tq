---
type: Report
title: "Enumerating paths"
description: "Measures listing all paths inside the metadata object."
workload: benchmark.issue5-paths
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Enumerating paths

## What this measures

The query enumerates `paths` below `.metadata` and counts them. It traverses
the metadata tree and materializes the path list before producing its length.

## Why it matters

Path inventories help schema tools, redactors, and diagnostics understand an
object without knowing its fields ahead of time.

## Input and output

The input is the metadata object from a natural snapshot. The output is one
integer containing the number of discovered paths, not the path arrays.

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

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq paths expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 2.9 ms | - | - | 3.5 ms | 4.3 ms | 4.3 ms |
| Wall time dispersion | 0.1 ms / 3.5 ms / 2.6-3.5 ms | - | - | 0.1 ms / 3.7 ms / 3.2-3.7 ms | 0.1 ms / 4.8 ms / 4.0-4.8 ms | 0.2 ms / 4.5 ms / 4.0-4.5 ms |
| First output | 2.8 ms | - | - | 3.2 ms | 4.1 ms | 4.0 ms |
| First output dispersion | 0.1 ms / 3.3 ms / 2.6-3.3 ms | - | - | 0.1 ms / 3.4 ms / 3.1-3.4 ms | 0.1 ms / 4.5 ms / 3.7-4.5 ms | 0.1 ms / 4.2 ms / 3.7-4.2 ms |
| User CPU | 1.7 ms | - | - | 1.6 ms | 2.6 ms | 2.9 ms |
| System CPU | 1.1 ms | - | - | 1.7 ms | 1.5 ms | 1.0 ms |
| Peak RSS | 4.3 MiB | - | - | 9.0 MiB | 9.5 MiB | 8.9 MiB |
| Logical throughput | 66655.2 records/s | - | - | 55940.0 records/s | 44664.4 records/s | 45205.6 records/s |
| Physical throughput | 45.6 MiB/s | - | - | 38.3 MiB/s | 30.5 MiB/s | 36.6 MiB/s |
| Output bytes | 2.0 B | - | - | 2.0 B | 2.0 B | 2.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq paths expression. | yq rejects the catalog jq paths expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq paths expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | - | - | 1.2 ms | 1.2 ms | 1.2 ms |
| Wall time dispersion | 0.0 ms / 1.7 ms / 1.3-1.7 ms | - | - | 0.1 ms / 1.5 ms / 1.1-1.6 ms | 0.1 ms / 1.5 ms / 1.1-1.6 ms | 0.0 ms / 1.5 ms / 1.2-1.5 ms |
| First output | 1.4 ms | - | - | 1.0 ms | 1.0 ms | 1.0 ms |
| First output dispersion | 0.0 ms / 1.5 ms / 1.2-1.6 ms | - | - | 0.0 ms / 1.3 ms / 1.0-1.4 ms | 0.0 ms / 1.3 ms / 1.0-1.4 ms | 0.0 ms / 1.4 ms / 1.0-1.4 ms |
| User CPU | 0.7 ms | - | - | 0.0 ms | 0.2 ms | 0.0 ms |
| System CPU | 0.8 ms | - | - | 1.1 ms | 1.0 ms | 1.1 ms |
| Peak RSS | 4.1 MiB | - | - | 7.9 MiB | 8.3 MiB | 7.9 MiB |
| Logical throughput | 1336.9 records/s | - | - | 1655.6 records/s | 1672.9 records/s | 1700.7 records/s |
| Physical throughput | 1.1 MiB/s | - | - | 1.4 MiB/s | 1.4 MiB/s | 1.7 MiB/s |
| Output bytes | 2.0 B | - | - | 2.0 B | 2.0 B | 2.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq paths expression. | yq rejects the catalog jq paths expression. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq paths expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 94.1 ms | - | - | 121.5 ms | 163.5 ms | 139.5 ms |
| Wall time dispersion | 0.8 ms / 96.7 ms / 93.0-96.7 ms | - | - | 0.4 ms / 125.5 ms / 120.6-125.5 ms | 1.4 ms / 170.2 ms / 161.7-170.2 ms | 0.5 ms / 142.0 ms / 138.1-142.0 ms |
| First output | 90.9 ms | - | - | 117.9 ms | 159.0 ms | 135.7 ms |
| First output dispersion | 0.7 ms / 93.3 ms / 89.7-93.3 ms | - | - | 0.3 ms / 121.7 ms / 117.2-121.7 ms | 1.3 ms / 164.8 ms / 156.8-164.8 ms | 0.4 ms / 137.8 ms / 134.7-137.8 ms |
| User CPU | 73.2 ms | - | - | 93.5 ms | 134.2 ms | 110.6 ms |
| System CPU | 20.5 ms | - | - | 28.5 ms | 29.0 ms | 29.1 ms |
| Peak RSS | 60.3 MiB | - | - | 69.3 MiB | 89.5 MiB | 78.1 MiB |
| Logical throughput | 119832.9 records/s | - | - | 92819.9 records/s | 68971.6 records/s | 80829.1 records/s |
| Physical throughput | 81.2 MiB/s | - | - | 62.9 MiB/s | 46.7 MiB/s | 64.9 MiB/s |
| Output bytes | 2.0 B | - | - | 2.0 B | 2.0 B | 2.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq paths expression. | yq rejects the catalog jq paths expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq paths expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 19.8 ms | - | - | 25.4 ms | 32.3 ms | 28.5 ms |
| Wall time dispersion | 0.3 ms / 20.4 ms / 18.5-20.4 ms | - | - | 0.9 ms / 26.6 ms / 24.1-26.6 ms | 0.7 ms / 33.0 ms / 29.8-33.0 ms | 0.5 ms / 30.2 ms / 27.7-30.2 ms |
| First output | 19.1 ms | - | - | 24.4 ms | 31.3 ms | 27.5 ms |
| First output dispersion | 0.2 ms / 19.6 ms / 18.1-19.6 ms | - | - | 0.8 ms / 25.5 ms / 23.3-25.5 ms | 0.6 ms / 32.0 ms / 28.8-32.0 ms | 0.5 ms / 29.2 ms / 26.9-29.2 ms |
| User CPU | 13.9 ms | - | - | 20.0 ms | 24.4 ms | 22.2 ms |
| System CPU | 5.7 ms | - | - | 5.4 ms | 7.9 ms | 5.9 ms |
| Peak RSS | 14.7 MiB | - | - | 20.1 MiB | 24.2 MiB | 21.8 MiB |
| Logical throughput | 112564.2 records/s | - | - | 87589.8 records/s | 68911.1 records/s | 78041.4 records/s |
| Physical throughput | 76.3 MiB/s | - | - | 59.4 MiB/s | 46.6 MiB/s | 62.6 MiB/s |
| Output bytes | 2.0 B | - | - | 2.0 B | 2.0 B | 2.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq paths expression. | yq rejects the catalog jq paths expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
