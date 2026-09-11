---
type: Report
title: "Selective filtering"
description: "Measures streaming selection and projection of matching features."
workload: benchmark.selective-filter
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Selective filtering

## What this measures

The query selects features with magnitude at least 2 and then projects their
IDs. It combines iteration, a numeric predicate, and a downstream field read.

## Why it matters

This is the shape of a useful event or records query: scan a collection, keep a
subset, and pass compact identifiers to the next command.

## Input and output

The input is a natural feature snapshot with numeric `properties.mag` values.
The output is the ordered sequence of IDs for matching features. It is not the
matching feature objects and not a packed array.

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
| Wall (ms) | 23.058 | 24.130 | 25.761 | 22.919 | 22.787 | 23.281 |
| Wall dispersion (MAD / p95 / range) | 0.408 / 24.143 / 22.568-24.143 | 0.793 / 25.575 / 22.921-25.575 | 0.298 / 26.586 / 25.242-26.586 | 0.361 / 24.334 / 22.111-24.334 | 0.435 / 24.189 / 22.147-24.189 | 0.431 / 24.360 / 22.175-24.360 |
| First output (ms) | 23.044 | 24.115 | 19.509 | 22.902 | 22.771 | 23.265 |
| First output dispersion (MAD / p95 / range) | 0.408 / 24.128 / 22.556-24.128 | 0.791 / 25.564 / 22.894-25.564 | 0.728 / 26.544 / 18.597-26.544 | 0.361 / 24.324 / 22.095-24.324 | 0.435 / 24.171 / 22.132-24.171 | 0.433 / 24.349 / 22.162-24.349 |
| User CPU (ms) | 0.000 | 0.000 | 10.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.40 (gnu-time-v) | 19.39 (gnu-time-v) | 32.39 (gnu-time-v) | 8.57 (gnu-time-v) | 9.70 (gnu-time-v) | 8.19 (gnu-time-v) |
| Logical throughput (records/s) | 8413.383 | 8039.951 | 7530.764 | 8464.777 | 8513.626 | 8332.975 |
| Physical throughput (MiB/s) | 5.760 | 5.504 | 5.149 | 5.795 | 5.820 | 6.745 |
| Output bytes | 918 | 918 | 918 | 786 | 786 | 786 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.291 | 23.276 | 23.733 | 23.621 | 23.503 | 23.642 |
| Wall dispersion (MAD / p95 / range) | 0.402 / 23.939 / 22.283-24.151 | 0.324 / 24.441 / 22.229-24.462 | 0.211 / 24.264 / 22.778-24.350 | 0.352 / 24.421 / 22.980-24.637 | 0.315 / 24.552 / 22.093-24.901 | 0.268 / 24.367 / 22.255-24.502 |
| First output (ms) | no output | no output | no output | no output | no output | no output |
| First output dispersion (MAD / p95 / range) | no output | no output | no output | no output | no output | no output |
| User CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.08 (gnu-time-v) | 11.27 (gnu-time-v) | 11.27 (gnu-time-v) | 8.45 (gnu-time-v) | 8.23 (gnu-time-v) | 8.04 (gnu-time-v) |
| Logical throughput (records/s) | 85.870 | 85.925 | 84.271 | 84.670 | 85.096 | 84.595 |
| Physical throughput (MiB/s) | 0.073 | 0.073 | 0.071 | 0.072 | 0.072 | 0.082 |
| Output bytes | 0 | 0 | 0 | 0 | 0 | 0 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 138.513 | 375.164 | 970.181 | 171.315 | 215.421 | 207.912 |
| Wall dispersion (MAD / p95 / range) | 0.874 / 139.595 / 136.857-139.595 | 1.533 / 377.928 / 371.681-377.928 | 35.199 / 1046.982 / 929.431-1046.982 | 0.615 / 178.418 / 169.739-178.418 | 2.405 / 218.064 / 211.584-218.064 | 0.849 / 209.516 / 205.546-209.516 |
| First output (ms) | 80.788 | 318.704 | 857.869 | 170.904 | 202.413 | 206.727 |
| First output dispersion (MAD / p95 / range) | 0.771 / 82.903 / 79.643-82.903 | 2.305 / 324.706 / 311.562-324.706 | 10.494 / 912.432 / 847.311-912.432 | 3.994 / 178.400 / 163.523-178.400 | 1.301 / 205.518 / 199.885-205.518 | 1.942 / 209.099 / 194.440-209.099 |
| User CPU (ms) | 70.000 | 380.000 | 1300.000 | 160.000 | 160.000 | 190.000 |
| System CPU (ms) | 20.000 | 130.000 | 410.000 | 0.000 | 40.000 | 0.000 |
| Peak RSS (MiB, source) | 60.30 (gnu-time-v) | 380.99 (gnu-time-v) | 1296.11 (gnu-time-v) | 9.09 (gnu-time-v) | 89.55 (gnu-time-v) | 8.16 (gnu-time-v) |
| Logical throughput (records/s) | 81392.788 | 30050.858 | 11620.518 | 65808.790 | 52334.731 | 54224.734 |
| Physical throughput (MiB/s) | 55.171 | 20.370 | 7.866 | 44.608 | 35.425 | 43.534 |
| Output bytes | 47722 | 47722 | 47722 | 40954 | 40954 | 40954 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.834 | 102.459 | 219.061 | 60.971 | 61.864 | 59.545 |
| Wall dispersion (MAD / p95 / range) | 0.483 / 25.340 / 23.152-25.340 | 0.543 / 103.698 / 101.515-103.698 | 2.148 / 222.655 / 180.663-222.655 | 0.382 / 63.342 / 59.016-63.342 | 0.619 / 62.908 / 61.018-62.908 | 0.262 / 61.559 / 58.420-61.559 |
| First output (ms) | 18.688 | 67.226 | 173.067 | 60.554 | 40.081 | 59.261 |
| First output dispersion (MAD / p95 / range) | 0.457 / 19.542 / 17.631-19.542 | 1.303 / 71.055 / 65.595-71.055 | 1.898 / 179.644 / 156.769-179.644 | 1.229 / 63.324 / 35.190-63.324 | 0.259 / 41.405 / 38.259-41.405 | 0.598 / 59.870 / 40.252-59.870 |
| User CPU (ms) | 10.000 | 80.000 | 210.000 | 30.000 | 30.000 | 30.000 |
| System CPU (ms) | 0.000 | 25.000 | 80.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 14.65 (gnu-time-v) | 88.02 (gnu-time-v) | 237.05 (gnu-time-v) | 8.70 (gnu-time-v) | 24.21 (gnu-time-v) | 8.12 (gnu-time-v) |
| Logical throughput (records/s) | 93354.032 | 21716.003 | 10156.988 | 36493.058 | 35965.990 | 37366.697 |
| Physical throughput (MiB/s) | 63.256 | 14.715 | 6.873 | 24.727 | 24.336 | 29.988 |
| Output bytes | 9875 | 9875 | 9875 | 8507 | 8507 | 8507 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
