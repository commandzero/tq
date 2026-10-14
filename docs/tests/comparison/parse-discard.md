---
type: Report
title: Discarding a stream
description: Measures parsing and stream control when a query emits no values.
workload: benchmark.parse-discard
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

# Discarding a stream

## What this measures

The `empty` query reads each natural input and emits nothing. The workload
isolates input handling and the cost of a filter that discards its result.

## Why it matters

Log and validation pipelines often reject or discard records early. This case
shows the cost of doing that without paying to serialize an output document.

## Input and output

The input is a natural JSON, YAML, or TOON snapshot at each catalog size. The
output is an empty value sequence, not an empty array. That distinction keeps
parsing cost separate from output construction.

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

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 2.9 ms | 9.7 ms | 20.9 ms | 3.3 ms | 4.2 ms | 4.5 ms |
| Wall time dispersion | 0.0 ms / 3.0 ms / 2.9-3.0 ms | 0.2 ms / 10.0 ms / 9.2-10.0 ms | 0.6 ms / 21.4 ms / 19.8-21.4 ms | 0.1 ms / 3.8 ms / 3.2-3.8 ms | 0.0 ms / 4.4 ms / 4.2-4.4 ms | 0.1 ms / 4.6 ms / 4.4-4.6 ms |
| First output | - | - | - | - | - | - |
| First output dispersion | - | - | - | - | - | - |
| User CPU | 1.9 ms | 8.3 ms | 18.3 ms | 1.0 ms | 2.2 ms | 3.2 ms |
| System CPU | 0.9 ms | 5.9 ms | 11.4 ms | 2.0 ms | 2.2 ms | 1.1 ms |
| Peak RSS | 3.3 MiB | 20.2 MiB | 35.8 MiB | 14.5 MiB | 16.8 MiB | 16.8 MiB |
| Logical throughput | 85047.1 records/s | 25041.1 records/s | 11686.9 records/s | 73471.8 records/s | 57465.9 records/s | 54054.1 records/s |
| Physical throughput | 58.0 MiB/s | 17.1 MiB/s | 8.0 MiB/s | 50.1 MiB/s | 39.1 MiB/s | 43.6 MiB/s |
| Output bytes | 0.0 B | 0.0 B | 0.0 B | 0.0 B | 0.0 B | 0.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.2 ms | 2.7 ms | 3.1 ms | 1.8 ms | 1.8 ms | 1.7 ms |
| Wall time dispersion | 0.0 ms / 1.2 ms / 1.2-1.2 ms | 0.1 ms / 2.8 ms / 2.5-2.8 ms | 0.0 ms / 3.2 ms / 3.0-3.2 ms | 0.1 ms / 1.8 ms / 1.6-1.8 ms | 0.0 ms / 1.8 ms / 1.8-1.8 ms | 0.0 ms / 1.8 ms / 1.7-1.8 ms |
| First output | - | - | - | - | - | - |
| First output dispersion | - | - | - | - | - | - |
| User CPU | 1.1 ms | 1.3 ms | 2.1 ms | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.0 ms | 1.3 ms | 1.1 ms | 1.6 ms | 1.6 ms | 1.6 ms |
| Peak RSS | 2.7 MiB | 12.1 MiB | 12.9 MiB | 14.5 MiB | 15.0 MiB | 14.9 MiB |
| Logical throughput | 5932.2 records/s | 2618.8 records/s | 2236.4 records/s | 3990.9 records/s | 3926.0 records/s | 4227.1 records/s |
| Physical throughput | 4.3 MiB/s | 1.9 MiB/s | 1.6 MiB/s | 2.9 MiB/s | 2.8 MiB/s | 3.6 MiB/s |
| Output bytes | 0.0 B | 0.0 B | 0.0 B | 0.0 B | 0.0 B | 0.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 88.1 ms | 257.6 ms | 833.5 ms | 73.5 ms | 116.6 ms | 123.0 ms |
| Wall time dispersion | 0.0 ms / 88.7 ms / 88.1-88.7 ms | 3.1 ms / 260.7 ms / 254.3-260.7 ms | 44.1 ms / 885.4 ms / 789.4-885.4 ms | 0.6 ms / 74.3 ms / 72.9-74.3 ms | 0.1 ms / 116.7 ms / 115.4-116.7 ms | 0.1 ms / 123.8 ms / 122.9-123.8 ms |
| First output | - | - | - | - | - | - |
| First output dispersion | - | - | - | - | - | - |
| User CPU | 63.8 ms | 221.5 ms | 1065.2 ms | 69.1 ms | 110.1 ms | 118.2 ms |
| System CPU | 23.9 ms | 118.6 ms | 443.7 ms | 4.0 ms | 6.0 ms | 5.1 ms |
| Peak RSS | 56.6 MiB | 280.7 MiB | 1027.7 MiB | 72.3 MiB | 116.9 MiB | 96.6 MiB |
| Logical throughput | 122523.6 records/s | 41888.4 records/s | 12947.9 records/s | 146855.9 records/s | 92582.7 records/s | 87744.1 records/s |
| Physical throughput | 83.0 MiB/s | 28.4 MiB/s | 8.8 MiB/s | 99.5 MiB/s | 62.6 MiB/s | 70.4 MiB/s |
| Output bytes | 0.0 B | 0.0 B | 0.0 B | 0.0 B | 0.0 B | 0.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 19.0 ms | 55.9 ms | 174.1 ms | 16.7 ms | 25.1 ms | 26.7 ms |
| Wall time dispersion | 0.5 ms / 20.7 ms / 18.4-20.7 ms | 0.5 ms / 57.4 ms / 55.4-57.4 ms | 0.2 ms / 183.2 ms / 174.0-183.2 ms | 0.5 ms / 17.8 ms / 16.2-17.8 ms | 0.0 ms / 25.2 ms / 24.1-25.2 ms | 0.5 ms / 27.2 ms / 26.0-27.2 ms |
| First output | - | - | - | - | - | - |
| First output dispersion | - | - | - | - | - | - |
| User CPU | 14.5 ms | 56.0 ms | 200.9 ms | 15.5 ms | 22.0 ms | 23.9 ms |
| System CPU | 5.1 ms | 21.9 ms | 104.1 ms | 1.9 ms | 3.0 ms | 2.9 ms |
| Peak RSS | 13.3 MiB | 67.7 MiB | 226.2 MiB | 26.4 MiB | 32.9 MiB | 32.7 MiB |
| Logical throughput | 117733.9 records/s | 39994.3 records/s | 12828.5 records/s | 133636.4 records/s | 88908.3 records/s | 83799.1 records/s |
| Physical throughput | 79.8 MiB/s | 27.1 MiB/s | 8.7 MiB/s | 90.6 MiB/s | 60.2 MiB/s | 67.3 MiB/s |
| Output bytes | 0.0 B | 0.0 B | 0.0 B | 0.0 B | 0.0 B | 0.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
