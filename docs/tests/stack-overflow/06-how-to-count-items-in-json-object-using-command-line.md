---
type: Report
title: "How to count items in JSON object using command line?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 06."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 06: How to count items in JSON object using command line?

[Stack Overflow question](https://stackoverflow.com/questions/21334348/how-to-count-items-in-json-object-using-command-line) · [selected answer](https://stackoverflow.com/a/21355442)

Rank: 6 · Question score: 474

Fixture: [source data](../../../tests/stack-overflow/06-how-to-count-items-in-json-object-using-command-line.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `length`

```json
[
  {
    "cid": 49,
    "l10n": "cent million"
  },
  {
    "cid": 50,
    "l10n": "100 millions"
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
| jq | JSON | timed | 189.464 | +0.0% | 2.58 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 178.229 | -5.9% | 15.36 (bsd-time-l) | +495.8% | 30 (+1 warmup) |
| tq | TOON | timed | 178.685 | -5.7% | 3.97 (bsd-time-l) | +53.9% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
2
```

#### Expanded JSON (`jq`)

```text
2
```

#### TOON (`tq -o toon`)

```text
2
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 2 | 2 | +0 | +0.00% |
| `o200k_base` | Expanded | 2 | 2 | +0 | +0.00% |
| `cl100k_base` | Compact | 2 | 2 | +0 | +0.00% |
| `cl100k_base` | Expanded | 2 | 2 | +0 | +0.00% |
<!-- STACK_OVERFLOW_RESULTS_END -->
