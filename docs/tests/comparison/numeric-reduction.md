---
type: Report
title: Numeric reduction
description: Measures a blocking sum across all feature magnitudes.
workload: benchmark.numeric-reduction
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

# Numeric reduction

## What this measures

The query reduces non-null feature magnitudes into one sum. It keeps an
accumulator while consuming the complete feature stream, so this is a blocking
reduction rather than an early-result projection.

## Why it matters

Totals, scores, and counters are common data-processing jobs. Their cost grows
with the number of values even when the final output is only one number.

## Input and output

The input is a natural snapshot whose features may have missing magnitudes. The
output is one numeric sum, with null magnitudes omitted. It is not one output
per feature.

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
| Wall time | 3.2 ms | 11.2 ms | 22.3 ms | 3.6 ms | 4.8 ms | 4.9 ms |
| Wall time dispersion | 0.0 ms / 3.2 ms / 3.0-3.2 ms | 0.1 ms / 11.3 ms / 11.1-11.3 ms | 0.1 ms / 22.3 ms / 21.2-22.3 ms | 0.0 ms / 3.9 ms / 3.6-3.9 ms | 0.0 ms / 5.3 ms / 4.7-5.3 ms | 0.1 ms / 5.5 ms / 4.7-5.5 ms |
| First output | 3.2 ms | 11.2 ms | 22.3 ms | 3.3 ms | 4.4 ms | 4.9 ms |
| First output dispersion | 0.0 ms / 3.2 ms / 3.0-3.2 ms | 0.1 ms / 11.3 ms / 11.1-11.3 ms | 0.1 ms / 22.3 ms / 21.2-22.3 ms | 0.0 ms / 3.9 ms / 3.3-3.9 ms | 0.0 ms / 5.3 ms / 4.4-5.3 ms | 0.5 ms / 5.5 ms / 4.4-5.5 ms |
| User CPU | 1.9 ms | 7.8 ms | 22.6 ms | 1.8 ms | 3.1 ms | 3.7 ms |
| System CPU | 0.9 ms | 7.0 ms | 9.8 ms | 1.8 ms | 1.9 ms | 0.9 ms |
| Peak RSS | 3.3 MiB | 21.6 MiB | 35.0 MiB | 15.1 MiB | 17.9 MiB | 17.6 MiB |
| Logical throughput | 77166.4 records/s | 21809.1 records/s | 10963.3 records/s | 67088.3 records/s | 51228.2 records/s | 50236.8 records/s |
| Physical throughput | 52.6 MiB/s | 14.9 MiB/s | 7.5 MiB/s | 45.7 MiB/s | 34.9 MiB/s | 40.5 MiB/s |
| Output bytes | 19.0 B | 19.0 B | 19.0 B | 19.0 B | 19.0 B | 19.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.2 ms | 3.1 ms | 3.6 ms | 1.9 ms | 1.9 ms | 1.9 ms |
| Wall time dispersion | 0.0 ms / 1.2 ms / 1.2-1.2 ms | 0.0 ms / 3.2 ms / 3.1-3.2 ms | 0.1 ms / 3.8 ms / 3.1-3.8 ms | 0.0 ms / 1.9 ms / 1.8-1.9 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms |
| First output | 1.2 ms | 3.1 ms | 3.3 ms | 1.9 ms | 1.9 ms | 1.9 ms |
| First output dispersion | 0.0 ms / 1.2 ms / 1.2-1.2 ms | 0.0 ms / 3.2 ms / 3.1-3.2 ms | 0.0 ms / 3.3 ms / 3.1-3.3 ms | 0.0 ms / 1.9 ms / 1.8-1.9 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms |
| User CPU | 0.0 ms | 1.1 ms | 1.9 ms | 0.9 ms | 0.0 ms | 0.0 ms |
| System CPU | 1.1 ms | 2.1 ms | 1.8 ms | 0.9 ms | 1.8 ms | 1.8 ms |
| Peak RSS | 2.7 MiB | 13.3 MiB | 13.9 MiB | 15.5 MiB | 15.6 MiB | 15.5 MiB |
| Logical throughput | 6008.6 records/s | 2223.6 records/s | 1922.0 records/s | 3638.3 records/s | 3636.4 records/s | 3599.0 records/s |
| Physical throughput | 4.4 MiB/s | 1.6 MiB/s | 1.4 MiB/s | 2.6 MiB/s | 2.6 MiB/s | 3.1 MiB/s |
| Output bytes | 6.0 B | 6.0 B | 6.0 B | 6.0 B | 6.0 B | 6.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 111.2 ms | 328.4 ms | 885.6 ms | 84.0 ms | 126.3 ms | 135.5 ms |
| Wall time dispersion | 1.1 ms / 113.7 ms / 110.1-113.7 ms | 2.0 ms / 330.4 ms / 322.1-330.4 ms | 25.0 ms / 910.6 ms / 810.1-910.6 ms | 0.8 ms / 84.8 ms / 82.8-84.8 ms | 0.0 ms / 126.3 ms / 125.4-126.3 ms | 0.5 ms / 136.0 ms / 134.2-136.0 ms |
| First output | 108.6 ms | 309.5 ms | 812.3 ms | 84.0 ms | 125.4 ms | 134.9 ms |
| First output dispersion | 1.1 ms / 110.6 ms / 107.6-110.6 ms | 1.2 ms / 310.7 ms / 303.9-310.7 ms | 23.2 ms / 835.6 ms / 753.0-835.6 ms | 0.4 ms / 84.3 ms / 82.3-84.3 ms | 0.0 ms / 125.5 ms / 125.4-125.5 ms | 1.1 ms / 136.0 ms / 133.8-136.0 ms |
| User CPU | 85.1 ms | 359.8 ms | 1125.4 ms | 80.5 ms | 117.0 ms | 128.0 ms |
| System CPU | 25.7 ms | 124.6 ms | 431.4 ms | 4.0 ms | 8.1 ms | 6.1 ms |
| Peak RSS | 56.6 MiB | 323.0 MiB | 1032.7 MiB | 72.9 MiB | 117.7 MiB | 97.6 MiB |
| Logical throughput | 97064.3 records/s | 32859.9 records/s | 12185.9 records/s | 128512.9 records/s | 85469.7 records/s | 79674.0 records/s |
| Physical throughput | 65.7 MiB/s | 22.3 MiB/s | 8.2 MiB/s | 87.0 MiB/s | 57.8 MiB/s | 63.9 MiB/s |
| Output bytes | 19.0 B | 19.0 B | 19.0 B | 19.0 B | 19.0 B | 19.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 20.6 ms | 69.5 ms | 190.1 ms | 19.1 ms | 26.6 ms | 28.1 ms |
| Wall time dispersion | 0.4 ms / 21.0 ms / 19.3-21.0 ms | 2.1 ms / 72.7 ms / 67.4-72.7 ms | 3.2 ms / 193.8 ms / 186.9-193.8 ms | 0.3 ms / 19.5 ms / 18.6-19.5 ms | 0.6 ms / 27.4 ms / 26.0-27.4 ms | 0.0 ms / 28.1 ms / 27.6-28.1 ms |
| First output | 20.1 ms | 65.4 ms | 174.1 ms | 19.1 ms | 26.4 ms | 27.6 ms |
| First output dispersion | 0.9 ms / 21.0 ms / 19.1-21.0 ms | 2.0 ms / 67.6 ms / 63.4-67.6 ms | 0.9 ms / 177.9 ms / 173.2-177.9 ms | 0.3 ms / 19.5 ms / 18.6-19.5 ms | 0.0 ms / 26.4 ms / 25.4-26.4 ms | 0.1 ms / 28.1 ms / 27.5-28.1 ms |
| User CPU | 13.2 ms | 57.7 ms | 214.3 ms | 17.0 ms | 23.2 ms | 24.5 ms |
| System CPU | 6.0 ms | 31.9 ms | 101.8 ms | 2.0 ms | 3.9 ms | 3.0 ms |
| Peak RSS | 13.4 MiB | 82.6 MiB | 226.5 MiB | 29.1 MiB | 33.5 MiB | 33.3 MiB |
| Logical throughput | 108647.0 records/s | 32157.8 records/s | 11753.5 records/s | 116664.1 records/s | 83852.6 records/s | 79462.2 records/s |
| Physical throughput | 73.7 MiB/s | 21.8 MiB/s | 8.0 MiB/s | 79.1 MiB/s | 56.8 MiB/s | 63.8 MiB/s |
| Output bytes | 19.0 B | 19.0 B | 19.0 B | 19.0 B | 19.0 B | 19.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
