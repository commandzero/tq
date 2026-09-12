---
type: Report
title: "How to sort a json file by keys and values of those keys in jq"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 21."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 21: How to sort a json file by keys and values of those keys in jq

[Stack Overflow question](https://stackoverflow.com/questions/30331504/how-to-sort-a-json-file-by-keys-and-values-of-those-keys-in-jq) · [selected answer](https://stackoverflow.com/a/58011343)

Rank: 21 · Question score: 176

Fixture: [source data](../../../tests/stack-overflow/21-how-to-sort-a-json-file-by-keys-and-values-of-those-keys-in-.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.components.rows | sort_by(.id)`

```json
{
  "components": {
    "rows": [
      {
        "id": "kjalajsdjf",
        "name": "Value1"
      },
      {
        "id": "CHARTS",
        "name": "Charts"
      }
    ]
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
| jq | JSON | timed | 114.035 | +0.0% | 2.61 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 120.942 | +6.1% | 15.95 (bsd-time-l) | +511.4% | 30 (+1 warmup) |
| tq | TOON | timed | 125.697 | +10.2% | 4.14 (bsd-time-l) | +58.7% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
[{"id":"CHARTS","name":"Charts"},{"id":"kjalajsdjf","name":"Value1"}]
```

#### Expanded JSON (`jq`)

```text
[
  {
    "id": "CHARTS",
    "name": "Charts"
  },
  {
    "id": "kjalajsdjf",
    "name": "Value1"
  }
]
```

#### TOON (`tq -o toon`)

```text
[2]{id,name}:
  CHARTS,Charts
  kjalajsdjf,Value1
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 26 | 24 | -2 | -7.69% |
| `o200k_base` | Expanded | 45 | 24 | -21 | -46.67% |
| `cl100k_base` | Compact | 26 | 24 | -2 | -7.69% |
| `cl100k_base` | Expanded | 45 | 24 | -21 | -46.67% |
<!-- STACK_OVERFLOW_RESULTS_END -->
