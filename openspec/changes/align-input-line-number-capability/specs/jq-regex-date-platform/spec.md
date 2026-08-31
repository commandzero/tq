## MODIFIED Requirements

### Requirement: Date and platform built-ins

The system SHALL provide reviewed jq date/time behavior, make decoder-owned
`input_line_number` context available without ambient capability admission, and
govern environment and platform I/O with explicit portability classifications.

#### Scenario: UTC round trip

- **WHEN** an admitted timestamp is parsed, converted, and formatted in UTC
- **THEN** the result matches jq or a documented platform divergence

#### Scenario: Input line number with platform access denied

- **WHEN** a query requests `input_line_number` for active input and platform access is not admitted
- **THEN** evaluation returns the input context's one-based line number instead of a capability-policy error

#### Scenario: Input line number unavailable

- **WHEN** a library evaluation requests `input_line_number` without active or supplied input context
- **THEN** evaluation returns a metadata-unavailable error rather than a platform capability-policy error

#### Scenario: Ambient access denied

- **WHEN** a query requests environment or platform I/O disallowed by policy
- **THEN** it fails without exposing ambient data in diagnostics or reports
