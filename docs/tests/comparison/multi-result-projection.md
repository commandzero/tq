---
type: Report
title: "Projecting many results"
description: "Measures streaming extraction of one field from every feature."
workload: benchmark.multi-result-projection
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
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
| Wall time | 2.9 ms | 10.0 ms | 18.7 ms | 3.8 ms | 5.3 ms | 3.6 ms |
| Wall time dispersion | 0.1 ms / 3.1 ms / 2.7-3.1 ms | 0.2 ms / 10.5 ms / 9.5-10.5 ms | 0.2 ms / 18.9 ms / 17.9-18.9 ms | 0.1 ms / 4.0 ms / 3.4-4.0 ms | 0.0 ms / 5.3 ms / 5.1-5.3 ms | 0.2 ms / 4.0 ms / 3.2-4.0 ms |
| First output | 2.8 ms | 8.6 ms | 17.4 ms | 3.6 ms | 5.0 ms | 3.4 ms |
| First output dispersion | 0.1 ms / 3.0 ms / 2.6-3.0 ms | 0.1 ms / 8.9 ms / 8.1-8.9 ms | 0.2 ms / 17.8 ms / 16.4-17.8 ms | 0.2 ms / 3.9 ms / 3.2-3.9 ms | 0.1 ms / 5.2 ms / 4.8-5.2 ms | 0.2 ms / 3.7 ms / 3.1-3.7 ms |
| User CPU | 1.6 ms | 8.6 ms | 15.4 ms | 2.6 ms | 3.3 ms | 2.4 ms |
| System CPU | 1.2 ms | 5.4 ms | 11.3 ms | 1.0 ms | 1.9 ms | 1.0 ms |
| Peak RSS | 4.4 MiB | 18.9 MiB | 32.1 MiB | 7.5 MiB | 9.6 MiB | 7.3 MiB |
| Logical throughput | 66279.5 records/s | 19382.6 records/s | 10397.1 records/s | 51609.5 records/s | 36579.6 records/s | 54632.5 records/s |
| Physical throughput | 45.4 MiB/s | 13.3 MiB/s | 7.1 MiB/s | 35.3 MiB/s | 25.0 MiB/s | 44.2 MiB/s |
| Output bytes | 839.0 B | 839.0 B | 839.0 B | 839.0 B | 839.0 B | 839.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | 3.0 ms | 3.0 ms | 1.2 ms | 1.2 ms | 1.2 ms |
| Wall time dispersion | 0.0 ms / 1.7 ms / 1.3-1.7 ms | 0.2 ms / 3.4 ms / 2.7-3.4 ms | 0.0 ms / 3.2 ms / 2.9-3.4 ms | 0.0 ms / 1.5 ms / 1.1-1.5 ms | 0.0 ms / 1.5 ms / 1.1-1.5 ms | 0.0 ms / 1.4 ms / 1.1-1.5 ms |
| First output | 1.3 ms | 2.6 ms | 2.6 ms | 1.0 ms | 1.0 ms | 1.0 ms |
| First output dispersion | 0.0 ms / 1.5 ms / 1.2-1.7 ms | 0.1 ms / 2.9 ms / 2.4-3.1 ms | 0.1 ms / 2.9 ms / 2.4-2.9 ms | 0.0 ms / 1.3 ms / 1.0-1.4 ms | 0.0 ms / 1.3 ms / 1.0-1.3 ms | 0.0 ms / 1.2 ms / 1.0-1.2 ms |
| User CPU | 0.6 ms | 1.3 ms | 1.1 ms | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.7 ms | 1.7 ms | 2.0 ms | 1.1 ms | 1.1 ms | 1.0 ms |
| Peak RSS | 4.0 MiB | 11.8 MiB | 12.0 MiB | 7.7 MiB | 8.4 MiB | 7.2 MiB |
| Logical throughput | 1351.8 records/s | 663.1 records/s | 656.1 records/s | 1669.4 records/s | 1666.7 records/s | 1683.5 records/s |
| Physical throughput | 1.1 MiB/s | 0.6 MiB/s | 0.6 MiB/s | 1.4 MiB/s | 1.4 MiB/s | 1.6 MiB/s |
| Output bytes | 10.0 B | 10.0 B | 10.0 B | 10.0 B | 10.0 B | 10.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 100.8 ms | 370.6 ms | 1005.5 ms | 134.0 ms | 229.6 ms | 118.7 ms |
| Wall time dispersion | 0.5 ms / 103.8 ms / 99.3-103.8 ms | 2.1 ms / 379.4 ms / 365.0-379.4 ms | 16.2 ms / 1022.8 ms / 938.5-1022.8 ms | 0.8 ms / 137.6 ms / 132.8-137.6 ms | 1.8 ms / 233.7 ms / 227.2-233.7 ms | 0.7 ms / 126.6 ms / 117.6-126.6 ms |
| First output | 80.6 ms | 269.8 ms | 856.5 ms | 133.8 ms | 225.8 ms | 118.5 ms |
| First output dispersion | 0.3 ms / 82.4 ms / 79.4-82.4 ms | 3.8 ms / 280.2 ms / 263.9-280.2 ms | 20.0 ms / 877.4 ms / 805.6-877.4 ms | 0.8 ms / 137.5 ms / 132.5-137.5 ms | 1.6 ms / 229.1 ms / 223.6-229.1 ms | 0.7 ms / 126.4 ms / 117.5-126.4 ms |
| User CPU | 75.1 ms | 392.8 ms | 1378.0 ms | 132.4 ms | 167.0 ms | 117.0 ms |
| System CPU | 25.0 ms | 143.5 ms | 482.0 ms | 1.0 ms | 63.2 ms | 2.0 ms |
| Peak RSS | 60.5 MiB | 332.4 MiB | 1303.8 MiB | 7.9 MiB | 89.3 MiB | 7.2 MiB |
| Logical throughput | 111797.5 records/s | 30423.0 records/s | 11212.6 records/s | 84123.0 records/s | 49113.5 records/s | 94972.9 records/s |
| Physical throughput | 75.8 MiB/s | 20.6 MiB/s | 7.6 MiB/s | 57.0 MiB/s | 33.2 MiB/s | 76.2 MiB/s |
| Output bytes | 50801.0 B | 50801.0 B | 50801.0 B | 50801.0 B | 50801.0 B | 50801.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 20.0 ms | 75.5 ms | 187.5 ms | 29.3 ms | 44.7 ms | 26.0 ms |
| Wall time dispersion | 0.3 ms / 20.7 ms / 18.7-20.7 ms | 0.4 ms / 78.9 ms / 74.4-78.9 ms | 2.8 ms / 197.9 ms / 181.9-197.9 ms | 0.3 ms / 33.1 ms / 28.7-33.1 ms | 0.2 ms / 45.4 ms / 42.7-45.4 ms | 0.5 ms / 26.7 ms / 24.9-26.7 ms |
| First output | 17.2 ms | 55.8 ms | 160.6 ms | 29.2 ms | 43.7 ms | 25.8 ms |
| First output dispersion | 0.3 ms / 17.8 ms / 16.4-17.8 ms | 0.6 ms / 56.8 ms / 53.8-56.8 ms | 5.3 ms / 169.1 ms / 152.6-169.1 ms | 0.4 ms / 32.8 ms / 28.5-32.8 ms | 0.4 ms / 44.1 ms / 41.8-44.1 ms | 0.5 ms / 26.6 ms / 24.8-26.6 ms |
| User CPU | 13.8 ms | 77.0 ms | 211.0 ms | 28.3 ms | 31.8 ms | 24.1 ms |
| System CPU | 5.4 ms | 31.2 ms | 99.3 ms | 1.0 ms | 12.7 ms | 1.0 ms |
| Peak RSS | 14.5 MiB | 84.8 MiB | 236.9 MiB | 7.9 MiB | 24.3 MiB | 7.2 MiB |
| Logical throughput | 111461.8 records/s | 29451.1 records/s | 11865.3 records/s | 75881.6 records/s | 49769.6 records/s | 85434.0 records/s |
| Physical throughput | 75.5 MiB/s | 20.0 MiB/s | 8.0 MiB/s | 51.4 MiB/s | 33.7 MiB/s | 68.6 MiB/s |
| Output bytes | 9903.0 B | 9903.0 B | 9903.0 B | 9903.0 B | 9903.0 B | 9903.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
