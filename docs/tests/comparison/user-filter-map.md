---
type: Report
title: "User function mapping"
description: "Measures mapping a named user filter across a feature collection."
workload: benchmark.user-filter-map
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# User function mapping

## What this measures

The query defines a no-argument `magnitude` filter and applies it with `map`.
It combines user-function lookup with array construction over all features.

## Why it matters

Named mapping functions are common in reusable transformations. The case tests
the cost and semantics of invoking a filter once for every array member.

## Input and output

The input is a natural snapshot with a feature array. The output is one array
of magnitudes, in feature order, rather than a stream of separate numbers.

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
| Wall (ms) | 23.027 | not measured | not measured | 23.577 | 23.230 | 24.041 |
| Wall dispersion (MAD / p95 / range) | 0.223 / 24.452 / 22.251-24.452 | not measured | not measured | 0.183 / 24.637 / 22.833-24.637 | 0.329 / 24.588 / 22.733-24.588 | 0.184 / 24.331 / 22.459-24.331 |
| First output (ms) | 23.012 | not measured | not measured | 23.563 | 23.212 | 24.029 |
| First output dispersion (MAD / p95 / range) | 0.226 / 24.436 / 22.236-24.436 | not measured | not measured | 0.181 / 24.619 / 22.816-24.619 | 0.329 / 24.562 / 22.718-24.562 | 0.182 / 24.314 / 22.441-24.314 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.36 (gnu-time-v) | not measured | not measured | 9.05 (gnu-time-v) | 9.67 (gnu-time-v) | 9.09 (gnu-time-v) |
| Logical throughput (records/s) | 8424.893 | not measured | not measured | 8228.358 | 8351.450 | 8069.716 |
| Physical throughput (MiB/s) | 5.768 | not measured | not measured | 5.633 | 5.710 | 6.532 |
| Output bytes | 1424 | not measured | not measured | 846 | 846 | 846 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.401 | not measured | not measured | 23.515 | 23.505 | 23.535 |
| Wall dispersion (MAD / p95 / range) | 0.430 / 25.050 / 21.713-26.778 | not measured | not measured | 0.445 / 24.221 / 21.812-24.565 | 0.343 / 24.405 / 22.692-24.931 | 0.347 / 24.210 / 22.213-24.236 |
| First output (ms) | 23.386 | not measured | not measured | 23.500 | 23.490 | 23.520 |
| First output dispersion (MAD / p95 / range) | 0.433 / 25.034 / 21.699-26.734 | not measured | not measured | 0.443 / 24.203 / 21.794-24.547 | 0.344 / 24.391 / 22.673-24.919 | 0.344 / 24.187 / 22.199-24.223 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.06 (gnu-time-v) | not measured | not measured | 8.08 (gnu-time-v) | 8.49 (gnu-time-v) | 8.01 (gnu-time-v) |
| Logical throughput (records/s) | 85.465 | not measured | not measured | 85.052 | 85.090 | 84.978 |
| Physical throughput (MiB/s) | 0.072 | not measured | not measured | 0.072 | 0.072 | 0.083 |
| Output bytes | 19 | not measured | not measured | 15 | 15 | 15 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 137.681 | not measured | not measured | 141.069 | 215.710 | 175.232 |
| Wall dispersion (MAD / p95 / range) | 2.135 / 140.027 / 101.258-140.027 | not measured | not measured | 1.379 / 177.216 / 137.716-177.216 | 1.063 / 217.935 / 208.479-217.935 | 0.983 / 177.624 / 172.219-177.624 |
| First output (ms) | 97.922 | not measured | not measured | 134.833 | 178.264 | 151.714 |
| First output dispersion (MAD / p95 / range) | 1.202 / 101.474 / 96.613-101.474 | not measured | not measured | 0.966 / 136.229 / 133.353-136.229 | 1.659 / 188.059 / 175.846-188.059 | 0.492 / 157.014 / 149.741-157.014 |
| User CPU (ms) | 70.000 | not captured | not captured | 105.000 | 140.000 | 120.000 |
| System CPU (ms) | 20.000 | not captured | not captured | 20.000 | 30.000 | 25.000 |
| Peak RSS (MiB, source) | 60.54 (gnu-time-v) | not measured | not measured | 70.21 (gnu-time-v) | 90.07 (gnu-time-v) | 78.92 (gnu-time-v) |
| Logical throughput (records/s) | 81884.937 | not measured | not measured | 79918.621 | 52264.493 | 64337.564 |
| Physical throughput (MiB/s) | 55.505 | not measured | not measured | 54.172 | 35.377 | 51.653 |
| Output bytes | 84626 | not measured | not measured | 50810 | 50810 | 50810 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.758 | not measured | not measured | 60.667 | 61.949 | 60.366 |
| Wall dispersion (MAD / p95 / range) | 0.416 / 25.134 / 22.720-25.134 | not measured | not measured | 0.837 / 61.906 / 59.611-61.906 | 0.269 / 62.734 / 61.312-62.734 | 0.348 / 61.983 / 59.464-61.983 |
| First output (ms) | 20.985 | not measured | not measured | 27.974 | 35.100 | 31.886 |
| First output dispersion (MAD / p95 / range) | 0.182 / 22.372 / 19.544-22.372 | not measured | not measured | 0.424 / 29.599 / 27.489-29.599 | 0.304 / 36.502 / 34.666-36.502 | 0.252 / 32.631 / 30.259-32.631 |
| User CPU (ms) | 10.000 | not captured | not captured | 20.000 | 20.000 | 20.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 14.73 (gnu-time-v) | not measured | not measured | 20.33 (gnu-time-v) | 24.34 (gnu-time-v) | 21.65 (gnu-time-v) |
| Logical throughput (records/s) | 93652.664 | not measured | not measured | 36675.623 | 35916.641 | 36858.497 |
| Physical throughput (MiB/s) | 63.458 | not measured | not measured | 24.851 | 24.303 | 29.581 |
| Output bytes | 16581 | not measured | not measured | 9911 | 9911 | 9911 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
