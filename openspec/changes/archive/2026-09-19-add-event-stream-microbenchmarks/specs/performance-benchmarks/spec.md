## ADDED Requirements

### Requirement: Event-stream microbenchmark attribution

The developer benchmark suite SHALL provide independently selectable in-process measurements for structural event decoding, jq path/value record production, compiled event-query execution, output encoding and framing, and the combined in-process runner. Decoding and combined groups SHALL cover JSON and TOON separately. Each group SHALL declare its measured operation, input size, included setup, allocation, destruction, and output costs, and applicable throughput units.

Compiled execution measurements SHALL exclude query compilation and fixture decoding. They SHALL include a production-representative measurement of per-event execution setup, evaluation, and teardown. A combined runner measurement SHALL disclose preparation and compilation costs that its callable boundary includes. Stage results SHALL NOT be presented as an additive decomposition of combined time.

#### Scenario: Select an attribution stage
- **WHEN** a developer runs a filtered stage for a fixture size
- **THEN** the suite reports a stable benchmark identity, the measured boundary, and time per operation
- **AND** byte throughput uses that format's actual input size, while event throughput uses its verified event count

#### Scenario: Measure compiled event execution
- **WHEN** the compiled execution group runs on predecoded records
- **THEN** it times per-event setup, evaluation, and teardown without timing compilation or decoding
- **AND** it consumes all results through the production-equivalent synchronous execution mode

### Requirement: Deterministic event-stream fixtures and correctness

Microbenchmarks SHALL use deterministic bounded fixtures prepared before timing without network access. They SHALL cover at least 2 input sizes and nested and tabular-compatible shapes with equivalent JSON and TOON logical values. Synthetic fixtures SHALL be identified as microbenchmark inputs and SHALL NOT be represented as resized natural campaign data.

The workload SHALL include the catalog query `select(length == 2 and (.[0] | length) == 1)`. Execution fixtures SHALL exercise accepted leaf records, rejected leaf records, and container-end records. Before timing each case, the suite SHALL validate its expected event or result sequence and output behavior against an independently defined expectation or companion correctness contract. Counts alone SHALL NOT establish ordered-result correctness. Timed work SHALL consume outputs through an optimization barrier or observable bounded consumer without retaining unbounded results.

#### Scenario: Equivalent input representations
- **WHEN** the JSON and TOON cases use the same fixture shape and size
- **THEN** correctness checks establish their expected ordered stream and filtered results before timing
- **AND** preparation records each representation's byte size and expected event and result counts

#### Scenario: Incorrect result
- **WHEN** a fixture produces an unexpected value, order, count, or output encoding
- **THEN** its correctness or smoke path fails and the suite does not publish a valid timing comparison for that case

### Requirement: Reproducible microbenchmark comparisons

The suite SHALL support repeatable baseline and candidate runs with stable case identities. Comparisons SHALL record source revisions and dirty state, benchmark and fixture identity, dependency lock identity, compiler and target, optimization settings, host and OS, commands, sampling settings, and declared measurement boundaries. They SHALL retain raw samples, dispersion, and statistical uncertainty.

An initial same-host comparison SHALL cover v0.3.0 and the recorded candidate revision for every comparable group. Historical adapters SHALL preserve the measured operation and include their source identity. Cases whose boundaries or semantics cannot be matched SHALL be labeled non-comparable with a reason rather than given a regression percentage. Changing the host, fixture, profile, or boundary SHALL invalidate direct comparison unless equivalence is explicitly established; cross-host timing SHALL NOT support a regression claim. Displayed nanoseconds SHALL NOT imply nanosecond accuracy.

#### Scenario: Compare historical and candidate execution
- **WHEN** the same host runs equivalent compiled execution cases on v0.3.0 and the candidate
- **THEN** the evidence retains both identities, raw measurements, uncertainty, and the measured difference
- **AND** the report distinguishes observed cost changes from unverified root-cause hypotheses

#### Scenario: Historical boundary unavailable
- **WHEN** a historical revision cannot expose an equivalent stage without changing production behavior
- **THEN** the report identifies that case as non-comparable and retains any valid comparisons at broader boundaries

### Requirement: Development-only microbenchmark operation

Benchmark dependencies and instrumentation SHALL remain outside normal production dependencies and runtime behavior. The suite SHALL run through the standard Rust benchmark command and offer documented full, filtered, smoke, and baseline-comparison invocations. CI SHALL compile optimized benchmark targets and execute their correctness/smoke path on the supported toolchain without enforcing shared-runner timing thresholds. First-party benchmark code SHALL preserve the repository's safe-Rust policy.

#### Scenario: Normal release build
- **WHEN** a developer builds the normal release executable
- **THEN** microbenchmark dependencies and instrumentation do not become part of the production dependency graph or execution path

#### Scenario: Shared CI validation
- **WHEN** CI checks a benchmark change
- **THEN** it verifies benchmark compilation and fixture correctness with bounded smoke runs
- **AND** noisy timing differences do not determine pass or fail

### Requirement: Microbenchmark and native evidence separation

Microbenchmark reports SHALL identify their in-process scope and remain separate from cross-tool comparison pages and process resource measurements. They SHALL NOT establish spawn-to-exit wall time, peak RSS, or resolution of a native campaign regression. Native `tq-bench` SHALL remain the authority for those measurements under the existing correctness, provenance, repeated-sample, and regression-threshold requirements.

Any claim that issue #48 is resolved SHALL include repeated same-host native JSON and TOON measurements against v0.3.0 on the same frozen corpus with equivalent explicit output settings. Microbenchmark evidence alone SHALL NOT satisfy that claim. Native Windows support remains deferred to issue #31; the broader cross-crate microbenchmark inventory remains outside this slice of issue #32.

#### Scenario: A microbenchmark becomes faster
- **WHEN** a candidate improves per-event execution time
- **THEN** the report limits its conclusion to that measured operation
- **AND** any end-to-end or RSS improvement claim requires separate native campaign evidence
