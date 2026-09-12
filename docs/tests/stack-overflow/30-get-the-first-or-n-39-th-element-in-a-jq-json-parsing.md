---
type: Report
title: "get the first (or n'th) element in a jq json parsing"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 30."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 30: get the first (or n'th) element in a jq json parsing

[Stack Overflow question](https://stackoverflow.com/questions/38500363/get-the-first-or-nth-element-in-a-jq-json-parsing) · [selected answer](https://stackoverflow.com/a/38500610)

Rank: 30 · Question score: 128

Fixture: [source data](../../../tests/stack-overflow/30-get-the-first-or-n-39-th-element-in-a-jq-json-parsing.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.[0]`

```json
[
  {
    "a": "x"
  },
  {
    "a": "y"
  },
  {
    "a": "z"
  }
]
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
| jq | JSON | timed | 84.542 | +0.0% | 2.59 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 80.381 | -4.9% | 15.75 (bsd-time-l) | +507.2% | 30 (+1 warmup) |
| tq | TOON | timed | 84.212 | -0.4% | 4.05 (bsd-time-l) | +56.0% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
{"a":"x"}
```

#### Expanded JSON (`jq`)

```text
{
  "a": "x"
}
```

#### TOON (`tq -o toon`)

```text
a: x
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 5 | 4 | -1 | -20.00% |
| `o200k_base` | Expanded | 9 | 4 | -5 | -55.56% |
| `cl100k_base` | Compact | 5 | 4 | -1 | -20.00% |
| `cl100k_base` | Expanded | 9 | 4 | -5 | -55.56% |
<!-- STACK_OVERFLOW_RESULTS_END -->
