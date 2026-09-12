---
type: Report
title: "Templated URI output"
description: "Measures building and URI-escaping a query-string template per feature."
workload: benchmark.format-template
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Templated URI output

## What this measures

The query builds `id=...&place=...` from each feature and applies URI escaping
to the template. It combines interpolation with text formatting.

## Why it matters

Query strings and small links are often assembled at the edge of a data
pipeline. Escaping each interpolated value is part of producing safe output.

## Input and output

The input is a natural feature snapshot. The output is one URI-escaped query
string per feature with `id` and `place` fields, not the source feature.

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

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq format-string expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 3.2 ms | - | - | 7.9 ms | 5.8 ms | 5.4 ms |
| Wall time dispersion | 0.2 ms / 3.7 ms / 2.9-3.7 ms | - | - | 0.1 ms / 8.2 ms / 7.5-8.2 ms | 0.1 ms / 6.2 ms / 5.6-6.2 ms | 0.2 ms / 5.9 ms / 5.1-5.9 ms |
| First output | 2.7 ms | - | - | 7.7 ms | 5.5 ms | 5.3 ms |
| First output dispersion | 0.2 ms / 3.0 ms / 2.4-3.0 ms | - | - | 0.1 ms / 7.9 ms / 7.3-7.9 ms | 0.1 ms / 5.9 ms / 5.4-5.9 ms | 0.1 ms / 5.8 ms / 4.9-5.8 ms |
| User CPU | 2.6 ms | - | - | 5.9 ms | 3.5 ms | 4.4 ms |
| System CPU | 0.5 ms | - | - | 1.9 ms | 2.3 ms | 1.0 ms |
| Peak RSS | 4.4 MiB | - | - | 8.8 MiB | 9.8 MiB | 8.0 MiB |
| Logical throughput | 60370.3 records/s | - | - | 24516.6 records/s | 33514.7 records/s | 35632.3 records/s |
| Physical throughput | 41.3 MiB/s | - | - | 16.8 MiB/s | 22.9 MiB/s | 28.8 MiB/s |
| Output bytes | 12699.0 B | - | - | 12311.0 B | 12311.0 B | 12311.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq format-string expression. | yq rejects the catalog jq format-string expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq format-string expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | - | - | 1.3 ms | 1.3 ms | 1.3 ms |
| Wall time dispersion | 0.1 ms / 1.7 ms / 1.3-1.8 ms | - | - | 0.0 ms / 1.6 ms / 1.3-1.6 ms | 0.0 ms / 1.5 ms / 1.2-1.6 ms | 0.0 ms / 1.5 ms / 1.2-1.5 ms |
| First output | 1.3 ms | - | - | 1.2 ms | 1.1 ms | 1.0 ms |
| First output dispersion | 0.0 ms / 1.6 ms / 1.2-1.7 ms | - | - | 0.0 ms / 1.5 ms / 1.0-1.5 ms | 0.1 ms / 1.4 ms / 1.0-1.4 ms | 0.1 ms / 1.3 ms / 1.0-1.3 ms |
| User CPU | 0.8 ms | - | - | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.7 ms | - | - | 1.2 ms | 1.1 ms | 1.2 ms |
| Peak RSS | 4.0 MiB | - | - | 8.7 MiB | 8.4 MiB | 8.1 MiB |
| Logical throughput | 1339.6 records/s | - | - | 1484.8 records/s | 1495.3 records/s | 1501.5 records/s |
| Physical throughput | 1.1 MiB/s | - | - | 1.3 MiB/s | 1.3 MiB/s | 1.5 MiB/s |
| Output bytes | 128.0 B | - | - | 124.0 B | 124.0 B | 124.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq format-string expression. | yq rejects the catalog jq format-string expression. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq format-string expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 120.5 ms | - | - | 374.0 ms | 258.9 ms | 212.9 ms |
| Wall time dispersion | 1.0 ms / 124.7 ms / 117.9-124.7 ms | - | - | 1.0 ms / 377.0 ms / 371.9-377.0 ms | 3.7 ms / 264.9 ms / 254.1-264.9 ms | 0.7 ms / 218.3 ms / 210.3-218.3 ms |
| First output | 80.5 ms | - | - | 301.7 ms | 153.8 ms | 21.8 ms |
| First output dispersion | 0.6 ms / 82.7 ms / 79.3-82.7 ms | - | - | 1.1 ms / 304.2 ms / 299.5-304.2 ms | 0.7 ms / 155.1 ms / 152.3-155.1 ms | 0.3 ms / 22.9 ms / 21.3-22.9 ms |
| User CPU | 93.9 ms | - | - | 349.0 ms | 195.5 ms | 209.8 ms |
| System CPU | 26.0 ms | - | - | 25.5 ms | 66.4 ms | 2.0 ms |
| Peak RSS | 60.2 MiB | - | - | 11.6 MiB | 89.4 MiB | 8.0 MiB |
| Logical throughput | 93535.7 records/s | - | - | 30141.3 records/s | 43541.9 records/s | 52944.0 records/s |
| Physical throughput | 63.4 MiB/s | - | - | 20.4 MiB/s | 29.5 MiB/s | 42.5 MiB/s |
| Output bytes | 727347.0 B | - | - | 704799.0 B | 704799.0 B | 704799.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq format-string expression. | yq rejects the catalog jq format-string expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq format-string expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 24.1 ms | - | - | 70.8 ms | 49.4 ms | 44.2 ms |
| Wall time dispersion | 0.3 ms / 24.4 ms / 23.0-24.4 ms | - | - | 0.6 ms / 74.3 ms / 70.1-74.3 ms | 0.7 ms / 50.9 ms / 48.1-50.9 ms | 0.4 ms / 45.6 ms / 43.7-45.6 ms |
| First output | 17.3 ms | - | - | 62.0 ms | 37.4 ms | 21.6 ms |
| First output dispersion | 0.2 ms / 17.8 ms / 16.5-17.8 ms | - | - | 0.4 ms / 65.7 ms / 61.5-65.7 ms | 0.5 ms / 38.1 ms / 36.6-38.1 ms | 0.3 ms / 22.9 ms / 21.2-22.9 ms |
| User CPU | 18.6 ms | - | - | 69.3 ms | 35.2 ms | 42.8 ms |
| System CPU | 5.0 ms | - | - | 1.0 ms | 14.7 ms | 1.0 ms |
| Peak RSS | 14.6 MiB | - | - | 9.7 MiB | 24.4 MiB | 8.1 MiB |
| Logical throughput | 92465.6 records/s | - | - | 31424.8 records/s | 45012.2 records/s | 50309.2 records/s |
| Physical throughput | 62.7 MiB/s | - | - | 21.3 MiB/s | 30.5 MiB/s | 40.4 MiB/s |
| Output bytes | 143982.0 B | - | - | 139532.0 B | 139532.0 B | 139532.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq format-string expression. | yq rejects the catalog jq format-string expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
