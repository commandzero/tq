---
type: Report
title: "Path update"
description: "Measures updating one nested object while returning the full document."
workload: benchmark.path-update
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Path update

## What this measures

The update query adds `benchmarked: true` inside `.metadata` and emits the
complete document. It exercises path focus, object update, and full-document
serialization.

## Why it matters

Enriching records in place is a normal ETL operation. The work includes both
finding a nested path and carrying the untouched parts of the document through.

## Input and output

The input is a natural snapshot with a metadata object. The output is the whole
snapshot with one metadata field added, not just the changed metadata value.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
