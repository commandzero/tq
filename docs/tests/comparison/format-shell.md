---
type: Report
title: Shell quoting
description: Measures shell-safe formatting of selected feature fields.
workload: benchmark.format-shell
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

# Shell quoting

## What this measures

The query selects `id`, `place`, and `mag` for each feature and applies `@sh`.
It exercises quoting and escaping for values that may contain shell-sensitive
characters.

## Why it matters

Generated command arguments need quoting that preserves data instead of letting
the shell interpret it. This is formatting work, not permission to execute the
result.

## Input and output

The input is a natural feature snapshot. The output is one shell-quoted string
per feature containing the selected fields, not a command and not the original
array.

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

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq @sh expression for this array.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 3.3 ms | - | - | 5.6 ms | 6.7 ms | 6.7 ms |
| Wall time dispersion | 0.0 ms / 3.6 ms / 3.3-3.6 ms | - | - | 0.1 ms / 5.6 ms / 5.5-5.6 ms | 0.2 ms / 6.9 ms / 6.4-6.9 ms | 0.0 ms / 7.0 ms / 6.7-7.0 ms |
| First output | 3.3 ms | - | - | 5.4 ms | 6.5 ms | 6.4 ms |
| First output dispersion | 0.0 ms / 3.3 ms / 3.3-3.3 ms | - | - | 0.0 ms / 5.5 ms / 5.4-5.5 ms | 0.0 ms / 6.5 ms / 6.4-6.5 ms | 0.1 ms / 7.0 ms / 6.4-7.0 ms |
| User CPU | 3.1 ms | - | - | 4.5 ms | 4.5 ms | 4.4 ms |
| System CPU | 0.0 ms | - | - | 1.0 ms | 2.1 ms | 2.2 ms |
| Peak RSS | 3.3 MiB | - | - | 17.0 MiB | 19.8 MiB | 19.2 MiB |
| Logical throughput | 73427.6 records/s | - | - | 43924.4 records/s | 36488.7 records/s | 36304.1 records/s |
| Physical throughput | 50.0 MiB/s | - | - | 29.9 MiB/s | 24.8 MiB/s | 29.3 MiB/s |
| Output bytes | 12507.0 B | - | - | 12505.0 B | 12505.0 B | 12505.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq @sh expression for this array. | yq rejects the catalog jq @sh expression for this array. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq @sh expression for this array.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.2 ms | - | - | 2.1 ms | 2.0 ms | 2.1 ms |
| Wall time dispersion | 0.0 ms / 1.3 ms / 1.2-1.3 ms | - | - | 0.1 ms / 2.2 ms / 1.9-2.2 ms | 0.1 ms / 2.1 ms / 1.9-2.1 ms | 0.0 ms / 2.1 ms / 2.0-2.1 ms |
| First output | 1.2 ms | - | - | 2.1 ms | 2.0 ms | 2.1 ms |
| First output dispersion | 0.0 ms / 1.3 ms / 1.2-1.3 ms | - | - | 0.1 ms / 2.2 ms / 1.9-2.2 ms | 0.1 ms / 2.1 ms / 1.9-2.1 ms | 0.0 ms / 2.1 ms / 2.0-2.1 ms |
| User CPU | 1.1 ms | - | - | 1.0 ms | 0.0 ms | 0.0 ms |
| System CPU | 0.0 ms | - | - | 1.0 ms | 1.8 ms | 1.9 ms |
| Peak RSS | 2.7 MiB | - | - | 15.3 MiB | 15.6 MiB | 15.5 MiB |
| Logical throughput | 5942.3 records/s | - | - | 3336.5 records/s | 3524.7 records/s | 3333.3 records/s |
| Physical throughput | 4.3 MiB/s | - | - | 2.4 MiB/s | 2.6 MiB/s | 2.8 MiB/s |
| Output bytes | 365.0 B | - | - | 365.0 B | 365.0 B | 365.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq @sh expression for this array. | yq rejects the catalog jq @sh expression for this array. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq @sh expression for this array.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 110.9 ms | - | - | 159.4 ms | 199.4 ms | 206.4 ms |
| Wall time dispersion | 0.4 ms / 111.5 ms / 110.6-111.5 ms | - | - | 0.6 ms / 159.9 ms / 157.8-159.9 ms | 1.2 ms / 206.6 ms / 198.2-206.6 ms | 1.6 ms / 209.3 ms / 204.9-209.3 ms |
| First output | 79.1 ms | - | - | 80.1 ms | 121.2 ms | 126.5 ms |
| First output dispersion | 0.0 ms / 79.1 ms / 79.0-79.1 ms | - | - | 1.1 ms / 81.2 ms / 79.0-81.2 ms | 1.1 ms / 124.4 ms / 120.1-124.4 ms | 0.0 ms / 127.5 ms / 126.5-127.5 ms |
| User CPU | 87.2 ms | - | - | 121.4 ms | 158.7 ms | 180.3 ms |
| System CPU | 23.2 ms | - | - | 43.1 ms | 39.7 ms | 31.4 ms |
| Peak RSS | 56.8 MiB | - | - | 73.3 MiB | 117.7 MiB | 97.4 MiB |
| Logical throughput | 97272.5 records/s | - | - | 67709.0 records/s | 54115.9 records/s | 52279.5 records/s |
| Physical throughput | 65.9 MiB/s | - | - | 45.9 MiB/s | 36.6 MiB/s | 41.9 MiB/s |
| Output bytes | 551073.0 B | - | - | 550793.0 B | 550793.0 B | 550793.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq @sh expression for this array. | yq rejects the catalog jq @sh expression for this array. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq @sh expression for this array.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 23.0 ms | - | - | 33.5 ms | 41.1 ms | 41.9 ms |
| Wall time dispersion | 0.2 ms / 23.2 ms / 22.7-23.2 ms | - | - | 0.5 ms / 34.0 ms / 32.7-34.0 ms | 0.6 ms / 41.9 ms / 40.5-41.9 ms | 0.6 ms / 43.0 ms / 41.3-43.0 ms |
| First output | 18.0 ms | - | - | 25.5 ms | 33.9 ms | 34.9 ms |
| First output dispersion | 0.0 ms / 18.0 ms / 18.0-18.0 ms | - | - | 0.2 ms / 26.4 ms / 25.3-26.4 ms | 0.0 ms / 33.9 ms / 32.8-33.9 ms | 1.1 ms / 36.0 ms / 33.8-36.0 ms |
| User CPU | 16.4 ms | - | - | 23.9 ms | 34.6 ms | 33.4 ms |
| System CPU | 6.3 ms | - | - | 9.5 ms | 6.7 ms | 9.3 ms |
| Peak RSS | 13.3 MiB | - | - | 31.0 MiB | 33.5 MiB | 33.4 MiB |
| Logical throughput | 97227.7 records/s | - | - | 66770.3 records/s | 54355.2 records/s | 53337.8 records/s |
| Physical throughput | 65.9 MiB/s | - | - | 45.3 MiB/s | 36.8 MiB/s | 42.8 MiB/s |
| Output bytes | 114996.0 B | - | - | 114950.0 B | 114950.0 B | 114950.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq @sh expression for this array. | yq rejects the catalog jq @sh expression for this array. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
