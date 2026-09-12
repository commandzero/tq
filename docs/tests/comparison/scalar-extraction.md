---
type: Report
title: "Scalar field extraction"
description: "Measures navigation from a document to one nested scalar."
workload: benchmark.scalar-extraction
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
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
Last updated: 2026-09-11
Profile: `standard` | Status: `observed-failures`

Tools: `jq` (jq-1.8.1), `tq` (tq 0.1.0 (TOON v3; jq target 1.8.x; revision a4b4d916-worktree)), `yq` (yq (https://github.com/mikefarah/yq/) version v4.53.2)
Environment: `linux` / `x86_64`, AMD Ryzen 7 7700 8-Core Processor, 16 logical CPUs, 61.9 GiB RAM; kernel `Linux 7.2.0-ogc4.1.fc44.x86_64 #1 SMP PREEMPT_DYNAMIC Thu Aug 20 16:15:37 UTC 2026`; compiler profile `release-benchmark`

Peak RSS is authoritative only after the campaign RSS preflight passes. Missing or invalid values are `-`, never estimates. RSS collector provenance (outside measurement tables): `linux-wait4`.
Measurement method (outside measurement tables): `tq-bench` native measurement: RSS scope `wait4 child lifetime including pre exec waited descendants and threads`; residual RSS floor `2.5 MiB` retained, not subtracted; observed control excess `1.1 ms`; primary timing is sampler-free.

Compare columns with the same input format to isolate tool differences. Missing adapters have `-` measurement cells and `not recorded` outcome/detail cells.

Timing rows show numeric medians followed by compact MAD / p95 / range rows, with one decimal place and a unit in each measurement cell. CPU, throughput, and output cells use the captured summary values.

### usgs-all-day

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 2.8 ms | 8.2 ms | 17.8 ms | 3.4 ms | 4.0 ms | 3.7 ms |
| Wall time dispersion | 0.1 ms / 3.2 ms / 2.6-3.2 ms | 0.2 ms / 8.6 ms / 7.8-8.6 ms | 0.3 ms / 18.3 ms / 17.0-18.3 ms | 0.0 ms / 3.5 ms / 3.2-3.5 ms | 0.1 ms / 4.4 ms / 3.7-4.4 ms | 0.1 ms / 3.8 ms / 3.6-3.8 ms |
| First output | 2.7 ms | 8.1 ms | 17.7 ms | 3.1 ms | 3.7 ms | 3.4 ms |
| First output dispersion | 0.1 ms / 3.0 ms / 2.5-3.0 ms | 0.2 ms / 8.6 ms / 7.7-8.6 ms | 0.2 ms / 18.3 ms / 17.0-18.3 ms | 0.1 ms / 3.2 ms / 3.1-3.2 ms | 0.1 ms / 4.2 ms / 3.6-4.2 ms | 0.1 ms / 3.6 ms / 3.3-3.6 ms |
| User CPU | 1.8 ms | 6.2 ms | 15.4 ms | 1.7 ms | 2.0 ms | 2.4 ms |
| System CPU | 0.9 ms | 5.3 ms | 11.0 ms | 1.7 ms | 1.9 ms | 1.1 ms |
| Peak RSS | 4.4 MiB | 18.1 MiB | 33.4 MiB | 9.0 MiB | 9.6 MiB | 8.9 MiB |
| Logical throughput | 68551.2 records/s | 23621.1 records/s | 10869.3 records/s | 57185.0 records/s | 48379.1 records/s | 52496.3 records/s |
| Physical throughput | 46.9 MiB/s | 16.2 MiB/s | 7.4 MiB/s | 39.1 MiB/s | 33.1 MiB/s | 42.5 MiB/s |
| Output bytes | 4.0 B | 4.0 B | 4.0 B | 4.0 B | 4.0 B | 4.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | 2.9 ms | 3.1 ms | 1.2 ms | 1.2 ms | 1.2 ms |
| Wall time dispersion | 0.1 ms / 1.7 ms / 1.3-1.7 ms | 0.1 ms / 3.2 ms / 2.7-3.2 ms | 0.1 ms / 3.4 ms / 2.7-3.4 ms | 0.0 ms / 1.5 ms / 1.1-1.5 ms | 0.0 ms / 1.5 ms / 1.1-1.7 ms | 0.0 ms / 1.5 ms / 1.1-1.6 ms |
| First output | 1.4 ms | 2.6 ms | 2.6 ms | 1.0 ms | 1.0 ms | 1.0 ms |
| First output dispersion | 0.0 ms / 1.6 ms / 1.2-1.7 ms | 0.1 ms / 2.7 ms / 2.4-2.8 ms | 0.1 ms / 2.9 ms / 2.4-2.9 ms | 0.0 ms / 1.2 ms / 1.0-1.4 ms | 0.0 ms / 1.4 ms / 1.0-1.4 ms | 0.0 ms / 1.3 ms / 1.0-1.3 ms |
| User CPU | 0.7 ms | 1.0 ms | 1.1 ms | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.6 ms | 2.0 ms | 1.9 ms | 1.1 ms | 1.1 ms | 1.1 ms |
| Peak RSS | 4.1 MiB | 11.8 MiB | 12.0 MiB | 8.0 MiB | 8.4 MiB | 8.0 MiB |
| Logical throughput | 1343.2 records/s | 682.8 records/s | 653.5 records/s | 1687.8 records/s | 1675.7 records/s | 1683.5 records/s |
| Physical throughput | 1.1 MiB/s | 0.6 MiB/s | 0.6 MiB/s | 1.4 MiB/s | 1.4 MiB/s | 1.6 MiB/s |
| Output bytes | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 94.1 ms | 260.0 ms | 848.6 ms | 121.3 ms | 162.3 ms | 137.3 ms |
| Wall time dispersion | 0.1 ms / 94.9 ms / 92.9-94.9 ms | 3.4 ms / 266.4 ms / 251.6-266.4 ms | 9.9 ms / 948.4 ms / 834.4-948.4 ms | 1.2 ms / 127.9 ms / 119.6-127.9 ms | 0.7 ms / 165.6 ms / 161.4-165.6 ms | 1.2 ms / 140.2 ms / 135.1-140.2 ms |
| First output | 91.1 ms | 241.2 ms | 790.3 ms | 117.7 ms | 158.1 ms | 133.9 ms |
| First output dispersion | 0.3 ms / 91.5 ms / 90.1-91.5 ms | 3.0 ms / 246.2 ms / 235.8-246.2 ms | 8.3 ms / 875.8 ms / 779.2-875.8 ms | 1.0 ms / 123.7 ms / 116.3-123.7 ms | 0.4 ms / 161.6 ms / 157.2-161.6 ms | 1.0 ms / 136.8 ms / 131.9-136.8 ms |
| User CPU | 70.0 ms | 232.1 ms | 1243.2 ms | 93.1 ms | 132.6 ms | 111.7 ms |
| System CPU | 23.8 ms | 113.3 ms | 404.9 ms | 27.4 ms | 29.1 ms | 24.9 ms |
| Peak RSS | 60.2 MiB | 294.1 MiB | 1301.3 MiB | 69.2 MiB | 89.3 MiB | 78.2 MiB |
| Logical throughput | 119759.1 records/s | 43368.1 records/s | 13284.8 records/s | 92956.9 records/s | 69445.1 records/s | 82126.8 records/s |
| Physical throughput | 81.2 MiB/s | 29.4 MiB/s | 9.0 MiB/s | 63.0 MiB/s | 47.0 MiB/s | 65.9 MiB/s |
| Output bytes | 6.0 B | 6.0 B | 6.0 B | 6.0 B | 6.0 B | 6.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 19.2 ms | 55.0 ms | 166.6 ms | 24.5 ms | 31.8 ms | 27.9 ms |
| Wall time dispersion | 0.2 ms / 19.9 ms / 18.7-19.9 ms | 0.6 ms / 57.8 ms / 54.0-57.8 ms | 4.0 ms / 177.0 ms / 152.2-177.0 ms | 0.6 ms / 25.5 ms / 22.7-25.5 ms | 0.5 ms / 32.9 ms / 30.0-32.9 ms | 0.5 ms / 28.9 ms / 26.8-28.9 ms |
| First output | 18.6 ms | 50.8 ms | 155.4 ms | 23.8 ms | 30.8 ms | 27.1 ms |
| First output dispersion | 0.2 ms / 19.3 ms / 18.1-19.3 ms | 0.4 ms / 53.2 ms / 49.5-53.2 ms | 3.6 ms / 161.3 ms / 142.3-161.3 ms | 0.6 ms / 24.7 ms / 22.0-24.7 ms | 0.5 ms / 31.9 ms / 29.0-31.9 ms | 0.6 ms / 28.2 ms / 26.1-28.2 ms |
| User CPU | 13.9 ms | 47.0 ms | 204.3 ms | 19.0 ms | 23.8 ms | 21.2 ms |
| System CPU | 5.0 ms | 27.8 ms | 86.4 ms | 5.1 ms | 7.0 ms | 6.5 ms |
| Peak RSS | 14.6 MiB | 68.2 MiB | 225.4 MiB | 20.1 MiB | 24.1 MiB | 21.8 MiB |
| Logical throughput | 115804.0 records/s | 40455.6 records/s | 13354.3 records/s | 90807.1 records/s | 70006.0 records/s | 79677.7 records/s |
| Physical throughput | 78.5 MiB/s | 27.4 MiB/s | 9.0 MiB/s | 61.5 MiB/s | 47.4 MiB/s | 63.9 MiB/s |
| Output bytes | 5.0 B | 5.0 B | 5.0 B | 5.0 B | 5.0 B | 5.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
