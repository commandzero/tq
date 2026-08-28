## MODIFIED Requirements

### Requirement: Execution capability analysis
Analysis SHALL classify a program's input and working-set requirements as
semantic identity, event-stream, subtree, document, whole-input, and/or
blocking. The resulting metadata MUST be available before input consumption and
included in the compiled program. Final plan selection MAY combine this metadata
with decoder and output-writer capabilities, but it MUST NOT weaken the query
requirements recorded by analysis.

#### Scenario: Semantic identity
- **WHEN** the resolved program returns each input value unchanged exactly once
- **THEN** analysis records semantic identity so output-aware planning may consider transcode

#### Scenario: Simple event consumer
- **WHEN** a program consumes jq-style stream path/value events without document operators
- **THEN** analysis permits an event execution plan

#### Scenario: Sort
- **WHEN** a program contains `sort` over a generated collection
- **THEN** analysis marks it blocking and document/subtree materializing as applicable

#### Scenario: Slurp
- **WHEN** CLI slurp mode is combined with a query
- **THEN** the execution plan is classified whole-input

### Requirement: Mode-safe execution plans
A compiled program SHALL be converted into a typed transcode, document, or event
plan before execution. A transcode plan MUST carry proof of semantic identity and
compatible decoder and writer capabilities. Explicit event mode MUST reject
programs requiring document values before reading input.

#### Scenario: Construct transcode plan
- **WHEN** semantic-identity analysis and compatible I/O capabilities are present
- **THEN** the planner constructs a typed transcode plan that cannot enter the document VM by accident

#### Scenario: Event-incompatible query
- **WHEN** an ordinary document update is requested in explicit event mode
- **THEN** planning fails with a capability diagnostic and no input bytes are consumed
