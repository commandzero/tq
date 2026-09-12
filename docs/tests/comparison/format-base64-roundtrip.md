---
type: Report
title: "Base64 round trip"
description: "Measures Base64 encoding followed by decoding for each place string."
workload: benchmark.format-base64-roundtrip
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Base64 round trip

## What this measures

The query extracts each place string, applies `@base64`, and decodes it with
`@base64d`. It includes both conversions for every feature.

## Why it matters

Encoding is common when text crosses a binary-safe transport or storage field.
A round trip shows the cost of conversion even when the final value is
unchanged.

## Input and output

The input is a natural feature snapshot. The output is one decoded place string
per feature, so it should have the same semantic text as the selected input.

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
| Wall time | 3.2 ms | 12.9 ms | 20.9 ms | 7.8 ms | 5.7 ms | 5.4 ms |
| Wall time dispersion | 0.2 ms / 3.6 ms / 2.7-3.6 ms | 0.3 ms / 13.5 ms / 12.2-13.5 ms | 0.4 ms / 21.7 ms / 20.1-21.7 ms | 0.2 ms / 8.7 ms / 7.4-8.7 ms | 0.1 ms / 6.1 ms / 5.4-6.1 ms | 0.3 ms / 6.1 ms / 5.1-6.1 ms |
| First output | 2.9 ms | 11.0 ms | 19.5 ms | 7.7 ms | 5.4 ms | 5.2 ms |
| First output dispersion | 0.2 ms / 3.3 ms / 2.4-3.3 ms | 0.3 ms / 11.4 ms / 10.3-11.4 ms | 0.4 ms / 20.6 ms / 18.8-20.6 ms | 0.1 ms / 8.4 ms / 7.2-8.4 ms | 0.1 ms / 5.8 ms / 5.1-5.8 ms | 0.2 ms / 5.8 ms / 5.0-5.8 ms |
| User CPU | 2.1 ms | 11.1 ms | 21.4 ms | 6.3 ms | 3.0 ms | 4.1 ms |
| System CPU | 1.1 ms | 7.2 ms | 10.3 ms | 1.4 ms | 2.6 ms | 1.0 ms |
| Peak RSS | 4.2 MiB | 22.0 MiB | 33.5 MiB | 8.6 MiB | 9.8 MiB | 8.0 MiB |
| Logical throughput | 61314.8 records/s | 15018.4 records/s | 9284.5 records/s | 24771.8 records/s | 34008.2 records/s | 35632.3 records/s |
| Physical throughput | 42.0 MiB/s | 10.3 MiB/s | 6.3 MiB/s | 17.0 MiB/s | 23.3 MiB/s | 28.8 MiB/s |
| Output bytes | 6025.0 B | 6025.0 B | 6025.0 B | 6019.0 B | 6019.0 B | 6019.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | 3.1 ms | 3.2 ms | 1.3 ms | 1.3 ms | 1.3 ms |
| Wall time dispersion | 0.2 ms / 1.8 ms / 1.3-1.8 ms | 0.1 ms / 3.7 ms / 2.9-3.7 ms | 0.1 ms / 3.5 ms / 2.9-3.5 ms | 0.0 ms / 1.7 ms / 1.2-1.8 ms | 0.1 ms / 1.6 ms / 1.2-1.7 ms | 0.0 ms / 1.5 ms / 1.2-1.6 ms |
| First output | 1.4 ms | 2.7 ms | 2.8 ms | 1.2 ms | 1.0 ms | 1.0 ms |
| First output dispersion | 0.0 ms / 1.7 ms / 1.2-1.7 ms | 0.1 ms / 3.2 ms / 2.6-3.2 ms | 0.1 ms / 3.1 ms / 2.6-3.1 ms | 0.0 ms / 1.4 ms / 1.0-1.5 ms | 0.0 ms / 1.4 ms / 1.0-1.5 ms | 0.0 ms / 1.3 ms / 1.0-1.3 ms |
| User CPU | 1.3 ms | 1.0 ms | 1.1 ms | 0.3 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.0 ms | 2.1 ms | 2.2 ms | 0.9 ms | 1.2 ms | 1.2 ms |
| Peak RSS | 4.1 MiB | 11.9 MiB | 12.0 MiB | 8.7 MiB | 8.4 MiB | 8.1 MiB |
| Logical throughput | 1335.6 records/s | 637.7 records/s | 620.2 records/s | 1490.9 records/s | 1525.6 records/s | 1502.1 records/s |
| Physical throughput | 1.1 MiB/s | 0.5 MiB/s | 0.5 MiB/s | 1.3 MiB/s | 1.3 MiB/s | 1.5 MiB/s |
| Output bytes | 58.0 B | 58.0 B | 58.0 B | 58.0 B | 58.0 B | 58.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 106.4 ms | 484.1 ms | 1082.3 ms | 369.7 ms | 245.1 ms | 202.9 ms |
| Wall time dispersion | 0.4 ms / 108.5 ms / 105.6-108.5 ms | 4.8 ms / 496.8 ms / 477.1-496.8 ms | 24.7 ms / 1123.5 ms / 1020.1-1123.5 ms | 1.8 ms / 372.4 ms / 365.0-372.4 ms | 3.8 ms / 251.2 ms / 235.3-251.2 ms | 1.4 ms / 223.4 ms / 201.3-223.4 ms |
| First output | 79.8 ms | 378.7 ms | 934.8 ms | 308.5 ms | 158.4 ms | 40.7 ms |
| First output dispersion | 0.1 ms / 80.0 ms / 79.3-80.0 ms | 2.5 ms / 386.9 ms / 373.2-386.9 ms | 19.9 ms / 966.4 ms / 878.5-966.4 ms | 1.9 ms / 310.8 ms / 305.1-310.8 ms | 2.1 ms / 163.2 ms / 153.9-163.2 ms | 0.5 ms / 43.7 ms / 39.6-43.7 ms |
| User CPU | 82.6 ms | 543.8 ms | 1585.9 ms | 343.8 ms | 184.6 ms | 199.7 ms |
| System CPU | 23.9 ms | 187.3 ms | 483.7 ms | 24.0 ms | 59.4 ms | 2.5 ms |
| Peak RSS | 60.2 MiB | 454.4 MiB | 1305.3 MiB | 11.6 MiB | 89.6 MiB | 8.1 MiB |
| Logical throughput | 105935.7 records/s | 23289.0 records/s | 10416.7 records/s | 30495.4 records/s | 45994.5 records/s | 55556.9 records/s |
| Physical throughput | 71.8 MiB/s | 15.8 MiB/s | 7.1 MiB/s | 20.7 MiB/s | 31.1 MiB/s | 44.6 MiB/s |
| Output bytes | 347435.0 B | 347435.0 B | 347435.0 B | 347161.0 B | 347161.0 B | 347161.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 21.5 ms | 98.4 ms | 203.4 ms | 69.8 ms | 47.8 ms | 42.5 ms |
| Wall time dispersion | 0.3 ms / 22.0 ms / 20.4-22.0 ms | 1.3 ms / 101.1 ms / 95.0-101.1 ms | 3.1 ms / 215.1 ms / 188.2-215.1 ms | 0.3 ms / 72.7 ms / 69.2-72.7 ms | 1.2 ms / 49.5 ms / 46.0-49.5 ms | 0.4 ms / 44.2 ms / 41.9-44.2 ms |
| First output | 17.2 ms | 76.0 ms | 176.6 ms | 69.0 ms | 43.9 ms | 40.5 ms |
| First output dispersion | 0.3 ms / 17.6 ms / 16.4-17.6 ms | 1.3 ms / 78.1 ms / 73.2-78.1 ms | 3.2 ms / 186.2 ms / 161.5-186.2 ms | 0.3 ms / 71.6 ms / 68.3-71.6 ms | 1.0 ms / 45.2 ms / 42.4-45.2 ms | 0.3 ms / 42.2 ms / 40.0-42.2 ms |
| User CPU | 16.6 ms | 102.3 ms | 277.3 ms | 68.4 ms | 33.4 ms | 40.8 ms |
| System CPU | 4.6 ms | 39.1 ms | 93.0 ms | 1.5 ms | 14.0 ms | 1.4 ms |
| Peak RSS | 14.5 MiB | 101.9 MiB | 237.3 MiB | 9.6 MiB | 24.2 MiB | 8.2 MiB |
| Logical throughput | 103332.2 records/s | 22606.8 records/s | 10940.0 records/s | 31857.4 records/s | 46513.6 records/s | 52396.7 records/s |
| Physical throughput | 70.0 MiB/s | 15.3 MiB/s | 7.4 MiB/s | 21.6 MiB/s | 31.5 MiB/s | 42.1 MiB/s |
| Output bytes | 68494.0 B | 68494.0 B | 68494.0 B | 68450.0 B | 68450.0 B | 68450.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
