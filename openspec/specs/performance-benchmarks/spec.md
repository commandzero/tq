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
Each valid benchmark sample SHALL capture monotonic spawn-to-exit wall duration and process exit status. Native macOS and Linux hosts MUST additionally collect the specific child's OS-recorded peak resident memory, user CPU, system CPU, and output bytes. Time to first result SHALL be reported for cases where it can be observed without changing semantics, with its observation method and precision stated. Peak RSS SHALL describe process resident memory including its threads, not heap allocation, physical footprint, or simultaneous process-tree peak. Independent process peaks MUST NOT be summed.

The recorded resource scope SHALL disclose validated pre-exec launch and waited-descendant effects. Native lifetime RSS MUST NOT be described as a pure post-exec metric. Campaign-runner memory MUST NOT determine the target's reported peak; a launch floor that obscures supported small-tool workloads SHALL block acceptance. Estimated parent or worker RSS MUST NOT be subtracted from the OS maximum.

Every authoritative campaign SHALL run outside restricted sandboxes with the permissions needed for native child accounting and SHALL complete an allocation preflight before corpus preparation. Preflight MUST verify positive OS peak RSS and explicit collection provenance through the same interface used by measured rows. It MUST NOT require a polling dwell or successful `ps` sampling unless a separately selected diagnostic or limit requires sampling. Collection failure, missing provenance, invalid units, overflow, or unavailable RSS SHALL invalidate the campaign and prevent publication of a valid comparison report. Diagnostics MAY retain the failure and unavailable fields; zero and sampled substitutes MUST NOT be fabricated. Provenance MUST identify the collector actually used, not be inferred from OS identity or a positive count.

Native process accounting SHALL replace mandatory `time` wrappers and recurring `ps` sampling for primary timing and RSS. Platform `time` SHALL remain an independent validation tool. Explicitly requested sampled diagnostics or RSS-limit enforcement SHALL state their scope, interval, and potential to miss peaks; they MUST NOT replace the OS peak metric. Unsupported native platforms, including Windows deferred to issue #31, SHALL fail preflight clearly without implying verification.

#### Scenario: Restricted process inspection
- **WHEN** permissions prevent native child resource collection on a supported host
- **THEN** the campaign aborts before corpus work or valid report publication and requires a rerun with working permissions

#### Scenario: Linux GNU time provenance
- **WHEN** a native Linux campaign measures a tool and separately validates its counter against GNU `time -v`
- **THEN** measured rows record native collector provenance and normalized bytes, and independent validation records retain `gnu-time-v` provenance

#### Scenario: macOS BSD time provenance
- **WHEN** a native macOS campaign measures a tool and separately validates its counter against BSD `time -l`
- **THEN** measured rows record native collector provenance and normalized bytes, and independent validation records retain `bsd-time-l` provenance

#### Scenario: Measured RSS disappears
- **WHEN** a preflight-passing campaign receives a measured invocation without valid positive OS peak RSS or explicit provenance
- **THEN** the harness aborts immediately without publishing a partial valid benchmark comparison and preserves an infrastructure diagnostic

#### Scenario: Large stream case
- **WHEN** an explicit stream benchmark runs
- **THEN** the report includes OS peak RSS and available time to first result in addition to total throughput, identifying latency observation precision

#### Scenario: Metric unavailable
- **WHEN** an operating system cannot provide a requested metric
- **THEN** optional metrics are marked unavailable, while unavailable authoritative RSS invalidates the campaign and prevents memory comparisons

#### Scenario: Primary run without sampling tools
- **WHEN** a native accounting preflight succeeds and no diagnostic sampling or sampled limit is selected
- **THEN** primary measurements succeed without `ps` or `/usr/bin/time` installed

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
Once a tq case has an accepted baseline, repeated local campaigns SHALL detect statistically meaningful regressions against tq's compatible baseline when machine, corpus, measurement contract, and command behavior are comparable. Cross-tool jq/yq ratios SHALL remain comparative evidence, not tq self-regressions. Campaigns SHALL remain locally reproducible without requiring full matrices in CI.

