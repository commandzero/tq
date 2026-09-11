---
type: Report
title: "Path update"
description: "Measures updating one nested object while returning the full document."
workload: benchmark.path-update
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Path update

## What this measures

The update query adds `benchmarked: true` inside `.metadata` and emits the
complete document. It exercises path focus, object update, and full-document
serialization.

## Why it matters

Enriching records in place is a normal ETL operation. The work includes both
finding a nested path and carrying the untouched parts of the document through.

## Input and output

The input is a natural snapshot with a metadata object. The output is the whole
snapshot with one metadata field added, not just the changed metadata value.

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
| Wall (ms) | 23.524 | 24.178 | 61.772 | 23.725 | 23.735 | 23.474 |
| Wall dispersion (MAD / p95 / range) | 0.563 / 25.434 / 22.645-25.434 | 0.598 / 25.067 / 23.038-25.067 | 1.345 / 63.308 / 25.635-63.308 | 0.751 / 25.011 / 22.045-25.011 | 0.316 / 24.710 / 22.284-24.710 | 0.387 / 24.438 / 21.879-24.438 |
| First output (ms) | 23.512 | 23.695 | 26.747 | 23.709 | 23.718 | 23.460 |
| First output dispersion (MAD / p95 / range) | 0.568 / 25.421 / 22.630-25.421 | 0.842 / 25.050 / 17.673-25.050 | 1.184 / 63.289 / 25.509-63.289 | 0.752 / 24.996 / 22.030-24.996 | 0.315 / 24.696 / 22.270-24.696 | 0.387 / 24.424 / 21.864-24.424 |
| User CPU (ms) | 0.000 | 10.000 | 20.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 10.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.56 (gnu-time-v) | 23.02 (gnu-time-v) | 32.77 (gnu-time-v) | 9.33 (gnu-time-v) | 10.07 (gnu-time-v) | 9.21 (gnu-time-v) |
| Logical throughput (records/s) | 8246.722 | 8023.657 | 3140.607 | 8177.028 | 8173.756 | 8264.639 |
| Physical throughput (MiB/s) | 5.646 | 5.493 | 2.147 | 5.598 | 5.588 | 6.690 |
| Output bytes | 212523 | 139093 | 139093 | 164690 | 164690 | 164690 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.311 | 23.543 | 23.506 | 23.252 | 23.351 | 23.188 |
| Wall dispersion (MAD / p95 / range) | 0.353 / 24.687 / 21.890-25.003 | 0.431 / 24.549 / 22.212-24.801 | 0.311 / 24.223 / 22.257-24.292 | 0.282 / 24.083 / 22.010-24.466 | 0.328 / 24.042 / 22.086-24.639 | 0.411 / 24.459 / 21.847-25.722 |
| First output (ms) | 23.294 | 23.527 | 23.491 | 23.241 | 23.337 | 23.173 |
| First output dispersion (MAD / p95 / range) | 0.356 / 24.670 / 21.876-24.992 | 0.429 / 24.536 / 22.198-24.790 | 0.311 / 24.208 / 22.241-24.278 | 0.283 / 24.068 / 21.994-24.456 | 0.329 / 24.028 / 22.072-24.626 | 0.411 / 24.443 / 21.828-25.704 |
| User CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.06 (gnu-time-v) | 12.02 (gnu-time-v) | 12.27 (gnu-time-v) | 8.37 (gnu-time-v) | 8.65 (gnu-time-v) | 8.16 (gnu-time-v) |
| Logical throughput (records/s) | 85.796 | 84.953 | 85.085 | 86.016 | 85.649 | 86.252 |
| Physical throughput (MiB/s) | 0.073 | 0.072 | 0.072 | 0.073 | 0.072 | 0.084 |
| Output bytes | 2648 | 1794 | 1794 | 2057 | 2057 | 2057 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 210.435 | 761.375 | 1327.983 | 175.259 | 215.352 | 209.443 |
| Wall dispersion (MAD / p95 / range) | 1.530 / 214.581 / 206.747-214.581 | 2.701 / 765.547 / 724.770-765.547 | 38.584 / 1367.601 / 1272.831-1367.601 | 1.782 / 178.131 / 173.340-178.131 | 2.401 / 250.736 / 210.594-250.736 | 1.675 / 213.429 / 206.232-213.429 |
| First output (ms) | 80.959 | 700.722 | 1235.355 | 105.097 | 145.610 | 118.718 |
| First output dispersion (MAD / p95 / range) | 0.567 / 82.599 / 79.808-82.599 | 3.135 / 710.341 / 695.221-710.341 | 32.755 / 1273.147 / 1193.106-1273.147 | 0.509 / 106.709 / 104.435-106.709 | 1.548 / 148.441 / 143.337-148.441 | 0.853 / 121.618 / 117.720-121.618 |
| User CPU (ms) | 160.000 | 790.000 | 1785.000 | 140.000 | 170.000 | 150.000 |
| System CPU (ms) | 30.000 | 170.000 | 465.000 | 20.000 | 30.000 | 30.000 |
| Peak RSS (MiB, source) | 64.86 (gnu-time-v) | 485.11 (gnu-time-v) | 1308.05 (gnu-time-v) | 69.71 (gnu-time-v) | 89.57 (gnu-time-v) | 78.30 (gnu-time-v) |
| Logical throughput (records/s) | 53574.738 | 14807.411 | 8489.570 | 64327.652 | 52351.377 | 53828.488 |
| Physical throughput (MiB/s) | 36.315 | 10.037 | 5.746 | 43.604 | 35.436 | 43.216 |
| Output bytes | 12263602 | 8001932 | 8001932 | 9491013 | 9491013 | 9491013 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 60.934 | 181.379 | 260.456 | 61.510 | 61.901 | 61.078 |
| Wall dispersion (MAD / p95 / range) | 0.943 / 62.858 / 59.286-62.858 | 0.979 / 185.578 / 178.823-185.578 | 0.501 / 264.683 / 259.116-264.683 | 0.573 / 63.062 / 60.433-63.062 | 0.476 / 63.107 / 61.121-63.107 | 0.517 / 62.312 / 59.751-62.312 |
| First output (ms) | 18.024 | 143.357 | 238.342 | 23.129 | 31.192 | 26.358 |
| First output dispersion (MAD / p95 / range) | 0.313 / 18.866 / 16.862-18.866 | 1.062 / 146.072 / 140.112-146.072 | 1.868 / 244.047 / 233.714-244.047 | 0.559 / 24.471 / 22.303-24.471 | 0.539 / 32.595 / 29.862-32.595 | 0.572 / 27.445 / 25.245-27.445 |
| User CPU (ms) | 30.000 | 170.000 | 320.000 | 20.000 | 30.000 | 30.000 |
| System CPU (ms) | 0.000 | 40.000 | 80.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 15.38 (gnu-time-v) | 104.20 (gnu-time-v) | 225.97 (gnu-time-v) | 20.30 (gnu-time-v) | 24.41 (gnu-time-v) | 21.77 (gnu-time-v) |
| Logical throughput (records/s) | 36514.618 | 12267.165 | 8542.710 | 36172.980 | 35944.202 | 36429.127 |
| Physical throughput (MiB/s) | 24.742 | 8.312 | 5.780 | 24.510 | 24.321 | 29.236 |
| Output bytes | 2419820 | 1578672 | 1578672 | 1872419 | 1872419 | 1872419 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
