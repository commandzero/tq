---
type: Guide
title: "Benchmark suites"
description: "Choose a benchmark campaign, in-process microbenchmark, or focused regression probe."
generated: { by: codex/gpt-6-astra, at: 2026-09-26T08:18:37Z }
---

# Benchmark suites

There is no single command that runs every performance suite. The main native
campaigns, Criterion microbenchmarks, and specialized probes answer different
questions and use different measurement boundaries. Do not combine their
numbers into one baseline or aggregate winner.

This page is the inventory and selection guide. The
[benchmark campaign guide](../../benchmarks/README.md) contains the detailed
corpus, accounting, calibration, reporting, and replay procedures. Run commands
below from the repository root.

## Suite and profile axes

Suites select cases and sources; profiles select repetition and reporting
extent. `natural-corpus` (default) uses USGS feeds and helper workloads,
`large-input` uses the Microsoft building-footprint source and JSON cases,
and `smoke` uses small fixtures. The separate `stack-overflow` compatibility
suite runs its 50 checked-in scenarios. These names are not profiles.

All four suites accept `quick` (0 warmups + 1 measured sample), `standard`
(1+3, default), or `extended` (1+5). Quick is a diagnostic, session-only
run. Benchmark quick and standard keep reports in temporary session locations
unless an output path is explicitly given; benchmark extended does not
automatically publish reviewed pages. Stack Overflow standard retains its
session report without rendering pages, while Stack Overflow quick retains
a session JSON diagnostic without captures or pages. Extended renders
diagnostic pages to the benchmark archive, not the checked-in reference
pages. Its compatibility checks are not benchmark self-regression gates.

Current suites keep their natural source sizes. The planned same-source
subset/size redesign is not implemented: changing a profile does not
generate smaller or larger subsets of earthquakes or buildings. The
selected-array sort belongs to the `large-input` suite, not a fourth
profile. Explicit `--sampling` changes repetitions, not cases or inputs.

## Choose a suite

