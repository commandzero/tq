---
type: Report
title: "Bounded recursion"
description: "Measures recursive traversal with a fixed output limit."
workload: benchmark.recurse-bounded
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Bounded recursion

## What this measures

The query recursively visits child values with `recurse(.[]?)` and keeps at
most 2,048 results. It tests traversal while putting a clear bound on work and
output.

## Why it matters

Recursive queries are useful for irregular documents, but an unbounded walk can
run away on large or cyclic-looking data. A fixed limit is a practical guard.

## Input and output

The input is a natural nested snapshot. The output is the depth-first sequence
of visited values up to the limit, including containers and scalars as the
recursion produces them.

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
| Wall (ms) | 23.509 | not measured | not measured | 23.477 | 23.316 | 23.163 |
| Wall dispersion (MAD / p95 / range) | 0.339 / 25.111 / 22.967-25.111 | not measured | not measured | 0.740 / 24.694 / 22.659-24.694 | 0.370 / 24.293 / 22.720-24.293 | 0.582 / 24.686 / 21.798-24.686 |
| First output (ms) | 23.497 | not measured | not measured | 13.059 | 12.899 | 12.646 |
| First output dispersion (MAD / p95 / range) | 0.339 / 25.096 / 22.952-25.096 | not measured | not measured | 0.445 / 14.565 / 12.524-14.565 | 0.268 / 13.576 / 12.349-13.576 | 0.461 / 13.979 / 12.098-13.979 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 10.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 10.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.38 (gnu-time-v) | not measured | not measured | 9.21 (gnu-time-v) | 9.87 (gnu-time-v) | 9.31 (gnu-time-v) |
| Logical throughput (records/s) | 8252.159 | not measured | not measured | 8263.583 | 8320.645 | 8375.607 |
| Physical throughput (MiB/s) | 5.650 | not measured | not measured | 5.657 | 5.689 | 6.780 |
| Output bytes | 536652 | not measured | not measured | 433312 | 433312 | 433312 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq recurse expression. | yq rejects the catalog jq recurse expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.343 | not measured | not measured | 23.314 | 23.448 | 23.236 |
| Wall dispersion (MAD / p95 / range) | 0.380 / 23.955 / 21.848-24.192 | not measured | not measured | 0.302 / 24.174 / 22.309-24.699 | 0.316 / 24.109 / 22.309-24.488 | 0.367 / 24.518 / 22.381-24.612 |
| First output (ms) | 23.328 | not measured | not measured | 23.302 | 23.434 | 23.223 |
| First output dispersion (MAD / p95 / range) | 0.381 / 23.941 / 21.831-24.174 | not measured | not measured | 0.302 / 24.161 / 22.294-24.688 | 0.318 / 24.094 / 22.294-24.473 | 0.367 / 24.508 / 22.366-24.593 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.06 (gnu-time-v) | not measured | not measured | 8.27 (gnu-time-v) | 8.45 (gnu-time-v) | 8.04 (gnu-time-v) |
| Logical throughput (records/s) | 85.679 | not measured | not measured | 85.785 | 85.297 | 86.073 |
| Physical throughput (MiB/s) | 0.073 | not measured | not measured | 0.073 | 0.072 | 0.084 |
| Output bytes | 9719 | not measured | not measured | 7938 | 7938 | 7938 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq recurse expression. | yq rejects the catalog jq recurse expression. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 293.298 | not measured | not measured | 250.351 | 288.916 | 252.353 |
| Wall dispersion (MAD / p95 / range) | 3.511 / 325.461 / 289.043-325.461 | not measured | not measured | 1.915 / 258.059 / 245.927-258.059 | 2.245 / 297.136 / 285.836-297.136 | 2.155 / 255.787 / 247.728-255.787 |
| First output (ms) | 80.579 | not measured | not measured | 104.746 | 147.454 | 120.263 |
| First output dispersion (MAD / p95 / range) | 0.524 / 81.168 / 78.820-81.168 | not measured | not measured | 0.509 / 105.855 / 104.032-105.855 | 0.744 / 148.476 / 144.816-148.476 | 0.869 / 121.879 / 118.860-121.879 |
| User CPU (ms) | 240.000 | not captured | not captured | 190.000 | 220.000 | 200.000 |
| System CPU (ms) | 30.000 | not captured | not captured | 30.000 | 40.000 | 30.000 |
| Peak RSS (MiB, source) | 64.86 (gnu-time-v) | not measured | not measured | 69.47 (gnu-time-v) | 89.36 (gnu-time-v) | 78.05 (gnu-time-v) |
| Logical throughput (records/s) | 38438.721 | not measured | not measured | 45032.684 | 39021.723 | 44675.426 |
| Physical throughput (MiB/s) | 26.055 | not measured | not measured | 30.525 | 26.413 | 35.868 |
| Output bytes | 23752426 | not measured | not measured | 19085974 | 19085974 | 19085974 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq recurse expression. | yq rejects the catalog jq recurse expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 62.600 | not measured | not measured | 62.188 | 65.042 | 63.059 |
| Wall dispersion (MAD / p95 / range) | 0.394 / 63.168 / 61.736-63.168 | not measured | not measured | 0.718 / 64.104 / 59.768-64.104 | 1.893 / 100.736 / 62.900-100.736 | 0.452 / 98.864 / 61.631-98.864 |
| First output (ms) | 18.120 | not measured | not measured | 23.686 | 30.807 | 26.721 |
| First output dispersion (MAD / p95 / range) | 0.408 / 19.440 / 16.832-19.440 | not measured | not measured | 0.819 / 24.645 / 22.324-24.645 | 0.495 / 32.046 / 30.165-32.046 | 0.296 / 27.873 / 25.914-27.873 |
| User CPU (ms) | 50.000 | not captured | not captured | 40.000 | 40.000 | 40.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 10.000 | 10.000 | 10.000 |
| Peak RSS (MiB, source) | 15.48 (gnu-time-v) | not measured | not measured | 20.28 (gnu-time-v) | 24.34 (gnu-time-v) | 21.84 (gnu-time-v) |
| Logical throughput (records/s) | 35543.131 | not measured | not measured | 35778.319 | 34208.665 | 35284.136 |
| Physical throughput (MiB/s) | 24.084 | not measured | not measured | 24.243 | 23.147 | 28.317 |
| Output bytes | 4788780 | not measured | not measured | 3848784 | 3848784 | 3848784 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq recurse expression. | yq rejects the catalog jq recurse expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
