---
type: Report
title: "JSON text formatting"
description: "Measures serializing each feature projection as compact JSON text."
workload: benchmark.format-json
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
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
| Wall time | 5.3 ms | 20.0 ms | 27.0 ms | 6.0 ms | 6.6 ms | 6.1 ms |
| Wall time dispersion | 0.2 ms / 5.9 ms / 5.0-5.9 ms | 0.2 ms / 20.6 ms / 19.3-20.6 ms | 0.5 ms / 29.7 ms / 25.9-29.7 ms | 0.1 ms / 6.5 ms / 5.8-6.5 ms | 0.1 ms / 6.8 ms / 6.4-6.8 ms | 0.1 ms / 6.9 ms / 5.9-6.9 ms |
| First output | 2.9 ms | 17.9 ms | 25.0 ms | 4.3 ms | 4.8 ms | 4.5 ms |
| First output dispersion | 0.0 ms / 3.0 ms / 2.6-3.0 ms | 0.2 ms / 18.3 ms / 17.2-18.3 ms | 0.2 ms / 25.6 ms / 23.9-25.6 ms | 0.2 ms / 4.6 ms / 4.0-4.6 ms | 0.1 ms / 5.1 ms / 4.7-5.1 ms | 0.1 ms / 4.8 ms / 4.3-4.8 ms |
| User CPU | 4.3 ms | 18.7 ms | 26.4 ms | 4.0 ms | 4.7 ms | 3.6 ms |
| System CPU | 1.1 ms | 9.0 ms | 11.7 ms | 2.0 ms | 1.7 ms | 2.3 ms |
| Peak RSS | 4.4 MiB | 27.5 MiB | 35.3 MiB | 9.2 MiB | 10.1 MiB | 9.2 MiB |
| Logical throughput | 36590.0 records/s | 9703.6 records/s | 7182.5 records/s | 32409.0 records/s | 29613.8 records/s | 31963.1 records/s |
| Physical throughput | 25.0 MiB/s | 6.6 MiB/s | 4.9 MiB/s | 22.2 MiB/s | 20.2 MiB/s | 25.9 MiB/s |
| Output bytes | 132463.0 B | 132463.0 B | 132463.0 B | 132463.0 B | 132463.0 B | 132463.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | 3.2 ms | 3.2 ms | 1.3 ms | 1.3 ms | 1.2 ms |
| Wall time dispersion | 0.0 ms / 1.8 ms / 1.3-1.8 ms | 0.1 ms / 3.5 ms / 2.9-3.6 ms | 0.1 ms / 3.4 ms / 2.9-3.4 ms | 0.0 ms / 1.5 ms / 1.1-1.7 ms | 0.0 ms / 1.7 ms / 1.2-1.7 ms | 0.0 ms / 1.5 ms / 1.1-1.6 ms |
| First output | 1.3 ms | 2.7 ms | 2.8 ms | 1.0 ms | 1.0 ms | 1.0 ms |
| First output dispersion | 0.0 ms / 1.6 ms / 1.2-1.7 ms | 0.1 ms / 3.1 ms / 2.6-3.1 ms | 0.0 ms / 2.9 ms / 2.6-3.1 ms | 0.0 ms / 1.3 ms / 1.0-1.4 ms | 0.0 ms / 1.4 ms / 1.0-1.4 ms | 0.0 ms / 1.3 ms / 1.0-1.3 ms |
| User CPU | 1.3 ms | 1.1 ms | 1.1 ms | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.0 ms | 2.0 ms | 2.2 ms | 1.1 ms | 1.2 ms | 1.1 ms |
| Peak RSS | 4.1 MiB | 12.0 MiB | 12.5 MiB | 8.2 MiB | 8.5 MiB | 8.1 MiB |
| Logical throughput | 1350.4 records/s | 625.1 records/s | 622.2 records/s | 1519.8 records/s | 1510.6 records/s | 1680.7 records/s |
| Physical throughput | 1.1 MiB/s | 0.5 MiB/s | 0.5 MiB/s | 1.3 MiB/s | 1.3 MiB/s | 1.6 MiB/s |
| Output bytes | 1353.0 B | 1353.0 B | 1353.0 B | 1353.0 B | 1353.0 B | 1353.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 225.3 ms | 974.4 ms | 1489.7 ms | 256.2 ms | 299.1 ms | 274.5 ms |
| Wall time dispersion | 1.2 ms / 228.4 ms / 222.9-228.4 ms | 7.3 ms / 982.4 ms / 952.7-982.4 ms | 22.1 ms / 1551.8 ms / 1466.1-1551.8 ms | 1.3 ms / 258.3 ms / 250.6-258.3 ms | 1.9 ms / 303.4 ms / 293.2-303.4 ms | 1.5 ms / 280.7 ms / 271.7-280.7 ms |
| First output | 80.2 ms | 826.5 ms | 1308.9 ms | 103.8 ms | 145.3 ms | 120.7 ms |
| First output dispersion | 0.6 ms / 81.8 ms / 79.0-81.8 ms | 3.7 ms / 847.5 ms / 806.3-847.5 ms | 22.5 ms / 1373.9 ms / 1284.1-1373.9 ms | 0.6 ms / 107.3 ms / 101.9-107.3 ms | 1.2 ms / 149.9 ms / 142.8-149.9 ms | 0.5 ms / 122.8 ms / 118.6-122.8 ms |
| User CPU | 195.1 ms | 1038.8 ms | 2023.2 ms | 189.9 ms | 232.7 ms | 210.4 ms |
| System CPU | 30.0 ms | 315.5 ms | 552.8 ms | 66.1 ms | 66.2 ms | 63.6 ms |
| Peak RSS | 63.7 MiB | 832.6 MiB | 1528.0 MiB | 69.6 MiB | 89.6 MiB | 78.3 MiB |
| Logical throughput | 50045.8 records/s | 11570.1 records/s | 7568.2 records/s | 44012.6 records/s | 37695.2 records/s | 41064.3 records/s |
| Physical throughput | 33.9 MiB/s | 7.8 MiB/s | 5.1 MiB/s | 29.8 MiB/s | 25.5 MiB/s | 33.0 MiB/s |
| Output bytes | 7670840.0 B | 7670840.0 B | 7670840.0 B | 7670840.0 B | 7670840.0 B | 7670840.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 44.8 ms | 192.9 ms | 293.0 ms | 50.1 ms | 56.8 ms | 53.3 ms |
| Wall time dispersion | 0.6 ms / 45.8 ms / 43.1-45.8 ms | 2.5 ms / 205.5 ms / 186.4-205.5 ms | 3.0 ms / 298.9 ms / 288.2-298.9 ms | 0.9 ms / 54.0 ms / 48.2-54.0 ms | 0.9 ms / 58.6 ms / 55.4-58.6 ms | 0.6 ms / 55.1 ms / 51.8-55.1 ms |
| First output | 17.0 ms | 164.5 ms | 255.0 ms | 22.7 ms | 30.1 ms | 26.1 ms |
| First output dispersion | 0.4 ms / 18.1 ms / 16.4-18.1 ms | 2.0 ms / 171.0 ms / 161.6-171.0 ms | 1.9 ms / 260.4 ms / 251.7-260.4 ms | 0.4 ms / 24.1 ms / 22.2-24.1 ms | 0.3 ms / 31.4 ms / 28.8-31.4 ms | 0.3 ms / 26.6 ms / 25.1-26.6 ms |
| User CPU | 38.8 ms | 204.5 ms | 349.0 ms | 36.5 ms | 41.9 ms | 39.3 ms |
| System CPU | 6.0 ms | 66.8 ms | 114.6 ms | 12.9 ms | 15.2 ms | 13.7 ms |
| Peak RSS | 15.2 MiB | 177.6 MiB | 267.6 MiB | 20.3 MiB | 24.1 MiB | 21.8 MiB |
| Logical throughput | 49691.8 records/s | 11536.8 records/s | 7594.3 records/s | 44413.0 records/s | 39182.5 records/s | 41774.2 records/s |
| Physical throughput | 33.7 MiB/s | 7.8 MiB/s | 5.1 MiB/s | 30.1 MiB/s | 26.5 MiB/s | 33.5 MiB/s |
| Output bytes | 1512411.0 B | 1512411.0 B | 1512411.0 B | 1512411.0 B | 1512411.0 B | 1512411.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
