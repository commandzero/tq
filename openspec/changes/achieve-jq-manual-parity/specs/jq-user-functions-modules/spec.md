## MODIFIED Requirements

### Requirement: User-defined filters
The system SHALL parse, resolve, and execute jq-compatible parameterized `def` filters with lexical scoping, generator cardinality, shadowing, recursion, and composition through built-ins that evaluate filter arguments. Filter parameters SHALL capture their definition-site lexical environment while consuming each invocation's input. Value parameters and variable shorthand SHALL preserve jq's distinction between evaluating a filter and capturing its results.

#### Scenario: Recursive parameterized filter
- **WHEN** a query defines and invokes a recursive filter within configured call and work limits
- **THEN** results and ordering match the jq reference without native-stack recursion

#### Scenario: User filter passed to a built-in
- **WHEN** a query passes a visible user filter to `map` or another supported built-in that evaluates filter arguments
- **THEN** every invocation uses the user filter's lexical environment and results match jq in value, cardinality, and order

#### Scenario: Unknown or invalid call
- **WHEN** a call has no visible definition or the wrong arity
- **THEN** compilation fails before input is consumed with a source-spanned diagnostic

#### Scenario: Captured filter inside nested mapping
- **WHEN** `def addvalue(f): f as $f | map(. + $f); map(addvalue(.foo))` executes
- **THEN** callback input, captured bindings, and result cardinality match jq rather than reusing another branch's environment

### Requirement: Deterministic modules
The process CLI SHALL support jq-compatible `include`, filter `import`, JSON data `import`, default and explicit search paths, startup definitions, search substitutions, metadata search restrictions, and dependency metadata. Module lookup SHALL follow the pinned reference's order and error rules, including relative module origin, `~`, `$ORIGIN`, repeated path components, search termination, and both single-file and directory forms. Compilation SHALL preserve caching, cycle detection, bounded reads, and source identity. Embedded callers SHALL retain canonical confinement to their explicitly allowed roots, independently of the normal CLI lookup contract.

#### Scenario: Module import
- **WHEN** an import resolves in an admitted search root
- **THEN** its definitions and metadata are loaded once with jq-compatible scope

#### Scenario: Escaping or cyclic import
- **WHEN** an import escapes an embedded caller's allowed roots or forms a cycle
- **THEN** compilation fails with the rejected path or complete cycle and consumes no data input

#### Scenario: JSON data import
- **WHEN** `import "data" as $data; $data::data` loads the manual data fixture
- **THEN** tq matches jq's namespace, value shape, file parsing, and result without treating the data as a filter module

#### Scenario: Default startup module
- **WHEN** the CLI starts with a controlled home directory containing the startup file used by the pinned jq reference
- **THEN** its definitions are available under the same lookup conditions as jq, including explicit library-path overrides

#### Scenario: Dependency metadata
- **WHEN** `modulemeta` inspects a module with filter and data dependencies
- **THEN** metadata fields, dependency ordering, aliases, and search metadata match jq

#### Scenario: Invalid repeated path and terminated search
- **WHEN** a module identifier contains a rejected repeated component or its search metadata terminates lookup
- **THEN** tq follows the reference failure or resolution rather than searching unrelated roots
