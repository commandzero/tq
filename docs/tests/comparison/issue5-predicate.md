---
type: Report
title: "Any-match predicate"
description: "Measures stopping a collection scan when one feature satisfies a predicate."
workload: benchmark.issue5-predicate
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Any-match predicate

## What this measures

The query asks whether any feature has a magnitude at least zero. It exercises
predicate evaluation with a possible early exit from the feature stream.

## Why it matters

Existence checks are common in alerts and guards. Their cost depends on where
the first match occurs, so they differ from reductions that always scan all
values.

## Input and output

The input is a natural feature snapshot. The output is one boolean, `true` when
at least one feature meets the predicate and `false` otherwise.

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

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq any predicate expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 2.9 ms | - | - | 3.4 ms | 4.1 ms | 3.8 ms |
| Wall time dispersion | 0.2 ms / 3.1 ms / 2.6-3.1 ms | - | - | 0.2 ms / 3.7 ms / 3.0-3.7 ms | 0.1 ms / 4.5 ms / 3.7-4.5 ms | 0.1 ms / 4.1 ms / 3.7-4.1 ms |
| First output | 2.7 ms | - | - | 3.1 ms | 3.9 ms | 3.6 ms |
| First output dispersion | 0.1 ms / 2.9 ms / 2.4-2.9 ms | - | - | 0.1 ms / 3.4 ms / 2.9-3.4 ms | 0.1 ms / 4.2 ms / 3.6-4.2 ms | 0.2 ms / 3.9 ms / 3.4-3.9 ms |
| User CPU | 1.8 ms | - | - | 2.1 ms | 2.8 ms | 2.8 ms |
| System CPU | 0.9 ms | - | - | 1.1 ms | 1.4 ms | 1.0 ms |
| Peak RSS | 4.3 MiB | - | - | 9.1 MiB | 9.6 MiB | 9.2 MiB |
| Logical throughput | 67407.9 records/s | - | - | 56675.4 records/s | 47224.9 records/s | 50396.2 records/s |
| Physical throughput | 46.1 MiB/s | - | - | 38.8 MiB/s | 32.3 MiB/s | 40.8 MiB/s |
| Output bytes | 5.0 B | - | - | 5.0 B | 5.0 B | 5.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq any predicate expression. | yq rejects the catalog jq any predicate expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq any predicate expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | - | - | 1.2 ms | 1.2 ms | 1.2 ms |
| Wall time dispersion | 0.0 ms / 1.7 ms / 1.3-1.8 ms | - | - | 0.0 ms / 1.5 ms / 1.1-1.6 ms | 0.1 ms / 1.5 ms / 1.1-1.5 ms | 0.0 ms / 1.5 ms / 1.1-1.5 ms |
| First output | 1.4 ms | - | - | 1.0 ms | 1.0 ms | 1.0 ms |
| First output dispersion | 0.0 ms / 1.5 ms / 1.2-1.7 ms | - | - | 0.0 ms / 1.4 ms / 1.0-1.5 ms | 0.0 ms / 1.4 ms / 1.0-1.4 ms | 0.0 ms / 1.3 ms / 1.0-1.4 ms |
| User CPU | 0.7 ms | - | - | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.7 ms | - | - | 1.1 ms | 1.1 ms | 1.1 ms |
| Peak RSS | 4.1 MiB | - | - | 8.0 MiB | 8.3 MiB | 7.9 MiB |
| Logical throughput | 1330.7 records/s | - | - | 1690.6 records/s | 1618.1 records/s | 1694.9 records/s |
| Physical throughput | 1.1 MiB/s | - | - | 1.4 MiB/s | 1.4 MiB/s | 1.6 MiB/s |
| Output bytes | 5.0 B | - | - | 5.0 B | 5.0 B | 5.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq any predicate expression. | yq rejects the catalog jq any predicate expression. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq any predicate expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 94.4 ms | - | - | 121.2 ms | 164.6 ms | 139.4 ms |
| Wall time dispersion | 0.5 ms / 95.4 ms / 92.0-95.4 ms | - | - | 0.9 ms / 124.5 ms / 120.0-124.5 ms | 0.6 ms / 181.5 ms / 163.1-181.5 ms | 0.5 ms / 142.3 ms / 138.5-142.3 ms |
| First output | 90.9 ms | - | - | 117.5 ms | 159.5 ms | 135.9 ms |
| First output dispersion | 0.2 ms / 92.4 ms / 89.2-92.4 ms | - | - | 0.9 ms / 120.6 ms / 116.5-120.6 ms | 0.3 ms / 175.4 ms / 158.4-175.4 ms | 0.5 ms / 138.2 ms / 134.6-138.2 ms |
| User CPU | 70.9 ms | - | - | 93.9 ms | 127.7 ms | 108.8 ms |
| System CPU | 22.9 ms | - | - | 28.0 ms | 36.5 ms | 30.4 ms |
| Peak RSS | 60.3 MiB | - | - | 69.3 MiB | 89.4 MiB | 78.1 MiB |
| Logical throughput | 119429.9 records/s | - | - | 93048.2 records/s | 68484.2 records/s | 80866.2 records/s |
| Physical throughput | 81.0 MiB/s | - | - | 63.1 MiB/s | 46.4 MiB/s | 64.9 MiB/s |
| Output bytes | 5.0 B | - | - | 5.0 B | 5.0 B | 5.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq any predicate expression. | yq rejects the catalog jq any predicate expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq any predicate expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 19.7 ms | - | - | 25.2 ms | 32.6 ms | 28.4 ms |
| Wall time dispersion | 0.3 ms / 20.1 ms / 19.0-20.1 ms | - | - | 0.2 ms / 27.2 ms / 24.6-27.2 ms | 0.7 ms / 34.2 ms / 31.2-34.2 ms | 0.2 ms / 30.0 ms / 27.7-30.0 ms |
| First output | 19.0 ms | - | - | 24.2 ms | 31.1 ms | 27.6 ms |
| First output dispersion | 0.2 ms / 19.4 ms / 18.2-19.4 ms | - | - | 0.3 ms / 26.1 ms / 23.8-26.1 ms | 0.7 ms / 33.1 ms / 30.1-33.1 ms | 0.3 ms / 29.0 ms / 26.9-29.0 ms |
| User CPU | 13.7 ms | - | - | 18.4 ms | 24.3 ms | 22.0 ms |
| System CPU | 5.9 ms | - | - | 7.0 ms | 7.9 ms | 6.2 ms |
| Peak RSS | 14.6 MiB | - | - | 20.3 MiB | 24.2 MiB | 21.7 MiB |
| Logical throughput | 113047.5 records/s | - | - | 88395.4 records/s | 68187.7 records/s | 78288.6 records/s |
| Physical throughput | 76.6 MiB/s | - | - | 59.9 MiB/s | 46.1 MiB/s | 62.8 MiB/s |
| Output bytes | 5.0 B | - | - | 5.0 B | 5.0 B | 5.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq any predicate expression. | yq rejects the catalog jq any predicate expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
