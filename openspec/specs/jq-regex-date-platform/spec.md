# jq Regex, Date, and Platform Specification

## Purpose

Define bounded jq-compatible regex and UTC date/time behavior alongside
explicitly governed environment and platform-dependent built-ins.

## Requirements

### Requirement: Regex built-ins

The system SHALL provide bounded jq-compatible regex match, capture, scan,
split, and substitution operations with documented Unicode behavior.

#### Scenario: Regex match and captures

- **WHEN** a supported pattern and flags are applied to a string
- **THEN** matches, offsets, captures, null captures, and ordering satisfy the reviewed jq contract

#### Scenario: Unsupported or excessive regex

- **WHEN** syntax is unsupported or configured work/input limits are exceeded
- **THEN** execution returns a stable unsupported or resource diagnostic

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
