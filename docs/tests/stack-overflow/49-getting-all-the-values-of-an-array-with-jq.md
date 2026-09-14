---
type: Report
title: "getting all the values of an array with jq"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 49."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 49: getting all the values of an array with jq

[Stack Overflow question](https://stackoverflow.com/questions/45523425/getting-all-the-values-of-an-array-with-jq) · [selected answer](https://stackoverflow.com/a/45524015)

Rank: 49 · Question score: 80

Fixture: [source data](../../../tests/stack-overflow/49-getting-all-the-values-of-an-array-with-jq.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.response[] | select(has("text")) | .text`

```json
{
  "response": [
    {
      "text": "first"
    },
    {
      "status": 200
    },
    {
      "text": "second"
    }
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
| jq | JSON | timed | 3.099 | +0.0% | 2.69 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.138 | +65.8% | 15.27 (darwin-wait4) | +468.0% | 30 (+1 warmup) |
| tq | TOON | timed | 2.749 | -11.3% | 4.59 (darwin-wait4) | +70.9% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.response[] | select(has("text")) | .text'`

```text
"first"
"second"
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.response[] | select(has("text")) | .text'`

```text
"first"
"second"
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.response[] | select(has("text")) | .text'`

```text
first
second
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 6 | 4 | -2 | -33.33% |
| `o200k_base` | Expanded | 6 | 4 | -2 | -33.33% |
| `cl100k_base` | Compact | 6 | 4 | -2 | -33.33% |
| `cl100k_base` | Expanded | 6 | 4 | -2 | -33.33% |
<!-- STACK_OVERFLOW_RESULTS_END -->
