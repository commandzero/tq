# Benchmark summary — 2026-08-01

This file summarizes the raw baseline reports in this directory. All timings
are median wall-clock times unless stated otherwise. A lower value is better.
`unsupported` means that an adapter did not implement the workload. It does not
mean that a timed run failed.

The reference tools are jq `1.8.2-8-g603db3f` and yq `4.53.2`. The reports use
the same 14-logical-CPU Apple Silicon macOS host and the `release-benchmark`
profile. The MVP report runs the reference tools with `tq`. Use its columns to
compare tq. Do not use the earlier reference-only report for that comparison.

## Report status

| Report | Profile | Timed | Unsupported | Result |
| --- | --- | ---: | ---: | --- |
| [standard-v2.json](performance/standard-v2.json) | Standard jq/yq reference | 136 | 152 | Passed |
| [tq-standard-mvp-v1.json](performance/tq-standard-mvp-v1.json) | Standard MVP comparison | 276 | 12 | Passed |
| [large-scalar-gate.json](performance/large-scalar-gate.json) | Large scalar extraction | 3 | 3 | Passed |
| [large-parse-discard.json](performance/large-parse-discard.json) | Large parse and discard | 3 | 3 | Passed |
| [tq-large-parse-discard-mvp-v1.json](performance/tq-large-parse-discard-mvp-v1.json) | Large MVP parse and discard | 6 | 0 | Passed |
| [large-event-stream-v1.json](performance/large-event-stream-v1.json) | Large event stream | 3 | 3 | Passed |
| [tq-standard-parse-discard-stable-v1.json](performance/tq-standard-parse-discard-stable-v1.json) | Focused standard check | 24 | 0 | Passed |
| [tq-standard-parse-discard-regression-v1.json](performance/tq-standard-parse-discard-regression-v1.json) | Focused regression check | 24 | 0 | Passed |

The output-heavy one-sample [full large-matrix attempt](performance/large-full-attempt.json)
ended with exit 137 after 44 minutes. It produced no atomic report. The report
records this as a harness resource outcome. It is not a jq, yq, or tq
measurement.

## Standard corpus: tq MVP comparison

The table below uses the largest standard fixture, `usgs-all-month` (7.35 MiB
JSON; 10,847 logical records), from the MVP report. Values are median total
wall time in milliseconds. JSON, YAML, and TOON are native input formats for
the corresponding columns.

| Workload | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Array construction | 117 | 330 | 483 | 117 | 235 | 142 |
| Blocking sort | 123 | 211 | 359 | 118 | 228 | 151 |
| Event stream | 269 | — | — | 2,822 | — | 528 |
| Identity re-encode | 349 | 526 | 706 | 182 | 349 | 236 |
| Multi-result projection | 125 | 259 | 405 | 148 | 264 | 181 |
| Numeric reduction | 120 | 227 | 414 | 122 | 268 | 141 |
| Object construction | 122 | 15,076 | 14,458 | 795 | 1,025 | 769 |
| Parse and discard | 115 | 185 | 365 | 123 | 267 | 151 |
| Path update | 302 | 469 | 638 | 180 | 317 | 203 |
| Scalar extraction | 89 | 174 | 320 | 121 | 235 | 154 |
| Selective filter | 113 | 206 | 398 | 121 | 239 | 140 |
| String reduction | 118 | 177 | 375 | 177 | 345 | 212 |

Across every matching workload/source pair in that report, the geometric-mean
wall-time ratios were:

| Comparison | Pairs | Geometric-mean ratio | Median ratio |
| --- | ---: | ---: | ---: |
| tq JSON / jq JSON | 48 | 1.17× | 1.01× |
| tq YAML / yq YAML | 44 | 0.71× | 0.86× |
| tq TOON / jq JSON* | 48 | 1.24× | 1.11× |

\*This last comparison uses different native encodings, so it is directional
rather than a same-format comparison.

### Event-stream latency

Total wall time includes completing the full output. For streaming workloads,
time to first result is the more useful responsiveness measure. On the same
`usgs-all-month` fixture, the MVP report records:

| Adapter | Median time to first result |
| --- | ---: |
| jq JSON | 256 ms |
| tq JSON | 7.1 ms |
| tq TOON | 9.5 ms |

## Large corpus results

The large corpus is the natural `microsoft-us-buildings-georgia` source with
3,981,792 logical features: 1.12 GB JSON and 1.61 GB TOON. Values below are
median total wall time in seconds. The reference and MVP parse-and-discard rows
are separate runs and are shown independently.

| Workload / report | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Scalar extraction ([reference](performance/large-scalar-gate.json)) | 12.68 | 130.44 | 334.08 | — | — | — |
| Parse and discard ([reference](performance/large-parse-discard.json)) | 13.10 | 96.32 | 273.71 | — | — | — |
| Parse and discard ([MVP rerun](performance/tq-large-parse-discard-mvp-v1.json)) | 14.14 | 82.84 | 209.92 | 19.20 | 32.04 | 20.74 |
| Event stream ([MVP](performance/large-event-stream-v1.json)) | 59.72 | — | — | 464.43 | — | 126.44 |

For the large event stream, full completion and first-result latency describe
different properties:

| Adapter | Full wall time | First result |
| --- | ---: | ---: |
| jq JSON | 59.72 s | 59.704 s |
| tq JSON | 464.43 s | 6.5 ms |
| tq TOON | 126.44 s | 16.7 ms |

Peak RSS was independently remeasured for the tq event-stream runs with the
same release binary and natural files. Both are comfortably below the 128 MiB
objective.

| Adapter | Peak RSS | Share of 128 MiB objective |
| --- | ---: | ---: |
| tq JSON | 3.2 MiB | 2.47% |
| tq TOON | 3.4 MiB | 2.62% |

## Focused parse-and-discard check

The later regression-gated standard run found no failures against its 50% wall
time and 20% RSS thresholds. For the largest standard fixture, its tq medians
remain close to the preceding stable report:

| Adapter | Stable report (ms) | Regression-gated report (ms) | Change |
| --- | ---: | ---: | ---: |
| tq JSON | 116.0 | 119.7 | +3.2% |
| tq YAML | 269.2 | 237.8 | -11.7% |
| tq TOON | 146.6 | 145.6 | -0.7% |

For corpus provenance, tool versions, per-sample distributions, throughput,
and correctness outcomes, use the linked JSON reports; this document does not
replace those source-of-record artifacts.
