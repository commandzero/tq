---
type: Report
title: "Regular-expression testing"
description: "Measures Unicode-aware pattern testing over feature place names."
workload: benchmark.regex-test
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Regular-expression testing

## What this measures

The query tests each string place value against `^[A-Z]` and counts the
matches. It combines string filtering, a regular expression, and a numeric
reduction.

## Why it matters

Pattern checks sit inside log filters, validation rules, and text cleanup jobs.
This case measures the work of testing every candidate rather than finding one
match and stopping.

## Input and output

The input is a natural feature snapshot with place strings and other values.
The output is one integer containing the number of strings that start with an
ASCII uppercase letter.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
