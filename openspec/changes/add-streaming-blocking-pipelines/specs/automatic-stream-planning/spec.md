## MODIFIED Requirements

### Requirement: Sound automatic plan selection
The system SHALL select transcode, event, subtree, hybrid streaming-blocking, document, whole-input, or blocking-document plans before semantic decoder consumption and SHALL preserve jq result, ordering, and error semantics. A hybrid plan MUST separate a proven bounded-retention producer from a suffix that consumes retained blocking state. A bounded format probe MAY precede selection when every probed byte is replayed unchanged to the selected decoder.

#### Scenario: Eligible projection
- **WHEN** analysis proves a query depends only on bounded event-local paths
- **THEN** the compiler produces an event plan without retaining the complete document

#### Scenario: Auto-detected decoder input
- **WHEN** an eligible query receives JSON or TOON through the default automatic input format
- **THEN** bounded probing and replay select the corresponding decoder-event plan without requiring an input-format override

#### Scenario: Eligible streaming collection with blocking suffix
- **WHEN** analysis proves that a collection producer can evaluate independently for each child under a static decoder path and that the remaining query consumes only the collected results
- **THEN** the compiler produces a hybrid streaming-blocking plan that retains the collected results and blocking state without retaining the complete input document

#### Scenario: Ineligible query
- **WHEN** a query requires whole-document, whole-input, mutation, or blocking state and analysis cannot prove a sound producer and suffix split
- **THEN** it selects the corresponding document or whole-input retaining plan without speculative partial output

### Requirement: Hybrid plan proof
A hybrid streaming-blocking plan MUST carry a pre-input proof of the decoder path prefix, per-item projection or subtree requirement, collection boundary, blocking suffix, value escape behavior, and syntax cause. If any required proof is unavailable, planning MUST fall back to a sound retaining plan before semantic input consumption.

#### Scenario: Static projected collection
- **WHEN** the query collects `.features[].properties.release` before an order-sensitive blocking suffix
- **THEN** the proof identifies `.features` as the iterated decoder prefix, `.properties.release` as the per-item projection, and the collected array as the blocking suffix input

#### Scenario: Missing projected member
- **WHEN** an iterated item lacks a statically projected member and jq semantics produce `null`
- **THEN** hybrid execution contributes the same `null` result that document execution contributes

#### Scenario: Dynamic path prevents proof
- **WHEN** the producer uses a dynamic path or cross-item operation that the analyzer cannot prove independent
- **THEN** the analyzer rejects the hybrid plan and records the rejection reason
