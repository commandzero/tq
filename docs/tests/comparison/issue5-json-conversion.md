---
type: Report
title: "JSON round trip"
description: "Measures converting metadata to JSON text and parsing it back."
workload: benchmark.issue5-json-conversion
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# JSON round trip

## What this measures

The query serializes `.metadata` with `tojson`, parses it with `fromjson`, and
returns the resulting object's length. It includes both text conversion steps.

## Why it matters

Applications cross JSON text boundaries when they log, cache, or hand data to
another process. A round trip can cost more than an in-memory field read.

## Input and output

The input is a metadata object from a natural snapshot. The output is one
integer for the number of members after the round trip, not the JSON text.

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
| Wall time | 2.9 ms | 8.8 ms | 17.8 ms | 3.6 ms | 4.2 ms | 3.9 ms |
| Wall time dispersion | 0.1 ms / 3.1 ms / 2.7-3.1 ms | 0.1 ms / 9.0 ms / 8.3-9.0 ms | 0.5 ms / 19.1 ms / 17.0-19.1 ms | 0.2 ms / 4.2 ms / 3.1-4.2 ms | 0.2 ms / 4.5 ms / 4.0-4.5 ms | 0.1 ms / 4.0 ms / 3.6-4.0 ms |
| First output | 2.8 ms | 8.7 ms | 17.7 ms | 3.3 ms | 3.9 ms | 3.6 ms |
| First output dispersion | 0.1 ms / 3.1 ms / 2.6-3.1 ms | 0.1 ms / 9.0 ms / 8.3-9.0 ms | 0.5 ms / 18.9 ms / 16.8-18.9 ms | 0.1 ms / 3.8 ms / 3.0-3.8 ms | 0.2 ms / 4.2 ms / 3.7-4.2 ms | 0.1 ms / 3.7 ms / 3.3-3.7 ms |
| User CPU | 1.8 ms | 7.5 ms | 16.8 ms | 2.1 ms | 2.1 ms | 2.5 ms |
| System CPU | 1.0 ms | 4.8 ms | 10.4 ms | 1.1 ms | 1.9 ms | 1.2 ms |
| Peak RSS | 4.4 MiB | 18.4 MiB | 33.4 MiB | 9.1 MiB | 9.8 MiB | 9.3 MiB |
| Logical throughput | 66954.3 records/s | 22163.8 records/s | 10883.0 records/s | 54532.7 records/s | 46350.5 records/s | 50252.6 records/s |
| Physical throughput | 45.8 MiB/s | 15.2 MiB/s | 7.4 MiB/s | 37.3 MiB/s | 31.7 MiB/s | 40.7 MiB/s |
| Output bytes | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | 3.1 ms | 3.1 ms | 1.3 ms | 1.3 ms | 1.2 ms |
| Wall time dispersion | 0.1 ms / 1.7 ms / 1.3-1.8 ms | 0.1 ms / 3.5 ms / 2.7-3.6 ms | 0.1 ms / 3.4 ms / 2.9-3.4 ms | 0.1 ms / 1.6 ms / 1.1-1.6 ms | 0.1 ms / 1.7 ms / 1.1-1.7 ms | 0.0 ms / 1.6 ms / 1.2-1.6 ms |
| First output | 1.3 ms | 2.7 ms | 2.7 ms | 1.0 ms | 1.0 ms | 1.0 ms |
| First output dispersion | 0.0 ms / 1.7 ms / 1.2-1.7 ms | 0.1 ms / 3.2 ms / 2.4-3.3 ms | 0.1 ms / 3.1 ms / 2.6-3.1 ms | 0.0 ms / 1.3 ms / 1.0-1.4 ms | 0.0 ms / 1.4 ms / 1.0-1.5 ms | 0.0 ms / 1.4 ms / 1.0-1.4 ms |
| User CPU | 1.3 ms | 1.0 ms | 1.1 ms | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.0 ms | 2.1 ms | 2.1 ms | 1.1 ms | 1.1 ms | 1.1 ms |
| Peak RSS | 4.1 MiB | 12.0 MiB | 12.0 MiB | 8.1 MiB | 8.5 MiB | 8.1 MiB |
| Logical throughput | 1340.0 records/s | 647.5 records/s | 650.4 records/s | 1543.2 records/s | 1586.0 records/s | 1668.8 records/s |
| Physical throughput | 1.1 MiB/s | 0.5 MiB/s | 0.6 MiB/s | 1.3 MiB/s | 1.3 MiB/s | 1.6 MiB/s |
| Output bytes | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 94.4 ms | 257.3 ms | 912.9 ms | 120.1 ms | 161.5 ms | 137.8 ms |
| Wall time dispersion | 1.1 ms / 96.0 ms / 92.2-96.0 ms | 2.1 ms / 261.9 ms / 253.3-261.9 ms | 12.1 ms / 929.1 ms / 837.7-929.1 ms | 0.5 ms / 122.4 ms / 118.1-122.4 ms | 1.3 ms / 167.9 ms / 159.5-167.9 ms | 1.3 ms / 141.2 ms / 136.3-141.2 ms |
| First output | 90.7 ms | 240.0 ms | 845.1 ms | 117.1 ms | 157.1 ms | 134.4 ms |
| First output dispersion | 1.0 ms / 92.3 ms / 89.5-92.3 ms | 1.9 ms / 243.7 ms / 235.1-243.7 ms | 8.4 ms / 858.0 ms / 781.5-858.0 ms | 0.7 ms / 118.9 ms / 115.0-118.9 ms | 1.0 ms / 163.6 ms / 155.9-163.6 ms | 1.0 ms / 137.9 ms / 133.2-137.9 ms |
| User CPU | 69.0 ms | 238.2 ms | 1243.8 ms | 96.1 ms | 131.5 ms | 109.6 ms |
| System CPU | 24.1 ms | 107.7 ms | 454.2 ms | 23.8 ms | 28.9 ms | 27.5 ms |
| Peak RSS | 60.3 MiB | 293.4 MiB | 1299.4 MiB | 69.3 MiB | 89.5 MiB | 78.3 MiB |
| Logical throughput | 119466.6 records/s | 43825.0 records/s | 12350.0 records/s | 93881.9 records/s | 69788.8 records/s | 81795.2 records/s |
| Physical throughput | 81.0 MiB/s | 29.7 MiB/s | 8.4 MiB/s | 63.6 MiB/s | 47.2 MiB/s | 65.7 MiB/s |
| Output bytes | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 19.5 ms | 56.3 ms | 168.9 ms | 24.9 ms | 32.0 ms | 28.7 ms |
| Wall time dispersion | 0.3 ms / 20.0 ms / 18.6-20.0 ms | 0.7 ms / 57.6 ms / 54.4-57.6 ms | 7.6 ms / 181.2 ms / 156.3-181.2 ms | 0.5 ms / 26.2 ms / 22.9-26.2 ms | 0.3 ms / 32.9 ms / 31.3-32.9 ms | 1.2 ms / 29.9 ms / 26.6-29.9 ms |
| First output | 18.8 ms | 51.8 ms | 157.2 ms | 24.1 ms | 31.0 ms | 27.9 ms |
| First output dispersion | 0.3 ms / 19.3 ms / 18.1-19.3 ms | 0.6 ms / 52.9 ms / 50.5-52.9 ms | 6.0 ms / 166.5 ms / 144.6-166.5 ms | 0.6 ms / 25.4 ms / 22.2-25.4 ms | 0.3 ms / 32.0 ms / 30.4-32.0 ms | 1.1 ms / 29.0 ms / 26.0-29.0 ms |
| User CPU | 14.0 ms | 48.7 ms | 204.6 ms | 18.5 ms | 24.6 ms | 21.9 ms |
| System CPU | 5.5 ms | 27.2 ms | 86.6 ms | 6.1 ms | 7.7 ms | 6.5 ms |
| Peak RSS | 14.5 MiB | 68.6 MiB | 226.3 MiB | 20.4 MiB | 24.3 MiB | 21.8 MiB |
| Logical throughput | 114155.3 records/s | 39545.0 records/s | 13174.7 records/s | 89526.4 records/s | 69429.3 records/s | 77611.3 records/s |
| Physical throughput | 77.4 MiB/s | 26.8 MiB/s | 8.9 MiB/s | 60.7 MiB/s | 47.0 MiB/s | 62.3 MiB/s |
| Output bytes | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
