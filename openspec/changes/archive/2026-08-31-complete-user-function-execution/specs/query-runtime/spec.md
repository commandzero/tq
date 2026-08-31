## MODIFIED Requirements

### Requirement: Validated bytecode
Compiled programs SHALL use an immutable bytecode representation with validated
instruction boundaries, constant/function references, stack effects, jumps,
fork targets, and executable operation closure. Validation MUST traverse the
root expression, referenced user-function bodies, and filter arguments.
Execution MUST reject malformed bytecode produced outside trusted constructors.

#### Scenario: Compile branch
- **WHEN** a conditional is compiled
- **THEN** bytecode validation confirms all branches target valid instructions and have compatible stack invariants

#### Scenario: Compile composed user filter
- **WHEN** a valid user filter is reachable through a built-in filter argument
- **THEN** validation confirms the complete reachable operation graph has an execution path before constructing the compiled program

#### Scenario: Runtime lacks a reachable operation
- **WHEN** trusted compilation finds a source operation that the selected runtime cannot execute
- **THEN** compilation fails before input with a source-spanned stable capability diagnostic and does not expose an internal bytecode operation error

#### Scenario: Corrupt bytecode
- **WHEN** a test constructs an invalid jump target through a test-only decoder
- **THEN** validation rejects the program before the VM executes it
