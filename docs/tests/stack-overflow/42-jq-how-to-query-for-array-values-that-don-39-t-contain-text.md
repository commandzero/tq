---
type: Report
title: "jq: how to query for array values that don't contain text \"foo\"?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 42."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 42: jq: how to query for array values that don't contain text "foo"?

[Stack Overflow question](https://stackoverflow.com/questions/42746828/jq-how-to-query-for-array-values-that-dont-contain-text-foo) · [selected answer](https://stackoverflow.com/a/42747910)

Rank: 42 · Question score: 92

Fixture: [source data](../../../tests/stack-overflow/42-jq-how-to-query-for-array-values-that-don-39-t-contain-text-.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark keeps only objects whose tag list does not contain the substring `latest`; the empty tag list remains a valid negative case.

## Benchmark input

Query: `.[] | select(any(.imageTags[]; contains("latest")) | not) | .name`

Output mode: structured, using `tq`'s default TOON output.

```json
[
  {
    "name": "one",
    "imageTags": [
      "latest",
      "stable"
    ]
  },
  {
    "name": "two",
    "imageTags": [
      "foo",
      "latest"
    ]
  },
  {
    "name": "three",
    "imageTags": []
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
| jq | JSON | timed | 3.567 | +0.0% | 2.69 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | incorrect | n/a | n/a | n/a | n/a | n/a |
| tq | TOON | timed | 2.715 | -23.9% | 4.75 (darwin-wait4) | +76.7% | 30 (+1 warmup) |

Details:

- `yq`: process exited with classified error RuntimeTypePath (exit status 1)

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.[] | select(any(.imageTags[]; contains("latest")) | not) | .name'`

```text
"three"
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.[] | select(any(.imageTags[]; contains("latest")) | not) | .name'`

```text
"three"
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.[] | select(any(.imageTags[]; contains("latest")) | not) | .name'`

```text
three
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 3 | 2 | -1 | -33.33% |
| `o200k_base` | Expanded | 3 | 2 | -1 | -33.33% |
| `cl100k_base` | Compact | 3 | 2 | -1 | -33.33% |
| `cl100k_base` | Expanded | 3 | 2 | -1 | -33.33% |
<!-- STACK_OVERFLOW_RESULTS_END -->
