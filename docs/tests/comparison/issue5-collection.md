---
type: Report
title: "Grouping a collection"
description: "Measures grouping features by magnitude and counting the groups."
workload: benchmark.issue5-collection
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Grouping a collection

## What this measures

The query groups the feature collection by `.properties.mag` and returns the
number of groups. It exercises sorting and collection grouping even though the
final result is only a count.

## Why it matters

Grouping supports summaries such as counts by severity or category. It must
keep enough information to form complete groups before reporting the count.

## Input and output

The input is a natural feature snapshot. The output is one integer for the
number of distinct magnitude groups, not the grouped arrays themselves.

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
| Wall (ms) | 23.111 | 24.887 | 26.015 | 23.194 | 23.497 | 23.099 |
| Wall dispersion (MAD / p95 / range) | 0.615 / 24.086 / 21.782-24.086 | 0.350 / 25.658 / 23.796-25.658 | 0.633 / 28.203 / 25.175-28.203 | 0.247 / 24.276 / 22.602-24.276 | 0.191 / 24.273 / 22.920-24.273 | 0.383 / 23.880 / 22.193-23.880 |
| First output (ms) | 23.098 | 24.869 | 25.646 | 23.177 | 23.480 | 23.085 |
| First output dispersion (MAD / p95 / range) | 0.615 / 24.072 / 21.767-24.072 | 0.350 / 25.645 / 23.779-25.645 | 0.913 / 28.183 / 21.515-28.183 | 0.249 / 24.263 / 22.587-24.263 | 0.189 / 24.259 / 22.906-24.259 | 0.383 / 23.867 / 22.178-23.867 |
| User CPU (ms) | 0.000 | 10.000 | 20.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 10.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.42 (gnu-time-v) | 24.89 (gnu-time-v) | 34.27 (gnu-time-v) | 9.28 (gnu-time-v) | 9.70 (gnu-time-v) | 9.21 (gnu-time-v) |
| Logical throughput (records/s) | 8394.271 | 7795.391 | 7457.236 | 8364.232 | 8256.549 | 8398.450 |
| Physical throughput (MiB/s) | 5.747 | 5.337 | 5.098 | 5.726 | 5.645 | 6.798 |
| Output bytes | 4 | 4 | 4 | 4 | 4 | 4 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.465 | 23.288 | 23.276 | 23.247 | 23.515 | 23.413 |
| Wall dispersion (MAD / p95 / range) | 0.387 / 24.259 / 22.380-25.311 | 0.470 / 24.311 / 21.919-25.028 | 0.510 / 24.167 / 21.705-24.464 | 0.387 / 24.245 / 22.416-24.295 | 0.423 / 24.586 / 22.645-24.839 | 0.370 / 24.107 / 21.927-24.183 |
| First output (ms) | 23.451 | 23.273 | 23.261 | 23.233 | 23.500 | 23.399 |
| First output dispersion (MAD / p95 / range) | 0.389 / 24.244 / 22.363-25.293 | 0.469 / 24.295 / 21.904-25.013 | 0.510 / 24.157 / 21.691-24.447 | 0.385 / 24.229 / 22.405-24.278 | 0.421 / 24.570 / 22.627-24.824 | 0.368 / 24.093 / 21.912-24.165 |
| User CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 3.98 (gnu-time-v) | 12.02 (gnu-time-v) | 12.14 (gnu-time-v) | 8.02 (gnu-time-v) | 8.45 (gnu-time-v) | 7.99 (gnu-time-v) |
| Logical throughput (records/s) | 85.233 | 85.879 | 85.924 | 86.031 | 85.050 | 85.421 |
| Physical throughput (MiB/s) | 0.072 | 0.073 | 0.073 | 0.073 | 0.072 | 0.083 |
| Output bytes | 2 | 2 | 2 | 2 | 2 | 2 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 139.129 | 574.232 | 1128.168 | 175.655 | 215.812 | 176.321 |
| Wall dispersion (MAD / p95 / range) | 0.723 / 140.414 / 136.311-140.414 | 1.878 / 577.821 / 571.860-577.821 | 21.395 / 1204.027 / 1051.399-1204.027 | 0.552 / 177.381 / 172.867-177.381 | 2.471 / 218.565 / 208.515-218.565 | 0.712 / 177.928 / 173.845-177.928 |
| First output (ms) | 111.603 | 526.553 | 1028.199 | 145.530 | 188.113 | 162.487 |
| First output dispersion (MAD / p95 / range) | 1.409 / 114.279 / 109.620-114.279 | 4.564 / 537.565 / 510.563-537.565 | 16.134 / 1103.999 / 969.503-1103.999 | 1.610 / 147.479 / 142.245-147.479 | 2.017 / 192.746 / 185.897-192.746 | 0.834 / 163.665 / 160.349-163.665 |
| User CPU (ms) | 85.000 | 770.000 | 1645.000 | 115.000 | 150.000 | 130.000 |
| System CPU (ms) | 20.000 | 200.000 | 485.000 | 20.000 | 30.000 | 20.000 |
| Peak RSS (MiB, source) | 61.42 (gnu-time-v) | 618.60 (gnu-time-v) | 1528.16 (gnu-time-v) | 72.40 (gnu-time-v) | 92.89 (gnu-time-v) | 81.12 (gnu-time-v) |
| Logical throughput (records/s) | 81032.711 | 19633.180 | 9993.188 | 64182.631 | 52239.791 | 63940.019 |
| Physical throughput (MiB/s) | 54.927 | 13.308 | 6.764 | 43.506 | 35.360 | 51.334 |
| Output bytes | 4 | 4 | 4 | 4 | 4 | 4 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 24.588 | 141.302 | 221.478 | 60.822 | 61.731 | 61.690 |
| Wall dispersion (MAD / p95 / range) | 0.357 / 60.436 / 23.909-60.436 | 0.465 / 144.481 / 140.772-144.481 | 0.656 / 222.977 / 219.699-222.977 | 0.514 / 61.472 / 59.143-61.472 | 0.431 / 63.250 / 60.772-63.250 | 1.144 / 64.034 / 60.105-64.034 |
| First output (ms) | 22.988 | 106.319 | 200.838 | 30.855 | 37.589 | 33.859 |
| First output dispersion (MAD / p95 / range) | 0.853 / 60.417 / 21.699-60.417 | 0.848 / 110.013 / 104.079-110.013 | 1.056 / 203.000 / 194.114-203.000 | 0.651 / 32.004 / 29.121-32.004 | 0.740 / 39.063 / 36.271-39.063 | 0.485 / 63.101 / 32.606-63.101 |
| User CPU (ms) | 10.000 | 140.000 | 275.000 | 20.000 | 20.000 | 20.000 |
| System CPU (ms) | 0.000 | 40.000 | 100.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 14.73 (gnu-time-v) | 142.89 (gnu-time-v) | 256.45 (gnu-time-v) | 20.54 (gnu-time-v) | 25.00 (gnu-time-v) | 21.99 (gnu-time-v) |
| Logical throughput (records/s) | 90491.297 | 15746.415 | 10046.122 | 36582.158 | 36043.187 | 36067.726 |
| Physical throughput (MiB/s) | 61.316 | 10.670 | 6.798 | 24.788 | 24.388 | 28.946 |
| Output bytes | 4 | 4 | 4 | 4 | 4 | 4 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
