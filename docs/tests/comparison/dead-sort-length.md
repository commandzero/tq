---
type: Report
title: "Sort before counting"
description: "Measures work whose sorted intermediate does not affect the final count."
workload: benchmark.dead-sort-length
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Sort before counting

## What this measures

The query collects the `release` values, sorts them, and then asks for the
length. The final count does not depend on the order, so the case exposes the
cost of doing unnecessary intermediate work.

## Why it matters

Real queries sometimes carry an expensive step that a later operation does not
use. This is a useful stress case for planning and optimization decisions.

## Input and output

The input is a natural feature snapshot. The output is one integer, the number
of collected values. It contains no sorted values.

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
| Wall time | 3.0 ms | 8.8 ms | 17.6 ms | 3.6 ms | 4.2 ms | 3.6 ms |
| Wall time dispersion | 0.1 ms / 3.3 ms / 2.6-3.3 ms | 0.3 ms / 9.3 ms / 8.3-9.3 ms | 0.1 ms / 18.6 ms / 17.2-18.6 ms | 0.1 ms / 4.2 ms / 3.4-4.2 ms | 0.2 ms / 4.8 ms / 3.8-4.8 ms | 0.2 ms / 3.9 ms / 3.3-3.9 ms |
| First output | 2.9 ms | 8.7 ms | 17.6 ms | 3.5 ms | 4.0 ms | 3.4 ms |
| First output dispersion | 0.1 ms / 3.1 ms / 2.6-3.1 ms | 0.3 ms / 9.3 ms / 8.2-9.3 ms | 0.2 ms / 18.4 ms / 17.1-18.4 ms | 0.1 ms / 3.9 ms / 3.2-3.9 ms | 0.2 ms / 4.5 ms / 3.7-4.5 ms | 0.2 ms / 3.7 ms / 3.1-3.7 ms |
| User CPU | 1.6 ms | 7.9 ms | 15.8 ms | 2.3 ms | 2.4 ms | 2.5 ms |
| System CPU | 1.3 ms | 5.0 ms | 9.9 ms | 1.1 ms | 1.9 ms | 1.0 ms |
| Peak RSS | 4.3 MiB | 18.8 MiB | 33.4 MiB | 8.2 MiB | 9.7 MiB | 7.8 MiB |
| Logical throughput | 64323.6 records/s | 22095.7 records/s | 10995.9 records/s | 53282.1 records/s | 46047.9 records/s | 54548.0 records/s |
| Physical throughput | 44.0 MiB/s | 15.1 MiB/s | 7.5 MiB/s | 36.5 MiB/s | 31.5 MiB/s | 44.2 MiB/s |
| Output bytes | 4.0 B | 4.0 B | 4.0 B | 4.0 B | 4.0 B | 4.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.3 ms | 3.1 ms | 3.0 ms | 1.3 ms | 1.3 ms | 1.3 ms |
| Wall time dispersion | 0.0 ms / 1.7 ms / 1.3-1.8 ms | 0.1 ms / 3.2 ms / 2.9-3.4 ms | 0.1 ms / 3.2 ms / 2.7-3.4 ms | 0.0 ms / 1.5 ms / 1.2-1.5 ms | 0.1 ms / 1.5 ms / 1.1-1.6 ms | 0.1 ms / 1.5 ms / 1.1-1.5 ms |
| First output | 1.3 ms | 2.6 ms | 2.6 ms | 1.0 ms | 1.0 ms | 1.0 ms |
| First output dispersion | 0.0 ms / 1.6 ms / 1.2-1.7 ms | 0.0 ms / 2.9 ms / 2.4-2.9 ms | 0.0 ms / 3.0 ms / 2.4-3.0 ms | 0.0 ms / 1.3 ms / 1.0-1.3 ms | 0.0 ms / 1.4 ms / 1.0-1.4 ms | 0.0 ms / 1.2 ms / 1.0-1.4 ms |
| User CPU | 0.6 ms | 1.0 ms | 1.0 ms | 0.6 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.8 ms | 2.0 ms | 2.0 ms | 0.7 ms | 1.1 ms | 1.1 ms |
| Peak RSS | 4.0 MiB | 11.9 MiB | 12.0 MiB | 8.3 MiB | 8.4 MiB | 7.8 MiB |
| Logical throughput | 1483.1 records/s | 650.5 records/s | 658.3 records/s | 1495.9 records/s | 1506.6 records/s | 1555.2 records/s |
| Physical throughput | 1.3 MiB/s | 0.6 MiB/s | 0.6 MiB/s | 1.3 MiB/s | 1.3 MiB/s | 1.5 MiB/s |
| Output bytes | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 99.3 ms | 304.0 ms | 957.7 ms | 125.7 ms | 171.4 ms | 117.4 ms |
| Wall time dispersion | 0.7 ms / 101.8 ms / 98.1-101.8 ms | 5.9 ms / 315.2 ms / 294.3-315.2 ms | 8.4 ms / 978.0 ms / 866.1-978.0 ms | 0.4 ms / 130.3 ms / 124.2-130.3 ms | 1.9 ms / 176.5 ms / 167.3-176.5 ms | 1.0 ms / 123.4 ms / 115.9-123.4 ms |
| First output | 95.7 ms | 283.4 ms | 885.7 ms | 125.5 ms | 166.4 ms | 117.3 ms |
| First output dispersion | 0.6 ms / 98.3 ms / 94.9-98.3 ms | 3.1 ms / 294.2 ms / 276.3-294.2 ms | 9.2 ms / 905.4 ms / 809.1-905.4 ms | 0.4 ms / 130.0 ms / 124.1-130.0 ms | 1.4 ms / 171.5 ms / 163.5-171.5 ms | 0.9 ms / 123.3 ms / 115.7-123.3 ms |
| User CPU | 74.8 ms | 345.2 ms | 1347.9 ms | 123.0 ms | 140.8 ms | 114.7 ms |
| System CPU | 24.5 ms | 120.8 ms | 474.5 ms | 2.0 ms | 30.4 ms | 2.5 ms |
| Peak RSS | 60.5 MiB | 331.4 MiB | 1296.0 MiB | 8.7 MiB | 90.1 MiB | 8.1 MiB |
| Logical throughput | 113523.3 records/s | 37087.8 records/s | 11771.6 records/s | 89719.7 records/s | 65759.1 records/s | 96014.3 records/s |
| Physical throughput | 77.0 MiB/s | 25.1 MiB/s | 8.0 MiB/s | 60.8 MiB/s | 44.5 MiB/s | 77.1 MiB/s |
| Output bytes | 6.0 B | 6.0 B | 6.0 B | 6.0 B | 6.0 B | 6.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 20.1 ms | 63.0 ms | 173.3 ms | 26.6 ms | 32.8 ms | 24.7 ms |
| Wall time dispersion | 0.9 ms / 21.6 ms / 18.7-21.6 ms | 1.1 ms / 64.7 ms / 60.8-64.7 ms | 1.6 ms / 186.2 ms / 161.4-186.2 ms | 0.3 ms / 27.4 ms / 26.2-27.4 ms | 0.5 ms / 33.6 ms / 31.4-33.6 ms | 0.4 ms / 26.5 ms / 23.9-26.5 ms |
| First output | 19.5 ms | 58.4 ms | 161.4 ms | 26.4 ms | 31.8 ms | 24.6 ms |
| First output dispersion | 0.9 ms / 20.6 ms / 18.2-20.6 ms | 0.7 ms / 60.2 ms / 56.8-60.2 ms | 1.9 ms / 171.2 ms / 151.0-171.2 ms | 0.3 ms / 27.1 ms / 25.9-27.1 ms | 0.5 ms / 32.4 ms / 30.5-32.4 ms | 0.4 ms / 26.1 ms / 23.8-26.1 ms |
| User CPU | 15.7 ms | 61.6 ms | 203.4 ms | 25.3 ms | 26.3 ms | 23.1 ms |
| System CPU | 4.2 ms | 27.8 ms | 90.9 ms | 1.0 ms | 6.6 ms | 1.0 ms |
| Peak RSS | 14.5 MiB | 76.1 MiB | 235.3 MiB | 8.4 MiB | 24.3 MiB | 7.9 MiB |
| Logical throughput | 110509.6 records/s | 35294.8 records/s | 12842.5 records/s | 83495.9 records/s | 67878.8 records/s | 90053.6 records/s |
| Physical throughput | 74.9 MiB/s | 23.9 MiB/s | 8.7 MiB/s | 56.6 MiB/s | 45.9 MiB/s | 72.3 MiB/s |
| Output bytes | 5.0 B | 5.0 B | 5.0 B | 5.0 B | 5.0 B | 5.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
