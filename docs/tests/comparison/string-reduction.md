---
type: Report
title: "String reduction"
description: "Measures concatenation of a field collected from every feature."
workload: benchmark.string-reduction
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# String reduction

## What this measures

The query collects non-null place names and combines them with `add`. It
exercises string accumulation over the entire feature collection.

## Why it matters

Reports and exports often assemble text from many records. This case makes
allocation and final-output cost visible when the result is one large string.

## Input and output

The input is a natural snapshot with `properties.place` strings. The output is
one concatenated string in feature order, not a list of place names.

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
| Wall (ms) | 23.662 | 23.607 | 25.532 | 23.642 | 23.316 | 23.424 |
| Wall dispersion (MAD / p95 / range) | 0.455 / 24.510 / 22.860-24.510 | 0.501 / 24.369 / 22.486-24.369 | 0.251 / 26.177 / 24.863-26.177 | 0.261 / 24.361 / 22.870-24.361 | 0.381 / 24.503 / 22.245-24.503 | 0.235 / 24.354 / 22.651-24.354 |
| First output (ms) | 23.646 | 23.593 | 25.358 | 23.625 | 23.300 | 23.409 |
| First output dispersion (MAD / p95 / range) | 0.453 / 24.494 / 22.842-24.494 | 0.501 / 24.353 / 22.472-24.353 | 0.461 / 26.156 / 18.656-26.156 | 0.261 / 24.345 / 22.855-24.345 | 0.378 / 24.478 / 22.230-24.478 | 0.238 / 24.343 / 22.637-24.343 |
| User CPU (ms) | 0.000 | 0.000 | 10.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 10.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.36 (gnu-time-v) | 18.64 (gnu-time-v) | 33.58 (gnu-time-v) | 9.19 (gnu-time-v) | 9.95 (gnu-time-v) | 9.15 (gnu-time-v) |
| Logical throughput (records/s) | 8198.973 | 8218.076 | 7598.308 | 8205.909 | 8320.645 | 8281.927 |
| Physical throughput (MiB/s) | 5.613 | 5.626 | 5.195 | 5.618 | 5.689 | 6.704 |
| Output bytes | 5446 | 5446 | 5446 | 5446 | 5446 | 5446 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.419 | 23.354 | 23.431 | 23.499 | 23.401 | 23.323 |
| Wall dispersion (MAD / p95 / range) | 0.295 / 24.164 / 22.380-24.529 | 0.297 / 24.142 / 22.195-24.346 | 0.240 / 24.164 / 22.213-24.487 | 0.344 / 24.477 / 22.419-24.934 | 0.367 / 24.430 / 21.832-24.828 | 0.320 / 24.046 / 22.047-24.117 |
| First output (ms) | 23.404 | 23.338 | 23.419 | 23.488 | 23.386 | 23.308 |
| First output dispersion (MAD / p95 / range) | 0.296 / 24.154 / 22.362-24.518 | 0.297 / 24.125 / 22.179-24.330 | 0.240 / 24.150 / 22.197-24.472 | 0.348 / 24.459 / 22.403-24.917 | 0.367 / 24.414 / 21.819-24.811 | 0.322 / 24.030 / 22.030-24.101 |
| User CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.06 (gnu-time-v) | 11.89 (gnu-time-v) | 12.02 (gnu-time-v) | 8.23 (gnu-time-v) | 8.57 (gnu-time-v) | 8.07 (gnu-time-v) |
| Logical throughput (records/s) | 85.399 | 85.640 | 85.355 | 85.110 | 85.465 | 85.754 |
| Physical throughput (MiB/s) | 0.072 | 0.072 | 0.072 | 0.072 | 0.072 | 0.083 |
| Output bytes | 55 | 55 | 55 | 55 | 55 | 55 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 137.102 | 336.334 | 945.400 | not measured | not measured | not measured |
| Wall dispersion (MAD / p95 / range) | 1.489 / 140.340 / 134.529-140.340 | 1.004 / 338.949 / 333.064-338.949 | 47.898 / 1001.630 / 882.144-1001.630 | not measured | not measured | not measured |
| First output (ms) | 100.420 | 293.938 | 855.089 | not measured | not measured | not measured |
| First output dispersion (MAD / p95 / range) | 0.994 / 107.542 / 98.326-107.542 | 1.913 / 300.300 / 289.983-300.300 | 32.218 / 898.965 / 805.674-898.965 | not measured | not measured | not measured |
| User CPU (ms) | 80.000 | 350.000 | 1270.000 | not captured | not captured | not captured |
| System CPU (ms) | 20.000 | 120.000 | 420.000 | not captured | not captured | not captured |
| Peak RSS (MiB, source) | 60.86 (gnu-time-v) | 332.57 (gnu-time-v) | 1293.37 (gnu-time-v) | not measured | not measured | not measured |
| Logical throughput (records/s) | 82230.448 | 33520.301 | 11925.105 | not measured | not measured | not measured |
| Physical throughput (MiB/s) | 55.739 | 22.721 | 8.072 | not measured | not measured | not measured |
| Output bytes | 313616 | 313616 | 313616 | not measured | not measured | not measured |
| Outcome | timed | timed | timed | resource-limit | resource-limit | resource-limit |
| Details | none | none | none | process exited with classified error Resource (exit status 5) | process exited with classified error Resource (exit status 5) | process exited with classified error Resource (exit status 5) |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | not timed | not timed | not timed |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 24.023 | 101.612 | 184.258 | not measured | not measured | not measured |
| Wall dispersion (MAD / p95 / range) | 0.295 / 24.340 / 22.587-24.340 | 0.970 / 103.854 / 64.658-103.854 | 1.905 / 222.040 / 181.328-222.040 | not measured | not measured | not measured |
| First output (ms) | 21.538 | 60.874 | 160.083 | not measured | not measured | not measured |
| First output dispersion (MAD / p95 / range) | 0.471 / 22.225 / 20.020-22.225 | 1.204 / 63.087 / 58.293-63.087 | 8.595 / 172.961 / 149.365-172.961 | not measured | not measured | not measured |
| User CPU (ms) | 10.000 | 60.000 | 205.000 | not captured | not captured | not captured |
| System CPU (ms) | 0.000 | 30.000 | 80.000 | not captured | not captured | not captured |
| Peak RSS (MiB, source) | 14.81 (gnu-time-v) | 75.14 (gnu-time-v) | 234.52 (gnu-time-v) | not measured | not measured | not measured |
| Logical throughput (records/s) | 92619.573 | 21896.912 | 12075.459 | not measured | not measured | not measured |
| Physical throughput (MiB/s) | 62.758 | 14.837 | 8.171 | not measured | not measured | not measured |
| Output bytes | 61822 | 61822 | 61822 | not measured | not measured | not measured |
| Outcome | timed | timed | timed | resource-limit | resource-limit | resource-limit |
| Details | none | none | none | process exited with classified error Resource (exit status 5) | process exited with classified error Resource (exit status 5) | process exited with classified error Resource (exit status 5) |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | not timed | not timed | not timed |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
