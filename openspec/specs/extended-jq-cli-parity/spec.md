# Extended jq CLI Parity Specification

## Purpose

Define reviewed jq 1.8.x command-line compatibility, explicit TOON/YAML
adaptations, and governed access to scripting integrations.

## Requirements

### Requirement: Reviewed jq option parity
For implementation/evidence closeout only, the Linux/Windows color-presentation and `manual.invoking.run-tests` diagnostic contracts and the 32 Windows newline contracts explicitly enumerated in `cross-tool-compatibility` are transferred to #70/#69. Exact observations and failed acceptance verdicts remain unchanged. This is not an option-implementation waiver, an ANSI/CRLF normalizer, a new disparity approval, or transfer of the macOS test-runner diagnostic obligation.

The system SHALL implement every jq 1.8 option documented by the pinned manual using jq 1.8.2 as the execution reference according to executable cases for argv parsing, input consumption, output bytes, diagnostics, and exit status. The default structured-output representation adaptation SHALL be TOON when no output format or compact option selects JSON. Color presentation SHALL follow the archived `output-colors` main spec's explicit tq contract rather than exact jq default ANSI styling. Unsupported recognized manual options SHALL be implementation gaps, not accepted permanent differences. Explicit tq-only options SHALL retain their documented contracts.

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

### Requirement: jq colors and terminal behavior
Color presentation for every supported output format, including JSON, compact JSON, JSON Lines/NDJSON, and native TOON, SHALL follow the archived `output-colors` main spec. Its shared eight-slot tq default palette, structural quotation-mark styling, `TQ_COLORS` precedence over `JQ_COLORS`, accepted seven/eight-slot overrides, and complete tq fallback for unset/invalid selected palettes SHALL supersede this change's earlier exact-jq-palette requirements. `NO_COLOR`, terminal auto-detection, ordered `-C`/`-M` precedence, and capability denial SHALL follow that main spec; setting a palette alone SHALL NOT enable color. Removing only tq-generated SGR SHALL reproduce exactly the corresponding plain serialization and framing. Raw string and proxy bytes SHALL remain verbatim; structured non-string raw fallback SHALL retain compact JSON and separators with shared palette styling. Color selection alone SHALL NOT change output format. Explicit presentation differences SHALL NOT be reported as exact jq color matches or safe-library disparity approvals.

#### Scenario: Custom palette
- **WHEN** `TQ_COLORS` is unset, `JQ_COLORS` selects a supported seven/eight-slot palette, and JSON color output is forced
- **THEN** scalar/key content uses the configured slots, enclosing quotes use their structural slots, and resets/framing satisfy the `output-colors` contract rather than an exact jq ANSI-byte assertion

#### Scenario: Forced color and environment suppression
- **WHEN** `NO_COLOR`, terminal detection, `-C`, and `-M` interact in documented combinations
- **THEN** tq makes the same color decision as jq for JSON output

#### Scenario: tq palette selection and fallback
- **WHEN** `TQ_COLORS` is set alongside `JQ_COLORS`, including an empty or invalid `TQ_COLORS`
- **THEN** tq selects `TQ_COLORS`, using its complete default on invalid selection rather than falling back to `JQ_COLORS` or jq's default

### Requirement: jq test file execution
`--run-tests` SHALL implement the pinned jq manual's test-file format, filter/input/expected-result processing, compilation-failure cases, comment and separator handling, file or stdin selection, option interactions, result ordering, diagnostics, and exit status. It SHALL run through tq's own evaluator rather than invoking jq. Product-name substitutions in diagnostics SHALL be explicit and limited to identity.

#### Scenario: Valid test file
- **WHEN** `--run-tests` reads a file containing passing filter/result and expected-compilation-failure tests
- **THEN** tq executes the tests and matches the reference's success status and test accounting

#### Scenario: Failing test file
- **WHEN** a result, compile expectation, or test-file structure differs from its expected contract
- **THEN** tq reports the failure and exits with the pinned reference's corresponding status
