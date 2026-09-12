---
type: Report
title: "How to merge 2 JSON objects from 2 files using jq?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 12."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 12: How to merge 2 JSON objects from 2 files using jq?

[Stack Overflow question](https://stackoverflow.com/questions/19529688/how-to-merge-2-json-objects-from-2-files-using-jq) · [selected answer](https://stackoverflow.com/a/24904276)

Rank: 12 · Question score: 266

Fixture: [source data](../../../tests/stack-overflow/12-how-to-merge-2-json-objects-from-2-files-using-jq.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.[] | .value`

```json
[
  {
    "value1": 200,
    "value": {
      "aaa": {
        "value1": "v1"
      },
      "bbb": {
        "value1": "v1"
      }
    }
  },
  {
    "status": 200,
    "value": {
      "aaa": {
        "value3": "v3"
      },
      "ddd": {
        "value3": "v3"
      }
    }
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
| jq | JSON | timed | 93.765 | +0.0% | 2.61 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 106.087 | +13.1% | 15.89 (bsd-time-l) | +509.0% | 30 (+1 warmup) |
| tq | TOON | timed | 85.016 | -9.3% | 3.97 (bsd-time-l) | +52.1% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
{"aaa":{"value1":"v1"},"bbb":{"value1":"v1"}}
{"aaa":{"value3":"v3"},"ddd":{"value3":"v3"}}
```

#### Expanded JSON (`jq`)

```text
{
  "aaa": {
    "value1": "v1"
  },
  "bbb": {
    "value1": "v1"
  }
}
{
  "aaa": {
    "value3": "v3"
  },
  "ddd": {
    "value3": "v3"
  }
}
```

#### TOON (`tq -o toon`)

```text
aaa:
  value1: v1
bbb:
  value1: v1
aaa:
  value3: v3
ddd:
  value3: v3
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 38 | 36 | -2 | -5.26% |
| `o200k_base` | Expanded | 68 | 36 | -32 | -47.06% |
| `cl100k_base` | Compact | 34 | 36 | +2 | +5.88% |
| `cl100k_base` | Expanded | 68 | 36 | -32 | -47.06% |
<!-- STACK_OVERFLOW_RESULTS_END -->
