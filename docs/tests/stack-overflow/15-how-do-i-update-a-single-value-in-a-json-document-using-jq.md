---
type: Report
title: "How do I update a single value in a json document using jq?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 15."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 15: How do I update a single value in a json document using jq?

[Stack Overflow question](https://stackoverflow.com/questions/31034746/how-do-i-update-a-single-value-in-a-json-document-using-jq) · [selected answer](https://stackoverflow.com/a/31037640)

Rank: 15 · Question score: 230

Fixture: [source data](../../../tests/stack-overflow/15-how-do-i-update-a-single-value-in-a-json-document-using-jq.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.shipping`

```json
{
  "shipping": {
    "local": true,
    "us": true,
    "us_rate": {
      "amount": "0.00",
      "currency": "USD",
      "symbol": "$"
    }
  }
}
```

Scenario source registry: [jq questions sorted by votes](https://stackoverflow.com/questions/tagged/jq?tab=votes&pagesize=50)

## Results
<!-- STACK_OVERFLOW_RESULTS_START -->
Last updated: 2026-09-11
Status: `passed`
Environment: `macos` / `aarch64` / `Apple M4 Pro` / 14 logical CPUs  
Tools: `jq` jq-1.8.1; `yq` yq (https://github.com/mikefarah/yq/) version v4.53.2; `tq` tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown)  
Memory evidence: authoritative peak RSS from the recorded time implementation; sampled-group RSS is not used as a substitute.  

| Tool | Output | Status | Wall (ms) | Δ vs jq | Peak RSS (MiB, source) | RSS Δ vs jq | Samples |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| jq | JSON | timed | 91.764 | +0.0% | 2.58 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 96.553 | +5.2% | 15.86 (bsd-time-l) | +515.2% | 30 (+1 warmup) |
| tq | TOON | timed | 100.871 | +9.9% | 4.05 (bsd-time-l) | +57.0% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
{"local":true,"us":true,"us_rate":{"amount":"0.00","currency":"USD","symbol":"$"}}
```

#### Expanded JSON (`jq`)

```text
{
  "local": true,
  "us": true,
  "us_rate": {
    "amount": "0.00",
    "currency": "USD",
    "symbol": "$"
  }
}
```

#### TOON (`tq -o toon`)

```text
local: true
us: true
us_rate:
  amount: "0.00"
  currency: USD
  symbol: $
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 27 | 28 | +1 | +3.70% |
| `o200k_base` | Expanded | 44 | 28 | -16 | -36.36% |
| `cl100k_base` | Compact | 26 | 28 | +2 | +7.69% |
| `cl100k_base` | Expanded | 44 | 28 | -16 | -36.36% |
<!-- STACK_OVERFLOW_RESULTS_END -->
