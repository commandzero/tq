---
type: Report
title: "How to install JQ on Mac on the command line?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 07."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
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
Last updated: 2026-09-11
Status: `passed`
Environment: `macos` / `aarch64` / `Apple M4 Pro` / 14 logical CPUs  
Tools: `jq` jq-1.8.1; `yq` yq (https://github.com/mikefarah/yq/) version v4.53.2; `tq` tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown)  
Memory evidence: authoritative peak RSS from the recorded time implementation; sampled-group RSS is not used as a substitute.  

| Tool | Output | Status | Wall (ms) | Δ vs jq | Peak RSS (MiB, source) | RSS Δ vs jq | Samples |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| jq | JSON | timed | 180.083 | +0.0% | 2.58 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 180.028 | -0.0% | 15.97 (bsd-time-l) | +519.4% | 30 (+1 warmup) |
| tq | TOON | timed | 181.006 | +0.5% | 3.62 (bsd-time-l) | +40.6% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
{"name":"jq","platform":"macOS"}
```

#### Expanded JSON (`jq`)

```text
{
  "name": "jq",
  "platform": "macOS"
}
```

#### TOON (`tq -o toon`)

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
