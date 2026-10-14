---
type: Report
title: Recursive scalar traversal
description: Measures depth-first traversal that emits every scalar value.
workload: benchmark.recursive-scalars
generated:
  by: codex/gpt-5.6-luna
  at: 2026-09-10T20:15:01Z
benchmark_runs:
- campaign_id: 2026-09-11
  provenance: Native Linux workload report; captured tool identities are listed in the Results section. No executable hashes or unique campaign ID retained.
  binaries:
    tq:
      version: tq 0.1.0 (TOON v3; jq target 1.8.x; revision a4b4d916-worktree)
      sha256: null
    jq:
      version: jq-1.8.1
      sha256: null
    yq:
      version: yq (https://github.com/mikefarah/yq/) version v4.53.2
      sha256: null
- campaign_id: 2026-10-08T15:10:27.867608118Z
  binaries:
    jq:
      version: jq-1.8.1
      sha256: 020468de7539ce70ef1bceaf7cde2e8c4f2ca6c3afb84642aabc5c97d9fc2a0d
    tq:
      version: tq 0.5.0 (TOON v4.1; jq target 1.8.x; revision unknown)
      sha256: 474625b11f0ec3a1fa125bbeb4e55008efa50ec713b0c398822ada48f0289b91
    yq:
      version: yq (https://github.com/mikefarah/yq/) version v4.53.2
      sha256: d56bf5c6819e8e696340c312bd70f849dc1678a7cda9c2ad63eebd906371d56b
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
Last updated: 2026-10-08
Suite: `natural-corpus` | Profile: `standard` | Campaign status: `observed-failures`

Tools: `jq` (jq-1.8.1), `tq` (tq 0.5.0 (TOON v4.1; jq target 1.8.x; revision unknown)), `yq` (yq (https://github.com/mikefarah/yq/) version v4.53.2)
Environment: `linux` / `x86_64`, AMD Ryzen 7 7700 8-Core Processor, 16 logical CPUs, 61.9 GiB RAM; kernel `Linux 7.2.4-ogc3.1.fc44.x86_64 #1 SMP PREEMPT_DYNAMIC Sun Sep 13 01:41:06 UTC 2026`; compiler profile `release-benchmark`

Peak RSS is authoritative only after the campaign RSS preflight passes. Missing or invalid values are `-`, never estimates. RSS collector provenance (outside measurement tables): `linux-wait4`.
Measurement method (outside measurement tables): `tq-bench` native measurement: RSS scope `wait4 child lifetime including pre exec waited descendants and threads`; residual RSS floor `2.6 MiB` retained, not subtracted; observed control excess `0.9 ms`; primary timing is sampler-free.

Compare columns with the same input format to isolate tool differences. Missing adapters have `-` measurement cells and `not recorded` outcome/detail cells.

Timing rows show numeric medians followed by compact MAD / p95 / range rows, with one decimal place and a unit in each measurement cell. CPU, throughput, and output cells use the captured summary values.

### usgs-all-day

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq recursive scalar expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 8.4 ms | - | - | 46.6 ms | 45.6 ms | 48.7 ms |
| Wall time dispersion | 0.1 ms / 8.5 ms / 8.2-8.5 ms | - | - | 0.3 ms / 46.9 ms / 46.1-46.9 ms | 0.9 ms / 47.3 ms / 44.7-47.3 ms | 1.4 ms / 50.0 ms / 44.0-50.0 ms |
| First output | 3.3 ms | - | - | 28.5 ms | 29.6 ms | 31.7 ms |
| First output dispersion | 0.0 ms / 3.3 ms / 3.3-3.3 ms | - | - | 0.9 ms / 29.4 ms / 27.5-29.4 ms | 1.1 ms / 30.7 ms / 27.5-30.7 ms | 0.0 ms / 31.7 ms / 28.6-31.7 ms |
| User CPU | 7.1 ms | - | - | 17.1 ms | 18.8 ms | 21.9 ms |
| System CPU | 1.0 ms | - | - | 27.8 ms | 27.4 ms | 26.0 ms |
| Peak RSS | 3.4 MiB | - | - | 14.9 MiB | 17.6 MiB | 17.5 MiB |
| Logical throughput | 29040.7 records/s | - | - | 5238.3 records/s | 5350.3 records/s | 5012.3 records/s |
| Physical throughput | 19.8 MiB/s | - | - | 3.6 MiB/s | 3.6 MiB/s | 4.0 MiB/s |
| Output bytes | 111840.0 B | - | - | 108202.0 B | 108202.0 B | 108202.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq recursive scalar expression. | yq rejects the catalog jq recursive scalar expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq recursive scalar expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.3 ms | - | - | 3.2 ms | 3.4 ms | 3.2 ms |
| Wall time dispersion | 0.0 ms / 1.3 ms / 1.3-1.3 ms | - | - | 0.0 ms / 3.3 ms / 3.2-3.3 ms | 0.0 ms / 3.5 ms / 3.3-3.5 ms | 0.2 ms / 3.3 ms / 3.0-3.3 ms |
| First output | 1.3 ms | - | - | 3.2 ms | 3.4 ms | 3.2 ms |
| First output dispersion | 0.0 ms / 1.3 ms / 1.3-1.3 ms | - | - | 0.0 ms / 3.3 ms / 3.2-3.3 ms | 0.0 ms / 3.5 ms / 3.3-3.5 ms | 0.2 ms / 3.3 ms / 3.0-3.3 ms |
| User CPU | 1.2 ms | - | - | 0.0 ms | 0.0 ms | 0.9 ms |
| System CPU | 0.0 ms | - | - | 3.1 ms | 3.3 ms | 2.2 ms |
| Peak RSS | 2.7 MiB | - | - | 15.0 MiB | 15.5 MiB | 15.3 MiB |
| Logical throughput | 5351.7 records/s | - | - | 2187.5 records/s | 2088.3 records/s | 2204.7 records/s |
| Physical throughput | 3.9 MiB/s | - | - | 1.6 MiB/s | 1.5 MiB/s | 1.9 MiB/s |
| Output bytes | 3436.0 B | - | - | 3332.0 B | 3332.0 B | 3332.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq recursive scalar expression. | yq rejects the catalog jq recursive scalar expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq recursive scalar expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 315.1 ms | - | - | 2054.1 ms | 2144.2 ms | 2115.5 ms |
| Wall time dispersion | 1.1 ms / 316.2 ms / 311.0-316.2 ms | - | - | 11.3 ms / 2065.4 ms / 2039.4-2065.4 ms | 55.1 ms / 2199.2 ms / 2077.2-2199.2 ms | 47.6 ms / 2163.1 ms / 2062.6-2163.1 ms |
| First output | 79.6 ms | - | - | 95.9 ms | 137.0 ms | 142.3 ms |
| First output dispersion | 0.5 ms / 80.2 ms / 79.1-80.2 ms | - | - | 1.1 ms / 97.0 ms / 93.9-97.0 ms | 0.1 ms / 140.0 ms / 136.9-140.0 ms | 0.0 ms / 143.3 ms / 142.3-143.3 ms |
| User CPU | 286.2 ms | - | - | 777.5 ms | 795.8 ms | 848.2 ms |
| System CPU | 27.9 ms | - | - | 1101.9 ms | 1138.3 ms | 1075.3 ms |
| Peak RSS | 60.9 MiB | - | - | 72.9 MiB | 117.4 MiB | 96.8 MiB |
| Logical throughput | 34246.9 records/s | - | - | 5253.9 records/s | 5033.2 records/s | 5101.5 records/s |
| Physical throughput | 23.2 MiB/s | - | - | 3.6 MiB/s | 3.4 MiB/s | 4.1 MiB/s |
| Output bytes | 4902253.0 B | - | - | 4741433.0 B | 4741433.0 B | 4741433.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq recursive scalar expression. | yq rejects the catalog jq recursive scalar expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq recursive scalar expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 64.4 ms | - | - | 428.0 ms | 429.4 ms | 426.0 ms |
| Wall time dispersion | 0.1 ms / 64.6 ms / 63.5-64.6 ms | - | - | 4.4 ms / 436.2 ms / 423.6-436.2 ms | 4.5 ms / 433.9 ms / 424.9-433.9 ms | 0.3 ms / 438.2 ms / 425.8-438.2 ms |
| First output | 17.0 ms | - | - | 43.0 ms | 49.6 ms | 51.8 ms |
| First output dispersion | 0.0 ms / 18.1 ms / 17.0-18.1 ms | - | - | 0.4 ms / 43.4 ms / 41.1-43.4 ms | 0.0 ms / 49.6 ms / 49.6-49.6 ms | 0.1 ms / 52.8 ms / 51.7-52.8 ms |
| User CPU | 58.2 ms | - | - | 176.3 ms | 180.0 ms | 171.6 ms |
| System CPU | 6.0 ms | - | - | 229.4 ms | 209.0 ms | 219.1 ms |
| Peak RSS | 14.3 MiB | - | - | 29.2 MiB | 33.7 MiB | 33.3 MiB |
| Logical throughput | 34684.6 records/s | - | - | 5219.4 records/s | 5202.6 records/s | 5243.9 records/s |
| Physical throughput | 23.5 MiB/s | - | - | 3.5 MiB/s | 3.5 MiB/s | 4.2 MiB/s |
| Output bytes | 1016544.0 B | - | - | 983474.0 B | 983474.0 B | 983474.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq recursive scalar expression. | yq rejects the catalog jq recursive scalar expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
