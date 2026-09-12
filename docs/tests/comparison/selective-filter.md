---
type: Report
title: "Selective filtering"
description: "Measures streaming selection and projection of matching features."
workload: benchmark.selective-filter
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Selective filtering

## What this measures

The query selects features with magnitude at least 2 and then projects their
IDs. It combines iteration, a numeric predicate, and a downstream field read.

## Why it matters

This is the shape of a useful event or records query: scan a collection, keep a
subset, and pass compact identifiers to the next command.

## Input and output

The input is a natural feature snapshot with numeric `properties.mag` values.
The output is the ordered sequence of IDs for matching features. It is not the
matching feature objects and not a packed array.

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
| Wall time | 2.9 ms | 9.9 ms | 18.5 ms | 4.5 ms | 4.9 ms | 5.2 ms |
| Wall time dispersion | 0.0 ms / 3.6 ms / 2.9-3.6 ms | 0.2 ms / 10.5 ms / 9.6-10.5 ms | 0.3 ms / 19.5 ms / 17.8-19.5 ms | 0.2 ms / 4.8 ms / 4.1-4.8 ms | 0.2 ms / 5.3 ms / 4.4-5.3 ms | 0.1 ms / 5.6 ms / 4.9-5.6 ms |
| First output | 2.9 ms | 9.4 ms | 18.0 ms | 4.3 ms | 4.6 ms | 4.9 ms |
| First output dispersion | 0.1 ms / 3.2 ms / 2.7-3.2 ms | 0.2 ms / 10.2 ms / 9.2-10.2 ms | 0.3 ms / 19.1 ms / 17.3-19.1 ms | 0.3 ms / 4.6 ms / 3.9-4.6 ms | 0.1 ms / 5.0 ms / 4.3-5.0 ms | 0.1 ms / 5.5 ms / 4.8-5.5 ms |
| User CPU | 1.9 ms | 8.6 ms | 17.6 ms | 3.1 ms | 3.4 ms | 4.0 ms |
| System CPU | 1.0 ms | 5.2 ms | 9.0 ms | 1.1 ms | 1.1 ms | 1.0 ms |
| Peak RSS | 4.3 MiB | 19.5 MiB | 33.1 MiB | 8.5 MiB | 9.8 MiB | 8.0 MiB |
| Logical throughput | 65907.9 records/s | 19515.1 records/s | 10509.2 records/s | 43082.4 records/s | 39335.0 records/s | 37607.8 records/s |
| Physical throughput | 45.1 MiB/s | 13.4 MiB/s | 7.2 MiB/s | 29.5 MiB/s | 26.9 MiB/s | 30.4 MiB/s |
| Output bytes | 918.0 B | 918.0 B | 918.0 B | 786.0 B | 786.0 B | 786.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.4 ms | 2.9 ms | 3.1 ms | 1.3 ms | 1.3 ms | 1.3 ms |
| Wall time dispersion | 0.1 ms / 1.7 ms / 1.3-1.8 ms | 0.1 ms / 3.2 ms / 2.7-3.3 ms | 0.1 ms / 3.4 ms / 2.9-3.4 ms | 0.0 ms / 1.5 ms / 1.2-1.7 ms | 0.1 ms / 1.5 ms / 1.2-1.5 ms | 0.1 ms / 1.7 ms / 1.2-1.7 ms |
| First output | - | - | - | - | - | - |
| First output dispersion | - | - | - | - | - | - |
| User CPU | 0.7 ms | 1.5 ms | 1.0 ms | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.6 ms | 1.5 ms | 2.1 ms | 1.1 ms | 1.1 ms | 1.1 ms |
| Peak RSS | 4.0 MiB | 11.1 MiB | 11.3 MiB | 8.6 MiB | 8.3 MiB | 7.9 MiB |
| Logical throughput | 1408.0 records/s | 684.8 records/s | 652.5 records/s | 1503.2 records/s | 1533.7 records/s | 1526.7 records/s |
| Physical throughput | 1.2 MiB/s | 0.6 MiB/s | 0.6 MiB/s | 1.3 MiB/s | 1.3 MiB/s | 1.5 MiB/s |
| Output bytes | 0.0 B | 0.0 B | 0.0 B | 0.0 B | 0.0 B | 0.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 101.4 ms | 365.8 ms | 969.3 ms | 163.4 ms | 202.3 ms | 197.2 ms |
| Wall time dispersion | 1.3 ms / 106.4 ms / 99.9-106.4 ms | 2.3 ms / 371.9 ms / 355.8-371.9 ms | 39.4 ms / 1027.0 ms / 925.4-1027.0 ms | 0.4 ms / 164.4 ms / 162.5-164.4 ms | 1.3 ms / 205.9 ms / 200.8-205.9 ms | 1.6 ms / 203.2 ms / 194.6-203.2 ms |
| First output | 79.6 ms | 321.1 ms | 883.6 ms | 163.2 ms | 197.8 ms | 197.1 ms |
| First output dispersion | 0.4 ms / 82.7 ms / 78.7-82.7 ms | 1.9 ms / 329.9 ms / 314.4-329.9 ms | 35.1 ms / 932.9 ms / 846.1-932.9 ms | 0.4 ms / 164.3 ms / 162.2-164.3 ms | 1.4 ms / 201.3 ms / 196.2-201.3 ms | 1.4 ms / 203.0 ms / 194.4-203.0 ms |
| User CPU | 76.0 ms | 393.8 ms | 1311.8 ms | 161.2 ms | 160.8 ms | 194.1 ms |
| System CPU | 24.9 ms | 128.1 ms | 445.1 ms | 2.0 ms | 41.1 ms | 3.0 ms |
| Peak RSS | 60.3 MiB | 344.6 MiB | 1296.4 MiB | 8.8 MiB | 89.4 MiB | 8.0 MiB |
| Logical throughput | 111195.5 records/s | 30819.7 records/s | 11631.3 records/s | 69001.6 records/s | 55733.8 records/s | 57161.3 records/s |
| Physical throughput | 75.4 MiB/s | 20.9 MiB/s | 7.9 MiB/s | 46.8 MiB/s | 37.7 MiB/s | 45.9 MiB/s |
| Output bytes | 47722.0 B | 47722.0 B | 47722.0 B | 40954.0 B | 40954.0 B | 40954.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 20.5 ms | 77.5 ms | 185.0 ms | 34.9 ms | 39.3 ms | 40.6 ms |
| Wall time dispersion | 0.2 ms / 21.4 ms / 19.2-21.4 ms | 2.5 ms / 81.2 ms / 73.9-81.2 ms | 11.7 ms / 203.0 ms / 170.1-203.0 ms | 0.5 ms / 37.2 ms / 33.9-37.2 ms | 0.5 ms / 40.1 ms / 37.8-40.1 ms | 0.3 ms / 42.5 ms / 39.2-42.5 ms |
| First output | 17.4 ms | 66.7 ms | 166.7 ms | 34.8 ms | 38.2 ms | 40.4 ms |
| First output dispersion | 0.4 ms / 18.3 ms / 16.5-18.3 ms | 1.8 ms / 69.2 ms / 64.1-69.2 ms | 10.4 ms / 182.0 ms / 155.1-182.0 ms | 0.6 ms / 37.0 ms / 33.8-37.0 ms | 0.6 ms / 39.2 ms / 36.9-39.2 ms | 0.2 ms / 42.3 ms / 39.1-42.3 ms |
| User CPU | 15.7 ms | 80.1 ms | 213.3 ms | 33.3 ms | 30.2 ms | 38.8 ms |
| System CPU | 4.6 ms | 33.3 ms | 88.0 ms | 1.5 ms | 8.7 ms | 1.5 ms |
| Peak RSS | 14.6 MiB | 88.1 MiB | 236.7 MiB | 8.6 MiB | 24.3 MiB | 7.9 MiB |
| Logical throughput | 108703.6 records/s | 28698.9 records/s | 12029.1 records/s | 63700.6 records/s | 56581.9 records/s | 54763.2 records/s |
| Physical throughput | 73.7 MiB/s | 19.4 MiB/s | 8.1 MiB/s | 43.2 MiB/s | 38.3 MiB/s | 43.9 MiB/s |
| Output bytes | 9875.0 B | 9875.0 B | 9875.0 B | 8507.0 B | 8507.0 B | 8507.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
