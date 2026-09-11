---
type: Report
title: "User function calls"
description: "Measures passing a value through a user-defined filter function."
workload: benchmark.user-filter-call
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# User function calls

## What this measures

The query defines `magnitude(f)` and calls it for each feature's magnitude.
This isolates function definition, argument passing, and repeated invocation.

## Why it matters

Reusable filters make production queries easier to maintain. A function call
should keep the same stream behavior as the equivalent inline expression.

## Input and output

The input is a natural feature snapshot. The output is one magnitude per
feature in input order, emitted as a sequence after passing through the user
function.

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
| Wall (ms) | 23.674 | not measured | not measured | 23.547 | 23.157 | 23.509 |
| Wall dispersion (MAD / p95 / range) | 0.222 / 24.860 / 22.913-24.860 | not measured | not measured | 0.202 / 24.901 / 22.047-24.901 | 0.515 / 24.145 / 22.203-24.145 | 0.428 / 24.632 / 22.799-24.632 |
| First output (ms) | 23.659 | not measured | not measured | 23.529 | 23.145 | 23.492 |
| First output dispersion (MAD / p95 / range) | 0.223 / 24.845 / 22.889-24.845 | not measured | not measured | 0.204 / 24.882 / 22.032-24.882 | 0.515 / 24.130 / 22.186-24.130 | 0.429 / 24.619 / 22.785-24.619 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.40 (gnu-time-v) | not measured | not measured | 9.06 (gnu-time-v) | 9.65 (gnu-time-v) | 9.12 (gnu-time-v) |
| Logical throughput (records/s) | 8194.471 | not measured | not measured | 8239.016 | 8377.416 | 8252.334 |
| Physical throughput (MiB/s) | 5.610 | not measured | not measured | 5.641 | 5.727 | 6.680 |
| Output bytes | 839 | not measured | not measured | 839 | 839 | 839 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.860 | not measured | not measured | 23.394 | 23.585 | 23.601 |
| Wall dispersion (MAD / p95 / range) | 0.434 / 24.812 / 22.708-25.453 | not measured | not measured | 0.488 / 24.370 / 22.239-25.144 | 0.296 / 24.419 / 22.566-24.612 | 0.288 / 24.519 / 22.093-24.797 |
| First output (ms) | 23.843 | not measured | not measured | 23.378 | 23.570 | 23.584 |
| First output dispersion (MAD / p95 / range) | 0.435 / 24.796 / 22.693-25.434 | not measured | not measured | 0.486 / 24.358 / 22.226-25.126 | 0.293 / 24.403 / 22.551-24.597 | 0.288 / 24.508 / 22.079-24.786 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.02 (gnu-time-v) | not measured | not measured | 8.03 (gnu-time-v) | 8.41 (gnu-time-v) | 7.93 (gnu-time-v) |
| Logical throughput (records/s) | 83.824 | not measured | not measured | 85.492 | 84.800 | 84.744 |
| Physical throughput (MiB/s) | 0.071 | not measured | not measured | 0.072 | 0.072 | 0.082 |
| Output bytes | 10 | not measured | not measured | 10 | 10 | 10 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 120.784 | not measured | not measured | 211.928 | 249.910 | 248.918 |
| Wall dispersion (MAD / p95 / range) | 17.605 / 140.153 / 100.473-140.153 | not measured | not measured | 1.014 / 216.378 / 207.547-216.378 | 1.657 / 285.988 / 247.108-285.988 | 1.703 / 251.414 / 214.153-251.414 |
| First output (ms) | 80.450 | not measured | not measured | 195.870 | 238.185 | 213.955 |
| First output dispersion (MAD / p95 / range) | 0.548 / 82.158 / 79.506-82.158 | not measured | not measured | 1.087 / 202.431 / 194.320-202.431 | 1.368 / 248.544 / 236.021-248.544 | 2.739 / 219.625 / 209.114-219.625 |
| User CPU (ms) | 70.000 | not captured | not captured | 140.000 | 175.000 | 150.000 |
| System CPU (ms) | 20.000 | not captured | not captured | 50.000 | 60.000 | 50.000 |
| Peak RSS (MiB, source) | 60.54 (gnu-time-v) | not measured | not measured | 69.29 (gnu-time-v) | 89.36 (gnu-time-v) | 77.99 (gnu-time-v) |
| Logical throughput (records/s) | 93340.564 | not measured | not measured | 53197.312 | 45112.240 | 45292.024 |
| Physical throughput (MiB/s) | 63.270 | not measured | not measured | 36.059 | 30.536 | 36.363 |
| Output bytes | 50801 | not measured | not measured | 50801 | 50801 | 50801 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 24.117 | not measured | not measured | 60.556 | 61.932 | 60.685 |
| Wall dispersion (MAD / p95 / range) | 0.554 / 24.940 / 22.178-24.940 | not measured | not measured | 0.473 / 61.387 / 59.801-61.387 | 0.459 / 62.955 / 60.607-62.955 | 0.508 / 61.771 / 59.551-61.771 |
| First output (ms) | 18.113 | not measured | not measured | 40.816 | 47.950 | 44.227 |
| First output dispersion (MAD / p95 / range) | 0.524 / 18.967 / 17.254-18.967 | not measured | not measured | 0.962 / 42.388 / 38.887-42.388 | 0.358 / 48.810 / 47.232-48.810 | 0.621 / 60.339 / 41.623-60.339 |
| User CPU (ms) | 10.000 | not captured | not captured | 20.000 | 30.000 | 25.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 5.000 | 10.000 | 10.000 |
| Peak RSS (MiB, source) | 14.73 (gnu-time-v) | not measured | not measured | 20.08 (gnu-time-v) | 24.20 (gnu-time-v) | 21.59 (gnu-time-v) |
| Logical throughput (records/s) | 92258.573 | not measured | not measured | 36742.546 | 35926.210 | 36664.744 |
| Physical throughput (MiB/s) | 62.514 | not measured | not measured | 24.896 | 24.309 | 29.425 |
| Output bytes | 9903 | not measured | not measured | 9903 | 9903 | 9903 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq def expression used by this adapter. | yq rejects the catalog jq def expression used by this adapter. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
