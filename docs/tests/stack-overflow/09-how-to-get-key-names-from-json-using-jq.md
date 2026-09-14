---
type: Report
title: "How to get key names from JSON using jq"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 09."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 09: How to get key names from JSON using jq

[Stack Overflow question](https://stackoverflow.com/questions/23118341/how-to-get-key-names-from-json-using-jq) · [selected answer](https://stackoverflow.com/a/23118607)

Rank: 9 · Question score: 314

Fixture: [source data](../../../tests/stack-overflow/09-how-to-get-key-names-from-json-using-jq.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `keys | sort | .[]`

```json
{
  "email": "madireddy@test.com",
  "id": "2323",
  "name": "test",
  "date": "02-03-2014-13:41",
  "type": "application"
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
| jq | JSON | timed | 3.110 | +0.0% | 2.69 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.110 | +64.3% | 15.25 (darwin-wait4) | +467.4% | 30 (+1 warmup) |
| tq | TOON | timed | 2.748 | -11.6% | 4.23 (darwin-wait4) | +57.6% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c 'keys | sort | .[]'`

```text
"date"
"email"
"id"
"name"
"type"
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq 'keys | sort | .[]'`

```text
"date"
"email"
"id"
"name"
"type"
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json 'keys | sort | .[]'`

```text
date
email
id
name
type
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 12 | 10 | -2 | -16.67% |
| `o200k_base` | Expanded | 12 | 10 | -2 | -16.67% |
| `cl100k_base` | Compact | 12 | 10 | -2 | -16.67% |
| `cl100k_base` | Expanded | 12 | 10 | -2 | -16.67% |
<!-- STACK_OVERFLOW_RESULTS_END -->
