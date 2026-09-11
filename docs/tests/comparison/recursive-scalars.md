---
type: Report
title: "Recursive scalar traversal"
description: "Measures depth-first traversal that emits every scalar value."
workload: benchmark.recursive-scalars
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Recursive scalar traversal

## What this measures

The query `.. | scalars` walks nested arrays and objects, then emits each
number, string, boolean, or null in traversal order. It combines recursion with
scalar filtering.

## Why it matters

Schema discovery, indexing, and redaction tools often need to inspect every
leaf in an unknown document. Depth and nesting affect the amount of work.

## Input and output

The input is a nested natural snapshot. The output is a depth-first sequence of
scalar leaves, not the containers that held them and not one flattened array.

## Results

<!-- benchmark-results:start -->
Last updated: 2026-09-11
Profile: `standard` | Status: `observed-failures`

Tools: `jq` (jq-1.8.1), `tq` (tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown)), `yq` (yq (https://github.com/mikefarah/yq/) version v4.53.2)
Environment: `linux` / `x86_64`, 16 logical CPUs, compiler profile `release-benchmark`

Peak RSS is authoritative only after the campaign RSS preflight passes. Captured values show their source when present; missing values are `not captured`, never estimates.

Compare columns with the same input format to isolate tool differences. Missing adapters are marked `not recorded`.

Timing rows show numeric medians followed by compact MAD / p95 / range rows. CPU, throughput, and output cells use the captured summary values.

### usgs-all-day

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.262 | not measured | not measured | 60.046 | 59.825 | 60.044 |
| Wall dispersion (MAD / p95 / range) | 0.206 / 23.918 / 22.014-23.918 | not measured | not measured | 0.756 / 62.640 / 58.886-62.640 | 0.487 / 60.394 / 58.798-60.394 | 0.306 / 60.955 / 59.222-60.955 |
| First output (ms) | 23.247 | not measured | not measured | 30.452 | 31.035 | 30.869 |
| First output dispersion (MAD / p95 / range) | 0.205 / 23.902 / 21.999-23.902 | not measured | not measured | 0.237 / 30.818 / 29.085-30.818 | 0.726 / 32.142 / 30.155-32.142 | 0.536 / 32.433 / 30.128-32.433 |
| User CPU (ms) | 0.000 | not captured | not captured | 10.000 | 10.000 | 10.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 20.000 | 20.000 | 20.000 |
| Peak RSS (MiB, source) | 4.49 (gnu-time-v) | not measured | not measured | 8.96 (gnu-time-v) | 9.63 (gnu-time-v) | 8.85 (gnu-time-v) |
| Logical throughput (records/s) | 8339.602 | not measured | not measured | 3230.856 | 3242.791 | 3230.964 |
| Physical throughput (MiB/s) | 5.709 | not measured | not measured | 2.212 | 2.217 | 2.615 |
| Output bytes | 89511 | not measured | not measured | 86587 | 86587 | 86587 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq recursive scalar expression. | yq rejects the catalog jq recursive scalar expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.150 | not measured | not measured | 23.402 | 23.648 | 23.718 |
| Wall dispersion (MAD / p95 / range) | 0.446 / 24.353 / 22.167-24.904 | not measured | not measured | 0.440 / 24.517 / 22.702-24.591 | 0.317 / 24.505 / 22.223-24.541 | 0.347 / 24.464 / 22.319-25.062 |
| First output (ms) | 23.134 | not measured | not measured | 23.387 | 23.634 | 23.702 |
| First output dispersion (MAD / p95 / range) | 0.446 / 24.340 / 22.152-24.888 | not measured | not measured | 0.439 / 24.502 / 22.687-24.578 | 0.318 / 24.487 / 22.205-24.526 | 0.347 / 24.451 / 22.301-25.045 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.06 (gnu-time-v) | not measured | not measured | 8.15 (gnu-time-v) | 8.35 (gnu-time-v) | 7.83 (gnu-time-v) |
| Logical throughput (records/s) | 86.393 | not measured | not measured | 85.461 | 84.576 | 84.324 |
| Physical throughput (MiB/s) | 0.073 | not measured | not measured | 0.072 | 0.072 | 0.082 |
| Output bytes | 1172 | not measured | not measured | 1140 | 1140 | 1140 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq recursive scalar expression. | yq rejects the catalog jq recursive scalar expression. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 360.748 | not measured | not measured | 2278.141 | 2316.437 | 2322.709 |
| Wall dispersion (MAD / p95 / range) | 1.756 / 363.700 / 355.846-363.700 | not measured | not measured | 6.436 / 2307.374 / 2247.478-2307.374 | 18.241 / 2363.694 / 2277.490-2363.694 | 35.184 / 2396.252 / 2253.265-2396.252 |
| First output (ms) | 80.743 | not measured | not measured | 133.486 | 174.327 | 147.829 |
| First output dispersion (MAD / p95 / range) | 0.373 / 82.037 / 80.001-82.037 | not measured | not measured | 0.682 / 136.524 / 131.676-136.524 | 0.727 / 176.893 / 172.029-176.893 | 0.871 / 151.377 / 145.042-151.377 |
| User CPU (ms) | 305.000 | not captured | not captured | 875.000 | 935.000 | 880.000 |
| System CPU (ms) | 25.000 | not captured | not captured | 1240.000 | 1220.000 | 1275.000 |
| Peak RSS (MiB, source) | 64.86 (gnu-time-v) | not measured | not measured | 69.30 (gnu-time-v) | 89.25 (gnu-time-v) | 78.18 (gnu-time-v) |
| Logical throughput (records/s) | 31251.733 | not measured | not measured | 4948.772 | 4866.957 | 4853.814 |
| Physical throughput (MiB/s) | 21.184 | not measured | not measured | 3.354 | 3.294 | 3.897 |
| Output bytes | 5126950 | not measured | not measured | 4958748 | 4958748 | 4958748 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq recursive scalar expression. | yq rejects the catalog jq recursive scalar expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 97.466 | not measured | not measured | 465.808 | 469.067 | 468.009 |
| Wall dispersion (MAD / p95 / range) | 0.322 / 98.510 / 95.900-98.510 | not measured | not measured | 2.800 / 472.686 / 431.517-472.686 | 2.811 / 478.556 / 465.853-478.556 | 0.677 / 470.741 / 464.891-470.741 |
| First output (ms) | 18.345 | not measured | not measured | 50.072 | 57.502 | 52.188 |
| First output dispersion (MAD / p95 / range) | 0.372 / 20.093 / 17.389-20.093 | not measured | not measured | 0.481 / 52.339 / 48.214-52.339 | 0.929 / 60.325 / 55.510-60.325 | 0.644 / 56.286 / 51.145-56.286 |
| User CPU (ms) | 60.000 | not captured | not captured | 175.000 | 185.000 | 170.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 245.000 | 230.000 | 240.000 |
| Peak RSS (MiB, source) | 15.36 (gnu-time-v) | not measured | not measured | 19.99 (gnu-time-v) | 24.32 (gnu-time-v) | 21.53 (gnu-time-v) |
| Logical throughput (records/s) | 22828.474 | not measured | not measured | 4776.651 | 4743.454 | 4754.182 |
| Physical throughput (MiB/s) | 15.468 | not measured | not measured | 3.237 | 3.210 | 3.815 |
| Output bytes | 1011185 | not measured | not measured | 977853 | 977853 | 977853 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq recursive scalar expression. | yq rejects the catalog jq recursive scalar expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
