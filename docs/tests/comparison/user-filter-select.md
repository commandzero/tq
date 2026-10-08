---
type: Report
title: User function selection
description: Measures filtering with a named predicate and returning matching IDs.
workload: benchmark.user-filter-select
generated:
  by: codex/gpt-5.6-luna
  at: 2026-09-10T20:15:01Z
benchmark_runs:
- campaign_id: 2026-09-11
  provenance: Native Linux workload report; captured tool identities are listed in the Results section. No executable hashes or unique campaign ID retained.
  binaries:
    tq:
      version: tq 0.1.0 (TOON v3; jq target 1.8.x; revision a4b4d916-worktree)
      sha256: null
    jq:
      version: jq-1.8.1
      sha256: null
    yq:
      version: yq (https://github.com/mikefarah/yq/) version v4.53.2
      sha256: null
- campaign_id: 2026-10-08T15:10:27.867608118Z
  binaries:
    jq:
      version: jq-1.8.1
      sha256: 020468de7539ce70ef1bceaf7cde2e8c4f2ca6c3afb84642aabc5c97d9fc2a0d
    tq:
      version: tq 0.5.0 (TOON v4.1; jq target 1.8.x; revision unknown)
      sha256: 474625b11f0ec3a1fa125bbeb4e55008efa50ec713b0c398822ada48f0289b91
    yq:
      version: yq (https://github.com/mikefarah/yq/) version v4.53.2
      sha256: d56bf5c6819e8e696340c312bd70f849dc1678a7cda9c2ad63eebd906371d56b
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
Last updated: 2026-10-08
Suite: `natural-corpus` | Profile: `standard` | Campaign status: `observed-failures`

Tools: `jq` (jq-1.8.1), `tq` (tq 0.5.0 (TOON v4.1; jq target 1.8.x; revision unknown)), `yq` (yq (https://github.com/mikefarah/yq/) version v4.53.2)
Environment: `linux` / `x86_64`, AMD Ryzen 7 7700 8-Core Processor, 16 logical CPUs, 61.9 GiB RAM; kernel `Linux 7.2.4-ogc3.1.fc44.x86_64 #1 SMP PREEMPT_DYNAMIC Sun Sep 13 01:41:06 UTC 2026`; compiler profile `release-benchmark`

Peak RSS is authoritative only after the campaign RSS preflight passes. Missing or invalid values are `-`, never estimates. RSS collector provenance (outside measurement tables): `linux-wait4`.
Measurement method (outside measurement tables): `tq-bench` native measurement: RSS scope `wait4 child lifetime including pre exec waited descendants and threads`; residual RSS floor `2.6 MiB` retained, not subtracted; observed control excess `0.9 ms`; primary timing is sampler-free.

Compare columns with the same input format to isolate tool differences. Missing adapters have `-` measurement cells and `not recorded` outcome/detail cells.

Timing rows show numeric medians followed by compact MAD / p95 / range rows, with one decimal place and a unit in each measurement cell. CPU, throughput, and output cells use the captured summary values.

### usgs-all-day

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 3.0 ms | - | - | 3.9 ms | 5.0 ms | 5.3 ms |
| Wall time dispersion | 0.0 ms / 3.3 ms / 3.0-3.3 ms | - | - | 0.1 ms / 4.3 ms / 3.8-4.3 ms | 0.0 ms / 5.2 ms / 5.0-5.2 ms | 0.1 ms / 5.4 ms / 5.2-5.4 ms |
| First output | 3.0 ms | - | - | 3.9 ms | 5.0 ms | 5.3 ms |
| First output dispersion | 0.0 ms / 3.3 ms / 3.0-3.3 ms | - | - | 0.1 ms / 4.3 ms / 3.8-4.3 ms | 0.0 ms / 5.2 ms / 5.0-5.2 ms | 0.1 ms / 5.4 ms / 5.2-5.4 ms |
| User CPU | 2.9 ms | - | - | 1.9 ms | 3.0 ms | 4.1 ms |
| System CPU | 0.0 ms | - | - | 1.9 ms | 2.0 ms | 1.0 ms |
| Peak RSS | 3.3 MiB | - | - | 17.1 MiB | 19.9 MiB | 19.6 MiB |
| Logical throughput | 80928.7 records/s | - | - | 61866.1 records/s | 48770.7 records/s | 46185.9 records/s |
| Physical throughput | 55.2 MiB/s | - | - | 42.2 MiB/s | 33.2 MiB/s | 37.3 MiB/s |
| Output bytes | 1272.0 B | - | - | 910.0 B | 910.0 B | 910.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.3 ms | - | - | 2.1 ms | 2.0 ms | 2.0 ms |
| Wall time dispersion | 0.1 ms / 1.5 ms / 1.2-1.5 ms | - | - | 0.1 ms / 2.2 ms / 2.0-2.2 ms | 0.2 ms / 2.2 ms / 1.8-2.2 ms | 0.0 ms / 2.1 ms / 1.9-2.1 ms |
| First output | 1.3 ms | - | - | 2.1 ms | 2.0 ms | 2.0 ms |
| First output dispersion | 0.1 ms / 1.5 ms / 1.2-1.5 ms | - | - | 0.1 ms / 2.2 ms / 2.0-2.2 ms | 0.2 ms / 2.2 ms / 1.8-2.2 ms | 0.0 ms / 2.1 ms / 1.9-2.1 ms |
| User CPU | 1.1 ms | - | - | 0.0 ms | 0.9 ms | 0.0 ms |
| System CPU | 0.0 ms | - | - | 1.9 ms | 0.9 ms | 1.8 ms |
| Peak RSS | 2.7 MiB | - | - | 17.3 MiB | 15.7 MiB | 15.2 MiB |
| Logical throughput | 5339.4 records/s | - | - | 3306.6 records/s | 3586.1 records/s | 3580.6 records/s |
| Physical throughput | 3.9 MiB/s | - | - | 2.4 MiB/s | 2.6 MiB/s | 3.0 MiB/s |
| Output bytes | 54.0 B | - | - | 41.0 B | 41.0 B | 41.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 111.9 ms | - | - | 89.5 ms | 132.5 ms | 140.4 ms |
| Wall time dispersion | 0.2 ms / 114.2 ms / 111.6-114.2 ms | - | - | 0.8 ms / 90.3 ms / 88.3-90.3 ms | 0.1 ms / 132.7 ms / 130.9-132.7 ms | 0.3 ms / 140.6 ms / 137.9-140.6 ms |
| First output | 108.6 ms | - | - | 89.5 ms | 131.6 ms | 140.2 ms |
| First output dispersion | 0.1 ms / 110.5 ms / 108.5-110.5 ms | - | - | 0.2 ms / 89.8 ms / 87.5-89.8 ms | 0.1 ms / 131.7 ms / 130.7-131.7 ms | 0.0 ms / 140.2 ms / 137.0-140.2 ms |
| User CPU | 89.8 ms | - | - | 83.1 ms | 127.0 ms | 133.1 ms |
| System CPU | 21.9 ms | - | - | 5.0 ms | 5.0 ms | 7.0 ms |
| Peak RSS | 56.8 MiB | - | - | 77.3 MiB | 117.8 MiB | 97.2 MiB |
| Logical throughput | 96462.2 records/s | - | - | 120529.8 records/s | 81420.8 records/s | 76877.6 records/s |
| Physical throughput | 65.3 MiB/s | - | - | 81.6 MiB/s | 55.1 MiB/s | 61.7 MiB/s |
| Output bytes | 53832.0 B | - | - | 38137.0 B | 38137.0 B | 38137.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 21.2 ms | - | - | 19.7 ms | 28.4 ms | 29.1 ms |
| Wall time dispersion | 0.2 ms / 21.4 ms / 20.7-21.4 ms | - | - | 0.0 ms / 20.6 ms / 19.7-20.6 ms | 0.4 ms / 28.9 ms / 28.1-28.9 ms | 0.2 ms / 29.4 ms / 28.9-29.4 ms |
| First output | 21.2 ms | - | - | 19.7 ms | 28.4 ms | 28.6 ms |
| First output dispersion | 0.0 ms / 21.2 ms / 20.1-21.2 ms | - | - | 0.4 ms / 20.1 ms / 19.1-20.1 ms | 0.2 ms / 28.6 ms / 27.5-28.6 ms | 0.0 ms / 29.4 ms / 28.6-29.4 ms |
| User CPU | 17.6 ms | - | - | 18.3 ms | 25.0 ms | 25.7 ms |
| System CPU | 3.0 ms | - | - | 2.0 ms | 3.0 ms | 3.0 ms |
| Peak RSS | 13.4 MiB | - | - | 31.3 MiB | 33.6 MiB | 33.7 MiB |
| Logical throughput | 105357.5 records/s | - | - | 113142.6 records/s | 78581.7 records/s | 76698.6 records/s |
| Physical throughput | 71.4 MiB/s | - | - | 76.7 MiB/s | 53.2 MiB/s | 61.6 MiB/s |
| Output bytes | 10831.0 B | - | - | 7660.0 B | 7660.0 B | 7660.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
