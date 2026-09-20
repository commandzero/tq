## Context

See [proposal.md](proposal.md) for the motivation and issue scope.

The explicit stream path sends JSON and TOON records into the same private `StreamExecutor::accept` boundary. Each record creates an event VM, calls `for_each_result`, drains effects, and merges observations. VM construction gained effect state after v0.3.0. These are candidates for attribution, not a diagnosed cause.

Existing public boundaries include JSON structural decoding, TOON `Decoder::decode_into`, `stream_json`, `stream_toon`, event-plan VM construction, output sequence encoding, and `run_with_io`. The runner already uses synchronous result consumption; benchmarking `next_result` instead could introduce worker-thread behavior absent from the measured workload.

## Goals / Non-Goals

Goals:

1. Make the shared per-event cost directly measurable without decoding or compilation.
2. Preserve production allocation and teardown in the primary execution benchmark.
3. Corroborate stage comparisons with a combined in-process measurement.

Non-goals:

1. Expose private executor internals or add release instrumentation.
2. Implement an optimization, prove a root cause, or close issue #48.
3. Complete issue #32's general parser, planner, collection, numeric, and format inventory.
4. Change natural-corpus campaign rules or replace native process accounting.

## Decisions

### 1. Use explicit Criterion benchmark targets

Add Criterion as a development dependency with `harness = false` targets. Select and lock a release and minimal feature set compatible with Rust 1.95 during implementation. Record that selection and its transitive dependency implications. Keep first-party unsafe code forbidden.

Place benchmarks beside their consumers. Share only deterministic fixture helpers through existing test support when the dependency graph permits it. Do not create a new workspace crate for this slice or benchmark test support merely for uniform coverage.

An ad hoc timer would require separate sampling and baseline machinery. Full cross-crate coverage would delay the focused investigation. Criterion supplies the needed developer measurement loop within the existing crate boundaries.

### 2. Measure 5 groups with stable identities

Use identities of the form `event-stream/<stage>/<format-or-record-class>/<shape>/<size>`. Record units and boundary descriptions in contributor documentation and the baseline manifest.

| Stage | Existing boundary and home | Included in primary timing | Excluded |
| --- | --- | --- | --- |
| `decode-events` | JSON structural decoder in `tq-formats`; TOON decoder in `tq-toon` | Fresh decoder state, parsing, event construction, bounded consuming sink, teardown | Fixture generation and input file I/O |
| `stream-records` | `stream_json` and `stream_toon` in `tq-formats` | Decode, path projection, jq record materialization, consuming sink, teardown | Fixture generation and query evaluation |
| `execute-events` | Event-plan VM in `tq-core` | Production-equivalent input handle clone, VM construction, synchronous evaluation, effect drain, result consumption, VM/result drop | Fixture decoding and query compilation |
| `encode-results` | Public output sequence API in `tq-formats` | Encoder/sequence setup, ordered writes, sequence finish, bounded byte consumption, teardown | Query execution and expected-output construction |
| `runner` | `run_with_io` in `tq-cli` | Runner preparation, compilation, decoding, execution, framing, encoding, teardown | Argument fixture construction, process launch, filesystem input |

Use a bounded counting/checksum writer whose byte processing is observable through `std::hint::black_box`. Validate exact bytes separately using a fresh buffer outside timing. This keeps encoder work observable without growing an output buffer across iterations. Label the writer cost as included.

Use explicit unframed TOON output for the issue #48 comparison and JSON as an encoding control. Feed both input formats the same logical fixture. Record runner options, capabilities, and environment-sensitive settings. Avoid automatic input detection and unrelated report/trace output.

For execution, include separate accepted-leaf, rejected-leaf, and container-end cases plus a mixed ordered stream. Use the exact catalog query. A VM-construction-and-drop control can help interpret changes, but cannot substitute for the required execution case. Fresh mutable state belongs inside timing where production creates it; use Criterion batching only for separately labeled controls that exclude setup.

