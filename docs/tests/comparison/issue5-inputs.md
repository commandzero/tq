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

Tools: `jq` (jq-1.8.1), `tq` (tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown)), `yq` (yq (https://github.com/mikefarah/yq/) version v4.53.2)
Environment: `linux` / `x86_64`, 16 logical CPUs, compiler profile `release-benchmark`

Peak RSS is authoritative only after the campaign RSS preflight passes. Captured values show their source when present; missing values are `not captured`, never estimates.

Compare columns with the same input format to isolate tool differences. Missing adapters are marked `not recorded`.

Timing rows show numeric medians followed by compact MAD / p95 / range rows. CPU, throughput, and output cells use the captured summary values.

### issue5-input-sequence

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 100.718 | not measured | not measured | 415.286 | 545.816 | 452.236 |
| Wall dispersion (MAD / p95 / range) | 0.615 / 101.846 / 98.484-101.846 | not measured | not measured | 19.736 / 443.843 / 393.267-443.843 | 1.629 / 579.579 / 506.837-579.579 | 18.543 / 472.048 / 431.205-472.048 |
| First output (ms) | 99.718 | not measured | not measured | 390.478 | 516.556 | 428.296 |
| First output dispersion (MAD / p95 / range) | 1.548 / 101.823 / 64.447-101.823 | not measured | not measured | 10.849 / 436.054 / 378.220-436.054 | 6.599 / 539.231 / 500.529-539.231 | 4.290 / 445.764 / 423.023-445.764 |
| User CPU (ms) | 40.000 | not captured | not captured | 170.000 | 260.000 | 160.000 |
| System CPU (ms) | 10.000 | not captured | not captured | 245.000 | 230.000 | 240.000 |
| Peak RSS (MiB, source) | 46.27 (gnu-time-v) | not measured | not measured | 48.40 (gnu-time-v) | 59.59 (gnu-time-v) | 59.15 (gnu-time-v) |
| Logical throughput (records/s) | 650688.060 | not measured | not measured | 157809.127 | 120069.767 | 144915.326 |
| Physical throughput (MiB/s) | 33.230 | not measured | not measured | 8.059 | 5.788 | 6.572 |
| Output bytes | 6 | not measured | not measured | 6 | 6 | 6 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq inputs expression. | yq rejects the catalog jq inputs expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
