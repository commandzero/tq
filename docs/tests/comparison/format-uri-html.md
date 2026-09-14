---
type: Report
title: "HTML and URI escaping"
description: "Measures two text-escaping passes over feature place names."
workload: benchmark.format-uri-html
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# HTML and URI escaping

## What this measures

The query takes each place string through `@html` and then `@uri`. It exercises
escaping rules and repeated text allocation for every feature.

## Why it matters

Data often crosses an HTML or URL boundary after it leaves a JSON document.
Applying the transformations in sequence is common in report and request
generation.

## Input and output

The input is a natural feature snapshot. The output is one URI-encoded string
per place after HTML escaping, not the original place text or full feature.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
