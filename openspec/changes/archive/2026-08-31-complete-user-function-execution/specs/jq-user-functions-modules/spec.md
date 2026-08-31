## MODIFIED Requirements

### Requirement: User-defined filters
The system SHALL parse, resolve, and execute jq-compatible parameterized `def`
filters with lexical scoping, generator cardinality, shadowing, recursion, and
composition through built-ins that evaluate filter arguments.

#### Scenario: Recursive parameterized filter
- **WHEN** a query defines and invokes a recursive filter within configured call and work limits
- **THEN** results and ordering match the jq reference without native-stack recursion

#### Scenario: User filter passed to a built-in
- **WHEN** a query passes a visible user filter to `map` or another supported built-in that evaluates a filter argument
- **THEN** every invocation uses the user filter's lexical environment and results match jq in value, cardinality, and order

#### Scenario: Unknown or invalid call
- **WHEN** a call has no visible definition or the wrong arity
- **THEN** compilation fails before input is consumed with a source-spanned diagnostic
