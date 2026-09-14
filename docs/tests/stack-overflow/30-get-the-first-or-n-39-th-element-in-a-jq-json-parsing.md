---
type: Report
title: "get the first (or n'th) element in a jq json parsing"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 30."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 30: get the first (or n'th) element in a jq json parsing

[Stack Overflow question](https://stackoverflow.com/questions/38500363/get-the-first-or-nth-element-in-a-jq-json-parsing) · [selected answer](https://stackoverflow.com/a/38500610)

Rank: 30 · Question score: 128

Fixture: [source data](../../../tests/stack-overflow/30-get-the-first-or-n-39-th-element-in-a-jq-json-parsing.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.[0]`

```json
[
  {
    "a": "x"
  },
  {
    "a": "y"
  },
  {
    "a": "z"
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
| jq | JSON | timed | 3.409 | +0.0% | 2.69 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.013 | +47.1% | 15.41 (darwin-wait4) | +473.3% | 30 (+1 warmup) |
| tq | TOON | timed | 2.672 | -21.6% | 4.19 (darwin-wait4) | +55.8% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.[0]'`

```text
{"a":"x"}
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.[0]'`

```text
{
  "a": "x"
}
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.[0]'`

```text
a: x
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 5 | 4 | -1 | -20.00% |
| `o200k_base` | Expanded | 9 | 4 | -5 | -55.56% |
| `cl100k_base` | Compact | 5 | 4 | -1 | -20.00% |
| `cl100k_base` | Expanded | 9 | 4 | -5 | -55.56% |
<!-- STACK_OVERFLOW_RESULTS_END -->
