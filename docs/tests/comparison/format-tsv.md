---
type: Report
title: "TSV formatting"
description: "Measures formatting selected feature fields as tab-separated rows."
workload: benchmark.format-tsv
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# TSV formatting

## What this measures

The query selects `id`, `place`, and `mag` for each feature and applies `@tsv`.
It measures tab escaping and text output across the complete feature stream.

## Why it matters

Tab-separated rows are easy to pipe into Unix tools and import into tables.
Escaping embedded tabs and control characters is part of the real work.

## Input and output

The input is a natural feature snapshot. The output is one TSV string per
feature containing the three selected fields, not a JSON representation.

The yq adapter converts null magnitudes to empty fields, matching jq TSV text. Numeric zero remains a numeric field.

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
| Wall (ms) | 23.713 | 23.963 | 25.886 | 23.447 | 23.413 | 23.556 |
| Wall dispersion (MAD / p95 / range) | 0.786 / 25.557 / 22.610-25.557 | 0.154 / 25.966 / 23.661-25.966 | 0.428 / 27.311 / 24.939-27.311 | 0.305 / 24.148 / 22.183-24.148 | 0.369 / 25.151 / 22.903-25.151 | 0.391 / 24.615 / 22.267-24.615 |
| First output (ms) | 23.696 | 18.605 | 21.288 | 23.432 | 23.398 | 23.542 |
| First output dispersion (MAD / p95 / range) | 0.787 / 25.535 / 22.596-25.535 | 5.468 / 25.950 / 12.954-25.950 | 0.659 / 22.489 / 19.431-22.489 | 0.307 / 24.128 / 22.168-24.128 | 0.368 / 25.136 / 22.888-25.136 | 0.390 / 24.599 / 22.252-24.599 |
| User CPU (ms) | 0.000 | 10.000 | 20.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 10.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.36 (gnu-time-v) | 21.57 (gnu-time-v) | 32.77 (gnu-time-v) | 9.21 (gnu-time-v) | 9.65 (gnu-time-v) | 9.09 (gnu-time-v) |
| Logical throughput (records/s) | 8180.994 | 8095.814 | 7494.543 | 8274.156 | 8285.995 | 8235.519 |
| Physical throughput (MiB/s) | 5.601 | 5.542 | 5.124 | 5.665 | 5.665 | 6.667 |
| Output bytes | 9570 | 9570 | 9570 | 9570 | 9570 | 9570 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.590 | 23.547 | 23.549 | 23.396 | 23.750 | 23.541 |
| Wall dispersion (MAD / p95 / range) | 0.431 / 24.768 / 22.739-25.121 | 0.389 / 24.227 / 22.154-24.253 | 0.293 / 24.530 / 22.080-24.628 | 0.506 / 24.221 / 22.705-24.380 | 0.426 / 24.769 / 22.970-24.957 | 0.324 / 24.449 / 22.326-24.491 |
| First output (ms) | 23.573 | 23.534 | 23.532 | 23.381 | 23.737 | 23.526 |
| First output dispersion (MAD / p95 / range) | 0.430 / 24.753 / 22.724-25.106 | 0.389 / 24.209 / 22.137-24.238 | 0.294 / 24.512 / 22.066-24.612 | 0.505 / 24.204 / 22.694-24.364 | 0.425 / 24.754 / 22.954-24.944 | 0.323 / 24.433 / 22.308-24.476 |
| User CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.06 (gnu-time-v) | 11.89 (gnu-time-v) | 12.14 (gnu-time-v) | 8.15 (gnu-time-v) | 8.39 (gnu-time-v) | 7.90 (gnu-time-v) |
| Logical throughput (records/s) | 84.783 | 84.937 | 84.929 | 85.485 | 84.211 | 84.956 |
| Physical throughput (MiB/s) | 0.072 | 0.072 | 0.072 | 0.072 | 0.071 | 0.082 |
| Output bytes | 94 | 94 | 94 | 94 | 94 | 94 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 137.263 | 528.732 | 1119.732 | 246.251 | 290.156 | 248.085 |
| Wall dispersion (MAD / p95 / range) | 1.133 / 140.494 / 133.827-140.494 | 1.197 / 530.449 / 523.836-530.449 | 26.556 / 1163.463 / 1051.234-1163.463 | 1.841 / 253.128 / 210.741-253.128 | 0.693 / 292.079 / 286.852-292.079 | 0.832 / 254.288 / 245.658-254.288 |
| First output (ms) | 81.240 | 406.469 | 943.047 | 115.583 | 157.025 | 131.352 |
| First output dispersion (MAD / p95 / range) | 0.430 / 83.416 / 79.041-83.416 | 2.495 / 413.400 / 399.924-413.400 | 33.847 / 986.831 / 903.445-986.831 | 1.608 / 118.416 / 112.775-118.416 | 1.482 / 160.794 / 155.119-160.794 | 1.516 / 134.710 / 129.418-134.710 |
| User CPU (ms) | 90.000 | 555.000 | 1635.000 | 160.000 | 195.000 | 170.000 |
| System CPU (ms) | 20.000 | 185.000 | 470.000 | 50.000 | 60.000 | 55.000 |
| Peak RSS (MiB, source) | 60.67 (gnu-time-v) | 460.48 (gnu-time-v) | 1304.88 (gnu-time-v) | 69.42 (gnu-time-v) | 89.61 (gnu-time-v) | 78.26 (gnu-time-v) |
| Logical throughput (records/s) | 82134.297 | 21322.692 | 10068.481 | 45782.555 | 38854.961 | 45444.102 |
| Physical throughput (MiB/s) | 55.674 | 14.453 | 6.815 | 31.033 | 26.300 | 36.485 |
| Output bytes | 553684 | 553684 | 553684 | 553684 | 553684 | 553684 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 59.733 | 140.426 | 221.748 | 61.689 | 62.031 | 60.739 |
| Wall dispersion (MAD / p95 / range) | 0.504 / 61.250 / 23.991-61.250 | 0.923 / 142.773 / 104.659-142.773 | 0.945 / 262.094 / 218.753-262.094 | 0.486 / 63.637 / 60.118-63.637 | 0.530 / 63.387 / 61.088-63.387 | 0.353 / 62.428 / 59.627-62.428 |
| First output (ms) | 17.903 | 84.163 | 185.351 | 34.477 | 42.636 | 37.589 |
| First output dispersion (MAD / p95 / range) | 0.396 / 18.846 / 17.409-18.846 | 0.704 / 86.081 / 82.336-86.081 | 1.906 / 194.469 / 170.198-194.469 | 0.896 / 36.169 / 32.486-36.169 | 0.631 / 43.502 / 40.704-43.502 | 0.366 / 38.841 / 35.758-38.841 |
| User CPU (ms) | 10.000 | 110.000 | 280.000 | 30.000 | 30.000 | 30.000 |
| System CPU (ms) | 0.000 | 40.000 | 95.000 | 10.000 | 10.000 | 10.000 |
| Peak RSS (MiB, source) | 14.67 (gnu-time-v) | 109.39 (gnu-time-v) | 238.04 (gnu-time-v) | 20.26 (gnu-time-v) | 24.41 (gnu-time-v) | 21.53 (gnu-time-v) |
| Logical throughput (records/s) | 37248.780 | 15844.644 | 10033.912 | 36068.019 | 35869.162 | 36632.449 |
| Physical throughput (MiB/s) | 25.239 | 10.736 | 6.789 | 24.439 | 24.270 | 29.399 |
| Output bytes | 109544 | 109544 | 109544 | 109544 | 109544 | 109544 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | parser-specific | parser-specific |
<!-- benchmark-results:end -->
