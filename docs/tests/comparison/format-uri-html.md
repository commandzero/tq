---
type: Report
title: "HTML and URI escaping"
description: "Measures two text-escaping passes over feature place names."
workload: benchmark.format-uri-html
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# HTML and URI escaping

## What this measures

The query takes each place string through `@html` and then `@uri`. It exercises
escaping rules and repeated text allocation for every feature.

## Why it matters

Data often crosses an HTML or URL boundary after it leaves a JSON document.
Applying the transformations in sequence is common in report and request
generation.

## Input and output

The input is a natural feature snapshot. The output is one URI-encoded string
per place after HTML escaping, not the original place text or full feature.

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

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq @html/@uri expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 3.1 ms | - | - | 8.0 ms | 5.7 ms | 5.4 ms |
| Wall time dispersion | 0.1 ms / 3.4 ms / 2.9-3.4 ms | - | - | 0.1 ms / 8.5 ms / 7.5-8.5 ms | 0.2 ms / 6.1 ms / 5.3-6.1 ms | 0.2 ms / 5.9 ms / 5.1-5.9 ms |
| First output | 2.7 ms | - | - | 7.8 ms | 5.5 ms | 5.2 ms |
| First output dispersion | 0.1 ms / 3.0 ms / 2.5-3.0 ms | - | - | 0.1 ms / 8.3 ms / 7.3-8.3 ms | 0.2 ms / 5.8 ms / 5.1-5.8 ms | 0.1 ms / 5.8 ms / 5.0-5.8 ms |
| User CPU | 2.0 ms | - | - | 6.8 ms | 4.0 ms | 4.2 ms |
| System CPU | 0.9 ms | - | - | 1.0 ms | 1.7 ms | 1.1 ms |
| Peak RSS | 4.3 MiB | - | - | 8.9 MiB | 9.6 MiB | 7.9 MiB |
| Logical throughput | 62189.5 records/s | - | - | 24278.8 records/s | 34103.9 records/s | 36200.8 records/s |
| Physical throughput | 42.6 MiB/s | - | - | 16.6 MiB/s | 23.3 MiB/s | 29.3 MiB/s |
| Output bytes | 8635.0 B | - | - | 8247.0 B | 8247.0 B | 8247.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq @html/@uri expression. | yq rejects the catalog jq @html/@uri expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq @html/@uri expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | - | - | 1.3 ms | 1.3 ms | 1.3 ms |
| Wall time dispersion | 0.2 ms / 1.8 ms / 1.3-1.8 ms | - | - | 0.0 ms / 1.7 ms / 1.3-1.7 ms | 0.1 ms / 1.5 ms / 1.2-1.7 ms | 0.0 ms / 1.5 ms / 1.1-1.5 ms |
| First output | 1.4 ms | - | - | 1.2 ms | 1.0 ms | 1.0 ms |
| First output dispersion | 0.1 ms / 1.7 ms / 1.2-1.7 ms | - | - | 0.0 ms / 1.4 ms / 1.0-1.5 ms | 0.0 ms / 1.3 ms / 1.0-1.3 ms | 0.0 ms / 1.3 ms / 1.0-1.3 ms |
| User CPU | 0.7 ms | - | - | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.7 ms | - | - | 1.2 ms | 1.1 ms | 1.1 ms |
| Peak RSS | 4.1 MiB | - | - | 8.7 MiB | 8.5 MiB | 8.1 MiB |
| Logical throughput | 1323.6 records/s | - | - | 1485.3 records/s | 1502.6 records/s | 1525.0 records/s |
| Physical throughput | 1.1 MiB/s | - | - | 1.3 MiB/s | 1.3 MiB/s | 1.5 MiB/s |
| Output bytes | 88.0 B | - | - | 84.0 B | 84.0 B | 84.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq @html/@uri expression. | yq rejects the catalog jq @html/@uri expression. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq @html/@uri expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 114.6 ms | - | - | 371.0 ms | 250.0 ms | 203.7 ms |
| Wall time dispersion | 0.9 ms / 122.3 ms / 113.2-122.3 ms | - | - | 0.7 ms / 389.9 ms / 370.1-389.9 ms | 3.6 ms / 257.2 ms / 243.8-257.2 ms | 1.0 ms / 208.9 ms / 201.2-208.9 ms |
| First output | 79.1 ms | - | - | 303.6 ms | 155.6 ms | 30.6 ms |
| First output dispersion | 0.3 ms / 87.0 ms / 78.4-87.0 ms | - | - | 1.1 ms / 322.0 ms / 301.9-322.0 ms | 1.1 ms / 162.6 ms / 153.5-162.6 ms | 0.5 ms / 31.7 ms / 29.8-31.7 ms |
| User CPU | 91.3 ms | - | - | 349.4 ms | 186.1 ms | 200.4 ms |
| System CPU | 23.8 ms | - | - | 22.9 ms | 65.0 ms | 3.0 ms |
| Peak RSS | 60.2 MiB | - | - | 11.6 MiB | 89.5 MiB | 8.1 MiB |
| Logical throughput | 98414.3 records/s | - | - | 30389.6 records/s | 45091.8 records/s | 55352.2 records/s |
| Physical throughput | 66.7 MiB/s | - | - | 20.6 MiB/s | 30.5 MiB/s | 44.4 MiB/s |
| Output bytes | 492984.0 B | - | - | 470436.0 B | 470436.0 B | 470436.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq @html/@uri expression. | yq rejects the catalog jq @html/@uri expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq @html/@uri expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 23.5 ms | - | - | 71.0 ms | 48.3 ms | 43.4 ms |
| Wall time dispersion | 0.3 ms / 24.5 ms / 22.7-24.5 ms | - | - | 0.4 ms / 73.1 ms / 70.2-73.1 ms | 0.3 ms / 49.7 ms / 47.0-49.7 ms | 0.3 ms / 47.9 ms / 42.4-47.9 ms |
| First output | 17.3 ms | - | - | 66.2 ms | 40.6 ms | 31.0 ms |
| First output dispersion | 0.3 ms / 18.0 ms / 16.6-18.0 ms | - | - | 0.5 ms / 68.3 ms / 65.4-68.3 ms | 0.4 ms / 41.5 ms / 39.4-41.5 ms | 0.1 ms / 34.5 ms / 30.2-34.5 ms |
| User CPU | 17.2 ms | - | - | 69.0 ms | 32.2 ms | 42.1 ms |
| System CPU | 6.4 ms | - | - | 1.0 ms | 15.6 ms | 1.5 ms |
| Peak RSS | 14.7 MiB | - | - | 9.8 MiB | 24.3 MiB | 8.0 MiB |
| Logical throughput | 94793.8 records/s | - | - | 31335.4 records/s | 46108.7 records/s | 51257.8 records/s |
| Physical throughput | 64.2 MiB/s | - | - | 21.2 MiB/s | 31.2 MiB/s | 41.1 MiB/s |
| Output bytes | 97260.0 B | - | - | 92810.0 B | 92810.0 B | 92810.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq @html/@uri expression. | yq rejects the catalog jq @html/@uri expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
