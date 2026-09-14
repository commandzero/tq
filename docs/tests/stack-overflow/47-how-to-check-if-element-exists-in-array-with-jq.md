---
type: Report
title: "How to check if element exists in array with jq"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 47."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 47: How to check if element exists in array with jq

[Stack Overflow question](https://stackoverflow.com/questions/43259563/how-to-check-if-element-exists-in-array-with-jq) · [selected answer](https://stackoverflow.com/a/43269105)

Rank: 47 · Question score: 82

Fixture: [source data](../../../tests/stack-overflow/47-how-to-check-if-element-exists-in-array-with-jq.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.fruit[] | select(. == "orange")`

```json
{
  "fruit": [
    "apple",
    "orange",
    "pomegranate",
    "apricot",
    "mango"
  ]
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
| jq | JSON | timed | 3.090 | +0.0% | 2.69 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.284 | +71.0% | 15.19 (darwin-wait4) | +465.1% | 30 (+1 warmup) |
| tq | TOON | timed | 2.683 | -13.2% | 4.45 (darwin-wait4) | +65.7% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.fruit[] | select(. == "orange")'`

```text
"orange"
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.fruit[] | select(. == "orange")'`

```text
"orange"
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.fruit[] | select(. == "orange")'`

```text
orange
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 3 | 2 | -1 | -33.33% |
| `o200k_base` | Expanded | 3 | 2 | -1 | -33.33% |
| `cl100k_base` | Compact | 3 | 2 | -1 | -33.33% |
| `cl100k_base` | Expanded | 3 | 2 | -1 | -33.33% |
<!-- STACK_OVERFLOW_RESULTS_END -->