The stages overlap. Do not subtract their medians to report an exact stage budget. Changes in cache state, consumer work, and allocation lifetimes make such arithmetic misleading.

### 3. Use bounded synthetic fixtures with independent expectations

Start with 16 and 1024 logical items for nested-object and tabular-compatible shapes. Include top-level scalars that the query accepts, nested leaves it rejects, and non-empty container endings. Keep fixture generation deterministic and offline.

Create JSON and TOON from the same defined logical model before timing. Retain manually defined small expected event/result sequences and companion correctness checks that cover larger fixture construction. Validate values and order, not just counts. Do not define expected output solely by running the function being benchmarked.

Consume structural events without collecting the whole document during timed decoding. Predecode bounded records outside execution timing. Validate complete output with fresh buffers outside encoding and runner timing. Each case's smoke path exercises the same correctness checks.

Report input bytes for decoder and runner throughput, events for record production and evaluation, and output bytes/results for encoding. These synthetic cases characterize operations; the complete frozen USGS corpus remains native campaign evidence.

### 4. Compare equivalent benchmark source on both revisions

Run the same benchmark implementation and fixtures in isolated checkouts of v0.3.0 and the recorded candidate revision. Preserve original production source. Limit historical adapters to benchmark and development-manifest changes; retain the adapter patch and hash with the evidence. Mark an unavailable stage non-comparable instead of changing the older implementation to expose it.

Use the same host, toolchain, target, optimization flags, sampling configuration, and dependency policy. Preserve each revision's production lockfile resolution; record additions required for benchmark dependencies. Dependency differences remain part of the revision comparison and must be disclosed before attributing a difference solely to source changes.

Use a documented benchmark profile with the release optimization settings, including thin LTO and 1 codegen unit, applied equally to both revisions. Do not assume the existing bench profile matches the release profile merely because both are optimized.

Use separate target/output directories to avoid overwriting baseline samples. Retain Criterion raw artifacts and a small evidence manifest with revision, dirty diff, fixture, adapter, lockfile, build, host, and command identities. Repeat runs under controlled host load and report confidence intervals and dispersion. Save developer evidence outside generated cross-tool comparison pages.

Prioritize `execute-events` when collecting the first comparison, then inspect decoding and record construction for format-specific changes. Evidence can identify a stage worth investigating without proving that a particular allocation or lock causes the regression.

### 5. Separate completion of this slice from regression resolution

Implement build and correctness/smoke checks through the shared repository preflight path used by CI. Document full and filtered `cargo bench` commands and baseline workflows. Shared CI does not enforce timing thresholds.

Complete this slice with working groups, correctness checks, contributor guidance, and a documented same-host baseline comparison. A later fix under issue #48 needs repeated native JSON and TOON campaigns on the same frozen corpus, explicit comparable output framing, and the existing independent wall-time/RSS thresholds. No native campaign or production fix is required merely to deliver the benchmark suite, and no microbenchmark result closes #48.

## Risks / Trade-offs

1. Synthetic shapes may miss corpus-specific behavior. Keep that limitation in the evidence and verify any subsequent fix on the complete native corpus.
2. A consuming callback or writer adds measurable work. Keep it stable across revisions and disclose it in each boundary.
3. Setup exclusion can hide the suspected regression. The primary execution case includes construction, effect drain, and destruction.
4. Historical API drift can break comparability. Retain narrow adapters and explicitly exclude cases with mismatched semantics.
5. A benchmark result can encourage premature root-cause claims. Report stage observations separately from hypotheses and confirm causes with focused tests or profiling in the later investigation.

## Migration Plan

Add development-only targets, correctness fixtures, documentation, and CI smoke coverage. Record the initial comparison after validation. Rollback removes those additions without changing CLI behavior or data formats. Keep the proposal active until implementation and verification complete, then synchronize and archive through the normal OpenSpec workflow.
