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

Tools: `jq` (jq-1.8.1), `tq` (tq 0.1.0 (TOON v3; jq target 1.8.x; revision a4b4d916-worktree)), `yq` (yq (https://github.com/mikefarah/yq/) version v4.53.2)
Environment: `linux` / `x86_64`, AMD Ryzen 7 7700 8-Core Processor, 16 logical CPUs, 61.9 GiB RAM; kernel `Linux 7.2.0-ogc4.1.fc44.x86_64 #1 SMP PREEMPT_DYNAMIC Thu Aug 20 16:15:37 UTC 2026`; compiler profile `release-benchmark`

Peak RSS is authoritative only after the campaign RSS preflight passes. Missing or invalid values are `-`, never estimates. RSS collector provenance (outside measurement tables): `instrumented linux-wait4`, `linux-wait4`.
Measurement method (outside measurement tables): `tq-bench` native measurement: RSS scope `wait4 child lifetime including pre exec waited descendants and threads`; residual RSS floor `2.5 MiB` retained, not subtracted; observed control excess `1.1 ms`; primary timing is sampler-free; instrumented `tq-bench` native measurement: RSS scope `wait4 child lifetime including pre exec waited descendants and threads; sampled worker process group`; residual RSS floor `2.5 MiB` retained, not subtracted; observed control excess `1.0 ms`; RSS-limit enforcement uses a separate worker process-group sampler and is not pooled with primary timing.

Compare columns with the same input format to isolate tool differences. Missing adapters have `-` measurement cells and `not recorded` outcome/detail cells.

Timing rows show numeric medians followed by compact MAD / p95 / range rows, with one decimal place and a unit in each measurement cell. CPU, throughput, and output cells use the captured summary values.

Separate RSS-limit enforcement samples remain in the raw report and are not included in these tables or pooled with primary measurements.

### usgs-all-day

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `tq stream mode requires TOON or JSON event input; YAML is document-at-a-time.`, `yq rejects the catalog jq stream-event invocation.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 6.0 ms | - | - | 21.8 ms | - | 20.7 ms |
| Wall time dispersion | 0.1 ms / 6.4 ms / 5.7-6.4 ms | - | - | 0.3 ms / 22.8 ms / 20.9-22.8 ms | - | 0.2 ms / 21.9 ms / 20.2-21.9 ms |
| First output | 5.9 ms | - | - | 21.5 ms | - | 20.5 ms |
| First output dispersion | 0.1 ms / 6.4 ms / 5.7-6.4 ms | - | - | 0.3 ms / 22.6 ms / 20.7-22.6 ms | - | 0.3 ms / 21.7 ms / 20.0-21.7 ms |
| User CPU | 5.0 ms | - | - | 21.4 ms | - | 19.7 ms |
| System CPU | 1.0 ms | - | - | 0.5 ms | - | 1.0 ms |
| Peak RSS | 4.0 MiB | - | - | 8.2 MiB | - | 7.7 MiB |
| Logical throughput | 32103.3 records/s | - | - | 8906.4 records/s | - | 9382.6 records/s |
| Physical throughput | 22.0 MiB/s | - | - | 6.1 MiB/s | - | 7.6 MiB/s |
| Output bytes | 46.0 B | - | - | 41.0 B | - | 41.0 B |
| Outcome | timed | unsupported | unsupported | timed | unsupported | timed |
| Details | none | yq rejects the catalog jq stream-event invocation. | yq rejects the catalog jq stream-event invocation. | none | tq stream mode requires TOON or JSON event input; YAML is document-at-a-time. | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | not timed | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `tq stream mode requires TOON or JSON event input; YAML is document-at-a-time.`, `yq rejects the catalog jq stream-event invocation.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.5 ms | - | - | 1.5 ms | - | 1.4 ms |
| Wall time dispersion | 0.0 ms / 1.8 ms / 1.4-1.8 ms | - | - | 0.1 ms / 1.7 ms / 1.3-1.7 ms | - | 0.0 ms / 1.7 ms / 1.3-1.7 ms |
| First output | 1.4 ms | - | - | 1.3 ms | - | 1.2 ms |
| First output dispersion | 0.1 ms / 1.7 ms / 1.3-1.8 ms | - | - | 0.1 ms / 1.5 ms / 1.1-1.5 ms | - | 0.0 ms / 1.5 ms / 1.1-1.6 ms |
| User CPU | 0.8 ms | - | - | 0.0 ms | - | 1.2 ms |
| System CPU | 0.7 ms | - | - | 1.3 ms | - | 0.0 ms |
| Peak RSS | 4.1 MiB | - | - | 8.2 MiB | - | 7.7 MiB |
| Logical throughput | 1331.6 records/s | - | - | 1333.3 records/s | - | 1481.5 records/s |
| Physical throughput | 1.1 MiB/s | - | - | 1.1 MiB/s | - | 1.4 MiB/s |
| Output bytes | 46.0 B | - | - | 41.0 B | - | 41.0 B |
| Outcome | timed | unsupported | unsupported | timed | unsupported | timed |
| Details | none | yq rejects the catalog jq stream-event invocation. | yq rejects the catalog jq stream-event invocation. | none | tq stream mode requires TOON or JSON event input; YAML is document-at-a-time. | none |
| Samples (warmups) | 30 measured (2 warmup) | not timed | not timed | 30 measured (2 warmup) | not timed | 30 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `tq stream mode requires TOON or JSON event input; YAML is document-at-a-time.`, `yq rejects the catalog jq stream-event invocation.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 257.4 ms | - | - | 1153.1 ms | - | 1081.8 ms |
| Wall time dispersion | 1.2 ms / 262.8 ms / 253.9-262.8 ms | - | - | 6.1 ms / 1216.9 ms / 1143.7-1216.9 ms | - | 8.3 ms / 1110.4 ms / 1065.8-1110.4 ms |
| First output | 257.2 ms | - | - | 1152.8 ms | - | 1081.5 ms |
| First output dispersion | 1.2 ms / 262.5 ms / 253.7-262.5 ms | - | - | 6.2 ms / 1216.4 ms / 1143.3-1216.4 ms | - | 8.3 ms / 1110.1 ms / 1065.5-1110.1 ms |
| User CPU | 253.7 ms | - | - | 1149.3 ms | - | 1075.9 ms |
| System CPU | 3.0 ms | - | - | 2.5 ms | - | 3.0 ms |
| Peak RSS | 3.9 MiB | - | - | 7.9 MiB | - | 7.7 MiB |
| Logical throughput | 43805.6 records/s | - | - | 9777.3 records/s | - | 10421.4 records/s |
| Physical throughput | 29.7 MiB/s | - | - | 6.6 MiB/s | - | 8.4 MiB/s |
| Output bytes | 46.0 B | - | - | 41.0 B | - | 41.0 B |
| Outcome | timed | unsupported | unsupported | timed | unsupported | timed |
| Details | none | yq rejects the catalog jq stream-event invocation. | yq rejects the catalog jq stream-event invocation. | none | tq stream mode requires TOON or JSON event input; YAML is document-at-a-time. | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | not timed | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tool identity differs`, `tq stream mode requires TOON or JSON event input; YAML is document-at-a-time.`, `yq rejects the catalog jq stream-event invocation.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 52.4 ms | - | - | 230.0 ms | - | 216.9 ms |
| Wall time dispersion | 0.4 ms / 53.5 ms / 51.8-53.5 ms | - | - | 1.2 ms / 242.0 ms / 225.8-242.0 ms | - | 2.5 ms / 230.4 ms / 213.4-230.4 ms |
| First output | 52.3 ms | - | - | 229.8 ms | - | 216.7 ms |
| First output dispersion | 0.4 ms / 53.4 ms / 51.7-53.4 ms | - | - | 1.2 ms / 241.9 ms / 225.5-241.9 ms | - | 2.6 ms / 230.2 ms / 213.1-230.2 ms |
| User CPU | 51.4 ms | - | - | 227.9 ms | - | 215.0 ms |
| System CPU | 1.0 ms | - | - | 1.0 ms | - | 1.0 ms |
| Peak RSS | 4.1 MiB | - | - | 8.0 MiB | - | 7.7 MiB |
| Logical throughput | 42440.8 records/s | - | - | 9673.0 records/s | - | 10260.0 records/s |
| Physical throughput | 28.8 MiB/s | - | - | 6.6 MiB/s | - | 8.2 MiB/s |
| Output bytes | 46.0 B | - | - | 41.0 B | - | 41.0 B |
| Outcome | timed | unsupported | unsupported | timed | unsupported | timed |
| Details | none | yq rejects the catalog jq stream-event invocation. | yq rejects the catalog jq stream-event invocation. | none | tq stream mode requires TOON or JSON event input; YAML is document-at-a-time. | none |
| Samples (warmups) | 10 measured (2 warmup) | not timed | not timed | 10 measured (2 warmup) | not timed | 10 measured (2 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