For issue #30 acceptance, each comparable workload's wall-time and peak-RSS increases above 20% SHALL be disclosed independently with baseline, candidate, samples, dispersion, and explanation. Documented increases greater than 20% and at most 50% SHALL be acceptable; an increase above 50% in either metric SHALL block acceptance until mitigated and remeasured. Exactly 20% does not cross the disclosure threshold; exactly 50% does not cross the blocking threshold. Other metrics or workloads MUST NOT hide a regression through aggregation. Earlier measurements with different timing boundaries or unverified memory provenance SHALL be marked non-comparable; they MUST NOT be used to claim that the new method improved tq itself. This issue-specific acceptance policy supersedes earlier refactor thresholds for this change only.

#### Scenario: tq regresses
- **WHEN** a stable comparable issue #30 workload exceeds 50% increase in wall time or peak RSS
- **THEN** acceptance fails and the report identifies the affected metric and supporting repeated measurements

#### Scenario: tq beats or trails a reference
- **WHEN** tq is faster or slower than jq or yq without violating its own thresholds
- **THEN** the report presents the cross-tool ratio without labeling it a tq self-regression

#### Scenario: Disclosed acceptable regression
- **WHEN** a comparable workload increases by more than 20% and at most 50% in either metric
- **THEN** the report discloses and explains the increase and permits acceptance when all other gates pass

#### Scenario: Changed measurement contract
- **WHEN** the previous report includes wrapper overhead or sampled memory and the candidate uses native accounting
- **THEN** the report retains both methods distinctly and requires equivalent-method baseline measurements before asserting a tq self-regression or improvement

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

### Requirement: Direct invocation and timing boundary
The benchmark SHALL launch the selected executable directly with its argument vector and requested environment and working directory. An isolated, low-memory Rust measurement worker MAY perform that launch and own the exact-child measurement after passing the worker-isolation validation below. It MUST NOT insert a shell or external measurement wrapper between itself and the selected executable unless that executable is explicitly the subject of a separate benchmark. Inputs, correctness checks, command construction, and capture destinations SHALL be prepared before timing. A monotonic interval in the process owning the target SHALL begin immediately before spawning and end when the child's exit is observed by its resource-collecting waiter. Worker startup, request and reply transfer, subsequent parsing, worker joins, cleanup, and report generation MUST NOT extend that duration. Spawn failures SHALL be infrastructure failures, not successful zero-duration executions.

Reports SHALL distinguish stored duration resolution and display precision from validated accuracy. Acceptance SHALL use comparisons within the same native host and OS with equivalent harness versions, timing boundaries, instrumentation families, and disclosed concessions; it SHALL NOT require a universal 1 ms accuracy guarantee. Cross-OS runtime comparisons or rankings MUST NOT be produced. One-decimal presentation MUST NOT imply accuracy finer than supported by evidence, and nanosecond storage MUST NOT imply nanosecond accuracy. Instrumentation that measurably distorts timing SHALL use separately labeled timing and memory repetitions with equivalent workloads and identities.

Known-duration controls SHALL report observed excess duration and dispersion. Full spawn-to-exit duration minus requested sleep includes legitimate startup, sleep overshoot, shutdown, and exit-observation delay; it MUST NOT be labeled isolated timer error or a guaranteed error bound. Host-specific effects are acceptable when all compared tools use the same measurement contract and concessions. Repeated samples and dispersion remain required; a shared harness MUST NOT be treated as proof that noise or bias cancels. CPU/RSS validation and tq self-regression gates remain unchanged.

#### Scenario: Literal arguments
- **WHEN** an argument contains spaces, wildcard characters, or shell metacharacters
- **THEN** the executable receives the argument literally without shell interpretation

