---
type: Report
title: "Grouping a collection"
description: "Measures grouping features by magnitude and counting the groups."
workload: benchmark.issue5-collection
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Grouping a collection

## What this measures

The query groups the feature collection by `.properties.mag` and returns the
number of groups. It exercises sorting and collection grouping even though the
final result is only a count.

## Why it matters

Grouping supports summaries such as counts by severity or category. It must
keep enough information to form complete groups before reporting the count.

## Input and output

The input is a natural feature snapshot. The output is one integer for the
number of distinct magnitude groups, not the grouped arrays themselves.

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
| Wall time | 3.1 ms | 13.3 ms | 20.4 ms | 3.8 ms | 4.5 ms | 4.2 ms |
| Wall time dispersion | 0.2 ms / 3.5 ms / 2.8-3.5 ms | 0.2 ms / 13.7 ms / 12.5-13.7 ms | 0.1 ms / 20.9 ms / 20.0-20.9 ms | 0.1 ms / 4.3 ms / 3.7-4.3 ms | 0.1 ms / 4.8 ms / 4.2-4.8 ms | 0.1 ms / 4.4 ms / 3.9-4.4 ms |
| First output | 2.9 ms | 13.1 ms | 20.3 ms | 3.6 ms | 4.3 ms | 3.9 ms |
| First output dispersion | 0.2 ms / 3.3 ms / 2.6-3.3 ms | 0.2 ms / 13.5 ms / 12.3-13.5 ms | 0.1 ms / 20.8 ms / 20.0-20.8 ms | 0.1 ms / 4.0 ms / 3.4-4.0 ms | 0.1 ms / 4.5 ms / 3.9-4.5 ms | 0.1 ms / 4.0 ms / 3.7-4.0 ms |
| User CPU | 2.4 ms | 15.1 ms | 20.7 ms | 2.0 ms | 2.5 ms | 2.7 ms |
| System CPU | 0.5 ms | 6.4 ms | 10.9 ms | 1.9 ms | 1.7 ms | 1.4 ms |
| Peak RSS | 4.4 MiB | 24.9 MiB | 34.3 MiB | 8.9 MiB | 9.6 MiB | 9.1 MiB |
| Logical throughput | 63151.0 records/s | 14581.5 records/s | 9497.9 records/s | 50428.9 records/s | 42991.7 records/s | 46730.1 records/s |
| Physical throughput | 43.2 MiB/s | 10.0 MiB/s | 6.5 MiB/s | 34.5 MiB/s | 29.4 MiB/s | 37.8 MiB/s |
| Output bytes | 4.0 B | 4.0 B | 4.0 B | 4.0 B | 4.0 B | 4.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | 3.1 ms | 3.1 ms | 1.3 ms | 1.3 ms | 1.2 ms |
| Wall time dispersion | 0.1 ms / 1.7 ms / 1.3-1.8 ms | 0.1 ms / 3.4 ms / 2.7-3.4 ms | 0.1 ms / 3.5 ms / 2.7-3.5 ms | 0.1 ms / 1.5 ms / 1.1-1.5 ms | 0.1 ms / 1.5 ms / 1.2-1.6 ms | 0.1 ms / 1.5 ms / 1.2-1.5 ms |
| First output | 1.3 ms | 2.7 ms | 2.7 ms | 1.0 ms | 1.0 ms | 1.0 ms |
| First output dispersion | 0.0 ms / 1.7 ms / 1.2-1.7 ms | 0.1 ms / 3.0 ms / 2.4-3.1 ms | 0.1 ms / 3.0 ms / 2.6-3.0 ms | 0.0 ms / 1.4 ms / 1.0-1.4 ms | 0.0 ms / 1.3 ms / 1.0-1.3 ms | 0.0 ms / 1.2 ms / 1.0-1.4 ms |
| User CPU | 1.3 ms | 1.1 ms | 1.1 ms | 0.0 ms | 0.2 ms | 0.0 ms |
| System CPU | 0.0 ms | 1.9 ms | 2.2 ms | 1.1 ms | 1.0 ms | 1.1 ms |
| Peak RSS | 4.1 MiB | 12.0 MiB | 12.1 MiB | 8.1 MiB | 8.3 MiB | 7.9 MiB |
| Logical throughput | 1346.8 records/s | 644.1 records/s | 652.3 records/s | 1512.9 records/s | 1515.7 records/s | 1640.0 records/s |
| Physical throughput | 1.1 MiB/s | 0.5 MiB/s | 0.6 MiB/s | 1.3 MiB/s | 1.3 MiB/s | 1.6 MiB/s |
| Output bytes | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B | 2.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 113.0 ms | 564.5 ms | 1090.4 ms | 146.7 ms | 187.2 ms | 164.0 ms |
| Wall time dispersion | 0.9 ms / 115.1 ms / 111.5-115.1 ms | 4.4 ms / 572.5 ms / 547.6-572.5 ms | 8.3 ms / 1103.1 ms / 1067.1-1103.1 ms | 1.2 ms / 149.1 ms / 145.2-149.1 ms | 1.8 ms / 189.6 ms / 185.2-189.6 ms | 0.9 ms / 167.1 ms / 161.2-167.1 ms |
| First output | 109.3 ms | 528.9 ms | 1020.3 ms | 142.8 ms | 182.6 ms | 160.3 ms |
| First output dispersion | 0.9 ms / 110.9 ms / 108.1-110.9 ms | 3.4 ms / 535.2 ms / 516.5-535.2 ms | 3.0 ms / 1028.9 ms / 1001.4-1028.9 ms | 1.0 ms / 144.9 ms / 141.1-144.9 ms | 1.3 ms / 184.6 ms / 181.0-184.6 ms | 0.9 ms / 163.3 ms / 157.5-163.3 ms |
| User CPU | 87.1 ms | 779.3 ms | 1659.1 ms | 117.7 ms | 155.2 ms | 131.5 ms |
| System CPU | 26.0 ms | 213.9 ms | 478.2 ms | 27.5 ms | 32.8 ms | 31.7 ms |
| Peak RSS | 61.4 MiB | 615.3 MiB | 1357.8 MiB | 72.3 MiB | 93.0 MiB | 81.5 MiB |
| Logical throughput | 99746.5 records/s | 19970.0 records/s | 10339.1 records/s | 76834.5 records/s | 60236.4 records/s | 68750.8 records/s |
| Physical throughput | 67.6 MiB/s | 13.5 MiB/s | 7.0 MiB/s | 52.1 MiB/s | 40.8 MiB/s | 55.2 MiB/s |
| Output bytes | 4.0 B | 4.0 B | 4.0 B | 4.0 B | 4.0 B | 4.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 21.6 ms | 114.4 ms | 212.8 ms | 29.8 ms | 36.1 ms | 32.5 ms |
| Wall time dispersion | 0.2 ms / 23.6 ms / 21.1-23.6 ms | 1.9 ms / 121.0 ms / 111.7-121.0 ms | 3.2 ms / 217.8 ms / 206.8-217.8 ms | 0.6 ms / 31.7 ms / 28.6-31.7 ms | 0.2 ms / 37.3 ms / 33.9-37.3 ms | 0.6 ms / 33.9 ms / 31.3-33.9 ms |
| First output | 20.9 ms | 105.9 ms | 198.6 ms | 29.0 ms | 35.1 ms | 31.7 ms |
| First output dispersion | 0.2 ms / 22.8 ms / 20.5-22.8 ms | 1.2 ms / 110.9 ms / 103.8-110.9 ms | 2.3 ms / 202.0 ms / 193.3-202.0 ms | 0.6 ms / 30.8 ms / 27.8-30.8 ms | 0.3 ms / 36.1 ms / 33.1-36.1 ms | 0.6 ms / 33.1 ms / 30.4-33.1 ms |
| User CPU | 17.4 ms | 146.0 ms | 276.8 ms | 23.7 ms | 27.8 ms | 26.8 ms |
| System CPU | 4.5 ms | 50.9 ms | 103.8 ms | 5.9 ms | 7.9 ms | 6.1 ms |
| Peak RSS | 14.9 MiB | 141.0 MiB | 256.6 MiB | 20.5 MiB | 24.9 MiB | 22.0 MiB |
| Logical throughput | 102883.0 records/s | 19456.2 records/s | 10453.8 records/s | 74550.6 records/s | 61662.5 records/s | 68533.2 records/s |
| Physical throughput | 69.7 MiB/s | 13.2 MiB/s | 7.1 MiB/s | 50.5 MiB/s | 41.7 MiB/s | 55.0 MiB/s |
| Output bytes | 4.0 B | 4.0 B | 4.0 B | 4.0 B | 4.0 B | 4.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
