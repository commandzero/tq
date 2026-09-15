---
type: Report
title: "How do I update a single value in a json document using jq?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 15."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
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
Status: `observed-failures`
Environment: `macos` / `aarch64` / `Apple M4 Pro` / 14 logical CPUs  
Tools: `jq` jq-1.8.2; `yq` yq (https://github.com/mikefarah/yq/) version v4.53.2; `tq` tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb)  
Memory evidence: authoritative peak RSS from the native collector; sampled-group RSS is not used as a substitute.  

Output profile: `structured`.


| Tool | Output | Status | Wall (ms) | Δ vs jq | Peak RSS (MiB, source) | RSS Δ vs jq | Samples |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| jq | JSON | timed | 3.091 | +0.0% | 2.67 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.301 | +71.5% | 15.44 (darwin-wait4) | +477.8% | 30 (+1 warmup) |
| tq | TOON | timed | 2.788 | -9.8% | 4.14 (darwin-wait4) | +55.0% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c .shipping`

```text
{"local":true,"us":true,"us_rate":{"amount":"0.00","currency":"USD","symbol":"$"}}
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq .shipping`

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

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json .shipping`

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
