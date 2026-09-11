---
type: Report
title: "Sort by multiple keys"
description: "Measures sorting feature objects by magnitude and then ID."
workload: benchmark.comma-generator-sort
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Sort by multiple keys

## What this measures

The query sorts the feature objects by `.properties.mag` and `.id`. The comma
generator supplies a second key for ties, so the whole collection is retained
until ordering is known.

## Why it matters

Stable tie-breaking makes reports and pagination repeatable. It also costs more
than sorting a single scalar because the result keeps complete feature objects.

## Input and output

The input is a natural feature snapshot. The output is one array of the
original feature objects, ordered by magnitude and then ID, rather than an
array of just the sort keys.

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
| Wall (ms) | 23.617 | 25.108 | 63.092 | 22.889 | 23.279 | 23.485 |
| Wall dispersion (MAD / p95 / range) | 0.309 / 24.544 / 23.079-24.544 | 0.302 / 26.297 / 24.617-26.297 | 0.718 / 66.019 / 62.368-66.019 | 0.292 / 24.133 / 22.163-24.133 | 0.469 / 24.474 / 22.343-24.474 | 0.385 / 24.610 / 22.928-24.610 |
| First output (ms) | 23.602 | 25.090 | 62.453 | 22.875 | 23.264 | 23.472 |
| First output dispersion (MAD / p95 / range) | 0.307 / 24.533 / 23.061-24.533 | 0.394 / 26.277 / 20.531-26.277 | 2.146 / 66.000 / 27.510-66.000 | 0.292 / 24.122 / 22.146-24.122 | 0.469 / 24.457 / 22.326-24.457 | 0.384 / 24.593 / 22.916-24.593 |
| User CPU (ms) | 0.000 | 15.000 | 20.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 10.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.54 (gnu-time-v) | 27.58 (gnu-time-v) | 35.27 (gnu-time-v) | 9.32 (gnu-time-v) | 9.82 (gnu-time-v) | 9.15 (gnu-time-v) |
| Logical throughput (records/s) | 8214.248 | 7726.621 | 3074.875 | 8475.872 | 8333.691 | 8260.416 |
| Physical throughput (MiB/s) | 5.624 | 5.290 | 2.102 | 5.803 | 5.697 | 6.687 |
| Output bytes | 196577 | 138764 | 138764 | 164367 | 164367 | 164367 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.269 | 23.303 | 23.480 | 23.634 | 23.273 | 23.440 |
| Wall dispersion (MAD / p95 / range) | 0.398 / 24.356 / 22.234-24.497 | 0.417 / 24.385 / 21.726-24.389 | 0.306 / 23.975 / 22.398-24.184 | 0.438 / 25.048 / 22.720-25.079 | 0.586 / 25.027 / 22.192-25.138 | 0.374 / 24.266 / 22.253-24.362 |
| First output (ms) | 23.255 | 23.287 | 23.462 | 23.613 | 23.261 | 23.424 |
| First output dispersion (MAD / p95 / range) | 0.397 / 24.339 / 22.219-24.486 | 0.417 / 24.372 / 21.711-24.375 | 0.308 / 23.957 / 22.381-24.169 | 0.438 / 25.037 / 22.702-25.064 | 0.587 / 25.014 / 22.177-25.122 | 0.372 / 24.254 / 22.238-24.339 |
| User CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.04 (gnu-time-v) | 12.02 (gnu-time-v) | 12.27 (gnu-time-v) | 8.21 (gnu-time-v) | 8.56 (gnu-time-v) | 7.90 (gnu-time-v) |
| Logical throughput (records/s) | 85.951 | 85.824 | 85.179 | 84.626 | 85.935 | 85.324 |
| Physical throughput (MiB/s) | 0.073 | 0.073 | 0.072 | 0.072 | 0.073 | 0.083 |
| Output bytes | 2038 | 1441 | 1441 | 1710 | 1710 | 1710 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 250.113 | 995.106 | 1516.708 | 215.988 | 291.662 | 254.006 |
| Wall dispersion (MAD / p95 / range) | 4.131 / 257.136 / 217.063-257.136 | 1.980 / 1029.323 / 988.954-1029.323 | 13.543 / 1588.695 / 1500.093-1588.695 | 0.824 / 221.373 / 213.906-221.373 | 2.015 / 293.888 / 288.133-293.888 | 1.151 / 260.737 / 249.088-260.737 |
| First output (ms) | 102.563 | 945.214 | 1429.506 | 136.741 | 183.299 | 153.055 |
| First output dispersion (MAD / p95 / range) | 1.295 / 104.795 / 99.402-104.795 | 2.102 / 954.374 / 929.629-954.374 | 17.694 / 1473.288 / 1404.194-1473.288 | 1.306 / 141.425 / 135.133-141.425 | 1.583 / 185.031 / 179.632-185.031 | 1.467 / 158.065 / 150.230-158.065 |
| User CPU (ms) | 180.000 | 1200.000 | 2100.000 | 170.000 | 215.000 | 190.000 |
| System CPU (ms) | 30.000 | 250.000 | 495.000 | 30.000 | 30.000 | 30.000 |
| Peak RSS (MiB, source) | 64.96 (gnu-time-v) | 738.21 (gnu-time-v) | 1524.91 (gnu-time-v) | 72.84 (gnu-time-v) | 93.03 (gnu-time-v) | 81.75 (gnu-time-v) |
| Logical throughput (records/s) | 45075.626 | 11329.446 | 7433.204 | 52197.465 | 38654.333 | 44384.778 |
| Physical throughput (MiB/s) | 30.554 | 7.680 | 5.031 | 35.382 | 26.165 | 35.634 |
| Output bytes | 11361260 | 8001607 | 8001607 | 9490694 | 9490694 | 9490694 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 61.563 | 219.785 | 298.705 | 62.074 | 62.202 | 61.983 |
| Wall dispersion (MAD / p95 / range) | 0.578 / 63.237 / 60.573-63.237 | 0.655 / 221.012 / 217.980-221.012 | 1.254 / 336.888 / 297.118-336.888 | 0.406 / 63.513 / 61.154-63.513 | 0.254 / 63.453 / 61.319-63.453 | 0.405 / 63.572 / 60.374-63.572 |
| First output (ms) | 20.970 | 188.600 | 279.022 | 29.107 | 36.490 | 31.925 |
| First output dispersion (MAD / p95 / range) | 0.581 / 22.383 / 20.153-22.383 | 1.423 / 193.475 / 184.743-193.475 | 2.815 / 283.716 / 275.033-283.716 | 0.398 / 30.280 / 27.514-30.280 | 0.515 / 37.096 / 35.029-37.096 | 0.753 / 33.109 / 30.572-33.109 |
| User CPU (ms) | 30.000 | 220.000 | 360.000 | 30.000 | 30.000 | 30.000 |
| System CPU (ms) | 0.000 | 50.000 | 100.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 15.42 (gnu-time-v) | 179.07 (gnu-time-v) | 267.02 (gnu-time-v) | 20.78 (gnu-time-v) | 24.99 (gnu-time-v) | 22.15 (gnu-time-v) |
| Logical throughput (records/s) | 36141.545 | 10123.530 | 7448.808 | 35844.315 | 35770.554 | 35896.939 |
| Physical throughput (MiB/s) | 24.489 | 6.860 | 5.040 | 24.288 | 24.204 | 28.809 |
| Output bytes | 2241402 | 1578351 | 1578351 | 1872104 | 1872104 | 1872104 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
