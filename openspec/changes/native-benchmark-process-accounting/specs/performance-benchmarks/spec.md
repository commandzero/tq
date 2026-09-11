## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: Direct invocation and timing boundary
The benchmark SHALL launch the selected executable directly with its argument vector and requested environment and working directory. An isolated, low-memory Rust measurement worker MAY perform that launch and own the exact-child measurement after passing the worker-isolation validation below. It MUST NOT insert a shell or external measurement wrapper between itself and the selected executable unless that executable is explicitly the subject of a separate benchmark. Inputs, correctness checks, command construction, and capture destinations SHALL be prepared before timing. A monotonic interval in the process owning the target SHALL begin immediately before spawning and end when the child's exit is observed by its resource-collecting waiter. Worker startup, request and reply transfer, subsequent parsing, worker joins, cleanup, and report generation MUST NOT extend that duration. Spawn failures SHALL be infrastructure failures, not successful zero-duration executions.

Reports SHALL distinguish stored duration resolution and display precision from validated accuracy. Millisecond accuracy SHALL be validated on supported hosts; one-decimal presentation MUST NOT imply accuracy finer than supported by control measurements. Nanosecond storage MUST NOT imply nanosecond accuracy. Instrumentation that measurably distorts timing SHALL use separately labeled timing and memory repetitions with equivalent workloads and identities.

#### Scenario: Literal arguments
- **WHEN** an argument contains spaces, wildcard characters, or shell metacharacters
- **THEN** the executable receives the argument literally without shell interpretation

#### Scenario: Slow preparation or cleanup
- **WHEN** capture setup or post-exit cleanup is deliberately delayed
- **THEN** measured command duration excludes that delay

#### Scenario: Timing calibration
- **WHEN** no-op and known-duration controls are repeated on a native host
- **THEN** validation reports spawn/wait overhead, dispersion, and supported reporting precision without subtracting invented overhead from samples

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
