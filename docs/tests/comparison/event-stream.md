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
<!-- benchmark-results:end -->
