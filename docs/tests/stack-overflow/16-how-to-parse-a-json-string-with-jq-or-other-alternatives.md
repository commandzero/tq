---
type: Report
title: "how to parse a JSON String with jq (or other alternatives)?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 16."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 16: how to parse a JSON String with jq (or other alternatives)?

[Stack Overflow question](https://stackoverflow.com/questions/35154684/how-to-parse-a-json-string-with-jq-or-other-alternatives) · [selected answer](https://stackoverflow.com/a/35155249)

Rank: 16 · Question score: 217

Fixture: [source data](../../../tests/stack-overflow/16-how-to-parse-a-json-string-with-jq-or-other-alternatives.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.c`

```json
{
  "a": 1,
  "b": 2,
  "c": "{\"id\":\"9ee\",\"parent\":\"abc\"}"
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
| jq | JSON | timed | 3.083 | +0.0% | 2.70 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 4.946 | +60.4% | 15.11 (darwin-wait4) | +459.0% | 30 (+1 warmup) |
| tq | TOON | timed | 2.586 | -16.1% | 4.06 (darwin-wait4) | +50.3% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c .c`

```text
"{\"id\":\"9ee\",\"parent\":\"abc\"}"
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq .c`

```text
"{\"id\":\"9ee\",\"parent\":\"abc\"}"
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json .c`

```text
"{\"id\":\"9ee\",\"parent\":\"abc\"}"
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 12 | 12 | +0 | +0.00% |
| `o200k_base` | Expanded | 12 | 12 | +0 | +0.00% |
| `cl100k_base` | Compact | 12 | 12 | +0 | +0.00% |
| `cl100k_base` | Expanded | 12 | 12 | +0 | +0.00% |
<!-- STACK_OVERFLOW_RESULTS_END -->
