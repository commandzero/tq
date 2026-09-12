---
type: Report
title: "User function selection"
description: "Measures filtering with a named predicate and returning matching IDs."
workload: benchmark.user-filter-select
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# User function selection

## What this measures

The query defines `significant` as a magnitude predicate, applies it inside
`select`, and collects the matching IDs. It tests callback invocation and
conditional selection in one blocking result.

## Why it matters

Teams often turn business rules into named filters. This case checks that the
rule remains usable inside a collection operation instead of only inline.

## Input and output

The input is a natural feature snapshot. The output is one array of IDs whose
magnitudes meet the threshold. It does not return the full selected features.

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

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 2.9 ms | - | - | 3.9 ms | 4.6 ms | 4.2 ms |
| Wall time dispersion | 0.1 ms / 3.2 ms / 2.7-3.2 ms | - | - | 0.1 ms / 4.1 ms / 3.7-4.1 ms | 0.2 ms / 5.5 ms / 4.2-5.5 ms | 0.2 ms / 4.7 ms / 4.0-4.7 ms |
| First output | 2.8 ms | - | - | 3.6 ms | 4.2 ms | 3.9 ms |
| First output dispersion | 0.0 ms / 3.2 ms / 2.6-3.2 ms | - | - | 0.0 ms / 3.7 ms / 3.5-3.7 ms | 0.2 ms / 5.3 ms / 3.9-5.3 ms | 0.1 ms / 4.4 ms / 3.9-4.4 ms |
| User CPU | 1.9 ms | - | - | 2.6 ms | 3.1 ms | 3.0 ms |
| System CPU | 0.9 ms | - | - | 1.1 ms | 1.1 ms | 1.1 ms |
| Peak RSS | 4.3 MiB | - | - | 9.1 MiB | 9.7 MiB | 9.1 MiB |
| Logical throughput | 66076.3 records/s | - | - | 49845.8 records/s | 42525.2 records/s | 46163.0 records/s |
| Physical throughput | 45.2 MiB/s | - | - | 34.1 MiB/s | 29.1 MiB/s | 37.4 MiB/s |
| Output bytes | 1119.0 B | - | - | 792.0 B | 792.0 B | 792.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | - | - | 1.3 ms | 1.3 ms | 1.2 ms |
| Wall time dispersion | 0.0 ms / 1.8 ms / 1.3-1.8 ms | - | - | 0.1 ms / 1.6 ms / 1.1-1.6 ms | 0.1 ms / 1.5 ms / 1.2-1.5 ms | 0.0 ms / 1.5 ms / 1.2-1.5 ms |
| First output | 1.4 ms | - | - | 1.0 ms | 1.0 ms | 1.0 ms |
| First output dispersion | 0.0 ms / 1.6 ms / 1.2-1.6 ms | - | - | 0.0 ms / 1.4 ms / 1.0-1.5 ms | 0.0 ms / 1.4 ms / 1.0-1.4 ms | 0.0 ms / 1.3 ms / 1.0-1.4 ms |
| User CPU | 0.0 ms | - | - | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 1.3 ms | - | - | 1.1 ms | 1.1 ms | 1.1 ms |
| Peak RSS | 4.0 MiB | - | - | 8.1 MiB | 8.4 MiB | 8.0 MiB |
| Logical throughput | 1330.7 records/s | - | - | 1550.4 records/s | 1525.6 records/s | 1696.4 records/s |
| Physical throughput | 1.1 MiB/s | - | - | 1.3 MiB/s | 1.3 MiB/s | 1.6 MiB/s |
| Output bytes | 3.0 B | - | - | 5.0 B | 5.0 B | 5.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 103.5 ms | - | - | 145.5 ms | 186.8 ms | 162.9 ms |
| Wall time dispersion | 0.5 ms / 106.7 ms / 101.7-106.7 ms | - | - | 0.5 ms / 148.6 ms / 143.5-148.6 ms | 1.5 ms / 190.3 ms / 184.0-190.3 ms | 2.0 ms / 170.4 ms / 159.4-170.4 ms |
| First output | 99.3 ms | - | - | 141.1 ms | 182.5 ms | 158.7 ms |
| First output dispersion | 0.6 ms / 102.8 ms / 97.8-102.8 ms | - | - | 0.6 ms / 143.4 ms / 139.6-143.4 ms | 1.5 ms / 184.9 ms / 179.8-184.9 ms | 1.9 ms / 165.5 ms / 155.1-165.5 ms |
| User CPU | 78.7 ms | - | - | 115.9 ms | 154.3 ms | 135.1 ms |
| System CPU | 24.1 ms | - | - | 28.4 ms | 31.0 ms | 28.7 ms |
| Peak RSS | 60.4 MiB | - | - | 69.6 MiB | 89.7 MiB | 78.4 MiB |
| Logical throughput | 108971.2 records/s | - | - | 77471.0 records/s | 60350.2 records/s | 69188.4 records/s |
| Physical throughput | 73.9 MiB/s | - | - | 52.5 MiB/s | 40.9 MiB/s | 55.5 MiB/s |
| Output bytes | 57877.0 B | - | - | 40962.0 B | 40962.0 B | 40962.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 20.7 ms | - | - | 29.0 ms | 36.2 ms | 32.8 ms |
| Wall time dispersion | 0.9 ms / 21.9 ms / 19.3-21.9 ms | - | - | 0.4 ms / 31.5 ms / 28.6-31.5 ms | 0.3 ms / 36.8 ms / 35.1-36.8 ms | 0.3 ms / 36.0 ms / 32.1-36.0 ms |
| First output | 20.0 ms | - | - | 28.1 ms | 35.0 ms | 31.8 ms |
| First output dispersion | 0.9 ms / 21.3 ms / 18.5-21.3 ms | - | - | 0.4 ms / 30.6 ms / 27.3-30.6 ms | 0.2 ms / 35.6 ms / 34.0-35.6 ms | 0.2 ms / 35.1 ms / 31.3-35.1 ms |
| User CPU | 13.8 ms | - | - | 23.5 ms | 28.2 ms | 27.2 ms |
| System CPU | 5.9 ms | - | - | 6.1 ms | 7.7 ms | 5.2 ms |
| Peak RSS | 14.6 MiB | - | - | 20.2 MiB | 24.3 MiB | 21.7 MiB |
| Logical throughput | 107309.1 records/s | - | - | 76654.1 records/s | 61423.4 records/s | 67880.9 records/s |
| Physical throughput | 72.7 MiB/s | - | - | 51.9 MiB/s | 41.6 MiB/s | 54.5 MiB/s |
| Output bytes | 11930.0 B | - | - | 8514.0 B | 8514.0 B | 8514.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
