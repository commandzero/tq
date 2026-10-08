---
type: Report
title: String and scalar utilities
description: Measures case conversion and Unicode scalar handling across place names.
workload: benchmark.issue5-scalar-utilities
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

# String and scalar utilities

## What this measures

The query lowercases each string place, expands it to Unicode code points with
`explode`, counts those scalars, and adds the counts. It combines text
transformation with a numeric reduction.

## Why it matters

Text utilities often sit in normalization and search pipelines. Unicode-aware
length work is different from counting bytes, especially for non-ASCII data.

## Input and output

The input is a natural snapshot with place values. The output is one integer
containing the total code-point count after lowercasing, not the transformed
strings.

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

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq scalar-utility expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 5.8 ms | - | - | 4.0 ms | 5.0 ms | 5.1 ms |
| Wall time dispersion | 0.0 ms / 5.8 ms / 5.5-5.8 ms | - | - | 0.2 ms / 4.1 ms / 3.8-4.1 ms | 0.0 ms / 5.2 ms / 5.0-5.2 ms | 0.1 ms / 5.2 ms / 4.9-5.2 ms |
| First output | 5.8 ms | - | - | 4.0 ms | 5.0 ms | 5.1 ms |
| First output dispersion | 0.0 ms / 5.8 ms / 5.5-5.8 ms | - | - | 0.2 ms / 4.1 ms / 3.8-4.1 ms | 0.0 ms / 5.2 ms / 5.0-5.2 ms | 0.1 ms / 5.2 ms / 4.9-5.2 ms |
| User CPU | 5.7 ms | - | - | 2.7 ms | 2.9 ms | 3.0 ms |
| System CPU | 0.0 ms | - | - | 1.3 ms | 1.9 ms | 2.0 ms |
| Peak RSS | 3.3 MiB | - | - | 17.3 MiB | 19.6 MiB | 19.6 MiB |
| Logical throughput | 42229.1 records/s | - | - | 61756.5 records/s | 48712.3 records/s | 47443.1 records/s |
| Physical throughput | 28.8 MiB/s | - | - | 42.1 MiB/s | 33.2 MiB/s | 38.3 MiB/s |
| Output bytes | 5.0 B | - | - | 5.0 B | 5.0 B | 5.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq scalar-utility expression. | yq rejects the catalog jq scalar-utility expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq scalar-utility expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.3 ms | - | - | 1.9 ms | 1.9 ms | 1.9 ms |
| Wall time dispersion | 0.0 ms / 1.6 ms / 1.3-1.6 ms | - | - | 0.0 ms / 2.0 ms / 1.9-2.0 ms | 0.0 ms / 1.9 ms / 1.9-1.9 ms | 0.0 ms / 1.9 ms / 1.9-1.9 ms |
| First output | 1.3 ms | - | - | 1.9 ms | 1.9 ms | 1.9 ms |
| First output dispersion | 0.0 ms / 1.6 ms / 1.3-1.6 ms | - | - | 0.0 ms / 2.0 ms / 1.9-2.0 ms | 0.0 ms / 1.9 ms / 1.9-1.9 ms | 0.0 ms / 1.9 ms / 1.9-1.9 ms |
| User CPU | 1.2 ms | - | - | 0.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.0 ms | - | - | 1.8 ms | 1.9 ms | 1.9 ms |
| Peak RSS | 2.7 MiB | - | - | 15.3 MiB | 15.7 MiB | 15.7 MiB |
| Logical throughput | 5291.0 records/s | - | - | 3664.9 records/s | 3630.7 records/s | 3668.8 records/s |
| Physical throughput | 3.8 MiB/s | - | - | 2.7 MiB/s | 2.6 MiB/s | 3.1 MiB/s |
| Output bytes | 4.0 B | - | - | 4.0 B | 4.0 B | 4.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq scalar-utility expression. | yq rejects the catalog jq scalar-utility expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq scalar-utility expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 209.7 ms | - | - | 91.0 ms | 132.7 ms | 142.3 ms |
| Wall time dispersion | 0.9 ms / 210.6 ms / 208.3-210.6 ms | - | - | 0.1 ms / 91.5 ms / 90.8-91.5 ms | 1.2 ms / 134.2 ms / 131.5-134.2 ms | 0.3 ms / 142.6 ms / 141.1-142.6 ms |
| First output | 207.0 ms | - | - | 90.8 ms | 131.8 ms | 142.3 ms |
| First output dispersion | 0.6 ms / 207.5 ms / 205.5-207.5 ms | - | - | 0.6 ms / 91.5 ms / 89.6-91.5 ms | 1.1 ms / 133.8 ms / 130.7-133.8 ms | 0.0 ms / 142.3 ms / 140.2-142.3 ms |
| User CPU | 177.0 ms | - | - | 86.5 ms | 127.0 ms | 136.7 ms |
| System CPU | 33.0 ms | - | - | 4.0 ms | 5.0 ms | 5.0 ms |
| Peak RSS | 57.1 MiB | - | - | 79.3 MiB | 117.5 MiB | 99.7 MiB |
| Logical throughput | 51453.2 records/s | - | - | 118648.2 records/s | 81346.5 records/s | 75857.9 records/s |
| Physical throughput | 34.9 MiB/s | - | - | 80.4 MiB/s | 55.0 MiB/s | 60.9 MiB/s |
| Output bytes | 7.0 B | - | - | 7.0 B | 7.0 B | 7.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq scalar-utility expression. | yq rejects the catalog jq scalar-utility expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq scalar-utility expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 41.3 ms | - | - | 20.5 ms | 28.9 ms | 29.7 ms |
| Wall time dispersion | 0.1 ms / 42.7 ms / 41.2-42.7 ms | - | - | 0.3 ms / 21.0 ms / 20.2-21.0 ms | 0.1 ms / 29.7 ms / 28.8-29.7 ms | 0.5 ms / 31.1 ms / 29.2-31.1 ms |
| First output | 41.3 ms | - | - | 20.2 ms | 28.5 ms | 29.7 ms |
| First output dispersion | 0.1 ms / 42.3 ms / 41.2-42.3 ms | - | - | 0.0 ms / 21.0 ms / 20.2-21.0 ms | 0.0 ms / 29.7 ms / 28.5-29.7 ms | 0.9 ms / 30.6 ms / 28.6-30.6 ms |
| User CPU | 36.1 ms | - | - | 18.1 ms | 26.6 ms | 26.4 ms |
| System CPU | 6.0 ms | - | - | 2.1 ms | 2.0 ms | 3.0 ms |
| Peak RSS | 13.4 MiB | - | - | 33.4 MiB | 37.7 MiB | 37.5 MiB |
| Logical throughput | 54092.0 records/s | - | - | 108996.9 records/s | 77408.2 records/s | 75163.2 records/s |
| Physical throughput | 36.7 MiB/s | - | - | 73.9 MiB/s | 52.4 MiB/s | 60.4 MiB/s |
| Output bytes | 6.0 B | - | - | 6.0 B | 6.0 B | 6.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq scalar-utility expression. | yq rejects the catalog jq scalar-utility expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
