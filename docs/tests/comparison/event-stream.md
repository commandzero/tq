---
type: Report
title: Event stream filtering
description: Measures filtering a stream of event-shaped values.
workload: benchmark.event-stream
generated:
  by: codex/gpt-5.6-luna
  at: 2026-09-10T20:15:01Z
benchmark_runs:
- campaign_id: 2026-09-11
  provenance: Native Linux workload report; captured tool identities are listed in the Results section. No executable hashes or unique campaign ID retained.
  binaries:
    tq:
      version: tq 0.1.0 (TOON v3; jq target 1.8.x; revision a4b4d916-worktree)
      sha256: null
    jq:
      version: jq-1.8.1
      sha256: null
    yq:
      version: yq (https://github.com/mikefarah/yq/) version v4.53.2
      sha256: null
- campaign_id: 2026-10-08T15:10:27.867608118Z
  binaries:
    jq:
      version: jq-1.8.1
      sha256: 020468de7539ce70ef1bceaf7cde2e8c4f2ca6c3afb84642aabc5c97d9fc2a0d
    tq:
      version: tq 0.5.0 (TOON v4.1; jq target 1.8.x; revision unknown)
      sha256: 474625b11f0ec3a1fa125bbeb4e55008efa50ec713b0c398822ada48f0289b91
    yq:
      version: yq (https://github.com/mikefarah/yq/) version v4.53.2
      sha256: d56bf5c6819e8e696340c312bd70f849dc1678a7cda9c2ad63eebd906371d56b
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
Last updated: 2026-10-08
Suite: `natural-corpus` | Profile: `standard` | Campaign status: `observed-failures`

Tools: `jq` (jq-1.8.1), `tq` (tq 0.5.0 (TOON v4.1; jq target 1.8.x; revision unknown)), `yq` (yq (https://github.com/mikefarah/yq/) version v4.53.2)
Environment: `linux` / `x86_64`, AMD Ryzen 7 7700 8-Core Processor, 16 logical CPUs, 61.9 GiB RAM; kernel `Linux 7.2.4-ogc3.1.fc44.x86_64 #1 SMP PREEMPT_DYNAMIC Sun Sep 13 01:41:06 UTC 2026`; compiler profile `release-benchmark`

Peak RSS is authoritative only after the campaign RSS preflight passes. Missing or invalid values are `-`, never estimates. RSS collector provenance (outside measurement tables): `linux-wait4`.
Measurement method (outside measurement tables): `tq-bench` native measurement: RSS scope `wait4 child lifetime including pre exec waited descendants and threads; sampled worker process group`; residual RSS floor `2.6 MiB` retained, not subtracted; observed control excess `0.9 ms`; primary timing includes process-group RSS sampling.

Compare columns with the same input format to isolate tool differences. Missing adapters have `-` measurement cells and `not recorded` outcome/detail cells.

Timing rows show numeric medians followed by compact MAD / p95 / range rows, with one decimal place and a unit in each measurement cell. CPU, throughput, and output cells use the captured summary values.

### usgs-all-day

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tq stream mode requires TOON or JSON event input; YAML is document-at-a-time.`, `yq rejects the catalog jq stream-event invocation.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 7.0 ms | - | - | 14.6 ms | - | 15.8 ms |
| Wall time dispersion | 0.1 ms / 7.0 ms / 6.9-7.0 ms | - | - | 0.1 ms / 15.5 ms / 14.4-15.5 ms | - | 0.3 ms / 16.1 ms / 15.0-16.1 ms |
| First output | 7.0 ms | - | - | 14.6 ms | - | 15.8 ms |
| First output dispersion | 0.1 ms / 7.0 ms / 6.9-7.0 ms | - | - | 0.1 ms / 15.5 ms / 14.4-15.5 ms | - | 0.1 ms / 15.9 ms / 14.9-15.9 ms |
| User CPU | 6.7 ms | - | - | 13.4 ms | - | 13.9 ms |
| System CPU | 0.0 ms | - | - | 1.0 ms | - | 1.1 ms |
| Peak RSS | 2.7 MiB | - | - | 15.7 MiB | - | 15.6 MiB |
| Logical throughput | 34967.0 records/s | - | - | 16712.3 records/s | - | 15467.5 records/s |
| Physical throughput | 23.8 MiB/s | - | - | 11.4 MiB/s | - | 12.5 MiB/s |
| Output bytes | 46.0 B | - | - | 41.0 B | - | 41.0 B |
| Outcome | timed | unsupported | unsupported | timed | unsupported | timed |
| Details | none | yq rejects the catalog jq stream-event invocation. | yq rejects the catalog jq stream-event invocation. | none | tq stream mode requires TOON or JSON event input; YAML is document-at-a-time. | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | not timed | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-hour

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tq stream mode requires TOON or JSON event input; YAML is document-at-a-time.`, `yq rejects the catalog jq stream-event invocation.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 1.4 ms | - | - | 2.3 ms | - | 2.3 ms |
| Wall time dispersion | 0.0 ms / 1.5 ms / 1.3-1.5 ms | - | - | 0.0 ms / 2.3 ms / 2.1-2.3 ms | - | 0.0 ms / 2.3 ms / 2.2-2.3 ms |
| First output | 1.4 ms | - | - | 2.3 ms | - | 2.3 ms |
| First output dispersion | 0.0 ms / 1.5 ms / 1.3-1.5 ms | - | - | 0.0 ms / 2.3 ms / 2.1-2.3 ms | - | 0.0 ms / 2.3 ms / 2.2-2.3 ms |
| User CPU | 1.2 ms | - | - | 1.1 ms | - | 1.1 ms |
| System CPU | 0.0 ms | - | - | 1.1 ms | - | 1.1 ms |
| Peak RSS | 2.7 MiB | - | - | 15.5 MiB | - | 15.6 MiB |
| Logical throughput | 5185.2 records/s | - | - | 3070.2 records/s | - | 3100.1 records/s |
| Physical throughput | 3.8 MiB/s | - | - | 2.2 MiB/s | - | 2.6 MiB/s |
| Output bytes | 46.0 B | - | - | 41.0 B | - | 41.0 B |
| Outcome | timed | unsupported | unsupported | timed | unsupported | timed |
| Details | none | yq rejects the catalog jq stream-event invocation. | yq rejects the catalog jq stream-event invocation. | none | tq stream mode requires TOON or JSON event input; YAML is document-at-a-time. | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | not timed | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-month

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tq stream mode requires TOON or JSON event input; YAML is document-at-a-time.`, `yq rejects the catalog jq stream-event invocation.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 240.6 ms | - | - | 557.6 ms | - | 600.8 ms |
| Wall time dispersion | 1.6 ms / 242.1 ms / 236.2-242.1 ms | - | - | 0.5 ms / 563.0 ms / 557.1-563.0 ms | - | 15.9 ms / 616.7 ms / 580.6-616.7 ms |
| First output | 240.6 ms | - | - | 557.6 ms | - | 600.8 ms |
| First output dispersion | 1.6 ms / 242.1 ms / 236.0-242.1 ms | - | - | 0.7 ms / 563.0 ms / 556.9-563.0 ms | - | 15.7 ms / 616.5 ms / 580.2-616.5 ms |
| User CPU | 238.0 ms | - | - | 554.6 ms | - | 596.0 ms |
| System CPU | 2.0 ms | - | - | 1.9 ms | - | 3.0 ms |
| Peak RSS | 2.7 MiB | - | - | 15.3 MiB | - | 15.5 MiB |
| Logical throughput | 44860.9 records/s | - | - | 19354.7 records/s | - | 17963.3 records/s |
| Physical throughput | 30.4 MiB/s | - | - | 13.1 MiB/s | - | 14.4 MiB/s |
| Output bytes | 46.0 B | - | - | 41.0 B | - | 41.0 B |
| Outcome | timed | unsupported | unsupported | timed | unsupported | timed |
| Details | none | yq rejects the catalog jq stream-event invocation. | yq rejects the catalog jq stream-event invocation. | none | tq stream mode requires TOON or JSON event input; YAML is document-at-a-time. | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | not timed | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |

### usgs-all-week

`-` denotes no valid comparable measurement, not zero. Missing, unavailable, unsupported, failed, unmeasured, and non-comparable measurements are excluded from rankings; outcome classifications and diagnostics remain in the rows below. Applicable exclusions or failures: `tq stream mode requires TOON or JSON event input; YAML is document-at-a-time.`, `yq rejects the catalog jq stream-event invocation.`.

| Metric | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Wall time | 51.2 ms | - | - | 116.3 ms | - | 122.2 ms |
| Wall time dispersion | 0.1 ms / 51.4 ms / 51.0-51.4 ms | - | - | 0.1 ms / 117.4 ms / 116.2-117.4 ms | - | 1.0 ms / 123.2 ms / 121.1-123.2 ms |
| First output | 51.2 ms | - | - | 116.0 ms | - | 122.2 ms |
| First output dispersion | 0.1 ms / 51.4 ms / 51.0-51.4 ms | - | - | 0.0 ms / 117.0 ms / 115.9-117.0 ms | - | 1.0 ms / 123.2 ms / 121.1-123.2 ms |
| User CPU | 49.9 ms | - | - | 115.0 ms | - | 119.8 ms |
| System CPU | 1.0 ms | - | - | 2.0 ms | - | 2.0 ms |
| Peak RSS | 2.7 MiB | - | - | 15.8 MiB | - | 15.3 MiB |
| Logical throughput | 43628.6 records/s | - | - | 19214.1 records/s | - | 18274.6 records/s |
| Physical throughput | 29.6 MiB/s | - | - | 13.0 MiB/s | - | 14.7 MiB/s |
| Output bytes | 46.0 B | - | - | 41.0 B | - | 41.0 B |
| Outcome | timed | unsupported | unsupported | timed | unsupported | timed |
| Details | none | yq rejects the catalog jq stream-event invocation. | yq rejects the catalog jq stream-event invocation. | none | tq stream mode requires TOON or JSON event input; YAML is document-at-a-time. | none |
| Samples (warmups) | 3 measured (1 warmup) | not timed | not timed | 3 measured (1 warmup) | not timed | 3 measured (1 warmup) |
| Comparison view | same input | same input | same input | same input | same input | native input |
<!-- benchmark-results:end -->
