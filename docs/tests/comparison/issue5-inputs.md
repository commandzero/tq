---
type: Report
title: "Reading additional inputs"
description: "Measures consuming the current value and following input values."
workload: benchmark.issue5-inputs
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
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
Last updated: 2026-09-11
Profile: `standard` | Status: `observed-failures`

Tools: `jq` (jq-1.8.1), `tq` (tq 0.1.0 (TOON v3; jq target 1.8.x; revision a4b4d916-worktree)), `yq` (yq (https://github.com/mikefarah/yq/) version v4.53.2)
Environment: `linux` / `x86_64`, AMD Ryzen 7 7700 8-Core Processor, 16 logical CPUs, 61.9 GiB RAM; kernel `Linux 7.2.0-ogc4.1.fc44.x86_64 #1 SMP PREEMPT_DYNAMIC Thu Aug 20 16:15:37 UTC 2026`; compiler profile `release-benchmark`

Peak RSS is authoritative only after the campaign RSS preflight passes. Missing or invalid values are `-`, never estimates. RSS collector provenance (outside measurement tables): `linux-wait4`.
Measurement method (outside measurement tables): `tq-bench` native measurement: RSS scope `wait4 child lifetime including pre exec waited descendants and threads`; residual RSS floor `2.5 MiB` retained, not subtracted; observed control excess `1.1 ms`; primary timing is sampler-free.

Compare columns with the same input format to isolate tool differences. Missing adapters have `-` measurement cells and `not recorded` outcome/detail cells.

Timing rows show numeric medians followed by compact MAD / p95 / range rows, with one decimal place and a unit in each measurement cell. CPU, throughput, and output cells use the captured summary values.

### issue5-input-sequence

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `yq rejects the catalog jq inputs expression.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 64.4 ms | - | - | 391.9 ms | 519.7 ms | 436.4 ms |
| Wall time dispersion | 0.6 ms / 66.0 ms / 63.2-66.0 ms | - | - | 8.7 ms / 426.1 ms / 381.6-426.1 ms | 8.4 ms / 538.5 ms / 510.3-538.5 ms | 3.7 ms / 462.3 ms / 427.4-462.3 ms |
| First output | 64.1 ms | - | - | 389.3 ms | 516.5 ms | 432.5 ms |
| First output dispersion | 0.6 ms / 65.7 ms / 62.9-65.7 ms | - | - | 8.5 ms / 423.2 ms / 378.2-423.2 ms | 8.4 ms / 535.4 ms / 507.2-535.4 ms | 3.9 ms / 458.0 ms / 423.4-458.0 ms |
| User CPU | 45.3 ms | - | - | 160.3 ms | 239.1 ms | 155.9 ms |
| System CPU | 19.2 ms | - | - | 257.4 ms | 247.5 ms | 250.5 ms |
| Peak RSS | 46.3 MiB | - | - | 49.1 MiB | 59.0 MiB | 58.1 MiB |
| Logical throughput | 1017742.5 records/s | - | - | 167216.3 records/s | 126095.5 records/s | 150180.2 records/s |
| Physical throughput | 52.0 MiB/s | - | - | 8.5 MiB/s | 6.1 MiB/s | 6.8 MiB/s |
| Output bytes | 6.0 B | - | - | 6.0 B | 6.0 B | 6.0 B |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq inputs expression. | yq rejects the catalog jq inputs expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
