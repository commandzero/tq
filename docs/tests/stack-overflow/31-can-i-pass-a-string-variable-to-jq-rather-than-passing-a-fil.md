---
type: Report
title: "Can I pass a string variable to jq rather than passing a file?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 31."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 31: Can I pass a string variable to jq rather than passing a file?

[Stack Overflow question](https://stackoverflow.com/questions/47105490/can-i-pass-a-string-variable-to-jq-rather-than-passing-a-file) · [selected answer](https://stackoverflow.com/a/47105805)

Rank: 31 · Question score: 127

Fixture: [source data](../../../tests/stack-overflow/31-can-i-pass-a-string-variable-to-jq-rather-than-passing-a-fil.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.key`

```json
{
  "key": "value"
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
| jq | JSON | timed | 82.047 | +0.0% | 2.59 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 78.377 | -4.5% | 15.42 (bsd-time-l) | +494.6% | 30 (+1 warmup) |
| tq | TOON | timed | 81.379 | -0.8% | 3.94 (bsd-time-l) | +51.8% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
"value"
```

#### Expanded JSON (`jq`)

```text
"value"
```

#### TOON (`tq -o toon`)

```text
value
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 2 | 2 | +0 | +0.00% |
| `o200k_base` | Expanded | 2 | 2 | +0 | +0.00% |
| `cl100k_base` | Compact | 2 | 2 | +0 | +0.00% |
| `cl100k_base` | Expanded | 2 | 2 | +0 | +0.00% |
<!-- STACK_OVERFLOW_RESULTS_END -->
