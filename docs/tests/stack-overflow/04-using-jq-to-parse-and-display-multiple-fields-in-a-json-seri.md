---
type: Report
title: "Using jq to parse and display multiple fields in a json serially"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 04."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 04: Using jq to parse and display multiple fields in a json serially

[Stack Overflow question](https://stackoverflow.com/questions/28164849/using-jq-to-parse-and-display-multiple-fields-in-a-json-serially) · [selected answer](https://stackoverflow.com/a/31418194)

Rank: 4 · Question score: 549

Fixture: [source data](../../../tests/stack-overflow/04-using-jq-to-parse-and-display-multiple-fields-in-a-json-seri.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.users[] | "\(.first) \(.last)"`

```json
{
  "users": [
    {
      "first": "Stevie",
      "last": "Wonder"
    },
    {
      "first": "Michael",
      "last": "Jackson"
    }
  ]
}
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
| jq | JSON | timed | 3.135 | +0.0% | 2.69 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.301 | +69.1% | 15.19 (darwin-wait4) | +465.1% | 30 (+1 warmup) |
| tq | TOON | timed | 2.640 | -15.8% | 4.42 (darwin-wait4) | +64.5% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.users[] | "\(.first) \(.last)"'`

```text
"Stevie Wonder"
"Michael Jackson"
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.users[] | "\(.first) \(.last)"'`

```text
"Stevie Wonder"
"Michael Jackson"
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.users[] | "\(.first) \(.last)"'`

```text
Stevie Wonder
Michael Jackson
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 9 | 7 | -2 | -22.22% |
| `o200k_base` | Expanded | 9 | 7 | -2 | -22.22% |
| `cl100k_base` | Compact | 10 | 8 | -2 | -20.00% |
| `cl100k_base` | Expanded | 10 | 8 | -2 | -20.00% |
<!-- STACK_OVERFLOW_RESULTS_END -->
