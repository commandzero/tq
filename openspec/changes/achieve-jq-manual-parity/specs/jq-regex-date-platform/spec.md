## MODIFIED Requirements

The matching requirements below are subject to the reviewed safe-library disparity contract in `cross-tool-compatibility`. Use existing Rust libraries through safe APIs; do not introduce native FFI engines or unsafe bridges for exact matching. Test actual engine work limits and cancellation between bounded operations. Document measured syntax, match-selection, platform, and cancellation limitations without treating them as exact matches or permitting unbounded hostile-pattern execution.

### Requirement: Regex built-ins
The system SHALL provide bounded jq-compatible `test`, `match`, `capture`, `scan`, `split`, `splits`, `sub`, and `gsub`, including every argument form and flag combination documented by the pinned manual. Array-supplied pattern/flag forms, null flags, scoped inline flags, extended-mode whitespace, longest-match mode, multiline/singleline behavior, and empty-match suppression SHALL follow jq. Offsets and lengths SHALL use jq's Unicode units. Replacement filters SHALL preserve capture scope, generator cardinality, and ordering rather than silently selecting a single replacement.

#### Scenario: Regex match and captures
- **WHEN** a supported pattern and flags are applied to a string
- **THEN** matches, offsets, captures, null captures, and ordering satisfy the reviewed jq contract

#### Scenario: Unsupported or excessive regex
- **WHEN** the pinned reference rejects a pattern or configured work/input limits are exceeded
- **THEN** execution returns the corresponding regex error or a stable resource diagnostic; documented jq syntax is not rejected merely because the previous engine lacked it

#### Scenario: Array and null argument forms
- **WHEN** a manual case calls `match(["foo", "ig"])`, `test(["foo"])`, or `split(", *"; null)`
- **THEN** argument interpretation, output shape, and ordering match jq

#### Scenario: Inline flags and longest mode
- **WHEN** a pattern uses scoped `(?i)`/`(?-i)`, extended whitespace handling, or flag `l`
- **THEN** it follows jq's match selection and scope rules rather than accepting an engine-specific approximation

#### Scenario: Replacement generator
- **WHEN** `[gsub("p"; "a", "b")]` or a capture-based `sub` emits multiple replacement choices
- **THEN** jq's complete output sequence is preserved, including empty branches and errors

### Requirement: Date and platform built-ins
The system SHALL provide the pinned manual's date/time behavior and policy-governed environment/platform I/O with explicit platform identities. The CLI SHALL permit the documented ambient operations by default; library callers SHALL retain denial controls. `input_filename` and `input_line_number` SHALL report the current consumed input's metadata, including values consumed by `input` and `inputs`, rather than retaining the first input's location. Clock and local-time tests SHALL use a controlled platform contract rather than literal wall-clock equality between separate processes.

#### Scenario: UTC round trip
- **WHEN** an admitted timestamp is parsed, converted, and formatted in UTC
- **THEN** the result matches the pinned jq build on the same platform

#### Scenario: Ambient access denied
- **WHEN** an embedded query requests environment or platform I/O disallowed by policy
- **THEN** it fails without exposing ambient data in diagnostics or reports

#### Scenario: Remaining input changes location
- **WHEN** a filter consumes another line or file through `input` or `inputs`
- **THEN** subsequent metadata queries identify the same source and line as jq

#### Scenario: Input line number with platform access denied
- **WHEN** a query requests `input_line_number` for active input and platform access is not admitted
- **THEN** evaluation returns the input context's one-based line number instead of a capability-policy error

#### Scenario: Input line number unavailable
- **WHEN** a library evaluation requests `input_line_number` without active or supplied input context
- **THEN** evaluation returns a metadata-unavailable error rather than a platform capability-policy error

#### Scenario: Platform-specific date behavior
- **WHEN** a manual date format depends on the C library or timezone
- **THEN** the gate compares matched platform settings and preserves any reference error as an executable contract
