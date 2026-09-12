---
type: Report
title: "Early break with a label"
description: "Measures stopping a feature traversal after the first result."
workload: benchmark.label-early-break
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Early break with a label

## What this measures

The labeled query emits a feature ID and then breaks to the outer label. It
tests non-local control flow and whether later features are left unread.

## Why it matters

First-match queries should stop work when the answer is known. Labels provide a
way to express that exit when a simple predicate is not enough.

## Input and output

The input is a natural feature snapshot. The output is the early result
sequence, normally the first feature ID, rather than every feature ID in the
document.

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

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq label/break expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 2.8 ms | - | - | 3.2 ms | 4.2 ms | 3.6 ms |
| Wall time dispersion | 0.1 ms / 3.1 ms / 2.6-3.1 ms | - | - | 0.1 ms / 3.6 ms / 3.1-3.6 ms | 0.1 ms / 4.8 ms / 3.9-4.8 ms | 0.2 ms / 3.9 ms / 3.4-3.9 ms |
| First output | 2.6 ms | - | - | 3.1 ms | 3.9 ms | 3.3 ms |
| First output dispersion | 0.1 ms / 2.9 ms / 2.4-2.9 ms | - | - | 0.2 ms / 3.2 ms / 2.9-3.2 ms | 0.1 ms / 4.5 ms / 3.7-4.5 ms | 0.1 ms / 3.7 ms / 3.2-3.7 ms |
| User CPU | 1.8 ms | - | - | 2.1 ms | 2.5 ms | 2.2 ms |
| System CPU | 0.9 ms | - | - | 1.1 ms | 1.6 ms | 1.1 ms |
| Peak RSS | 4.3 MiB | - | - | 9.1 MiB | 9.8 MiB | 9.2 MiB |
| Logical throughput | 70010.8 records/s | - | - | 60043.3 records/s | 46251.0 records/s | 54456.1 records/s |
| Physical throughput | 47.9 MiB/s | - | - | 41.1 MiB/s | 31.6 MiB/s | 44.1 MiB/s |
| Output bytes | 13.0 B | - | - | 11.0 B | 11.0 B | 11.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq label/break expression. | yq rejects the catalog jq label/break expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq label/break expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | - | - | 1.2 ms | 1.3 ms | 1.2 ms |
| Wall time dispersion | 0.1 ms / 1.8 ms / 1.3-2.1 ms | - | - | 0.0 ms / 1.5 ms / 1.1-1.5 ms | 0.1 ms / 1.5 ms / 1.1-1.7 ms | 0.0 ms / 1.5 ms / 1.1-1.6 ms |
| First output | 1.3 ms | - | - | 1.0 ms | 1.0 ms | 1.0 ms |
| First output dispersion | 0.1 ms / 1.6 ms / 1.2-1.8 ms | - | - | 0.0 ms / 1.4 ms / 1.0-1.4 ms | 0.0 ms / 1.4 ms / 1.0-1.4 ms | 0.0 ms / 1.4 ms / 0.9-1.4 ms |
| User CPU | 1.3 ms | - | - | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.0 ms | - | - | 1.1 ms | 1.1 ms | 1.1 ms |
| Peak RSS | 4.0 MiB | - | - | 8.1 MiB | 8.4 MiB | 8.0 MiB |
| Logical throughput | 1343.2 records/s | - | - | 1697.1 records/s | 1556.4 records/s | 1702.1 records/s |
| Physical throughput | 1.1 MiB/s | - | - | 1.4 MiB/s | 1.3 MiB/s | 1.7 MiB/s |
| Output bytes | 13.0 B | - | - | 11.0 B | 11.0 B | 11.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq label/break expression. | yq rejects the catalog jq label/break expression. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq label/break expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 95.2 ms | - | - | 122.6 ms | 164.7 ms | 141.1 ms |
| Wall time dispersion | 0.7 ms / 98.2 ms / 94.4-98.2 ms | - | - | 0.9 ms / 123.8 ms / 121.0-123.8 ms | 1.0 ms / 169.1 ms / 162.8-169.1 ms | 1.3 ms / 145.6 ms / 137.6-145.6 ms |
| First output | 91.5 ms | - | - | 118.6 ms | 160.0 ms | 137.2 ms |
| First output dispersion | 0.6 ms / 94.7 ms / 90.8-94.7 ms | - | - | 0.4 ms / 120.0 ms / 117.7-120.0 ms | 0.6 ms / 162.6 ms / 157.5-162.6 ms | 1.2 ms / 140.6 ms / 134.2-140.6 ms |
| User CPU | 71.1 ms | - | - | 95.2 ms | 132.7 ms | 110.1 ms |
| System CPU | 23.9 ms | - | - | 26.6 ms | 32.0 ms | 32.4 ms |
| Peak RSS | 60.3 MiB | - | - | 69.4 MiB | 89.5 MiB | 78.0 MiB |
| Logical throughput | 118477.9 records/s | - | - | 91964.7 records/s | 68468.2 records/s | 79893.1 records/s |
| Physical throughput | 80.3 MiB/s | - | - | 62.3 MiB/s | 46.3 MiB/s | 64.1 MiB/s |
| Output bytes | 13.0 B | - | - | 11.0 B | 11.0 B | 11.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq label/break expression. | yq rejects the catalog jq label/break expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq label/break expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 19.5 ms | - | - | 24.8 ms | 32.4 ms | 28.8 ms |
| Wall time dispersion | 0.6 ms / 23.0 ms / 18.7-23.0 ms | - | - | 0.7 ms / 25.7 ms / 22.7-25.7 ms | 0.6 ms / 34.0 ms / 30.6-34.0 ms | 0.4 ms / 29.7 ms / 26.6-29.7 ms |
| First output | 18.8 ms | - | - | 23.9 ms | 31.1 ms | 27.8 ms |
| First output dispersion | 0.6 ms / 22.2 ms / 18.0-22.2 ms | - | - | 0.6 ms / 24.7 ms / 22.1-24.7 ms | 0.4 ms / 32.7 ms / 29.8-32.7 ms | 0.5 ms / 28.6 ms / 26.0-28.6 ms |
| User CPU | 14.5 ms | - | - | 18.5 ms | 24.4 ms | 20.7 ms |
| System CPU | 4.1 ms | - | - | 5.5 ms | 7.2 ms | 7.0 ms |
| Peak RSS | 14.4 MiB | - | - | 20.2 MiB | 24.1 MiB | 21.6 MiB |
| Logical throughput | 114381.2 records/s | - | - | 89685.2 records/s | 68716.3 records/s | 77349.6 records/s |
| Physical throughput | 77.5 MiB/s | - | - | 60.8 MiB/s | 46.5 MiB/s | 62.1 MiB/s |
| Output bytes | 13.0 B | - | - | 11.0 B | 11.0 B | 11.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq label/break expression. | yq rejects the catalog jq label/break expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
