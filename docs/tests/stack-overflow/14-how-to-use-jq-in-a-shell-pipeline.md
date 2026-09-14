---
type: Report
title: "How to use `jq` in a shell pipeline?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 14."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 14: How to use `jq` in a shell pipeline?

[Stack Overflow question](https://stackoverflow.com/questions/33247228/how-to-use-jq-in-a-shell-pipeline) · [selected answer](https://stackoverflow.com/a/33247259)

Rank: 14 · Question score: 244

Fixture: [source data](../../../tests/stack-overflow/14-how-to-use-jq-in-a-shell-pipeline.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.`

```json
{
  "repos": [
    {
      "name": "jq",
      "stars": 1
    },
    {
      "name": "tq",
      "stars": 2
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
| jq | JSON | timed | 3.095 | +0.0% | 2.67 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.130 | +65.7% | 15.45 (darwin-wait4) | +478.4% | 30 (+1 warmup) |
| tq | TOON | timed | 2.882 | -6.9% | 3.84 (darwin-wait4) | +43.9% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c .`

```text
{"repos":[{"name":"jq","stars":1},{"name":"tq","stars":2}]}
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq .`

```text
{
  "repos": [
    {
      "name": "jq",
      "stars": 1
    },
    {
      "name": "tq",
      "stars": 2
    }
  ]
}
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json .`

```text
repos[2]{name,stars}:
  jq,1
  tq,2
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 22 | 19 | -3 | -13.64% |
| `o200k_base` | Expanded | 46 | 19 | -27 | -58.70% |
| `cl100k_base` | Compact | 21 | 19 | -2 | -9.52% |
| `cl100k_base` | Expanded | 46 | 19 | -27 | -58.70% |
<!-- STACK_OVERFLOW_RESULTS_END -->
