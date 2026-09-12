---
type: Report
title: "Regular-expression testing"
description: "Measures Unicode-aware pattern testing over feature place names."
workload: benchmark.regex-test
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Regular-expression testing

## What this measures

The query tests each string place value against `^[A-Z]` and counts the
matches. It combines string filtering, a regular expression, and a numeric
reduction.

## Why it matters

Pattern checks sit inside log filters, validation rules, and text cleanup jobs.
This case measures the work of testing every candidate rather than finding one
match and stopping.

## Input and output

The input is a natural feature snapshot with place strings and other values.
The output is one integer containing the number of strings that start with an
ASCII uppercase letter.

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

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq strings/test expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 3.2 ms | - | - | 5.2 ms | 5.7 ms | 5.3 ms |
| Wall time dispersion | 0.2 ms / 3.5 ms / 2.9-3.5 ms | - | - | 0.1 ms / 5.3 ms / 4.9-5.3 ms | 0.1 ms / 6.1 ms / 5.6-6.1 ms | 0.1 ms / 5.8 ms / 4.9-5.8 ms |
| First output | 3.1 ms | - | - | 4.8 ms | 5.4 ms | 5.0 ms |
| First output dispersion | 0.2 ms / 3.4 ms / 2.8-3.4 ms | - | - | 0.1 ms / 5.0 ms / 4.6-5.0 ms | 0.1 ms / 5.8 ms / 5.3-5.8 ms | 0.1 ms / 5.3 ms / 4.6-5.3 ms |
| User CPU | 2.9 ms | - | - | 3.1 ms | 4.3 ms | 3.6 ms |
| System CPU | 0.0 ms | - | - | 2.0 ms | 1.3 ms | 1.4 ms |
| Peak RSS | 4.4 MiB | - | - | 10.0 MiB | 10.3 MiB | 9.9 MiB |
| Logical throughput | 61363.3 records/s | - | - | 37604.2 records/s | 34139.9 records/s | 36610.7 records/s |
| Physical throughput | 42.0 MiB/s | - | - | 25.7 MiB/s | 23.3 MiB/s | 29.6 MiB/s |
| Output bytes | 4.0 B | - | - | 4.0 B | 4.0 B | 4.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq strings/test expression. | yq rejects the catalog jq strings/test expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq strings/test expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | - | - | 1.5 ms | 1.5 ms | 1.3 ms |
| Wall time dispersion | 0.0 ms / 1.8 ms / 1.3-1.8 ms | - | - | 0.1 ms / 1.7 ms / 1.3-1.8 ms | 0.1 ms / 1.8 ms / 1.3-1.8 ms | 0.0 ms / 1.7 ms / 1.3-1.8 ms |
| First output | 1.3 ms | - | - | 1.2 ms | 1.2 ms | 1.2 ms |
| First output dispersion | 0.0 ms / 1.7 ms / 1.3-1.7 ms | - | - | 0.0 ms / 1.5 ms / 1.0-1.5 ms | 0.0 ms / 1.5 ms / 1.1-1.6 ms | 0.0 ms / 1.5 ms / 1.2-1.5 ms |
| User CPU | 0.8 ms | - | - | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.7 ms | - | - | 1.3 ms | 1.3 ms | 1.2 ms |
| Peak RSS | 4.1 MiB | - | - | 8.9 MiB | 9.0 MiB | 8.8 MiB |
| Logical throughput | 1333.3 records/s | - | - | 1376.9 records/s | 1361.0 records/s | 1493.1 records/s |
| Physical throughput | 1.1 MiB/s | - | - | 1.2 MiB/s | 1.2 MiB/s | 1.4 MiB/s |
| Output bytes | 2.0 B | - | - | 2.0 B | 2.0 B | 2.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq strings/test expression. | yq rejects the catalog jq strings/test expression. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq strings/test expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 113.8 ms | - | - | 194.6 ms | 235.6 ms | 211.4 ms |
| Wall time dispersion | 1.0 ms / 117.7 ms / 112.4-117.7 ms | - | - | 1.9 ms / 197.5 ms / 192.1-197.5 ms | 2.8 ms / 244.6 ms / 231.8-244.6 ms | 0.7 ms / 214.6 ms / 209.3-214.6 ms |
| First output | 110.1 ms | - | - | 190.5 ms | 230.1 ms | 207.1 ms |
| First output dispersion | 0.8 ms / 114.2 ms / 108.6-114.2 ms | - | - | 1.7 ms / 192.9 ms / 188.0-192.9 ms | 2.5 ms / 238.3 ms / 227.6-238.3 ms | 0.3 ms / 210.7 ms / 205.1-210.7 ms |
| User CPU | 85.0 ms | - | - | 165.5 ms | 200.9 ms | 177.4 ms |
| System CPU | 28.4 ms | - | - | 28.5 ms | 35.4 ms | 32.9 ms |
| Peak RSS | 60.7 MiB | - | - | 70.9 MiB | 90.9 MiB | 79.8 MiB |
| Logical throughput | 99030.3 records/s | - | - | 57934.8 records/s | 47847.8 records/s | 53338.3 records/s |
| Physical throughput | 67.1 MiB/s | - | - | 39.3 MiB/s | 32.4 MiB/s | 42.8 MiB/s |
| Output bytes | 6.0 B | - | - | 6.0 B | 6.0 B | 6.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq strings/test expression. | yq rejects the catalog jq strings/test expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq strings/test expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 22.9 ms | - | - | 39.1 ms | 45.9 ms | 43.0 ms |
| Wall time dispersion | 0.5 ms / 24.5 ms / 22.1-24.5 ms | - | - | 0.4 ms / 40.9 ms / 38.3-40.9 ms | 0.6 ms / 46.9 ms / 44.5-46.9 ms | 0.6 ms / 44.8 ms / 42.1-44.8 ms |
| First output | 22.1 ms | - | - | 38.0 ms | 45.0 ms | 42.2 ms |
| First output dispersion | 0.5 ms / 23.7 ms / 21.4-23.7 ms | - | - | 0.4 ms / 39.9 ms / 37.4-39.9 ms | 0.7 ms / 45.9 ms / 43.5-45.9 ms | 0.6 ms / 43.9 ms / 41.2-43.9 ms |
| User CPU | 17.9 ms | - | - | 33.5 ms | 38.4 ms | 36.0 ms |
| System CPU | 5.1 ms | - | - | 5.0 ms | 7.4 ms | 7.0 ms |
| Peak RSS | 14.8 MiB | - | - | 21.3 MiB | 25.1 MiB | 22.6 MiB |
| Logical throughput | 97238.0 records/s | - | - | 56976.0 records/s | 48467.6 records/s | 51686.5 records/s |
| Physical throughput | 65.9 MiB/s | - | - | 38.6 MiB/s | 32.8 MiB/s | 41.5 MiB/s |
| Output bytes | 5.0 B | - | - | 5.0 B | 5.0 B | 5.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq strings/test expression. | yq rejects the catalog jq strings/test expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
