---
type: Report
title: JSON text formatting
description: Measures serializing each feature projection as compact JSON text.
workload: benchmark.format-json
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

# JSON text formatting

## What this measures

The query projects `[id, properties]` for each feature and applies `@json`.
It measures repeated conversion from values to JSON text.

## Why it matters

Small JSON fragments are common message and logging boundaries. Serialization
cost can dominate when the query emits many compact records.

## Input and output

The input is a natural feature snapshot. The output is one JSON string per
feature containing its ID and properties, not an array of value pairs.

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
| Wall time | 6.2 ms | 24.8 ms | 36.0 ms | 6.9 ms | 8.1 ms | 8.3 ms |
| Wall time dispersion | 0.2 ms / 6.5 ms / 5.9-6.5 ms | 0.1 ms / 24.9 ms / 24.1-24.9 ms | 0.2 ms / 36.2 ms / 33.5-36.2 ms | 0.1 ms / 7.1 ms / 6.8-7.1 ms | 0.0 ms / 8.1 ms / 8.1-8.1 ms | 0.2 ms / 8.6 ms / 8.1-8.6 ms |
| First output | 3.3 ms | 23.3 ms | 30.7 ms | 5.4 ms | 5.4 ms | 6.4 ms |
| First output dispersion | 0.0 ms / 3.3 ms / 3.3-3.3 ms | 0.0 ms / 23.3 ms / 22.2-23.3 ms | 0.0 ms / 30.7 ms / 30.6-30.7 ms | 0.0 ms / 5.4 ms / 4.4-5.4 ms | 0.0 ms / 6.5 ms / 5.4-6.5 ms | 0.0 ms / 6.5 ms / 6.4-6.5 ms |
| User CPU | 6.1 ms | 23.8 ms | 28.7 ms | 4.4 ms | 4.5 ms | 5.7 ms |
| System CPU | 0.0 ms | 9.5 ms | 18.4 ms | 2.2 ms | 3.4 ms | 2.3 ms |
| Peak RSS | 3.5 MiB | 32.0 MiB | 41.7 MiB | 17.3 MiB | 19.7 MiB | 19.3 MiB |
| Logical throughput | 39514.2 records/s | 9824.8 records/s | 6775.3 records/s | 35270.3 records/s | 30157.0 records/s | 29518.5 records/s |
| Physical throughput | 26.9 MiB/s | 6.7 MiB/s | 4.6 MiB/s | 24.0 MiB/s | 20.5 MiB/s | 23.8 MiB/s |
| Output bytes | 166379.0 B | 166379.0 B | 166379.0 B | 166379.0 B | 166379.0 B | 166379.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.3 ms | 3.5 ms | 3.8 ms | 1.9 ms | 2.0 ms | 2.1 ms |
| Wall time dispersion | 0.0 ms / 1.3 ms / 1.2-1.3 ms | 0.1 ms / 3.6 ms / 3.3-3.6 ms | 0.1 ms / 3.8 ms / 3.6-3.8 ms | 0.0 ms / 2.1 ms / 1.9-2.1 ms | 0.0 ms / 2.1 ms / 1.9-2.1 ms | 0.0 ms / 2.1 ms / 2.1-2.1 ms |
| First output | 1.3 ms | 3.3 ms | 3.3 ms | 1.9 ms | 2.0 ms | 2.1 ms |
| First output dispersion | 0.0 ms / 1.3 ms / 1.2-1.3 ms | 0.0 ms / 3.3 ms / 3.3-3.3 ms | 0.0 ms / 3.8 ms / 3.3-3.8 ms | 0.0 ms / 2.1 ms / 1.9-2.1 ms | 0.0 ms / 2.1 ms / 1.9-2.1 ms | 0.0 ms / 2.1 ms / 2.1-2.1 ms |
| User CPU | 0.0 ms | 1.7 ms | 2.5 ms | 0.9 ms | 0.0 ms | 0.0 ms |
| System CPU | 1.1 ms | 1.8 ms | 1.3 ms | 0.9 ms | 1.8 ms | 1.9 ms |
| Peak RSS | 2.7 MiB | 13.8 MiB | 14.2 MiB | 15.3 MiB | 15.6 MiB | 15.6 MiB |
| Logical throughput | 5287.0 records/s | 2013.2 records/s | 1864.7 records/s | 3630.7 records/s | 3553.3 records/s | 3352.5 records/s |
| Physical throughput | 3.8 MiB/s | 1.5 MiB/s | 1.3 MiB/s | 2.6 MiB/s | 2.6 MiB/s | 2.8 MiB/s |
| Output bytes | 4751.0 B | 4751.0 B | 4751.0 B | 4751.0 B | 4751.0 B | 4751.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 246.4 ms | 1007.5 ms | 1458.1 ms | 229.4 ms | 272.0 ms | 281.9 ms |
| Wall time dispersion | 3.5 ms / 249.9 ms / 241.2-249.9 ms | 9.6 ms / 1017.8 ms / 997.9-1017.8 ms | 20.0 ms / 1521.4 ms / 1438.2-1521.4 ms | 0.1 ms / 229.7 ms / 229.3-229.7 ms | 1.0 ms / 273.0 ms / 270.6-273.0 ms | 0.4 ms / 284.2 ms / 281.4-284.2 ms |
| First output | 79.1 ms | 833.6 ms | 1243.6 ms | 69.6 ms | 111.7 ms | 119.1 ms |
| First output dispersion | 1.0 ms / 81.2 ms / 78.1-81.2 ms | 3.0 ms / 836.6 ms / 818.0-836.6 ms | 8.6 ms / 1301.9 ms / 1235.0-1301.9 ms | 0.1 ms / 70.7 ms / 69.6-70.7 ms | 0.0 ms / 114.9 ms / 111.6-114.9 ms | 0.1 ms / 119.2 ms / 118.2-119.2 ms |
| User CPU | 224.7 ms | 1035.7 ms | 1876.7 ms | 170.9 ms | 215.0 ms | 221.6 ms |
| System CPU | 21.3 ms | 327.5 ms | 527.8 ms | 40.0 ms | 36.1 ms | 37.6 ms |
| Peak RSS | 59.8 MiB | 798.3 MiB | 1270.3 MiB | 73.0 MiB | 117.8 MiB | 97.2 MiB |
| Logical throughput | 43791.4 records/s | 10712.2 records/s | 7401.2 records/s | 47053.7 records/s | 39676.0 records/s | 38288.5 records/s |
| Physical throughput | 29.7 MiB/s | 7.3 MiB/s | 5.0 MiB/s | 31.9 MiB/s | 26.8 MiB/s | 30.7 MiB/s |
| Output bytes | 7336173.0 B | 7336173.0 B | 7336173.0 B | 7336173.0 B | 7336173.0 B | 7336173.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 46.9 ms | 208.5 ms | 308.0 ms | 48.9 ms | 57.5 ms | 57.3 ms |
| Wall time dispersion | 0.1 ms / 47.0 ms / 46.7-47.0 ms | 1.6 ms / 210.1 ms / 201.7-210.1 ms | 0.8 ms / 309.6 ms / 307.2-309.6 ms | 0.0 ms / 48.9 ms / 48.8-48.9 ms | 0.8 ms / 58.3 ms / 56.4-58.3 ms | 0.2 ms / 58.3 ms / 57.1-58.3 ms |
| First output | 18.0 ms | 170.3 ms | 265.0 ms | 19.1 ms | 25.4 ms | 27.4 ms |
| First output dispersion | 0.0 ms / 18.1 ms / 18.0-18.1 ms | 1.7 ms / 172.3 ms / 168.7-172.3 ms | 1.2 ms / 266.2 ms / 262.8-266.2 ms | 0.0 ms / 19.1 ms / 18.0-19.1 ms | 0.0 ms / 27.6 ms / 25.4-27.6 ms | 0.1 ms / 27.5 ms / 26.5-27.5 ms |
| User CPU | 41.6 ms | 205.4 ms | 369.3 ms | 34.1 ms | 49.4 ms | 45.6 ms |
| System CPU | 5.1 ms | 76.8 ms | 115.9 ms | 10.5 ms | 5.1 ms | 8.0 ms |
| Peak RSS | 14.1 MiB | 179.9 MiB | 269.5 MiB | 31.2 MiB | 33.7 MiB | 33.6 MiB |
| Logical throughput | 47676.0 records/s | 10714.6 records/s | 7253.1 records/s | 45719.7 records/s | 38843.4 records/s | 38994.6 records/s |
| Physical throughput | 32.3 MiB/s | 7.3 MiB/s | 4.9 MiB/s | 31.0 MiB/s | 26.3 MiB/s | 31.3 MiB/s |
| Output bytes | 1518106.0 B | 1518106.0 B | 1518106.0 B | 1518106.0 B | 1518106.0 B | 1518106.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
