---
type: Report
title: Blocking sort
description: Measures collecting and sorting all feature magnitudes.
workload: benchmark.blocking-sort
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

# Blocking sort

## What this measures

The query collects `.properties.mag` from every feature and applies `sort`. It
must consume the collection before it can emit the sorted array.

## Why it matters

Ranking and threshold preparation are common jobs where input size affects both
memory and time. This is a clear contrast with streaming projection.

## Input and output

The input is a natural feature snapshot. The output is one array of magnitudes
in ascending order, including the collected values rather than the original
feature objects.

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
| Wall time | 3.1 ms | 10.5 ms | 21.5 ms | 5.0 ms | 4.8 ms | 5.5 ms |
| Wall time dispersion | 0.0 ms / 3.1 ms / 3.0-3.1 ms | 0.1 ms / 10.7 ms / 10.4-10.7 ms | 0.1 ms / 21.5 ms / 20.8-21.5 ms | 0.0 ms / 5.0 ms / 4.7-5.0 ms | 0.0 ms / 4.9 ms / 4.7-4.9 ms | 0.3 ms / 5.9 ms / 5.2-5.9 ms |
| First output | 3.1 ms | 10.5 ms | 21.5 ms | 5.0 ms | 4.8 ms | 5.5 ms |
| First output dispersion | 0.0 ms / 3.1 ms / 3.0-3.1 ms | 0.1 ms / 10.7 ms / 10.4-10.7 ms | 0.1 ms / 21.5 ms / 20.8-21.5 ms | 0.0 ms / 5.0 ms / 4.7-5.0 ms | 0.0 ms / 4.9 ms / 4.7-4.9 ms | 0.3 ms / 5.9 ms / 5.2-5.9 ms |
| User CPU | 2.1 ms | 8.2 ms | 21.0 ms | 4.0 ms | 3.5 ms | 5.4 ms |
| System CPU | 1.0 ms | 6.1 ms | 11.5 ms | 2.6 ms | 1.2 ms | 1.4 ms |
| Peak RSS | 3.3 MiB | 21.0 MiB | 34.6 MiB | 19.7 MiB | 19.6 MiB | 17.8 MiB |
| Logical throughput | 77905.5 records/s | 23289.1 records/s | 11372.6 records/s | 48868.4 records/s | 50496.7 records/s | 44412.1 records/s |
| Physical throughput | 53.1 MiB/s | 15.9 MiB/s | 7.7 MiB/s | 33.3 MiB/s | 34.4 MiB/s | 35.8 MiB/s |
| Output bytes | 1816.0 B | 1083.0 B | 1083.0 B | 1088.0 B | 1088.0 B | 1088.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.2 ms | 3.0 ms | 3.2 ms | 2.4 ms | 2.0 ms | 2.1 ms |
| Wall time dispersion | 0.0 ms / 1.2 ms / 1.2-1.2 ms | 0.0 ms / 3.1 ms / 3.0-3.1 ms | 0.0 ms / 3.3 ms / 3.2-3.3 ms | 0.2 ms / 2.6 ms / 2.3-2.6 ms | 0.2 ms / 2.3 ms / 1.8-2.3 ms | 0.0 ms / 2.5 ms / 2.1-2.5 ms |
| First output | 1.2 ms | 3.0 ms | 3.2 ms | 2.3 ms | 2.0 ms | 2.1 ms |
| First output dispersion | 0.0 ms / 1.2 ms / 1.2-1.2 ms | 0.0 ms / 3.1 ms / 3.0-3.1 ms | 0.0 ms / 3.3 ms / 3.2-3.3 ms | 0.0 ms / 2.6 ms / 2.2-2.6 ms | 0.2 ms / 2.3 ms / 1.8-2.3 ms | 0.0 ms / 2.5 ms / 2.1-2.5 ms |
| User CPU | 1.1 ms | 1.1 ms | 1.6 ms | 0.8 ms | 0.8 ms | 1.8 ms |
| System CPU | 0.0 ms | 2.1 ms | 1.6 ms | 3.2 ms | 0.9 ms | 1.9 ms |
| Peak RSS | 2.7 MiB | 13.3 MiB | 13.8 MiB | 19.9 MiB | 15.7 MiB | 17.9 MiB |
| Logical throughput | 5957.4 records/s | 2314.8 records/s | 2195.0 records/s | 2893.8 records/s | 3571.4 records/s | 3292.6 records/s |
| Physical throughput | 4.3 MiB/s | 1.7 MiB/s | 1.6 MiB/s | 2.1 MiB/s | 2.6 MiB/s | 2.8 MiB/s |
| Output bytes | 58.0 B | 36.0 B | 36.0 B | 39.0 B | 39.0 B | 39.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 120.1 ms | 313.9 ms | 898.3 ms | 115.2 ms | 131.8 ms | 143.3 ms |
| Wall time dispersion | 1.0 ms / 121.1 ms / 116.9-121.1 ms | 1.9 ms / 315.8 ms / 308.7-315.8 ms | 3.4 ms / 901.6 ms / 881.8-901.6 ms | 0.3 ms / 115.5 ms / 114.7-115.5 ms | 0.9 ms / 132.8 ms / 128.5-132.8 ms | 0.7 ms / 145.5 ms / 142.6-145.5 ms |
| First output | 114.9 ms | 290.0 ms | 826.1 ms | 114.9 ms | 131.7 ms | 143.3 ms |
| First output dispersion | 2.1 ms / 116.9 ms / 112.7-116.9 ms | 1.1 ms / 295.3 ms / 288.9-295.3 ms | 1.8 ms / 827.9 ms / 816.4-827.9 ms | 0.1 ms / 115.5 ms / 114.7-115.5 ms | 0.1 ms / 131.8 ms / 128.5-131.8 ms | 1.1 ms / 145.5 ms / 142.2-145.5 ms |
| User CPU | 93.7 ms | 271.8 ms | 1085.2 ms | 112.5 ms | 122.0 ms | 140.6 ms |
| System CPU | 25.9 ms | 130.9 ms | 463.9 ms | 4.0 ms | 6.0 ms | 3.1 ms |
| Peak RSS | 57.3 MiB | 322.4 MiB | 1032.3 MiB | 27.8 MiB | 87.8 MiB | 21.9 MiB |
| Logical throughput | 89836.8 records/s | 34380.0 records/s | 12014.2 records/s | 93716.3 records/s | 81853.1 records/s | 75297.9 records/s |
| Physical throughput | 60.9 MiB/s | 23.3 MiB/s | 8.1 MiB/s | 63.5 MiB/s | 55.4 MiB/s | 60.4 MiB/s |
| Output bytes | 80999.0 B | 48622.0 B | 48622.0 B | 48629.0 B | 48629.0 B | 48629.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 21.5 ms | 67.3 ms | 173.0 ms | 26.6 ms | 28.3 ms | 32.4 ms |
| Wall time dispersion | 0.4 ms / 21.9 ms / 21.0-21.9 ms | 0.2 ms / 67.6 ms / 67.1-67.6 ms | 2.6 ms / 196.0 ms / 170.4-196.0 ms | 0.3 ms / 26.8 ms / 26.2-26.8 ms | 0.2 ms / 28.5 ms / 27.5-28.5 ms | 0.2 ms / 33.1 ms / 32.1-33.1 ms |
| First output | 21.2 ms | 62.3 ms | 159.4 ms | 26.6 ms | 28.3 ms | 32.4 ms |
| First output dispersion | 0.0 ms / 21.2 ms / 21.0-21.2 ms | 0.5 ms / 63.3 ms / 61.8-63.3 ms | 2.4 ms / 178.5 ms / 157.0-178.5 ms | 0.3 ms / 26.8 ms / 26.2-26.8 ms | 0.2 ms / 28.5 ms / 27.5-28.5 ms | 0.4 ms / 32.8 ms / 31.8-32.8 ms |
| User CPU | 16.5 ms | 62.4 ms | 211.6 ms | 26.9 ms | 25.0 ms | 31.5 ms |
| System CPU | 5.2 ms | 26.5 ms | 89.5 ms | 1.1 ms | 3.0 ms | 3.0 ms |
| Peak RSS | 13.4 MiB | 76.5 MiB | 234.3 MiB | 23.7 MiB | 37.5 MiB | 21.9 MiB |
| Logical throughput | 103675.5 records/s | 33195.6 records/s | 12912.5 records/s | 84092.4 records/s | 79015.3 records/s | 69042.2 records/s |
| Physical throughput | 70.3 MiB/s | 22.5 MiB/s | 8.7 MiB/s | 57.0 MiB/s | 53.5 MiB/s | 55.5 MiB/s |
| Output bytes | 16892.0 B | 10189.0 B | 10189.0 B | 10195.0 B | 10195.0 B | 10195.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
