---
type: Report
title: "Numeric reduction"
description: "Measures a blocking sum across all feature magnitudes."
workload: benchmark.numeric-reduction
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Numeric reduction

## What this measures

The query reduces non-null feature magnitudes into one sum. It keeps an
accumulator while consuming the complete feature stream, so this is a blocking
reduction rather than an early-result projection.

## Why it matters

Totals, scores, and counters are common data-processing jobs. Their cost grows
with the number of values even when the final output is only one number.

## Input and output

The input is a natural snapshot whose features may have missing magnitudes. The
output is one numeric sum, with null magnitudes omitted. It is not one output
per feature.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
