---
type: Report
title: "How to install JQ on Mac on the command line?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 07."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 07: How to install JQ on Mac on the command line?

[Stack Overflow question](https://stackoverflow.com/questions/37668134/how-to-install-jq-on-mac-on-the-command-line) · [selected answer](https://stackoverflow.com/a/49261843)

Rank: 7 · Question score: 349

Fixture: [source data](../../../tests/stack-overflow/07-how-to-install-jq-on-mac-on-the-command-line.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `.`

```json
{
  "name": "jq",
  "platform": "macOS"
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
| jq | JSON | timed | 3.244 | +0.0% | 2.67 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.183 | +59.8% | 15.50 (darwin-wait4) | +480.1% | 30 (+1 warmup) |
| tq | TOON | timed | 2.678 | -17.4% | 3.67 (darwin-wait4) | +37.4% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c .`

```text
{"name":"jq","platform":"macOS"}
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq .`

```text
{
  "name": "jq",
  "platform": "macOS"
}
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json .`

```text
name: jq
platform: macOS
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 10 | 9 | -1 | -10.00% |
| `o200k_base` | Expanded | 17 | 9 | -8 | -47.06% |
| `cl100k_base` | Compact | 10 | 8 | -2 | -20.00% |
| `cl100k_base` | Expanded | 17 | 8 | -9 | -52.94% |
<!-- STACK_OVERFLOW_RESULTS_END -->
