---
type: Report
title: "How do I keep colors when piping \"jq\" output to \"less\"?"
description: "Benchmark reproduction and measured results for Stack Overflow scenario 43."
generated: { by: codex/gpt-5.6-luna, at: 2026-09-13T05:53:31Z }
---

# 43: How do I keep colors when piping "jq" output to "less"?

[Stack Overflow question](https://stackoverflow.com/questions/62809196/how-do-i-keep-colors-when-piping-jq-output-to-less) · [selected answer](https://stackoverflow.com/a/62809275)

Rank: 43 · Question score: 92

Fixture: [source data](../../../tests/stack-overflow/43-how-do-i-keep-colors-when-piping-quot-jq-quot-output-to-quot.toon)

This page records the checked-in benchmark reproduction for the question. The benchmark checks forced color on redirected output, not an interactive `less -R` pager.

## Benchmark input

Query: `.`

Output mode: `color` with JSON output for `tq`. The runner passes `-C` so ANSI color bytes remain present when output is redirected.

```json
{
  "name": "jq",
  "color": "terminal"
}
```

Scenario source registry: [jq questions sorted by votes](https://stackoverflow.com/questions/tagged/jq?tab=votes&pagesize=50)

## Results
<!-- STACK_OVERFLOW_RESULTS_START -->
Status: `observed-failures`
Environment: `macos` / `aarch64` / `Apple M4 Pro` / 14 logical CPUs  
Tools: `jq` jq-1.8.2; `yq` yq (https://github.com/mikefarah/yq/) version v4.53.2; `tq` tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb)  
Memory evidence: authoritative peak RSS from the native collector; sampled-group RSS is not used as a substitute.  

Output profile: `color`.


| Tool | Output | Status | Wall (ms) | Δ vs jq | Peak RSS (MiB, source) | RSS Δ vs jq | Samples |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| jq | colored JSON | timed | 3.100 | +0.0% | 2.67 (darwin-wait4) | +0.0% | 30 (+1 warmup) |
| yq | colored JSON | timed | 5.000 | +61.3% | 15.41 (darwin-wait4) | +476.6% | 30 (+1 warmup) |
| tq | colored JSON | timed | 2.866 | -7.5% | 3.70 (darwin-wait4) | +38.6% | 30 (+1 warmup) |

### Output and tokens

Captured separately from timed samples.
Output profile: `color`.
ANSI control bytes are escaped for display; saved stdout retains the exact raw bytes.
Output tools: jq-1.8.2; yq (https://github.com/mikefarah/yq/) version v4.53.2; tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb).


#### jq

Status: `exited-0`.
Command: `jq -C .`

```text
\x1b[1;39m{\x1b[0m\n  \x1b[1;34m\"name\"\x1b[0m\x1b[1;39m:\x1b[0m \x1b[0;32m\"jq\"\x1b[0m\x1b[1;39m,\x1b[0m\n  \x1b[1;34m\"color\"\x1b[0m\x1b[1;39m:\x1b[0m \x1b[0;32m\"terminal\"\x1b[0m\n\x1b[1;39m}\x1b[0m\n
```

#### yq

Status: `exited-0`.
Command: `yq -p=json -o=json -I=0 -C .`

```text
{\x1b[36m\"name\"\x1b[0m:\x1b[32m\"jq\"\x1b[0m,\x1b[36m\"color\"\x1b[0m:\x1b[32m\"terminal\"\x1b[0m}\n
```

#### tq

Status: `exited-0`.
Command: `tq --input-format json --output-format json -C .`

```text
\x1b[1;39m{\x1b[0m\n  \x1b[1;34m\"name\"\x1b[0m\x1b[1;39m:\x1b[0m \x1b[0;32m\"jq\"\x1b[0m\x1b[1;39m,\x1b[0m\n  \x1b[1;34m\"color\"\x1b[0m\x1b[1;39m:\x1b[0m \x1b[0;32m\"terminal\"\x1b[0m\n\x1b[1;39m}\x1b[0m\n
```

Token comparison is not applicable to raw or color output profiles.
<!-- STACK_OVERFLOW_RESULTS_END -->
