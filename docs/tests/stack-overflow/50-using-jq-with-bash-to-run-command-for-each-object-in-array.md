---
type: Report
title: "Using jq with bash to run command for each object in array"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 50."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
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
Status: `observed-failures`
Environment: `macos` / `aarch64` / `Apple M4 Pro` / 14 logical CPUs  
Tools: `jq` jq-1.8.2; `yq` yq (https://github.com/mikefarah/yq/) version v4.53.2; `tq` tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb)  
Memory evidence: authoritative peak RSS from the native collector; sampled-group RSS is not used as a substitute.  

Output profile: `structured`.


| Tool | Output | Status | Wall (ms) | Δ vs jq | Peak RSS (MiB, source) | RSS Δ vs jq | Samples |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| jq | JSON | timed | 3.119 | +0.0% | 2.67 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.136 | +64.6% | 15.22 (darwin-wait4) | +469.6% | 30 (+1 warmup) |
| tq | TOON | timed | 2.700 | -13.5% | 4.14 (darwin-wait4) | +55.0% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.[].user, .[].date, .[].email'`

```text
"danielrvt"
"second"
"11/10/1988"
"12/11/1989"
"myemail@domain.com"
"second@domain.com"
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.[].user, .[].date, .[].email'`

```text
"danielrvt"
"second"
"11/10/1988"
"12/11/1989"
"myemail@domain.com"
"second@domain.com"
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.[].user, .[].date, .[].email'`

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
