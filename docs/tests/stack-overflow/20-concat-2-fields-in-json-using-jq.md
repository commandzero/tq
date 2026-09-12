---
type: Report
title: "Concat 2 fields in JSON using jq"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 20."
generated: { by: tq-stack-overflow, at: 2026-09-11T22:35:22.2189Z }
---

# 20: Concat 2 fields in JSON using jq

[Stack Overflow question](https://stackoverflow.com/questions/37710718/concat-2-fields-in-json-using-jq) · [selected answer](https://stackoverflow.com/a/37710802)

Rank: 20 · Question score: 177

Fixture: [source data](../../../tests/stack-overflow/20-concat-2-fields-in-json-using-jq.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

## Benchmark input

Query: `{"channel": (.profile_type + "." + .channel)}`

```json
{
  "channel": "youtube",
  "profile_type": "video",
  "member_key": "hello"
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
| jq | JSON | timed | 119.031 | +0.0% | 2.61 (bsd-time-l) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 129.069 | +8.4% | 16.33 (bsd-time-l) | +525.7% | 30 (+1 warmup) |
| tq | TOON | timed | 122.443 | +2.9% | 4.14 (bsd-time-l) | +58.7% | 30 (+1 warmup) |

### Output and tokens

Captured separately on 2026-09-11T23:05:31.105542Z. Timing samples are unchanged.
Output tools: jq-1.8.1; tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown).


#### Compact JSON (`jq -c`)

```text
{"channel":"video.youtube"}
```

#### Expanded JSON (`jq`)

```text
{
  "channel": "video.youtube"
}
```

#### TOON (`tq -o toon`)

```text
channel: video.youtube
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 6 | 5 | -1 | -16.67% |
| `o200k_base` | Expanded | 10 | 5 | -5 | -50.00% |
| `cl100k_base` | Compact | 6 | 5 | -1 | -16.67% |
| `cl100k_base` | Expanded | 10 | 5 | -5 | -50.00% |
<!-- STACK_OVERFLOW_RESULTS_END -->
