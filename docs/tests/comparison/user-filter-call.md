---
type: Report
title: User function calls
description: Measures passing a value through a user-defined filter function.
workload: benchmark.user-filter-call
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

# User function calls

## What this measures

The query defines `magnitude(f)` and calls it for each feature's magnitude.
This isolates function definition, argument passing, and repeated invocation.

## Why it matters

Reusable filters make production queries easier to maintain. A function call
should keep the same stream behavior as the equivalent inline expression.

## Input and output

The input is a natural feature snapshot. The output is one magnitude per
feature in input order, emitted as a sequence after passing through the user
function.

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
| Wall time | 2.9 ms | - | - | 5.0 ms | 6.1 ms | 6.4 ms |
| Wall time dispersion | 0.0 ms / 3.0 ms / 2.9-3.0 ms | - | - | 0.0 ms / 5.1 ms / 5.0-5.1 ms | 0.1 ms / 6.3 ms / 6.0-6.3 ms | 0.0 ms / 6.6 ms / 6.4-6.6 ms |
| First output | 2.9 ms | - | - | 5.0 ms | 6.1 ms | 6.4 ms |
| First output dispersion | 0.0 ms / 3.0 ms / 2.9-3.0 ms | - | - | 0.0 ms / 5.1 ms / 5.0-5.1 ms | 0.1 ms / 6.3 ms / 6.0-6.3 ms | 0.0 ms / 6.4 ms / 6.4-6.4 ms |
| User CPU | 0.9 ms | - | - | 2.0 ms | 4.2 ms | 4.7 ms |
| System CPU | 1.9 ms | - | - | 3.0 ms | 2.1 ms | 1.5 ms |
| Peak RSS | 3.3 MiB | - | - | 15.0 MiB | 17.7 MiB | 17.3 MiB |
| Logical throughput | 85404.3 records/s | - | - | 48878.2 records/s | 40026.2 records/s | 38059.6 records/s |
| Physical throughput | 58.2 MiB/s | - | - | 33.3 MiB/s | 27.2 MiB/s | 30.7 MiB/s |
| Output bytes | 1081.0 B | - | - | 1081.0 B | 1081.0 B | 1081.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.3 ms | - | - | 1.9 ms | 2.0 ms | 1.9 ms |
| Wall time dispersion | 0.1 ms / 1.5 ms / 1.2-1.5 ms | - | - | 0.2 ms / 2.3 ms / 1.8-2.3 ms | 0.1 ms / 2.1 ms / 1.8-2.1 ms | 0.0 ms / 2.0 ms / 1.8-2.0 ms |
| First output | 1.3 ms | - | - | 1.9 ms | 2.0 ms | 1.9 ms |
| First output dispersion | 0.1 ms / 1.5 ms / 1.2-1.5 ms | - | - | 0.2 ms / 2.3 ms / 1.8-2.3 ms | 0.1 ms / 2.1 ms / 1.8-2.1 ms | 0.0 ms / 2.0 ms / 1.8-2.0 ms |
| User CPU | 1.1 ms | - | - | 0.8 ms | 0.8 ms | 0.0 ms |
| System CPU | 0.0 ms | - | - | 1.0 ms | 0.9 ms | 1.8 ms |
| Peak RSS | 2.7 MiB | - | - | 15.1 MiB | 15.5 MiB | 15.6 MiB |
| Logical throughput | 5422.2 records/s | - | - | 3642.0 records/s | 3443.2 records/s | 3642.0 records/s |
| Physical throughput | 3.9 MiB/s | - | - | 2.6 MiB/s | 2.5 MiB/s | 3.1 MiB/s |
| Output bytes | 34.0 B | - | - | 34.0 B | 34.0 B | 34.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 95.2 ms | - | - | 148.4 ms | 189.2 ms | 197.8 ms |
| Wall time dispersion | 0.1 ms / 96.6 ms / 95.1-96.6 ms | - | - | 0.4 ms / 148.8 ms / 147.1-148.8 ms | 1.0 ms / 190.2 ms / 184.9-190.2 ms | 4.2 ms / 202.0 ms / 191.4-202.0 ms |
| First output | 79.0 ms | - | - | 147.4 ms | 188.5 ms | 197.0 ms |
| First output dispersion | 0.0 ms / 79.1 ms / 79.0-79.1 ms | - | - | 0.1 ms / 147.5 ms / 146.5-147.5 ms | 1.0 ms / 189.5 ms / 184.4-189.5 ms | 4.2 ms / 201.2 ms / 190.7-201.2 ms |
| User CPU | 73.1 ms | - | - | 106.7 ms | 150.5 ms | 162.6 ms |
| System CPU | 21.9 ms | - | - | 34.3 ms | 31.2 ms | 31.5 ms |
| Peak RSS | 56.9 MiB | - | - | 73.0 MiB | 117.6 MiB | 97.2 MiB |
| Logical throughput | 113334.0 records/s | - | - | 72718.9 records/s | 57047.1 records/s | 54559.3 records/s |
| Physical throughput | 76.8 MiB/s | - | - | 49.3 MiB/s | 38.6 MiB/s | 43.8 MiB/s |
| Output bytes | 48620.0 B | - | - | 48620.0 B | 48620.0 B | 48620.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq def expression used by this adapter.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 20.3 ms | - | - | 31.2 ms | 39.7 ms | 42.5 ms |
| Wall time dispersion | 0.1 ms / 20.4 ms / 19.3-20.4 ms | - | - | 0.5 ms / 31.9 ms / 30.7-31.9 ms | 0.1 ms / 39.8 ms / 38.8-39.8 ms | 1.1 ms / 43.6 ms / 40.3-43.6 ms |
| First output | 18.0 ms | - | - | 31.2 ms | 39.7 ms | 42.3 ms |
| First output dispersion | 0.0 ms / 18.1 ms / 18.0-18.1 ms | - | - | 0.5 ms / 31.6 ms / 30.7-31.6 ms | 0.1 ms / 39.8 ms / 38.8-39.8 ms | 1.0 ms / 43.3 ms / 40.2-43.3 ms |
| User CPU | 16.0 ms | - | - | 22.1 ms | 29.8 ms | 30.9 ms |
| System CPU | 3.0 ms | - | - | 7.7 ms | 8.2 ms | 9.7 ms |
| Peak RSS | 13.4 MiB | - | - | 29.0 MiB | 33.5 MiB | 33.3 MiB |
| Logical throughput | 110033.0 records/s | - | - | 71699.1 records/s | 56333.1 records/s | 52520.2 records/s |
| Physical throughput | 74.6 MiB/s | - | - | 48.6 MiB/s | 38.1 MiB/s | 42.2 MiB/s |
| Output bytes | 10187.0 B | - | - | 10187.0 B | 10187.0 B | 10187.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
