---
type: Report
title: "Scalar field extraction"
description: "Measures navigation from a document to one nested scalar."
workload: benchmark.scalar-extraction
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Scalar field extraction

## What this measures

The query `.metadata.count` parses a document, follows two object fields, and
returns one scalar. It is a small document-mode navigation case.

## Why it matters

Configuration checks and request handlers commonly need one field from a larger
payload. The result shows the cost of that narrow read rather than a full walk.

## Input and output

The input is a natural snapshot with a `metadata.count` field. The output is a
single scalar in the result sequence, not the surrounding metadata object or
the original document.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
