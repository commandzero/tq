---
type: Report
title: HTML and URI escaping
description: Measures two text-escaping passes over feature place names.
workload: benchmark.format-uri-html
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

# HTML and URI escaping

## What this measures

The query takes each place string through `@html` and then `@uri`. It exercises
escaping rules and repeated text allocation for every feature.

## Why it matters

Data often crosses an HTML or URL boundary after it leaves a JSON document.
Applying the transformations in sequence is common in report and request
generation.

## Input and output

The input is a natural feature snapshot. The output is one URI-encoded string
per place after HTML escaping, not the original place text or full feature.

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

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq @html/@uri expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 3.3 ms | - | - | 8.4 ms | 6.4 ms | 6.4 ms |
| Wall time dispersion | 0.0 ms / 3.3 ms / 3.2-3.3 ms | - | - | 0.2 ms / 8.6 ms / 8.2-8.6 ms | 0.1 ms / 6.8 ms / 6.3-6.8 ms | 0.0 ms / 6.7 ms / 6.4-6.7 ms |
| First output | 3.3 ms | - | - | 8.4 ms | 6.4 ms | 6.4 ms |
| First output dispersion | 0.0 ms / 3.3 ms / 3.2-3.3 ms | - | - | 0.2 ms / 8.6 ms / 8.2-8.6 ms | 0.1 ms / 6.5 ms / 6.3-6.5 ms | 0.0 ms / 6.5 ms / 6.4-6.5 ms |
| User CPU | 3.1 ms | - | - | 7.2 ms | 2.1 ms | 5.5 ms |
| System CPU | 0.0 ms | - | - | 1.0 ms | 4.1 ms | 1.0 ms |
| Peak RSS | 3.3 MiB | - | - | 18.3 MiB | 19.6 MiB | 16.0 MiB |
| Logical throughput | 73917.0 records/s | - | - | 29009.6 records/s | 38202.6 records/s | 38083.3 records/s |
| Physical throughput | 50.4 MiB/s | - | - | 19.8 MiB/s | 26.0 MiB/s | 30.7 MiB/s |
| Output bytes | 10713.0 B | - | - | 10225.0 B | 10225.0 B | 10225.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq @html/@uri expression. | yq rejects the catalog jq @html/@uri expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq @html/@uri expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.2 ms | - | - | 2.1 ms | 2.1 ms | 2.0 ms |
| Wall time dispersion | 0.0 ms / 1.5 ms / 1.2-1.5 ms | - | - | 0.0 ms / 2.1 ms / 1.8-2.1 ms | 0.0 ms / 2.1 ms / 2.1-2.1 ms | 0.1 ms / 2.1 ms / 1.9-2.1 ms |
| First output | 1.2 ms | - | - | 2.1 ms | 2.1 ms | 2.0 ms |
| First output dispersion | 0.0 ms / 1.5 ms / 1.2-1.5 ms | - | - | 0.0 ms / 2.1 ms / 1.8-2.1 ms | 0.0 ms / 2.1 ms / 2.1-2.1 ms | 0.1 ms / 2.1 ms / 1.9-2.1 ms |
| User CPU | 0.0 ms | - | - | 0.9 ms | 0.0 ms | 0.9 ms |
| System CPU | 1.1 ms | - | - | 1.0 ms | 1.9 ms | 0.9 ms |
| Peak RSS | 2.7 MiB | - | - | 16.2 MiB | 15.7 MiB | 15.9 MiB |
| Logical throughput | 5942.3 records/s | - | - | 3389.8 records/s | 3388.2 records/s | 3582.4 records/s |
| Physical throughput | 4.3 MiB/s | - | - | 2.5 MiB/s | 2.5 MiB/s | 3.0 MiB/s |
| Output bytes | 310.0 B | - | - | 296.0 B | 296.0 B | 296.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq @html/@uri expression. | yq rejects the catalog jq @html/@uri expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq @html/@uri expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 109.6 ms | - | - | 274.1 ms | 193.9 ms | 207.1 ms |
| Wall time dispersion | 0.3 ms / 110.0 ms / 109.3-110.0 ms | - | - | 0.7 ms / 276.7 ms / 273.4-276.7 ms | 2.0 ms / 195.9 ms / 188.8-195.9 ms | 0.9 ms / 208.0 ms / 205.5-208.0 ms |
| First output | 79.1 ms | - | - | 219.0 ms | 122.4 ms | 32.8 ms |
| First output dispersion | 0.0 ms / 79.1 ms / 79.1-79.1 ms | - | - | 0.0 ms / 222.3 ms / 219.0-222.3 ms | 0.1 ms / 125.4 ms / 122.3-125.4 ms | 0.0 ms / 32.8 ms / 32.8-32.8 ms |
| User CPU | 85.3 ms | - | - | 270.2 ms | 165.4 ms | 203.2 ms |
| System CPU | 23.8 ms | - | - | 3.0 ms | 26.3 ms | 3.0 ms |
| Peak RSS | 56.6 MiB | - | - | 30.3 MiB | 117.5 MiB | 15.9 MiB |
| Logical throughput | 98510.3 records/s | - | - | 39373.2 records/s | 55668.5 records/s | 52114.1 records/s |
| Physical throughput | 66.7 MiB/s | - | - | 26.7 MiB/s | 37.7 MiB/s | 41.8 MiB/s |
| Output bytes | 471249.0 B | - | - | 449667.0 B | 449667.0 B | 449667.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq @html/@uri expression. | yq rejects the catalog jq @html/@uri expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq @html/@uri expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 23.2 ms | - | - | 58.2 ms | 40.0 ms | 45.0 ms |
| Wall time dispersion | 0.6 ms / 23.8 ms / 22.5-23.8 ms | - | - | 0.5 ms / 59.0 ms / 57.7-59.0 ms | 0.3 ms / 40.3 ms / 39.5-40.3 ms | 0.6 ms / 47.5 ms / 44.5-47.5 ms |
| First output | 18.0 ms | - | - | 53.8 ms | 34.9 ms | 32.5 ms |
| First output dispersion | 0.0 ms / 18.1 ms / 18.0-18.1 ms | - | - | 0.0 ms / 54.9 ms / 53.8-54.9 ms | 0.0 ms / 34.9 ms / 34.9-34.9 ms | 0.7 ms / 33.8 ms / 31.8-33.8 ms |
| User CPU | 16.4 ms | - | - | 56.7 ms | 28.8 ms | 44.2 ms |
| System CPU | 6.7 ms | - | - | 1.0 ms | 11.3 ms | 1.0 ms |
| Peak RSS | 13.4 MiB | - | - | 22.0 MiB | 33.5 MiB | 16.2 MiB |
| Logical throughput | 96230.9 records/s | - | - | 38415.2 records/s | 55862.6 records/s | 49625.7 records/s |
| Physical throughput | 65.3 MiB/s | - | - | 26.1 MiB/s | 37.8 MiB/s | 39.9 MiB/s |
| Output bytes | 99055.0 B | - | - | 94589.0 B | 94589.0 B | 94589.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq @html/@uri expression. | yq rejects the catalog jq @html/@uri expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
