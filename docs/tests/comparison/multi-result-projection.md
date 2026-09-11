---
type: Report
title: "Projecting many results"
description: "Measures streaming extraction of one field from every feature."
workload: benchmark.multi-result-projection
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Projecting many results

## What this measures

The query `.features[].properties.mag` walks the feature collection and emits
each magnitude as soon as the stream reaches it. It exercises repeated field
navigation without a final collection step.

## Why it matters

Streaming projections feed pipes, counters, and line-oriented consumers. Time
to the first value matters here as much as total time.

## Input and output

The input is a natural snapshot containing a `features` array. The output is
one number per feature in input order, as a sequence of values rather than one
array of magnitudes.

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
| Wall (ms) | 23.752 | 24.049 | 25.585 | 23.437 | 22.833 | 22.966 |
| Wall dispersion (MAD / p95 / range) | 0.204 / 24.574 / 22.489-24.574 | 0.250 / 25.149 / 23.629-25.149 | 0.568 / 26.361 / 24.618-26.361 | 0.493 / 25.530 / 22.472-25.530 | 0.328 / 23.863 / 22.153-23.863 | 0.407 / 24.250 / 22.546-24.250 |
| First output (ms) | 23.733 | 24.037 | 18.669 | 23.422 | 22.820 | 22.951 |
| First output dispersion (MAD / p95 / range) | 0.207 / 24.557 / 22.473-24.557 | 0.250 / 25.135 / 23.614-25.135 | 0.258 / 20.534 / 17.858-20.534 | 0.492 / 25.512 / 22.457-25.512 | 0.327 / 23.851 / 22.139-23.851 | 0.407 / 24.234 / 22.529-24.234 |
| User CPU (ms) | 0.000 | 0.000 | 10.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 10.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.36 (gnu-time-v) | 19.02 (gnu-time-v) | 31.96 (gnu-time-v) | 7.45 (gnu-time-v) | 9.65 (gnu-time-v) | 7.26 (gnu-time-v) |
| Logical throughput (records/s) | 8167.905 | 8066.696 | 7582.420 | 8277.510 | 8496.474 | 8447.270 |
| Physical throughput (MiB/s) | 5.592 | 5.523 | 5.184 | 5.667 | 5.809 | 6.838 |
| Output bytes | 839 | 839 | 839 | 839 | 839 | 839 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.308 | 23.322 | 23.354 | 23.860 | 23.366 | 23.227 |
| Wall dispersion (MAD / p95 / range) | 0.360 / 23.836 / 22.340-24.090 | 0.441 / 23.999 / 21.612-24.057 | 0.292 / 23.959 / 22.224-25.545 | 0.631 / 25.670 / 22.251-26.184 | 0.307 / 24.098 / 22.133-24.364 | 0.251 / 24.231 / 22.570-24.667 |
| First output (ms) | 23.288 | 23.306 | 23.339 | 23.846 | 23.352 | 23.212 |
| First output dispersion (MAD / p95 / range) | 0.364 / 23.816 / 22.322-24.075 | 0.438 / 23.981 / 21.597-24.042 | 0.289 / 23.941 / 22.209-25.529 | 0.634 / 25.655 / 22.234-26.168 | 0.307 / 24.082 / 22.117-24.348 | 0.253 / 24.217 / 22.557-24.649 |
| User CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.06 (gnu-time-v) | 11.89 (gnu-time-v) | 12.14 (gnu-time-v) | 7.57 (gnu-time-v) | 8.41 (gnu-time-v) | 7.26 (gnu-time-v) |
| Logical throughput (records/s) | 85.807 | 85.756 | 85.638 | 83.822 | 85.593 | 86.105 |
| Physical throughput (MiB/s) | 0.073 | 0.073 | 0.072 | 0.071 | 0.072 | 0.084 |
| Output bytes | 10 | 10 | 10 | 10 | 10 | 10 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 136.729 | 375.233 | 1040.836 | 169.334 | 251.576 | 132.898 |
| Wall dispersion (MAD / p95 / range) | 0.950 / 138.988 / 133.993-138.988 | 1.667 / 415.208 / 372.867-415.208 | 6.405 / 1051.852 / 958.215-1051.852 | 1.002 / 171.454 / 137.590-171.454 | 1.522 / 254.315 / 247.457-254.315 | 0.416 / 135.158 / 132.454-135.158 |
| First output (ms) | 81.589 | 271.096 | 871.972 | 137.492 | 233.137 | 124.714 |
| First output dispersion (MAD / p95 / range) | 0.455 / 82.180 / 79.576-82.180 | 1.365 / 273.010 / 268.919-273.010 | 5.278 / 881.588 / 806.966-881.588 | 3.681 / 171.438 / 133.440-171.438 | 2.365 / 238.402 / 230.052-238.402 | 4.494 / 133.077 / 119.929-133.077 |
| User CPU (ms) | 70.000 | 380.000 | 1315.000 | 130.000 | 170.000 | 110.000 |
| System CPU (ms) | 20.000 | 140.000 | 490.000 | 0.000 | 60.000 | 0.000 |
| Peak RSS (MiB, source) | 60.54 (gnu-time-v) | 331.81 (gnu-time-v) | 1302.93 (gnu-time-v) | 7.77 (gnu-time-v) | 89.38 (gnu-time-v) | 7.23 (gnu-time-v) |
| Logical throughput (records/s) | 82455.075 | 30045.292 | 10831.678 | 66578.281 | 44813.496 | 84832.296 |
| Physical throughput (MiB/s) | 55.891 | 20.366 | 7.332 | 45.129 | 30.334 | 68.107 |
| Output bytes | 50801 | 50801 | 50801 | 50801 | 50801 | 50801 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 24.616 | 102.381 | 217.724 | 59.455 | 61.461 | 60.297 |
| Wall dispersion (MAD / p95 / range) | 0.816 / 26.202 / 23.571-26.202 | 0.548 / 103.739 / 101.707-103.739 | 1.416 / 219.907 / 181.101-219.907 | 0.199 / 60.974 / 58.229-60.974 | 0.169 / 62.577 / 60.054-62.577 | 0.573 / 62.025 / 58.824-62.025 |
| First output (ms) | 18.215 | 57.634 | 159.700 | 59.330 | 45.340 | 60.086 |
| First output dispersion (MAD / p95 / range) | 0.611 / 19.289 / 17.226-19.289 | 0.749 / 61.144 / 56.246-61.144 | 6.712 / 169.433 / 147.590-169.433 | 0.511 / 60.959 / 28.362-60.959 | 0.589 / 47.146 / 44.636-47.146 | 0.667 / 62.014 / 26.153-62.014 |
| User CPU (ms) | 10.000 | 80.000 | 210.000 | 20.000 | 30.000 | 20.000 |
| System CPU (ms) | 0.000 | 30.000 | 90.000 | 0.000 | 10.000 | 0.000 |
| Peak RSS (MiB, source) | 14.73 (gnu-time-v) | 85.39 (gnu-time-v) | 234.78 (gnu-time-v) | 7.69 (gnu-time-v) | 24.21 (gnu-time-v) | 7.24 (gnu-time-v) |
| Logical throughput (records/s) | 90388.365 | 21732.548 | 10219.384 | 37423.261 | 36201.819 | 36900.675 |
| Physical throughput (MiB/s) | 61.246 | 14.726 | 6.915 | 25.358 | 24.496 | 29.614 |
| Output bytes | 9903 | 9903 | 9903 | 9903 | 9903 | 9903 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
