## ADDED Requirements

### Requirement: Reduce semantics
The system SHALL execute jq `reduce` with lexical accumulator scope, ordered
generator consumption, and jq-compatible update multiplicity and errors.

#### Scenario: Ordered reduction
- **WHEN** a generator emits zero or more values into a reduction
- **THEN** the initial value and each update produce the same final cardinality and values as jq

### Requirement: Foreach semantics
The system SHALL execute jq `foreach` with independent initialization, update,
and extraction filters in the same order and scope as jq.

#### Scenario: Intermediate extraction
- **WHEN** `foreach` updates an accumulator and extracts multiple intermediate results
- **THEN** all results are emitted in jq order and retain valid earlier frames on a later error

### Requirement: Managed fold resources
Fold execution MUST use bounded managed continuations and report retained state.

#### Scenario: Fold limit
- **WHEN** a fold crosses its configured call, work, result, or output limit
- **THEN** it terminates with the stable resource class and cleans up all continuations
