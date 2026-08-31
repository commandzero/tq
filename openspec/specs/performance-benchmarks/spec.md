# Performance Benchmarks Specification

## Purpose

Define correctness-gated, reproducible performance campaigns across natural
datasets, native formats, resource metrics, and reviewed regression policies.

## Requirements

### Requirement: Correctness-gated performance cases
Every timed benchmark case SHALL reference a passing compatibility case or a separately reviewed result digest. A tool MUST pass its correctness gate for the exact corpus snapshot before its timing is considered valid.

#### Scenario: Correct result
- **WHEN** a tool's normalized result and exit behavior match the case contract
- **THEN** the harness may execute and report timed samples

#### Scenario: Incorrect result
- **WHEN** a tool produces a faster but semantically different result
- **THEN** the harness records `incorrect` and does not include its timing in comparative summaries

### Requirement: Baseline-first performance campaign
The first performance milestone SHALL run jq and yq across all applicable benchmark cases before tq performance implementation or optimization begins. tq SHALL join a benchmark case only after the corresponding compatibility capability passes.

#### Scenario: Record reference baseline
- **WHEN** the corpus, compatibility runner, and benchmark harness are ready
- **THEN** the project records jq/yq wall-time and resource baselines on a manifest-recorded local benchmark host before implementing tq language features

#### Scenario: Incremental tq admission
- **WHEN** tq gains a new compatible capability
- **THEN** only benchmark cases whose correctness gates now pass become eligible for tq timing

### Requirement: Natural dataset campaigns
Performance campaigns SHALL use complete natural corpus artifacts and SHALL report their observed byte sizes and logical record counts. The harness MUST NOT resize a dataset to fit labels such as small, medium, or large.

#### Scenario: USGS sizes drift
- **WHEN** refreshed earthquake feeds contain different counts from a previous campaign
- **THEN** the report uses the new exact sizes/counts and associates results with the new snapshot manifest

#### Scenario: Large natural file
- **WHEN** the large campaign runs
- **THEN** it uses the entire configured large dataset and records completion, timeout, OOM, or resource-limit outcomes for every applicable tool

### Requirement: Benchmark workload breadth
The benchmark manifest SHALL include cases for process startup/query compilation, parse-and-discard, scalar extraction, multi-result projection, selective filtering, numeric/string reduction, array/object construction, path update, blocking sort, explicit event streaming, and identity decode/re-encode. It MUST include both small-output and output-heavy workloads.

#### Scenario: Startup benchmark
- **WHEN** the startup case runs
- **THEN** it measures complete process invocation with a trivial valid input/query and does not reuse an already running process

#### Scenario: Blocking benchmark
- **WHEN** a sort or collection-building case runs
- **THEN** the report labels it blocking and does not compare its memory behavior to an event-stream guarantee

#### Scenario: Output-heavy benchmark
- **WHEN** identity or a large transformation is benchmarked
- **THEN** required serialization and writes to the configured sink remain inside the timed interval

### Requirement: Same-format and native-format comparisons
The benchmark suite SHALL include the complete native input-support matrix: jq, yq, and tq on JSON; yq and tq on YAML; and tq on TOON. The matrix SHALL be applied to every workload and corpus tier whose representation is lossless and whose execution mode is supported by the parser. tq parser-specific benchmark commands MUST pass the corresponding `--input-format` override so automatic precedence cannot cause JSON to exercise YAML or add detection overhead. For same-format cases, participating tools MUST consume the identical input artifact, implement the same logical query, and satisfy the same normalized result contract. Tools without native support for a format SHALL be marked not applicable, and the harness MUST NOT insert conversion into the timed command.

The suite SHALL separately report the native-format end-to-end comparison of jq on validated JSON, yq on validated YAML, and tq on validated TOON representations of the same logical snapshot. Reports MUST include both physical input throughput and logical records throughput and MUST NOT infer format-independent superiority from physical MiB/s alone.

