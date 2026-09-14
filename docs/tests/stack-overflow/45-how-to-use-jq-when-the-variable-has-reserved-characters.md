---
type: Report
title: "How to use jq when the variable has reserved characters?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 45."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 45: How to use jq when the variable has reserved characters?

[Stack Overflow question](https://stackoverflow.com/questions/37018393/how-to-use-jq-when-the-variable-has-reserved-characters) · [selected answer](https://stackoverflow.com/a/37018703)

Rank: 45 · Question score: 84

Fixture: [source data](../../../tests/stack-overflow/45-how-to-use-jq-when-the-variable-has-reserved-characters.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark quotes the key containing a period and leaves the similarly named `USD` object as a decoy.

## Benchmark input

Query: `.["OPEN.BTC"]`

Output mode: structured, using `tq`'s default TOON output.

```json
{
  "OPEN.BTC": {
    "volume24": 0.932166,
    "price": 0.09995,
    "updated": "2016-05-04T03:03:29.000Z"
  },
  "USD": {
    "price": 1
  }
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
| jq | JSON | timed | 3.078 | +0.0% | 2.67 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.003 | +62.5% | 15.44 (darwin-wait4) | +477.8% | 30 (+1 warmup) |
| tq | TOON | timed | 2.598 | -15.6% | 4.23 (darwin-wait4) | +58.5% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.["OPEN.BTC"]'`

```text
{"volume24":0.932166,"price":0.09995,"updated":"2016-05-04T03:03:29.000Z"}
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.["OPEN.BTC"]'`

```text
{
  "volume24": 0.932166,
  "price": 0.09995,
  "updated": "2016-05-04T03:03:29.000Z"
}
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.["OPEN.BTC"]'`

```text
volume24: 0.932166
price: 0.09995
updated: "2016-05-04T03:03:29.000Z"
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 34 | 36 | +2 | +5.88% |
| `o200k_base` | Expanded | 44 | 36 | -8 | -18.18% |
| `cl100k_base` | Compact | 34 | 36 | +2 | +5.88% |
| `cl100k_base` | Expanded | 44 | 36 | -8 | -18.18% |
<!-- STACK_OVERFLOW_RESULTS_END -->
