---
type: Report
title: "Any-match predicate"
description: "Measures stopping a collection scan when one feature satisfies a predicate."
workload: benchmark.issue5-predicate
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Any-match predicate

## What this measures

The query asks whether any feature has a magnitude at least zero. It exercises
predicate evaluation with a possible early exit from the feature stream.

## Why it matters

Existence checks are common in alerts and guards. Their cost depends on where
the first match occurs, so they differ from reductions that always scan all
values.

## Input and output

The input is a natural feature snapshot. The output is one boolean, `true` when
at least one feature meets the predicate and `false` otherwise.

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
| Wall (ms) | 23.309 | not measured | not measured | 23.061 | 23.439 | 23.448 |
| Wall dispersion (MAD / p95 / range) | 0.393 / 24.023 / 22.628-24.023 | not measured | not measured | 0.349 / 24.356 / 22.529-24.356 | 0.413 / 24.541 / 22.616-24.541 | 0.333 / 24.243 / 22.902-24.243 |
| First output (ms) | 23.297 | not measured | not measured | 23.046 | 23.424 | 23.434 |
| First output dispersion (MAD / p95 / range) | 0.392 / 24.010 / 22.614-24.010 | not measured | not measured | 0.353 / 24.340 / 22.510-24.340 | 0.412 / 24.523 / 22.601-24.523 | 0.333 / 24.227 / 22.886-24.227 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.23 (gnu-time-v) | not measured | not measured | 9.08 (gnu-time-v) | 9.70 (gnu-time-v) | 9.15 (gnu-time-v) |
| Logical throughput (records/s) | 8322.787 | not measured | not measured | 8412.471 | 8276.804 | 8273.627 |
| Physical throughput (MiB/s) | 5.698 | not measured | not measured | 5.759 | 5.659 | 6.697 |
| Output bytes | 5 | not measured | not measured | 5 | 5 | 5 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq any predicate expression. | yq rejects the catalog jq any predicate expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.389 | not measured | not measured | 23.383 | 23.260 | 23.506 |
| Wall dispersion (MAD / p95 / range) | 0.308 / 24.280 / 22.515-24.536 | not measured | not measured | 0.423 / 24.266 / 22.579-24.365 | 0.277 / 23.876 / 22.106-24.148 | 0.401 / 24.544 / 22.509-24.689 |
| First output (ms) | 23.372 | not measured | not measured | 23.366 | 23.245 | 23.491 |
| First output dispersion (MAD / p95 / range) | 0.307 / 24.265 / 22.498-24.521 | not measured | not measured | 0.422 / 24.248 / 22.561-24.350 | 0.277 / 23.865 / 22.087-24.132 | 0.401 / 24.526 / 22.493-24.675 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.09 (gnu-time-v) | not measured | not measured | 8.10 (gnu-time-v) | 8.44 (gnu-time-v) | 8.07 (gnu-time-v) |
| Logical throughput (records/s) | 85.512 | not measured | not measured | 85.534 | 85.985 | 85.086 |
| Physical throughput (MiB/s) | 0.072 | not measured | not measured | 0.072 | 0.073 | 0.083 |
| Output bytes | 5 | not measured | not measured | 5 | 5 | 5 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq any predicate expression. | yq rejects the catalog jq any predicate expression. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 100.031 | not measured | not measured | 136.101 | 176.332 | 175.238 |
| Wall dispersion (MAD / p95 / range) | 1.403 / 134.362 / 97.340-134.362 | not measured | not measured | 0.659 / 137.468 / 134.600-137.468 | 0.792 / 177.663 / 172.375-177.663 | 1.173 / 176.794 / 170.348-176.794 |
| First output (ms) | 92.493 | not measured | not measured | 121.012 | 162.096 | 136.929 |
| First output dispersion (MAD / p95 / range) | 1.123 / 94.965 / 90.772-94.965 | not measured | not measured | 0.626 / 122.737 / 119.576-122.737 | 0.373 / 164.010 / 161.384-164.010 | 0.365 / 139.025 / 135.588-139.025 |
| User CPU (ms) | 60.000 | not captured | not captured | 90.000 | 130.000 | 110.000 |
| System CPU (ms) | 20.000 | not captured | not captured | 20.000 | 30.000 | 20.000 |
| Peak RSS (MiB, source) | 60.25 (gnu-time-v) | not measured | not measured | 69.46 (gnu-time-v) | 89.31 (gnu-time-v) | 78.00 (gnu-time-v) |
| Logical throughput (records/s) | 112705.625 | not measured | not measured | 82835.541 | 63936.211 | 64335.361 |
| Physical throughput (MiB/s) | 76.396 | not measured | not measured | 56.149 | 43.278 | 51.652 |
| Output bytes | 5 | not measured | not measured | 5 | 5 | 5 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq any predicate expression. | yq rejects the catalog jq any predicate expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 24.049 | not measured | not measured | 60.971 | 61.428 | 60.546 |
| Wall dispersion (MAD / p95 / range) | 0.517 / 24.816 / 23.149-24.816 | not measured | not measured | 0.683 / 61.866 / 59.540-61.866 | 0.381 / 65.381 / 60.539-65.381 | 0.906 / 63.314 / 58.835-63.314 |
| First output (ms) | 20.593 | not measured | not measured | 25.721 | 34.322 | 28.742 |
| First output dispersion (MAD / p95 / range) | 1.419 / 24.472 / 19.165-24.472 | not measured | not measured | 0.326 / 26.843 / 25.335-26.843 | 0.291 / 37.350 / 31.770-37.350 | 0.524 / 29.970 / 27.318-29.970 |
| User CPU (ms) | 10.000 | not captured | not captured | 10.000 | 20.000 | 20.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 14.45 (gnu-time-v) | not measured | not measured | 20.15 (gnu-time-v) | 24.27 (gnu-time-v) | 21.77 (gnu-time-v) |
| Logical throughput (records/s) | 92517.516 | not measured | not measured | 36492.759 | 36221.267 | 36748.918 |
| Physical throughput (MiB/s) | 62.689 | not measured | not measured | 24.727 | 24.509 | 29.493 |
| Output bytes | 5 | not measured | not measured | 5 | 5 | 5 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq any predicate expression. | yq rejects the catalog jq any predicate expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
