---
type: Report
title: "How to run jq from gitbash in Windows?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 26."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-14T06:23:44Z }
---

# 26: How to run jq from gitbash in Windows?

[Stack Overflow question](https://stackoverflow.com/questions/53967693/how-to-run-jq-from-gitbash-in-windows) · [selected answer](https://stackoverflow.com/a/53967916)

Rank: 26 · Question score: 154

Fixture: [source data](../../../tests/stack-overflow/26-how-to-run-jq-from-gitbash-in-windows.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark query and input can be smaller or derived from the original answer examples.

These macOS measurements are a cross-platform proxy for the Windows/Git Bash scenario. They do not test a Windows binary, Git Bash shell parsing, or Windows newline behavior.

## Benchmark input

Query: `.`

```json
{
  "name": "jq",
  "platform": "windows"
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
| jq | JSON | timed | 3.127 | +0.0% | 2.67 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | JSON | timed | 5.111 | +63.4% | 15.31 (darwin-wait4) | +473.1% | 30 (+1 warmup) |
| tq | TOON | timed | 2.543 | -18.7% | 3.67 (darwin-wait4) | +37.4% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `structured`.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### Compact JSON (`jq -c`)

Status: `exited-0`.
Command: `jq -c .`

```text
{"name":"jq","platform":"windows"}
```

#### Expanded JSON (`jq`)

Status: `exited-0`.
Command: `jq .`

```text
{
  "name": "jq",
  "platform": "windows"
}
```

#### TOON (`tq` default)

Status: `exited-0`.
Command: `tq --input-format json .`

```text
name: jq
platform: windows
```

Counts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.

| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |
| --- | --- | ---: | ---: | ---: | ---: |
| `o200k_base` | Compact | 9 | 8 | -1 | -11.11% |
| `o200k_base` | Expanded | 16 | 8 | -8 | -50.00% |
| `cl100k_base` | Compact | 9 | 8 | -1 | -11.11% |
| `cl100k_base` | Expanded | 16 | 8 | -8 | -50.00% |
<!-- STACK_OVERFLOW_RESULTS_END -->
