---
type: Report
title: "jq to replace text directly on file (like sed -i)"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 19."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 19: jq to replace text directly on file (like sed -i)

[Stack Overflow question](https://stackoverflow.com/questions/36565295/jq-to-replace-text-directly-on-file-like-sed-i) · [selected answer](https://stackoverflow.com/a/36577521)

Rank: 19 · Question score: 183

Fixture: [source data](../../../tests/stack-overflow/19-jq-to-replace-text-directly-on-file-like-sed-i.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.Actions[] | .properties.other`

```json
{
  "Actions": [
    {
      "value": "1",
      "properties": {
        "name": "abc",
        "age": "2",
        "other": "test1"
      }
    },
    {
      "value": "2",
      "properties": {
        "name": "def",
        "age": "3",
        "other": "test2"
      }
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
| jq | JSON | timed | 3.050 | +0.0% | 2.67 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.088 | +66.8% | 15.14 (darwin-wait4) | +466.7% | 30 (+1 warmup) |
| tq | TOON | timed | 2.692 | -11.7% | 3.98 (darwin-wait4) | +49.1% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.Actions[] | .properties.other'`

```text
"test1"
"test2"
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.Actions[] | .properties.other'`

```text
"test1"
"test2"
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.Actions[] | .properties.other'`

```text
test1
test2
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 8 | 6 | -2 | -25.00% |
| `o200k_base` | Expanded | 8 | 6 | -2 | -25.00% |
| `cl100k_base` | Compact | 8 | 6 | -2 | -25.00% |
| `cl100k_base` | Expanded | 8 | 6 | -2 | -25.00% |
<!-- STACK_OVERFLOW_RESULTS_END -->
