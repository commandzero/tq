---
type: Report
title: "Select objects based on value of variable in object using jq"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 03."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 03: Select objects based on value of variable in object using jq

[Stack Overflow question](https://stackoverflow.com/questions/18592173/select-objects-based-on-value-of-variable-in-object-using-jq) · [selected answer](https://stackoverflow.com/a/18608100)

Rank: 3 · Question score: 556

Fixture: [source data](../../../tests/stack-overflow/03-select-objects-based-on-value-of-variable-in-object-using-jq.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.[] | select(.location == "Stockholm") | .name`

```json
{
  "FOO": {
    "name": "Donald",
    "location": "Stockholm"
  },
  "BAR": {
    "name": "Walt",
    "location": "Stockholm"
  },
  "BAZ": {
    "name": "Jack",
    "location": "Whereever"
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
| jq | JSON | timed | 180.380 | +0.0% | 2.61 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 181.253 | +0.5% | 15.50 (bsd-time-l) | +494.0% | 30 (+1 warmup) |
| tq | TOON | timed | 168.706 | -6.5% | 4.55 (bsd-time-l) | +74.3% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
"Donald"
"Walt"
```

#### Expanded JSON (`jq`)

```text
"Donald"
"Walt"
```

#### TOON (`tq -o toon`)

```text
Donald
Walt
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 6 | 5 | -1 | -16.67% |
| `o200k_base` | Expanded | 6 | 5 | -1 | -16.67% |
| `cl100k_base` | Compact | 6 | 5 | -1 | -16.67% |
| `cl100k_base` | Expanded | 6 | 5 | -1 | -16.67% |
<!-- STACK_OVERFLOW_RESULTS_END -->
