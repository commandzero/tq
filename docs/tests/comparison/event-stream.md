---
type: Report
title: "Event stream filtering"
description: "Measures filtering a stream of event-shaped values."
workload: benchmark.event-stream
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Event stream filtering

## What this measures

The adapters run in JSON stream mode, where the parser emits `[path, value]`
events. The query keeps events whose path has one element. It checks streamed
path/value shapes while preserving matching events in stream order.

## Why it matters

Streaming parsers expose document structure without waiting for a complete
tree. Filtering path/value events is useful for large documents and early
selection.

## Input and output

The input is a natural snapshot read through the stream parser. The output is
the matching `[path, value]` event sequence. Nonmatching events do not become
null placeholders.

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
| Wall (ms) | 23.549 | not measured | not measured | 23.613 | not measured | 23.572 |
| Wall dispersion (MAD / p95 / range) | 0.283 / 24.257 / 22.877-24.257 | not measured | not measured | 0.274 / 59.246 / 23.156-59.246 | not measured | 0.875 / 24.665 / 22.371-24.665 |
| First output (ms) | 23.534 | not measured | not measured | 23.166 | not measured | 23.079 |
| First output dispersion (MAD / p95 / range) | 0.281 / 24.241 / 22.859-24.241 | not measured | not measured | 0.462 / 59.228 / 21.817-59.228 | not measured | 0.989 / 24.552 / 20.767-24.552 |
| User CPU (ms) | 0.000 | not captured | not captured | 20.000 | not captured | 10.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | not captured | 0.000 |
| Peak RSS (MiB, source) | 4.06 (gnu-time-v) | not measured | not measured | 8.10 (gnu-time-v) | not measured | 7.87 (gnu-time-v) |
| Logical throughput (records/s) | 8238.142 | not measured | not measured | 8215.639 | not measured | 8230.104 |
| Physical throughput (MiB/s) | 5.640 | not measured | not measured | 5.625 | not measured | 6.662 |
| Output bytes | 46 | not measured | not measured | 41 | not measured | 41 |
| Outcome | timed | unsupported | unsupported | timed | unsupported | timed |
| Details | none | yq rejects the catalog jq stream-event invocation. | yq rejects the catalog jq stream-event invocation. | none | tq stream mode requires TOON or JSON event input; YAML is document-at-a-time. | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | not timed | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 23.282 | not measured | not measured | 23.407 | not measured | 23.224 |
| Wall dispersion (MAD / p95 / range) | 0.369 / 24.161 / 21.978-24.282 | not measured | not measured | 0.297 / 24.449 / 22.254-24.641 | not measured | 0.352 / 24.170 / 22.148-24.708 |
| First output (ms) | 23.265 | not measured | not measured | 23.391 | not measured | 23.208 |
| First output dispersion (MAD / p95 / range) | 0.368 / 24.150 / 21.964-24.264 | not measured | not measured | 0.293 / 24.437 / 22.239-24.624 | not measured | 0.353 / 24.153 / 22.133-24.692 |
| User CPU (ms) | 0.000 | not captured | not captured | 0.000 | not captured | 0.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | not captured | 0.000 |
| Peak RSS (MiB, source) | 4.06 (gnu-time-v) | not measured | not measured | 8.23 (gnu-time-v) | not measured | 7.86 (gnu-time-v) |
| Logical throughput (records/s) | 85.903 | not measured | not measured | 85.445 | not measured | 86.118 |
| Physical throughput (MiB/s) | 0.073 | not measured | not measured | 0.072 | not measured | 0.084 |
| Output bytes | 46 | not measured | not measured | 41 | not measured | 41 |
| Outcome | timed | unsupported | unsupported | timed | unsupported | timed |
| Details | none | yq rejects the catalog jq stream-event invocation. | yq rejects the catalog jq stream-event invocation. | none | tq stream mode requires TOON or JSON event input; YAML is document-at-a-time. | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | not timed | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 282.396 | not measured | not measured | 1138.871 | not measured | 1097.745 |
| Wall dispersion (MAD / p95 / range) | 0.912 / 284.127 / 279.453-284.127 | not measured | not measured | 1.871 / 1179.115 / 1134.030-1179.115 | not measured | 3.075 / 1145.253 / 1092.861-1145.253 |
| First output (ms) | 280.784 | not measured | not measured | 1138.384 | not measured | 1094.524 |
| First output dispersion (MAD / p95 / range) | 2.803 / 283.895 / 258.719-283.895 | not measured | not measured | 4.053 / 1175.774 / 1125.772-1175.774 | not measured | 3.141 / 1145.239 / 1084.470-1145.239 |
| User CPU (ms) | 250.000 | not captured | not captured | 1120.000 | not captured | 1080.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | not captured | 0.000 |
| Peak RSS (MiB, source) | 4.00 (gnu-time-v) | not measured | not measured | 8.15 (gnu-time-v) | not measured | 7.91 (gnu-time-v) |
| Logical throughput (records/s) | 39922.662 | not measured | not measured | 9899.277 | not measured | 10270.145 |
| Physical throughput (MiB/s) | 27.061 | not measured | not measured | 6.710 | not measured | 8.245 |
| Output bytes | 46 | not measured | not measured | 41 | not measured | 41 |
| Outcome | timed | unsupported | unsupported | timed | unsupported | timed |
| Details | none | yq rejects the catalog jq stream-event invocation. | yq rejects the catalog jq stream-event invocation. | none | tq stream mode requires TOON or JSON event input; YAML is document-at-a-time. | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | not timed | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall (ms) | 60.495 | not measured | not measured | 246.371 | not measured | 243.808 |
| Wall dispersion (MAD / p95 / range) | 0.524 / 61.539 / 59.813-61.539 | not measured | not measured | 1.196 / 248.844 / 243.588-248.844 | not measured | 0.955 / 247.100 / 242.330-247.100 |
| First output (ms) | 60.478 | not measured | not measured | 243.590 | not measured | 232.155 |
| First output dispersion (MAD / p95 / range) | 0.524 / 61.524 / 59.798-61.524 | not measured | not measured | 4.263 / 248.831 / 229.397-248.831 | not measured | 11.950 / 244.678 / 212.701-244.678 |
| User CPU (ms) | 50.000 | not captured | not captured | 220.000 | not captured | 210.000 |
| System CPU (ms) | 0.000 | not captured | not captured | 0.000 | not captured | 0.000 |
| Peak RSS (MiB, source) | 4.06 (gnu-time-v) | not measured | not measured | 8.05 (gnu-time-v) | not measured | 7.84 (gnu-time-v) |
| Logical throughput (records/s) | 36780.203 | not measured | not measured | 9031.095 | not measured | 9126.052 |
| Physical throughput (MiB/s) | 24.922 | not measured | not measured | 6.119 | not measured | 7.324 |
| Output bytes | 46 | not measured | not measured | 41 | not measured | 41 |
| Outcome | timed | unsupported | unsupported | timed | unsupported | timed |
| Details | none | yq rejects the catalog jq stream-event invocation. | yq rejects the catalog jq stream-event invocation. | none | tq stream mode requires TOON or JSON event input; YAML is document-at-a-time. | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | not timed | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
