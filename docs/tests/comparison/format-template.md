---
type: Report
title: Templated URI output
description: Measures building and URI-escaping a query-string template per feature.
workload: benchmark.format-template
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

# Templated URI output

## What this measures

The query builds `id=...&place=...` from each feature and applies URI escaping
to the template. It combines interpolation with text formatting.

## Why it matters

Query strings and small links are often assembled at the edge of a data
pipeline. Escaping each interpolated value is part of producing safe output.

## Input and output

The input is a natural feature snapshot. The output is one URI-escaped query
string per feature with `id` and `place` fields, not the source feature.

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

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq format-string expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 3.3 ms | - | - | 8.4 ms | 6.4 ms | 6.9 ms |
| Wall time dispersion | 0.0 ms / 3.5 ms / 3.3-3.5 ms | - | - | 0.3 ms / 8.9 ms / 8.1-8.9 ms | 0.0 ms / 6.5 ms / 6.4-6.5 ms | 0.1 ms / 7.0 ms / 6.7-7.0 ms |
| First output | 3.3 ms | - | - | 8.4 ms | 6.4 ms | 6.9 ms |
| First output dispersion | 0.0 ms / 3.3 ms / 3.3-3.3 ms | - | - | 0.2 ms / 8.6 ms / 8.1-8.6 ms | 0.0 ms / 6.5 ms / 6.4-6.5 ms | 0.1 ms / 7.0 ms / 6.5-7.0 ms |
| User CPU | 2.4 ms | - | - | 7.3 ms | 5.2 ms | 4.9 ms |
| System CPU | 0.8 ms | - | - | 1.0 ms | 1.3 ms | 1.9 ms |
| Peak RSS | 3.3 MiB | - | - | 17.8 MiB | 17.7 MiB | 15.7 MiB |
| Logical throughput | 73032.0 records/s | - | - | 29054.5 records/s | 38142.9 records/s | 35133.2 records/s |
| Physical throughput | 49.8 MiB/s | - | - | 19.8 MiB/s | 26.0 MiB/s | 28.3 MiB/s |
| Output bytes | 15824.0 B | - | - | 15336.0 B | 15336.0 B | 15336.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq format-string expression. | yq rejects the catalog jq format-string expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq format-string expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.3 ms | - | - | 2.0 ms | 2.0 ms | 2.1 ms |
| Wall time dispersion | 0.0 ms / 1.3 ms / 1.2-1.3 ms | - | - | 0.0 ms / 2.2 ms / 2.0-2.2 ms | 0.1 ms / 2.3 ms / 1.9-2.3 ms | 0.1 ms / 2.3 ms / 1.9-2.3 ms |
| First output | 1.3 ms | - | - | 2.0 ms | 2.0 ms | 2.1 ms |
| First output dispersion | 0.0 ms / 1.3 ms / 1.2-1.3 ms | - | - | 0.0 ms / 2.2 ms / 2.0-2.2 ms | 0.1 ms / 2.3 ms / 1.9-2.3 ms | 0.1 ms / 2.3 ms / 1.9-2.3 ms |
| User CPU | 0.0 ms | - | - | 1.9 ms | 0.0 ms | 1.9 ms |
| System CPU | 1.1 ms | - | - | 0.0 ms | 1.9 ms | 0.0 ms |
| Peak RSS | 2.7 MiB | - | - | 16.2 MiB | 15.7 MiB | 16.0 MiB |
| Logical throughput | 5384.6 records/s | - | - | 3576.9 records/s | 3531.8 records/s | 3380.0 records/s |
| Physical throughput | 3.9 MiB/s | - | - | 2.6 MiB/s | 2.6 MiB/s | 2.9 MiB/s |
| Output bytes | 454.0 B | - | - | 440.0 B | 440.0 B | 440.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq format-string expression. | yq rejects the catalog jq format-string expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq format-string expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 113.1 ms | - | - | 276.2 ms | 198.4 ms | 208.2 ms |
| Wall time dispersion | 0.6 ms / 114.1 ms / 112.5-114.1 ms | - | - | 2.8 ms / 283.0 ms / 273.5-283.0 ms | 1.0 ms / 199.4 ms / 191.9-199.4 ms | 0.5 ms / 213.5 ms / 207.8-213.5 ms |
| First output | 78.1 ms | - | - | 217.1 ms | 119.1 ms | 22.3 ms |
| First output dispersion | 0.1 ms / 79.1 ms / 78.0-79.1 ms | - | - | 3.1 ms / 220.2 ms / 213.8-220.2 ms | 3.1 ms / 122.2 ms / 116.0-122.2 ms | 0.0 ms / 24.4 ms / 22.3-24.4 ms |
| User CPU | 87.5 ms | - | - | 271.5 ms | 162.9 ms | 205.2 ms |
| System CPU | 26.1 ms | - | - | 4.0 ms | 35.8 ms | 3.0 ms |
| Peak RSS | 56.6 MiB | - | - | 30.0 MiB | 87.8 MiB | 15.9 MiB |
| Logical throughput | 95405.6 records/s | - | - | 39068.6 records/s | 54386.9 records/s | 51825.1 records/s |
| Physical throughput | 64.6 MiB/s | - | - | 26.5 MiB/s | 36.8 MiB/s | 41.6 MiB/s |
| Output bytes | 696021.0 B | - | - | 674437.0 B | 674437.0 B | 674437.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq format-string expression. | yq rejects the catalog jq format-string expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq format-string expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 23.3 ms | - | - | 58.6 ms | 40.1 ms | 45.4 ms |
| Wall time dispersion | 0.3 ms / 24.8 ms / 23.0-24.8 ms | - | - | 0.1 ms / 62.7 ms / 58.5-62.7 ms | 0.9 ms / 40.9 ms / 39.2-40.9 ms | 0.1 ms / 45.5 ms / 44.7-45.5 ms |
| First output | 18.1 ms | - | - | 51.8 ms | 30.7 ms | 22.3 ms |
| First output dispersion | 0.0 ms / 19.1 ms / 18.1-19.1 ms | - | - | 0.0 ms / 54.9 ms / 51.7-54.9 ms | 0.0 ms / 32.8 ms / 30.7-32.8 ms | 0.1 ms / 23.3 ms / 22.2-23.3 ms |
| User CPU | 18.7 ms | - | - | 57.3 ms | 33.3 ms | 43.4 ms |
| System CPU | 4.4 ms | - | - | 1.0 ms | 8.0 ms | 1.0 ms |
| Peak RSS | 13.3 MiB | - | - | 22.3 MiB | 33.3 MiB | 16.0 MiB |
| Logical throughput | 95871.6 records/s | - | - | 38108.6 records/s | 55776.1 records/s | 49176.7 records/s |
| Physical throughput | 65.0 MiB/s | - | - | 25.8 MiB/s | 37.8 MiB/s | 39.5 MiB/s |
| Output bytes | 145509.0 B | - | - | 141041.0 B | 141041.0 B | 141041.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq format-string expression. | yq rejects the catalog jq format-string expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
