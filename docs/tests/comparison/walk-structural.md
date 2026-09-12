---
type: Report
title: "Structural walking"
description: "Measures a post-order walk that increments every numeric value."
workload: benchmark.walk-structural
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Structural walking

## What this measures

The `walk` query visits the whole document and adds one to every value whose
type is `number`. It combines recursive traversal, type checks, and rebuilding
the original structure.

## Why it matters

Tree-wide edits support migrations and normalization when the fields are not
known in advance. The result must preserve shape while changing selected
leaves.

## Input and output

The input is a natural nested snapshot. The output is one complete document
with every numeric leaf incremented; strings, booleans, arrays, and object keys
keep their roles.

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

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq walk expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 16.9 ms | - | - | 13.9 ms | 14.8 ms | 14.2 ms |
| Wall time dispersion | 0.3 ms / 17.9 ms / 16.3-17.9 ms | - | - | 0.4 ms / 14.5 ms / 13.3-14.5 ms | 0.2 ms / 15.1 ms / 13.7-15.1 ms | 0.2 ms / 14.7 ms / 13.9-14.7 ms |
| First output | 14.9 ms | - | - | 12.7 ms | 13.6 ms | 13.0 ms |
| First output dispersion | 0.3 ms / 15.9 ms / 14.3-15.9 ms | - | - | 0.3 ms / 13.2 ms / 12.1-13.2 ms | 0.1 ms / 13.7 ms / 12.7-13.7 ms | 0.2 ms / 13.4 ms / 12.7-13.4 ms |
| User CPU | 15.7 ms | - | - | 11.8 ms | 13.1 ms | 11.9 ms |
| System CPU | 1.0 ms | - | - | 1.6 ms | 1.5 ms | 2.2 ms |
| Peak RSS | 4.9 MiB | - | - | 9.8 MiB | 10.3 MiB | 9.9 MiB |
| Logical throughput | 11469.1 records/s | - | - | 14003.2 records/s | 13104.1 records/s | 13706.9 records/s |
| Physical throughput | 7.9 MiB/s | - | - | 9.6 MiB/s | 9.0 MiB/s | 11.1 MiB/s |
| Output bytes | 213177.0 B | - | - | 165349.0 B | 165349.0 B | 165349.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq walk expression. | yq rejects the catalog jq walk expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq walk expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.7 ms | - | - | 1.5 ms | 1.5 ms | 1.5 ms |
| Wall time dispersion | 0.1 ms / 2.0 ms / 1.6-2.0 ms | - | - | 0.1 ms / 1.8 ms / 1.3-1.8 ms | 0.1 ms / 1.8 ms / 1.3-1.8 ms | 0.1 ms / 1.7 ms / 1.3-1.7 ms |
| First output | 1.6 ms | - | - | 1.3 ms | 1.2 ms | 1.2 ms |
| First output dispersion | 0.1 ms / 1.8 ms / 1.5-1.9 ms | - | - | 0.1 ms / 1.5 ms / 1.1-1.5 ms | 0.1 ms / 1.5 ms / 1.1-1.7 ms | 0.0 ms / 1.5 ms / 1.1-1.5 ms |
| User CPU | 0.8 ms | - | - | 0.7 ms | 0.0 ms | 0.7 ms |
| System CPU | 0.8 ms | - | - | 0.8 ms | 1.3 ms | 0.7 ms |
| Peak RSS | 4.1 MiB | - | - | 8.2 MiB | 8.5 MiB | 8.1 MiB |
| Logical throughput | 1183.4 records/s | - | - | 1352.7 records/s | 1360.5 records/s | 1344.5 records/s |
| Physical throughput | 1.0 MiB/s | - | - | 1.1 MiB/s | 1.2 MiB/s | 1.3 MiB/s |
| Output bytes | 2623.0 B | - | - | 2037.0 B | 2037.0 B | 2037.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq walk expression. | yq rejects the catalog jq walk expression. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq walk expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 940.8 ms | - | - | 690.3 ms | 734.7 ms | 708.0 ms |
| Wall time dispersion | 10.3 ms / 953.6 ms / 901.1-953.6 ms | - | - | 3.7 ms / 694.6 ms / 684.9-694.6 ms | 1.8 ms / 744.1 ms / 730.5-744.1 ms | 6.0 ms / 737.7 ms / 701.0-737.7 ms |
| First output | 810.4 ms | - | - | 600.1 ms | 646.1 ms | 617.7 ms |
| First output dispersion | 13.6 ms / 825.3 ms / 775.1-825.3 ms | - | - | 3.8 ms / 604.2 ms / 593.9-604.2 ms | 1.6 ms / 654.1 ms / 642.7-654.1 ms | 6.7 ms / 646.2 ms / 609.8-646.2 ms |
| User CPU | 898.2 ms | - | - | 635.9 ms | 672.3 ms | 652.6 ms |
| System CPU | 40.9 ms | - | - | 52.9 ms | 61.8 ms | 57.8 ms |
| Peak RSS | 86.5 MiB | - | - | 107.4 MiB | 127.3 MiB | 116.2 MiB |
| Logical throughput | 11983.3 records/s | - | - | 16331.7 records/s | 15345.8 records/s | 15922.7 records/s |
| Physical throughput | 8.1 MiB/s | - | - | 11.1 MiB/s | 10.4 MiB/s | 12.8 MiB/s |
| Output bytes | 12307507.0 B | - | - | 9534923.0 B | 9534923.0 B | 9534923.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq walk expression. | yq rejects the catalog jq walk expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq walk expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 177.8 ms | - | - | 135.4 ms | 140.9 ms | 138.7 ms |
| Wall time dispersion | 0.7 ms / 183.1 ms / 175.4-183.1 ms | - | - | 3.1 ms / 140.6 ms / 131.3-140.6 ms | 1.1 ms / 142.5 ms / 137.3-142.5 ms | 3.1 ms / 143.3 ms / 134.7-143.3 ms |
| First output | 154.8 ms | - | - | 120.9 ms | 126.7 ms | 123.9 ms |
| First output dispersion | 0.8 ms / 159.8 ms / 152.8-159.8 ms | - | - | 1.4 ms / 124.2 ms / 117.8-124.2 ms | 1.0 ms / 129.5 ms / 124.0-129.5 ms | 1.7 ms / 126.7 ms / 121.3-126.7 ms |
| User CPU | 168.6 ms | - | - | 122.9 ms | 129.1 ms | 125.7 ms |
| System CPU | 8.5 ms | - | - | 12.5 ms | 12.1 ms | 11.0 ms |
| Peak RSS | 19.8 MiB | - | - | 27.9 MiB | 31.7 MiB | 29.2 MiB |
| Logical throughput | 12517.4 records/s | - | - | 16432.1 records/s | 15791.0 records/s | 16044.6 records/s |
| Physical throughput | 8.5 MiB/s | - | - | 11.1 MiB/s | 10.7 MiB/s | 12.9 MiB/s |
| Output bytes | 2427690.0 B | - | - | 1880294.0 B | 1880294.0 B | 1880294.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq walk expression. | yq rejects the catalog jq walk expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
