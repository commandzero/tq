---
type: Report
title: "Discarding a stream"
description: "Measures parsing and stream control when a query emits no values."
workload: benchmark.parse-discard
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Discarding a stream

## What this measures

The `empty` query reads each natural input and emits nothing. The workload
isolates input handling and the cost of a filter that discards its result.

## Why it matters

Log and validation pipelines often reject or discard records early. This case
shows the cost of doing that without paying to serialize an output document.

## Input and output

The input is a natural JSON, YAML, or TOON snapshot at each catalog size. The
output is an empty value sequence, not an empty array. That distinction keeps
parsing cost separate from output construction.

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
| Wall (ms) | 23.096 | 24.151 | 25.597 | 23.243 | 23.046 | 23.484 |
| Wall dispersion (MAD / p95 / range) | 0.385 / 24.398 / 21.857-24.398 | 0.177 / 25.081 / 22.897-25.081 | 0.443 / 26.218 / 24.403-26.218 | 0.304 / 24.358 / 22.539-24.358 | 0.417 / 24.040 / 22.259-24.040 | 0.330 / 25.167 / 22.812-25.167 |
| First output (ms) | no output | no output | no output | no output | no output | no output |
| First output dispersion (MAD / p95 / range) | no output | no output | no output | no output | no output | no output |
| User CPU (ms) | 0.000 | 0.000 | 10.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.36 (gnu-time-v) | 17.52 (gnu-time-v) | 32.77 (gnu-time-v) | 8.32 (gnu-time-v) | 8.93 (gnu-time-v) | 8.20 (gnu-time-v) |
| Logical throughput (records/s) | 8399.541 | 8032.794 | 7579.013 | 8346.599 | 8417.947 | 8260.944 |
| Physical throughput (MiB/s) | 5.750 | 5.499 | 5.182 | 5.714 | 5.755 | 6.687 |
| Output bytes | 0 | 0 | 0 | 0 | 0 | 0 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.592 | 23.437 | 23.284 | 23.253 | 23.581 | 23.384 |
| Wall dispersion (MAD / p95 / range) | 0.292 / 24.314 / 22.484-24.647 | 0.286 / 24.473 / 22.460-24.796 | 0.414 / 23.996 / 22.477-24.752 | 0.402 / 24.786 / 21.838-24.803 | 0.684 / 25.338 / 21.961-25.350 | 0.554 / 24.413 / 22.136-24.558 |
| First output (ms) | no output | no output | no output | no output | no output | no output |
| First output dispersion (MAD / p95 / range) | no output | no output | no output | no output | no output | no output |
| User CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| System CPU (ms) | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 4.05 (gnu-time-v) | 11.08 (gnu-time-v) | 11.14 (gnu-time-v) | 7.28 (gnu-time-v) | 7.66 (gnu-time-v) | 7.15 (gnu-time-v) |
| Logical throughput (records/s) | 84.774 | 85.335 | 85.896 | 86.010 | 84.812 | 85.529 |
| Physical throughput (MiB/s) | 0.072 | 0.072 | 0.073 | 0.073 | 0.072 | 0.083 |
| Output bytes | 0 | 0 | 0 | 0 | 0 | 0 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 100.554 | 260.953 | 926.953 | 137.826 | 175.416 | 173.940 |
| Wall dispersion (MAD / p95 / range) | 1.058 / 101.959 / 97.938-101.959 | 3.902 / 295.522 / 257.004-295.522 | 2.228 / 962.467 / 922.580-962.467 | 0.798 / 139.392 / 133.999-139.392 | 1.421 / 178.012 / 173.508-178.012 | 1.635 / 177.531 / 137.795-177.531 |
| First output (ms) | no output | no output | no output | no output | no output | no output |
| First output dispersion (MAD / p95 / range) | no output | no output | no output | no output | no output | no output |
| User CPU (ms) | 60.000 | 230.000 | 1225.000 | 90.000 | 130.000 | 110.000 |
| System CPU (ms) | 20.000 | 110.000 | 455.000 | 20.000 | 20.000 | 20.000 |
| Peak RSS (MiB, source) | 60.34 (gnu-time-v) | 292.48 (gnu-time-v) | 1300.65 (gnu-time-v) | 68.70 (gnu-time-v) | 88.78 (gnu-time-v) | 77.20 (gnu-time-v) |
| Logical throughput (records/s) | 112118.862 | 43203.182 | 12162.422 | 81799.087 | 64270.261 | 64815.454 |
| Physical throughput (MiB/s) | 75.999 | 29.285 | 8.233 | 55.447 | 43.504 | 52.037 |
| Output bytes | 0 | 0 | 0 | 0 | 0 | 0 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 24.165 | 64.270 | 181.794 | 60.499 | 62.346 | 60.678 |
| Wall dispersion (MAD / p95 / range) | 0.715 / 25.021 / 22.381-25.021 | 0.103 / 65.582 / 63.496-65.582 | 0.831 / 183.330 / 179.421-183.330 | 0.526 / 61.860 / 25.314-61.860 | 0.534 / 64.958 / 61.441-64.958 | 0.526 / 61.517 / 59.984-61.517 |
| First output (ms) | no output | no output | no output | no output | no output | no output |
| First output dispersion (MAD / p95 / range) | no output | no output | no output | no output | no output | no output |
| User CPU (ms) | 10.000 | 40.000 | 200.000 | 10.000 | 20.000 | 20.000 |
| System CPU (ms) | 0.000 | 20.000 | 70.000 | 0.000 | 0.000 | 0.000 |
| Peak RSS (MiB, source) | 14.65 (gnu-time-v) | 67.64 (gnu-time-v) | 235.02 (gnu-time-v) | 19.35 (gnu-time-v) | 23.58 (gnu-time-v) | 20.97 (gnu-time-v) |
| Logical throughput (records/s) | 92077.221 | 34619.574 | 12239.128 | 36777.467 | 35688.221 | 36668.672 |
| Physical throughput (MiB/s) | 62.391 | 23.458 | 8.281 | 24.920 | 24.148 | 29.428 |
| Output bytes | 0 | 0 | 0 | 0 | 0 | 0 |
| Outcome | timed | timed | timed | timed | timed | timed |
| Details | none | none | none | none | none | none |
| Samples (warmups) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
