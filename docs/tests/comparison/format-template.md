---
type: Report
title: "Templated URI output"
description: "Measures building and URI-escaping a query-string template per feature."
workload: benchmark.format-template
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Templated URI output

## What this measures

The query builds `id=...&place=...` from each feature and applies URI escaping
to the template. It combines interpolation with text formatting.

## Why it matters

Query strings and small links are often assembled at the edge of a data
pipeline. Escaping each interpolated value is part of producing safe output.

## Input and output

The input is a natural feature snapshot. The output is one URI-escaped query
string per feature with `id` and `place` fields, not the source feature.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
