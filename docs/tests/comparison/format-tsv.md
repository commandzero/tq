---
type: Report
title: "TSV formatting"
description: "Measures formatting selected feature fields as tab-separated rows."
workload: benchmark.format-tsv
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# TSV formatting

## What this measures

The query selects `id`, `place`, and `mag` for each feature and applies `@tsv`.
It measures tab escaping and text output across the complete feature stream.

## Why it matters

Tab-separated rows are easy to pipe into Unix tools and import into tables.
Escaping embedded tabs and control characters is part of the real work.

## Input and output

The input is a natural feature snapshot. The output is one TSV string per
feature containing the three selected fields, not a JSON representation.

The yq adapter converts null magnitudes to empty fields, matching jq TSV text. Numeric zero remains a numeric field.

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
| Wall time | 3.2 ms | 13.1 ms | 21.5 ms | 5.2 ms | 5.8 ms | 5.5 ms |
| Wall time dispersion | 0.1 ms / 3.7 ms / 2.9-3.7 ms | 0.2 ms / 13.4 ms / 12.5-13.4 ms | 0.2 ms / 22.5 ms / 20.9-22.5 ms | 0.2 ms / 5.6 ms / 4.8-5.6 ms | 0.2 ms / 6.1 ms / 5.6-6.1 ms | 0.1 ms / 6.2 ms / 5.3-6.2 ms |
| First output | 2.7 ms | 11.1 ms | 20.3 ms | 4.9 ms | 5.5 ms | 5.3 ms |
| First output dispersion | 0.1 ms / 3.2 ms / 2.5-3.2 ms | 0.2 ms / 11.5 ms / 10.6-11.5 ms | 0.1 ms / 21.2 ms / 19.7-21.2 ms | 0.2 ms / 5.4 ms / 4.7-5.4 ms | 0.1 ms / 5.8 ms / 5.3-5.8 ms | 0.2 ms / 6.1 ms / 5.0-6.1 ms |
| User CPU | 2.1 ms | 10.4 ms | 20.5 ms | 3.0 ms | 3.2 ms | 3.6 ms |
| System CPU | 1.0 ms | 8.4 ms | 12.0 ms | 2.0 ms | 2.5 ms | 2.1 ms |
| Peak RSS | 4.3 MiB | 22.0 MiB | 32.9 MiB | 9.2 MiB | 9.7 MiB | 9.0 MiB |
| Logical throughput | 61450.7 records/s | 14810.3 records/s | 9012.8 records/s | 37118.5 records/s | 33622.2 records/s | 34955.0 records/s |
| Physical throughput | 42.1 MiB/s | 10.1 MiB/s | 6.2 MiB/s | 25.4 MiB/s | 23.0 MiB/s | 28.3 MiB/s |
| Output bytes | 9570.0 B | 9570.0 B | 9570.0 B | 9570.0 B | 9570.0 B | 9570.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | 3.2 ms | 3.2 ms | 1.3 ms | 1.3 ms | 1.2 ms |
| Wall time dispersion | 0.0 ms / 1.6 ms / 1.3-1.7 ms | 0.1 ms / 3.5 ms / 2.9-3.9 ms | 0.2 ms / 3.5 ms / 2.9-3.5 ms | 0.1 ms / 1.5 ms / 1.2-1.5 ms | 0.0 ms / 1.7 ms / 1.2-1.7 ms | 0.0 ms / 1.5 ms / 1.2-1.6 ms |
| First output | 1.4 ms | 2.7 ms | 2.8 ms | 1.0 ms | 1.0 ms | 1.0 ms |
| First output dispersion | 0.0 ms / 1.5 ms / 1.2-1.6 ms | 0.1 ms / 3.1 ms / 2.5-3.4 ms | 0.2 ms / 3.0 ms / 2.4-3.0 ms | 0.0 ms / 1.4 ms / 1.0-1.4 ms | 0.0 ms / 1.4 ms / 1.0-1.4 ms | 0.0 ms / 1.3 ms / 1.0-1.3 ms |
| User CPU | 0.6 ms | 1.1 ms | 1.1 ms | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.7 ms | 2.0 ms | 2.2 ms | 1.1 ms | 1.2 ms | 1.1 ms |
| Peak RSS | 4.0 MiB | 12.0 MiB | 12.0 MiB | 8.1 MiB | 8.4 MiB | 7.9 MiB |
| Logical throughput | 1330.2 records/s | 626.6 records/s | 616.5 records/s | 1545.6 records/s | 1504.9 records/s | 1681.4 records/s |
| Physical throughput | 1.1 MiB/s | 0.5 MiB/s | 0.5 MiB/s | 1.3 MiB/s | 1.3 MiB/s | 1.6 MiB/s |
| Output bytes | 94.0 B | 94.0 B | 94.0 B | 94.0 B | 94.0 B | 94.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 118.1 ms | 506.4 ms | 1052.9 ms | 215.0 ms | 258.2 ms | 236.5 ms |
| Wall time dispersion | 0.5 ms / 120.3 ms / 115.3-120.3 ms | 5.8 ms / 515.9 ms / 492.3-515.9 ms | 14.5 ms / 1133.2 ms / 1036.5-1133.2 ms | 4.2 ms / 223.2 ms / 210.1-223.2 ms | 3.2 ms / 266.5 ms / 253.2-266.5 ms | 3.3 ms / 246.7 ms / 227.3-246.7 ms |
| First output | 80.4 ms | 405.3 ms | 914.4 ms | 112.5 ms | 154.7 ms | 130.3 ms |
| First output dispersion | 0.6 ms / 81.9 ms / 79.2-81.9 ms | 2.3 ms / 413.8 ms / 401.3-413.8 ms | 11.3 ms / 984.4 ms / 902.8-984.4 ms | 1.3 ms / 117.4 ms / 111.1-117.4 ms | 1.3 ms / 157.9 ms / 152.8-157.9 ms | 1.3 ms / 133.6 ms / 128.7-133.6 ms |
| User CPU | 91.1 ms | 561.8 ms | 1612.8 ms | 158.7 ms | 195.6 ms | 175.1 ms |
| System CPU | 25.5 ms | 194.0 ms | 471.7 ms | 58.6 ms | 62.7 ms | 61.2 ms |
| Peak RSS | 60.5 MiB | 463.3 MiB | 1309.7 MiB | 69.5 MiB | 89.4 MiB | 78.0 MiB |
| Logical throughput | 95465.5 records/s | 22263.1 records/s | 10707.2 records/s | 52444.2 records/s | 43660.2 records/s | 47669.8 records/s |
| Physical throughput | 64.7 MiB/s | 15.1 MiB/s | 7.2 MiB/s | 35.5 MiB/s | 29.6 MiB/s | 38.3 MiB/s |
| Output bytes | 553684.0 B | 553684.0 B | 553684.0 B | 553684.0 B | 553684.0 B | 553684.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 22.9 ms | 105.5 ms | 211.6 ms | 42.2 ms | 49.1 ms | 45.2 ms |
| Wall time dispersion | 0.4 ms / 26.8 ms / 22.3-26.8 ms | 2.2 ms / 108.2 ms / 101.0-108.2 ms | 3.9 ms / 220.1 ms / 198.8-220.1 ms | 0.5 ms / 43.1 ms / 40.9-43.1 ms | 0.4 ms / 51.1 ms / 47.3-51.1 ms | 0.6 ms / 47.3 ms / 44.1-47.3 ms |
| First output | 16.9 ms | 83.1 ms | 185.0 ms | 32.4 ms | 39.4 ms | 35.3 ms |
| First output dispersion | 0.3 ms / 20.6 ms / 16.5-20.6 ms | 0.6 ms / 87.6 ms / 80.2-87.6 ms | 4.4 ms / 192.0 ms / 170.7-192.0 ms | 0.2 ms / 32.8 ms / 31.2-32.8 ms | 0.5 ms / 41.0 ms / 37.7-41.0 ms | 0.6 ms / 37.4 ms / 34.5-37.4 ms |
| User CPU | 17.8 ms | 103.8 ms | 278.2 ms | 29.6 ms | 36.5 ms | 33.9 ms |
| System CPU | 5.4 ms | 43.0 ms | 93.8 ms | 12.1 ms | 13.1 ms | 11.8 ms |
| Peak RSS | 14.7 MiB | 109.4 MiB | 238.1 MiB | 20.2 MiB | 24.2 MiB | 21.8 MiB |
| Logical throughput | 97140.4 records/s | 21094.0 records/s | 10513.6 records/s | 52732.0 records/s | 45345.2 records/s | 49262.7 records/s |
| Physical throughput | 65.8 MiB/s | 14.3 MiB/s | 7.1 MiB/s | 35.7 MiB/s | 30.7 MiB/s | 39.5 MiB/s |
| Output bytes | 109544.0 B | 109544.0 B | 109544.0 B | 109544.0 B | 109544.0 B | 109544.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
