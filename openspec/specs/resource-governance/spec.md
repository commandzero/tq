# Resource Governance Specification

## Purpose

Define bounded execution, honest memory classification, secure spooling,
cancellation behavior, resource diagnostics, and observable resource outcomes.

## Requirements

### Requirement: Pre-execution plan explanation
The system SHALL expose a human-readable and machine-readable explanation of
query capabilities and resource behavior before execution. The explanation MUST
identify transcode, event, subtree, document, whole-input, blocking, and spooling
requirements and the query or I/O condition responsible. A transcode explanation
MUST distinguish direct sequence commitment from atomic unframed preparation.

#### Scenario: Explain identity transcode
- **WHEN** `--explain` is used with an eligible identity conversion
- **THEN** the explanation labels the plan transcode and reports that no root document is materialized

#### Scenario: Explain sort
- **WHEN** `--explain` is used with a sort query
- **THEN** the explanation labels the plan blocking and names the sort expression as the cause

#### Scenario: Explain explicit stream rejection
- **WHEN** a document-only query is combined with explicit stream mode
- **THEN** planning reports incompatibility before reading input

### Requirement: Configurable resource limits
The CLI and library SHALL support configured limits for format-detection lookahead/replay bytes, nesting depth, query/VM stack entries, token or line bytes, in-memory preparation bytes, spool bytes, emitted results, output bytes, and VM execution steps. Defaults MUST be finite for untrusted structural dimensions and documented.

#### Scenario: Result explosion
- **WHEN** a query emits more results than the configured maximum
- **THEN** execution stops with a result-limit diagnostic and nonzero status

#### Scenario: Oversized token
- **WHEN** a single input token exceeds the configured byte limit
- **THEN** decoding fails without allocating the declared/excessive size unchecked

#### Scenario: Detection replay bound
- **WHEN** automatic input detection remains ambiguous beyond the configured replay budget
- **THEN** tq commits according to the documented best-effort precedence or emits a detection-limit diagnostic without retaining unbounded input

### Requirement: Default nesting protection
The default maximum structured-data nesting depth SHALL be 256 unless conformance constraints require a lower value. Query, parser, DOM builder, event path, and writer components MUST apply a coherent depth policy.

#### Scenario: Depth 257
- **WHEN** input nesting exceeds the default maximum
- **THEN** processing fails with a depth-limit error rather than overflowing the native stack

### Requirement: Honest memory classification
The system SHALL describe memory in terms of retained working set rather than
claiming universal streaming. Document mode MAY retain one complete input
document. Slurp, whole-input, and blocking operations MUST be labeled. Event and
transcode modes MUST satisfy their stated bounded-memory contracts. Transcode
classification MUST state whether arrays or unframed publication can consume
temporary disk space proportional to output size.

#### Scenario: General jq filter
- **WHEN** a filter requires a complete root value
- **THEN** `--explain` states that one document is materialized

#### Scenario: Event mode
- **WHEN** an event-compatible query processes a large input
- **THEN** completed input subtrees are releasable and the plan does not retain the root document

#### Scenario: Transcode mode
- **WHEN** an identity transcode processes a large object containing unknown-length arrays
- **THEN** explanation states which object and array state writes directly and which state may spool to configured temporary storage

### Requirement: Safe declared-length handling
Input array lengths, string lengths, counts, or other untrusted numeric declarations MUST NOT be used for unchecked preallocation. Declared counts SHALL be validated incrementally and bounded by configured resource policies.

#### Scenario: Huge declared array
- **WHEN** a TOON header declares an extremely large array but supplies little input
- **THEN** the decoder does not immediately allocate memory proportional to the declaration

### Requirement: Secure bounded spooling
Temporary spooling SHALL use securely created files with restrictive permissions, configurable location, per-run and per-result byte limits, cleanup on success/error/cancellation, and no predictable reusable names.

#### Scenario: Spool limit
- **WHEN** prepared output exceeds the configured spool limit
- **THEN** writing stops with a spool-limit diagnostic and temporary files are cleaned up

#### Scenario: Process interruption
- **WHEN** execution is interrupted while a spool is active
- **THEN** normal cleanup guards remove the temporary artifact on supported termination paths

### Requirement: Cancellation and pipe behavior
The CLI SHALL respond to user interruption and downstream pipe closure without uncontrolled panic or corrupted diagnostics. A normal broken pipe caused by a downstream consumer finishing early SHALL use conventional successful pipeline behavior.

#### Scenario: Downstream closes
- **WHEN** stdout receives a broken-pipe error because a downstream command exits normally
- **THEN** tq stops producing results without printing a Rust panic or treating the condition as data corruption

#### Scenario: User interrupts
- **WHEN** the user sends an interrupt signal
- **THEN** tq stops evaluation promptly, cleans up spools, and returns the documented interrupted status

### Requirement: Partial output transparency
If an error occurs after results have already been written, the CLI SHALL
preserve earlier valid framed records and report that execution ended with an
error. Direct transcode output MAY leave an incomplete current framed record and
MUST identify that possibility in plan explanation. Unframed output MUST remain
atomic. The CLI MUST NOT claim atomic sequence output unless an explicit atomic
or fully spooled mode was requested.

