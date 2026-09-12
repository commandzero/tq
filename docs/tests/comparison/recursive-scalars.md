---
type: Report
title: "Recursive scalar traversal"
description: "Measures depth-first traversal that emits every scalar value."
workload: benchmark.recursive-scalars
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Recursive scalar traversal

## What this measures

The query `.. | scalars` walks nested arrays and objects, then emits each
number, string, boolean, or null in traversal order. It combines recursion with
scalar filtering.

## Why it matters

Schema discovery, indexing, and redaction tools often need to inspect every
leaf in an unknown document. Depth and nesting affect the amount of work.

## Input and output

The input is a nested natural snapshot. The output is a depth-first sequence of
scalar leaves, not the containers that held them and not one flattened array.

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

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq recursive scalar expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 7.3 ms | - | - | 38.4 ms | 39.9 ms | 38.1 ms |
| Wall time dispersion | 0.2 ms / 7.6 ms / 7.0-7.6 ms | - | - | 1.1 ms / 41.9 ms / 36.8-41.9 ms | 0.6 ms / 42.3 ms / 38.1-42.3 ms | 0.7 ms / 40.5 ms / 37.3-40.5 ms |
| First output | 2.9 ms | - | - | 28.8 ms | 30.2 ms | 29.5 ms |
| First output dispersion | 0.1 ms / 3.1 ms / 2.6-3.1 ms | - | - | 0.9 ms / 32.2 ms / 27.8-32.2 ms | 0.3 ms / 32.1 ms / 28.9-32.1 ms | 0.5 ms / 31.9 ms / 28.6-31.9 ms |
| User CPU | 6.0 ms | - | - | 18.2 ms | 15.2 ms | 16.4 ms |
| System CPU | 1.1 ms | - | - | 19.6 ms | 24.4 ms | 21.7 ms |
| Peak RSS | 4.4 MiB | - | - | 9.0 MiB | 9.5 MiB | 9.1 MiB |
| Logical throughput | 26608.1 records/s | - | - | 5050.4 records/s | 4867.8 records/s | 5093.5 records/s |
| Physical throughput | 18.2 MiB/s | - | - | 3.5 MiB/s | 3.3 MiB/s | 4.1 MiB/s |
| Output bytes | 89511.0 B | - | - | 86587.0 B | 86587.0 B | 86587.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq recursive scalar expression. | yq rejects the catalog jq recursive scalar expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq recursive scalar expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | - | - | 1.8 ms | 1.8 ms | 1.7 ms |
| Wall time dispersion | 0.0 ms / 1.8 ms / 1.4-1.9 ms | - | - | 0.1 ms / 2.1 ms / 1.6-2.3 ms | 0.0 ms / 2.0 ms / 1.6-2.0 ms | 0.1 ms / 2.0 ms / 1.6-2.0 ms |
| First output | 1.5 ms | - | - | 1.6 ms | 1.5 ms | 1.5 ms |
| First output dispersion | 0.1 ms / 1.7 ms / 1.3-1.8 ms | - | - | 0.1 ms / 2.0 ms / 1.3-2.0 ms | 0.0 ms / 1.8 ms / 1.5-1.8 ms | 0.0 ms / 1.8 ms / 1.3-1.8 ms |
| User CPU | 0.8 ms | - | - | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.8 ms | - | - | 1.5 ms | 1.5 ms | 1.5 ms |
| Peak RSS | 4.1 MiB | - | - | 8.0 MiB | 8.3 MiB | 7.8 MiB |
| Logical throughput | 1321.9 records/s | - | - | 1110.2 records/s | 1111.7 records/s | 1201.9 records/s |
| Physical throughput | 1.1 MiB/s | - | - | 0.9 MiB/s | 0.9 MiB/s | 1.2 MiB/s |
| Output bytes | 1172.0 B | - | - | 1140.0 B | 1140.0 B | 1140.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq recursive scalar expression. | yq rejects the catalog jq recursive scalar expression. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq recursive scalar expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 342.7 ms | - | - | 2297.7 ms | 2314.8 ms | 2336.4 ms |
| Wall time dispersion | 2.2 ms / 347.7 ms / 338.6-347.7 ms | - | - | 6.6 ms / 2343.2 ms / 2252.8-2343.2 ms | 7.1 ms / 2348.3 ms / 2281.2-2348.3 ms | 13.1 ms / 2376.7 ms / 2298.1-2376.7 ms |
| First output | 80.8 ms | - | - | 132.0 ms | 172.4 ms | 148.1 ms |
| First output dispersion | 0.6 ms / 82.3 ms / 80.1-82.3 ms | - | - | 1.1 ms / 133.5 ms / 128.4-133.5 ms | 1.3 ms / 175.4 ms / 169.9-175.4 ms | 1.3 ms / 152.2 ms / 144.1-152.2 ms |
| User CPU | 312.0 ms | - | - | 891.3 ms | 891.1 ms | 853.3 ms |
| System CPU | 29.9 ms | - | - | 1201.4 ms | 1250.9 ms | 1251.7 ms |
| Peak RSS | 64.9 MiB | - | - | 69.1 MiB | 89.1 MiB | 77.9 MiB |
| Logical throughput | 32895.2 records/s | - | - | 4906.5 records/s | 4870.3 records/s | 4825.3 records/s |
| Physical throughput | 22.3 MiB/s | - | - | 3.3 MiB/s | 3.3 MiB/s | 3.9 MiB/s |
| Output bytes | 5126950.0 B | - | - | 4958748.0 B | 4958748.0 B | 4958748.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq recursive scalar expression. | yq rejects the catalog jq recursive scalar expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq recursive scalar expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 67.4 ms | - | - | 445.1 ms | 452.9 ms | 451.2 ms |
| Wall time dispersion | 0.9 ms / 69.0 ms / 65.8-69.0 ms | - | - | 8.1 ms / 459.7 ms / 427.4-459.7 ms | 3.8 ms / 464.3 ms / 434.7-464.3 ms | 3.4 ms / 471.6 ms / 434.5-471.6 ms |
| First output | 17.6 ms | - | - | 49.3 ms | 55.6 ms | 52.4 ms |
| First output dispersion | 0.6 ms / 18.5 ms / 16.5-18.5 ms | - | - | 0.7 ms / 51.3 ms / 46.9-51.3 ms | 1.2 ms / 58.6 ms / 52.9-58.6 ms | 1.3 ms / 54.5 ms / 50.2-54.5 ms |
| User CPU | 60.1 ms | - | - | 175.5 ms | 172.4 ms | 172.1 ms |
| System CPU | 6.9 ms | - | - | 240.2 ms | 246.5 ms | 248.2 ms |
| Peak RSS | 15.4 MiB | - | - | 20.0 MiB | 24.1 MiB | 21.6 MiB |
| Logical throughput | 33016.0 records/s | - | - | 4998.3 records/s | 4912.7 records/s | 4931.5 records/s |
| Physical throughput | 22.4 MiB/s | - | - | 3.4 MiB/s | 3.3 MiB/s | 4.0 MiB/s |
| Output bytes | 1011185.0 B | - | - | 977853.0 B | 977853.0 B | 977853.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq recursive scalar expression. | yq rejects the catalog jq recursive scalar expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
