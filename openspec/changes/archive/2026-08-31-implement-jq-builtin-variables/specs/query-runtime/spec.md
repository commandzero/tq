## ADDED Requirements

### Requirement: Special variable execution

The compiled runtime SHALL preserve the immutable source identity and line information needed by `$__loc__`, and SHALL evaluate `$ENV` from the invocation's admitted environment snapshot. These values MUST remain stable across supported document, event, and multi-input execution boundaries.

#### Scenario: Location does not depend on input
- **WHEN** a compiled query containing `$__loc__` is run with null input, one document, or multiple documents
- **THEN** each evaluation returns the same source-location object and does not require input data to construct it

#### Scenario: Environment snapshot is stable across inputs
- **WHEN** a query containing `$ENV` is evaluated for multiple input documents
- **THEN** every evaluation observes the same environment snapshot captured for that invocation

#### Scenario: Denied environment access is a runtime policy failure
- **WHEN** a compiled query containing `$ENV` executes without an admitted environment snapshot
- **THEN** execution returns a stable environment-policy runtime error and does not include environment values in the error or diagnostic context

#### Scenario: Special variables survive plan selection
- **WHEN** a query containing either special variable is compiled and assigned a supported execution plan
- **THEN** the plan evaluates the special variable with the same value and error semantics as document execution
