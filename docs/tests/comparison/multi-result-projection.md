---
type: Report
title: Projecting many results
description: Measures streaming extraction of one field from every feature.
workload: benchmark.multi-result-projection
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

# Projecting many results

## What this measures

The query `.features[].properties.mag` walks the feature collection and emits
each magnitude as soon as the stream reaches it. It exercises repeated field
navigation without a final collection step.

## Why it matters

Streaming projections feed pipes, counters, and line-oriented consumers. Time
to the first value matters here as much as total time.

## Input and output

The input is a natural snapshot containing a `features` array. The output is
one number per feature in input order, as a sequence of values rather than one
array of magnitudes.

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
| Wall time | 2.9 ms | 11.2 ms | 23.1 ms | 4.5 ms | 6.0 ms | 5.4 ms |
| Wall time dispersion | 0.0 ms / 3.2 ms / 2.9-3.2 ms | 0.0 ms / 11.2 ms / 11.0-11.2 ms | 0.1 ms / 23.2 ms / 21.9-23.2 ms | 0.0 ms / 4.5 ms / 4.4-4.5 ms | 0.0 ms / 6.1 ms / 5.9-6.1 ms | 0.1 ms / 5.5 ms / 5.3-5.5 ms |
| First output | 2.9 ms | 9.6 ms | 22.2 ms | 4.4 ms | 6.0 ms | 5.4 ms |
| First output dispersion | 0.0 ms / 3.2 ms / 2.9-3.2 ms | 0.0 ms / 10.6 ms / 9.6-10.6 ms | 0.1 ms / 22.3 ms / 21.1-22.3 ms | 0.0 ms / 4.4 ms / 4.3-4.4 ms | 0.0 ms / 6.1 ms / 5.9-6.1 ms | 0.1 ms / 5.5 ms / 5.3-5.5 ms |
| User CPU | 2.0 ms | 7.8 ms | 19.1 ms | 3.2 ms | 3.0 ms | 4.1 ms |
| System CPU | 1.0 ms | 6.9 ms | 14.6 ms | 1.1 ms | 2.9 ms | 1.2 ms |
| Peak RSS | 3.3 MiB | 21.7 MiB | 36.9 MiB | 15.0 MiB | 17.7 MiB | 15.2 MiB |
| Logical throughput | 84869.6 records/s | 21834.5 records/s | 10581.6 records/s | 54078.0 records/s | 40877.9 records/s | 45590.4 records/s |
| Physical throughput | 57.8 MiB/s | 14.9 MiB/s | 7.2 MiB/s | 36.9 MiB/s | 27.8 MiB/s | 36.8 MiB/s |
| Output bytes | 1081.0 B | 1081.0 B | 1081.0 B | 1081.0 B | 1081.0 B | 1081.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.2 ms | 3.0 ms | 3.2 ms | 1.8 ms | 2.1 ms | 1.8 ms |
| Wall time dispersion | 0.0 ms / 1.2 ms / 1.1-1.2 ms | 0.0 ms / 3.3 ms / 3.0-3.3 ms | 0.0 ms / 3.2 ms / 3.1-3.2 ms | 0.0 ms / 1.8 ms / 1.6-1.8 ms | 0.2 ms / 2.3 ms / 2.0-2.3 ms | 0.0 ms / 2.1 ms / 1.8-2.1 ms |
| First output | 1.2 ms | 3.0 ms | 3.2 ms | 1.8 ms | 2.1 ms | 1.8 ms |
| First output dispersion | 0.0 ms / 1.2 ms / 1.1-1.2 ms | 0.0 ms / 3.3 ms / 3.0-3.3 ms | 0.0 ms / 3.2 ms / 3.1-3.2 ms | 0.0 ms / 1.8 ms / 1.6-1.8 ms | 0.2 ms / 2.3 ms / 2.0-2.3 ms | 0.0 ms / 2.1 ms / 1.8-2.1 ms |
| User CPU | 1.1 ms | 1.0 ms | 0.0 ms | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.0 ms | 2.0 ms | 3.2 ms | 1.6 ms | 1.9 ms | 1.7 ms |
| Peak RSS | 2.7 MiB | 13.3 MiB | 13.8 MiB | 15.2 MiB | 15.9 MiB | 15.2 MiB |
| Logical throughput | 5962.5 records/s | 2307.2 records/s | 2215.2 records/s | 3930.4 records/s | 3319.1 records/s | 3831.4 records/s |
| Physical throughput | 4.3 MiB/s | 1.7 MiB/s | 1.6 MiB/s | 2.8 MiB/s | 2.4 MiB/s | 3.3 MiB/s |
| Output bytes | 34.0 B | 34.0 B | 34.0 B | 34.0 B | 34.0 B | 34.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 95.9 ms | 351.2 ms | 922.8 ms | 111.3 ms | 182.8 ms | 140.5 ms |
| Wall time dispersion | 0.5 ms / 96.4 ms / 94.0-96.4 ms | 1.0 ms / 352.3 ms / 349.5-352.3 ms | 14.8 ms / 937.6 ms / 899.7-937.6 ms | 1.0 ms / 112.3 ms / 108.2-112.3 ms | 0.2 ms / 183.0 ms / 179.3-183.0 ms | 0.2 ms / 140.7 ms / 135.9-140.7 ms |
| First output | 79.1 ms | 265.7 ms | 796.9 ms | 111.3 ms | 181.2 ms | 140.5 ms |
| First output dispersion | 0.1 ms / 80.2 ms / 79.0-80.2 ms | 2.2 ms / 267.9 ms / 263.5-267.9 ms | 9.9 ms / 806.8 ms / 781.7-806.8 ms | 1.0 ms / 112.3 ms / 108.2-112.3 ms | 1.0 ms / 182.2 ms / 179.3-182.2 ms | 0.2 ms / 140.7 ms / 135.9-140.7 ms |
| User CPU | 67.6 ms | 372.6 ms | 1240.7 ms | 107.9 ms | 143.2 ms | 137.2 ms |
| System CPU | 26.8 ms | 135.0 ms | 443.3 ms | 1.1 ms | 33.5 ms | 3.0 ms |
| Peak RSS | 56.9 MiB | 327.2 MiB | 1015.0 MiB | 17.1 MiB | 117.7 MiB | 15.1 MiB |
| Logical throughput | 112489.3 records/s | 30725.0 records/s | 11694.8 records/s | 96956.2 records/s | 59050.1 records/s | 76797.7 records/s |
| Physical throughput | 76.2 MiB/s | 20.8 MiB/s | 7.9 MiB/s | 65.7 MiB/s | 39.9 MiB/s | 61.6 MiB/s |
| Output bytes | 48620.0 B | 48620.0 B | 48620.0 B | 48620.0 B | 48620.0 B | 48620.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 19.2 ms | 74.2 ms | 190.6 ms | 24.9 ms | 39.1 ms | 30.6 ms |
| Wall time dispersion | 0.2 ms / 20.3 ms / 19.0-20.3 ms | 0.3 ms / 76.0 ms / 73.9-76.0 ms | 12.0 ms / 202.6 ms / 177.7-202.6 ms | 0.0 ms / 25.0 ms / 24.9-25.0 ms | 0.4 ms / 39.5 ms / 37.9-39.5 ms | 0.3 ms / 31.0 ms / 30.4-31.0 ms |
| First output | 18.0 ms | 57.6 ms | 166.9 ms | 24.9 ms | 39.1 ms | 30.6 ms |
| First output dispersion | 1.0 ms / 19.1 ms / 17.0-19.1 ms | 0.3 ms / 57.9 ms / 57.0-57.9 ms | 6.1 ms / 173.0 ms / 155.0-173.0 ms | 0.0 ms / 25.0 ms / 24.9-25.0 ms | 0.0 ms / 39.1 ms / 37.9-39.1 ms | 0.0 ms / 30.6 ms / 30.4-30.6 ms |
| User CPU | 13.9 ms | 72.7 ms | 219.0 ms | 22.7 ms | 29.3 ms | 28.7 ms |
| System CPU | 4.9 ms | 29.1 ms | 102.0 ms | 2.1 ms | 9.0 ms | 2.0 ms |
| Peak RSS | 13.4 MiB | 86.7 MiB | 236.3 MiB | 15.2 MiB | 33.7 MiB | 15.4 MiB |
| Logical throughput | 116233.1 records/s | 30125.7 records/s | 11719.4 records/s | 89754.9 records/s | 57115.1 records/s | 72949.3 records/s |
| Physical throughput | 78.8 MiB/s | 20.4 MiB/s | 7.9 MiB/s | 60.9 MiB/s | 38.7 MiB/s | 58.6 MiB/s |
| Output bytes | 10187.0 B | 10187.0 B | 10187.0 B | 10187.0 B | 10187.0 B | 10187.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
