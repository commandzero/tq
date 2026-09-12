---
type: Report
title: "Using jq to parse and display multiple fields in a json serially"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 04."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 04: Using jq to parse and display multiple fields in a json serially

[Stack Overflow question](https://stackoverflow.com/questions/28164849/using-jq-to-parse-and-display-multiple-fields-in-a-json-serially) · [selected answer](https://stackoverflow.com/a/31418194)

Rank: 4 · Question score: 549

Fixture: [source data](../../../tests/stack-overflow/04-using-jq-to-parse-and-display-multiple-fields-in-a-json-seri.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.users[] | "\(.first) \(.last)"`

```json
{
  "users": [
    {
      "first": "Stevie",
      "last": "Wonder"
    },
    {
      "first": "Michael",
      "last": "Jackson"
    }
  ]
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
| jq | JSON | timed | 185.745 | +0.0% | 2.59 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 189.137 | +1.8% | 15.47 (bsd-time-l) | +496.4% | 30 (+1 warmup) |
| tq | TOON | timed | 174.624 | -6.0% | 4.36 (bsd-time-l) | +68.1% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
"Stevie Wonder"
"Michael Jackson"
```

#### Expanded JSON (`jq`)

```text
"Stevie Wonder"
"Michael Jackson"
```

#### TOON (`tq -o toon`)

```text
Stevie Wonder
Michael Jackson
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 9 | 7 | -2 | -22.22% |
| `o200k_base` | Expanded | 9 | 7 | -2 | -22.22% |
| `cl100k_base` | Compact | 10 | 8 | -2 | -20.00% |
| `cl100k_base` | Expanded | 10 | 8 | -2 | -20.00% |
<!-- STACK_OVERFLOW_RESULTS_END -->
