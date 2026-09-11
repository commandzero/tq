---
type: Report
title: "Startup identity"
description: "Measures the fixed cost of launching a query that returns its input."
workload: benchmark.startup
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Startup identity

## What this measures

The query is `.` on a small synthetic helper input. It measures the startup
path and an identity query, including the cost of launching a short-lived
command rather than processing a large document.

## Why it matters

Many scripts invoke a JSON tool for one small value. Startup cost can dominate
that kind of use, so this is a useful baseline for CLI latency.

## Input and output

The input is one small JSON value. The output is the same semantic value and
object order. No filtering or transformation is part of this workload.

## Results

<!-- benchmark-results:start -->
Last updated: 2026-09-11
Profile: `smoke` | Status: `passed`

Tools: `jq` (jq-1.8.1), `tq` (tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown)), `yq` (yq (https://github.com/mikefarah/yq/) version v4.53.2)
Environment: `linux` / `x86_64`, 16 logical CPUs, compiler profile `release-benchmark`

Peak RSS is authoritative only after the campaign RSS preflight passes. Captured values show their source when present; missing values are `not captured`, never estimates.

Compare columns with the same input format to isolate tool differences. Missing adapters are marked `not recorded`.

Timing rows show numeric medians followed by compact MAD / p95 / range rows. CPU, throughput, and output cells use the captured summary values.

### startup

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 22.956 | 23.326 | 23.427 | 23.259 | 23.294 | 23.047 |
| Wall dispersion (MAD / p95 / range) | 0.314 / 24.223 / 22.050-24.930 | 0.436 / 24.480 / 21.886-24.525 | 0.350 / 24.161 / 22.478-24.409 | 0.468 / 24.776 / 22.004-25.143 | 0.328 / 24.314 / 22.024-25.023 | 0.446 / 24.528 / 21.676-24.534 |
| First output (ms) | 22.941 | 23.313 | 23.411 | 23.244 | 23.282 | 23.031 |
| First output dispersion (MAD / p95 / range) | 0.314 / 24.209 / 22.035-24.914 | 0.438 / 24.469 / 21.869-24.505 | 0.352 / 24.144 / 22.463-24.398 | 0.468 / 24.762 / 21.982-25.131 | 0.326 / 24.300 / 22.010-25.011 | 0.445 / 24.514 / 21.661-24.521 |
| User CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.08 (gnu-time-v) | 10.77 (gnu-time-v) | 11.14 (gnu-time-v) | 7.02 (gnu-time-v) | 7.31 (gnu-time-v) | 7.01 (gnu-time-v) |
| Logical throughput (records/s) | 43.562 | 42.870 | 42.687 | 42.993 | 42.930 | 43.390 |
| Physical throughput (MiB/s) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Output bytes | 5 | 5 | 5 | 5 | 5 | 5 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
