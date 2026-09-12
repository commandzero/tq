---
type: Report
title: "How to use jq when the variable has reserved characters?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 45."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 45: How to use jq when the variable has reserved characters?

[Stack Overflow question](https://stackoverflow.com/questions/37018393/how-to-use-jq-when-the-variable-has-reserved-characters) · [selected answer](https://stackoverflow.com/a/37018703)

Rank: 45 · Question score: 84

Fixture: [source data](../../../tests/stack-overflow/45-how-to-use-jq-when-the-variable-has-reserved-characters.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.USD.price`

```json
{
  "OPEN.BTC": {
    "volume24": 0.932166,
    "price": 0.09995,
    "updated": "2016-05-04T03:03:29.000Z"
  },
  "USD": {
    "price": 1
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
| jq | JSON | timed | 110.612 | +0.0% | 2.59 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 111.599 | +0.9% | 16.09 (bsd-time-l) | +520.5% | 30 (+1 warmup) |
| tq | TOON | timed | 123.668 | +11.8% | 3.98 (bsd-time-l) | +53.6% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
1
```

#### Expanded JSON (`jq`)

```text
1
```

#### TOON (`tq -o toon`)

```text
1
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 2 | 2 | +0 | +0.00% |
| `o200k_base` | Expanded | 2 | 2 | +0 | +0.00% |
| `cl100k_base` | Compact | 2 | 2 | +0 | +0.00% |
| `cl100k_base` | Expanded | 2 | 2 | +0 | +0.00% |
<!-- STACK_OVERFLOW_RESULTS_END -->
