---
type: Report
title: "Discarding a stream"
description: "Measures parsing and stream control when a query emits no values."
workload: benchmark.parse-discard
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Discarding a stream

## What this measures

The `empty` query reads each natural input and emits nothing. The workload
isolates input handling and the cost of a filter that discards its result.

## Why it matters

Log and validation pipelines often reject or discard records early. This case
shows the cost of doing that without paying to serialize an output document.

## Input and output

The input is a natural JSON, YAML, or TOON snapshot at each catalog size. The
output is an empty value sequence, not an empty array. That distinction keeps
parsing cost separate from output construction.

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

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 2.8 ms | 8.0 ms | 17.4 ms | 3.3 ms | 4.0 ms | 3.7 ms |
| Wall time dispersion | 0.1 ms / 3.1 ms / 2.6-3.1 ms | 0.1 ms / 8.6 ms / 7.8-8.6 ms | 0.3 ms / 17.9 ms / 16.4-17.9 ms | 0.2 ms / 3.6 ms / 2.9-3.6 ms | 0.0 ms / 4.0 ms / 3.7-4.0 ms | 0.2 ms / 4.1 ms / 3.4-4.1 ms |
| First output | - | - | - | - | - | - |
| First output dispersion | - | - | - | - | - | - |
| User CPU | 1.8 ms | 6.8 ms | 16.8 ms | 2.1 ms | 2.8 ms | 2.4 ms |
| System CPU | 0.9 ms | 4.1 ms | 9.0 ms | 1.0 ms | 1.0 ms | 1.1 ms |
| Peak RSS | 4.3 MiB | 17.6 MiB | 32.1 MiB | 8.4 MiB | 8.9 MiB | 8.2 MiB |
| Logical throughput | 70162.7 records/s | 24248.5 records/s | 11179.0 records/s | 59673.9 records/s | 48252.7 records/s | 51788.6 records/s |
| Physical throughput | 48.0 MiB/s | 16.6 MiB/s | 7.6 MiB/s | 40.9 MiB/s | 33.0 MiB/s | 41.9 MiB/s |
| Output bytes | 0.0 B | 0.0 B | 0.0 B | 0.0 B | 0.0 B | 0.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | 2.8 ms | 2.9 ms | 1.0 ms | 1.2 ms | 1.1 ms |
| Wall time dispersion | 0.1 ms / 1.7 ms / 1.3-1.8 ms | 0.2 ms / 3.1 ms / 2.4-3.1 ms | 0.1 ms / 3.2 ms / 2.7-3.4 ms | 0.0 ms / 1.5 ms / 1.0-1.5 ms | 0.1 ms / 1.5 ms / 1.0-1.5 ms | 0.1 ms / 1.3 ms / 1.0-1.4 ms |
| First output | - | - | - | - | - | - |
| First output dispersion | - | - | - | - | - | - |
| User CPU | 1.3 ms | 0.9 ms | 1.0 ms | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.0 ms | 2.0 ms | 2.0 ms | 1.0 ms | 1.0 ms | 0.9 ms |
| Peak RSS | 4.1 MiB | 10.9 MiB | 11.0 MiB | 7.2 MiB | 7.6 MiB | 7.2 MiB |
| Logical throughput | 1336.0 records/s | 726.6 records/s | 692.6 records/s | 1942.7 records/s | 1716.0 records/s | 1822.3 records/s |
| Physical throughput | 1.1 MiB/s | 0.6 MiB/s | 0.6 MiB/s | 1.6 MiB/s | 1.5 MiB/s | 1.8 MiB/s |
| Output bytes | 0.0 B | 0.0 B | 0.0 B | 0.0 B | 0.0 B | 0.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 93.5 ms | 261.7 ms | 906.2 ms | 120.3 ms | 161.3 ms | 137.0 ms |
| Wall time dispersion | 0.7 ms / 95.5 ms / 92.6-95.5 ms | 1.6 ms / 265.9 ms / 258.1-265.9 ms | 24.3 ms / 935.6 ms / 838.1-935.6 ms | 1.1 ms / 122.9 ms / 118.1-122.9 ms | 1.2 ms / 163.3 ms / 158.7-163.3 ms | 0.9 ms / 138.5 ms / 135.6-138.5 ms |
| First output | - | - | - | - | - | - |
| First output dispersion | - | - | - | - | - | - |
| User CPU | 66.3 ms | 227.6 ms | 1258.6 ms | 93.7 ms | 130.3 ms | 110.0 ms |
| System CPU | 26.5 ms | 120.3 ms | 446.4 ms | 26.0 ms | 30.8 ms | 26.5 ms |
| Peak RSS | 60.3 MiB | 294.4 MiB | 1301.3 MiB | 68.6 MiB | 88.7 MiB | 77.4 MiB |
| Logical throughput | 120602.7 records/s | 43084.8 records/s | 12440.6 records/s | 93744.9 records/s | 69911.3 records/s | 82280.0 records/s |
| Physical throughput | 81.7 MiB/s | 29.2 MiB/s | 8.4 MiB/s | 63.5 MiB/s | 47.3 MiB/s | 66.1 MiB/s |
| Output bytes | 0.0 B | 0.0 B | 0.0 B | 0.0 B | 0.0 B | 0.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 19.3 ms | 55.5 ms | 170.8 ms | 24.6 ms | 31.9 ms | 28.3 ms |
| Wall time dispersion | 0.3 ms / 21.1 ms / 18.5-21.1 ms | 0.4 ms / 56.3 ms / 53.2-56.3 ms | 3.8 ms / 175.4 ms / 155.3-175.4 ms | 0.8 ms / 25.4 ms / 23.2-25.4 ms | 0.6 ms / 33.5 ms / 29.5-33.5 ms | 0.7 ms / 29.2 ms / 26.6-29.2 ms |
| First output | - | - | - | - | - | - |
| First output dispersion | - | - | - | - | - | - |
| User CPU | 15.1 ms | 48.4 ms | 199.7 ms | 18.4 ms | 26.2 ms | 22.3 ms |
| System CPU | 5.0 ms | 25.2 ms | 90.2 ms | 6.4 ms | 5.7 ms | 5.1 ms |
| Peak RSS | 14.6 MiB | 67.6 MiB | 235.0 MiB | 19.4 MiB | 23.4 MiB | 20.8 MiB |
| Logical throughput | 115180.5 records/s | 40096.6 records/s | 13030.0 records/s | 90351.7 records/s | 69774.4 records/s | 78519.3 records/s |
| Physical throughput | 78.0 MiB/s | 27.2 MiB/s | 8.8 MiB/s | 61.2 MiB/s | 47.2 MiB/s | 63.0 MiB/s |
| Output bytes | 0.0 B | 0.0 B | 0.0 B | 0.0 B | 0.0 B | 0.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
