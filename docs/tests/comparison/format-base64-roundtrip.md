---
type: Report
title: "Base64 round trip"
description: "Measures Base64 encoding followed by decoding for each place string."
workload: benchmark.format-base64-roundtrip
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Base64 round trip

## What this measures

The query extracts each place string, applies `@base64`, and decodes it with
`@base64d`. It includes both conversions for every feature.

## Why it matters

Encoding is common when text crosses a binary-safe transport or storage field.
A round trip shows the cost of conversion even when the final value is
unchanged.

## Input and output

The input is a natural feature snapshot. The output is one decoded place string
per feature, so it should have the same semantic text as the selected input.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