#### Scenario: Direct JSON comparison
- **WHEN** a JSON benchmark case runs
- **THEN** jq, yq, and tq are correctness-gated and timed against the same JSON bytes and normalized result contract

#### Scenario: Direct YAML comparison
- **WHEN** a YAML benchmark case runs
- **THEN** yq and tq are correctness-gated and timed against the same YAML bytes and normalized result contract while jq is marked not applicable

#### Scenario: TOON input benchmark
- **WHEN** a TOON benchmark case runs
- **THEN** tq is correctness-gated and timed on TOON while jq and yq are marked not applicable without timed conversion

#### Scenario: Compare tq input adapters
- **WHEN** a document-mode workload has validated JSON, YAML, and TOON representations
- **THEN** tq is correctness-gated and timed separately on all three inputs and the report presents its format-specific startup, throughput, and peak-memory results together

#### Scenario: Pin the measured tq parser
- **WHEN** the harness times tq for a JSON, YAML, or TOON format row
- **THEN** the command supplies the matching `--input-format` override and records it so the row measures that parser rather than automatic detection

#### Scenario: Compare native tools
- **WHEN** a native-format case completes for all three tools
- **THEN** the report shows each input's byte size, record count, output size, wall time, records/s, and physical MiB/s together

#### Scenario: Representation size differs
- **WHEN** YAML, JSON, and TOON payload sizes differ substantially
- **THEN** the report retains the difference and does not normalize physical throughput by pretending the files have equal sizes

#### Scenario: Separate comparison families
- **WHEN** a report contains same-format and native-format results
- **THEN** it presents them in separate labeled tables and does not combine their ratios into one ranking

### Requirement: Resource and latency metrics
Each valid benchmark sample SHALL capture wall-clock duration and process exit status. Supported local benchmark hosts MUST additionally capture user CPU, system CPU, peak resident memory, output bytes, and time to first result for cases where the harness can observe it without changing semantics.

#### Scenario: Large stream case
- **WHEN** an explicit stream benchmark runs
- **THEN** the report includes peak RSS and time to first result in addition to total throughput

#### Scenario: Metric unavailable
- **WHEN** an operating system cannot provide a requested metric
- **THEN** the report marks that metric unavailable rather than substituting zero

### Requirement: Statistically useful sampling
The harness SHALL use warmups and repeated samples appropriate to the natural input size. The default policy MUST run at least 30 measured samples for startup/small cases, 10 for medium cases, and 3 for large cases unless a reviewed campaign time budget explicitly lowers the count.

#### Scenario: Run small case
- **WHEN** a small benchmark completes normally
- **THEN** the report includes at least 30 measured process invocations and reports median plus dispersion

#### Scenario: Large campaign time budget
- **WHEN** a large case would exceed the campaign's reviewed time budget
- **THEN** the report records the reduced sample count and excludes it from comparisons requiring the default sample policy

### Requirement: Environment manifest
Every performance report SHALL include timestamp, OS/kernel, architecture, CPU model and logical/physical core counts when available, total memory, filesystem, power/performance settings when observable, tool paths/versions/digests, compiler profile for tq, corpus manifests, warmup/sample policy, timeout, resource limits, and command lines with secrets removed.

#### Scenario: Compare two reports
- **WHEN** reports come from different machine or corpus identities
- **THEN** the presentation visibly marks them non-comparable by default

### Requirement: Failure preservation
Timeout, OOM kill, signal termination, unsupported syntax, failed correctness gate, and configured resource-limit exhaustion SHALL be first-class benchmark outcomes. The harness MUST NOT drop them from summaries.

#### Scenario: yq cannot materialize large YAML
- **WHEN** yq is killed or exceeds the configured limit on a large case
- **THEN** the report records the failure with elapsed time and available resource observations instead of omitting yq