#### Scenario: Slow preparation or cleanup
- **WHEN** capture setup or post-exit cleanup is deliberately delayed
- **THEN** measured command duration excludes that delay

#### Scenario: Timing calibration
- **WHEN** no-op and known-duration controls are repeated on a native host
- **THEN** validation reports spawn/wait overhead, dispersion, and supported reporting precision without subtracting invented overhead from samples

#### Scenario: Same-OS comparison with observed scheduling effects
- **WHEN** repeated controls show excess duration on a native host and all compared tools use the same harness, timing boundaries, instrumentation family and disclosed concessions
- **THEN** comparisons within that host and OS may proceed without a universal 1 ms accuracy guarantee, retaining observed control values and dispersion
- **AND** reports neither label the control excess as isolated timer error nor use it to compare tools across operating systems

### Requirement: Isolated measurement worker
An internal Rust worker MAY isolate target launching from the campaign coordinator's memory. It SHALL initialize its own executable image before launching the target and SHALL return the target's exact-child resource usage, never its own lifetime usage. Bulk input and capture preparation SHALL remain outside the worker's target-launch memory footprint. Control messages SHALL be bounded and preserve the existing public invocation contract without loading corpus-sized input or reports into the worker before target launch.

The worker SHALL remain the single owner of target completion and resource collection. Coordinator management of worker completion MUST NOT race target reaping. Startup, communication, cancellation, and cleanup SHALL be bounded. Worker absence, failure, malformed or truncated replies, and lost control channels SHALL fail closed with retained diagnostics. Target and process-group cleanup SHALL be verified for worker and coordinator failure. No failure MAY silently fall back to the contaminated in-process launch path.

Reports and calibration SHALL identify the worker executable, launch protocol, collector sources, resource scope, and residual launch-floor evidence. Changed workers or protocols SHALL invalidate calibration and prevent incompatible comparisons. Worker adoption SHALL require native parent-memory independence, request-sequence isolation, and independent platform-time validation on both hosts. This permission does not establish that a worker is a verified solution; inability to meet these conditions SHALL require another design decision.

#### Scenario: Coordinator memory changes
- **WHEN** an identical small target is measured with page-touched coordinator allocations of zero, 32 MiB, and a larger corpus-representative size
- **THEN** its RSS remains independent of coordinator allocation within declared page-aware tolerances and agrees with independent native time controls

#### Scenario: Large prepared input and repeated requests
- **WHEN** a target receives large prepared stdin or a low-memory request follows a high-memory request
- **THEN** exact input delivery is preserved and worker payload copies or prior requests do not determine the target's RSS

#### Scenario: Worker overhead is delayed
- **WHEN** worker startup, request transfer, reply transfer, or teardown is deliberately delayed
- **THEN** the measured target interval excludes those delays and still uses the target owner's monotonic clock

#### Scenario: Worker control failure
- **WHEN** the worker fails or its control channel is lost while a target is running
- **THEN** bounded cleanup leaves no live target or descendant, diagnostics identify the infrastructure failure, and no valid measurement is published

#### Scenario: Worker changes after calibration
- **WHEN** a worker executable or launch protocol differs from the independently validated identity
- **THEN** old calibration cannot authorize the new measurement path and fresh native controls are required

### Requirement: Single-owner child resource lifecycle
One measurement lifecycle SHALL own child completion, resource collection, timeout, cancellation, forced termination, and cleanup. The resource collector MUST target the specific child and collect its usage during reaping; ordinary waits MUST NOT consume status first. Cumulative accounting across prior children MUST NOT be used. Normal exit, nonzero exit, signal, timeout, output limit, and configured memory-limit outcomes SHALL remain distinct. All paths SHALL close input delivery and bound process and worker cleanup without zombies or double reaping. Collection errors SHALL retain available exit diagnostics without manufacturing metrics.

#### Scenario: Independent repeated children
- **WHEN** a high-memory child is followed by a low-memory child or independent samples overlap
- **THEN** each result belongs to its exact child and excludes accounting from unrelated samples

