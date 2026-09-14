---
type: Report
title: "JQ: Select multiple conditions"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 10."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
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
Status: `observed-failures`
Environment: `macos` / `aarch64` / `Apple M4 Pro` / 14 logical CPUs  
Tools: `jq` jq-1.8.2; `yq` yq (https://github.com/mikefarah/yq/) version v4.53.2; `tq` tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb)  
Memory evidence: authoritative peak RSS from the native collector; sampled-group RSS is not used as a substitute.  

Output profile: `structured`.


| Tool | Output | Status | Wall (ms) | Δ vs jq | Peak RSS (MiB, source) | RSS Δ vs jq | Samples |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| jq | JSON | timed | 3.278 | +0.0% | 2.70 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.269 | +60.7% | 15.50 (darwin-wait4) | +473.4% | 30 (+1 warmup) |
| tq | TOON | timed | 2.590 | -21.0% | 4.59 (darwin-wait4) | +69.9% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.[] | select((.processedBarsVolume <= 5) and .processedBars > 0)'`

```text
{"processedBarsVolume":4,"processedBars":2}
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.[] | select((.processedBarsVolume <= 5) and .processedBars > 0)'`

```text
{
  "processedBarsVolume": 4,
  "processedBars": 2
}
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.[] | select((.processedBarsVolume <= 5) and .processedBars > 0)'`

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
