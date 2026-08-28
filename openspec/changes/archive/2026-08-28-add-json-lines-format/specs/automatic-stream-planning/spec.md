## MODIFIED Requirements

### Requirement: Sound automatic plan selection
The system SHALL select event, subtree, document, whole-input, or blocking plans before semantic decoder consumption and SHALL preserve jq result, record ordering, and error semantics. A bounded format probe MAY precede selection when every probed byte is replayed unchanged to the selected decoder. A recognized JSON Lines file extension or explicit JSON Lines override SHALL allow eligible event and subtree plans, and each record SHALL have an independent decoder root and execution boundary.

#### Scenario: Eligible projection
- **WHEN** analysis proves a query depends only on bounded event-local paths
- **THEN** the compiler produces an event plan without retaining the complete document

#### Scenario: Auto-detected decoder input
- **WHEN** an eligible query receives JSON or TOON through the default automatic input format
- **THEN** bounded probing and replay select the corresponding decoder-event plan without requiring an input-format override

#### Scenario: JSON Lines event plan
- **WHEN** an eligible event query receives a `.jsonl` or `.ndjson` file containing multiple records
- **THEN** tq runs the event plan independently for each record and preserves record and result order

#### Scenario: JSON Lines subtree plan
- **WHEN** an eligible subtree query receives JSON Lines input containing multiple records
- **THEN** tq resets capture state at each record boundary and does not retain completed records

#### Scenario: Ineligible query
- **WHEN** a query requires whole-document, whole-input, mutation, or blocking state
- **THEN** it selects the corresponding retaining plan without speculative partial output
