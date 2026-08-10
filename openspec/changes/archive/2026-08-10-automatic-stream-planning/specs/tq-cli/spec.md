## MODIFIED Requirements

### Requirement: Best-effort input detection with strict override
When `--input-format` is absent, tq SHALL probe each structured input source independently with bounded lookahead and replay. Canonical TOON syntax SHALL remain preferred, JSON object and non-TOON array openers SHALL commit to JSON before YAML, and YAML document, directive, or root-sequence markers SHALL commit to YAML. Once a parser commits, later syntax errors SHALL be reported for that format without restarting detection. If every parser rejects, tq SHALL emit a combined input diagnostic containing bounded, useful failure context from each candidate.

`--input-format toon|yaml|json` SHALL select exactly one parser and disable detection/faildown. TOON SHALL remain the default structured output format, while JSON output is available through `--output-format json`.

#### Scenario: Default format
- **WHEN** no format option is provided
- **THEN** bounded syntax probing selects TOON, JSON, or YAML and structured output uses TOON Text Sequence framing

#### Scenario: JSON interoperability
- **WHEN** both input and output formats are explicitly set to JSON
- **THEN** the CLI evaluates the same core program over the JSON data model and emits jq-compatible JSON result texts

#### Scenario: YAML interoperability
- **WHEN** YAML input and JSON output are selected
- **THEN** each accepted YAML document is converted to the shared ordered JSON-shaped value model and evaluated by the same core program

#### Scenario: YAML output remains deferred
- **WHEN** YAML is requested as an output format in the MVP
- **THEN** the CLI reports an unsupported-output-format usage error rather than implying source-preserving YAML support

#### Scenario: Ambiguous content
- **WHEN** input bytes would be valid as both a JSON container and YAML
- **THEN** automatic mode commits an object or non-TOON array opener to JSON, while an explicit override selects only the requested parser

#### Scenario: JSON container enables automatic events
- **WHEN** an eligible query receives a JSON object or array without an input-format override
- **THEN** bounded detection commits to JSON before planning and execution uses JSON decoder events

#### Scenario: Strict input override
- **WHEN** `--input-format json` is supplied for bytes that YAML could also parse
- **THEN** tq invokes only the JSON parser and never probes TOON or YAML

#### Scenario: Mixed-format files
- **WHEN** multiple files are supplied without an input-format override
- **THEN** tq detects each file independently and evaluates the resulting documents in file order

#### Scenario: Override applies to every source
- **WHEN** multiple files are supplied with an input-format override
- **THEN** tq applies only that parser to every structured source and fails on the first source invalid for it

#### Scenario: All probes reject
- **WHEN** TOON, YAML, and JSON all reject before commitment
- **THEN** tq exits with an input error that identifies bounded failure information for all three formats

#### Scenario: Late selected-format failure
- **WHEN** automatic detection commits to a parser and that parser encounters a later syntax error
- **THEN** tq reports that format's parse error and does not reinterpret the source with another parser
