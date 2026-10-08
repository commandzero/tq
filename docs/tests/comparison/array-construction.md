---
type: Report
title: Array construction
description: Measures building one array of compact objects from a feature collection.
workload: benchmark.array-construction
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

# Array construction

## What this measures

The query maps every feature to `{id, mag}` and wraps the results in one array.
It measures collection growth and object creation in a blocking pipeline.

## Why it matters

APIs and batch jobs often need a compact array for one downstream request. The
cost includes retaining all projected values until the array is complete.

## Input and output

The input is a natural feature snapshot. The output is one array with one
object per feature, containing only `id` and `mag`, rather than a stream of
individual objects.

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

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 3.4 ms | 15.4 ms | 26.4 ms | 4.0 ms | 4.8 ms | 5.3 ms |
| Wall time dispersion | 0.1 ms / 3.5 ms / 3.1-3.5 ms | 0.3 ms / 15.8 ms / 15.1-15.8 ms | 0.1 ms / 26.8 ms / 26.3-26.8 ms | 0.0 ms / 4.3 ms / 3.9-4.3 ms | 0.0 ms / 5.3 ms / 4.8-5.3 ms | 0.2 ms / 5.5 ms / 4.9-5.5 ms |
| First output | 3.3 ms | 15.4 ms | 26.4 ms | 4.0 ms | 4.8 ms | 5.3 ms |
| First output dispersion | 0.1 ms / 3.4 ms / 3.1-3.4 ms | 0.3 ms / 15.8 ms / 15.1-15.8 ms | 0.1 ms / 26.8 ms / 26.3-26.8 ms | 0.0 ms / 4.3 ms / 3.9-4.3 ms | 0.4 ms / 5.3 ms / 4.4-5.3 ms | 0.2 ms / 5.5 ms / 4.9-5.5 ms |
| User CPU | 3.2 ms | 12.9 ms | 25.4 ms | 2.7 ms | 3.5 ms | 3.2 ms |
| System CPU | 0.0 ms | 9.1 ms | 14.1 ms | 1.3 ms | 1.2 ms | 1.9 ms |
| Peak RSS | 3.4 MiB | 25.4 MiB | 38.0 MiB | 17.0 MiB | 19.7 MiB | 19.5 MiB |
| Logical throughput | 72554.3 records/s | 15843.1 records/s | 9237.2 records/s | 61694.1 records/s | 50319.7 records/s | 45692.9 records/s |
| Physical throughput | 49.5 MiB/s | 10.8 MiB/s | 6.3 MiB/s | 42.1 MiB/s | 34.3 MiB/s | 36.9 MiB/s |
| Output bytes | 12051.0 B | 7658.0 B | 7658.0 B | 4499.0 B | 4499.0 B | 4499.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.3 ms | 3.7 ms | 3.5 ms | 2.0 ms | 1.9 ms | 1.9 ms |
| Wall time dispersion | 0.0 ms / 1.4 ms / 1.3-1.4 ms | 0.1 ms / 3.9 ms / 3.5-3.9 ms | 0.1 ms / 3.6 ms / 3.4-3.6 ms | 0.1 ms / 2.1 ms / 1.9-2.1 ms | 0.0 ms / 2.1 ms / 1.9-2.1 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms |
| First output | 1.3 ms | 3.3 ms | 3.3 ms | 2.0 ms | 1.9 ms | 1.9 ms |
| First output dispersion | 0.0 ms / 1.4 ms / 1.3-1.4 ms | 0.0 ms / 3.9 ms / 3.3-3.9 ms | 0.0 ms / 3.3 ms / 3.3-3.3 ms | 0.1 ms / 2.1 ms / 1.9-2.1 ms | 0.0 ms / 2.1 ms / 1.9-2.1 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms |
| User CPU | 1.3 ms | 0.9 ms | 0.9 ms | 0.8 ms | 0.9 ms | 0.0 ms |
| System CPU | 0.0 ms | 2.8 ms | 2.8 ms | 1.1 ms | 1.0 ms | 1.8 ms |
| Peak RSS | 2.7 MiB | 13.6 MiB | 14.1 MiB | 15.2 MiB | 15.8 MiB | 15.3 MiB |
| Logical throughput | 5200.6 records/s | 1914.1 records/s | 1985.3 records/s | 3576.9 records/s | 3589.7 records/s | 3589.7 records/s |
| Physical throughput | 3.8 MiB/s | 1.4 MiB/s | 1.4 MiB/s | 2.6 MiB/s | 2.6 MiB/s | 3.1 MiB/s |
| Output bytes | 349.0 B | 222.0 B | 222.0 B | 142.0 B | 142.0 B | 142.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 125.5 ms | 552.4 ms | 1085.5 ms | 91.4 ms | 131.8 ms | 143.6 ms |
| Wall time dispersion | 1.1 ms / 129.7 ms / 124.4-129.7 ms | 2.7 ms / 555.9 ms / 549.7-555.9 ms | 7.7 ms / 1093.3 ms / 1032.9-1093.3 ms | 0.0 ms / 91.4 ms / 90.9-91.4 ms | 0.3 ms / 134.5 ms / 131.5-134.5 ms | 2.1 ms / 149.2 ms / 141.5-149.2 ms |
| First output | 113.9 ms | 521.1 ms | 1017.8 ms | 84.4 ms | 124.4 ms | 134.8 ms |
| First output dispersion | 2.1 ms / 117.0 ms / 111.8-117.0 ms | 0.5 ms / 521.6 ms / 519.5-521.6 ms | 0.5 ms / 1018.2 ms / 967.6-1018.2 ms | 0.0 ms / 84.4 ms / 83.3-84.4 ms | 0.0 ms / 127.5 ms / 124.4-127.5 ms | 3.1 ms / 140.0 ms / 131.7-140.0 ms |
| User CPU | 92.0 ms | 628.9 ms | 1520.5 ms | 85.4 ms | 126.5 ms | 134.7 ms |
| System CPU | 29.4 ms | 177.5 ms | 461.9 ms | 5.0 ms | 7.0 ms | 6.0 ms |
| Peak RSS | 60.8 MiB | 454.9 MiB | 1040.3 MiB | 78.8 MiB | 117.7 MiB | 99.5 MiB |
| Logical throughput | 86020.8 records/s | 19536.1 records/s | 9941.7 records/s | 118136.4 records/s | 81869.2 records/s | 75128.1 records/s |
| Physical throughput | 58.3 MiB/s | 13.2 MiB/s | 6.7 MiB/s | 80.0 MiB/s | 55.4 MiB/s | 60.3 MiB/s |
| Output bytes | 532399.0 B | 338142.0 B | 338142.0 B | 197861.0 B | 197861.0 B | 197861.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 22.3 ms | 114.9 ms | 232.4 ms | 20.2 ms | 28.1 ms | 29.7 ms |
| Wall time dispersion | 0.1 ms / 24.6 ms / 22.2-24.6 ms | 0.9 ms / 116.0 ms / 114.1-116.0 ms | 2.7 ms / 235.0 ms / 228.9-235.0 ms | 0.0 ms / 21.3 ms / 20.2-21.3 ms | 0.1 ms / 28.2 ms / 27.4-28.2 ms | 0.1 ms / 29.9 ms / 29.2-29.9 ms |
| First output | 21.2 ms | 107.1 ms | 215.5 ms | 20.2 ms | 28.1 ms | 29.6 ms |
| First output dispersion | 0.0 ms / 23.3 ms / 21.2-23.3 ms | 0.6 ms / 109.5 ms / 106.5-109.5 ms | 1.7 ms / 217.2 ms / 211.8-217.2 ms | 0.0 ms / 21.3 ms / 20.2-21.3 ms | 0.1 ms / 28.2 ms / 27.4-28.2 ms | 0.1 ms / 29.7 ms / 29.2-29.7 ms |
| User CPU | 18.0 ms | 120.5 ms | 302.3 ms | 18.0 ms | 24.1 ms | 26.5 ms |
| System CPU | 5.8 ms | 45.6 ms | 103.8 ms | 2.0 ms | 3.1 ms | 2.9 ms |
| Peak RSS | 14.3 MiB | 107.1 MiB | 238.9 MiB | 33.1 MiB | 37.6 MiB | 37.6 MiB |
| Logical throughput | 100058.2 records/s | 19436.7 records/s | 9614.5 records/s | 110824.5 records/s | 79363.4 records/s | 75107.6 records/s |
| Physical throughput | 67.9 MiB/s | 13.2 MiB/s | 6.5 MiB/s | 75.2 MiB/s | 53.7 MiB/s | 60.3 MiB/s |
| Output bytes | 110256.0 B | 70043.0 B | 70043.0 B | 41015.0 B | 41015.0 B | 41015.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