#### Scenario: Timeout races with exit
- **WHEN** the deadline and child exit occur together
- **THEN** one owner resolves the outcome, collects available usage once, and completes cleanup without a second reap

#### Scenario: Forced termination with blocked input
- **WHEN** a child does not consume stdin and exceeds its deadline or output limit
- **THEN** termination, pipe closure, and worker cleanup complete within bounded time and report the forced outcome with collected usage or explicit collection failure

#### Scenario: Normal and abnormal exit
- **WHEN** helpers exit successfully, exit nonzero, or terminate by signal
- **THEN** status and usage are associated with the correct helper and no zombie remains

### Requirement: Native accounting validation
macOS and Linux SHALL exercise the same measurement interface through safe Rust APIs. Dependency selection MUST audit lifecycle semantics, target support, resource units, maintenance, licensing, and transitive dependencies. First-party unsafe code or FFI bridges SHALL NOT be introduced; failure to find suitable safe APIs SHALL require a separate design decision before implementation proceeds.

The workspace SHALL declare a minimum Rust version of 1.95 to support the selected `wait4 0.2.0` dependency. The consuming wait API and dependency-internal interrupted-call retries SHALL preserve the existing exact-child observation, cleanup, and reap ordering. Raising MSRV SHALL NOT be treated as evidence that Linux pre-exec RSS inheritance is resolved.

The approved dependency MAY rely on valid, nonnegative OS counters within its audited numeric range when its internal conversions expose no overflow error. This assumption and its limits MUST be documented; the implementation MUST NOT claim it can detect every malformed upstream counter after conversion. First-party conversions SHALL use checked arithmetic and reject detectable invalid, zero-RSS, unavailable, or overflowed results. Independent native validation remains mandatory.

Validation SHALL use helpers that allocate and touch pages, release memory, and exit, including bursts shorter than the previous sampling interval without an intentional polling dwell. Multiple allocation sizes, independent children, and allocations across threads SHALL validate peak behavior and platform conversion. Tolerances SHALL account for page size, runtime baseline, and allocator behavior rather than equating heap bytes to RSS. Native `time` comparisons SHALL independently validate accounting on both OSes. Cross-compilation or emulation alone MUST NOT satisfy native verification.

Native versus independent-time user and system CPU comparisons SHALL classify each absolute difference as a percentage of native target spawn-to-exit wall duration. At most 10% SHALL be automatic green; above 10% through 20% SHALL be green with an info notice; above 20% but below 50% SHALL be yellow with a warning requiring explicit approval; 50% or more SHALL be red with an acceptance block and automatic investigation by the implementing agent. Below 500 ms, differences strictly less than the larger of 20 ms and 10% of runtime SHALL override these bands to automatic green. Exactly 500 ms SHALL use the percentage bands without the short-run floor. Zero difference SHALL be green; for zero duration, differences outside the short-run floor SHALL be red. Validation SHALL use unrounded measurements, exclude worker and time-wrapper overhead from the duration basis, and retain severity and diagnostics. Yellow SHALL NOT authorize calibration without recorded approval. Red SHALL require investigation and resolution before acceptance. This policy SHALL NOT change RSS tolerances, timing-accuracy claims, or tq self-regression thresholds. Earlier failed evidence SHALL remain unchanged; reevaluation SHALL identify the new policy separately.

#### Scenario: CPU validation tolerance boundaries
- **WHEN** paired CPU measurements differ by 30 ms for a 300 ms native target duration
- **THEN** the CPU comparison is automatically green, and a difference above 30 ms through 60 ms is green with an info notice
- **AND** at 500 ms duration, differences of 50 ms, 100 ms, 100.001 ms, and 250 ms are respectively green, green with info, yellow requiring approval, and red requiring investigation

