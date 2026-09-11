---
type: Report
title: "Sort before counting"
description: "Measures work whose sorted intermediate does not affect the final count."
workload: benchmark.dead-sort-length
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Sort before counting

## What this measures

The query collects the `release` values, sorts them, and then asks for the
length. The final count does not depend on the order, so the case exposes the
cost of doing unnecessary intermediate work.

## Why it matters

Real queries sometimes carry an expensive step that a later operation does not
use. This is a useful stress case for planning and optimization decisions.

## Input and output

The input is a natural feature snapshot. The output is one integer, the number
of collected values. It contains no sorted values.

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
| Wall (ms) | 23.646 | 24.396 | 25.055 | 23.430 | 23.611 | 23.846 |
| Wall dispersion (MAD / p95 / range) | 0.199 / 24.082 / 22.724-24.082 | 0.244 / 24.821 / 23.017-24.821 | 0.442 / 26.490 / 24.524-26.490 | 0.412 / 24.169 / 22.528-24.169 | 0.389 / 24.159 / 23.079-24.159 | 0.445 / 24.321 / 22.181-24.321 |
| First output (ms) | 23.633 | 24.380 | 24.710 | 23.416 | 23.597 | 23.826 |
| First output dispersion (MAD / p95 / range) | 0.198 / 24.068 / 22.705-24.068 | 0.245 / 24.803 / 23.006-24.803 | 0.731 / 26.101 / 17.718-26.101 | 0.410 / 24.152 / 22.515-24.152 | 0.387 / 24.148 / 23.064-24.148 | 0.444 / 24.310 / 22.167-24.310 |
| User CPU (ms) | 0.000 | 0.000 | 10.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.42 (gnu-time-v) | 19.02 (gnu-time-v) | 33.27 (gnu-time-v) | 8.21 (gnu-time-v) | 9.65 (gnu-time-v) | 7.79 (gnu-time-v) |
| Logical throughput (records/s) | 8204.174 | 7952.123 | 7742.965 | 8280.160 | 8216.509 | 8135.366 |
| Physical throughput (MiB/s) | 5.617 | 5.444 | 5.294 | 5.669 | 5.617 | 6.585 |
| Output bytes | 4 | 4 | 4 | 4 | 4 | 4 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.723 | 23.483 | 23.519 | 23.392 | 23.675 | 23.840 |
| Wall dispersion (MAD / p95 / range) | 0.284 / 24.267 / 22.515-25.279 | 0.365 / 25.133 / 22.596-25.341 | 0.454 / 24.746 / 22.601-24.952 | 0.255 / 24.228 / 21.901-24.866 | 0.530 / 24.760 / 22.210-25.063 | 0.406 / 24.548 / 23.137-25.037 |
| First output (ms) | 23.709 | 23.470 | 23.504 | 23.378 | 23.660 | 23.826 |
| First output dispersion (MAD / p95 / range) | 0.284 / 24.249 / 22.504-25.260 | 0.368 / 25.118 / 22.578-25.328 | 0.452 / 24.735 / 22.586-24.939 | 0.257 / 24.213 / 21.887-24.850 | 0.533 / 24.742 / 22.194-25.052 | 0.404 / 24.537 / 23.119-25.019 |
| User CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.04 (gnu-time-v) | 11.89 (gnu-time-v) | 12.02 (gnu-time-v) | 8.23 (gnu-time-v) | 8.32 (gnu-time-v) | 7.86 (gnu-time-v) |
| Logical throughput (records/s) | 84.306 | 85.168 | 85.038 | 85.499 | 84.477 | 83.894 |
| Physical throughput (MiB/s) | 0.071 | 0.072 | 0.072 | 0.072 | 0.072 | 0.081 |
| Output bytes | 2 | 2 | 2 | 2 | 2 | 2 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 101.633 | 336.211 | 927.943 | 135.675 | 178.732 | 134.135 |
| Wall dispersion (MAD / p95 / range) | 1.169 / 138.846 / 100.197-138.846 | 1.389 / 338.443 / 332.688-338.443 | 38.577 / 1014.731 / 851.710-1014.731 | 2.748 / 172.215 / 132.402-172.215 | 2.421 / 212.702 / 176.024-212.702 | 1.022 / 135.962 / 131.987-135.962 |
| First output (ms) | 97.245 | 287.659 | 846.374 | 132.911 | 169.501 | 127.540 |
| First output dispersion (MAD / p95 / range) | 0.700 / 98.699 / 96.067-98.699 | 4.034 / 296.311 / 278.324-296.311 | 32.261 / 914.068 / 792.826-914.068 | 3.433 / 172.195 / 127.054-172.195 | 1.008 / 172.335 / 166.843-172.335 | 6.074 / 135.180 / 120.412-135.180 |
| User CPU (ms) | 70.000 | 340.000 | 1265.000 | 120.000 | 140.000 | 110.000 |
| System CPU (ms) | 20.000 | 120.000 | 430.000 | 0.000 | 20.000 | 0.000 |
| Peak RSS (MiB, source) | 60.48 (gnu-time-v) | 332.97 (gnu-time-v) | 1302.80 (gnu-time-v) | 8.51 (gnu-time-v) | 90.09 (gnu-time-v) | 8.08 (gnu-time-v) |
| Logical throughput (records/s) | 110927.991 | 33532.514 | 12149.447 | 83095.327 | 63077.504 | 84049.965 |
| Physical throughput (MiB/s) | 75.191 | 22.730 | 8.224 | 56.325 | 42.696 | 67.479 |
| Output bytes | 6 | 6 | 6 | 6 | 6 | 6 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.939 | 65.850 | 183.231 | 60.490 | 61.501 | 60.067 |
| Wall dispersion (MAD / p95 / range) | 0.344 / 25.153 / 23.451-25.153 | 0.432 / 66.627 / 64.126-66.627 | 0.655 / 223.837 / 181.009-223.837 | 0.337 / 61.335 / 59.477-61.335 | 0.465 / 62.886 / 59.307-62.886 | 0.941 / 63.346 / 58.492-63.346 |
| First output (ms) | 21.026 | 59.605 | 161.219 | 28.531 | 33.780 | 59.638 |
| First output dispersion (MAD / p95 / range) | 0.549 / 25.137 / 19.878-25.137 | 1.038 / 61.144 / 57.222-61.144 | 5.236 / 172.154 / 150.276-172.154 | 1.665 / 60.576 / 26.745-60.576 | 0.685 / 36.954 / 32.934-36.954 | 1.196 / 63.323 / 25.463-63.323 |
| User CPU (ms) | 10.000 | 60.000 | 200.000 | 20.000 | 20.000 | 20.000 |
| System CPU (ms) | 0.000 | 20.000 | 85.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 14.73 (gnu-time-v) | 77.45 (gnu-time-v) | 226.53 (gnu-time-v) | 8.27 (gnu-time-v) | 24.40 (gnu-time-v) | 8.02 (gnu-time-v) |
| Logical throughput (records/s) | 92944.567 | 33788.914 | 12143.175 | 36782.635 | 36178.274 | 37041.970 |
| Physical throughput (MiB/s) | 62.978 | 22.895 | 8.217 | 24.924 | 24.480 | 29.728 |
| Output bytes | 5 | 5 | 5 | 5 | 5 | 5 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