#### Scenario: Error after first result
- **WHEN** a filter emits one result and then raises an error
- **THEN** the first complete record remains on stdout, the diagnostic appears on stderr, and the process exits nonzero

#### Scenario: Error during transcode record
- **WHEN** a transcode decoder fails after writing part of the current sequence record
- **THEN** the CLI reports an incomplete final record and exits nonzero without retracting earlier complete records

#### Scenario: Error during unframed preparation
- **WHEN** a transcode decoder fails while preparing unframed output
- **THEN** no bytes from that result are published and its temporary files are cleaned up

### Requirement: Resource diagnostics
Every resource-limit error SHALL identify the limit name, configured threshold, observed or attempted value when safe, execution phase, and relevant query/input span. Diagnostics MUST avoid dumping unbounded input contents.

#### Scenario: Step limit
- **WHEN** VM execution exceeds the configured step limit
- **THEN** the error identifies the step limit and active query span without printing the full document

### Requirement: Panic-free hostile-input boundary
Malformed queries, malformed TOON/JSON/YAML, excessive declarations, and ordinary runtime type errors SHALL not cause uncontrolled panics in release builds. The benchmark and compatibility harnesses MUST include adversarial cases and fuzz regressions.

#### Scenario: Fuzz regression
- **WHEN** a previously crashing input is added to the regression corpus
- **THEN** the CLI returns a stable diagnostic and the process remains memory-safe

### Requirement: Resource observations in reports
Compatibility and benchmark reports SHALL record active limits, selected or
overridden input format, detection bytes and rejected probes when detection ran,
execution classification, whether materialization or spooling occurred,
high-water marks available from managed stacks and buffers, and the final
resource outcome. Transcode reports MUST add aggregate preparation high-water
bytes, spool bytes written and replayed, and output commitment mode.

#### Scenario: Compare memory behavior
- **WHEN** a benchmark report contains document, event, and transcode cases
- **THEN** each result shows its plan classification and observed peak RSS so unlike memory guarantees are not conflated

#### Scenario: Observe transcode spool
- **WHEN** a transcode benchmark prepares a container on disk
- **THEN** the result records preparation high-water bytes and disk bytes written and replayed

### Requirement: Bounded parallel decode retention
Parallel selected decoding SHALL impose finite limits on queued batches and queued source bytes. Backpressure MUST stop framing additional elements when either bound is reached, and reports SHALL expose configured bounds and observed high-water values.

#### Scenario: Slow earliest batch
- **WHEN** later batches finish while the earliest outstanding batch remains incomplete
- **THEN** the reorder buffer remains within the configured batch and byte limits

### Requirement: Parallel decode cancellation
Cancellation or an ordered failure SHALL stop new batch submission promptly, allow outstanding workers to observe cancellation, and release queued buffers without publishing later results.

#### Scenario: Worker limit failure
- **WHEN** a worker detects a token, numeric, nesting, or cancellation limit failure
- **THEN** submission stops and no later batch is delivered past that failure

### Requirement: Bounded hybrid handoff
A hybrid streaming-blocking plan SHALL bound decoder-event state, in-flight batches, and parallel preparation work. Queue saturation MUST apply backpressure to decoding instead of allocating an unbounded handoff. The plan MAY retain projected results and blocking operator state proportional to result cardinality, but MUST NOT retain unrelated completed input subtrees or the complete root document.

#### Scenario: Large projection from geometry-heavy input
- **WHEN** a hybrid query retains one scalar from each feature in a large GeoJSON document
- **THEN** completed geometry subtrees are releasable and retained memory consists of bounded decoder state, bounded in-flight work, projected values, and blocking state

#### Scenario: Workers trail the decoder
- **WHEN** parallel preparation workers cannot consume batches as fast as the decoder produces them
- **THEN** the decoder waits at the configured in-flight bound rather than growing the queue without limit

#### Scenario: Cancellation with work in flight
- **WHEN** the user interrupts hybrid execution while decoder or preparation work is active
- **THEN** all stages observe cancellation, discard unpublished state, and return the documented interrupted status

### Requirement: Observable hybrid retention
Human-readable and machine-readable explain and execution reports SHALL identify the hybrid plan, streaming prefix, projected path or subtree, blocking suffix, applied optimizer rewrites, configured handoff bounds, and available high-water observations. Reports MUST distinguish root-document materialization from projected-result retention.

#### Scenario: Explain hybrid sort
- **WHEN** `--explain` analyzes an order-sensitive sort over a proven streaming collection
- **THEN** it reports a hybrid streaming-blocking plan and names both the streaming producer and blocking sort cause

#### Scenario: Report hybrid high-water marks
- **WHEN** a hybrid execution report is requested
- **THEN** it records decoder depth, in-flight batches and bytes, retained result count and estimated bytes, blocking state, worker count, and whether the root document was materialized
