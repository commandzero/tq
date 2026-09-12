---
type: Report
title: "Path update"
description: "Measures updating one nested object while returning the full document."
workload: benchmark.path-update
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Path update

## What this measures

The update query adds `benchmarked: true` inside `.metadata` and emits the
complete document. It exercises path focus, object update, and full-document
serialization.

## Why it matters

Enriching records in place is a normal ETL operation. The work includes both
finding a nested path and carrying the untouched parts of the document through.

## Input and output

The input is a natural snapshot with a metadata object. The output is the whole
snapshot with one metadata field added, not just the changed metadata value.

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
| Wall time | 4.8 ms | 16.8 ms | 24.4 ms | 4.1 ms | 5.0 ms | 4.5 ms |
| Wall time dispersion | 0.1 ms / 5.5 ms / 4.4-5.5 ms | 0.5 ms / 17.8 ms / 16.2-17.8 ms | 0.2 ms / 25.1 ms / 23.1-25.1 ms | 0.1 ms / 4.3 ms / 3.9-4.3 ms | 0.2 ms / 5.4 ms / 4.6-5.4 ms | 0.2 ms / 5.0 ms / 4.2-5.0 ms |
| First output | 2.7 ms | 16.8 ms | 24.3 ms | 3.2 ms | 4.1 ms | 3.5 ms |
| First output dispersion | 0.1 ms / 3.1 ms / 2.4-3.1 ms | 0.4 ms / 17.6 ms / 16.1-17.6 ms | 0.3 ms / 25.1 ms / 23.0-25.1 ms | 0.1 ms / 3.4 ms / 3.1-3.4 ms | 0.1 ms / 4.4 ms / 3.7-4.4 ms | 0.2 ms / 4.0 ms / 3.2-4.0 ms |
| User CPU | 3.5 ms | 16.4 ms | 24.4 ms | 2.5 ms | 2.9 ms | 3.4 ms |
| System CPU | 1.0 ms | 5.8 ms | 10.8 ms | 1.7 ms | 1.9 ms | 1.1 ms |
| Peak RSS | 4.4 MiB | 22.6 MiB | 33.8 MiB | 9.1 MiB | 9.9 MiB | 9.2 MiB |
| Logical throughput | 40764.9 records/s | 11520.2 records/s | 7964.0 records/s | 47514.1 records/s | 38427.3 records/s | 43236.0 records/s |
| Physical throughput | 27.9 MiB/s | 7.9 MiB/s | 5.4 MiB/s | 32.5 MiB/s | 26.3 MiB/s | 35.0 MiB/s |
| Output bytes | 212523.0 B | 139093.0 B | 139093.0 B | 164690.0 B | 164690.0 B | 164690.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | 3.2 ms | 3.4 ms | 1.3 ms | 1.3 ms | 1.2 ms |
| Wall time dispersion | 0.0 ms / 1.8 ms / 1.3-2.0 ms | 0.1 ms / 3.6 ms / 3.0-3.7 ms | 0.1 ms / 3.6 ms / 3.1-3.7 ms | 0.0 ms / 1.5 ms / 1.2-1.6 ms | 0.1 ms / 1.6 ms / 1.2-1.6 ms | 0.0 ms / 1.5 ms / 1.2-1.6 ms |
| First output | 1.4 ms | 2.8 ms | 2.9 ms | 1.0 ms | 1.0 ms | 1.0 ms |
| First output dispersion | 0.0 ms / 1.7 ms / 1.2-1.8 ms | 0.1 ms / 3.1 ms / 2.6-3.2 ms | 0.2 ms / 3.2 ms / 2.6-3.2 ms | 0.0 ms / 1.4 ms / 1.0-1.4 ms | 0.0 ms / 1.4 ms / 1.0-1.5 ms | 0.0 ms / 1.3 ms / 1.0-1.4 ms |
| User CPU | 0.7 ms | 1.1 ms | 1.6 ms | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.7 ms | 2.2 ms | 1.8 ms | 1.1 ms | 1.2 ms | 1.1 ms |
| Peak RSS | 4.1 MiB | 12.0 MiB | 12.5 MiB | 8.1 MiB | 8.5 MiB | 8.0 MiB |
| Logical throughput | 1332.9 records/s | 621.0 records/s | 591.7 records/s | 1510.6 records/s | 1504.9 records/s | 1697.1 records/s |
| Physical throughput | 1.1 MiB/s | 0.5 MiB/s | 0.5 MiB/s | 1.3 MiB/s | 1.3 MiB/s | 1.6 MiB/s |
| Output bytes | 2648.0 B | 1794.0 B | 1794.0 B | 2057.0 B | 2057.0 B | 2057.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 196.8 ms | 737.7 ms | 1337.6 ms | 168.6 ms | 210.4 ms | 184.6 ms |
| Wall time dispersion | 0.8 ms / 199.8 ms / 195.1-199.8 ms | 1.8 ms / 741.0 ms / 732.2-741.0 ms | 14.9 ms / 1357.9 ms / 1261.8-1357.9 ms | 1.0 ms / 170.1 ms / 165.0-170.1 ms | 0.3 ms / 212.9 ms / 208.0-212.9 ms | 0.8 ms / 188.7 ms / 182.8-188.7 ms |
| First output | 80.1 ms | 707.7 ms | 1261.0 ms | 102.5 ms | 143.6 ms | 118.1 ms |
| First output dispersion | 0.5 ms / 81.5 ms / 79.4-81.5 ms | 2.1 ms / 710.8 ms / 701.8-710.8 ms | 13.2 ms / 1278.8 ms / 1202.1-1278.8 ms | 0.7 ms / 104.7 ms / 100.6-104.7 ms | 0.5 ms / 144.4 ms / 142.2-144.4 ms | 0.5 ms / 122.7 ms / 117.1-122.7 ms |
| User CPU | 164.8 ms | 798.4 ms | 1776.8 ms | 137.1 ms | 173.0 ms | 153.2 ms |
| System CPU | 31.4 ms | 177.1 ms | 481.5 ms | 29.5 ms | 36.4 ms | 31.3 ms |
| Peak RSS | 64.9 MiB | 481.8 MiB | 1362.4 MiB | 69.5 MiB | 89.6 MiB | 78.0 MiB |
| Logical throughput | 57279.0 records/s | 15282.1 records/s | 8428.6 records/s | 66885.8 records/s | 53572.6 records/s | 61073.6 records/s |
| Physical throughput | 38.8 MiB/s | 10.4 MiB/s | 5.7 MiB/s | 45.3 MiB/s | 36.3 MiB/s | 49.0 MiB/s |
| Output bytes | 12263602.0 B | 8001932.0 B | 8001932.0 B | 9491013.0 B | 9491013.0 B | 9491013.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 39.5 ms | 149.2 ms | 253.6 ms | 32.9 ms | 41.3 ms | 37.2 ms |
| Wall time dispersion | 0.7 ms / 41.3 ms / 38.2-41.3 ms | 1.7 ms / 152.0 ms / 146.7-152.0 ms | 7.1 ms / 263.9 ms / 238.1-263.9 ms | 0.6 ms / 34.8 ms / 31.5-34.8 ms | 1.2 ms / 43.5 ms / 38.8-43.5 ms | 0.5 ms / 37.9 ms / 34.4-37.9 ms |
| First output | 17.0 ms | 142.5 ms | 240.3 ms | 21.9 ms | 29.7 ms | 25.6 ms |
| First output dispersion | 0.3 ms / 17.9 ms / 16.5-17.9 ms | 1.9 ms / 145.1 ms / 139.6-145.1 ms | 6.0 ms / 248.8 ms / 227.3-248.8 ms | 0.4 ms / 23.1 ms / 20.6-23.1 ms | 0.5 ms / 30.7 ms / 27.9-30.7 ms | 0.2 ms / 26.2 ms / 23.9-26.2 ms |
| User CPU | 32.2 ms | 163.3 ms | 327.0 ms | 27.9 ms | 32.5 ms | 30.7 ms |
| System CPU | 6.9 ms | 41.3 ms | 96.5 ms | 5.1 ms | 9.1 ms | 6.0 ms |
| Peak RSS | 15.4 MiB | 104.5 MiB | 237.7 MiB | 20.2 MiB | 24.3 MiB | 21.8 MiB |
| Logical throughput | 56315.6 records/s | 14911.4 records/s | 8772.6 records/s | 67693.0 records/s | 53913.3 records/s | 59860.1 records/s |
| Physical throughput | 38.2 MiB/s | 10.1 MiB/s | 5.9 MiB/s | 45.9 MiB/s | 36.5 MiB/s | 48.0 MiB/s |
| Output bytes | 2419820.0 B | 1578672.0 B | 1578672.0 B | 1872419.0 B | 1872419.0 B | 1872419.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
