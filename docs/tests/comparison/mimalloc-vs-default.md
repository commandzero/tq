---
type: Report
title: "mimalloc versus the system allocator"
description: "Extended diagnostic comparison of default mimalloc tq, system-allocator tq, and jq, with speed, memory, and publication limitations."
generated: { by: openai-codex/gpt-6-astra, at: 2026-09-26T20:44:24Z }
benchmark_runs:
  - campaign_id: "2026-09-26T19:27:09.746840362Z"
    identity_status: not-recorded
    provenance: "Extended Linux diagnostic system-allocator campaign; exact tool version strings and executable SHA-256 values are retained in system.json; tq revision is unknown."
    run_scope: system-allocator
    binaries:
      tq: { version: "tq 0.4.0 (TOON v3; jq target 1.8.x; revision unknown)", sha256: "221c680db8481f965b8579d036183766e180473185799590c72247be7fb107c4", identity_status: not-recorded, allocator: system }
      jq: { version: "jq-1.8.1", sha256: "136748786226819bf582738e8be963638c9d721aa0c5d1d650b506a2a52ddb97" }
      yq: { version: "yq (https://github.com/mikefarah/yq/) version v4.53.2", sha256: "d56bf5c6819e8e696340c312bd70f849dc1678a7cda9c2ad63eebd906371d56b" }
  - campaign_id: "2026-09-26T19:45:18.07883121Z"
    identity_status: not-recorded
    provenance: "Extended Linux diagnostic mimalloc campaign; exact tool version strings and executable SHA-256 values are retained in mimalloc.json; tq revision is unknown."
    run_scope: default-mimalloc
    binaries:
      tq: { version: "tq 0.4.0 (TOON v3; jq target 1.8.x; revision unknown)", sha256: "7068a178ce8f62f6cbfaaaf588049882dd5500d8213a7711a121043db6a0d338", identity_status: not-recorded }
      jq: { version: "jq-1.8.1", sha256: "136748786226819bf582738e8be963638c9d721aa0c5d1d650b506a2a52ddb97" }
      yq: { version: "yq (https://github.com/mikefarah/yq/) version v4.53.2", sha256: "d56bf5c6819e8e696340c312bd70f849dc1678a7cda9c2ad63eebd906371d56b" }
---

# mimalloc versus the system allocator

## Decision and scope

The `tq` binary enables the `mimalloc` Cargo feature by default. This experiment
supports that speed-versus-memory tradeoff for allocation-heavy queries, not a
claim that every workload becomes faster. The system allocator remains available
with `--no-default-features`; library consumers retain control of their allocator.
Here, “system” means Rust's system allocator, not tq's new default.

**Diagnostic evidence only.** These complete extended campaigns were collected
without linked native timing calibration and launch-isolation evidence. The
publication renderer rejected the saved mimalloc report:

```text
benchmark.array-construction jq-json primary sample requires positive observed control excess plus calibrated worker, lifetime-scope, and launch-isolation evidence
```

Consequently, these numbers do not supersede the calibrated historical tables in
this directory, establish a formal self-regression pass, or waive their publication
gate. A publication-ready campaign needs matching native controls under the
[benchmark publication procedure](../../../benchmarks/README.md).

## Method

- Source: merge commit `04e924f`, incorporating main `14447d9`, plus the retained
  mimalloc feature patch. The archived patch also contains the README explanation
  subsequently removed; that documentation change does not affect these binaries.
- Host platform: Linux x86-64, AMD Ryzen 7 7700, 16 logical CPUs, 61.9 GiB RAM;
  kernel `7.2.4-ogc3.1.fc44.x86_64`; Rust 1.98.1, release profile with
  `codegen-units = 1`, `lto = "thin"`, and `strip = "symbols"` at the tested revision.
- Candidate: `mimalloc` 0.1.52 with default crate features, `libmimalloc-sys`
  0.1.49; no secure-mode opt-in. Baseline tq: identical source with
  `--no-default-features`. The benchmark harness was built once and unchanged.
- Command: `./scripts/campaign-run.sh benchmark natural-corpus extended`, with
  exact precompiled binary paths, exhaustive coverage, one warmup and five measured
  samples per timed row. Both campaigns used identical corpus and manifest hashes.
- Reference tools: jq 1.8.1 and yq 4.53.2. The tables below compare JSON input
  against jq; the full raw reports also retain YAML, TOON, yq, and helper workloads.
- System campaign first, mimalloc second, on the same host; not randomized paired
  trials. jq was measured in both campaigns. Tables below use jq from the mimalloc
  campaign; the CSV retains both jq baselines and campaign-specific ratios.
- Wall time: direct child spawn to exit observation. CPU and lifetime peak RSS:
  native Linux `wait4`. RSS is the maximum peak across five samples, not virtual
  address reservation. First-result timing and raw protocol fields remain in the
  reports. RSS-limited rows may also sample process-group RSS for live enforcement;
  these campaigns did not request separate instrumented repetitions.

Main commit `80eccc0` subsequently removed the explicit codegen-unit and LTO
overrides. These measurements predate that change and do not establish performance
under the current Cargo-default optimization settings.

```console
# System allocator baseline
cargo build --release --locked -p tq-cli --bin tq --no-default-features
# Default mimalloc candidate
cargo build --release --locked -p tq-cli --bin tq
```

Each binary returned `{"count":10000,"sum":49995000}` for
`[range(0;10000)] | {count: length, sum: add}`. A separate smoke invocation with
`MIMALLOC_SHOW_STATS=1` confirmed allocator activation only in the mimalloc build;
that variable was not set during the campaigns.

## Representative results

USGS month corpus, JSON input. Wall cells are **median / median absolute deviation**
from five samples. Lower wall time and RSS are better. Allocator change compares
mimalloc tq with system tq, not with jq. One-decimal presentation is not a claim
of calibrated timing accuracy.

