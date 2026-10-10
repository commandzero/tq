---
type: Report
title: Enumerating paths
description: Measures listing all paths inside the metadata object.
workload: benchmark.issue5-paths
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

# Enumerating paths

## What this measures

The query enumerates `paths` below `.metadata` and counts them. It traverses
the metadata tree and materializes the path list before producing its length.

## Why it matters

Path inventories help schema tools, redactors, and diagnostics understand an
object without knowing its fields ahead of time.

## Input and output

The input is the metadata object from a natural snapshot. The output is one
integer containing the number of discovered paths, not the path arrays.

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

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq paths expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 3.0 ms | - | - | 3.5 ms | 4.4 ms | 4.9 ms |
| Wall time dispersion | 0.0 ms / 3.2 ms / 3.0-3.2 ms | - | - | 0.1 ms / 3.6 ms / 3.4-3.6 ms | 0.2 ms / 4.9 ms / 4.3-4.9 ms | 0.4 ms / 5.3 ms / 4.5-5.3 ms |
| First output | 3.0 ms | - | - | 3.3 ms | 4.3 ms | 4.9 ms |
| First output dispersion | 0.0 ms / 3.2 ms / 3.0-3.2 ms | - | - | 0.0 ms / 3.3 ms / 3.3-3.3 ms | 0.1 ms / 4.4 ms / 4.3-4.4 ms | 0.4 ms / 5.3 ms / 4.4-5.3 ms |
| User CPU | 2.9 ms | - | - | 2.5 ms | 2.6 ms | 3.3 ms |
| System CPU | 0.0 ms | - | - | 0.9 ms | 1.7 ms | 1.1 ms |
| Peak RSS | 3.4 MiB | - | - | 15.1 MiB | 17.9 MiB | 17.5 MiB |
| Logical throughput | 81036.2 records/s | - | - | 69754.1 records/s | 54917.8 records/s | 49816.3 records/s |
| Physical throughput | 55.2 MiB/s | - | - | 47.5 MiB/s | 37.4 MiB/s | 40.2 MiB/s |
| Output bytes | 2.0 B | - | - | 2.0 B | 2.0 B | 2.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq paths expression. | yq rejects the catalog jq paths expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq paths expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.2 ms | - | - | 1.9 ms | 2.0 ms | 1.9 ms |
| Wall time dispersion | 0.0 ms / 1.2 ms / 1.1-1.2 ms | - | - | 0.1 ms / 2.2 ms / 1.8-2.2 ms | 0.0 ms / 2.1 ms / 1.9-2.1 ms | 0.0 ms / 1.9 ms / 1.9-1.9 ms |
| First output | 1.2 ms | - | - | 1.9 ms | 2.0 ms | 1.9 ms |
| First output dispersion | 0.0 ms / 1.2 ms / 1.1-1.2 ms | - | - | 0.1 ms / 2.2 ms / 1.8-2.2 ms | 0.0 ms / 2.1 ms / 1.9-2.1 ms | 0.0 ms / 1.9 ms / 1.9-1.9 ms |
| User CPU | 1.1 ms | - | - | 1.2 ms | 0.0 ms | 0.9 ms |
| System CPU | 0.0 ms | - | - | 0.9 ms | 1.8 ms | 0.9 ms |
| Peak RSS | 2.7 MiB | - | - | 15.2 MiB | 15.6 MiB | 15.5 MiB |
| Logical throughput | 6044.9 records/s | - | - | 3597.1 records/s | 3576.9 records/s | 3595.3 records/s |
| Physical throughput | 4.4 MiB/s | - | - | 2.6 MiB/s | 2.6 MiB/s | 3.1 MiB/s |
| Output bytes | 2.0 B | - | - | 2.0 B | 2.0 B | 2.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq paths expression. | yq rejects the catalog jq paths expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq paths expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 105.8 ms | - | - | 73.5 ms | 117.6 ms | 123.2 ms |
| Wall time dispersion | 1.7 ms / 107.7 ms / 104.1-107.7 ms | - | - | 0.2 ms / 74.1 ms / 73.3-74.1 ms | 0.6 ms / 118.3 ms / 115.9-118.3 ms | 1.4 ms / 126.3 ms / 121.8-126.3 ms |
| First output | 103.2 ms | - | - | 73.5 ms | 116.8 ms | 122.3 ms |
| First output dispersion | 1.2 ms / 104.5 ms / 101.2-104.5 ms | - | - | 0.5 ms / 73.9 ms / 72.8-73.9 ms | 0.2 ms / 117.0 ms / 115.9-117.0 ms | 1.2 ms / 125.4 ms / 121.1-125.4 ms |
| User CPU | 80.3 ms | - | - | 66.8 ms | 110.5 ms | 116.7 ms |
| System CPU | 25.1 ms | - | - | 6.9 ms | 5.0 ms | 5.0 ms |
| Peak RSS | 56.6 MiB | - | - | 73.2 MiB | 117.8 MiB | 97.4 MiB |
| Logical throughput | 102020.2 records/s | - | - | 146921.9 records/s | 91749.2 records/s | 87605.9 records/s |
| Physical throughput | 69.1 MiB/s | - | - | 99.5 MiB/s | 62.1 MiB/s | 70.3 MiB/s |
| Output bytes | 2.0 B | - | - | 2.0 B | 2.0 B | 2.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq paths expression. | yq rejects the catalog jq paths expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq paths expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 19.8 ms | - | - | 16.8 ms | 26.1 ms | 27.1 ms |
| Wall time dispersion | 0.5 ms / 20.4 ms / 19.3-20.4 ms | - | - | 0.0 ms / 16.9 ms / 16.5-16.9 ms | 0.1 ms / 26.1 ms / 25.2-26.1 ms | 0.1 ms / 27.2 ms / 26.7-27.2 ms |
| First output | 19.8 ms | - | - | 16.8 ms | 26.1 ms | 26.5 ms |
| First output dispersion | 0.3 ms / 20.1 ms / 19.1-20.1 ms | - | - | 0.0 ms / 16.9 ms / 16.5-16.9 ms | 0.1 ms / 26.1 ms / 25.2-26.1 ms | 0.1 ms / 27.1 ms / 26.4-27.1 ms |
| User CPU | 14.8 ms | - | - | 15.6 ms | 23.0 ms | 24.9 ms |
| System CPU | 4.9 ms | - | - | 1.0 ms | 2.1 ms | 2.0 ms |
| Peak RSS | 13.4 MiB | - | - | 29.0 MiB | 34.0 MiB | 33.6 MiB |
| Logical throughput | 112777.0 records/s | - | - | 132739.2 records/s | 85646.4 records/s | 82459.8 records/s |
| Physical throughput | 76.5 MiB/s | - | - | 90.0 MiB/s | 58.0 MiB/s | 66.2 MiB/s |
| Output bytes | 2.0 B | - | - | 2.0 B | 2.0 B | 2.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq paths expression. | yq rejects the catalog jq paths expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
