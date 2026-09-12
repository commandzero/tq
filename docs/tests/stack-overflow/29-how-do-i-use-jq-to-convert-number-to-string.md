---
type: Report
title: "How do I use jq to convert number to string?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 29."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 29: How do I use jq to convert number to string?

[Stack Overflow question](https://stackoverflow.com/questions/35365769/how-do-i-use-jq-to-convert-number-to-string) · [selected answer](https://stackoverflow.com/a/35365770)

Rank: 29 · Question score: 130

Fixture: [source data](../../../tests/stack-overflow/29-how-do-i-use-jq-to-convert-number-to-string.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.[] | .number | tostring`

```json
[
  {
    "number": 3,
    "string": "three"
  },
  {
    "number": 7,
    "string": "seven"
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
| jq | JSON | timed | 96.811 | +0.0% | 2.61 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 103.949 | +7.4% | 15.64 (bsd-time-l) | +499.4% | 30 (+1 warmup) |
| tq | TOON | timed | 90.383 | -6.6% | 4.33 (bsd-time-l) | +65.9% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
"3"
"7"
```

#### Expanded JSON (`jq`)

```text
"3"
"7"
```

#### TOON (`tq -o toon`)

```text
"3"
"7"
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 6 | 6 | +0 | +0.00% |
| `o200k_base` | Expanded | 6 | 6 | +0 | +0.00% |
| `cl100k_base` | Compact | 6 | 6 | +0 | +0.00% |
| `cl100k_base` | Expanded | 6 | 6 | +0 | +0.00% |
<!-- STACK_OVERFLOW_RESULTS_END -->
