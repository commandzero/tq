---
type: Report
title: "JQ: Select multiple conditions"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 10."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 10: JQ: Select multiple conditions

[Stack Overflow question](https://stackoverflow.com/questions/33057420/jq-select-multiple-conditions) · [selected answer](https://stackoverflow.com/a/33059058)

Rank: 10 · Question score: 301

Fixture: [source data](../../../tests/stack-overflow/10-jq-select-multiple-conditions.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.[] | select((.processedBarsVolume <= 5) and .processedBars > 0)`

```json
[
  {
    "processedBarsVolume": 4,
    "processedBars": 2
  },
  {
    "processedBarsVolume": 8,
    "processedBars": 3
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
| jq | JSON | timed | 154.255 | +0.0% | 2.59 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 124.186 | -19.5% | 16.03 (bsd-time-l) | +518.1% | 30 (+1 warmup) |
| tq | TOON | timed | 117.112 | -24.1% | 4.50 (bsd-time-l) | +73.5% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
{"processedBarsVolume":4,"processedBars":2}
```

#### Expanded JSON (`jq`)

```text
{
  "processedBarsVolume": 4,
  "processedBars": 2
}
```

#### TOON (`tq -o toon`)

```text
processedBarsVolume: 4
processedBars: 2
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 12 | 13 | +1 | +8.33% |
| `o200k_base` | Expanded | 19 | 13 | -6 | -31.58% |
| `cl100k_base` | Compact | 12 | 13 | +1 | +8.33% |
| `cl100k_base` | Expanded | 19 | 13 | -6 | -31.58% |
<!-- STACK_OVERFLOW_RESULTS_END -->
