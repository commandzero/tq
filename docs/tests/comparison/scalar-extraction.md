---
type: Report
title: "Scalar field extraction"
description: "Measures navigation from a document to one nested scalar."
workload: benchmark.scalar-extraction
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Scalar field extraction

## What this measures

The query `.metadata.count` parses a document, follows two object fields, and
returns one scalar. It is a small document-mode navigation case.

## Why it matters

Configuration checks and request handlers commonly need one field from a larger
payload. The result shows the cost of that narrow read rather than a full walk.

## Input and output

The input is a natural snapshot with a `metadata.count` field. The output is a
single scalar in the result sequence, not the surrounding metadata object or
the original document.

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
| Wall (ms) | 23.768 | 24.309 | 26.130 | 24.287 | 23.730 | 23.567 |
| Wall dispersion (MAD / p95 / range) | 0.415 / 25.333 / 22.953-25.333 | 0.594 / 25.389 / 22.257-25.389 | 0.585 / 27.165 / 24.869-27.165 | 0.487 / 26.365 / 22.690-26.365 | 0.199 / 24.196 / 22.192-24.196 | 0.304 / 24.384 / 22.253-24.384 |
| First output (ms) | 23.754 | 24.293 | 26.111 | 24.271 | 23.715 | 23.549 |
| First output dispersion (MAD / p95 / range) | 0.415 / 25.314 / 22.942-25.314 | 0.594 / 25.374 / 22.241-25.374 | 0.585 / 27.147 / 20.141-27.147 | 0.488 / 26.345 / 22.675-26.345 | 0.200 / 24.183 / 22.135-24.183 | 0.304 / 24.367 / 22.235-24.367 |
| User CPU (ms) | 0.000 | 0.000 | 10.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 5.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.43 (gnu-time-v) | 18.02 (gnu-time-v) | 33.14 (gnu-time-v) | 9.05 (gnu-time-v) | 9.65 (gnu-time-v) | 9.05 (gnu-time-v) |
| Logical throughput (records/s) | 8162.235 | 7980.583 | 7424.558 | 7987.977 | 8175.306 | 8232.024 |
| Physical throughput (MiB/s) | 5.588 | 5.464 | 5.076 | 5.469 | 5.589 | 6.664 |
| Output bytes | 4 | 4 | 4 | 4 | 4 | 4 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.369 | 23.355 | 23.314 | 23.238 | 23.735 | 23.335 |
| Wall dispersion (MAD / p95 / range) | 0.532 / 24.118 / 22.000-24.347 | 0.501 / 24.643 / 22.280-24.966 | 0.393 / 24.193 / 22.398-24.845 | 0.498 / 26.062 / 22.163-26.300 | 0.565 / 24.607 / 21.981-24.939 | 0.354 / 24.623 / 22.113-25.821 |
| First output (ms) | 23.357 | 23.341 | 23.299 | 23.223 | 23.720 | 23.321 |
| First output dispersion (MAD / p95 / range) | 0.533 / 24.101 / 21.982-24.328 | 0.502 / 24.629 / 22.264-24.947 | 0.395 / 24.177 / 22.385-24.828 | 0.498 / 26.046 / 22.146-26.282 | 0.560 / 24.588 / 21.967-24.923 | 0.355 / 24.606 / 22.098-25.796 |
| User CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.02 (gnu-time-v) | 11.77 (gnu-time-v) | 11.89 (gnu-time-v) | 8.12 (gnu-time-v) | 8.37 (gnu-time-v) | 8.07 (gnu-time-v) |
| Logical throughput (records/s) | 85.582 | 85.635 | 85.787 | 86.064 | 84.264 | 85.706 |
| Physical throughput (MiB/s) | 0.072 | 0.072 | 0.073 | 0.073 | 0.071 | 0.083 |
| Output bytes | 2 | 2 | 2 | 2 | 2 | 2 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 98.934 | 259.637 | 922.644 | 136.457 | 176.583 | 172.616 |
| Wall dispersion (MAD / p95 / range) | 0.935 / 103.078 / 96.998-103.078 | 0.465 / 297.629 / 256.982-297.629 | 28.689 / 966.527 / 853.264-966.527 | 0.803 / 138.078 / 135.045-138.078 | 0.870 / 181.060 / 173.132-181.060 | 3.525 / 177.381 / 139.003-177.381 |
| First output (ms) | 92.326 | 239.110 | 836.203 | 119.748 | 161.433 | 136.178 |
| First output dispersion (MAD / p95 / range) | 0.253 / 93.602 / 91.184-93.602 | 2.047 / 246.052 / 234.412-246.052 | 20.051 / 858.490 / 785.871-858.490 | 0.633 / 120.700 / 118.046-120.700 | 0.858 / 164.273 / 159.736-164.273 | 0.577 / 137.734 / 134.967-137.734 |
| User CPU (ms) | 60.000 | 230.000 | 1230.000 | 90.000 | 130.000 | 110.000 |
| System CPU (ms) | 20.000 | 100.000 | 445.000 | 20.000 | 30.000 | 20.000 |
| Peak RSS (MiB, source) | 60.23 (gnu-time-v) | 293.19 (gnu-time-v) | 1301.94 (gnu-time-v) | 69.27 (gnu-time-v) | 89.49 (gnu-time-v) | 78.29 (gnu-time-v) |
| Logical throughput (records/s) | 113954.758 | 43422.162 | 12219.231 | 82619.433 | 63845.511 | 65312.601 |
| Physical throughput (MiB/s) | 77.243 | 29.433 | 8.271 | 56.003 | 43.216 | 52.436 |
| Output bytes | 6 | 6 | 6 | 6 | 6 | 6 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.921 | 64.439 | 182.258 | 60.610 | 62.077 | 60.778 |
| Wall dispersion (MAD / p95 / range) | 0.144 / 24.622 / 23.465-24.622 | 0.398 / 64.932 / 63.141-64.932 | 0.819 / 184.691 / 180.158-184.691 | 0.675 / 62.158 / 59.388-62.158 | 0.310 / 62.677 / 60.856-62.677 | 0.448 / 61.423 / 60.157-61.423 |
| First output (ms) | 19.689 | 52.007 | 152.357 | 26.139 | 32.453 | 29.206 |
| First output dispersion (MAD / p95 / range) | 1.396 / 24.603 / 18.189-24.603 | 0.454 / 54.792 / 51.234-54.792 | 6.187 / 161.187 / 144.557-161.187 | 0.609 / 26.988 / 24.965-26.988 | 0.579 / 34.513 / 31.208-34.513 | 0.469 / 30.958 / 28.328-30.958 |
| User CPU (ms) | 10.000 | 50.000 | 195.000 | 20.000 | 20.000 | 20.000 |
| System CPU (ms) | 0.000 | 20.000 | 80.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 14.48 (gnu-time-v) | 68.14 (gnu-time-v) | 225.98 (gnu-time-v) | 20.07 (gnu-time-v) | 24.27 (gnu-time-v) | 21.58 (gnu-time-v) |
| Logical throughput (records/s) | 93016.450 | 34528.511 | 12207.935 | 36710.114 | 35842.871 | 36608.641 |
| Physical throughput (MiB/s) | 63.027 | 23.396 | 8.260 | 24.874 | 24.253 | 29.380 |
| Output bytes | 5 | 5 | 5 | 5 | 5 | 5 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
