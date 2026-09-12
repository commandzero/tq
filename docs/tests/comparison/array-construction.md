---
type: Report
title: "Array construction"
description: "Measures building one array of compact objects from a feature collection."
workload: benchmark.array-construction
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Array construction

## What this measures

The query maps every feature to `{id, mag}` and wraps the results in one array.
It measures collection growth and object creation in a blocking pipeline.

## Why it matters

APIs and batch jobs often need a compact array for one downstream request. The
cost includes retaining all projected values until the array is complete.

## Input and output

The input is a natural feature snapshot. The output is one array with one
object per feature, containing only `id` and `mag`, rather than a stream of
individual objects.

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
| Wall time | 3.1 ms | 13.8 ms | 21.7 ms | 3.9 ms | 4.5 ms | 4.2 ms |
| Wall time dispersion | 0.1 ms / 3.5 ms / 2.9-3.5 ms | 0.3 ms / 14.3 ms / 13.2-14.3 ms | 0.3 ms / 22.2 ms / 21.1-22.2 ms | 0.1 ms / 4.2 ms / 3.7-4.2 ms | 0.2 ms / 5.0 ms / 4.2-5.0 ms | 0.2 ms / 4.5 ms / 3.9-4.5 ms |
| First output | 2.9 ms | 13.7 ms | 21.6 ms | 3.7 ms | 4.3 ms | 3.9 ms |
| First output dispersion | 0.1 ms / 3.2 ms / 2.8-3.2 ms | 0.2 ms / 14.1 ms / 13.2-14.1 ms | 0.2 ms / 22.0 ms / 21.1-22.0 ms | 0.1 ms / 3.9 ms / 3.5-3.9 ms | 0.2 ms / 4.6 ms / 4.0-4.6 ms | 0.1 ms / 4.4 ms / 3.7-4.4 ms |
| User CPU | 2.9 ms | 11.9 ms | 18.9 ms | 2.9 ms | 3.3 ms | 2.6 ms |
| System CPU | 0.0 ms | 7.6 ms | 14.2 ms | 0.9 ms | 1.2 ms | 1.6 ms |
| Peak RSS | 4.4 MiB | 22.8 MiB | 33.1 MiB | 9.2 MiB | 9.8 MiB | 9.2 MiB |
| Logical throughput | 62834.0 records/s | 14023.9 records/s | 8929.2 records/s | 49351.3 records/s | 42887.1 records/s | 46190.5 records/s |
| Physical throughput | 43.0 MiB/s | 9.6 MiB/s | 6.1 MiB/s | 33.8 MiB/s | 29.3 MiB/s | 37.4 MiB/s |
| Output bytes | 9562.0 B | 6069.0 B | 6069.0 B | 3560.0 B | 3560.0 B | 3560.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | 3.2 ms | 3.1 ms | 1.3 ms | 1.3 ms | 1.3 ms |
| Wall time dispersion | 0.1 ms / 1.8 ms / 1.3-2.0 ms | 0.1 ms / 3.4 ms / 2.9-3.6 ms | 0.1 ms / 3.5 ms / 2.9-3.6 ms | 0.0 ms / 1.4 ms / 1.1-1.5 ms | 0.1 ms / 1.5 ms / 1.1-1.5 ms | 0.1 ms / 1.5 ms / 1.1-1.5 ms |
| First output | 1.4 ms | 2.7 ms | 2.8 ms | 1.0 ms | 1.0 ms | 1.0 ms |
| First output dispersion | 0.0 ms / 1.7 ms / 1.2-1.8 ms | 0.0 ms / 2.9 ms / 2.5-3.1 ms | 0.1 ms / 3.1 ms / 2.6-3.1 ms | 0.0 ms / 1.2 ms / 1.0-1.3 ms | 0.0 ms / 1.3 ms / 1.0-1.4 ms | 0.0 ms / 1.3 ms / 1.0-1.3 ms |
| User CPU | 1.3 ms | 1.1 ms | 1.1 ms | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.0 ms | 2.0 ms | 2.1 ms | 1.1 ms | 1.1 ms | 1.2 ms |
| Peak RSS | 4.1 MiB | 12.0 MiB | 12.3 MiB | 8.1 MiB | 8.5 MiB | 8.0 MiB |
| Logical throughput | 1324.9 records/s | 630.8 records/s | 639.1 records/s | 1522.1 records/s | 1512.3 records/s | 1498.7 records/s |
| Physical throughput | 1.1 MiB/s | 0.5 MiB/s | 0.5 MiB/s | 1.3 MiB/s | 1.3 MiB/s | 1.5 MiB/s |
| Output bytes | 101.0 B | 64.0 B | 64.0 B | 49.0 B | 49.0 B | 49.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 114.7 ms | 559.5 ms | 1157.8 ms | 147.5 ms | 190.6 ms | 166.6 ms |
| Wall time dispersion | 1.2 ms / 117.2 ms / 112.1-117.2 ms | 5.2 ms / 565.0 ms / 546.8-565.0 ms | 28.0 ms / 1194.1 ms / 1086.3-1194.1 ms | 1.2 ms / 149.5 ms / 145.0-149.5 ms | 0.6 ms / 193.2 ms / 189.6-193.2 ms | 1.0 ms / 172.4 ms / 165.1-172.4 ms |
| First output | 103.8 ms | 531.6 ms | 1088.1 ms | 125.0 ms | 167.9 ms | 142.2 ms |
| First output dispersion | 1.0 ms / 105.8 ms / 101.4-105.8 ms | 3.4 ms / 536.8 ms / 522.0-536.8 ms | 22.1 ms / 1115.2 ms / 1025.8-1115.2 ms | 0.5 ms / 127.3 ms / 122.7-127.3 ms | 0.8 ms / 170.6 ms / 166.1-170.6 ms | 0.8 ms / 148.1 ms / 140.1-148.1 ms |
| User CPU | 86.8 ms | 647.5 ms | 1721.3 ms | 119.5 ms | 155.7 ms | 136.1 ms |
| System CPU | 26.7 ms | 174.3 ms | 484.2 ms | 27.9 ms | 35.4 ms | 31.5 ms |
| Peak RSS | 64.6 MiB | 474.4 MiB | 1310.3 MiB | 74.2 MiB | 94.2 MiB | 82.9 MiB |
| Logical throughput | 98305.8 records/s | 20151.3 records/s | 9737.3 records/s | 76427.9 records/s | 59159.5 records/s | 67687.1 records/s |
| Physical throughput | 66.6 MiB/s | 13.7 MiB/s | 6.6 MiB/s | 51.8 MiB/s | 40.0 MiB/s | 54.3 MiB/s |
| Output bytes | 555750.0 B | 352817.0 B | 352817.0 B | 206270.0 B | 206270.0 B | 206270.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 22.0 ms | 114.5 ms | 213.7 ms | 29.8 ms | 36.8 ms | 32.9 ms |
| Wall time dispersion | 0.8 ms / 24.2 ms / 20.2-24.2 ms | 1.2 ms / 116.5 ms / 111.1-116.5 ms | 6.3 ms / 225.5 ms / 204.7-225.5 ms | 0.4 ms / 30.9 ms / 29.3-30.9 ms | 0.4 ms / 37.6 ms / 35.8-37.6 ms | 0.4 ms / 34.0 ms / 31.2-34.0 ms |
| First output | 20.2 ms | 106.9 ms | 199.8 ms | 28.8 ms | 35.7 ms | 32.0 ms |
| First output dispersion | 0.6 ms / 22.4 ms / 18.5-22.4 ms | 0.9 ms / 109.5 ms / 104.8-109.5 ms | 5.0 ms / 211.0 ms / 193.6-211.0 ms | 0.4 ms / 30.0 ms / 28.4-30.0 ms | 0.3 ms / 36.2 ms / 34.8-36.2 ms | 0.5 ms / 33.1 ms / 30.4-33.1 ms |
| User CPU | 17.3 ms | 126.3 ms | 295.9 ms | 23.1 ms | 27.7 ms | 26.4 ms |
| System CPU | 5.0 ms | 43.4 ms | 86.0 ms | 6.6 ms | 8.9 ms | 6.2 ms |
| Peak RSS | 15.3 MiB | 112.6 MiB | 237.5 MiB | 21.1 MiB | 25.1 MiB | 22.7 MiB |
| Logical throughput | 101049.1 records/s | 19437.7 records/s | 10412.6 records/s | 74545.6 records/s | 60467.7 records/s | 67538.9 records/s |
| Physical throughput | 68.5 MiB/s | 13.2 MiB/s | 7.0 MiB/s | 50.5 MiB/s | 40.9 MiB/s | 54.2 MiB/s |
| Output bytes | 110028.0 B | 69977.0 B | 69977.0 B | 41066.0 B | 41066.0 B | 41066.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
