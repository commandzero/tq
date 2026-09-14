---
type: Report
title: "How to count items in JSON object using command line?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 06."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
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
Status: `observed-failures`
Environment: `macos` / `aarch64` / `Apple M4 Pro` / 14 logical CPUs  
Tools: `jq` jq-1.8.2; `yq` yq (https://github.com/mikefarah/yq/) version v4.53.2; `tq` tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb)  
Memory evidence: authoritative peak RSS from the native collector; sampled-group RSS is not used as a substitute.  

Output profile: `structured`.


| Tool | Output | Status | Wall (ms) | Δ vs jq | Peak RSS (MiB, source) | RSS Δ vs jq | Samples |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| jq | JSON | timed | 3.071 | +0.0% | 2.69 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 4.965 | +61.6% | 15.14 (darwin-wait4) | +463.4% | 30 (+1 warmup) |
| tq | TOON | timed | 2.523 | -17.8% | 4.12 (darwin-wait4) | +53.5% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c length`

```text
2
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq length`

```text
2
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json length`

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
