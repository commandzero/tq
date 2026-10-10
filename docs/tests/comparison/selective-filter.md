---
type: Report
title: Selective filtering
description: Measures streaming selection and projection of matching features.
workload: benchmark.selective-filter
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

# Selective filtering

## What this measures

The query selects features with magnitude at least 2 and then projects their
IDs. It combines iteration, a numeric predicate, and a downstream field read.

## Why it matters

This is the shape of a useful event or records query: scan a collection, keep a
subset, and pass compact identifiers to the next command.

## Input and output

The input is a natural feature snapshot with numeric `properties.mag` values.
The output is the ordered sequence of IDs for matching features. It is not the
matching feature objects and not a packed array.

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
| Wall time | 2.9 ms | 11.8 ms | 22.4 ms | 6.5 ms | 5.5 ms | 6.3 ms |
| Wall time dispersion | 0.0 ms / 3.1 ms / 2.8-3.1 ms | 0.2 ms / 12.0 ms / 11.3-12.0 ms | 0.0 ms / 22.6 ms / 22.4-22.6 ms | 0.1 ms / 7.5 ms / 6.4-7.5 ms | 0.1 ms / 5.6 ms / 5.3-5.6 ms | 0.0 ms / 6.6 ms / 6.2-6.6 ms |
| First output | 2.9 ms | 11.7 ms | 22.2 ms | 6.5 ms | 5.3 ms | 6.3 ms |
| First output dispersion | 0.0 ms / 3.1 ms / 2.8-3.1 ms | 0.1 ms / 11.8 ms / 11.3-11.8 ms | 0.0 ms / 22.3 ms / 22.2-22.3 ms | 0.1 ms / 7.5 ms / 6.4-7.5 ms | 0.0 ms / 5.4 ms / 5.3-5.4 ms | 0.0 ms / 6.6 ms / 6.2-6.6 ms |
| User CPU | 2.0 ms | 7.6 ms | 20.0 ms | 5.2 ms | 4.1 ms | 3.5 ms |
| System CPU | 0.9 ms | 7.6 ms | 13.1 ms | 1.9 ms | 1.2 ms | 2.6 ms |
| Peak RSS | 3.3 MiB | 22.0 MiB | 36.2 MiB | 15.9 MiB | 17.6 MiB | 15.8 MiB |
| Logical throughput | 85344.5 records/s | 20702.5 records/s | 10881.7 records/s | 37371.7 records/s | 44460.6 records/s | 39015.0 records/s |
| Physical throughput | 58.2 MiB/s | 14.1 MiB/s | 7.4 MiB/s | 25.5 MiB/s | 30.3 MiB/s | 31.5 MiB/s |
| Output bytes | 1050.0 B | 1050.0 B | 1050.0 B | 904.0 B | 904.0 B | 904.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.2 ms | 3.0 ms | 3.3 ms | 1.8 ms | 2.1 ms | 2.2 ms |
| Wall time dispersion | 0.0 ms / 1.2 ms / 1.2-1.2 ms | 0.0 ms / 3.2 ms / 3.0-3.2 ms | 0.0 ms / 3.4 ms / 3.3-3.4 ms | 0.0 ms / 1.9 ms / 1.8-1.9 ms | 0.0 ms / 2.1 ms / 2.0-2.1 ms | 0.0 ms / 2.3 ms / 1.9-2.3 ms |
| First output | 1.2 ms | 3.0 ms | 3.3 ms | 1.8 ms | 2.1 ms | 2.2 ms |
| First output dispersion | 0.0 ms / 1.2 ms / 1.2-1.2 ms | 0.0 ms / 3.2 ms / 3.0-3.2 ms | 0.0 ms / 3.4 ms / 3.3-3.4 ms | 0.0 ms / 1.9 ms / 1.8-1.9 ms | 0.0 ms / 2.1 ms / 2.0-2.1 ms | 0.0 ms / 2.3 ms / 1.9-2.3 ms |
| User CPU | 1.1 ms | 1.0 ms | 1.7 ms | 1.7 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.0 ms | 2.1 ms | 1.7 ms | 0.0 ms | 1.8 ms | 1.8 ms |
| Peak RSS | 2.7 MiB | 13.2 MiB | 14.0 MiB | 16.0 MiB | 15.6 MiB | 16.2 MiB |
| Logical throughput | 5972.7 records/s | 2304.1 records/s | 2120.6 records/s | 3867.4 records/s | 3391.5 records/s | 3154.6 records/s |
| Physical throughput | 4.3 MiB/s | 1.7 MiB/s | 1.5 MiB/s | 2.8 MiB/s | 2.5 MiB/s | 2.7 MiB/s |
| Output bytes | 42.0 B | 42.0 B | 42.0 B | 36.0 B | 36.0 B | 36.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 113.1 ms | 350.1 ms | 943.5 ms | 218.9 ms | 146.8 ms | 200.4 ms |
| Wall time dispersion | 0.3 ms / 117.5 ms / 112.7-117.5 ms | 1.9 ms / 356.2 ms / 348.2-356.2 ms | 0.2 ms / 950.3 ms / 943.3-950.3 ms | 2.8 ms / 221.8 ms / 210.4-221.8 ms | 1.6 ms / 148.4 ms / 144.5-148.4 ms | 0.2 ms / 200.6 ms / 197.8-200.6 ms |
| First output | 80.2 ms | 315.4 ms | 852.3 ms | 218.9 ms | 145.4 ms | 200.1 ms |
| First output dispersion | 0.1 ms / 81.2 ms / 80.1-81.2 ms | 1.3 ms / 316.7 ms / 307.3-316.7 ms | 1.8 ms / 854.1 ms / 850.3-854.1 ms | 2.8 ms / 221.8 ms / 210.4-221.8 ms | 1.3 ms / 146.8 ms / 144.1-146.8 ms | 0.2 ms / 200.3 ms / 197.8-200.3 ms |
| User CPU | 90.1 ms | 375.1 ms | 1139.6 ms | 217.4 ms | 132.2 ms | 195.7 ms |
| System CPU | 22.0 ms | 131.3 ms | 477.0 ms | 1.0 ms | 15.0 ms | 2.0 ms |
| Peak RSS | 56.6 MiB | 327.9 MiB | 1033.9 MiB | 18.3 MiB | 117.8 MiB | 15.9 MiB |
| Logical throughput | 95436.9 records/s | 30825.6 records/s | 11438.3 records/s | 49297.7 records/s | 73523.0 records/s | 53846.1 records/s |
| Physical throughput | 64.6 MiB/s | 20.9 MiB/s | 7.7 MiB/s | 33.4 MiB/s | 49.7 MiB/s | 43.2 MiB/s |
| Output bytes | 44409.0 B | 44409.0 B | 44409.0 B | 38129.0 B | 38129.0 B | 38129.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 20.0 ms | 78.7 ms | 190.3 ms | 46.0 ms | 29.9 ms | 44.1 ms |
| Wall time dispersion | 0.1 ms / 20.4 ms / 19.8-20.4 ms | 1.2 ms / 80.0 ms / 77.2-80.0 ms | 3.2 ms / 193.4 ms / 179.7-193.4 ms | 0.5 ms / 47.5 ms / 45.5-47.5 ms | 0.2 ms / 31.0 ms / 29.6-31.0 ms | 0.1 ms / 44.2 ms / 42.5-44.2 ms |
| First output | 18.1 ms | 68.2 ms | 174.2 ms | 46.0 ms | 29.6 ms | 44.1 ms |
| First output dispersion | 0.0 ms / 19.1 ms / 18.1-19.1 ms | 0.5 ms / 71.0 ms / 67.7-71.0 ms | 0.8 ms / 175.0 ms / 165.5-175.0 ms | 0.5 ms / 47.5 ms / 45.5-47.5 ms | 0.0 ms / 30.6 ms / 29.6-30.6 ms | 0.1 ms / 44.2 ms / 42.3-44.2 ms |
| User CPU | 16.3 ms | 76.3 ms | 220.9 ms | 44.2 ms | 25.6 ms | 41.6 ms |
| System CPU | 3.8 ms | 36.3 ms | 85.6 ms | 1.0 ms | 5.0 ms | 1.0 ms |
| Peak RSS | 13.4 MiB | 88.0 MiB | 213.6 MiB | 16.4 MiB | 33.8 MiB | 16.1 MiB |
| Logical throughput | 111890.2 records/s | 28369.3 records/s | 11742.0 records/s | 48540.9 records/s | 74828.3 records/s | 50671.4 records/s |
| Physical throughput | 75.9 MiB/s | 19.2 MiB/s | 8.0 MiB/s | 32.9 MiB/s | 50.7 MiB/s | 40.7 MiB/s |
| Output bytes | 8923.0 B | 8923.0 B | 8923.0 B | 7653.0 B | 7653.0 B | 7653.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
