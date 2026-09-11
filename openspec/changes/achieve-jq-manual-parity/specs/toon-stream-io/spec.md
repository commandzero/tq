## MODIFIED Requirements

### Requirement: TOON Text Sequence framing
Explicit TOON sequence output requested with `--seq` SHALL encode every result
as ASCII RS (`0x1e`), followed by one canonical TOON document, followed by LF.
Sequence framing SHALL be distinct from the bytes of a standalone TOON document.

#### Scenario: Emit multiple results
- **WHEN** a filter emits two multiline objects with `--seq`
- **THEN** the output contains two independently parseable RS-framed records in emission order

#### Scenario: Emit zero results
- **WHEN** a filter emits no values with `--seq`
- **THEN** the structured sequence output is empty

#### Scenario: Emit one result
- **WHEN** a filter emits one structured value with `--seq`
- **THEN** the output contains exactly one RS-framed record

### Requirement: Default TOON result output
Without `--seq` or `--unframed`, the CLI SHALL encode zero or more results in emission order. Each result SHALL use canonical TOON followed by LF, without an RS prefix. Cardinality and filter keywords MUST NOT select framing or cause an output error. Zero results SHALL produce empty stdout. Completed results SHALL remain visible if evaluation later fails. Formatting applies to each value; concatenated output is not guaranteed to be one parseable TOON document. Explicit sequence mode provides unambiguous record boundaries.

#### Scenario: Selection emits several values
- **WHEN** `.[] | select(. % 2 == 0)` selects 2 and 4
- **THEN** stdout is `2\n4\n` and execution succeeds without `--seq`

#### Scenario: Selection emits no values
- **WHEN** `empty` or an unmatched `select` emits no values
- **THEN** stdout is empty and ordinary execution succeeds, subject to jq's explicit `-e` exit-status rules

#### Scenario: Prior result survives a later error
- **WHEN** a filter emits 1 and then raises an error
- **THEN** stdout retains `1\n` and execution reports the error
