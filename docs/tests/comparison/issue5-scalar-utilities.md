---
type: Report
title: "String and scalar utilities"
description: "Measures case conversion and Unicode scalar handling across place names."
workload: benchmark.issue5-scalar-utilities
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# String and scalar utilities

## What this measures

The query lowercases each string place, expands it to Unicode code points with
`explode`, counts those scalars, and adds the counts. It combines text
transformation with a numeric reduction.

## Why it matters

Text utilities often sit in normalization and search pipelines. Unicode-aware
length work is different from counting bytes, especially for non-ASCII data.

## Input and output

The input is a natural snapshot with place values. The output is one integer
containing the total code-point count after lowercasing, not the transformed
strings.

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
| Wall (ms) | 23.271 | not measured | not measured | 23.399 | 23.573 | 23.386 |
| Wall dispersion (MAD / p95 / range) | 0.451 / 25.339 / 22.452-25.339 | not measured | not measured | 0.257 / 23.984 / 22.755-23.984 | 0.157 / 24.371 / 23.078-24.371 | 0.562 / 24.304 / 21.859-24.304 |
| First output (ms) | 23.255 | not measured | not measured | 23.383 | 23.556 | 23.370 |
| First output dispersion (MAD / p95 / range) | 0.452 / 25.326 / 22.436-25.326 | not measured | not measured | 0.257 / 23.968 / 22.740-23.968 | 0.157 / 24.357 / 23.062-24.357 | 0.562 / 24.290 / 21.842-24.290 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.36 (gnu-time-v) | not measured | not measured | 9.09 (gnu-time-v) | 9.60 (gnu-time-v) | 9.15 (gnu-time-v) |
| Logical throughput (records/s) | 8336.556 | not measured | not measured | 8290.775 | 8229.929 | 8295.739 |
| Physical throughput (MiB/s) | 5.707 | not measured | not measured | 5.676 | 5.627 | 6.715 |
| Output bytes | 5 | not measured | not measured | 5 | 5 | 5 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq scalar-utility expression. | yq rejects the catalog jq scalar-utility expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.474 | not measured | not measured | 23.274 | 23.523 | 23.140 |
| Wall dispersion (MAD / p95 / range) | 0.388 / 26.328 / 22.017-27.131 | not measured | not measured | 0.479 / 24.350 / 21.747-24.927 | 0.444 / 24.361 / 22.273-24.592 | 0.241 / 23.812 / 22.218-24.341 |
| First output (ms) | 23.459 | not measured | not measured | 23.259 | 23.506 | 23.128 |
| First output dispersion (MAD / p95 / range) | 0.387 / 26.312 / 22.002-27.108 | not measured | not measured | 0.479 / 24.338 / 21.729-24.912 | 0.444 / 24.348 / 22.258-24.580 | 0.240 / 23.797 / 22.199-24.329 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.02 (gnu-time-v) | not measured | not measured | 8.37 (gnu-time-v) | 8.56 (gnu-time-v) | 8.15 (gnu-time-v) |
| Logical throughput (records/s) | 85.202 | not measured | not measured | 85.933 | 85.025 | 86.429 |
| Physical throughput (MiB/s) | 0.072 | not measured | not measured | 0.073 | 0.072 | 0.084 |
| Output bytes | 3 | not measured | not measured | 3 | 3 | 3 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq scalar-utility expression. | yq rejects the catalog jq scalar-utility expression. | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 213.758 | not measured | not measured | 175.484 | 213.125 | 210.055 |
| Wall dispersion (MAD / p95 / range) | 1.700 / 247.757 / 211.184-247.757 | not measured | not measured | 1.194 / 178.431 / 173.239-178.431 | 2.003 / 216.239 / 210.667-216.239 | 2.479 / 212.598 / 174.015-212.598 |
| First output (ms) | 206.901 | not measured | not measured | 151.401 | 195.329 | 170.231 |
| First output dispersion (MAD / p95 / range) | 0.856 / 213.055 / 204.112-213.055 | not measured | not measured | 0.660 / 154.574 / 149.842-154.574 | 1.155 / 197.449 / 192.784-197.449 | 1.234 / 172.439 / 167.934-172.439 |
| User CPU (ms) | 180.000 | not captured | not captured | 120.000 | 160.000 | 140.000 |
| System CPU (ms) | 20.000 | not captured | not captured | 20.000 | 30.000 | 30.000 |
| Peak RSS (MiB, source) | 60.61 (gnu-time-v) | not measured | not measured | 70.47 (gnu-time-v) | 90.59 (gnu-time-v) | 79.18 (gnu-time-v) |
| Logical throughput (records/s) | 52741.886 | not measured | not measured | 64244.990 | 52898.658 | 53671.657 |
| Physical throughput (MiB/s) | 35.751 | not measured | not measured | 43.548 | 35.806 | 43.090 |
| Output bytes | 7 | not measured | not measured | 7 | 7 | 7 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq scalar-utility expression. | yq rejects the catalog jq scalar-utility expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 60.892 | not measured | not measured | 60.269 | 61.986 | 61.444 |
| Wall dispersion (MAD / p95 / range) | 0.135 / 61.365 / 59.531-61.365 | not measured | not measured | 0.233 / 60.672 / 59.551-60.672 | 0.365 / 62.681 / 61.339-62.681 | 0.366 / 63.291 / 60.270-63.291 |
| First output (ms) | 42.453 | not measured | not measured | 32.182 | 39.330 | 35.681 |
| First output dispersion (MAD / p95 / range) | 0.819 / 60.923 / 40.653-60.923 | not measured | not measured | 0.714 / 33.313 / 30.838-33.313 | 0.138 / 40.721 / 38.899-40.721 | 0.420 / 36.623 / 34.651-36.623 |
| User CPU (ms) | 30.000 | not captured | not captured | 20.000 | 30.000 | 20.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 14.73 (gnu-time-v) | not measured | not measured | 20.50 (gnu-time-v) | 24.47 (gnu-time-v) | 21.96 (gnu-time-v) |
| Logical throughput (records/s) | 36539.804 | not measured | not measured | 36918.125 | 35895.202 | 36211.835 |
| Physical throughput (MiB/s) | 24.759 | not measured | not measured | 25.015 | 24.288 | 29.062 |
| Output bytes | 6 | not measured | not measured | 6 | 6 | 6 |
| Outcome | timed | unsupported | unsupported | timed | timed | timed |
| Details | none | yq rejects the catalog jq scalar-utility expression. | yq rejects the catalog jq scalar-utility expression. | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
