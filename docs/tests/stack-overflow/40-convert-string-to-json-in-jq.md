---
type: Report
title: "Convert string to json in jq"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 40."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 40: Convert string to json in jq

[Stack Overflow question](https://stackoverflow.com/questions/34340549/convert-string-to-json-in-jq) · [selected answer](https://stackoverflow.com/a/34340787)

Rank: 40 · Question score: 100

Fixture: [source data](../../../tests/stack-overflow/40-convert-string-to-json-in-jq.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark parses the JSON text in `response.text` before selecting its title.

## Benchmark input

Query: `.response.text | fromjson | .title`

Output mode: structured, using `tq`'s default TOON output.

```json
{
  "requestType": "POST",
  "response": {
    "size": 78,
    "text": "{\"recordID\":123,\"title\":\"Hello World\",\"content\":\"Lorem ipsum...\"}"
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
| jq | JSON | timed | 3.100 | +0.0% | 2.69 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.132 | +65.5% | 15.44 (darwin-wait4) | +474.4% | 30 (+1 warmup) |
| tq | TOON | timed | 2.603 | -16.0% | 4.31 (darwin-wait4) | +60.5% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c '.response.text | fromjson | .title'`

```text
"Hello World"
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq '.response.text | fromjson | .title'`

```text
"Hello World"
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json '.response.text | fromjson | .title'`

```text
Hello World
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 4 | 3 | -1 | -25.00% |
| `o200k_base` | Expanded | 4 | 3 | -1 | -25.00% |
| `cl100k_base` | Compact | 4 | 3 | -1 | -25.00% |
| `cl100k_base` | Expanded | 4 | 3 | -1 | -25.00% |
<!-- STACK_OVERFLOW_RESULTS_END -->
