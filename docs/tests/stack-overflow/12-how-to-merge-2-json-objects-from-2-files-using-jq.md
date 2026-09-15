---
type: Report
title: "How to merge 2 JSON objects from 2 files using jq?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 12."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
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
Status: `observed-failures`
Environment: `macos` / `aarch64` / `Apple M4 Pro` / 14 logical CPUs  
Tools: `jq` jq-1.8.2; `yq` yq (https://github.com/mikefarah/yq/) version v4.53.2; `tq` tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb)  
Memory evidence: authoritative peak RSS from the native collector; sampled-group RSS is not used as a substitute.  

Output profile: `structured`.


| Tool | Output | Status | Wall (ms) | Δ vs jq | Peak RSS (MiB, source) | RSS Δ vs jq | Samples |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| jq | JSON | timed | 3.409 | +0.0% | 2.67 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.019 | +47.2% | 15.52 (darwin-wait4) | +480.7% | 30 (+1 warmup) |
| tq | TOON | timed | 2.716 | -20.3% | 4.06 (darwin-wait4) | +52.0% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.[] | .value'`

```text
{"aaa":{"value1":"v1"},"bbb":{"value1":"v1"}}
{"aaa":{"value3":"v3"},"ddd":{"value3":"v3"}}
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.[] | .value'`

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

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.[] | .value'`

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
