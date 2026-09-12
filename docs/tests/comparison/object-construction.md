---
type: Report
title: "Object construction"
description: "Measures building a lookup object with computed feature IDs as keys."
workload: benchmark.object-construction
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Object construction

## What this measures

The query turns the feature collection into an object whose keys come from
`.id` and whose values are magnitudes. It exercises computed keys, repeated
merges, and a blocking object result.

## Why it matters

Lookup maps are a common boundary between raw records and application code.
They also stress a different memory pattern from an output array.

## Input and output

The input is a natural feature snapshot with IDs and magnitudes. The output is
one object mapping each ID to its magnitude. It is not a list of key-value
records.

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
| Wall time | 3.1 ms | 17.6 ms | 25.5 ms | 4.5 ms | 5.5 ms | 4.8 ms |
| Wall time dispersion | 0.1 ms / 3.6 ms / 2.8-3.6 ms | 0.2 ms / 18.4 ms / 16.5-18.4 ms | 0.3 ms / 27.6 ms / 24.7-27.6 ms | 0.1 ms / 5.0 ms / 4.3-5.0 ms | 0.0 ms / 5.9 ms / 5.1-5.9 ms | 0.2 ms / 5.3 ms / 4.6-5.3 ms |
| First output | 2.9 ms | 17.5 ms | 25.3 ms | 4.2 ms | 5.2 ms | 4.6 ms |
| First output dispersion | 0.1 ms / 3.3 ms / 2.6-3.3 ms | 0.2 ms / 18.3 ms / 16.4-18.3 ms | 0.1 ms / 26.3 ms / 24.6-26.3 ms | 0.2 ms / 4.5 ms / 4.0-4.5 ms | 0.1 ms / 5.6 ms / 4.8-5.6 ms | 0.2 ms / 5.1 ms / 4.3-5.1 ms |
| User CPU | 2.0 ms | 17.9 ms | 26.6 ms | 2.1 ms | 3.8 ms | 2.8 ms |
| System CPU | 1.0 ms | 11.4 ms | 16.3 ms | 2.1 ms | 1.6 ms | 1.9 ms |
| Peak RSS | 4.5 MiB | 24.9 MiB | 38.3 MiB | 9.2 MiB | 9.8 MiB | 9.2 MiB |
| Logical throughput | 63017.7 records/s | 10996.8 records/s | 7618.6 records/s | 43115.9 records/s | 35527.9 records/s | 40492.6 records/s |
| Physical throughput | 43.1 MiB/s | 7.5 MiB/s | 5.2 MiB/s | 29.5 MiB/s | 24.3 MiB/s | 32.8 MiB/s |
| Output bytes | 4324.0 B | 3547.0 B | 3547.0 B | 3351.0 B | 3351.0 B | 3351.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | 3.1 ms | 3.1 ms | 1.2 ms | 1.3 ms | 1.2 ms |
| Wall time dispersion | 0.0 ms / 1.8 ms / 1.3-1.8 ms | 0.1 ms / 3.4 ms / 2.9-3.7 ms | 0.1 ms / 3.4 ms / 2.9-3.5 ms | 0.0 ms / 1.5 ms / 1.1-1.7 ms | 0.1 ms / 1.5 ms / 1.2-1.6 ms | 0.1 ms / 1.5 ms / 1.1-1.5 ms |
| First output | 1.4 ms | 2.7 ms | 2.8 ms | 1.0 ms | 1.0 ms | 1.0 ms |
| First output dispersion | 0.0 ms / 1.7 ms / 1.2-1.7 ms | 0.1 ms / 3.1 ms / 2.4-3.2 ms | 0.0 ms / 3.1 ms / 2.6-3.1 ms | 0.0 ms / 1.4 ms / 1.0-1.4 ms | 0.0 ms / 1.4 ms / 1.0-1.5 ms | 0.0 ms / 1.4 ms / 1.0-1.4 ms |
| User CPU | 1.3 ms | 1.0 ms | 1.1 ms | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.0 ms | 2.0 ms | 2.1 ms | 1.1 ms | 1.2 ms | 1.2 ms |
| Peak RSS | 4.1 MiB | 11.9 MiB | 12.1 MiB | 8.0 MiB | 8.5 MiB | 8.1 MiB |
| Logical throughput | 1344.5 records/s | 651.6 records/s | 646.4 records/s | 1679.3 records/s | 1504.9 records/s | 1622.7 records/s |
| Physical throughput | 1.1 MiB/s | 0.6 MiB/s | 0.5 MiB/s | 1.4 MiB/s | 1.3 MiB/s | 1.6 MiB/s |
| Output bytes | 47.0 B | 38.0 B | 38.0 B | 34.0 B | 34.0 B | 34.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `process exited with classified error Resource (exit status 5)`, `tool identity differs`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 111.3 ms | 23812.7 ms | 22755.4 ms | - | - | - |
| Wall time dispersion | 0.5 ms / 113.9 ms / 110.4-113.9 ms | 24.9 ms / 23938.3 ms / 23775.6-23938.3 ms | 167.3 ms / 23182.8 ms / 22450.2-23182.8 ms | - | - | - |
| First output | 104.9 ms | 23781.1 ms | 22671.8 ms | - | - | - |
| First output dispersion | 0.6 ms / 108.1 ms / 104.2-108.1 ms | 25.4 ms / 23905.7 ms / 23743.6-23905.7 ms | 165.6 ms / 23094.1 ms / 22368.2-23094.1 ms | - | - | - |
| User CPU | 87.4 ms | 54849.7 ms | 50771.0 ms | - | - | - |
| System CPU | 24.5 ms | 343.8 ms | 657.5 ms | - | - | - |
| Peak RSS | 65.8 MiB | 491.4 MiB | 1475.8 MiB | - | - | - |
| Logical throughput | 101296.1 records/s | 473.4 records/s | 495.4 records/s | - | - | - |
| Physical throughput | 68.7 MiB/s | 0.3 MiB/s | 0.3 MiB/s | - | - | - |
| Output bytes | 251352.0 B | 206255.0 B | 206255.0 B | - | - | - |
| Outcome | timed | timed | timed | resource-limit | resource-limit | resource-limit |
| Details | none | none | none | process exited with classified error Resource (exit status 5) | process exited with classified error Resource (exit status 5) | process exited with classified error Resource (exit status 5) |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | not timed | not timed | not timed |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 21.9 ms | 739.1 ms | 825.4 ms | 102.2 ms | 109.3 ms | 105.5 ms |
| Wall time dispersion | 0.7 ms / 23.0 ms / 20.6-23.0 ms | 1.7 ms / 750.5 ms / 733.7-750.5 ms | 2.8 ms / 831.4 ms / 807.4-831.4 ms | 0.6 ms / 103.3 ms / 100.2-103.3 ms | 1.0 ms / 115.3 ms / 107.1-115.3 ms | 0.4 ms / 107.7 ms / 103.7-107.7 ms |
| First output | 20.7 ms | 731.0 ms | 807.7 ms | 101.2 ms | 108.1 ms | 104.5 ms |
| First output dispersion | 0.5 ms / 21.8 ms / 19.5-21.8 ms | 1.3 ms / 744.2 ms / 726.1-744.2 ms | 3.3 ms / 811.9 ms / 791.9-811.9 ms | 0.5 ms / 102.0 ms / 99.2-102.0 ms | 0.8 ms / 114.1 ms / 105.7-114.1 ms | 0.4 ms / 106.5 ms / 102.6-106.5 ms |
| User CPU | 15.7 ms | 2134.3 ms | 2129.9 ms | 94.4 ms | 100.9 ms | 98.5 ms |
| System CPU | 5.1 ms | 58.5 ms | 128.2 ms | 7.1 ms | 8.0 ms | 6.6 ms |
| Peak RSS | 15.6 MiB | 110.5 MiB | 283.1 MiB | 21.3 MiB | 25.6 MiB | 23.2 MiB |
| Logical throughput | 101691.0 records/s | 3010.4 records/s | 2695.5 records/s | 21764.4 records/s | 20353.6 records/s | 21098.9 records/s |
| Physical throughput | 68.9 MiB/s | 2.0 MiB/s | 1.8 MiB/s | 14.7 MiB/s | 13.8 MiB/s | 16.9 MiB/s |
| Output bytes | 49953.0 B | 41052.0 B | 41052.0 B | 38825.0 B | 38825.0 B | 38825.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
