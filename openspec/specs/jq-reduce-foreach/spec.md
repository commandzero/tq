# jq Reduce and Foreach Specification

## Purpose

Define jq-compatible stateful `reduce` and `foreach` folds with bounded managed
execution state and stable resource behavior.

## Requirements

### Requirement: Reduce semantics
The system SHALL execute jq `reduce` with lexical accumulator scope, ordered generator consumption, scalar or nested array/object destructuring bindings, and jq-compatible update multiplicity and errors. Binding patterns SHALL use the same semantics as ordinary `as` expressions. Saved accumulator branches and input values SHALL not be mutated by subsequent branches.

#### Scenario: Ordered reduction
- **WHEN** a generator emits zero or more values into a reduction
- **THEN** the initial value and each update produce the same final cardinality and values as jq

#### Scenario: Destructured accumulator update
- **WHEN** `reduce .[] as [$i,$j] (0; . + $i * $j)` or the manual's object-pattern reduction is evaluated
- **THEN** variable binding, missing-field behavior, accumulator values, and errors match jq

#### Scenario: Generator emits several update states
- **WHEN** a reduction's initializer or update emits zero or multiple results
- **THEN** all surviving accumulator branches have jq's cardinality, order, and independent state

### Requirement: Foreach semantics
The system SHALL execute jq `foreach` with independent initialization, update, and optional extraction filters in the same order and scope as jq. Omitted extraction SHALL expose the updated accumulator. Destructuring and lexical variables SHALL behave consistently with `reduce` and ordinary bindings.

#### Scenario: Intermediate extraction
- **WHEN** `foreach` updates an accumulator and extracts multiple intermediate results
- **THEN** all results are emitted in jq order and retain valid earlier frames on a later error

#### Scenario: Omitted extraction
- **WHEN** `foreach .[] as $item (0; . + $item)` consumes an array
- **THEN** it emits jq's sequence of updated accumulators without requiring a third argument

#### Scenario: Bound variable in extraction
- **WHEN** the extraction constructs `{index: ., $item}`
- **THEN** the output uses the current item binding and accumulator independently for every iteration

### Requirement: Managed fold resources
Fold execution MUST bound its accumulator and evaluation state, report retained
state, and preserve jq-observable ordering, cardinality, errors, and cleanup.
The internal evaluator strategy MAY vary provided those behaviors and resource
contracts remain stable.

#### Scenario: Fold limit
- **WHEN** a fold crosses its configured call, work, result, or output limit
- **THEN** it terminates with the stable resource class and cleans up all fold state
