---
type: Report
title: "How do I keep colors when piping \"jq\" output to \"less\"?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 43."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 43: How do I keep colors when piping "jq" output to "less"?

[Stack Overflow question](https://stackoverflow.com/questions/62809196/how-do-i-keep-colors-when-piping-jq-output-to-less) · [selected answer](https://stackoverflow.com/a/62809275)

Rank: 43 · Question score: 92

Fixture: [source data](../../../tests/stack-overflow/43-how-do-i-keep-colors-when-piping-quot-jq-quot-output-to-quot.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.`

```json
{
  "name": "jq",
  "color": "terminal"
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
| jq | JSON | timed | 123.618 | +0.0% | 2.58 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 128.370 | +3.8% | 16.02 (bsd-time-l) | +521.2% | 30 (+1 warmup) |
| tq | TOON | timed | 122.550 | -0.9% | 3.62 (bsd-time-l) | +40.6% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
{"name":"jq","color":"terminal"}
```

#### Expanded JSON (`jq`)

```text
{
  "name": "jq",
  "color": "terminal"
}
```

#### TOON (`tq -o toon`)

```text
name: jq
color: terminal
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 9 | 8 | -1 | -11.11% |
| `o200k_base` | Expanded | 16 | 8 | -8 | -50.00% |
| `cl100k_base` | Compact | 9 | 8 | -1 | -11.11% |
| `cl100k_base` | Expanded | 16 | 8 | -8 | -50.00% |
<!-- STACK_OVERFLOW_RESULTS_END -->
