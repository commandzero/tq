---
type: Report
title: "User function sorting"
description: "Measures sorting with a named key filter and projecting IDs."
workload: benchmark.user-filter-sort-by
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# User function sorting

## What this measures

The query defines `magnitude`, uses it as the `sort_by` key, and maps the
ordered features to IDs. It exercises a user filter in a sorting callback.

## Why it matters

Sorting by a reusable key function is a practical pattern for reports and
ranking. It also combines callback overhead with a blocking operation.

## Input and output

The input is a natural feature snapshot. The output is one array of IDs ordered
by each feature's magnitude, not the sorted feature objects themselves.

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
| Wall (ms) | 23.512 | not measured | not measured | 23.495 | 23.482 | 23.181 |
| Wall dispersion (MAD / p95 / range) | 0.245 / 24.288 / 23.183-24.288 | not measured | not measured | 0.633 / 24.998 / 22.730-24.998 | 0.351 / 24.449 / 22.652-24.449 | 0.282 / 24.700 / 22.620-24.700 |
| First output (ms) | 23.497 | not measured | not measured | 23.479 | 23.468 | 23.166 |
| First output dispersion (MAD / p95 / range) | 0.244 / 24.271 / 23.168-24.271 | not measured | not measured | 0.633 / 24.981 / 22.714-24.981 | 0.352 / 24.434 / 22.637-24.434 | 0.282 / 24.631 / 22.609-24.631 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.42 (gnu-time-v) | not measured | not measured | 9.32 (gnu-time-v) | 9.73 (gnu-time-v) | 9.22 (gnu-time-v) |
| Logical throughput (records/s) | 8250.930 | not measured | not measured | 8257.076 | 8261.823 | 8368.742 |
| Physical throughput (MiB/s) | 5.649 | not measured | not measured | 5.653 | 5.648 | 6.774 |
| Output bytes | 3291 | not measured | not measured | 2325 | 2325 | 2325 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.354 | not measured | not measured | 23.398 | 23.546 | 23.302 |
| Wall dispersion (MAD / p95 / range) | 0.431 / 24.645 / 22.013-25.586 | not measured | not measured | 0.188 / 24.012 / 22.440-24.205 | 0.408 / 24.350 / 21.509-24.570 | 0.288 / 24.318 / 22.524-24.471 |
| First output (ms) | 23.336 | not measured | not measured | 23.386 | 23.530 | 23.290 |
| First output dispersion (MAD / p95 / range) | 0.428 / 24.633 / 21.998-25.575 | not measured | not measured | 0.195 / 24.000 / 22.428-24.190 | 0.406 / 24.338 / 21.492-24.559 | 0.287 / 24.307 / 22.506-24.460 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.07 (gnu-time-v) | not measured | not measured | 8.19 (gnu-time-v) | 8.57 (gnu-time-v) | 8.00 (gnu-time-v) |
| Logical throughput (records/s) | 85.638 | not measured | not measured | 85.479 | 84.942 | 85.830 |
| Physical throughput (MiB/s) | 0.072 | not measured | not measured | 0.072 | 0.072 | 0.083 |
| Output bytes | 35 | not measured | not measured | 27 | 27 | 27 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 138.519 | not measured | not measured | 176.911 | 215.619 | 180.022 |
| Wall dispersion (MAD / p95 / range) | 1.438 / 140.750 / 135.639-140.750 | not measured | not measured | 0.537 / 178.426 / 175.303-178.426 | 0.964 / 217.419 / 213.826-217.419 | 2.511 / 215.354 / 177.410-215.354 |
| First output (ms) | 113.954 | not measured | not measured | 138.413 | 182.357 | 154.863 |
| First output dispersion (MAD / p95 / range) | 1.230 / 116.413 / 112.167-116.413 | not measured | not measured | 0.953 / 141.946 / 136.759-141.946 | 0.921 / 184.324 / 180.198-184.324 | 0.812 / 157.161 / 151.529-157.161 |
| User CPU (ms) | 90.000 | not captured | not captured | 130.000 | 160.000 | 140.000 |
| System CPU (ms) | 20.000 | not captured | not captured | 20.000 | 30.000 | 25.000 |
| Peak RSS (MiB, source) | 61.42 (gnu-time-v) | not measured | not measured | 72.45 (gnu-time-v) | 92.93 (gnu-time-v) | 81.25 (gnu-time-v) |
| Logical throughput (records/s) | 81389.557 | not measured | not measured | 63727.139 | 52286.672 | 62625.853 |
| Physical throughput (MiB/s) | 55.169 | not measured | not measured | 43.197 | 35.392 | 50.279 |
| Output bytes | 189277 | not measured | not measured | 132913 | 132913 | 132913 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 24.532 | not measured | not measured | 60.857 | 62.178 | 61.573 |
| Wall dispersion (MAD / p95 / range) | 0.828 / 60.410 / 23.590-60.410 | not measured | not measured | 0.386 / 62.299 / 60.103-62.299 | 0.722 / 63.174 / 60.750-63.174 | 0.289 / 61.934 / 60.231-61.934 |
| First output (ms) | 23.456 | not measured | not measured | 31.585 | 38.611 | 35.886 |
| First output dispersion (MAD / p95 / range) | 0.888 / 60.396 / 21.996-60.396 | not measured | not measured | 0.401 / 35.397 / 30.278-35.397 | 0.489 / 40.356 / 37.162-40.356 | 0.864 / 60.293 / 33.810-60.293 |
| User CPU (ms) | 10.000 | not captured | not captured | 20.000 | 30.000 | 20.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 14.86 (gnu-time-v) | not measured | not measured | 20.94 (gnu-time-v) | 25.08 (gnu-time-v) | 22.25 (gnu-time-v) |
| Logical throughput (records/s) | 90696.015 | not measured | not measured | 36561.119 | 35784.073 | 36135.969 |
| Physical throughput (MiB/s) | 61.455 | not measured | not measured | 24.773 | 24.213 | 29.001 |
| Output bytes | 37825 | not measured | not measured | 26705 | 26705 | 26705 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
