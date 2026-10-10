---
type: Report
title: Scalar field extraction
description: Measures navigation from a document to one nested scalar.
workload: benchmark.scalar-extraction
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

# Scalar field extraction

## What this measures

The query `.metadata.count` parses a document, follows two object fields, and
returns one scalar. It is a small document-mode navigation case.

## Why it matters

Configuration checks and request handlers commonly need one field from a larger
payload. The result shows the cost of that narrow read rather than a full walk.

## Input and output

The input is a natural snapshot with a `metadata.count` field. The output is a
single scalar in the result sequence, not the surrounding metadata object or
the original document.

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
| Wall time | 3.0 ms | 9.8 ms | 21.5 ms | 3.5 ms | 4.7 ms | 4.8 ms |
| Wall time dispersion | 0.0 ms / 3.0 ms / 3.0-3.0 ms | 0.0 ms / 9.8 ms / 9.5-9.8 ms | 0.4 ms / 21.9 ms / 20.8-21.9 ms | 0.1 ms / 3.6 ms / 3.3-3.6 ms | 0.2 ms / 5.2 ms / 4.5-5.2 ms | 0.2 ms / 5.0 ms / 4.7-5.0 ms |
| First output | 3.0 ms | 9.8 ms | 21.5 ms | 3.3 ms | 4.5 ms | 4.4 ms |
| First output dispersion | 0.0 ms / 3.0 ms / 3.0-3.0 ms | 0.0 ms / 9.8 ms / 9.5-9.8 ms | 0.4 ms / 21.9 ms / 20.8-21.9 ms | 0.0 ms / 3.3 ms / 3.3-3.3 ms | 0.1 ms / 5.2 ms / 4.3-5.2 ms | 0.1 ms / 5.0 ms / 4.3-5.0 ms |
| User CPU | 1.4 ms | 9.2 ms | 20.2 ms | 2.2 ms | 3.0 ms | 3.6 ms |
| System CPU | 1.4 ms | 4.6 ms | 11.6 ms | 1.1 ms | 2.0 ms | 0.9 ms |
| Peak RSS | 3.3 MiB | 20.6 MiB | 36.1 MiB | 15.0 MiB | 17.6 MiB | 17.2 MiB |
| Logical throughput | 82349.0 records/s | 24931.0 records/s | 11353.1 records/s | 70663.2 records/s | 52136.8 records/s | 50465.4 records/s |
| Physical throughput | 56.1 MiB/s | 17.0 MiB/s | 7.7 MiB/s | 48.2 MiB/s | 35.5 MiB/s | 40.7 MiB/s |
| Output bytes | 4.0 B | 4.0 B | 4.0 B | 4.0 B | 4.0 B | 4.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.2 ms | 3.0 ms | 3.2 ms | 1.8 ms | 2.1 ms | 1.9 ms |
| Wall time dispersion | 0.0 ms / 1.3 ms / 1.2-1.3 ms | 0.2 ms / 3.3 ms / 2.8-3.3 ms | 0.1 ms / 3.3 ms / 2.9-3.3 ms | 0.0 ms / 1.9 ms / 1.8-1.9 ms | 0.0 ms / 2.1 ms / 1.8-2.1 ms | 0.0 ms / 1.9 ms / 1.9-1.9 ms |
| First output | 1.2 ms | 3.0 ms | 3.2 ms | 1.8 ms | 2.1 ms | 1.9 ms |
| First output dispersion | 0.0 ms / 1.3 ms / 1.2-1.3 ms | 0.2 ms / 3.3 ms / 2.8-3.3 ms | 0.1 ms / 3.3 ms / 2.9-3.3 ms | 0.0 ms / 1.9 ms / 1.8-1.9 ms | 0.0 ms / 2.1 ms / 1.8-2.1 ms | 0.0 ms / 1.9 ms / 1.9-1.9 ms |
| User CPU | 0.0 ms | 0.8 ms | 0.0 ms | 0.0 ms | 0.9 ms | 0.0 ms |
| System CPU | 1.0 ms | 2.5 ms | 3.0 ms | 1.6 ms | 1.1 ms | 1.8 ms |
| Peak RSS | 2.7 MiB | 13.2 MiB | 13.9 MiB | 15.0 MiB | 15.6 MiB | 15.4 MiB |
| Logical throughput | 5998.3 records/s | 2334.9 records/s | 2197.1 records/s | 3891.1 records/s | 3334.9 records/s | 3678.4 records/s |
| Physical throughput | 4.3 MiB/s | 1.7 MiB/s | 1.6 MiB/s | 2.8 MiB/s | 2.4 MiB/s | 3.1 MiB/s |
| Output bytes | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 105.3 ms | 255.4 ms | 865.9 ms | 73.5 ms | 116.0 ms | 123.1 ms |
| Wall time dispersion | 1.1 ms / 108.6 ms / 104.2-108.6 ms | 0.2 ms / 257.2 ms / 255.2-257.2 ms | 6.9 ms / 872.9 ms / 857.1-872.9 ms | 0.4 ms / 74.7 ms / 73.1-74.7 ms | 0.1 ms / 116.2 ms / 115.9-116.2 ms | 0.3 ms / 128.4 ms / 122.8-128.4 ms |
| First output | 102.3 ms | 237.7 ms | 790.9 ms | 72.8 ms | 116.0 ms | 122.3 ms |
| First output dispersion | 1.1 ms / 105.4 ms / 101.2-105.4 ms | 0.3 ms / 238.0 ms / 237.3-238.0 ms | 0.9 ms / 791.8 ms / 784.7-791.8 ms | 0.2 ms / 73.9 ms / 72.6-73.9 ms | 0.0 ms / 116.0 ms / 115.9-116.0 ms | 0.1 ms / 127.6 ms / 122.2-127.6 ms |
| User CPU | 82.6 ms | 217.1 ms | 1048.1 ms | 69.1 ms | 108.6 ms | 118.3 ms |
| System CPU | 24.8 ms | 122.6 ms | 484.9 ms | 4.9 ms | 7.0 ms | 5.0 ms |
| Peak RSS | 56.6 MiB | 279.6 MiB | 1031.4 MiB | 72.8 MiB | 117.4 MiB | 97.1 MiB |
| Logical throughput | 102501.8 records/s | 42260.9 records/s | 12462.7 records/s | 146899.9 records/s | 93033.7 records/s | 87651.5 records/s |
| Physical throughput | 69.4 MiB/s | 28.6 MiB/s | 8.4 MiB/s | 99.5 MiB/s | 62.9 MiB/s | 70.3 MiB/s |
| Output bytes | 6.0 B | 6.0 B | 6.0 B | 6.0 B | 6.0 B | 6.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 20.1 ms | 57.3 ms | 176.0 ms | 16.8 ms | 24.8 ms | 26.0 ms |
| Wall time dispersion | 0.4 ms / 21.0 ms / 19.7-21.0 ms | 0.6 ms / 57.9 ms / 56.7-57.9 ms | 8.0 ms / 184.1 ms / 167.0-184.1 ms | 0.2 ms / 17.9 ms / 16.6-17.9 ms | 0.2 ms / 25.6 ms / 24.6-25.6 ms | 0.5 ms / 26.6 ms / 25.5-26.6 ms |
| First output | 20.1 ms | 52.9 ms | 161.9 ms | 16.8 ms | 24.4 ms | 26.0 ms |
| First output dispersion | 0.1 ms / 20.1 ms / 19.1-20.1 ms | 0.1 ms / 54.0 ms / 52.8-54.0 ms | 8.5 ms / 170.4 ms / 152.7-170.4 ms | 0.2 ms / 17.9 ms / 16.6-17.9 ms | 0.0 ms / 25.4 ms / 24.4-25.4 ms | 0.5 ms / 26.6 ms / 25.5-26.6 ms |
| User CPU | 13.9 ms | 48.7 ms | 206.2 ms | 13.8 ms | 21.3 ms | 23.9 ms |
| System CPU | 6.9 ms | 25.8 ms | 103.4 ms | 2.9 ms | 4.1 ms | 2.0 ms |
| Peak RSS | 13.3 MiB | 69.0 MiB | 234.3 MiB | 29.0 MiB | 33.6 MiB | 33.1 MiB |
| Logical throughput | 111321.5 records/s | 38992.5 records/s | 12689.9 records/s | 132841.8 records/s | 90048.0 records/s | 85810.9 records/s |
| Physical throughput | 75.5 MiB/s | 26.4 MiB/s | 8.6 MiB/s | 90.1 MiB/s | 61.0 MiB/s | 68.9 MiB/s |
| Output bytes | 5.0 B | 5.0 B | 5.0 B | 5.0 B | 5.0 B | 5.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
