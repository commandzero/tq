---
type: Report
title: Stack Overflow jq top 50 benchmark results
description: "Measured results for the Stack Overflow jq scenario campaign."
generated: { by: tq-stack-overflow, at: 2026-09-13T00:00:00Z }
benchmark_runs:
  - campaign_id: "2026-09-13"
    identity_status: not-recorded
    provenance: "Scenario Results sections preserve native macOS reports but do not record exact campaign IDs or binary hashes. Captured version strings identify tq 0.3.0 revision 24bbd2581677b951c5bf275ca80e64e366c9cefb, jq 1.8.2, and yq 4.53.2."
    binaries:
      tq: { version: "tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb)", sha256: null }
      jq: { version: "jq-1.8.2", sha256: null }
      yq: { version: "yq (https://github.com/mikefarah/yq/) version v4.53.2", sha256: null }
  - campaign_id: "2026-09-26"
    identity_status: not-recorded
    provenance: "Scenarios 38 and 42 are generated 2026-09-26 and retain the same native macOS tool version strings; exact campaign ID and binary hashes are not recorded."
    binaries:
      tq: { version: "tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb)", sha256: null }
      jq: { version: "jq-1.8.2", sha256: null }
      yq: { version: "yq (https://github.com/mikefarah/yq/) version v4.53.2", sha256: null }
  - campaign_id: "not-recorded"
    identity_status: not-recorded
    provenance: "Index text mentions earlier BSD-time/jq 1.8.1 measurements; tq version, any other measured tools, date, and binary hashes are not recorded."
    binaries:
      tq: { version: null, sha256: null, identity_status: not-recorded }
      jq: { version: "jq 1.8.1", sha256: null }
---

# Stack Overflow jq top 50 benchmark results

One scenario page corresponds to each checked-in question fixture. The fixtures are the source of the question metadata, benchmark query, and benchmark input used by the harness.

## Method

This compatibility suite runs separately from ordinary compatibility cases
and benchmark suites. From the repository root:

```console
./scripts/campaign-run.sh compatibility stack-overflow extended
```

The current extended profile takes one warmup and five measured samples per
configuration; quick takes 0+1 and standard takes 1+3. The historical Results
below were recorded using a different 30-sample policy and are unchanged by
this new invocation.

The 148 successful timing rows used one warmup and 30 measured samples for each
tool, for 4,440 accepted timing samples. The two correctness-failure rows each
retained one observed sample but no accepted timing summary, for 4,442 observed
samples total. Every observed sample passed the authoritative native RSS check.
No workspace tests or builds were launched concurrently with this campaign. This
native campaign is not directly comparable to the earlier BSD-time/jq 1.8.1
measurements and does not establish a self-regression baseline.

To regenerate the Results sections from saved measurements:

```console
cargo run --quiet -p tq-test-support --bin tq-stack-overflow -- render \
  --input /path/to/saved-report.json \
  --report-dir docs/tests/stack-overflow
```

Render-only does not execute the tools or collect new measurements.

Use an enriched report to regenerate the output examples and token tables.
The [`outputs` command](../../../benchmarks/README.md#stack-overflow-top-50)
captures compact JSON, expanded JSON, and TOON without repeating the timing
campaign. Token comparisons accept nonempty complete JSON values or ordered JSON
result streams when their values, order, and cardinality match TOON. Raw,
non-JSON, empty output, and errors are excluded. Counts use exact captured
stdout bytes decoded as UTF-8, including trailing newlines.
Diff and % are negative when TOON uses fewer tokens.

## Results
<!-- STACK_OVERFLOW_RESULTS_START -->
Last updated: 2026-09-13
Status: `observed-failures`
Environment: `macos` / `aarch64` / `Apple M4 Pro` / 14 logical CPUs  
Tools: `jq` jq-1.8.2; `yq` yq (https://github.com/mikefarah/yq/) version v4.53.2; `tq` tq 0.3.0 (TOON v3; jq target 1.8.x; revision 24bbd2581677b951c5bf275ca80e64e366c9cefb)  
Memory evidence: authoritative peak RSS from the native collector; sampled-group RSS is not used as a substitute.  

Coverage: 50 scenarios, 150 observations.

| Outcome | jq | yq | tq | Total |
| --- | ---: | ---: | ---: | ---: |
| timed | 50 | 48 | 50 | 148 |
| incorrect | 0 | 2 | 0 | 2 |
| unsupported | 0 | 0 | 0 | 0 |
| timeout | 0 | 0 | 0 | 0 |
| resource-limit | 0 | 0 | 0 | 0 |
| oom-or-signal | 0 | 0 | 0 | 0 |

### Matched successful scenario comparison

Aggregate values use the same 48 complete, RSS-verified scenarios for all three tools. They are medians of per-scenario medians and per-scenario authoritative peak RSS values; incomplete or failed scenarios are excluded from every column.

| Metric | jq | yq | yq Δ vs jq | tq | tq Δ vs jq |
| --- | ---: | ---: | ---: | ---: | ---: |
| Median wall (ms) | 3.118 | 5.134 | +64.7% | 2.634 | -15.5% |
| Median peak RSS (MiB) | 2.688 | 15.383 | +472.3% | 4.242 | +57.8% |

### Output token coverage

47 of 47 structured scenarios have successful, equivalent JSON and TOON output, including ordered result streams. Raw and color profiles use separate output gates and do not contribute token savings. Each eligible structured page compares exact TOON stdout with both compact and expanded jq JSON using `o200k_base` and `cl100k_base`. Diff and % are negative for savings. Output captures are separate from timing samples; their exact capture timestamp is preserved in the saved output artifact.
<!-- STACK_OVERFLOW_RESULTS_END -->
