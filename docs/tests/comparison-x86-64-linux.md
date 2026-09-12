---
type: Report
title: "Linux jq manual compatibility summary"
description: "Historical Linux campaign totals and their limits."
generated: { by: codex/gpt-6-astra, at: 2026-09-12T01:46:15Z }
---

# Linux jq manual compatibility summary

This is the historical 905-case x86_64 GNU/Linux campaign, not a new run or
the current case inventory. See the [jq manual sections](jq-manual/index.md)
for per-section reference results and [reviewed disparities](../jq-compatibility-disparities.md)
for the platform-specific limits. Full process-output records are generated
locally and are not source fixtures.

## Results

| Verdict | Cases |
| --- | ---: |
| Match | 896 |
| Reviewed disparity | 9 |

| Independent output campaign | Matches | Cases |
| --- | ---: | ---: |
| Compact JSON | 867 | 876 |
| TOON | 876 | 876 |

JSON matches preserve ordered values, exact decimal values, and process
behavior while ignoring presentation whitespace and object key order.
Compact JSON additionally compares stdout bytes. Raw CLI contracts are checked
separately. Reviewed disparities are not exact matches.

The nine disparities concern Linux math rounding, out-of-range exponent
conversion, and the unsupported regex longest-match flag, including their
composed-call witnesses. Approval is limited to the recorded cases and
platform; it does not waive other inputs.

## Output size

These historical totals cover 714 successful, equivalent, single-result
examples. They compare default pretty JSON with standalone TOON, including
trailing newlines. Explicit sequence framing and error-only cases are excluded.

| Tokenizer | Pretty JSON tokens | TOON tokens | Tokens saved |
| --- | ---: | ---: | ---: |
| `o200k_base` | 7,156 | 4,677 | 2,479 |
| `cl100k_base` | 7,158 | 4,686 | 2,472 |

The manual is a correctness corpus, not a representative performance workload.
See the [workload comparisons](comparison/index.md) for execution measurements.
