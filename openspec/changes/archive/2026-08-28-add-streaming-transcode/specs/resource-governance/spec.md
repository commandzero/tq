## MODIFIED Requirements

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

### Requirement: Partial output transparency
If an error occurs after results have already been written, the CLI SHALL
preserve earlier valid framed records and report that execution ended with an
error. Direct transcode output MAY leave an incomplete current framed record and
MUST identify that possibility in plan explanation. Unframed output MUST remain
atomic. The CLI MUST NOT claim atomic sequence output unless an explicit atomic
or fully spooled mode was requested.

#### Scenario: Error after first result
- **WHEN** a filter emits one complete framed result and then raises an error
- **THEN** the first complete record remains on stdout, the diagnostic appears on stderr, and the process exits nonzero

#### Scenario: Error during transcode record
- **WHEN** a transcode decoder fails after writing part of the current sequence record
- **THEN** the CLI reports an incomplete final record and exits nonzero without retracting earlier complete records

#### Scenario: Error during unframed preparation
- **WHEN** a transcode decoder fails while preparing unframed output
- **THEN** no bytes from that result are published and its temporary files are cleaned up

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
