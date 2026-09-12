---
type: Report
title: "Shell quoting"
description: "Measures shell-safe formatting of selected feature fields."
workload: benchmark.format-shell
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Shell quoting

## What this measures

The query selects `id`, `place`, and `mag` for each feature and applies `@sh`.
It exercises quoting and escaping for values that may contain shell-sensitive
characters.

## Why it matters

Generated command arguments need quoting that preserves data instead of letting
the shell interpret it. This is formatting work, not permission to execute the
result.

## Input and output

The input is a natural feature snapshot. The output is one shell-quoted string
per feature containing the selected fields, not a command and not the original
array.

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

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq @sh expression for this array.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 3.2 ms | - | - | 5.1 ms | 5.9 ms | 5.5 ms |
| Wall time dispersion | 0.1 ms / 3.7 ms / 3.1-3.7 ms | - | - | 0.2 ms / 5.5 ms / 4.9-5.5 ms | 0.1 ms / 6.4 ms / 5.8-6.4 ms | 0.1 ms / 6.1 ms / 5.3-6.1 ms |
| First output | 2.8 ms | - | - | 4.8 ms | 5.6 ms | 5.2 ms |
| First output dispersion | 0.1 ms / 3.2 ms / 2.6-3.2 ms | - | - | 0.1 ms / 5.2 ms / 4.6-5.2 ms | 0.1 ms / 6.1 ms / 5.5-6.1 ms | 0.1 ms / 5.8 ms / 5.0-5.8 ms |
| User CPU | 3.0 ms | - | - | 3.0 ms | 3.9 ms | 3.3 ms |
| System CPU | 0.0 ms | - | - | 2.1 ms | 1.9 ms | 1.9 ms |
| Peak RSS | 4.4 MiB | - | - | 9.1 MiB | 9.7 MiB | 9.0 MiB |
| Logical throughput | 60314.0 records/s | - | - | 37961.1 records/s | 32789.7 records/s | 35560.4 records/s |
| Physical throughput | 41.3 MiB/s | - | - | 26.0 MiB/s | 22.4 MiB/s | 28.8 MiB/s |
| Output bytes | 9958.0 B | - | - | 9952.0 B | 9952.0 B | 9952.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq @sh expression for this array. | yq rejects the catalog jq @sh expression for this array. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq @sh expression for this array.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | - | - | 1.3 ms | 1.3 ms | 1.2 ms |
| Wall time dispersion | 0.0 ms / 1.8 ms / 1.3-1.8 ms | - | - | 0.2 ms / 1.5 ms / 1.2-1.5 ms | 0.1 ms / 1.5 ms / 1.2-1.5 ms | 0.0 ms / 1.5 ms / 1.1-1.5 ms |
| First output | 1.4 ms | - | - | 1.2 ms | 1.0 ms | 1.0 ms |
| First output dispersion | 0.0 ms / 1.7 ms / 1.2-1.7 ms | - | - | 0.1 ms / 1.4 ms / 1.0-1.4 ms | 0.0 ms / 1.3 ms / 1.0-1.4 ms | 0.0 ms / 1.3 ms / 1.0-1.4 ms |
| User CPU | 0.7 ms | - | - | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.7 ms | - | - | 1.1 ms | 1.1 ms | 1.1 ms |
| Peak RSS | 4.0 MiB | - | - | 8.0 MiB | 8.4 MiB | 7.9 MiB |
| Logical throughput | 1330.7 records/s | - | - | 1499.3 records/s | 1533.2 records/s | 1680.7 records/s |
| Physical throughput | 1.1 MiB/s | - | - | 1.3 MiB/s | 1.3 MiB/s | 1.6 MiB/s |
| Output bytes | 98.0 B | - | - | 98.0 B | 98.0 B | 98.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq @sh expression for this array. | yq rejects the catalog jq @sh expression for this array. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq @sh expression for this array.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 116.6 ms | - | - | 217.7 ms | 260.4 ms | 240.5 ms |
| Wall time dispersion | 0.5 ms / 117.3 ms / 114.2-117.3 ms | - | - | 3.2 ms / 221.6 ms / 211.8-221.6 ms | 2.5 ms / 264.7 ms / 255.3-264.7 ms | 3.3 ms / 246.5 ms / 233.7-246.5 ms |
| First output | 79.5 ms | - | - | 114.2 ms | 155.1 ms | 131.3 ms |
| First output dispersion | 0.2 ms / 80.2 ms / 78.7-80.2 ms | - | - | 1.1 ms / 116.2 ms / 112.1-116.2 ms | 0.7 ms / 157.9 ms / 152.9-157.9 ms | 1.3 ms / 135.6 ms / 129.3-135.6 ms |
| User CPU | 92.7 ms | - | - | 157.2 ms | 192.2 ms | 178.0 ms |
| System CPU | 23.0 ms | - | - | 58.2 ms | 67.1 ms | 62.0 ms |
| Peak RSS | 60.5 MiB | - | - | 69.4 MiB | 89.4 MiB | 78.1 MiB |
| Logical throughput | 96653.9 records/s | - | - | 51795.7 records/s | 43296.9 records/s | 46879.9 records/s |
| Physical throughput | 65.5 MiB/s | - | - | 35.1 MiB/s | 29.3 MiB/s | 37.6 MiB/s |
| Output bytes | 576240.0 B | - | - | 575966.0 B | 575966.0 B | 575966.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq @sh expression for this array. | yq rejects the catalog jq @sh expression for this array. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq @sh expression for this array.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 23.0 ms | - | - | 42.8 ms | 49.9 ms | 47.0 ms |
| Wall time dispersion | 0.5 ms / 25.2 ms / 22.0-25.2 ms | - | - | 0.7 ms / 44.4 ms / 39.7-44.4 ms | 0.5 ms / 51.5 ms / 49.3-51.5 ms | 1.3 ms / 48.6 ms / 43.7-48.6 ms |
| First output | 17.1 ms | - | - | 32.4 ms | 39.6 ms | 36.1 ms |
| First output dispersion | 0.2 ms / 18.9 ms / 16.5-18.9 ms | - | - | 0.6 ms / 33.8 ms / 30.3-33.8 ms | 0.5 ms / 40.8 ms / 38.8-40.8 ms | 0.7 ms / 37.5 ms / 33.7-37.5 ms |
| User CPU | 18.3 ms | - | - | 28.1 ms | 35.2 ms | 33.9 ms |
| System CPU | 5.5 ms | - | - | 12.5 ms | 14.6 ms | 12.4 ms |
| Peak RSS | 14.6 MiB | - | - | 20.2 MiB | 24.4 MiB | 21.7 MiB |
| Logical throughput | 96697.1 records/s | - | - | 52015.8 records/s | 44627.2 records/s | 47347.5 records/s |
| Physical throughput | 65.5 MiB/s | - | - | 35.2 MiB/s | 30.2 MiB/s | 38.0 MiB/s |
| Output bytes | 113994.0 B | - | - | 113950.0 B | 113950.0 B | 113950.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq @sh expression for this array. | yq rejects the catalog jq @sh expression for this array. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
