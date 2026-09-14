---
type: Report
title: "Modify a key-value in a json using jq in-place"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 22."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 22: Modify a key-value in a json using jq in-place

[Stack Overflow question](https://stackoverflow.com/questions/42716734/modify-a-key-value-in-a-json-using-jq-in-place) · [selected answer](https://stackoverflow.com/a/42717073)

Rank: 22 · Question score: 172

Fixture: [source data](../../../tests/stack-overflow/22-modify-a-key-value-in-a-json-using-jq-in-place.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.address = "abcde"`

```json
{
  "address": "old",
  "city": "Boston"
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
| jq | JSON | timed | 3.284 | +0.0% | 2.69 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.116 | +55.8% | 15.42 (darwin-wait4) | +473.8% | 30 (+1 warmup) |
| tq | TOON | timed | 2.552 | -22.3% | 4.23 (darwin-wait4) | +57.6% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.address = "abcde"'`

```text
{"address":"abcde","city":"Boston"}
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.address = "abcde"'`

```text
{
  "address": "abcde",
  "city": "Boston"
}
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.address = "abcde"'`

```text
address: abcde
city: Boston
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 10 | 9 | -1 | -10.00% |
| `o200k_base` | Expanded | 17 | 9 | -8 | -47.06% |
| `cl100k_base` | Compact | 10 | 9 | -1 | -10.00% |
| `cl100k_base` | Expanded | 17 | 9 | -8 | -47.06% |
<!-- STACK_OVERFLOW_RESULTS_END -->
