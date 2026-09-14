---
type: Report
title: "Passing bash variable to jq"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 11."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 11: Passing bash variable to jq

[Stack Overflow question](https://stackoverflow.com/questions/40027395/passing-bash-variable-to-jq) · [selected answer](https://stackoverflow.com/a/40027637)

Rank: 11 · Question score: 281

Fixture: [source data](../../../tests/stack-overflow/11-passing-bash-variable-to-jq.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.resource[] | select(.username == "myemail@hotmail.com") | .id`

```json
{
  "resource": [
    {
      "username": "myemail@hotmail.com",
      "id": 42
    },
    {
      "username": "other@example.com",
      "id": 7
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
| jq | JSON | timed | 3.103 | +0.0% | 2.69 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.011 | +61.5% | 15.27 (darwin-wait4) | +468.0% | 30 (+1 warmup) |
| tq | TOON | timed | 3.029 | -2.4% | 4.53 (darwin-wait4) | +68.6% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.resource[] | select(.username == "myemail@hotmail.com") | .id'`

```text
42
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.resource[] | select(.username == "myemail@hotmail.com") | .id'`

```text
42
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.resource[] | select(.username == "myemail@hotmail.com") | .id'`

```text
42
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 2 | 2 | +0 | +0.00% |
| `o200k_base` | Expanded | 2 | 2 | +0 | +0.00% |
| `cl100k_base` | Compact | 2 | 2 | +0 | +0.00% |
| `cl100k_base` | Expanded | 2 | 2 | +0 | +0.00% |
<!-- STACK_OVERFLOW_RESULTS_END -->
