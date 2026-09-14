---
type: Report
title: "How to combine the sequence of objects in jq into one object?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 38."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 38: How to combine the sequence of objects in jq into one object?

[Stack Overflow question](https://stackoverflow.com/questions/34477547/how-to-combine-the-sequence-of-objects-in-jq-into-one-object) · [selected answer](https://stackoverflow.com/a/34477713)

Rank: 38 · Question score: 109

Fixture: [source data](../../../tests/stack-overflow/38-how-to-combine-the-sequence-of-objects-in-jq-into-one-object.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark input is already an array of objects, and `add` merges those values. It does not test the command-line `-s` slurp step used when separate input documents arrive as a stream.

## Benchmark input

Query: `add`

Output mode: structured, using `tq`'s default TOON output.

```json
[
  {
    "a": "green",
    "b": "white"
  },
  {
    "a": "red",
    "c": "purple"
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
| jq | JSON | timed | 3.106 | +0.0% | 2.69 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | incorrect | n/a | n/a | n/a | n/a | n/a |
| tq | TOON | timed | 2.579 | -17.0% | 4.16 (darwin-wait4) | +54.7% | 30 (+1 warmup) |

Details:

- `yq`: process exited with classified error RuntimeTypePath (exit status 1)

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c add`

```text
{"a":"red","b":"white","c":"purple"}
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq add`

```text
{
  "a": "red",
  "b": "white",
  "c": "purple"
}
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json add`

```text
a: red
b: white
c: purple
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 13 | 12 | -1 | -7.69% |
| `o200k_base` | Expanded | 23 | 12 | -11 | -47.83% |
| `cl100k_base` | Compact | 13 | 12 | -1 | -7.69% |
| `cl100k_base` | Expanded | 23 | 12 | -11 | -47.83% |
<!-- STACK_OVERFLOW_RESULTS_END -->
