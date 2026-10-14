---
type: Report
title: JSON round trip
description: Measures converting metadata to JSON text and parsing it back.
workload: benchmark.issue5-json-conversion
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

# JSON round trip

## What this measures

The query serializes `.metadata` with `tojson`, parses it with `fromjson`, and
returns the resulting object's length. It includes both text conversion steps.

## Why it matters

Applications cross JSON text boundaries when they log, cache, or hand data to
another process. A round trip can cost more than an in-memory field read.

## Input and output

The input is a metadata object from a natural snapshot. The output is one
integer for the number of members after the round trip, not the JSON text.

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
| Wall time | 3.0 ms | 9.6 ms | 21.9 ms | 3.5 ms | 4.7 ms | 4.7 ms |
| Wall time dispersion | 0.1 ms / 3.0 ms / 2.9-3.0 ms | 0.0 ms / 9.6 ms / 9.5-9.6 ms | 0.5 ms / 22.8 ms / 21.4-22.8 ms | 0.0 ms / 3.5 ms / 3.5-3.5 ms | 0.0 ms / 4.7 ms / 4.6-4.7 ms | 0.0 ms / 4.8 ms / 4.7-4.8 ms |
| First output | 3.0 ms | 9.6 ms | 21.9 ms | 3.3 ms | 4.4 ms | 4.3 ms |
| First output dispersion | 0.1 ms / 3.0 ms / 2.9-3.0 ms | 0.0 ms / 9.6 ms / 9.5-9.6 ms | 0.8 ms / 22.8 ms / 21.2-22.8 ms | 0.0 ms / 3.3 ms / 3.3-3.3 ms | 0.0 ms / 4.4 ms / 4.4-4.4 ms | 0.0 ms / 4.4 ms / 4.3-4.4 ms |
| User CPU | 1.9 ms | 7.6 ms | 21.9 ms | 2.3 ms | 2.7 ms | 2.8 ms |
| System CPU | 0.9 ms | 5.5 ms | 10.4 ms | 1.1 ms | 1.8 ms | 1.9 ms |
| Peak RSS | 3.3 MiB | 20.9 MiB | 36.9 MiB | 17.1 MiB | 19.8 MiB | 19.4 MiB |
| Logical throughput | 82460.3 records/s | 25437.9 records/s | 11128.8 records/s | 70014.3 records/s | 51705.9 records/s | 51981.3 records/s |
| Physical throughput | 56.2 MiB/s | 17.3 MiB/s | 7.6 MiB/s | 47.7 MiB/s | 35.2 MiB/s | 41.9 MiB/s |
| Output bytes | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.2 ms | 3.0 ms | 3.2 ms | 1.9 ms | 2.0 ms | 1.9 ms |
| Wall time dispersion | 0.0 ms / 1.2 ms / 1.2-1.2 ms | 0.2 ms / 3.2 ms / 2.8-3.2 ms | 0.1 ms / 3.4 ms / 3.1-3.4 ms | 0.0 ms / 2.1 ms / 1.9-2.1 ms | 0.0 ms / 2.1 ms / 1.9-2.1 ms | 0.0 ms / 2.1 ms / 1.9-2.1 ms |
| First output | 1.2 ms | 3.0 ms | 3.2 ms | 1.9 ms | 2.0 ms | 1.9 ms |
| First output dispersion | 0.0 ms / 1.2 ms / 1.2-1.2 ms | 0.2 ms / 3.2 ms / 2.8-3.2 ms | 0.0 ms / 3.3 ms / 3.1-3.3 ms | 0.0 ms / 2.1 ms / 1.9-2.1 ms | 0.0 ms / 2.1 ms / 1.9-2.1 ms | 0.0 ms / 2.1 ms / 1.9-2.1 ms |
| User CPU | 1.1 ms | 1.9 ms | 0.8 ms | 0.0 ms | 0.0 ms | 0.9 ms |
| System CPU | 0.0 ms | 1.0 ms | 2.5 ms | 1.7 ms | 1.8 ms | 0.9 ms |
| Peak RSS | 2.7 MiB | 13.2 MiB | 13.9 MiB | 15.3 MiB | 15.8 MiB | 15.4 MiB |
| Logical throughput | 5988.0 records/s | 2341.1 records/s | 2162.5 records/s | 3625.1 records/s | 3587.9 records/s | 3626.9 records/s |
| Physical throughput | 4.3 MiB/s | 1.7 MiB/s | 1.6 MiB/s | 2.6 MiB/s | 2.6 MiB/s | 3.1 MiB/s |
| Output bytes | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 107.7 ms | 252.8 ms | 864.9 ms | 75.4 ms | 116.0 ms | 124.3 ms |
| Wall time dispersion | 0.0 ms / 107.7 ms / 107.2-107.7 ms | 0.2 ms / 254.4 ms / 252.6-254.4 ms | 1.2 ms / 867.1 ms / 863.7-867.1 ms | 0.8 ms / 76.9 ms / 74.6-76.9 ms | 2.0 ms / 119.9 ms / 114.0-119.9 ms | 0.7 ms / 124.9 ms / 122.5-124.9 ms |
| First output | 104.3 ms | 234.3 ms | 790.0 ms | 74.9 ms | 116.0 ms | 124.3 ms |
| First output dispersion | 0.0 ms / 104.4 ms / 104.3-104.4 ms | 0.6 ms / 238.4 ms / 233.7-238.4 ms | 2.5 ms / 792.5 ms / 786.6-792.5 ms | 1.1 ms / 76.9 ms / 73.8-76.9 ms | 2.0 ms / 119.1 ms / 114.0-119.1 ms | 0.1 ms / 124.4 ms / 122.3-124.4 ms |
| User CPU | 82.7 ms | 232.5 ms | 1078.7 ms | 70.5 ms | 111.5 ms | 117.5 ms |
| System CPU | 23.9 ms | 105.8 ms | 457.3 ms | 6.0 ms | 6.0 ms | 7.0 ms |
| Peak RSS | 56.6 MiB | 281.1 MiB | 1030.9 MiB | 73.3 MiB | 117.9 MiB | 97.9 MiB |
| Logical throughput | 100210.8 records/s | 42691.6 records/s | 12477.8 records/s | 143162.3 records/s | 93041.7 records/s | 86855.0 records/s |
| Physical throughput | 67.9 MiB/s | 28.9 MiB/s | 8.4 MiB/s | 97.0 MiB/s | 62.9 MiB/s | 69.7 MiB/s |
| Output bytes | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 20.0 ms | 56.0 ms | 180.6 ms | 16.7 ms | 24.8 ms | 26.5 ms |
| Wall time dispersion | 0.0 ms / 20.1 ms / 19.8-20.1 ms | 0.4 ms / 57.0 ms / 55.6-57.0 ms | 3.1 ms / 184.2 ms / 177.4-184.2 ms | 0.1 ms / 16.9 ms / 16.7-16.9 ms | 0.5 ms / 25.3 ms / 24.3-25.3 ms | 0.1 ms / 26.6 ms / 25.8-26.6 ms |
| First output | 20.0 ms | 51.9 ms | 166.4 ms | 16.7 ms | 24.4 ms | 26.5 ms |
| First output dispersion | 0.0 ms / 20.1 ms / 19.1-20.1 ms | 0.2 ms / 52.1 ms / 51.7-52.1 ms | 1.9 ms / 168.3 ms / 161.8-168.3 ms | 0.1 ms / 16.9 ms / 16.7-16.9 ms | 0.1 ms / 25.3 ms / 24.3-25.3 ms | 0.1 ms / 26.6 ms / 25.3-26.6 ms |
| User CPU | 14.9 ms | 51.6 ms | 214.1 ms | 15.5 ms | 21.5 ms | 24.3 ms |
| System CPU | 4.9 ms | 24.9 ms | 97.3 ms | 1.0 ms | 3.0 ms | 2.0 ms |
| Peak RSS | 13.3 MiB | 69.3 MiB | 226.5 MiB | 31.1 MiB | 33.5 MiB | 33.5 MiB |
| Logical throughput | 111510.4 records/s | 39924.9 records/s | 12372.8 records/s | 133620.4 records/s | 89910.3 records/s | 84359.2 records/s |
| Physical throughput | 75.6 MiB/s | 27.1 MiB/s | 8.4 MiB/s | 90.6 MiB/s | 60.9 MiB/s | 67.8 MiB/s |
| Output bytes | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
