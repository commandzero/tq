---
type: Report
title: "Identity re-encoding"
description: "Measures reading and writing a complete natural document unchanged."
workload: benchmark.identity-reencode
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Identity re-encoding

## What this measures

The query is `.` on natural snapshots. Unlike the startup identity case, the
documents span the catalog's small, medium, and large tiers, exposing full
parse and output costs.

## Why it matters

Format conversion and pass-through filters are useful baselines for pipelines
that do not change the data. They show the cost of carrying a document through
the tool.

## Input and output

The input is a complete natural snapshot in the selected format. The output is
the same semantic document, including its structure and object members, not a
single field or a stream of projected values.

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
| Wall (ms) | 23.361 | 24.890 | 62.136 | 23.112 | 23.268 | 23.329 |
| Wall dispersion (MAD / p95 / range) | 0.198 / 23.953 / 23.026-23.953 | 0.475 / 25.805 / 24.170-25.805 | 0.654 / 64.686 / 25.078-64.686 | 0.315 / 24.233 / 22.174-24.233 | 0.425 / 24.089 / 22.221-24.089 | 0.141 / 23.476 / 22.814-23.476 |
| First output (ms) | 23.347 | 24.728 | 62.116 | 23.099 | 23.253 | 23.319 |
| First output dispersion (MAD / p95 / range) | 0.195 / 23.942 / 23.010-23.942 | 0.451 / 25.791 / 16.827-25.791 | 0.645 / 64.661 / 24.252-64.661 | 0.315 / 24.218 / 22.159-24.218 | 0.427 / 24.077 / 22.205-24.077 | 0.134 / 23.460 / 22.802-23.460 |
| User CPU (ms) | 0.000 | 10.000 | 20.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 10.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.46 (gnu-time-v) | 22.14 (gnu-time-v) | 33.89 (gnu-time-v) | 7.68 (gnu-time-v) | 9.24 (gnu-time-v) | 7.42 (gnu-time-v) |
| Logical throughput (records/s) | 8304.261 | 7794.295 | 3122.184 | 8393.908 | 8337.452 | 8315.830 |
| Physical throughput (MiB/s) | 5.685 | 5.336 | 2.135 | 5.747 | 5.700 | 6.732 |
| Output bytes | 212498 | 139074 | 139074 | 164670 | 164670 | 164670 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.290 | 23.163 | 23.337 | 23.210 | 23.353 | 23.287 |
| Wall dispersion (MAD / p95 / range) | 0.454 / 24.034 / 22.214-24.099 | 0.393 / 24.388 / 21.949-24.461 | 0.430 / 24.199 / 22.453-24.936 | 0.406 / 23.969 / 22.189-24.204 | 0.480 / 24.298 / 21.635-24.469 | 0.353 / 24.299 / 22.550-24.408 |
| First output (ms) | 23.275 | 23.149 | 23.322 | 23.197 | 23.340 | 23.273 |
| First output dispersion (MAD / p95 / range) | 0.454 / 24.020 / 22.203-24.088 | 0.395 / 24.373 / 21.937-24.445 | 0.430 / 24.189 / 22.442-24.920 | 0.410 / 23.955 / 22.178-24.193 | 0.482 / 24.284 / 21.620-24.454 | 0.351 / 24.277 / 22.535-24.393 |
| User CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.06 (gnu-time-v) | 11.89 (gnu-time-v) | 12.39 (gnu-time-v) | 7.50 (gnu-time-v) | 7.95 (gnu-time-v) | 7.23 (gnu-time-v) |
| Logical throughput (records/s) | 85.874 | 86.346 | 85.699 | 86.170 | 85.642 | 85.887 |
| Physical throughput (MiB/s) | 0.073 | 0.073 | 0.073 | 0.073 | 0.072 | 0.083 |
| Output bytes | 2623 | 1775 | 1775 | 2037 | 2037 | 2037 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 211.411 | 760.572 | 1297.316 | 543.793 | 215.493 | 541.504 |
| Wall dispersion (MAD / p95 / range) | 0.782 / 215.422 / 208.840-215.422 | 3.538 / 767.771 / 724.180-767.771 | 23.131 / 1385.509 / 1268.968-1385.509 | 3.322 / 575.987 / 539.952-575.987 | 1.696 / 246.458 / 211.941-246.458 | 1.472 / 543.283 / 534.800-543.283 |
| First output (ms) | 80.145 | 703.941 | 1227.197 | 535.189 | 145.091 | 521.878 |
| First output dispersion (MAD / p95 / range) | 0.157 / 81.288 / 79.703-81.288 | 3.552 / 710.492 / 696.030-710.492 | 28.092 / 1274.350 / 1186.150-1274.350 | 3.709 / 544.608 / 530.272-544.608 | 0.621 / 147.094 / 144.463-147.094 | 2.243 / 524.793 / 516.637-524.793 |
| User CPU (ms) | 160.000 | 780.000 | 1785.000 | 270.000 | 170.000 | 250.000 |
| System CPU (ms) | 30.000 | 170.000 | 435.000 | 260.000 | 30.000 | 260.000 |
| Peak RSS (MiB, source) | 64.92 (gnu-time-v) | 483.57 (gnu-time-v) | 1361.74 (gnu-time-v) | 15.27 (gnu-time-v) | 88.91 (gnu-time-v) | 15.22 (gnu-time-v) |
| Logical throughput (records/s) | 53327.531 | 14823.064 | 8690.246 | 20732.135 | 52317.245 | 20819.810 |
| Physical throughput (MiB/s) | 36.148 | 10.048 | 5.882 | 14.053 | 35.413 | 16.715 |
| Output bytes | 12263577 | 8001913 | 8001913 | 9490993 | 9490993 | 9490993 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 62.032 | 181.015 | 259.855 | 97.592 | 62.913 | 96.092 |
| Wall dispersion (MAD / p95 / range) | 0.239 / 62.737 / 61.470-62.737 | 0.889 / 182.179 / 178.386-182.179 | 1.268 / 298.046 / 257.106-298.046 | 0.787 / 98.640 / 95.594-98.640 | 0.885 / 64.822 / 61.004-64.822 | 2.534 / 98.973 / 60.659-98.973 |
| First output (ms) | 18.131 | 144.191 | 241.565 | 96.180 | 31.742 | 95.808 |
| First output dispersion (MAD / p95 / range) | 0.233 / 18.622 / 17.559-18.622 | 0.782 / 146.626 / 140.197-146.626 | 3.318 / 253.924 / 233.795-253.924 | 2.183 / 98.623 / 61.988-98.623 | 0.331 / 32.717 / 30.939-32.717 | 2.925 / 98.956 / 59.872-98.956 |
| User CPU (ms) | 30.000 | 160.000 | 315.000 | 50.000 | 30.000 | 40.000 |
| System CPU (ms) | 0.000 | 40.000 | 95.000 | 10.000 | 0.000 | 10.000 |
| Peak RSS (MiB, source) | 15.42 (gnu-time-v) | 104.52 (gnu-time-v) | 237.92 (gnu-time-v) | 9.14 (gnu-time-v) | 23.77 (gnu-time-v) | 8.96 (gnu-time-v) |
| Logical throughput (records/s) | 35868.295 | 12291.799 | 8562.468 | 22799.000 | 35366.581 | 23154.893 |
| Physical throughput (MiB/s) | 24.304 | 8.329 | 5.794 | 15.448 | 23.930 | 18.583 |
| Output bytes | 2419795 | 1578653 | 1578653 | 1872399 | 1872399 | 1872399 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
