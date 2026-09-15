## MODIFIED Requirements

### Requirement: TOON Text Sequence framing
Explicit TOON sequence output requested with `-o toon-seq` (or
`--output-format toon-seq`), or with `--seq` when TOON output is selected (the
default or `-o toon`/`--output-format toon`), SHALL encode every result
as ASCII RS (`0x1e`), followed by one canonical TOON document, followed by LF.
Sequence framing SHALL be distinct from the bytes of a standalone TOON document.

#### Scenario: Emit multiple results
- **WHEN** a filter emits two multiline objects with `--seq` using default TOON output or `-o toon`/`--output-format toon`
- **THEN** the output contains two independently parseable RS-framed records in emission order

#### Scenario: Emit zero results
- **WHEN** a filter emits no values with `--seq` using default TOON output or `-o toon`/`--output-format toon`
- **THEN** the structured sequence output is empty

#### Scenario: Emit one result
- **WHEN** a filter emits one structured value with `--seq` using default TOON output or `-o toon`/`--output-format toon`
- **THEN** the output contains exactly one RS-framed record

### Requirement: Default TOON result output
When structured TOON output is selected by default or explicitly, and neither sequence framing nor `--unframed` is requested, the CLI SHALL encode zero or more results in emission order. Each result SHALL use canonical TOON followed by LF, without an RS prefix. Cardinality and filter keywords MUST NOT select framing or cause an output error. Zero results SHALL produce empty stdout. Completed results SHALL remain visible if evaluation later fails. Formatting applies to each value; concatenated output is not guaranteed to be one parseable TOON document. Explicit sequence mode provides unambiguous record boundaries.

#### Scenario: Selection emits several values
- **WHEN** with default structured TOON output, `.[] | select(. % 2 == 0)` selects 2 and 4
- **THEN** stdout is `2\n4\n` and execution succeeds without `--seq`

#### Scenario: Selection emits no values
- **WHEN** with default structured TOON output, `empty` or an unmatched `select` emits no values
- **THEN** stdout is empty and ordinary execution succeeds, subject to jq's explicit `-e` exit-status rules

#### Scenario: Prior result survives a later error
- **WHEN** with default structured TOON output, a filter emits 1 and then raises an error
- **THEN** stdout retains `1\n` and execution reports the error
