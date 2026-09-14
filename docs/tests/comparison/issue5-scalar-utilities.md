---
type: Report
title: "String and scalar utilities"
description: "Measures case conversion and Unicode scalar handling across place names."
workload: benchmark.issue5-scalar-utilities
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# String and scalar utilities

## What this measures

The query lowercases each string place, expands it to Unicode code points with
`explode`, counts those scalars, and adds the counts. It combines text
transformation with a numeric reduction.

## Why it matters

Text utilities often sit in normalization and search pipelines. Unicode-aware
length work is different from counting bytes, especially for non-ASCII data.

## Input and output

The input is a natural snapshot with place values. The output is one integer
containing the total code-point count after lowercasing, not the transformed
strings.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
