## ADDED Requirements

### Requirement: Sound automatic plan selection
The system SHALL select event, subtree, document, whole-input, or blocking plans
before input consumption and SHALL preserve jq result, ordering, and error semantics.

#### Scenario: Eligible projection
- **WHEN** analysis proves a query depends only on bounded event-local paths
- **THEN** the compiler produces an event plan without retaining the complete document

#### Scenario: Ineligible query
- **WHEN** a query requires whole-document, whole-input, mutation, or blocking state
- **THEN** it selects the corresponding retaining plan without speculative partial output

### Requirement: Observable bounded retention
Automatic plans MUST expose their proof causes, retained working set, limits,
and high-water observations.

#### Scenario: Natural large regression
- **WHEN** an eligible query processes the versioned natural large corpus
- **THEN** compatibility passes and peak RSS stays within its manifest-aware regression gate
