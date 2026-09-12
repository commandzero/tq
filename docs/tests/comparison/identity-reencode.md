---
type: Report
title: "Identity re-encoding"
description: "Measures reading and writing a complete natural document unchanged."
workload: benchmark.identity-reencode
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Identity re-encoding

## What this measures

The query is `.` on natural snapshots. Unlike the startup identity case, the
documents span the catalog's small, medium, and large tiers, exposing full
parse and output costs.

## Why it matters

Format conversion and pass-through filters are useful baselines for pipelines
that do not change the data. They show the cost of carrying a document through
the tool.

## Input and output

The input is a complete natural snapshot in the selected format. The output is
the same semantic document, including its structure and object members, not a
single field or a stream of projected values.

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
| Wall time | 5.0 ms | 17.0 ms | 25.1 ms | 7.2 ms | 4.8 ms | 6.9 ms |
| Wall time dispersion | 0.2 ms / 5.4 ms / 4.3-5.4 ms | 0.3 ms / 17.8 ms / 16.3-17.8 ms | 0.2 ms / 25.7 ms / 23.9-25.7 ms | 0.2 ms / 7.5 ms / 6.6-7.5 ms | 0.2 ms / 5.4 ms / 4.6-5.4 ms | 0.1 ms / 7.0 ms / 6.6-7.0 ms |
| First output | 2.8 ms | 16.9 ms | 24.9 ms | 6.9 ms | 4.0 ms | 6.7 ms |
| First output dispersion | 0.1 ms / 3.1 ms / 2.3-3.1 ms | 0.3 ms / 17.8 ms / 16.2-17.8 ms | 0.2 ms / 25.5 ms / 23.8-25.5 ms | 0.2 ms / 7.4 ms / 6.4-7.4 ms | 0.2 ms / 4.5 ms / 3.7-4.5 ms | 0.1 ms / 6.9 ms / 6.4-6.9 ms |
| User CPU | 3.6 ms | 15.6 ms | 26.6 ms | 5.5 ms | 3.7 ms | 4.6 ms |
| System CPU | 1.1 ms | 6.8 ms | 9.6 ms | 1.5 ms | 1.0 ms | 2.0 ms |
| Peak RSS | 4.4 MiB | 22.3 MiB | 33.7 MiB | 7.5 MiB | 9.1 MiB | 7.5 MiB |
| Logical throughput | 39140.5 records/s | 11439.7 records/s | 7741.0 records/s | 27055.3 records/s | 40207.3 records/s | 27955.9 records/s |
| Physical throughput | 26.8 MiB/s | 7.8 MiB/s | 5.3 MiB/s | 18.5 MiB/s | 27.5 MiB/s | 22.6 MiB/s |
| Output bytes | 212498.0 B | 139074.0 B | 139074.0 B | 164670.0 B | 164670.0 B | 164670.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | 3.2 ms | 3.2 ms | 1.4 ms | 1.3 ms | 1.3 ms |
| Wall time dispersion | 0.0 ms / 1.7 ms / 1.3-1.7 ms | 0.1 ms / 3.4 ms / 2.9-3.5 ms | 0.1 ms / 3.5 ms / 2.9-3.6 ms | 0.1 ms / 1.7 ms / 1.2-1.8 ms | 0.0 ms / 1.4 ms / 1.0-1.4 ms | 0.1 ms / 1.5 ms / 1.1-1.5 ms |
| First output | 1.4 ms | 2.7 ms | 2.7 ms | 1.2 ms | 1.0 ms | 1.1 ms |
| First output dispersion | 0.0 ms / 1.5 ms / 1.2-1.5 ms | 0.1 ms / 2.9 ms / 2.4-3.0 ms | 0.0 ms / 3.0 ms / 2.6-3.1 ms | 0.1 ms / 1.4 ms / 1.0-1.5 ms | 0.1 ms / 1.2 ms / 0.9-1.2 ms | 0.1 ms / 1.2 ms / 1.0-1.3 ms |
| User CPU | 0.7 ms | 1.5 ms | 2.1 ms | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.7 ms | 1.7 ms | 1.2 ms | 1.2 ms | 1.0 ms | 1.2 ms |
| Peak RSS | 4.1 MiB | 11.9 MiB | 12.1 MiB | 7.4 MiB | 7.8 MiB | 7.3 MiB |
| Logical throughput | 1324.5 records/s | 629.7 records/s | 621.6 records/s | 1463.6 records/s | 1485.3 records/s | 1515.2 records/s |
| Physical throughput | 1.1 MiB/s | 0.5 MiB/s | 0.5 MiB/s | 1.2 MiB/s | 1.3 MiB/s | 1.5 MiB/s |
| Output bytes | 2623.0 B | 1775.0 B | 1775.0 B | 2037.0 B | 2037.0 B | 2037.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 196.7 ms | 728.9 ms | 1318.5 ms | 533.6 ms | 210.9 ms | 527.2 ms |
| Wall time dispersion | 1.0 ms / 200.6 ms / 195.2-200.6 ms | 4.5 ms / 739.9 ms / 719.8-739.9 ms | 31.3 ms / 1356.9 ms / 1241.7-1356.9 ms | 1.8 ms / 546.0 ms / 531.0-546.0 ms | 1.4 ms / 212.8 ms / 206.1-212.8 ms | 1.8 ms / 535.9 ms / 524.6-535.9 ms |
| First output | 79.7 ms | 699.8 ms | 1249.2 ms | 529.8 ms | 144.8 ms | 523.2 ms |
| First output dispersion | 0.5 ms / 81.5 ms / 78.8-81.5 ms | 3.6 ms / 709.0 ms / 691.7-709.0 ms | 24.7 ms / 1277.1 ms / 1182.7-1277.1 ms | 1.7 ms / 542.1 ms / 527.4-542.1 ms | 1.0 ms / 146.2 ms / 141.0-146.2 ms | 1.8 ms / 531.9 ms / 520.7-531.9 ms |
| User CPU | 163.0 ms | 796.8 ms | 1848.7 ms | 272.5 ms | 175.8 ms | 267.3 ms |
| System CPU | 35.4 ms | 177.7 ms | 464.0 ms | 261.7 ms | 33.9 ms | 261.7 ms |
| Peak RSS | 64.9 MiB | 487.3 MiB | 1362.2 MiB | 15.4 MiB | 88.8 MiB | 15.2 MiB |
| Logical throughput | 57322.6 records/s | 15468.0 records/s | 8550.6 records/s | 21128.4 records/s | 53450.2 records/s | 21384.5 records/s |
| Physical throughput | 38.9 MiB/s | 10.5 MiB/s | 5.8 MiB/s | 14.3 MiB/s | 36.2 MiB/s | 17.2 MiB/s |
| Output bytes | 12263577.0 B | 8001913.0 B | 8001913.0 B | 9490993.0 B | 9490993.0 B | 9490993.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 40.6 ms | 149.8 ms | 248.7 ms | 61.9 ms | 40.0 ms | 60.5 ms |
| Wall time dispersion | 0.8 ms / 42.1 ms / 38.8-42.1 ms | 1.0 ms / 153.3 ms / 147.1-153.3 ms | 4.1 ms / 256.7 ms / 240.7-256.7 ms | 0.3 ms / 62.4 ms / 61.4-62.4 ms | 1.0 ms / 42.7 ms / 38.5-42.7 ms | 0.2 ms / 61.5 ms / 59.7-61.5 ms |
| First output | 17.4 ms | 142.8 ms | 236.4 ms | 61.6 ms | 29.1 ms | 60.2 ms |
| First output dispersion | 0.2 ms / 18.1 ms / 16.6-18.1 ms | 0.5 ms / 145.1 ms / 139.9-145.1 ms | 3.8 ms / 242.5 ms / 229.5-242.5 ms | 0.2 ms / 62.1 ms / 61.0-62.1 ms | 0.6 ms / 31.9 ms / 27.7-31.9 ms | 0.3 ms / 61.2 ms / 59.3-61.2 ms |
| User CPU | 33.2 ms | 168.7 ms | 325.2 ms | 48.2 ms | 32.4 ms | 46.6 ms |
| System CPU | 7.1 ms | 42.1 ms | 90.7 ms | 13.6 ms | 7.0 ms | 13.5 ms |
| Peak RSS | 15.4 MiB | 105.2 MiB | 237.2 MiB | 9.3 MiB | 23.5 MiB | 8.9 MiB |
| Logical throughput | 54855.0 records/s | 14852.2 records/s | 8947.4 records/s | 35942.5 records/s | 55666.8 records/s | 36751.0 records/s |
| Physical throughput | 37.2 MiB/s | 10.1 MiB/s | 6.1 MiB/s | 24.4 MiB/s | 37.7 MiB/s | 29.5 MiB/s |
| Output bytes | 2419795.0 B | 1578653.0 B | 1578653.0 B | 1872399.0 B | 1872399.0 B | 1872399.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