| Workload | jq | tq system | tq mimalloc (default) | Allocator wall change |
| --- | ---: | ---: | ---: | ---: |
| parse-discard | 93.9 / 0.8 ms | 119.5 / 0.6 ms | 86.0 / 0.6 ms | -28.1% |
| scalar-extraction | 92.7 / 1.0 ms | 118.6 / 0.3 ms | 85.5 / 0.2 ms | -27.9% |
| array-construction | 112.4 / 0.7 ms | 138.3 / 0.7 ms | 102.1 / 0.3 ms | -26.2% |
| blocking-sort | 102.7 / 0.4 ms | 138.9 / 0.2 ms | 124.7 / 0.5 ms | -10.2% |
| event-stream | 259.6 / 2.3 ms | 553.7 / 3.6 ms | 554.1 / 5.0 ms | +0.1% |
| walk-structural | 1018.9 / 4.5 ms | 492.9 / 3.3 ms | 414.9 / 1.7 ms | -15.8% |

| Workload | jq peak RSS | tq system peak RSS | tq mimalloc peak RSS | Added peak RSS |
| --- | ---: | ---: | ---: | ---: |
| parse-discard | 60.3 MiB | 68.0 MiB | 74.1 MiB | +6.1 MiB |
| scalar-extraction | 60.2 MiB | 68.4 MiB | 74.5 MiB | +6.1 MiB |
| array-construction | 64.6 MiB | 71.6 MiB | 82.9 MiB | +11.3 MiB |
| blocking-sort | 60.8 MiB | 10.0 MiB | 23.1 MiB | +13.1 MiB |
| event-stream | 3.9 MiB | 8.9 MiB | 15.1 MiB | +6.2 MiB |
| walk-structural | 86.5 MiB | 104.9 MiB | 112.8 MiB | +7.9 MiB |

Parse/discard, scalar extraction, and array construction reduced median time by
26–28%, adding approximately 6–11 MiB peak RSS. Mimalloc tq was also faster than
jq on those rows. Blocking sort improved by 10.2% with peak RSS still below
24 MiB, but remained slower than jq. Event streaming was effectively unchanged
and remained slower than jq. Structural walk was 15.8% faster than system tq and
2.46 times as fast as jq, using 7.9 MiB more peak RSS than system tq.

## Coverage, unfavorable results, and limitations

Each campaign completed all 858 planned rows: 715 timed, 134 unsupported, and
nine resource-limit outcomes. Every timed row has five samples, totaling 3,575
timed samples per campaign. Both commands exited 1 with `observed-failures`;
there were no outcome differences between allocator configurations.

The same nine failures occurred in both campaigns: object construction on the
month corpus and string reduction on the week/month corpora, each across
JSON/YAML/TOON. They exited with the classified resource status 5 and cannot
support speed rankings. Unsupported rows comprise 130 yq observations and four
tq YAML event-stream observations.

Across 416 matched timed tq rows:

| Observation | Rows |
| --- | ---: |
| Lower median wall time with mimalloc | 293 |
| Higher median wall time with mimalloc | 123 |
| Wall-time increase above 20% | 86 |
| Wall-time increase above 50% | 0 |
| Higher peak RSS with mimalloc | 416 |
| Peak-RSS increase above 20% | 373 |
| Peak-RSS increase above 50% | 291 |

The 86 wall-time increases above 20% are all tiny-input cases: 85 hour-corpus
rows and one eight-record native-format row. The largest relative increase was
TOON numeric reduction on the hour corpus, 1.340 to 1.965 ms (+46.6%). Startup
and campaign-order effects are possible contributors, not established causes.
The exact allocator retention costs were not profiled. The full regression CSV
retains each affected row's baseline, candidate, sample count, wall dispersion,
and individual RSS samples; aggregate counts are not a universal winner or a
formal acceptance gate.

## Retained evidence

Raw reports and comparison CSVs are retained in the separate `tq-benchmarks`
checkout under `.work/mimalloc-extended-20260926/`. This is local retained evidence,
not a promise of publicly downloadable artifacts. The directory also contains
build/smoke logs, exit statuses, and `provenance.json`. The archived provenance
contains local routing details and is not embedded in this published summary.

System campaign: `2026-09-26T19:27:09.746840362Z`.
Mimalloc campaign: `2026-09-26T19:45:18.07883121Z`.

| Artifact | SHA-256 |
| --- | --- |
| `system.json` | `a98896e5e9ca44442df5f205b21581f7a2a7cb4173fd427ad35a469d87c1d174` |
| `mimalloc.json` | `94a9b6af5e7da6899f93418b814450cc7d0f2c8cec30496d0b933ff2faeddf31` |
| `allocator-comparison.csv` | `e7564253cfc283d57214f0996b5010a1c1faf7435945fbb082f57db5cc2419f7` |
| `jq-baseline-comparison.csv` | `b19ec2438a53bd4eeb19733fc6e4b0c6bbc2243a72b8b7584802210518212664` |
| `regressions-over-20-percent.csv` | `1d60dc6f3c64a07939f9639be1e1317fd1470f9381575c12565cc6aafcb163c1` |
| `mimalloc-feature.patch` | `0947d070636c928554b336ee53ae0226fc28ece7fc513aa5629dec325667e83e` |

The system tq executable hash is
`221c680db8481f965b8579d036183766e180473185799590c72247be7fb107c4`;
the mimalloc tq executable hash is
`7068a178ce8f62f6cbfaaaf588049882dd5500d8213a7711a121043db6a0d338`.
The report's tq version string records revision `unknown`, because the remote
source was transferred as an archive; source/patch provenance above identifies
the tested code rather than inventing embedded revision metadata.
