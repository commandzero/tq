## MODIFIED Requirements

### Requirement: Reviewed jq option parity
The system SHALL implement every jq 1.8.1 option documented by the pinned manual according to executable cases for argv parsing, input consumption, output bytes, diagnostics, and exit status. The only default structured-output adaptation SHALL be TOON when no output format or compact option selects JSON. Unsupported recognized manual options SHALL be implementation gaps, not accepted permanent differences. Explicit tq-only options SHALL retain their documented contracts.

#### Scenario: Supported option combination
- **WHEN** a documented jq option combination is invoked with JSON output selected
- **THEN** tq matches the reference contract without requiring additional capability-enabling flags

#### Scenario: Invalid combination
- **WHEN** options conflict or arguments are invalid
- **THEN** tq rejects them with the corresponding usage behavior and does not consume input prematurely

### Requirement: Governed scripting integration
The process CLI SHALL admit the environment, terminal, platform, argument-file, and module integrations used by the jq manual without additional allow flags. Embedded library callers SHALL retain explicit capability policy and resource controls. Denied capabilities SHALL fail without leaking ambient data or credentials. Filesystem canonicalization, bounded reads, cancellation, and cleanup SHALL remain enforced; ordinary jq module lookup SHALL not be rejected solely by the previous explicit-root-only CLI policy.

#### Scenario: Broken pipe and multiple files
- **WHEN** ordered files produce results and a downstream reader closes
- **THEN** complete prior frames remain valid, open resources are cleaned up, and tq exits without a noisy pipe error

#### Scenario: Environment from a process CLI
- **WHEN** a manual program accesses `$ENV.PAGER` or `env.PAGER`
- **THEN** tq observes the same controlled process environment as jq without a tq-specific allow flag

#### Scenario: Environment from a restricted library
- **WHEN** an embedded caller denies environment access
- **THEN** the request fails under capability policy without exposing environment contents

## ADDED Requirements

### Requirement: jq colors and terminal behavior
JSON and compact JSON output SHALL implement jq's default palette, `JQ_COLORS` configuration, `NO_COLOR`, terminal auto-detection, and ordered `-C`/`-M` precedence. Invalid or incomplete color configuration SHALL follow the pinned reference's fallback behavior. Raw output and data channels SHALL retain their jq contracts. Native TOON output SHALL remain a separate format contract and SHALL not be silently converted by color selection alone.

#### Scenario: Custom palette
- **WHEN** `JQ_COLORS` selects a custom style/color list and JSON color output is forced
- **THEN** emitted escape sequences, value slots, object-key colors, and resets match jq

#### Scenario: Forced color and environment suppression
- **WHEN** `NO_COLOR`, terminal detection, `-C`, and `-M` interact in documented combinations
- **THEN** tq makes the same color decision as jq for JSON output

### Requirement: jq test file execution
`--run-tests` SHALL implement the pinned jq manual's test-file format, filter/input/expected-result processing, compilation-failure cases, comment and separator handling, file or stdin selection, option interactions, result ordering, diagnostics, and exit status. It SHALL run through tq's own evaluator rather than invoking jq. Product-name substitutions in diagnostics SHALL be explicit and limited to identity.

#### Scenario: Valid test file
- **WHEN** `--run-tests` reads a file containing passing filter/result and expected-compilation-failure tests
- **THEN** tq executes the tests and matches the reference's success status and test accounting

#### Scenario: Failing test file
- **WHEN** a result, compile expectation, or test-file structure differs from its expected contract
- **THEN** tq reports the failure and exits with the pinned reference's corresponding status
