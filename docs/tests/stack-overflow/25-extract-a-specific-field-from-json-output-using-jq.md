---
type: Report
title: "Extract a specific field from JSON output using jq"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 25."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 25: Extract a specific field from JSON output using jq

[Stack Overflow question](https://stackoverflow.com/questions/39228500/extract-a-specific-field-from-json-output-using-jq) · [selected answer](https://stackoverflow.com/a/39228718)

Rank: 25 · Question score: 158

Fixture: [source data](../../../tests/stack-overflow/25-extract-a-specific-field-from-json-output-using-jq.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.example["sub-example"][] | .name`

```json
{
  "example": {
    "sub-example": [
      {
        "name": "123-345",
        "tag": 100
      },
      {
        "name": "234-456",
        "tag": 100
      },
      {
        "name": "4a7-a07a5",
        "tag": 100
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
| jq | JSON | timed | 3.106 | +0.0% | 2.67 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.044 | +62.4% | 15.39 (darwin-wait4) | +476.0% | 30 (+1 warmup) |
| tq | TOON | timed | 2.898 | -6.7% | 4.05 (darwin-wait4) | +51.5% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.example["sub-example"][] | .name'`

```text
"123-345"
"234-456"
"4a7-a07a5"
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.example["sub-example"][] | .name'`

```text
"123-345"
"234-456"
"4a7-a07a5"
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.example["sub-example"][] | .name'`

```text
123-345
234-456
4a7-a07a5
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 19 | 16 | -3 | -15.79% |
| `o200k_base` | Expanded | 19 | 16 | -3 | -15.79% |
| `cl100k_base` | Compact | 19 | 16 | -3 | -15.79% |
| `cl100k_base` | Expanded | 19 | 16 | -3 | -15.79% |
<!-- STACK_OVERFLOW_RESULTS_END -->
