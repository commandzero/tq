---
type: Report
title: Reading additional inputs
description: Measures consuming the current value and following input values.
workload: benchmark.issue5-inputs
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

# Reading additional inputs

## What this measures

The query builds `[., inputs]` and counts the values. It exercises the CLI input
sequence, including the difference between the current value and values read by
`inputs`.

## Why it matters

Filters that combine multiple JSON roots are useful for batch files and stream
processors. They also expose buffering and input-boundary behavior that a
single-document query cannot show.

## Input and output

The input is the medium issue-sequence stream, with one current value followed
by additional values. The output is one integer equal to the number of values
consumed, not the values themselves.

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

### issue5-input-sequence

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `yq rejects the catalog jq inputs expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 69.5 ms | - | - | 405.1 ms | 466.7 ms | 439.7 ms |
| Wall time dispersion | 0.0 ms / 70.5 ms / 69.5-70.5 ms | - | - | 5.4 ms / 411.9 ms / 399.7-411.9 ms | 0.5 ms / 478.1 ms / 466.2-478.1 ms | 0.1 ms / 444.9 ms / 439.5-444.9 ms |
| First output | 67.6 ms | - | - | 405.1 ms | 466.4 ms | 439.7 ms |
| First output dispersion | 0.1 ms / 68.5 ms / 67.5-68.5 ms | - | - | 5.9 ms / 411.0 ms / 399.0-411.0 ms | 1.1 ms / 478.1 ms / 465.3-478.1 ms | 0.5 ms / 444.4 ms / 439.2-444.4 ms |
| User CPU | 47.7 ms | - | - | 139.5 ms | 216.1 ms | 173.8 ms |
| System CPU | 21.3 ms | - | - | 235.0 ms | 215.4 ms | 231.9 ms |
| Peak RSS | 45.1 MiB | - | - | 49.2 MiB | 67.6 MiB | 49.1 MiB |
| Logical throughput | 942991.2 records/s | - | - | 161786.1 records/s | 140415.8 records/s | 149057.6 records/s |
| Physical throughput | 48.2 MiB/s | - | - | 8.3 MiB/s | 6.8 MiB/s | 6.8 MiB/s |
| Output bytes | 6.0 B | - | - | 6.0 B | 6.0 B | 6.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq inputs expression. | yq rejects the catalog jq inputs expression. | none | none | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | 3 measured (1 warmup) | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
