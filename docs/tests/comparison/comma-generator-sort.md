---
type: Report
title: "Sort by multiple keys"
description: "Measures sorting feature objects by magnitude and then ID."
workload: benchmark.comma-generator-sort
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Sort by multiple keys

## What this measures

The query sorts the feature objects by `.properties.mag` and `.id`. The comma
generator supplies a second key for ties, so the whole collection is retained
until ordering is known.

## Why it matters

Stable tie-breaking makes reports and pagination repeatable. It also costs more
than sorting a single scalar because the result keeps complete feature objects.

## Input and output

The input is a natural feature snapshot. The output is one array of the
original feature objects, ordered by magnitude and then ID, rather than an
array of just the sort keys.

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
| Wall time | 4.8 ms | 20.2 ms | 26.1 ms | 4.8 ms | 5.5 ms | 5.0 ms |
| Wall time dispersion | 0.1 ms / 5.3 ms / 4.5-5.3 ms | 0.3 ms / 20.7 ms / 19.6-20.7 ms | 0.2 ms / 27.8 ms / 25.6-27.8 ms | 0.1 ms / 5.2 ms / 4.5-5.2 ms | 0.1 ms / 5.7 ms / 5.1-5.7 ms | 0.0 ms / 5.4 ms / 4.7-5.4 ms |
| First output | 2.9 ms | 20.1 ms | 26.0 ms | 3.7 ms | 4.5 ms | 4.0 ms |
| First output dispersion | 0.1 ms / 3.3 ms / 2.6-3.3 ms | 0.3 ms / 20.6 ms / 19.3-20.6 ms | 0.2 ms / 27.6 ms / 25.6-27.6 ms | 0.2 ms / 4.0 ms / 3.5-4.0 ms | 0.1 ms / 4.7 ms / 4.2-4.7 ms | 0.0 ms / 4.2 ms / 3.7-4.2 ms |
| User CPU | 3.3 ms | 20.9 ms | 25.4 ms | 3.0 ms | 3.5 ms | 3.6 ms |
| System CPU | 1.5 ms | 7.5 ms | 12.1 ms | 1.6 ms | 1.9 ms | 1.2 ms |
| Peak RSS | 4.5 MiB | 27.3 MiB | 35.1 MiB | 9.2 MiB | 9.8 MiB | 9.1 MiB |
| Logical throughput | 40679.4 records/s | 9582.1 records/s | 7445.4 records/s | 40315.9 records/s | 35122.7 records/s | 39188.0 records/s |
| Physical throughput | 27.8 MiB/s | 6.6 MiB/s | 5.1 MiB/s | 27.6 MiB/s | 24.0 MiB/s | 31.7 MiB/s |
| Output bytes | 196577.0 B | 138764.0 B | 138764.0 B | 164367.0 B | 164367.0 B | 164367.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | 3.1 ms | 3.2 ms | 1.3 ms | 1.3 ms | 1.2 ms |
| Wall time dispersion | 0.0 ms / 1.7 ms / 1.3-1.7 ms | 0.1 ms / 3.5 ms / 2.7-3.6 ms | 0.0 ms / 3.6 ms / 3.1-3.8 ms | 0.1 ms / 1.5 ms / 1.1-1.5 ms | 0.0 ms / 1.6 ms / 1.2-1.6 ms | 0.0 ms / 1.5 ms / 1.1-1.5 ms |
| First output | 1.3 ms | 2.7 ms | 2.8 ms | 1.0 ms | 1.0 ms | 1.0 ms |
| First output dispersion | 0.0 ms / 1.5 ms / 1.2-1.5 ms | 0.1 ms / 3.2 ms / 2.5-3.2 ms | 0.0 ms / 3.1 ms / 2.6-3.3 ms | 0.0 ms / 1.3 ms / 1.0-1.3 ms | 0.0 ms / 1.3 ms / 1.0-1.3 ms | 0.0 ms / 1.3 ms / 1.0-1.3 ms |
| User CPU | 0.7 ms | 1.1 ms | 1.1 ms | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.7 ms | 2.0 ms | 2.2 ms | 1.1 ms | 1.2 ms | 1.1 ms |
| Peak RSS | 4.1 MiB | 12.0 MiB | 12.3 MiB | 8.1 MiB | 8.4 MiB | 8.0 MiB |
| Logical throughput | 1348.2 records/s | 635.7 records/s | 620.6 records/s | 1549.2 records/s | 1503.8 records/s | 1675.0 records/s |
| Physical throughput | 1.1 MiB/s | 0.5 MiB/s | 0.5 MiB/s | 1.3 MiB/s | 1.3 MiB/s | 1.6 MiB/s |
| Output bytes | 2038.0 B | 1441.0 B | 1441.0 B | 1710.0 B | 1710.0 B | 1710.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 216.3 ms | 983.0 ms | 1512.5 ms | 207.1 ms | 249.8 ms | 226.5 ms |
| Wall time dispersion | 1.6 ms / 222.2 ms / 213.7-222.2 ms | 5.7 ms / 1017.9 ms / 971.4-1017.9 ms | 16.9 ms / 1571.0 ms / 1484.8-1571.0 ms | 2.8 ms / 211.7 ms / 202.9-211.7 ms | 1.4 ms / 254.1 ms / 245.9-254.1 ms | 1.8 ms / 231.6 ms / 221.7-231.6 ms |
| First output | 101.7 ms | 941.3 ms | 1435.5 ms | 134.5 ms | 175.1 ms | 150.1 ms |
| First output dispersion | 1.5 ms / 105.9 ms / 98.9-105.9 ms | 5.6 ms / 967.6 ms / 928.4-967.6 ms | 11.6 ms / 1485.5 ms / 1408.2-1485.5 ms | 1.0 ms / 140.2 ms / 131.9-140.2 ms | 0.9 ms / 180.7 ms / 173.6-180.7 ms | 1.0 ms / 153.5 ms / 147.5-153.5 ms |
| User CPU | 182.0 ms | 1184.0 ms | 2113.9 ms | 175.4 ms | 217.3 ms | 194.1 ms |
| System CPU | 34.8 ms | 267.5 ms | 501.2 ms | 28.1 ms | 33.0 ms | 32.5 ms |
| Peak RSS | 64.7 MiB | 851.4 MiB | 1528.0 MiB | 72.7 MiB | 93.3 MiB | 81.8 MiB |
| Logical throughput | 52118.8 records/s | 11469.5 records/s | 7454.0 records/s | 54442.1 records/s | 45134.5 records/s | 49769.3 records/s |
| Physical throughput | 35.3 MiB/s | 7.8 MiB/s | 5.0 MiB/s | 36.9 MiB/s | 30.6 MiB/s | 40.0 MiB/s |
| Output bytes | 11361260.0 B | 8001607.0 B | 8001607.0 B | 9490694.0 B | 9490694.0 B | 9490694.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 41.1 ms | 194.9 ms | 294.2 ms | 38.4 ms | 45.5 ms | 41.7 ms |
| Wall time dispersion | 0.8 ms / 43.8 ms / 39.4-43.8 ms | 1.1 ms / 199.1 ms / 191.8-199.1 ms | 3.7 ms / 299.1 ms / 288.2-299.1 ms | 0.4 ms / 40.4 ms / 37.7-40.4 ms | 0.4 ms / 47.3 ms / 44.2-47.3 ms | 0.5 ms / 42.6 ms / 39.9-42.6 ms |
| First output | 20.0 ms | 185.3 ms | 278.1 ms | 27.0 ms | 34.2 ms | 30.0 ms |
| First output dispersion | 0.5 ms / 21.5 ms / 18.8-21.5 ms | 0.9 ms / 188.8 ms / 181.3-188.8 ms | 4.0 ms / 282.7 ms / 272.7-282.7 ms | 0.3 ms / 28.7 ms / 26.0-28.7 ms | 0.2 ms / 35.1 ms / 33.1-35.1 ms | 0.3 ms / 31.2 ms / 28.9-31.2 ms |
| User CPU | 35.4 ms | 222.9 ms | 365.1 ms | 31.3 ms | 37.1 ms | 33.7 ms |
| System CPU | 6.4 ms | 62.4 ms | 103.8 ms | 7.0 ms | 8.1 ms | 7.7 ms |
| Peak RSS | 15.4 MiB | 177.8 MiB | 267.5 MiB | 20.9 MiB | 25.2 MiB | 22.4 MiB |
| Logical throughput | 54149.4 records/s | 11418.5 records/s | 7563.6 records/s | 57894.5 records/s | 48853.9 records/s | 53382.9 records/s |
| Physical throughput | 36.7 MiB/s | 7.7 MiB/s | 5.1 MiB/s | 39.2 MiB/s | 33.1 MiB/s | 42.8 MiB/s |
| Output bytes | 2241402.0 B | 1578351.0 B | 1578351.0 B | 1872104.0 B | 1872104.0 B | 1872104.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
