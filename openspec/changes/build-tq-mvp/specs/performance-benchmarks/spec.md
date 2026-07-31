## ADDED Requirements

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
