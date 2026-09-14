---
type: Report
title: "CSV formatting"
description: "Measures formatting selected feature fields as CSV rows."
workload: benchmark.format-csv
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# CSV formatting

## What this measures

The query selects `id`, `place`, and `mag` for each feature and applies `@csv`.
It measures quoting, delimiter handling, and text output for repeated rows.

## Why it matters

CSV remains a practical interchange format for spreadsheets and simple data
loads. Correct quoting matters when fields contain commas or special text.

## Input and output

The input is a natural feature snapshot. The output is one CSV string per
feature containing the three selected fields, not JSON arrays or feature
objects.

The yq adapter forces quoted string fields to match jq CSV text. It emits an empty magnitude field for null values.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