### Requirement: tq performance regression policy
Once a tq case has an accepted baseline, repeated local performance campaigns SHALL detect statistically meaningful regressions against tq's own compatible baseline when the machine and corpus manifests are comparable. Cross-tool jq/yq numbers SHALL remain comparative evidence rather than a universal pass/fail requirement. Running these campaigns in CI SHALL NOT be required during MVP development.

#### Scenario: tq regresses
- **WHEN** a stable benchmark exceeds its configured regression threshold with sufficient samples
- **THEN** the local campaign exits unsuccessfully and identifies the changed metric and confidence/dispersion evidence

#### Scenario: tq beats or trails a reference
- **WHEN** tq is faster or slower than jq or yq without violating its own release thresholds
- **THEN** the report presents the measured ratio without converting it into an unqualified project-wide winner claim

### Requirement: Large streaming memory objective
On the manifest-recorded local large-campaign host, tq explicit event-stream cases over a naturally approximately 1 GiB input SHALL complete within a configured 128 MiB process RSS objective, excluding filesystem page cache, unless the case manifest declares a larger current token or result bound. The campaign SHALL be locally reproducible and opt-in during development rather than dependent on CI.

#### Scenario: Stream-compatible large query
- **WHEN** tq runs a correctness-approved event-stream query over the large corpus
- **THEN** it completes without document materialization and the report evaluates peak RSS against the 128 MiB objective

### Requirement: Identity-transcode performance campaign
The benchmark suite SHALL include correctness-gated JSON-to-TOON identity cases
for wide root objects, nested objects, root arrays, nested arrays, scalar arrays,
and tabular candidates. Every case MUST compare transcode with a forced-document
baseline and record wall time, CPU time, peak RSS, output bytes, output commitment
mode, preparation high-water bytes, spool bytes, and time to first output byte
when output can commit incrementally. Sequence cases MUST also record time to the
first payload byte so an early framing flush cannot hide delayed conversion.

#### Scenario: Cross-plan correctness gate
- **WHEN** an identity-transcode case is admitted for timing
- **THEN** transcode and forced-document output bytes and exit classifications match for the exact corpus snapshot

#### Scenario: Shape-sensitive report
- **WHEN** a campaign contains object-heavy and array-heavy inputs
- **THEN** the report presents each shape separately and includes preparation and spool observations beside time and RSS

#### Scenario: Unframed disk cost
- **WHEN** an unframed transcode result exceeds the in-memory publication threshold
- **THEN** the report includes temporary bytes written and replayed rather than presenting low RSS without its disk cost

### Requirement: Identity-transcode memory objectives
On the manifest-recorded Apple Silicon benchmark host, the accepted natural
`recovery` and `segments` identity cases SHALL each complete below 64 MiB peak
process RSS in direct TOON sequence mode. Array-heavy cases SHALL demonstrate
that peak RSS remains within the case's configured bounded-retention objective as
input size grows, while reports record proportional spool growth separately.
These campaigns SHALL remain reproducible and opt-in rather than required in CI.

#### Scenario: Wide-object memory gate
- **WHEN** the accepted natural `recovery` or `segments` identity case runs through direct sequence transcode on the recorded host
- **THEN** semantic correctness passes and peak process RSS is below 64 MiB

#### Scenario: Array scaling gate
- **WHEN** correctness-equivalent root-array inputs increase in element count beyond the in-memory preparation threshold
- **THEN** spool bytes may grow with input size but peak RSS stays within the configured case objective

#### Scenario: Document baseline remains visible
- **WHEN** transcode satisfies its memory objective but is slower than forced-document execution
- **THEN** the report retains both timings and does not hide the CPU or disk trade-off

### Requirement: Single-pass latency and throughput gate
The accepted direct-sequence JSON campaign SHALL read each source once without
whole-source staging or duplicate prevalidation. Reports SHALL compare throughput
and first-payload latency with both forced document execution and the recorded
streaming `toon` baseline.

#### Scenario: No whole-source prepass
- **WHEN** a direct-sequence JSON benchmark completes
- **THEN** input-stage bytes are zero and first-byte latency does not scale with total source length before decoding begins

