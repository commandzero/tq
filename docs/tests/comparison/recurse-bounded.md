---
type: Report
title: "Bounded recursion"
description: "Measures recursive traversal with a fixed output limit."
workload: benchmark.recurse-bounded
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Bounded recursion

## What this measures

The query recursively visits child values with `recurse(.[]?)` and keeps at
most 2,048 results. It tests traversal while putting a clear bound on work and
output.

## Why it matters

Recursive queries are useful for irregular documents, but an unbounded walk can
run away on large or cyclic-looking data. A fixed limit is a practical guard.

## Input and output

The input is a natural nested snapshot. The output is the depth-first sequence
of visited values up to the limit, including containers and scalars as the
recursion produces them.

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

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq recurse expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 8.3 ms | - | - | 17.8 ms | 18.3 ms | 17.7 ms |
| Wall time dispersion | 0.3 ms / 8.6 ms / 7.9-8.6 ms | - | - | 0.3 ms / 18.9 ms / 16.7-18.9 ms | 0.3 ms / 19.4 ms / 17.4-19.4 ms | 0.4 ms / 18.6 ms / 17.0-18.6 ms |
| First output | 2.7 ms | - | - | 3.4 ms | 3.9 ms | 3.5 ms |
| First output dispersion | 0.1 ms / 3.1 ms / 2.4-3.1 ms | - | - | 0.1 ms / 3.6 ms / 3.1-3.6 ms | 0.0 ms / 4.4 ms / 3.9-4.4 ms | 0.1 ms / 3.9 ms / 3.4-3.9 ms |
| User CPU | 7.2 ms | - | - | 8.4 ms | 10.6 ms | 10.3 ms |
| System CPU | 1.0 ms | - | - | 9.0 ms | 8.3 ms | 7.8 ms |
| Peak RSS | 4.5 MiB | - | - | 9.1 MiB | 9.7 MiB | 9.1 MiB |
| Logical throughput | 23370.7 records/s | - | - | 10874.7 records/s | 10599.1 records/s | 10972.5 records/s |
| Physical throughput | 16.0 MiB/s | - | - | 7.4 MiB/s | 7.2 MiB/s | 8.9 MiB/s |
| Output bytes | 536652.0 B | - | - | 433312.0 B | 433312.0 B | 433312.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq recurse expression. | yq rejects the catalog jq recurse expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq recurse expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.7 ms | - | - | 1.9 ms | 2.0 ms | 1.8 ms |
| Wall time dispersion | 0.2 ms / 2.0 ms / 1.5-2.0 ms | - | - | 0.1 ms / 2.1 ms / 1.8-2.2 ms | 0.1 ms / 2.1 ms / 1.8-2.2 ms | 0.0 ms / 2.3 ms / 1.8-2.3 ms |
| First output | 1.4 ms | - | - | 1.7 ms | 1.8 ms | 1.7 ms |
| First output dispersion | 0.0 ms / 1.8 ms / 1.3-1.8 ms | - | - | 0.1 ms / 2.0 ms / 1.5-2.0 ms | 0.1 ms / 2.0 ms / 1.6-2.1 ms | 0.1 ms / 2.1 ms / 1.5-2.1 ms |
| User CPU | 1.2 ms | - | - | 0.0 ms | 0.4 ms | 0.0 ms |
| System CPU | 0.4 ms | - | - | 1.8 ms | 1.4 ms | 1.7 ms |
| Peak RSS | 4.1 MiB | - | - | 8.2 MiB | 8.4 MiB | 7.9 MiB |
| Logical throughput | 1206.6 records/s | - | - | 1026.2 records/s | 1020.9 records/s | 1095.0 records/s |
| Physical throughput | 1.0 MiB/s | - | - | 0.9 MiB/s | 0.9 MiB/s | 1.1 MiB/s |
| Output bytes | 9719.0 B | - | - | 7938.0 B | 7938.0 B | 7938.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq recurse expression. | yq rejects the catalog jq recurse expression. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq recurse expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 284.7 ms | - | - | 226.8 ms | 274.3 ms | 247.5 ms |
| Wall time dispersion | 0.7 ms / 291.9 ms / 282.6-291.9 ms | - | - | 2.0 ms / 231.6 ms / 224.1-231.6 ms | 2.0 ms / 280.6 ms / 269.4-280.6 ms | 2.4 ms / 253.5 ms / 243.3-253.5 ms |
| First output | 79.5 ms | - | - | 102.7 ms | 145.8 ms | 120.2 ms |
| First output dispersion | 0.4 ms / 81.3 ms / 78.7-81.3 ms | - | - | 0.6 ms / 105.1 ms / 101.6-105.1 ms | 1.3 ms / 148.7 ms / 143.8-148.7 ms | 0.6 ms / 124.3 ms / 117.3-124.3 ms |
| User CPU | 244.8 ms | - | - | 189.3 ms | 229.9 ms | 203.4 ms |
| System CPU | 39.9 ms | - | - | 37.6 ms | 43.7 ms | 43.9 ms |
| Peak RSS | 64.9 MiB | - | - | 69.4 MiB | 89.3 MiB | 78.1 MiB |
| Logical throughput | 39601.7 records/s | - | - | 49708.9 records/s | 41105.8 records/s | 45557.1 records/s |
| Physical throughput | 26.8 MiB/s | - | - | 33.7 MiB/s | 27.8 MiB/s | 36.6 MiB/s |
| Output bytes | 23752426.0 B | - | - | 19085974.0 B | 19085974.0 B | 19085974.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq recurse expression. | yq rejects the catalog jq recurse expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq recurse expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 58.4 ms | - | - | 54.3 ms | 61.8 ms | 58.4 ms |
| Wall time dispersion | 0.8 ms / 61.2 ms / 57.6-61.2 ms | - | - | 0.8 ms / 55.4 ms / 52.8-55.4 ms | 0.5 ms / 63.0 ms / 60.4-63.0 ms | 0.7 ms / 59.6 ms / 56.6-59.6 ms |
| First output | 17.7 ms | - | - | 21.8 ms | 29.3 ms | 25.5 ms |
| First output dispersion | 0.5 ms / 18.2 ms / 17.0-18.2 ms | - | - | 0.5 ms / 22.6 ms / 21.1-22.6 ms | 0.3 ms / 30.4 ms / 27.9-30.4 ms | 0.5 ms / 26.8 ms / 24.8-26.8 ms |
| User CPU | 50.4 ms | - | - | 41.2 ms | 45.5 ms | 44.3 ms |
| System CPU | 8.0 ms | - | - | 12.6 ms | 15.3 ms | 14.1 ms |
| Peak RSS | 15.4 MiB | - | - | 20.1 MiB | 24.2 MiB | 21.6 MiB |
| Logical throughput | 38081.1 records/s | - | - | 40973.4 records/s | 36021.9 records/s | 38128.0 records/s |
| Physical throughput | 25.8 MiB/s | - | - | 27.8 MiB/s | 24.4 MiB/s | 30.6 MiB/s |
| Output bytes | 4788780.0 B | - | - | 3848784.0 B | 3848784.0 B | 3848784.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq recurse expression. | yq rejects the catalog jq recurse expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
