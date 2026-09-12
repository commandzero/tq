---
type: Report
title: "Extract a specific field from JSON output using jq"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 25."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
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
Last updated: 2026-09-11
Status: `passed`
Environment: `macos` / `aarch64` / `Apple M4 Pro` / 14 logical CPUs  
Tools: `jq` jq-1.8.1; `yq` yq (https://github.com/mikefarah/yq/) version v4.53.2; `tq` tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown)  
Memory evidence: authoritative peak RSS from the recorded time implementation; sampled-group RSS is not used as a substitute.  

| Tool | Output | Status | Wall (ms) | Δ vs jq | Peak RSS (MiB, source) | RSS Δ vs jq | Samples |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| jq | JSON | timed | 100.617 | +0.0% | 2.58 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 100.880 | +0.3% | 15.64 (bsd-time-l) | +506.7% | 30 (+1 warmup) |
| tq | TOON | timed | 111.705 | +11.0% | 3.95 (bsd-time-l) | +53.3% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
"123-345"
"234-456"
"4a7-a07a5"
```

#### Expanded JSON (`jq`)

```text
"123-345"
"234-456"
"4a7-a07a5"
```

#### TOON (`tq -o toon`)

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
