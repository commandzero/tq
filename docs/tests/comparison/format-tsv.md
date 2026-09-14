---
type: Report
title: "TSV formatting"
description: "Measures formatting selected feature fields as tab-separated rows."
workload: benchmark.format-tsv
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# TSV formatting

## What this measures

The query selects `id`, `place`, and `mag` for each feature and applies `@tsv`.
It measures tab escaping and text output across the complete feature stream.

## Why it matters

Tab-separated rows are easy to pipe into Unix tools and import into tables.
Escaping embedded tabs and control characters is part of the real work.

## Input and output

The input is a natural feature snapshot. The output is one TSV string per
feature containing the three selected fields, not a JSON representation.

The yq adapter converts null magnitudes to empty fields, matching jq TSV text. Numeric zero remains a numeric field.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
