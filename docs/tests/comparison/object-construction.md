---
type: Report
title: "Object construction"
description: "Measures building a lookup object with computed feature IDs as keys."
workload: benchmark.object-construction
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Object construction

## What this measures

The query turns the feature collection into an object whose keys come from
`.id` and whose values are magnitudes. It exercises computed keys, repeated
merges, and a blocking object result.

## Why it matters

Lookup maps are a common boundary between raw records and application code.
They also stress a different memory pattern from an output array.

## Input and output

The input is a natural feature snapshot with IDs and magnitudes. The output is
one object mapping each ID to its magnitude. It is not a list of key-value
records.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
