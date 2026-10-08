---
type: Report
title: CSV formatting
description: Measures formatting selected feature fields as CSV rows.
workload: benchmark.format-csv
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

# CSV formatting

## What this measures

The query selects `id`, `place`, and `mag` for each feature and applies `@csv`.
It measures quoting, delimiter handling, and text output for repeated rows.

## Why it matters

CSV remains a practical interchange format for spreadsheets and simple data
loads. Correct quoting matters when fields contain commas or special text.

## Input and output

The input is a natural feature snapshot. The output is one CSV string per
feature containing the three selected fields, not JSON arrays or feature
objects.

The yq adapter forces quoted string fields to match jq CSV text. It emits an empty magnitude field for null values.

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

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 3.3 ms | 28.5 ms | 41.4 ms | 6.0 ms | 6.7 ms | 6.5 ms |
| Wall time dispersion | 0.0 ms / 3.4 ms / 3.3-3.4 ms | 0.0 ms / 28.5 ms / 28.3-28.5 ms | 0.9 ms / 42.4 ms / 40.3-42.4 ms | 0.2 ms / 6.1 ms / 5.8-6.1 ms | 0.0 ms / 6.7 ms / 6.6-6.7 ms | 0.0 ms / 7.0 ms / 6.5-7.0 ms |
| First output | 3.3 ms | 27.5 ms | 39.1 ms | 6.0 ms | 6.5 ms | 6.5 ms |
| First output dispersion | 0.0 ms / 3.4 ms / 3.3-3.4 ms | 0.0 ms / 27.6 ms / 27.5-27.6 ms | 0.0 ms / 39.1 ms / 38.1-39.1 ms | 0.2 ms / 6.1 ms / 5.4-6.1 ms | 0.0 ms / 6.5 ms / 6.4-6.5 ms | 0.0 ms / 7.0 ms / 6.5-7.0 ms |
| User CPU | 2.1 ms | 33.6 ms | 40.8 ms | 3.5 ms | 3.3 ms | 4.7 ms |
| System CPU | 1.1 ms | 9.1 ms | 18.8 ms | 2.3 ms | 3.2 ms | 1.9 ms |
| Peak RSS | 3.3 MiB | 27.0 MiB | 44.4 MiB | 17.1 MiB | 19.7 MiB | 19.3 MiB |
| Logical throughput | 73984.2 records/s | 8559.9 records/s | 5890.7 records/s | 40891.6 records/s | 36456.0 records/s | 37257.6 records/s |
| Physical throughput | 50.4 MiB/s | 5.8 MiB/s | 4.0 MiB/s | 27.9 MiB/s | 24.8 MiB/s | 30.1 MiB/s |
| Output bytes | 13483.0 B | 13483.0 B | 13483.0 B | 13483.0 B | 13483.0 B | 13483.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.2 ms | 4.7 ms | 5.1 ms | 2.0 ms | 2.1 ms | 1.9 ms |
| Wall time dispersion | 0.0 ms / 1.3 ms / 1.2-1.3 ms | 0.0 ms / 4.8 ms / 4.7-4.8 ms | 0.0 ms / 5.1 ms / 4.9-5.1 ms | 0.0 ms / 2.1 ms / 1.9-2.1 ms | 0.0 ms / 2.1 ms / 2.0-2.1 ms | 0.1 ms / 2.1 ms / 1.8-2.1 ms |
| First output | 1.2 ms | 4.7 ms | 5.1 ms | 2.0 ms | 2.1 ms | 1.9 ms |
| First output dispersion | 0.0 ms / 1.3 ms / 1.2-1.3 ms | 0.0 ms / 4.7 ms / 4.4-4.7 ms | 0.0 ms / 5.1 ms / 4.9-5.1 ms | 0.0 ms / 2.1 ms / 1.9-2.1 ms | 0.0 ms / 2.1 ms / 2.0-2.1 ms | 0.1 ms / 2.1 ms / 1.8-2.1 ms |
| User CPU | 0.6 ms | 4.3 ms | 4.6 ms | 1.0 ms | 0.0 ms | 0.8 ms |
| System CPU | 0.6 ms | 1.7 ms | 1.5 ms | 1.0 ms | 1.9 ms | 0.9 ms |
| Peak RSS | 2.7 MiB | 14.8 MiB | 15.5 MiB | 15.2 MiB | 15.6 MiB | 15.3 MiB |
| Logical throughput | 5917.2 records/s | 1487.1 records/s | 1365.6 records/s | 3564.2 records/s | 3358.9 records/s | 3651.5 records/s |
| Physical throughput | 4.3 MiB/s | 1.1 MiB/s | 1.0 MiB/s | 2.6 MiB/s | 2.4 MiB/s | 3.1 MiB/s |
| Output bytes | 393.0 B | 393.0 B | 393.0 B | 393.0 B | 393.0 B | 393.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `ordered result sequence`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 112.8 ms | - | - | 158.5 ms | 200.5 ms | 207.1 ms |
| Wall time dispersion | 1.0 ms / 115.6 ms / 111.8-115.6 ms | - | - | 0.3 ms / 158.9 ms / 158.2-158.9 ms | 2.2 ms / 202.7 ms / 195.8-202.7 ms | 0.8 ms / 208.0 ms / 206.3-208.0 ms |
| First output | 79.1 ms | - | - | 79.1 ms | 120.1 ms | 126.5 ms |
| First output dispersion | 0.9 ms / 80.1 ms / 78.1-80.1 ms | - | - | 0.2 ms / 80.2 ms / 78.9-80.2 ms | 0.8 ms / 123.3 ms / 119.4-123.3 ms | 0.0 ms / 126.6 ms / 126.4-126.6 ms |
| User CPU | 89.3 ms | - | - | 114.3 ms | 160.5 ms | 174.7 ms |
| System CPU | 23.1 ms | - | - | 45.6 ms | 36.4 ms | 36.8 ms |
| Peak RSS | 56.8 MiB | - | - | 73.1 MiB | 117.8 MiB | 97.0 MiB |
| Logical throughput | 95656.0 records/s | - | - | 68091.3 records/s | 53823.6 records/s | 52100.0 records/s |
| Physical throughput | 64.8 MiB/s | - | - | 46.1 MiB/s | 36.4 MiB/s | 41.8 MiB/s |
| Output bytes | 594235.0 B | - | - | 594235.0 B | 594235.0 B | 594235.0 B |
| Outcome | timed | incorrect | incorrect | timed | timed | timed |
| Details | none | ordered result sequence | ordered result sequence | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `ordered result sequence`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 24.2 ms | - | - | 32.8 ms | 41.9 ms | 43.2 ms |
| Wall time dispersion | 0.6 ms / 25.2 ms / 23.6-25.2 ms | - | - | 0.2 ms / 33.0 ms / 32.5-33.0 ms | 0.2 ms / 43.1 ms / 41.7-43.1 ms | 0.3 ms / 43.5 ms / 42.6-43.5 ms |
| First output | 18.0 ms | - | - | 24.4 ms | 33.9 ms | 34.9 ms |
| First output dispersion | 0.0 ms / 19.1 ms / 18.0-19.1 ms | - | - | 0.0 ms / 25.4 ms / 24.4-25.4 ms | 0.0 ms / 33.9 ms / 32.8-33.9 ms | 1.0 ms / 35.9 ms / 33.9-35.9 ms |
| User CPU | 19.0 ms | - | - | 23.3 ms | 35.8 ms | 36.1 ms |
| System CPU | 5.0 ms | - | - | 9.8 ms | 6.5 ms | 6.9 ms |
| Peak RSS | 13.4 MiB | - | - | 31.2 MiB | 33.6 MiB | 33.7 MiB |
| Logical throughput | 92417.2 records/s | - | - | 68070.3 records/s | 53303.4 records/s | 51686.6 records/s |
| Physical throughput | 62.7 MiB/s | - | - | 46.2 MiB/s | 36.1 MiB/s | 41.5 MiB/s |
| Output bytes | 123926.0 B | - | - | 123926.0 B | 123926.0 B | 123926.0 B |
| Outcome | timed | incorrect | incorrect | timed | timed | timed |
| Details | none | ordered result sequence | ordered result sequence | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
