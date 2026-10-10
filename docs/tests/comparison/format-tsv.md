---
type: Report
title: TSV formatting
description: Measures formatting selected feature fields as tab-separated rows.
workload: benchmark.format-tsv
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

# TSV formatting

## What this measures

The query selects `id`, `place`, and `mag` for each feature and applies `@tsv`.
It measures tab escaping and text output across the complete feature stream.

## Why it matters

Tab-separated rows are easy to pipe into Unix tools and import into tables.
Escaping embedded tabs and control characters is part of the real work.

## Input and output

The input is a natural feature snapshot. The output is one TSV string per
feature containing the three selected fields, not a JSON representation.

The yq adapter converts null magnitudes to empty fields, matching jq TSV text. Numeric zero remains a numeric field.

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
| Wall time | 3.2 ms | 14.4 ms | 25.4 ms | 5.6 ms | 6.6 ms | 6.7 ms |
| Wall time dispersion | 0.0 ms / 3.6 ms / 3.2-3.6 ms | 0.0 ms / 14.9 ms / 14.3-14.9 ms | 0.2 ms / 27.7 ms / 25.2-27.7 ms | 0.1 ms / 5.9 ms / 5.5-5.9 ms | 0.0 ms / 6.8 ms / 6.6-6.8 ms | 0.0 ms / 6.9 ms / 6.7-6.9 ms |
| First output | 3.2 ms | 13.8 ms | 24.4 ms | 5.5 ms | 6.6 ms | 6.5 ms |
| First output dispersion | 0.0 ms / 3.3 ms / 3.2-3.3 ms | 0.0 ms / 13.8 ms / 12.8-13.8 ms | 0.0 ms / 25.4 ms / 24.4-25.4 ms | 0.1 ms / 5.9 ms / 5.4-5.9 ms | 0.0 ms / 6.6 ms / 6.5-6.6 ms | 0.0 ms / 6.5 ms / 6.5-6.5 ms |
| User CPU | 3.0 ms | 14.5 ms | 28.2 ms | 2.7 ms | 3.4 ms | 4.9 ms |
| System CPU | 0.0 ms | 6.8 ms | 11.1 ms | 2.7 ms | 3.3 ms | 1.9 ms |
| Peak RSS | 3.3 MiB | 25.5 MiB | 37.1 MiB | 17.1 MiB | 19.8 MiB | 19.5 MiB |
| Logical throughput | 76729.6 records/s | 16978.6 records/s | 9619.9 records/s | 43385.5 records/s | 37104.6 records/s | 36396.2 records/s |
| Physical throughput | 52.3 MiB/s | 11.6 MiB/s | 6.5 MiB/s | 29.6 MiB/s | 25.3 MiB/s | 29.4 MiB/s |
| Output bytes | 12019.0 B | 12019.0 B | 12019.0 B | 12019.0 B | 12019.0 B | 12019.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.2 ms | 3.3 ms | 3.5 ms | 2.0 ms | 1.9 ms | 2.1 ms |
| Wall time dispersion | 0.0 ms / 1.5 ms / 1.1-1.5 ms | 0.1 ms / 3.5 ms / 3.2-3.5 ms | 0.0 ms / 3.7 ms / 3.4-3.7 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms | 0.1 ms / 2.0 ms / 1.8-2.0 ms | 0.1 ms / 2.2 ms / 2.0-2.2 ms |
| First output | 1.2 ms | 3.3 ms | 3.3 ms | 2.0 ms | 1.9 ms | 2.1 ms |
| First output dispersion | 0.0 ms / 1.5 ms / 1.1-1.5 ms | 0.1 ms / 3.3 ms / 3.2-3.3 ms | 0.0 ms / 3.3 ms / 3.3-3.3 ms | 0.0 ms / 2.0 ms / 1.9-2.0 ms | 0.1 ms / 2.0 ms / 1.8-2.0 ms | 0.1 ms / 2.2 ms / 2.0-2.2 ms |
| User CPU | 1.1 ms | 1.1 ms | 1.9 ms | 0.9 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.0 ms | 2.1 ms | 1.9 ms | 0.9 ms | 1.9 ms | 1.9 ms |
| Peak RSS | 2.7 MiB | 13.3 MiB | 14.1 MiB | 15.0 MiB | 15.7 MiB | 15.4 MiB |
| Logical throughput | 5998.3 records/s | 2091.4 records/s | 2013.2 records/s | 3560.5 records/s | 3591.6 records/s | 3328.6 records/s |
| Physical throughput | 4.3 MiB/s | 1.5 MiB/s | 1.5 MiB/s | 2.6 MiB/s | 2.6 MiB/s | 2.8 MiB/s |
| Output bytes | 351.0 B | 351.0 B | 351.0 B | 351.0 B | 351.0 B | 351.0 B |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `ordered result sequence`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 112.0 ms | - | - | 156.8 ms | 197.9 ms | 207.1 ms |
| Wall time dispersion | 0.9 ms / 112.9 ms / 109.4-112.9 ms | - | - | 0.3 ms / 157.1 ms / 155.4-157.1 ms | 0.5 ms / 198.6 ms / 197.4-198.6 ms | 1.2 ms / 208.3 ms / 202.7-208.3 ms |
| First output | 78.1 ms | - | - | 81.2 ms | 122.2 ms | 127.6 ms |
| First output dispersion | 0.0 ms / 80.1 ms / 78.1-80.1 ms | - | - | 0.0 ms / 81.2 ms / 80.1-81.2 ms | 1.0 ms / 124.3 ms / 121.2-124.3 ms | 0.0 ms / 131.8 ms / 127.6-131.8 ms |
| User CPU | 89.2 ms | - | - | 124.5 ms | 166.6 ms | 174.6 ms |
| System CPU | 22.3 ms | - | - | 33.7 ms | 31.6 ms | 33.3 ms |
| Peak RSS | 56.8 MiB | - | - | 73.2 MiB | 117.8 MiB | 97.3 MiB |
| Logical throughput | 96348.5 records/s | - | - | 68827.8 records/s | 54544.7 records/s | 52102.3 records/s |
| Physical throughput | 65.3 MiB/s | - | - | 46.6 MiB/s | 36.9 MiB/s | 41.8 MiB/s |
| Output bytes | 529487.0 B | - | - | 529487.0 B | 529487.0 B | 529487.0 B |
| Outcome | timed | incorrect | incorrect | timed | timed | timed |
| Details | none | ordered result sequence | ordered result sequence | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `ordered result sequence`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 22.8 ms | - | - | 33.1 ms | 41.0 ms | 43.5 ms |
| Wall time dispersion | 0.2 ms / 23.0 ms / 22.2-23.0 ms | - | - | 0.6 ms / 33.6 ms / 32.4-33.6 ms | 1.0 ms / 42.0 ms / 39.6-42.0 ms | 0.4 ms / 43.9 ms / 42.3-43.9 ms |
| First output | 18.1 ms | - | - | 26.4 ms | 33.8 ms | 36.0 ms |
| First output dispersion | 0.0 ms / 18.1 ms / 17.0-18.1 ms | - | - | 0.0 ms / 26.5 ms / 25.4-26.5 ms | 1.1 ms / 34.9 ms / 32.7-34.9 ms | 0.0 ms / 36.0 ms / 35.9-36.0 ms |
| User CPU | 19.0 ms | - | - | 22.3 ms | 29.5 ms | 33.3 ms |
| System CPU | 3.1 ms | - | - | 11.1 ms | 10.5 ms | 9.6 ms |
| Peak RSS | 13.4 MiB | - | - | 31.2 MiB | 33.9 MiB | 33.3 MiB |
| Logical throughput | 97883.7 records/s | - | - | 67576.2 records/s | 54438.7 records/s | 51411.9 records/s |
| Physical throughput | 66.4 MiB/s | - | - | 45.8 MiB/s | 36.9 MiB/s | 41.3 MiB/s |
| Output bytes | 110526.0 B | - | - | 110526.0 B | 110526.0 B | 110526.0 B |
| Outcome | timed | incorrect | incorrect | timed | timed | timed |
| Details | none | ordered result sequence | ordered result sequence | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
