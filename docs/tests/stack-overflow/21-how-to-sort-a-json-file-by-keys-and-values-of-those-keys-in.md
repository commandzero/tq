---
type: Report
title: "How to sort a json file by keys and values of those keys in jq"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 21."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
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
Status: `observed-failures`
Environment: `macos` / `aarch64` / `Apple M4 Pro` / 14 logical CPUs  
Tools: `jq` jq-1.8.2; `yq` yq (https://github.com/mikefarah/yq/) version v4.53.2; `tq` tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb)  
Memory evidence: authoritative peak RSS from the native collector; sampled-group RSS is not used as a substitute.  

Output profile: `structured`.


| Tool | Output | Status | Wall (ms) | Δ vs jq | Peak RSS (MiB, source) | RSS Δ vs jq | Samples |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| jq | JSON | timed | 3.122 | +0.0% | 2.69 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.071 | +62.5% | 15.50 (darwin-wait4) | +476.7% | 30 (+1 warmup) |
| tq | TOON | timed | 2.655 | -14.9% | 4.31 (darwin-wait4) | +60.5% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.components.rows | sort_by(.id)'`

```text
[{"id":"CHARTS","name":"Charts"},{"id":"kjalajsdjf","name":"Value1"}]
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.components.rows | sort_by(.id)'`

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

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.components.rows | sort_by(.id)'`

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
