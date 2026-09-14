---
type: Report
title: "Can I pass a string variable to jq rather than passing a file?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 31."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 31: Can I pass a string variable to jq rather than passing a file?

[Stack Overflow question](https://stackoverflow.com/questions/47105490/can-i-pass-a-string-variable-to-jq-rather-than-passing-a-file) · [selected answer](https://stackoverflow.com/a/47105805)

Rank: 31 · Question score: 127

Fixture: [source data](../../../tests/stack-overflow/31-can-i-pass-a-string-variable-to-jq-rather-than-passing-a-fil.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.key`

```json
{
  "key": "value"
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
| jq | JSON | timed | 3.123 | +0.0% | 2.67 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 4.963 | +58.9% | 15.11 (darwin-wait4) | +465.5% | 30 (+1 warmup) |
| tq | TOON | timed | 2.761 | -11.6% | 4.03 (darwin-wait4) | +50.9% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c .key`

```text
"value"
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq .key`

```text
"value"
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json .key`

```text
value
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 2 | 2 | +0 | +0.00% |
| `o200k_base` | Expanded | 2 | 2 | +0 | +0.00% |
| `cl100k_base` | Compact | 2 | 2 | +0 | +0.00% |
| `cl100k_base` | Expanded | 2 | 2 | +0 | +0.00% |
<!-- STACK_OVERFLOW_RESULTS_END -->
