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
<!-- benchmark-results:end -->
