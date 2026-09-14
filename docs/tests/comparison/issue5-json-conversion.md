---
type: Report
title: "JSON round trip"
description: "Measures converting metadata to JSON text and parsing it back."
workload: benchmark.issue5-json-conversion
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# JSON round trip

## What this measures

The query serializes `.metadata` with `tojson`, parses it with `fromjson`, and
returns the resulting object's length. It includes both text conversion steps.

## Why it matters

Applications cross JSON text boundaries when they log, cache, or hand data to
another process. A round trip can cost more than an in-memory field read.

## Input and output

The input is a metadata object from a natural snapshot. The output is one
integer for the number of members after the round trip, not the JSON text.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
