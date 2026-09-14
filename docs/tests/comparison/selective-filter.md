---
type: Report
title: "Selective filtering"
description: "Measures streaming selection and projection of matching features."
workload: benchmark.selective-filter
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Selective filtering

## What this measures

The query selects features with magnitude at least 2 and then projects their
IDs. It combines iteration, a numeric predicate, and a downstream field read.

## Why it matters

This is the shape of a useful event or records query: scan a collection, keep a
subset, and pass compact identifiers to the next command.

## Input and output

The input is a natural feature snapshot with numeric `properties.mag` values.
The output is the ordered sequence of IDs for matching features. It is not the
matching feature objects and not a packed array.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
