---
type: Report
title: "String and scalar utilities"
description: "Measures case conversion and Unicode scalar handling across place names."
workload: benchmark.issue5-scalar-utilities
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# String and scalar utilities

## What this measures

The query lowercases each string place, expands it to Unicode code points with
`explode`, counts those scalars, and adds the counts. It combines text
transformation with a numeric reduction.

## Why it matters

Text utilities often sit in normalization and search pipelines. Unicode-aware
length work is different from counting bytes, especially for non-ASCII data.

## Input and output

The input is a natural snapshot with place values. The output is one integer
containing the total code-point count after lowercasing, not the transformed
strings.

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

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq scalar-utility expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 5.1 ms | - | - | 4.3 ms | 4.9 ms | 4.4 ms |
| Wall time dispersion | 0.1 ms / 5.6 ms / 4.7-5.6 ms | - | - | 0.1 ms / 4.9 ms / 4.2-4.9 ms | 0.1 ms / 5.0 ms / 4.6-5.0 ms | 0.1 ms / 4.7 ms / 4.0-4.7 ms |
| First output | 5.0 ms | - | - | 4.0 ms | 4.6 ms | 4.2 ms |
| First output dispersion | 0.1 ms / 5.6 ms / 4.5-5.6 ms | - | - | 0.1 ms / 4.4 ms / 3.9-4.4 ms | 0.1 ms / 4.8 ms / 4.2-4.8 ms | 0.1 ms / 4.4 ms / 3.9-4.4 ms |
| User CPU | 4.4 ms | - | - | 3.0 ms | 2.8 ms | 3.2 ms |
| System CPU | 0.5 ms | - | - | 1.1 ms | 1.9 ms | 1.1 ms |
| Peak RSS | 4.4 MiB | - | - | 9.3 MiB | 9.8 MiB | 9.1 MiB |
| Logical throughput | 37842.6 records/s | - | - | 45032.5 records/s | 39938.2 records/s | 44246.8 records/s |
| Physical throughput | 25.9 MiB/s | - | - | 30.8 MiB/s | 27.3 MiB/s | 35.8 MiB/s |
| Output bytes | 5.0 B | - | - | 5.0 B | 5.0 B | 5.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq scalar-utility expression. | yq rejects the catalog jq scalar-utility expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq scalar-utility expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | - | - | 1.3 ms | 1.3 ms | 1.3 ms |
| Wall time dispersion | 0.0 ms / 1.8 ms / 1.5-1.8 ms | - | - | 0.0 ms / 1.6 ms / 1.1-1.7 ms | 0.0 ms / 1.6 ms / 1.2-1.6 ms | 0.1 ms / 1.5 ms / 1.2-1.6 ms |
| First output | 1.4 ms | - | - | 1.0 ms | 1.1 ms | 1.0 ms |
| First output dispersion | 0.1 ms / 1.7 ms / 1.3-1.7 ms | - | - | 0.0 ms / 1.3 ms / 1.0-1.4 ms | 0.1 ms / 1.3 ms / 1.0-1.4 ms | 0.0 ms / 1.4 ms / 1.0-1.4 ms |
| User CPU | 0.7 ms | - | - | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.7 ms | - | - | 1.2 ms | 1.2 ms | 1.1 ms |
| Peak RSS | 4.0 MiB | - | - | 8.1 MiB | 8.4 MiB | 8.1 MiB |
| Logical throughput | 1325.4 records/s | - | - | 1499.8 records/s | 1498.1 records/s | 1508.3 records/s |
| Physical throughput | 1.1 MiB/s | - | - | 1.3 MiB/s | 1.3 MiB/s | 1.5 MiB/s |
| Output bytes | 3.0 B | - | - | 3.0 B | 3.0 B | 3.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq scalar-utility expression. | yq rejects the catalog jq scalar-utility expression. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq scalar-utility expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 211.6 ms | - | - | 154.7 ms | 198.5 ms | 172.7 ms |
| Wall time dispersion | 2.1 ms / 215.2 ms / 205.9-215.2 ms | - | - | 0.9 ms / 157.0 ms / 153.6-157.0 ms | 1.8 ms / 201.3 ms / 194.5-201.3 ms | 0.7 ms / 176.4 ms / 171.5-176.4 ms |
| First output | 207.3 ms | - | - | 150.5 ms | 192.7 ms | 168.7 ms |
| First output dispersion | 1.9 ms / 211.6 ms / 202.3-211.6 ms | - | - | 0.5 ms / 153.0 ms / 149.5-153.0 ms | 0.9 ms / 196.2 ms / 189.7-196.2 ms | 0.5 ms / 172.4 ms / 167.6-172.4 ms |
| User CPU | 185.6 ms | - | - | 126.2 ms | 160.5 ms | 141.6 ms |
| System CPU | 25.4 ms | - | - | 27.9 ms | 35.0 ms | 30.6 ms |
| Peak RSS | 60.7 MiB | - | - | 70.4 MiB | 90.6 MiB | 79.3 MiB |
| Logical throughput | 53270.7 records/s | - | - | 72877.0 records/s | 56808.6 records/s | 65287.1 records/s |
| Physical throughput | 36.1 MiB/s | - | - | 49.4 MiB/s | 38.5 MiB/s | 52.4 MiB/s |
| Output bytes | 7.0 B | - | - | 7.0 B | 7.0 B | 7.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq scalar-utility expression. | yq rejects the catalog jq scalar-utility expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq scalar-utility expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 41.5 ms | - | - | 31.4 ms | 38.5 ms | 34.8 ms |
| Wall time dispersion | 0.5 ms / 42.6 ms / 40.5-42.6 ms | - | - | 0.4 ms / 32.4 ms / 30.8-32.4 ms | 0.4 ms / 40.7 ms / 36.9-40.7 ms | 0.6 ms / 35.8 ms / 33.9-35.8 ms |
| First output | 40.7 ms | - | - | 30.5 ms | 37.3 ms | 33.8 ms |
| First output dispersion | 0.5 ms / 41.8 ms / 39.7-41.8 ms | - | - | 0.5 ms / 31.5 ms / 29.8-31.5 ms | 0.5 ms / 39.3 ms / 35.8-39.3 ms | 0.6 ms / 34.7 ms / 33.1-34.7 ms |
| User CPU | 37.1 ms | - | - | 25.4 ms | 31.2 ms | 27.7 ms |
| System CPU | 4.0 ms | - | - | 6.1 ms | 7.4 ms | 6.6 ms |
| Peak RSS | 14.6 MiB | - | - | 20.4 MiB | 24.3 MiB | 22.1 MiB |
| Logical throughput | 53670.1 records/s | - | - | 70824.9 records/s | 57835.0 records/s | 63957.9 records/s |
| Physical throughput | 36.4 MiB/s | - | - | 48.0 MiB/s | 39.1 MiB/s | 51.3 MiB/s |
| Output bytes | 6.0 B | - | - | 6.0 B | 6.0 B | 6.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq scalar-utility expression. | yq rejects the catalog jq scalar-utility expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
