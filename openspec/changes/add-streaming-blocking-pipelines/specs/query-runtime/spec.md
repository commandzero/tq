## MODIFIED Requirements

### Requirement: Execution capability analysis
Analysis SHALL classify a program's input and working-set requirements as transcode, event-stream, subtree, hybrid streaming-blocking, document, whole-input, and/or blocking-document. The resulting metadata MUST be available before input consumption and included in the compiled program.

#### Scenario: Simple event consumer
- **WHEN** a program consumes jq-style stream path/value events without document operators
- **THEN** analysis permits an event execution plan

#### Scenario: Sort over a streamable collection
- **WHEN** a program constructs an array from an independently streamable projection and applies an order-sensitive `sort` suffix
- **THEN** analysis permits a hybrid streaming-blocking plan whose retained state is the projected collection rather than the complete source document

#### Scenario: Sort
- **WHEN** a program applies `sort` to a value whose production requires the complete input document
- **THEN** analysis marks it blocking-document and materializes the document as applicable

#### Scenario: Slurp
- **WHEN** CLI slurp mode is combined with a query
- **THEN** the execution plan is classified whole-input

## ADDED Requirements

### Requirement: Mode-safe hybrid execution
A compiled hybrid plan SHALL represent its streaming producer, collection boundary, and blocking suffix as validated typed components. Execution MUST evaluate producer results in jq encounter order, pass only completed owned values across the boundary, and invoke the suffix exactly once with the same collected value document execution would construct.

#### Scenario: Mixed projected values
- **WHEN** a hybrid producer emits nulls, booleans, numbers, strings, arrays, and objects
- **THEN** the blocking suffix receives the same ordered array of values as document execution

#### Scenario: Producer error after completed items
- **WHEN** item evaluation fails after earlier items have entered the blocking collection
- **THEN** execution returns the jq-compatible error without publishing a result from the blocking suffix

#### Scenario: Stable equal-value sort
- **WHEN** equal-comparing values enter parallel sort preparation in different batches
- **THEN** their encounter order is preserved wherever the accepted jq baseline makes that order observable

### Requirement: Semantics-preserving blocking rewrites
The compiler MAY remove a blocking operator only when resolved query structure and input-type proof establish that the operator cannot change the result sequence, values, ordering, or jq-visible errors. Explain output MUST identify each applied rewrite.

#### Scenario: Array sort followed by length
- **WHEN** a resolved query applies the built-in `sort` and then `length` to a value proven to be an array
- **THEN** the compiler may evaluate `length` without sorting and reports the removed blocking operation in explain output

#### Scenario: Sort input is not proven array
- **WHEN** the input to `sort | length` is not statically proven to be an array
- **THEN** the compiler retains `sort` so its jq-compatible type error remains observable

#### Scenario: Sort key can fail
- **WHEN** a sort-like operation evaluates a user expression or otherwise has jq-visible evaluation effects
- **THEN** the compiler retains that operation unless it separately proves those effects unobservable