| Question | Suite/profile | Start here |
| --- | --- | --- |
| Does the native benchmark path still work? | Smoke, standard | `./scripts/campaign-run.sh benchmark smoke` |
| How does this build look in a single diagnostic iteration? | Natural corpus, quick | `./scripts/campaign-run.sh benchmark natural-corpus quick` |
| What is the routine natural-corpus comparison? | Natural corpus, standard (both defaults) | `./scripts/campaign-run.sh benchmark` |
| How does the build behave on the roughly 1 GB building corpus? | Large input, standard or extended | `./scripts/campaign-run.sh benchmark large-input extended` |
| How does selected-array sorting behave at that scale? | Large input, selected case | `benchmarks/cases/parallel-selected-json.sh INPUT.geojson target/release/tq` |
| Which in-process stage explains an event-stream cost? | Criterion event-stream microbenchmarks | `./scripts/microbench-smoke.sh`, then focused `cargo bench` runs |
| How do realistic, independently documented queries compare? | Stack Overflow compatibility, standard or extended | `./scripts/campaign-run.sh compatibility stack-overflow extended` |
| What does colored output cost? | Output-color suite | `tq-color-bench`; see the [run instructions](output-colors-performance.md#regenerate) |
| How do JSON sequences, CSV, and TSV compare with reference tools? | Native-format reference cases | Selected `tq-bench` cases; see below |
| Are we investigating a specific format, transcode, or optimizer regression? | Specialized regression/probe scripts | See the focused-probe inventory below |

## Main native campaigns

`tq-bench` correctness-gates each row before timing it. Its catalog compares jq
on JSON, yq on JSON/YAML, and tq on JSON/YAML/TOON where applicable. Native-format
cases add other inputs. Unsupported adapters are explicit, not inferred product
capability failures. The isolated measurement worker records spawn-to-exit wall
time, CPU, and native peak RSS; applicable rows also report first-result latency
and throughput.

| Suite | Inputs and coverage | Default profile and sampling |
| --- | --- | --- |
| `smoke` | Checked-in examples and synthetic helpers. The **dispatcher selects five cases**: startup, parse/discard, scalar extraction, event stream, and object deep merge. | Standard, 1+3; direct CLI invocation can select a broader catalog matrix on small fixtures. |
| `natural-corpus` | Cached USGS feeds and applicable helper workloads. | Standard, 1+3; quick plans the same cases, inputs, and adapters with 0+1, stopping at its end-to-end deadline. |
| `large-input` | Microsoft Georgia building footprints, roughly 1 GB. Fast standard selects JSON parse/discard and dead-sort-length; extended adds selected-array sorting. | Standard, 1+3; extended, 1+5; fast-mode budgets of 900 seconds per campaign and 300 seconds per case. |

A campaign can expire before completing all planned rows; its wall budget is
not a promise of complete coverage. For standard and extended, dispatcher
builds, downloads, and conversions precede the benchmark campaign budget, and
cleanup may extend that deadline. The dispatcher runs a single `WORKERS`
setting for the selected-sort wrapper, not an automatic thread-count matrix.

**Quick must finish in less than one minute, excluding only compilation.**
Compilation stages the native guard in a private installation root. The script
executes it directly, without invoking Cargo again. Installation cleanup and
executable artifact discovery are guarded; non-compilation dispatcher setup
reduces the remaining 50-second work budget.
That budget covers corpus preparation, tool discovery, preflight, correctness,
measurements, reporting, and cleanup. The supervisor imposes a 55-second hard
cutoff. Direct CLI runs and `--sampling quick` use the same guard.
Budget expiry returns status 124 with incomplete coverage; SIGINT and SIGTERM
clean up owned children before returning 130 and 143. Cleanup targets the exact
coordinator child as well as its process group; a reserved reaper retains any
unreaped child at the hard cutoff without extending the deadline. The OS takes
over reaping after guard exit. A completed checkpoint is accepted only after
terminal reporting and child cleanup finish. Preparation can consume the budget
before any measurement exists.

Quick reports are session-only summary evidence, including `SAMPLING=quick`
with an extended positional profile. Explicit `--output` selects a session
artifact path. Quick rejects `--markdown-dir`, `--baseline`, and
`--instrument-rss`; its reports cannot later be published with `--render-only`
or used as formal regression baselines by non-quick runs.

```console
# Same natural corpus and cases as standard, one measured iteration.
./scripts/campaign-run.sh benchmark natural-corpus quick

# Explicitly trim the quick workload selection.
target/release/tq-bench run --suite natural-corpus --profile quick \
  --case benchmark.parse-discard --manifest "$MANIFEST" --cache-root "$CACHE_ROOT"

# Apply quick sampling to the large-input JSON cases.
target/release/tq-bench run --suite large-input --profile quick \
  --case benchmark.dead-sort-length --manifest "$MANIFEST" --cache-root "$CACHE_ROOT"
```

The single iteration refers to measurement, not the correctness gate or native
RSS preflight. Those prerequisites remain enabled. Quick summary tables are
diagnostic evidence, not a statistically qualified performance regression gate.

### Select coverage and repetition policy

Build the CLI and native harness binaries before invoking them directly:

```console
cargo build --release --locked -p tq-cli -p tq-test-support \
  --bin tq --bin tq-bench --bin tq-bench-worker --bin tq-corpus

# Broad applicable matrix on small fixtures, with screening repetitions.
target/release/tq-bench run --suite smoke --profile standard --sampling screen \
  --output "$TQ_BENCHMARK_ARCHIVE_ROOT/.work/smoke-matrix.json"

# A selected large-input comparison: one warmup and three samples.
target/release/tq-bench run --suite large-input --profile standard \
  --manifest "$MANIFEST" --cache-root "$CACHE_ROOT" \
  --case benchmark.dead-sort-length \
  --output "$TQ_BENCHMARK_ARCHIVE_ROOT/.work/selected-comparison.json"

# Opt into the exhaustive applicable large-input JSON workloads and catalog sampling.
SAMPLING=catalog BENCHMARK_MODE=exhaustive ./scripts/campaign-run.sh benchmark large-input extended

# JSON-only selected sorting without preparing YAML or TOON.
target/release/tq-bench run --suite large-input --profile extended \
  --input "$INPUT_JSON" --case benchmark.large-selected-sort \
  --output "$TQ_BENCHMARK_ARCHIVE_ROOT/.work/selected-sort.json"
```

Set the uppercase path variables to existing corpus inputs/cache locations and
the separate benchmark archive checkout before using these examples.

- `--sampling quick`: no warmup, one measured sample; session-only summary reporting.
- `--sampling screen`: one warmup, one sample.
- `--sampling compare`: one warmup, three samples; standard's default.
- `--sampling extended`: one warmup, five samples; extended's default. As an override this selects repetitions, not larger input variants or automatic publication.
- `--sampling catalog`: each workload's configured repetitions; set `SAMPLING=catalog` for the dispatcher when exhaustive mode should use those counts.
- `--mode exhaustive`: opt into full applicable coverage rather than the fast
  large-input selection; non-quick default budgets become 3600 seconds per
  campaign and 600 seconds per case.
- `--adapter ID`: restrict adapters, retaining the correctness reference.
- `--campaign-budget-seconds` and `--case-budget-seconds`: override wall budgets. Quick allows only values up to 50 seconds; smaller campaign limits also bound setup work.
- `--instrument-rss`: opt into separate RSS-enforcement repetitions. Native peak
  RSS is always collected; by default an RSS-limited primary run also performs
  live sampling rather than duplicating the work. Separate repetitions are forbidden for quick runs.

These controls belong to `tq-bench`; they are not universal flags for the color,
Stack Overflow, Criterion, or legacy script suites. Three comparison samples
also do not satisfy the native self-regression gate's default five-sample minimum.

Native campaign reports are atomically checkpointed after completed rows.
Interrupted reports remain explicitly incomplete and cannot publish comparison
pages or pass a regression comparison. Corpus admission reuses semantic evidence
only when artifact, validator executable, and policy identities match.
High-cardinality correctness witnesses are disk-backed, but the correctness
capture limit remains 32 MiB; this is not unlimited-output validation.

## Other maintained suites

### Event-stream microbenchmarks

The [event-stream guide](event-stream-microbenchmarks.md) documents 50 Criterion
cases across `tq-core`, `tq-toon`, `tq-formats`, and `tq-cli`: event decoding,
stream-record projection, VM execution, output encoding, and the in-process
runner. Fixtures are deterministic and small. These measurements exclude native
process startup and are not peak-RSS campaign evidence.

```console
./scripts/microbench-smoke.sh
cargo bench --locked --bench event_stream \
  -p tq-core -p tq-toon -p tq-formats -p tq-cli
```

The measured suite defaults to 30 samples, a one-second warmup, and a
three-second measurement target per case. Use its filters and saved Criterion
baselines for attribution; stage medians overlap and must not be added together.

### Stack Overflow top 50

The [scenario index](stack-overflow/index.md) links each query, input,
explanation, and result page. The dedicated `tq-stack-overflow` runner compares
jq, yq, and tq with jq as the correctness reference. Benchmark suites and the
normal tests do not invoke these 50 scenarios automatically.

The dispatcher defaults to standard, retaining a temporary session report.
Quick prints diagnostic summaries and saves session JSON without captures or
pages. Extended writes a report and diagnostic pages to the archive; it does
not overwrite checked-in reference pages because this runner does not enforce
the calibrated publication gate. See the
[Stack Overflow campaign instructions](../../benchmarks/README.md#stack-overflow-top-50)
for capture-only and saved-report rendering modes.

### Output colors

[`tq-color-bench`](output-colors-performance.md) compares monochrome and forced
color using the same tq binary, covering JSON document output, multi-result JSON,
and TOON. It uses cached USGS week/month sources, correctness gates, two warmups,
and five samples per row. A complete passing run can update its result page;
failed or incomplete reports cannot replace published results.

### Native-format reference cases

The [native-format catalog](../../benchmarks/cases/native-formats.jsonl) provides
`benchmark.native-json-seq`, `benchmark.native-csv`, and `benchmark.native-tsv`.
These use the native `tq-bench` harness, with jq or yq references as appropriate:

```console
target/release/tq-bench run --suite smoke --profile standard --sampling compare \
  --case benchmark.native-json-seq --case benchmark.native-csv \
  --case benchmark.native-tsv \
  --output "$TQ_BENCHMARK_ARCHIVE_ROOT/.work/native-formats.json"
```

See the [native-format reference instructions](../../benchmarks/README.md#native-format-reference-campaign)
for tool versions, fixture semantics, and comparison objectives.

## Specialized and historical probes

These are explicit investigations, not additions to the default standard campaign.
Many use macOS `/usr/bin/time -l` wrappers rather than the native worker. Keep
their methodology and baselines separate, and consult each procedure for platform,
input, frozen-binary, and release-gate requirements.

| Probe | Purpose | Entrypoint / documentation |
| --- | --- | --- |
| Native-format refactor matrix | Frozen baseline/candidate comparison across input formats, execution modes, and native output; includes an investigation phase. | [Procedure](../../benchmarks/README.md#native-format-refactor-regression-campaign), `native-format-regression.py` |
| Native-format focused probe | Recheck named workload regressions with alternating binary pairs. | [Script](../../benchmarks/cases/native-format-probe.py) and the refactor procedure |
| Native sequence performance | Paired CSV, TSV, JSON-sequence, TOON-sequence, and JSONL extraction checks. | [Script](../../benchmarks/cases/native-sequence-perf.py) and the refactor procedure |
| Streaming transcode | Automatic structural transcode versus forced document execution; synthetic shapes plus accepted natural inputs. | [Procedure](../../benchmarks/README.md#streaming-transcode-campaign), `streaming-transcode.sh` |
| `toon` faceoff | tq versus default `toon` and its `json_stream` build. | [Procedure](../../benchmarks/README.md#toon-faceoff), `toon-vs-tq.sh` |
| Earlier optimizer/transcode experiments | Reproduce focused historical comparisons; not equivalent-method native campaign baselines. | [Fast discard](../../benchmarks/cases/fast-discard.sh), [hybrid blocking](../../benchmarks/cases/hybrid-blocking.sh), [lightweight transcode](../../benchmarks/cases/lightweight-transcode.sh) |

`parallel-selected-json.sh` invokes the native large-input extended selected
case, not a legacy timing wrapper. It accepts `SAMPLING` overrides and `WORKERS`
and does not assert that a parallel decoding plan is active.

## Evidence and documentation map

1. [Benchmark campaign guide](../../benchmarks/README.md): detailed execution,
   corpus preparation, calibration, replay, and specialized probe instructions.
2. [Performance comparison pages](comparison/index.md): reviewed workload
   findings, not an inventory of all available suites.
3. [Benchmark harness review](benchmark-harness.md): correctness/accounting repair
   history and acceptance evidence, not the primary launch guide.
4. [Recorded Linux comparison](comparison-x86-64-linux.md): platform-specific
   findings, not portable performance guarantees.
5. [Test reviews index](index.md): performance reviews plus adjacent correctness
   reviews such as the jq manual comparison.

Raw reports, downloaded inputs, generated formats, sample data, and executable
provenance belong in the separate `commandzero/tq-benchmarks` archive. This
repository owns the harnesses, catalogs, reproducible fixtures, and reviewed
result pages. Native accounting controls validate the measurement machinery;
they are not jq/yq/tq workload benchmarks. The normal Rust regression tests and
compatibility/manual comparisons likewise should not be described as performance
campaign results.
