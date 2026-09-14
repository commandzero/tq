---
type: Report
title: "Any-match predicate"
description: "Measures stopping a collection scan when one feature satisfies a predicate."
workload: benchmark.issue5-predicate
generated: { by: codex/gpt-5.6-luna, at: 2026-09-10T20:15:01Z }
---

# Any-match predicate

## What this measures

The query asks whether any feature has a magnitude at least zero. It exercises
predicate evaluation with a possible early exit from the feature stream.

## Why it matters

Existence checks are common in alerts and guards. Their cost depends on where
the first match occurs, so they differ from reductions that always scan all
values.

## Input and output

The input is a natural feature snapshot. The output is one boolean, `true` when
at least one feature meets the predicate and `false` otherwise.

## Results

<!-- benchmark-results:start -->
<!-- benchmark-results:end -->
