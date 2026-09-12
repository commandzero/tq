---
type: Report
title: "Blocking sort"
description: "Measures collecting and sorting all feature magnitudes."
workload: benchmark.blocking-sort
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Blocking sort

## What this measures

The query collects `.properties.mag` from every feature and applies `sort`. It
must consume the collection before it can emit the sorted array.

## Why it matters

Ranking and threshold preparation are common jobs where input size affects both
memory and time. This is a clear contrast with streaming projection.

## Input and output

The input is a natural feature snapshot. The output is one array of magnitudes
in ascending order, including the collected values rather than the original
feature objects.

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

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 3.0 ms | 8.9 ms | 17.4 ms | 4.3 ms | 4.4 ms | 4.0 ms |
| Wall time dispersion | 0.1 ms / 3.2 ms / 2.8-3.2 ms | 0.3 ms / 10.3 ms / 8.6-10.3 ms | 0.4 ms / 18.7 ms / 16.2-18.7 ms | 0.1 ms / 4.7 ms / 4.0-4.7 ms | 0.1 ms / 4.5 ms / 4.2-4.5 ms | 0.1 ms / 4.8 ms / 3.8-4.8 ms |
| First output | 2.9 ms | 8.9 ms | 17.3 ms | 4.2 ms | 4.2 ms | 3.9 ms |
| First output dispersion | 0.2 ms / 3.1 ms / 2.6-3.1 ms | 0.2 ms / 10.1 ms / 8.6-10.1 ms | 0.3 ms / 18.7 ms / 16.2-18.7 ms | 0.2 ms / 4.7 ms / 3.9-4.7 ms | 0.0 ms / 4.4 ms / 4.0-4.4 ms | 0.0 ms / 4.7 ms / 3.8-4.7 ms |
| User CPU | 1.9 ms | 7.6 ms | 16.3 ms | 3.9 ms | 3.0 ms | 4.2 ms |
| System CPU | 0.9 ms | 4.9 ms | 9.6 ms | 1.4 ms | 1.3 ms | 1.3 ms |
| Peak RSS | 4.4 MiB | 18.8 MiB | 32.4 MiB | 8.4 MiB | 9.8 MiB | 7.8 MiB |
| Logical throughput | 63679.6 records/s | 21700.2 records/s | 11177.4 records/s | 44618.2 records/s | 43792.3 records/s | 48258.7 records/s |
| Physical throughput | 43.6 MiB/s | 14.9 MiB/s | 7.6 MiB/s | 30.5 MiB/s | 29.9 MiB/s | 39.1 MiB/s |
| Output bytes | 1424.0 B | 841.0 B | 841.0 B | 846.0 B | 846.0 B | 846.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | 3.1 ms | 3.1 ms | 1.5 ms | 1.2 ms | 1.5 ms |
| Wall time dispersion | 0.0 ms / 1.7 ms / 1.3-1.7 ms | 0.0 ms / 3.4 ms / 2.9-3.5 ms | 0.1 ms / 3.4 ms / 2.6-3.5 ms | 0.0 ms / 1.8 ms / 1.5-1.8 ms | 0.0 ms / 1.4 ms / 1.1-1.5 ms | 0.0 ms / 1.8 ms / 1.4-1.9 ms |
| First output | 1.4 ms | 2.6 ms | 2.7 ms | 1.5 ms | 1.0 ms | 1.5 ms |
| First output dispersion | 0.0 ms / 1.5 ms / 1.2-1.7 ms | 0.1 ms / 2.9 ms / 2.4-3.2 ms | 0.1 ms / 3.0 ms / 2.6-3.1 ms | 0.0 ms / 1.8 ms / 1.3-1.8 ms | 0.0 ms / 1.2 ms / 1.0-1.2 ms | 0.0 ms / 1.7 ms / 1.3-1.8 ms |
| User CPU | 0.7 ms | 1.1 ms | 1.9 ms | 0.0 ms | 0.0 ms | 0.9 ms |
| System CPU | 0.7 ms | 2.1 ms | 1.5 ms | 2.4 ms | 1.1 ms | 1.7 ms |
| Peak RSS | 4.1 MiB | 11.9 MiB | 12.1 MiB | 8.5 MiB | 8.4 MiB | 7.9 MiB |
| Logical throughput | 1339.6 records/s | 652.6 records/s | 651.4 records/s | 1326.7 records/s | 1695.6 records/s | 1334.7 records/s |
| Physical throughput | 1.1 MiB/s | 0.6 MiB/s | 0.6 MiB/s | 1.1 MiB/s | 1.4 MiB/s | 1.3 MiB/s |
| Output bytes | 19.0 B | 12.0 B | 12.0 B | 15.0 B | 15.0 B | 15.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 106.0 ms | 320.5 ms | 932.5 ms | 146.2 ms | 180.8 ms | 139.8 ms |
| Wall time dispersion | 0.5 ms / 108.1 ms / 103.3-108.1 ms | 6.4 ms / 333.6 ms / 313.5-333.6 ms | 36.9 ms / 997.6 ms / 885.7-997.6 ms | 0.5 ms / 149.6 ms / 145.3-149.6 ms | 1.7 ms / 188.5 ms / 177.9-188.5 ms | 0.7 ms / 141.5 ms / 139.0-141.5 ms |
| First output | 101.1 ms | 301.2 ms | 865.9 ms | 146.1 ms | 176.4 ms | 139.8 ms |
| First output dispersion | 0.7 ms / 103.6 ms / 98.7-103.6 ms | 6.8 ms / 311.8 ms / 293.0-311.8 ms | 33.2 ms / 925.8 ms / 827.7-925.8 ms | 0.5 ms / 149.5 ms / 145.3-149.5 ms | 1.3 ms / 183.5 ms / 173.8-183.5 ms | 0.6 ms / 141.3 ms / 138.9-141.3 ms |
| User CPU | 79.9 ms | 361.0 ms | 1342.8 ms | 143.5 ms | 148.0 ms | 137.3 ms |
| System CPU | 25.3 ms | 123.8 ms | 435.2 ms | 3.5 ms | 33.6 ms | 3.6 ms |
| Peak RSS | 60.9 MiB | 332.9 MiB | 1303.5 MiB | 9.3 MiB | 90.7 MiB | 8.6 MiB |
| Logical throughput | 106323.4 records/s | 35173.2 records/s | 12089.5 records/s | 77107.7 records/s | 62369.5 records/s | 80645.8 records/s |
| Physical throughput | 72.1 MiB/s | 23.8 MiB/s | 8.2 MiB/s | 52.3 MiB/s | 42.2 MiB/s | 64.7 MiB/s |
| Output bytes | 84626.0 B | 50803.0 B | 50803.0 B | 50810.0 B | 50810.0 B | 50810.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 20.7 ms | 64.4 ms | 181.8 ms | 31.1 ms | 35.6 ms | 29.5 ms |
| Wall time dispersion | 0.2 ms / 21.1 ms / 19.2-21.1 ms | 0.6 ms / 66.4 ms / 62.7-66.4 ms | 5.7 ms / 187.8 ms / 164.7-187.8 ms | 0.9 ms / 34.6 ms / 30.1-34.6 ms | 0.3 ms / 36.0 ms / 34.0-36.0 ms | 0.4 ms / 30.3 ms / 28.8-30.3 ms |
| First output | 19.8 ms | 60.1 ms | 167.2 ms | 31.0 ms | 34.5 ms | 29.5 ms |
| First output dispersion | 0.2 ms / 20.3 ms / 18.6-20.3 ms | 0.8 ms / 61.9 ms / 58.8-61.9 ms | 5.9 ms / 173.2 ms / 154.2-173.2 ms | 0.8 ms / 34.6 ms / 30.1-34.6 ms | 0.5 ms / 35.1 ms / 33.1-35.1 ms | 0.5 ms / 30.3 ms / 28.8-30.3 ms |
| User CPU | 17.0 ms | 64.1 ms | 208.5 ms | 30.0 ms | 26.3 ms | 29.2 ms |
| System CPU | 3.5 ms | 29.7 ms | 86.2 ms | 2.5 ms | 8.5 ms | 1.6 ms |
| Peak RSS | 14.8 MiB | 74.6 MiB | 234.4 MiB | 8.7 MiB | 24.6 MiB | 8.1 MiB |
| Logical throughput | 107745.6 records/s | 34544.9 records/s | 12237.4 records/s | 71602.1 records/s | 62572.9 records/s | 75480.0 records/s |
| Physical throughput | 73.0 MiB/s | 23.4 MiB/s | 8.3 MiB/s | 48.5 MiB/s | 42.3 MiB/s | 60.6 MiB/s |
| Output bytes | 16581.0 B | 9905.0 B | 9905.0 B | 9911.0 B | 9911.0 B | 9911.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
