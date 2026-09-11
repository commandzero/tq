---
type: Report
title: "User function selection"
description: "Measures filtering with a named predicate and returning matching IDs."
workload: benchmark.user-filter-select
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# User function selection

## What this measures

The query defines `significant` as a magnitude predicate, applies it inside
`select`, and collects the matching IDs. It tests callback invocation and
conditional selection in one blocking result.

## Why it matters

Teams often turn business rules into named filters. This case checks that the
rule remains usable inside a collection operation instead of only inline.

## Input and output

The input is a natural feature snapshot. The output is one array of IDs whose
magnitudes meet the threshold. It does not return the full selected features.

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
| Wall (ms) | 23.366 | not measured | not measured | 23.415 | 23.420 | 23.180 |
| Wall dispersion (MAD / p95 / range) | 0.368 / 24.273 / 22.358-24.273 | not measured | not measured | 0.336 / 24.513 / 22.747-24.513 | 0.353 / 24.251 / 22.141-24.251 | 0.508 / 24.046 / 22.550-24.046 |
| First output (ms) | 23.352 | not measured | not measured | 23.401 | 23.406 | 23.165 |
| First output dispersion (MAD / p95 / range) | 0.365 / 24.260 / 22.343-24.260 | not measured | not measured | 0.335 / 24.498 / 22.732-24.498 | 0.357 / 24.235 / 22.125-24.235 | 0.508 / 24.031 / 22.532-24.031 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.36 (gnu-time-v) | not measured | not measured | 9.24 (gnu-time-v) | 9.83 (gnu-time-v) | 9.23 (gnu-time-v) |
| Logical throughput (records/s) | 8302.662 | not measured | not measured | 8285.287 | 8283.518 | 8369.103 |
| Physical throughput (MiB/s) | 5.684 | not measured | not measured | 5.672 | 5.663 | 6.775 |
| Output bytes | 1119 | not measured | not measured | 792 | 792 | 792 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.320 | not measured | not measured | 23.201 | 23.348 | 23.169 |
| Wall dispersion (MAD / p95 / range) | 0.405 / 23.994 / 22.304-24.002 | not measured | not measured | 0.373 / 24.210 / 22.260-24.581 | 0.427 / 24.468 / 22.405-25.071 | 0.501 / 24.096 / 21.729-24.107 |
| First output (ms) | 23.305 | not measured | not measured | 23.185 | 23.332 | 23.155 |
| First output dispersion (MAD / p95 / range) | 0.403 / 23.981 / 22.289-23.987 | not measured | not measured | 0.376 / 24.199 / 22.243-24.565 | 0.430 / 24.453 / 22.393-25.051 | 0.503 / 24.081 / 21.714-24.092 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.06 (gnu-time-v) | not measured | not measured | 8.15 (gnu-time-v) | 8.41 (gnu-time-v) | 8.11 (gnu-time-v) |
| Logical throughput (records/s) | 85.765 | not measured | not measured | 86.203 | 85.660 | 86.320 |
| Physical throughput (MiB/s) | 0.073 | not measured | not measured | 0.073 | 0.073 | 0.084 |
| Output bytes | 3 | not measured | not measured | 5 | 5 | 5 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 137.483 | not measured | not measured | 175.120 | 215.022 | 176.126 |
| Wall dispersion (MAD / p95 / range) | 0.819 / 139.012 / 103.131-139.012 | not measured | not measured | 1.034 / 177.876 / 172.991-177.876 | 0.984 / 218.605 / 213.060-218.605 | 0.825 / 177.125 / 172.280-177.125 |
| First output (ms) | 100.660 | not measured | not measured | 144.387 | 185.923 | 161.276 |
| First output dispersion (MAD / p95 / range) | 0.586 / 102.075 / 98.205-102.075 | not measured | not measured | 1.678 / 146.824 / 141.919-146.824 | 1.742 / 191.923 / 183.638-191.923 | 1.057 / 164.264 / 159.038-164.264 |
| User CPU (ms) | 70.000 | not captured | not captured | 120.000 | 150.000 | 130.000 |
| System CPU (ms) | 20.000 | not captured | not captured | 20.000 | 30.000 | 20.000 |
| Peak RSS (MiB, source) | 60.33 (gnu-time-v) | not measured | not measured | 69.60 (gnu-time-v) | 89.80 (gnu-time-v) | 78.38 (gnu-time-v) |
| Logical throughput (records/s) | 82002.866 | not measured | not measured | 64378.528 | 52431.844 | 64010.810 |
| Physical throughput (MiB/s) | 55.585 | not measured | not measured | 43.638 | 35.490 | 51.391 |
| Output bytes | 57877 | not measured | not measured | 40962 | 40962 | 40962 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.820 | not measured | not measured | 61.137 | 61.739 | 60.849 |
| Wall dispersion (MAD / p95 / range) | 0.246 / 24.346 / 23.023-24.346 | not measured | not measured | 0.390 / 62.791 / 59.951-62.791 | 0.965 / 64.651 / 59.693-64.651 | 0.586 / 62.510 / 59.477-62.510 |
| First output (ms) | 20.938 | not measured | not measured | 30.451 | 38.462 | 33.380 |
| First output dispersion (MAD / p95 / range) | 0.662 / 23.595 / 19.973-23.595 | not measured | not measured | 0.730 / 31.473 / 29.363-31.473 | 0.417 / 38.917 / 36.683-38.917 | 0.652 / 35.050 / 31.105-35.050 |
| User CPU (ms) | 10.000 | not captured | not captured | 20.000 | 30.000 | 20.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 14.71 (gnu-time-v) | not measured | not measured | 20.32 (gnu-time-v) | 24.28 (gnu-time-v) | 21.77 (gnu-time-v) |
| Logical throughput (records/s) | 93408.900 | not measured | not measured | 36393.673 | 36039.100 | 36566.226 |
| Physical throughput (MiB/s) | 63.293 | not measured | not measured | 24.660 | 24.385 | 29.346 |
| Output bytes | 11930 | not measured | not measured | 8514 | 8514 | 8514 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
