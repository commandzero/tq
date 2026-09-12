---
type: Report
title: "Using jq with bash to run command for each object in array"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 50."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 50: Using jq with bash to run command for each object in array

[Stack Overflow question](https://stackoverflow.com/questions/43192556/using-jq-with-bash-to-run-command-for-each-object-in-array) · [selected answer](https://stackoverflow.com/a/43192740)

Rank: 50 · Question score: 79

Fixture: [source data](../../../tests/stack-overflow/50-using-jq-with-bash-to-run-command-for-each-object-in-array.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.[].user, .[].date, .[].email`

```json
[
  {
    "user": "danielrvt",
    "date": "11/10/1988",
    "email": "myemail@domain.com"
  },
  {
    "user": "second",
    "date": "12/11/1989",
    "email": "second@domain.com"
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
| jq | JSON | timed | 102.278 | +0.0% | 2.56 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 103.022 | +0.7% | 15.58 (bsd-time-l) | +507.9% | 30 (+1 warmup) |
| tq | TOON | timed | 104.993 | +2.7% | 4.09 (bsd-time-l) | +59.8% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
"danielrvt"
"second"
"11/10/1988"
"12/11/1989"
"myemail@domain.com"
"second@domain.com"
```

#### Expanded JSON (`jq`)

```text
"danielrvt"
"second"
"11/10/1988"
"12/11/1989"
"myemail@domain.com"
"second@domain.com"
```

#### TOON (`tq -o toon`)

```text
danielrvt
second
11/10/1988
12/11/1989
myemail@domain.com
second@domain.com
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 38 | 32 | -6 | -15.79% |
| `o200k_base` | Expanded | 38 | 32 | -6 | -15.79% |
| `cl100k_base` | Compact | 37 | 32 | -5 | -13.51% |
| `cl100k_base` | Expanded | 37 | 32 | -5 | -13.51% |
<!-- STACK_OVERFLOW_RESULTS_END -->
