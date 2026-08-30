## ADDED Requirements

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
