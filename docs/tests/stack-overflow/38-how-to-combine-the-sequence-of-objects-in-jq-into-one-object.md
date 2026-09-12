---
type: Report
title: "How to combine the sequence of objects in jq into one object?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 38."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 38: How to combine the sequence of objects in jq into one object?

[Stack Overflow question](https://stackoverflow.com/questions/34477547/how-to-combine-the-sequence-of-objects-in-jq-into-one-object) · [selected answer](https://stackoverflow.com/a/34477713)

Rank: 38 · Question score: 109

Fixture: [source data](../../../tests/stack-overflow/38-how-to-combine-the-sequence-of-objects-in-jq-into-one-object.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `map(.a)`

```json
[
  {
    "a": "green",
    "b": "white"
  },
  {
    "a": "red",
    "c": "purple"
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
| jq | JSON | timed | 197.365 | +0.0% | 2.61 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 177.424 | -10.1% | 15.91 (bsd-time-l) | +509.6% | 30 (+1 warmup) |
| tq | TOON | timed | 142.468 | -27.8% | 4.11 (bsd-time-l) | +57.5% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
["green","red"]
```

#### Expanded JSON (`jq`)

```text
[
  "green",
  "red"
]
```

#### TOON (`tq -o toon`)

```text
[2]: green,red
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 5 | 7 | +2 | +40.00% |
| `o200k_base` | Expanded | 10 | 7 | -3 | -30.00% |
| `cl100k_base` | Compact | 5 | 7 | +2 | +40.00% |
| `cl100k_base` | Expanded | 10 | 7 | -3 | -30.00% |
<!-- STACK_OVERFLOW_RESULTS_END -->
