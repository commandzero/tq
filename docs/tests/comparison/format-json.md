---
type: Report
title: "JSON text formatting"
description: "Measures serializing each feature projection as compact JSON text."
workload: benchmark.format-json
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# JSON text formatting

## What this measures

The query projects `[id, properties]` for each feature and applies `@json`.
It measures repeated conversion from values to JSON text.

## Why it matters

Small JSON fragments are common message and logging boundaries. Serialization
cost can dominate when the query emits many compact records.

## Input and output

The input is a natural feature snapshot. The output is one JSON string per
feature containing its ID and properties, not an array of value pairs.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
