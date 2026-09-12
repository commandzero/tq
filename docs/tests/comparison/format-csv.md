---
type: Report
title: "CSV formatting"
description: "Measures formatting selected feature fields as CSV rows."
workload: benchmark.format-csv
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# CSV formatting

## What this measures

The query selects `id`, `place`, and `mag` for each feature and applies `@csv`.
It measures quoting, delimiter handling, and text output for repeated rows.

## Why it matters

CSV remains a practical interchange format for spreadsheets and simple data
loads. Correct quoting matters when fields contain commas or special text.

## Input and output

The input is a natural feature snapshot. The output is one CSV string per
feature containing the three selected fields, not JSON arrays or feature
objects.

The yq adapter forces quoted string fields to match jq CSV text. It emits an empty magnitude field for null values.

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
| Wall time | 3.2 ms | 24.5 ms | 33.2 ms | 5.1 ms | 5.9 ms | 5.5 ms |
| Wall time dispersion | 0.1 ms / 3.6 ms / 2.9-3.6 ms | 0.3 ms / 24.9 ms / 23.5-24.9 ms | 0.9 ms / 35.6 ms / 31.6-35.6 ms | 0.2 ms / 5.4 ms / 4.8-5.4 ms | 0.1 ms / 6.2 ms / 5.6-6.2 ms | 0.1 ms / 5.6 ms / 5.3-5.6 ms |
| First output | 2.8 ms | 23.2 ms | 31.8 ms | 4.9 ms | 5.6 ms | 5.2 ms |
| First output dispersion | 0.1 ms / 2.9 ms / 2.5-2.9 ms | 0.4 ms / 23.7 ms / 22.0-23.7 ms | 0.3 ms / 32.5 ms / 30.3-32.5 ms | 0.2 ms / 5.1 ms / 4.7-5.1 ms | 0.1 ms / 5.9 ms / 5.4-5.9 ms | 0.1 ms / 5.4 ms / 4.9-5.4 ms |
| User CPU | 2.6 ms | 29.4 ms | 33.1 ms | 2.7 ms | 4.0 ms | 3.3 ms |
| System CPU | 0.5 ms | 8.2 ms | 14.3 ms | 2.3 ms | 1.7 ms | 2.2 ms |
| Peak RSS | 4.3 MiB | 23.9 MiB | 37.6 MiB | 9.2 MiB | 9.7 MiB | 9.0 MiB |
| Logical throughput | 59876.5 records/s | 7904.2 records/s | 5848.8 records/s | 37717.5 records/s | 33091.7 records/s | 35567.0 records/s |
| Physical throughput | 41.0 MiB/s | 5.4 MiB/s | 4.0 MiB/s | 25.8 MiB/s | 22.6 MiB/s | 28.8 MiB/s |
| Output bytes | 10734.0 B | 10734.0 B | 10734.0 B | 10734.0 B | 10734.0 B | 10734.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | 3.9 ms | 4.0 ms | 1.3 ms | 1.3 ms | 1.3 ms |
| Wall time dispersion | 0.0 ms / 1.5 ms / 1.3-1.6 ms | 0.1 ms / 4.2 ms / 3.7-4.3 ms | 0.1 ms / 4.3 ms / 3.8-4.4 ms | 0.1 ms / 1.6 ms / 1.2-1.7 ms | 0.1 ms / 1.5 ms / 1.2-1.7 ms | 0.1 ms / 1.5 ms / 1.1-1.5 ms |
| First output | 1.3 ms | 3.5 ms | 3.5 ms | 1.0 ms | 1.0 ms | 1.1 ms |
| First output dispersion | 0.0 ms / 1.4 ms / 1.2-1.5 ms | 0.1 ms / 3.7 ms / 3.2-3.7 ms | 0.1 ms / 3.8 ms / 3.4-3.8 ms | 0.0 ms / 1.5 ms / 1.0-1.5 ms | 0.0 ms / 1.4 ms / 1.0-1.4 ms | 0.1 ms / 1.4 ms / 1.0-1.4 ms |
| User CPU | 0.7 ms | 1.9 ms | 2.0 ms | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.7 ms | 2.0 ms | 2.1 ms | 1.1 ms | 1.2 ms | 1.1 ms |
| Peak RSS | 4.0 MiB | 12.0 MiB | 12.3 MiB | 8.1 MiB | 8.4 MiB | 8.0 MiB |
| Logical throughput | 1347.3 records/s | 517.9 records/s | 501.7 records/s | 1542.6 records/s | 1504.9 records/s | 1528.5 records/s |
| Physical throughput | 1.1 MiB/s | 0.4 MiB/s | 0.4 MiB/s | 1.3 MiB/s | 1.3 MiB/s | 1.5 MiB/s |
| Output bytes | 106.0 B | 106.0 B | 106.0 B | 106.0 B | 106.0 B | 106.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 117.8 ms | 1093.8 ms | 1709.8 ms | 217.6 ms | 258.0 ms | 235.9 ms |
| Wall time dispersion | 1.1 ms / 120.7 ms / 115.1-120.7 ms | 5.1 ms / 1110.9 ms / 1083.9-1110.9 ms | 5.4 ms / 1737.6 ms / 1698.5-1737.6 ms | 2.7 ms / 221.8 ms / 210.8-221.8 ms | 2.1 ms / 262.8 ms / 252.6-262.8 ms | 2.0 ms / 244.2 ms / 231.3-244.2 ms |
| First output | 79.8 ms | 996.1 ms | 1562.1 ms | 111.9 ms | 152.8 ms | 127.9 ms |
| First output dispersion | 0.5 ms / 82.3 ms / 78.9-82.3 ms | 4.0 ms / 1006.4 ms / 984.7-1006.4 ms | 5.5 ms / 1585.2 ms / 1552.6-1585.2 ms | 1.4 ms / 115.4 ms / 110.0-115.4 ms | 1.2 ms / 155.6 ms / 151.5-155.6 ms | 0.4 ms / 135.0 ms / 127.4-135.0 ms |
| User CPU | 90.6 ms | 1548.2 ms | 2425.5 ms | 157.8 ms | 197.7 ms | 176.3 ms |
| System CPU | 26.9 ms | 214.3 ms | 547.2 ms | 56.4 ms | 61.2 ms | 60.8 ms |
| Peak RSS | 60.6 MiB | 476.7 MiB | 1439.7 MiB | 69.3 MiB | 89.4 MiB | 78.1 MiB |
| Logical throughput | 95724.1 records/s | 10307.3 records/s | 6593.8 records/s | 51814.1 records/s | 43694.2 records/s | 47794.3 records/s |
| Physical throughput | 64.9 MiB/s | 7.0 MiB/s | 4.5 MiB/s | 35.1 MiB/s | 29.6 MiB/s | 38.4 MiB/s |
| Output bytes | 621328.0 B | 621328.0 B | 621328.0 B | 621328.0 B | 621328.0 B | 621328.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 23.3 ms | 221.5 ms | 341.4 ms | 42.5 ms | 49.8 ms | 45.2 ms |
| Wall time dispersion | 0.6 ms / 24.6 ms / 21.4-24.6 ms | 0.8 ms / 222.5 ms / 219.4-222.5 ms | 3.9 ms / 354.1 ms / 333.8-354.1 ms | 0.6 ms / 45.8 ms / 41.4-45.8 ms | 0.4 ms / 52.2 ms / 46.7-52.2 ms | 1.1 ms / 48.2 ms / 43.3-48.2 ms |
| First output | 17.1 ms | 200.6 ms | 309.2 ms | 31.6 ms | 38.7 ms | 34.5 ms |
| First output dispersion | 0.3 ms / 18.1 ms / 15.9-18.1 ms | 0.4 ms / 202.9 ms / 198.4-202.9 ms | 3.5 ms / 316.1 ms / 303.5-316.1 ms | 0.5 ms / 33.4 ms / 30.9-33.4 ms | 0.3 ms / 40.1 ms / 36.2-40.1 ms | 0.7 ms / 35.7 ms / 32.8-35.7 ms |
| User CPU | 18.5 ms | 311.1 ms | 484.7 ms | 29.2 ms | 37.4 ms | 32.2 ms |
| System CPU | 5.1 ms | 45.3 ms | 125.7 ms | 13.6 ms | 12.1 ms | 14.2 ms |
| Peak RSS | 14.8 MiB | 106.2 MiB | 278.5 MiB | 20.2 MiB | 24.3 MiB | 21.8 MiB |
| Logical throughput | 95348.3 records/s | 10046.9 records/s | 6516.6 records/s | 52372.7 records/s | 44659.0 records/s | 49201.7 records/s |
| Physical throughput | 64.6 MiB/s | 6.8 MiB/s | 4.4 MiB/s | 35.5 MiB/s | 30.2 MiB/s | 39.5 MiB/s |
| Output bytes | 122894.0 B | 122894.0 B | 122894.0 B | 122894.0 B | 122894.0 B | 122894.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
