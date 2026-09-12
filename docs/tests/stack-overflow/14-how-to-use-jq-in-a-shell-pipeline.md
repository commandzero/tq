---
type: Report
title: "How to use `jq` in a shell pipeline?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 14."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 14: How to use `jq` in a shell pipeline?

[Stack Overflow question](https://stackoverflow.com/questions/33247228/how-to-use-jq-in-a-shell-pipeline) · [selected answer](https://stackoverflow.com/a/33247259)

Rank: 14 · Question score: 244

Fixture: [source data](../../../tests/stack-overflow/14-how-to-use-jq-in-a-shell-pipeline.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.`

```json
{
  "repos": [
    {
      "name": "jq",
      "stars": 1
    },
    {
      "name": "tq",
      "stars": 2
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
| jq | JSON | timed | 104.492 | +0.0% | 2.59 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 118.131 | +13.1% | 15.94 (bsd-time-l) | +514.5% | 30 (+1 warmup) |
| tq | TOON | timed | 108.825 | +4.1% | 3.78 (bsd-time-l) | +45.8% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
{"repos":[{"name":"jq","stars":1},{"name":"tq","stars":2}]}
```

#### Expanded JSON (`jq`)

```text
{
  "repos": [
    {
      "name": "jq",
      "stars": 1
    },
    {
      "name": "tq",
      "stars": 2
    }
  ]
}
```

#### TOON (`tq -o toon`)

```text
repos[2]{name,stars}:
  jq,1
  tq,2
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 22 | 19 | -3 | -13.64% |
| `o200k_base` | Expanded | 46 | 19 | -27 | -58.70% |
| `cl100k_base` | Compact | 21 | 19 | -2 | -9.52% |
| `cl100k_base` | Expanded | 46 | 19 | -27 | -58.70% |
<!-- STACK_OVERFLOW_RESULTS_END -->
