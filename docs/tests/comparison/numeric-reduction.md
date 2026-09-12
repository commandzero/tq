---
type: Report
title: "Numeric reduction"
description: "Measures a blocking sum across all feature magnitudes."
workload: benchmark.numeric-reduction
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
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
| Wall time | 2.9 ms | 9.4 ms | 18.2 ms | 3.6 ms | 4.3 ms | 4.1 ms |
| Wall time dispersion | 0.1 ms / 3.2 ms / 2.7-3.2 ms | 0.2 ms / 10.2 ms / 9.0-10.2 ms | 0.2 ms / 19.2 ms / 17.6-19.2 ms | 0.1 ms / 4.0 ms / 3.4-4.0 ms | 0.3 ms / 4.7 ms / 4.0-4.7 ms | 0.2 ms / 4.5 ms / 3.9-4.5 ms |
| First output | 2.8 ms | 9.3 ms | 18.1 ms | 3.4 ms | 4.0 ms | 3.8 ms |
| First output dispersion | 0.1 ms / 3.1 ms / 2.6-3.1 ms | 0.2 ms / 10.2 ms / 9.0-10.2 ms | 0.2 ms / 19.0 ms / 17.4-19.0 ms | 0.2 ms / 3.7 ms / 3.2-3.7 ms | 0.2 ms / 4.5 ms / 3.7-4.5 ms | 0.1 ms / 4.2 ms / 3.5-4.2 ms |
| User CPU | 1.6 ms | 7.4 ms | 16.4 ms | 2.4 ms | 2.8 ms | 2.9 ms |
| System CPU | 1.2 ms | 6.1 ms | 10.3 ms | 1.2 ms | 1.7 ms | 1.0 ms |
| Peak RSS | 4.4 MiB | 19.0 MiB | 33.4 MiB | 9.2 MiB | 9.6 MiB | 8.9 MiB |
| Logical throughput | 66792.9 records/s | 20711.0 records/s | 10677.8 records/s | 53399.4 records/s | 45380.1 records/s | 47427.0 records/s |
| Physical throughput | 45.7 MiB/s | 14.2 MiB/s | 7.3 MiB/s | 36.6 MiB/s | 31.0 MiB/s | 38.4 MiB/s |
| Output bytes | 19.0 B | 19.0 B | 19.0 B | 19.0 B | 19.0 B | 19.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | 3.2 ms | 3.2 ms | 1.3 ms | 1.3 ms | 1.2 ms |
| Wall time dispersion | 0.1 ms / 1.8 ms / 1.3-1.8 ms | 0.1 ms / 3.4 ms / 2.9-3.4 ms | 0.1 ms / 3.5 ms / 3.0-3.5 ms | 0.1 ms / 1.5 ms / 1.2-1.6 ms | 0.1 ms / 1.5 ms / 1.2-1.5 ms | 0.0 ms / 1.5 ms / 1.1-1.5 ms |
| First output | 1.4 ms | 2.7 ms | 2.7 ms | 1.0 ms | 1.0 ms | 1.0 ms |
| First output dispersion | 0.0 ms / 1.7 ms / 1.2-1.7 ms | 0.1 ms / 2.9 ms / 2.6-3.0 ms | 0.0 ms / 3.1 ms / 2.6-3.1 ms | 0.0 ms / 1.4 ms / 1.0-1.4 ms | 0.0 ms / 1.3 ms / 1.0-1.4 ms | 0.0 ms / 1.3 ms / 1.0-1.3 ms |
| User CPU | 1.0 ms | 1.1 ms | 1.1 ms | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.3 ms | 2.1 ms | 2.1 ms | 1.1 ms | 1.2 ms | 1.1 ms |
| Peak RSS | 4.1 MiB | 11.9 MiB | 12.3 MiB | 8.1 MiB | 8.4 MiB | 7.9 MiB |
| Logical throughput | 1328.5 records/s | 627.8 records/s | 623.6 records/s | 1539.6 records/s | 1512.9 records/s | 1680.0 records/s |
| Physical throughput | 1.1 MiB/s | 0.5 MiB/s | 0.5 MiB/s | 1.3 MiB/s | 1.3 MiB/s | 1.6 MiB/s |
| Output bytes | 5.0 B | 5.0 B | 5.0 B | 5.0 B | 5.0 B | 5.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 100.5 ms | 327.5 ms | 894.5 ms | 136.3 ms | 175.9 ms | 152.7 ms |
| Wall time dispersion | 0.7 ms / 105.0 ms / 98.0-105.0 ms | 3.9 ms / 335.1 ms / 317.4-335.1 ms | 10.9 ms / 988.0 ms / 881.5-988.0 ms | 0.7 ms / 139.7 ms / 135.0-139.7 ms | 2.2 ms / 178.8 ms / 173.0-178.8 ms | 0.6 ms / 156.5 ms / 149.9-156.5 ms |
| First output | 96.8 ms | 305.8 ms | 837.8 ms | 132.3 ms | 171.4 ms | 148.7 ms |
| First output dispersion | 0.4 ms / 100.7 ms / 95.1-100.7 ms | 2.3 ms / 316.3 ms / 298.9-316.3 ms | 10.3 ms / 916.6 ms / 826.9-916.6 ms | 0.6 ms / 135.6 ms / 131.1-135.6 ms | 1.5 ms / 174.1 ms / 169.0-174.1 ms | 0.8 ms / 152.4 ms / 145.4-152.4 ms |
| User CPU | 76.8 ms | 358.6 ms | 1273.4 ms | 110.0 ms | 144.3 ms | 122.6 ms |
| System CPU | 23.9 ms | 130.1 ms | 406.8 ms | 25.6 ms | 30.9 ms | 30.7 ms |
| Peak RSS | 60.1 MiB | 335.4 MiB | 1296.8 MiB | 69.5 MiB | 89.3 MiB | 78.1 MiB |
| Logical throughput | 112143.4 records/s | 34428.4 records/s | 12604.2 records/s | 82699.7 records/s | 64100.7 records/s | 73818.0 records/s |
| Physical throughput | 76.0 MiB/s | 23.3 MiB/s | 8.5 MiB/s | 56.1 MiB/s | 43.4 MiB/s | 59.3 MiB/s |
| Output bytes | 19.0 B | 19.0 B | 19.0 B | 19.0 B | 19.0 B | 19.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 20.2 ms | 66.8 ms | 176.6 ms | 26.9 ms | 34.0 ms | 31.5 ms |
| Wall time dispersion | 0.2 ms / 21.1 ms / 19.5-21.1 ms | 0.8 ms / 69.0 ms / 64.5-69.0 ms | 6.9 ms / 185.0 ms / 161.4-185.0 ms | 0.6 ms / 28.2 ms / 25.6-28.2 ms | 0.2 ms / 35.4 ms / 33.4-35.4 ms | 1.0 ms / 34.0 ms / 30.5-34.0 ms |
| First output | 19.6 ms | 62.5 ms | 165.0 ms | 26.2 ms | 33.0 ms | 30.6 ms |
| First output dispersion | 0.2 ms / 20.3 ms / 18.8-20.3 ms | 0.6 ms / 64.6 ms / 59.9-64.6 ms | 5.6 ms / 170.9 ms / 149.2-170.9 ms | 0.6 ms / 27.4 ms / 24.8-27.4 ms | 0.2 ms / 34.2 ms / 32.5-34.2 ms | 1.0 ms / 32.9 ms / 29.4-32.9 ms |
| User CPU | 14.4 ms | 68.1 ms | 198.6 ms | 20.8 ms | 25.4 ms | 25.9 ms |
| System CPU | 5.0 ms | 28.0 ms | 93.9 ms | 5.5 ms | 8.9 ms | 5.9 ms |
| Peak RSS | 14.5 MiB | 80.8 MiB | 236.1 MiB | 20.3 MiB | 24.2 MiB | 21.8 MiB |
| Logical throughput | 110268.6 records/s | 33308.6 records/s | 12601.1 records/s | 82627.7 records/s | 65470.1 records/s | 70591.2 records/s |
| Physical throughput | 74.7 MiB/s | 22.6 MiB/s | 8.5 MiB/s | 56.0 MiB/s | 44.3 MiB/s | 56.7 MiB/s |
| Output bytes | 18.0 B | 18.0 B | 18.0 B | 18.0 B | 18.0 B | 18.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
