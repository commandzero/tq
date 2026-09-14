---
type: Report
title: "Shell quoting"
description: "Measures shell-safe formatting of selected feature fields."
workload: benchmark.format-shell
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Shell quoting

## What this measures

The query selects `id`, `place`, and `mag` for each feature and applies `@sh`.
It exercises quoting and escaping for values that may contain shell-sensitive
characters.

## Why it matters

Generated command arguments need quoting that preserves data instead of letting
the shell interpret it. This is formatting work, not permission to execute the
result.

## Input and output

The input is a natural feature snapshot. The output is one shell-quoted string
per feature containing the selected fields, not a command and not the original
array.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
