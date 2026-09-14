---
type: Report
title: "Install jq JSON processor on Ubuntu 10.04"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 37."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 37: Install jq JSON processor on Ubuntu 10.04

[Stack Overflow question](https://stackoverflow.com/questions/33184780/install-jq-json-processor-on-ubuntu-10-04) · [selected answer](https://stackoverflow.com/a/44546590)

Rank: 37 · Question score: 112

Fixture: [source data](../../../tests/stack-overflow/37-install-jq-json-processor-on-ubuntu-10-04.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.`

```json
{
  "name": "John",
  "age": 31,
  "city": "New York"
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
| jq | JSON | timed | 3.107 | +0.0% | 2.67 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.316 | +71.1% | 15.53 (darwin-wait4) | +481.3% | 30 (+1 warmup) |
| tq | TOON | timed | 2.570 | -17.3% | 3.75 (darwin-wait4) | +40.4% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c .`

```text
{"name":"John","age":31,"city":"New York"}
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq .`

```text
{
  "name": "John",
  "age": 31,
  "city": "New York"
}
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json .`

```text
name: John
age: 31
city: New York
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 14 | 14 | +0 | +0.00% |
| `o200k_base` | Expanded | 24 | 14 | -10 | -41.67% |
| `cl100k_base` | Compact | 14 | 14 | +0 | +0.00% |
| `cl100k_base` | Expanded | 24 | 14 | -10 | -41.67% |
<!-- STACK_OVERFLOW_RESULTS_END -->