#### Scenario: Short-run allowance overrides severity
- **WHEN** a target runs for 30 ms and CPU differs by 19.999 ms
- **THEN** the short-run floor makes the comparison automatically green
- **AND** a difference of exactly 20 ms falls outside the floor and is red under the percentage bands

#### Scenario: Brief released allocation
- **WHEN** a helper touches and releases a large allocation before exiting without waiting for a sampler
- **THEN** the OS high-water mark captures the expected increase above a matched control within documented page-aware tolerances

#### Scenario: Platform units and threads
- **WHEN** multiple allocation sizes and concurrent thread allocations are tested on each native OS
- **THEN** normalized bytes and process scope are verified against controls and independent native `time` evidence

#### Scenario: Platform unverified
- **WHEN** native Linux or macOS validation has not run, or only Windows is available
- **THEN** missing native evidence remains unverified and issue #30 acceptance is incomplete; Windows work remains deferred

### Requirement: Native benchmark evidence publication
The supported jq/yq/tq workload matrix SHALL be rerun on verified native macOS and Linux hosts with correctness gates outside timed repetitions, warmups, repeated samples, tool/input identity, equivalent commands, and separate comparison families preserved. Reports SHALL record timing method, validated precision, RSS collector and process scope, availability, instrumentation, and host identity. Old sampled and wrapper measurements SHALL retain their original provenance and MUST NOT be silently relabeled or pooled with the new method.

Stable `docs/tests/comparison/` Results blocks SHALL be updated with the new metric names while preserving authored explanations outside generated regions. Measurement cells SHALL include their units and exactly one decimal place, including time, memory, throughput, and percentages; integer counts SHALL remain integers. Metric headers SHALL use plain names without a source annotation. Source labels such as `gnu-time-v`, `bsd-time-l`, or native collector identifiers MUST NOT appear in table cells or headers. Provenance SHALL remain in machine-readable reports and measurement-method descriptions outside tables. Rounding SHALL affect presentation only, never stored samples, aggregation, or regression decisions.

Unavailable, unsupported, failed, unmeasured, or non-comparable measurement cells SHALL display `-`. Text before each affected table SHALL explain that `-` denotes no valid comparable measurement rather than zero and describe applicable exclusions or failures. Detailed outcome classifications SHALL remain in machine-readable reports; coverage tables MAY retain named outcomes and integer counts. User-facing pages MUST NOT acquire dated report filenames or local benchmark-archive paths. Failures and non-comparable rows SHALL remain visible without being ranked as valid timings or memory comparisons.

#### Scenario: Refresh stable comparison pages
- **WHEN** verified native campaign results are published
- **THEN** stable Results blocks show spawn-to-exit duration and OS-recorded process peak RSS with one decimal and units in each measurement cell, provenance is described outside tables, and authored explanations remain unchanged

#### Scenario: Mixed measurement units
- **WHEN** one table contains duration, resident memory, and throughput rows
- **THEN** its cells display values such as `138.6 ms`, `64.7 MiB`, and `42.0 MiB/s`, without source annotations or extra decimal places

#### Scenario: Placeholder explanation
- **WHEN** a table includes unsupported, failed, unavailable, unmeasured, or non-comparable results
- **THEN** affected measurement cells contain `-`, preceding text explains the placeholder and applicable reasons, and exact outcome classifications remain available in the machine-readable report

#### Scenario: Presentation rounding preserves decisions
- **WHEN** a value near an acceptance threshold rounds to the same displayed value as that threshold
- **THEN** acceptance uses the unrounded measurement while the cell displays one decimal place and its unit

#### Scenario: Historical record loaded
- **WHEN** a prior sampled or `time`-wrapper report is read
- **THEN** its collection method remains explicit or unknown, and it is excluded from incompatible new-method comparisons

#### Scenario: One native host is missing
- **WHEN** only one required native platform has a verified campaign
- **THEN** reports identify that coverage accurately and do not claim the complete matrix acceptance criterion passed
