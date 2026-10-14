---
type: Report
title: Grouping a collection
description: Measures grouping features by magnitude and counting the groups.
workload: benchmark.issue5-collection
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

# Grouping a collection

## What this measures

The query groups the feature collection by `.properties.mag` and returns the
number of groups. It exercises sorting and collection grouping even though the
final result is only a count.

## Why it matters

Grouping supports summaries such as counts by severity or category. It must
keep enough information to form complete groups before reporting the count.

## Input and output

The input is a natural feature snapshot. The output is one integer for the
number of distinct magnitude groups, not the grouped arrays themselves.

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
| Wall time | 3.3 ms | 15.8 ms | 26.4 ms | 3.8 ms | 4.9 ms | 5.4 ms |
| Wall time dispersion | 0.0 ms / 3.5 ms / 3.3-3.5 ms | 0.1 ms / 15.9 ms / 15.6-15.9 ms | 0.3 ms / 26.7 ms / 25.3-26.7 ms | 0.0 ms / 3.9 ms / 3.7-3.9 ms | 0.1 ms / 5.0 ms / 4.7-5.0 ms | 0.0 ms / 5.5 ms / 5.4-5.5 ms |
| First output | 3.3 ms | 15.8 ms | 26.4 ms | 3.8 ms | 4.9 ms | 5.4 ms |
| First output dispersion | 0.0 ms / 3.5 ms / 3.3-3.5 ms | 0.1 ms / 15.9 ms / 15.6-15.9 ms | 0.3 ms / 26.7 ms / 25.3-26.7 ms | 0.0 ms / 3.9 ms / 3.7-3.9 ms | 0.1 ms / 5.0 ms / 4.4-5.0 ms | 0.0 ms / 5.4 ms / 5.4-5.4 ms |
| User CPU | 2.1 ms | 17.7 ms | 25.7 ms | 2.7 ms | 2.8 ms | 3.3 ms |
| System CPU | 1.1 ms | 7.7 ms | 14.5 ms | 0.9 ms | 1.9 ms | 2.1 ms |
| Peak RSS | 3.3 MiB | 28.7 MiB | 41.0 MiB | 17.0 MiB | 19.8 MiB | 19.6 MiB |
| Logical throughput | 73141.5 records/s | 15465.6 records/s | 9255.7 records/s | 64635.8 records/s | 50174.8 records/s | 44852.9 records/s |
| Physical throughput | 49.9 MiB/s | 10.5 MiB/s | 6.3 MiB/s | 44.1 MiB/s | 34.2 MiB/s | 36.2 MiB/s |
| Output bytes | 4.0 B | 4.0 B | 4.0 B | 4.0 B | 4.0 B | 4.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.2 ms | 3.2 ms | 3.4 ms | 1.9 ms | 1.9 ms | 2.0 ms |
| Wall time dispersion | 0.0 ms / 1.2 ms / 1.2-1.2 ms | 0.0 ms / 3.2 ms / 3.1-3.2 ms | 0.1 ms / 3.5 ms / 3.3-3.5 ms | 0.0 ms / 2.0 ms / 1.6-2.0 ms | 0.0 ms / 1.9 ms / 1.8-1.9 ms | 0.0 ms / 2.2 ms / 1.9-2.2 ms |
| First output | 1.2 ms | 3.2 ms | 3.3 ms | 1.9 ms | 1.9 ms | 2.0 ms |
| First output dispersion | 0.0 ms / 1.2 ms / 1.2-1.2 ms | 0.0 ms / 3.2 ms / 3.1-3.2 ms | 0.0 ms / 3.3 ms / 3.3-3.3 ms | 0.0 ms / 2.0 ms / 1.6-2.0 ms | 0.0 ms / 1.9 ms / 1.8-1.9 ms | 0.0 ms / 2.2 ms / 1.9-2.2 ms |
| User CPU | 0.0 ms | 0.0 ms | 1.2 ms | 0.0 ms | 0.0 ms | 0.9 ms |
| System CPU | 1.1 ms | 3.2 ms | 2.3 ms | 1.6 ms | 1.8 ms | 0.9 ms |
| Peak RSS | 2.7 MiB | 13.7 MiB | 14.1 MiB | 15.1 MiB | 15.9 MiB | 15.5 MiB |
| Logical throughput | 5972.7 records/s | 2220.1 records/s | 2037.8 records/s | 3626.9 records/s | 3657.3 records/s | 3560.5 records/s |
| Physical throughput | 4.3 MiB/s | 1.6 MiB/s | 1.5 MiB/s | 2.6 MiB/s | 2.6 MiB/s | 3.0 MiB/s |
| Output bytes | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 122.8 ms | 572.8 ms | 1075.0 ms | 90.4 ms | 133.1 ms | 140.5 ms |
| Wall time dispersion | 0.9 ms / 125.3 ms / 121.9-125.3 ms | 0.7 ms / 578.0 ms / 572.0-578.0 ms | 10.4 ms / 1085.4 ms / 1059.8-1085.4 ms | 0.9 ms / 94.9 ms / 89.5-94.9 ms | 0.7 ms / 134.1 ms / 132.4-134.1 ms | 0.1 ms / 141.2 ms / 140.4-141.2 ms |
| First output | 120.2 ms | 530.6 ms | 988.6 ms | 90.4 ms | 132.8 ms | 140.0 ms |
| First output dispersion | 1.3 ms / 122.2 ms / 118.9-122.2 ms | 5.6 ms / 539.2 ms / 525.0-539.2 ms | 7.3 ms / 1001.3 ms / 981.3-1001.3 ms | 0.9 ms / 94.9 ms / 89.5-94.9 ms | 1.1 ms / 133.9 ms / 131.7-133.9 ms | 0.2 ms / 141.2 ms / 139.8-141.2 ms |
| User CPU | 98.4 ms | 741.6 ms | 1440.7 ms | 85.2 ms | 125.9 ms | 133.9 ms |
| System CPU | 23.9 ms | 245.3 ms | 520.4 ms | 4.0 ms | 6.0 ms | 5.9 ms |
| Peak RSS | 57.7 MiB | 599.1 MiB | 1185.2 MiB | 81.5 MiB | 117.7 MiB | 99.6 MiB |
| Logical throughput | 87874.9 records/s | 18841.6 records/s | 10039.3 records/s | 119344.9 records/s | 81062.4 records/s | 76814.7 records/s |
| Physical throughput | 59.5 MiB/s | 12.8 MiB/s | 6.8 MiB/s | 80.8 MiB/s | 54.8 MiB/s | 61.6 MiB/s |
| Output bytes | 4.0 B | 4.0 B | 4.0 B | 4.0 B | 4.0 B | 4.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 22.8 ms | 117.1 ms | 221.9 ms | 20.6 ms | 28.4 ms | 30.2 ms |
| Wall time dispersion | 0.4 ms / 23.2 ms / 22.4-23.2 ms | 1.8 ms / 121.9 ms / 115.3-121.9 ms | 1.6 ms / 223.5 ms / 220.2-223.5 ms | 0.1 ms / 21.2 ms / 20.5-21.2 ms | 0.3 ms / 28.7 ms / 27.6-28.7 ms | 0.6 ms / 31.2 ms / 29.6-31.2 ms |
| First output | 22.3 ms | 108.0 ms | 206.3 ms | 20.2 ms | 28.4 ms | 29.6 ms |
| First output dispersion | 0.1 ms / 23.2 ms / 22.2-23.2 ms | 1.1 ms / 112.3 ms / 106.9-112.3 ms | 0.7 ms / 207.0 ms / 204.7-207.0 ms | 0.1 ms / 21.2 ms / 20.1-21.2 ms | 0.1 ms / 28.6 ms / 27.6-28.6 ms | 0.0 ms / 30.7 ms / 29.6-30.7 ms |
| User CPU | 17.4 ms | 147.8 ms | 292.4 ms | 16.5 ms | 26.3 ms | 27.0 ms |
| System CPU | 4.8 ms | 52.4 ms | 102.6 ms | 3.9 ms | 2.2 ms | 3.1 ms |
| Peak RSS | 13.6 MiB | 141.1 MiB | 258.6 MiB | 37.6 MiB | 39.7 MiB | 39.6 MiB |
| Logical throughput | 97875.1 records/s | 19078.5 records/s | 10068.4 records/s | 108673.4 records/s | 78529.2 records/s | 73961.3 records/s |
| Physical throughput | 66.4 MiB/s | 12.9 MiB/s | 6.8 MiB/s | 73.7 MiB/s | 53.2 MiB/s | 59.4 MiB/s |
| Output bytes | 4.0 B | 4.0 B | 4.0 B | 4.0 B | 4.0 B | 4.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
