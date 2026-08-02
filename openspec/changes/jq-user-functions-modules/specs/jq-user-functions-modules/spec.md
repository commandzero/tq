## ADDED Requirements

### Requirement: User-defined filters
The system SHALL parse, resolve, and execute jq-compatible parameterized `def`
filters with lexical scoping, generator cardinality, shadowing, and recursion.

#### Scenario: Recursive parameterized filter
- **WHEN** a query defines and invokes a recursive filter within configured call and work limits
- **THEN** results and ordering match the jq reference without native-stack recursion

#### Scenario: Unknown or invalid call
- **WHEN** a call has no visible definition or the wrong arity
- **THEN** compilation fails before input is consumed with a source-spanned diagnostic

### Requirement: Deterministic modules
The system SHALL support jq-compatible `include` and `import` from explicit
module roots with canonical path confinement, caching, and cycle detection.

#### Scenario: Module import
- **WHEN** an import resolves within a configured root
- **THEN** its definitions and metadata are loaded once with jq-compatible scope

#### Scenario: Escaping or cyclic import
- **WHEN** an import escapes all roots or forms a cycle
- **THEN** compilation fails with the rejected path or complete cycle and consumes no input
