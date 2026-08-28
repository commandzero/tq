## ADDED Requirements

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

