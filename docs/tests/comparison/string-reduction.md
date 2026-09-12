---
type: Report
title: "String reduction"
description: "Measures concatenation of a field collected from every feature."
workload: benchmark.string-reduction
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# String reduction

## What this measures

The query collects non-null place names and combines them with `add`. It
exercises string accumulation over the entire feature collection.

## Why it matters

Reports and exports often assemble text from many records. This case makes
allocation and final-output cost visible when the result is one large string.

## Input and output

The input is a natural snapshot with `properties.place` strings. The output is
one concatenated string in feature order, not a list of place names.

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
| Wall time | 3.0 ms | 9.1 ms | 17.8 ms | 4.5 ms | 5.2 ms | 4.9 ms |
| Wall time dispersion | 0.1 ms / 3.4 ms / 2.8-3.4 ms | 0.1 ms / 9.7 ms / 8.7-9.7 ms | 0.4 ms / 18.9 ms / 17.1-18.9 ms | 0.1 ms / 4.8 ms / 4.2-4.8 ms | 0.1 ms / 5.6 ms / 5.0-5.6 ms | 0.1 ms / 5.2 ms / 4.5-5.2 ms |
| First output | 2.8 ms | 9.0 ms | 17.7 ms | 4.2 ms | 5.0 ms | 4.7 ms |
| First output dispersion | 0.1 ms / 3.2 ms / 2.6-3.2 ms | 0.1 ms / 9.7 ms / 8.7-9.7 ms | 0.4 ms / 18.8 ms / 17.0-18.8 ms | 0.1 ms / 4.6 ms / 4.0-4.6 ms | 0.1 ms / 5.5 ms / 4.7-5.5 ms | 0.1 ms / 5.0 ms / 4.3-5.0 ms |
| User CPU | 2.0 ms | 9.3 ms | 18.2 ms | 2.9 ms | 3.1 ms | 2.9 ms |
| System CPU | 0.9 ms | 3.8 ms | 8.1 ms | 1.7 ms | 2.0 ms | 2.0 ms |
| Peak RSS | 4.5 MiB | 19.0 MiB | 31.8 MiB | 9.1 MiB | 9.7 MiB | 9.3 MiB |
| Logical throughput | 64861.3 records/s | 21273.1 records/s | 10872.3 records/s | 43264.9 records/s | 37118.5 records/s | 39628.2 records/s |
| Physical throughput | 44.4 MiB/s | 14.6 MiB/s | 7.4 MiB/s | 29.6 MiB/s | 25.4 MiB/s | 32.1 MiB/s |
| Output bytes | 5446.0 B | 5446.0 B | 5446.0 B | 5446.0 B | 5446.0 B | 5446.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | 3.0 ms | 3.2 ms | 1.2 ms | 1.3 ms | 1.2 ms |
| Wall time dispersion | 0.0 ms / 1.8 ms / 1.3-1.8 ms | 0.1 ms / 3.2 ms / 2.7-3.2 ms | 0.1 ms / 3.4 ms / 3.0-3.4 ms | 0.0 ms / 1.4 ms / 1.2-1.4 ms | 0.1 ms / 1.5 ms / 1.2-1.5 ms | 0.0 ms / 1.5 ms / 1.1-1.5 ms |
| First output | 1.3 ms | 2.6 ms | 2.7 ms | 1.0 ms | 1.0 ms | 1.0 ms |
| First output dispersion | 0.0 ms / 1.7 ms / 1.2-1.7 ms | 0.1 ms / 2.8 ms / 2.4-2.9 ms | 0.0 ms / 2.9 ms / 2.6-2.9 ms | 0.0 ms / 1.1 ms / 1.0-1.2 ms | 0.0 ms / 1.3 ms / 1.0-1.4 ms | 0.0 ms / 1.2 ms / 1.0-1.3 ms |
| User CPU | 1.3 ms | 1.0 ms | 1.1 ms | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.0 ms | 2.1 ms | 2.1 ms | 1.1 ms | 1.1 ms | 1.1 ms |
| Peak RSS | 4.0 MiB | 11.9 MiB | 12.1 MiB | 8.2 MiB | 8.5 MiB | 8.0 MiB |
| Logical throughput | 1333.8 records/s | 660.3 records/s | 627.7 records/s | 1692.0 records/s | 1536.7 records/s | 1692.0 records/s |
| Physical throughput | 1.1 MiB/s | 0.6 MiB/s | 0.5 MiB/s | 1.4 MiB/s | 1.3 MiB/s | 1.6 MiB/s |
| Output bytes | 55.0 B | 55.0 B | 55.0 B | 55.0 B | 55.0 B | 55.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `process exited with classified error Resource (exit status 5)`, `tool identity differs`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 104.5 ms | 314.8 ms | 886.1 ms | - | - | - |
| Wall time dispersion | 0.8 ms / 106.1 ms / 103.5-106.1 ms | 3.2 ms / 320.7 ms / 301.2-320.7 ms | 8.2 ms / 968.6 ms / 867.4-968.6 ms | - | - | - |
| First output | 98.9 ms | 292.9 ms | 828.5 ms | - | - | - |
| First output dispersion | 0.7 ms / 100.5 ms / 98.0-100.5 ms | 2.8 ms / 298.5 ms / 281.8-298.5 ms | 12.1 ms / 899.4 ms / 812.7-899.4 ms | - | - | - |
| User CPU | 81.4 ms | 352.4 ms | 1267.7 ms | - | - | - |
| System CPU | 23.0 ms | 123.6 ms | 406.5 ms | - | - | - |
| Peak RSS | 60.9 MiB | 333.4 MiB | 1296.1 MiB | - | - | - |
| Logical throughput | 107869.7 records/s | 35815.9 records/s | 12722.7 records/s | - | - | - |
| Physical throughput | 73.1 MiB/s | 24.3 MiB/s | 8.6 MiB/s | - | - | - |
| Output bytes | 313616.0 B | 313616.0 B | 313616.0 B | - | - | - |
| Outcome | timed | timed | timed | resource-limit | resource-limit | resource-limit |
| Details | none | none | none | process exited with classified error Resource (exit status 5) | process exited with classified error Resource (exit status 5) | process exited with classified error Resource (exit status 5) |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | not timed | not timed | not timed |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `process exited with classified error Resource (exit status 5)`, `tool identity differs`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 21.0 ms | 64.8 ms | 174.3 ms | - | - | - |
| Wall time dispersion | 0.5 ms / 21.6 ms / 19.5-21.6 ms | 0.5 ms / 65.6 ms / 62.6-65.6 ms | 7.0 ms / 187.1 ms / 161.4-187.1 ms | - | - | - |
| First output | 19.8 ms | 59.9 ms | 161.4 ms | - | - | - |
| First output dispersion | 0.4 ms / 20.6 ms / 18.4-20.6 ms | 0.5 ms / 60.8 ms / 57.8-60.8 ms | 5.6 ms / 171.9 ms / 150.5-171.9 ms | - | - | - |
| User CPU | 17.0 ms | 63.7 ms | 202.6 ms | - | - | - |
| System CPU | 4.0 ms | 29.3 ms | 88.5 ms | - | - | - |
| Peak RSS | 14.8 MiB | 75.8 MiB | 235.6 MiB | - | - | - |
| Logical throughput | 106063.5 records/s | 34353.1 records/s | 12767.7 records/s | - | - | - |
| Physical throughput | 71.9 MiB/s | 23.3 MiB/s | 8.6 MiB/s | - | - | - |
| Output bytes | 61822.0 B | 61822.0 B | 61822.0 B | - | - | - |
| Outcome | timed | timed | timed | resource-limit | resource-limit | resource-limit |
| Details | none | none | none | process exited with classified error Resource (exit status 5) | process exited with classified error Resource (exit status 5) | process exited with classified error Resource (exit status 5) |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | not timed | not timed | not timed |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