### Requirement: Parallel selected-decoding campaign
The benchmark suite SHALL compare one-worker and multi-worker tq execution for a correctness-approved blocking projection over the largest catalogued JSON input. Each row MUST record wall time, user CPU, system CPU, total CPU, peak RSS, effective worker count, and output digest.

#### Scenario: Multi-worker comparison
- **WHEN** the parallel selected decoder is benchmarked with one and fourteen effective workers on the recorded host
- **THEN** the report presents both measurements and their wall-time, CPU-time, and memory ratios

#### Scenario: Parallel candidate regresses
- **WHEN** multi-worker execution does not materially improve wall time or violates correctness or memory bounds
- **THEN** the report preserves the result and the optimization does not silently replace the serial path for that workload

### Requirement: Hybrid blocking benchmark coverage
The benchmark catalogue SHALL include correctness-gated workloads in which a small projection from a large structured document feeds an order-sensitive blocking operator. Reports MUST distinguish document, single-thread hybrid, and configured multi-thread hybrid execution and capture wall time, user CPU, system CPU, total CPU, peak resident memory, and effective worker count.

#### Scenario: Large GeoJSON projected sort
- **WHEN** the large GeoJSON campaign evaluates a sort over projected feature metadata
- **THEN** jq, single-thread tq, and multi-thread tq run against the same source and result contract, and the report records each required timing and memory metric

#### Scenario: Compare hybrid with document baseline
- **WHEN** both hybrid and document execution are available for the same tq query
- **THEN** the report labels their retained-state guarantees and presents wall-time, CPU-time, and peak-memory differences without treating the hybrid plan as bounded-memory event execution

### Requirement: Optimizer-resistant performance cases
A benchmark intended to measure a blocking operator MUST make that operator's result observable under its correctness contract. The harness MUST record optimizer rewrites and MUST reject or relabel a sample when the measured operator was removed before execution.

#### Scenario: Cardinality ignores sorted order
- **WHEN** a candidate benchmark ends in `sort | length` and array length is its only observable result
- **THEN** the harness does not label it a blocking-sort measurement if explain output reports dead-sort elimination

#### Scenario: Sorted content is observed
- **WHEN** a blocking-sort benchmark passes its correctness gate
- **THEN** the expected result depends on sorted content and explain output confirms that the sort remained in the executed plan

### Requirement: Issue 5 comparative performance objective
The benchmark suite SHALL include representative correctness-gated JSON workloads for the issue #5 builtin families. On a manifest-recorded host, the report SHALL evaluate a soft objective that tq's median wall-clock duration is no more than 2.0 times jq's and tq's maximum observed peak resident memory is no more than 1.5 times jq's. Each comparison MUST use the same input, equivalent query, output sink, warmup and sampling policy, and complete process invocation. A miss SHALL remain visible for investigation but MUST NOT fail implementation acceptance or override tq's own regression policy.

#### Scenario: Both objectives are met
- **WHEN** a representative tq workload has a median wall-time ratio at or below 2.0 and a maximum observed peak-RSS ratio at or below 1.5 relative to jq
- **THEN** the report marks both issue #5 objectives as met and retains the measured values and ratios

#### Scenario: Time objective is missed
- **WHEN** tq's median wall time exceeds 2.0 times jq's for a comparable workload
- **THEN** the report marks the soft time objective as missed without failing the correctness gate or the benchmark campaign

#### Scenario: Memory objective is missed
- **WHEN** tq's maximum observed peak RSS exceeds 1.5 times jq's for a comparable workload
- **THEN** the report marks the soft memory objective as missed without failing the correctness gate or the benchmark campaign

#### Scenario: Comparison is not valid
- **WHEN** correctness, host identity, corpus identity, output behavior, or a required metric differs or is unavailable
- **THEN** the report marks the objective not comparable rather than claiming it passed or failed
