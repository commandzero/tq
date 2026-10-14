---
type: Report
title: Regular-expression testing
description: Measures Unicode-aware pattern testing over feature place names.
workload: benchmark.regex-test
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

# Regular-expression testing

## What this measures

The query tests each string place value against `^[A-Z]` and counts the
matches. It combines string filtering, a regular expression, and a numeric
reduction.

## Why it matters

Pattern checks sit inside log filters, validation rules, and text cleanup jobs.
This case measures the work of testing every candidate rather than finding one
match and stopping.

## Input and output

The input is a natural feature snapshot with place strings and other values.
The output is one integer containing the number of strings that start with an
ASCII uppercase letter.

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

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq strings/test expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 3.3 ms | - | - | 5.5 ms | 6.7 ms | 6.7 ms |
| Wall time dispersion | 0.1 ms / 3.6 ms / 3.3-3.6 ms | - | - | 0.0 ms / 5.5 ms / 5.3-5.5 ms | 0.3 ms / 7.0 ms / 6.4-7.0 ms | 0.1 ms / 6.9 ms / 6.6-6.9 ms |
| First output | 3.3 ms | - | - | 5.5 ms | 6.4 ms | 6.5 ms |
| First output dispersion | 0.1 ms / 3.6 ms / 3.3-3.6 ms | - | - | 0.0 ms / 5.5 ms / 5.3-5.5 ms | 0.0 ms / 6.5 ms / 6.4-6.5 ms | 0.0 ms / 6.5 ms / 6.5-6.5 ms |
| User CPU | 2.6 ms | - | - | 3.2 ms | 4.7 ms | 5.5 ms |
| System CPU | 0.9 ms | - | - | 2.1 ms | 1.9 ms | 1.1 ms |
| Peak RSS | 3.6 MiB | - | - | 18.2 MiB | 20.8 MiB | 20.3 MiB |
| Logical throughput | 73383.5 records/s | - | - | 44647.8 records/s | 36428.8 records/s | 36331.1 records/s |
| Physical throughput | 50.0 MiB/s | - | - | 30.4 MiB/s | 24.8 MiB/s | 29.3 MiB/s |
| Output bytes | 4.0 B | - | - | 4.0 B | 4.0 B | 4.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq strings/test expression. | yq rejects the catalog jq strings/test expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq strings/test expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.4 ms | - | - | 2.4 ms | 2.2 ms | 2.2 ms |
| Wall time dispersion | 0.0 ms / 1.6 ms / 1.3-1.6 ms | - | - | 0.0 ms / 2.4 ms / 2.3-2.4 ms | 0.1 ms / 2.4 ms / 1.9-2.4 ms | 0.0 ms / 2.4 ms / 2.2-2.4 ms |
| First output | 1.4 ms | - | - | 2.3 ms | 2.2 ms | 2.2 ms |
| First output dispersion | 0.0 ms / 1.6 ms / 1.3-1.6 ms | - | - | 0.0 ms / 2.4 ms / 2.3-2.4 ms | 0.1 ms / 2.4 ms / 1.9-2.4 ms | 0.0 ms / 2.3 ms / 2.2-2.3 ms |
| User CPU | 1.2 ms | - | - | 1.0 ms | 1.0 ms | 0.8 ms |
| System CPU | 0.0 ms | - | - | 1.3 ms | 0.9 ms | 1.5 ms |
| Peak RSS | 2.9 MiB | - | - | 17.8 MiB | 16.6 MiB | 16.3 MiB |
| Logical throughput | 5169.9 records/s | - | - | 2902.2 records/s | 3126.4 records/s | 3140.4 records/s |
| Physical throughput | 3.7 MiB/s | - | - | 2.1 MiB/s | 2.3 MiB/s | 2.7 MiB/s |
| Output bytes | 2.0 B | - | - | 2.0 B | 2.0 B | 2.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq strings/test expression. | yq rejects the catalog jq strings/test expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq strings/test expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 122.3 ms | - | - | 136.9 ms | 179.4 ms | 187.0 ms |
| Wall time dispersion | 0.0 ms / 126.7 ms / 122.3-126.7 ms | - | - | 0.9 ms / 137.8 ms / 135.7-137.8 ms | 1.7 ms / 181.2 ms / 176.7-181.2 ms | 0.7 ms / 187.6 ms / 184.9-187.6 ms |
| First output | 119.1 ms | - | - | 135.9 ms | 179.1 ms | 186.5 ms |
| First output dispersion | 0.0 ms / 123.2 ms / 119.1-123.2 ms | - | - | 1.0 ms / 137.1 ms / 134.9-137.1 ms | 1.1 ms / 180.1 ms / 176.0-180.1 ms | 1.1 ms / 187.6 ms / 184.3-187.6 ms |
| User CPU | 98.9 ms | - | - | 131.3 ms | 171.4 ms | 179.9 ms |
| System CPU | 23.0 ms | - | - | 5.0 ms | 6.0 ms | 6.0 ms |
| Peak RSS | 57.1 MiB | - | - | 80.3 MiB | 118.7 MiB | 100.5 MiB |
| Logical throughput | 88239.9 records/s | - | - | 78834.7 records/s | 60149.7 records/s | 57722.3 records/s |
| Physical throughput | 59.8 MiB/s | - | - | 53.4 MiB/s | 40.7 MiB/s | 46.3 MiB/s |
| Output bytes | 6.0 B | - | - | 6.0 B | 6.0 B | 6.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq strings/test expression. | yq rejects the catalog jq strings/test expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq strings/test expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 23.1 ms | - | - | 30.1 ms | 38.7 ms | 39.4 ms |
| Wall time dispersion | 0.0 ms / 23.2 ms / 22.8-23.2 ms | - | - | 0.8 ms / 31.4 ms / 29.3-31.4 ms | 0.7 ms / 39.5 ms / 37.7-39.5 ms | 0.4 ms / 40.1 ms / 39.0-40.1 ms |
| First output | 23.1 ms | - | - | 29.5 ms | 38.0 ms | 39.1 ms |
| First output dispersion | 0.0 ms / 23.2 ms / 22.2-23.2 ms | - | - | 0.2 ms / 31.4 ms / 29.3-31.4 ms | 1.1 ms / 39.1 ms / 36.9-39.1 ms | 0.1 ms / 40.1 ms / 39.0-40.1 ms |
| User CPU | 17.7 ms | - | - | 26.7 ms | 36.1 ms | 37.6 ms |
| System CPU | 5.0 ms | - | - | 3.0 ms | 3.1 ms | 1.3 ms |
| Peak RSS | 13.8 MiB | - | - | 34.2 MiB | 38.5 MiB | 38.4 MiB |
| Logical throughput | 96542.8 records/s | - | - | 74248.9 records/s | 57688.8 records/s | 56726.4 records/s |
| Physical throughput | 65.5 MiB/s | - | - | 50.4 MiB/s | 39.1 MiB/s | 45.6 MiB/s |
| Output bytes | 5.0 B | - | - | 5.0 B | 5.0 B | 5.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq strings/test expression. | yq rejects the catalog jq strings/test expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
