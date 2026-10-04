## MODIFIED Requirements

### Requirement: Output formatting controls
TOON output SHALL support indentation, comma/tab/pipe delimiter selection, and safe key folding options. JSON output SHALL support pretty and compact formatting options. JSON Lines output SHALL always be compact and SHALL reject `--pretty-output`, `--indent`, `--tab`, and raw or joined output modes. `--compact-output`, ASCII escaping, and recursive key sorting MAY be combined with JSON Lines output. Color controls SHALL use the format-independent output-colors policy, including for JSON Lines. Incompatible option and format combinations MUST fail before input is consumed.

#### Scenario: Pipe delimiter
- **WHEN** TOON output selects the pipe delimiter
- **THEN** eligible arrays use valid TOON pipe-delimited syntax with correct quoting

#### Scenario: JSON-only compact option
- **WHEN** a JSON-only compact option is applied to TOON output
- **THEN** the CLI reports an incompatible-option usage error

#### Scenario: JSON Lines aliases
- **WHEN** `--output-format jsonl` or `--output-format ndjson` is selected
- **THEN** tq selects the same compact LF-terminated JSON Lines writer

#### Scenario: Pretty JSON Lines conflict
- **WHEN** JSON Lines output is combined with `--pretty-output`, `--indent`, or `--tab`
- **THEN** the CLI reports an incompatible-option usage error before reading input

#### Scenario: Raw JSON Lines conflict
- **WHEN** JSON Lines output is combined with raw or joined output
- **THEN** the CLI reports an incompatible-option usage error before reading input

#### Scenario: Colored JSON Lines
- **WHEN** JSON Lines output is combined with forced color
- **THEN** tq accepts the option and styles compact JSON tokens while preserving the underlying LF-terminated serialization

### Requirement: Native format option compatibility
The CLI SHALL validate format-specific controls before consuming semantic input. All supported native output formats SHALL accept color and monochrome controls as presentation options under the output-colors policy. JSON sequence color SHALL decorate only the document payload and preserve unstyled RS/LF boundaries. JSON sequence output SHALL accept the JSON formatting controls that preserve valid RFC 7464 framing. CSV and TSV output MUST reject JSON-only, TOON-only, raw-output, and joined-output controls unless a control has an explicitly documented delimited-text meaning.

#### Scenario: Pretty JSON sequence
- **WHEN** JSON sequence output uses a compatible JSON indentation control
- **THEN** each frame contains the configured valid JSON document between RS and LF

#### Scenario: Raw CSV conflict
- **WHEN** CSV output is combined with raw or joined output
- **THEN** tq reports an incompatible-option usage error before reading input

#### Scenario: TOON option on TSV
- **WHEN** TSV output is combined with a TOON folding or delimiter option
- **THEN** tq reports an incompatible-option usage error before reading input

#### Scenario: Colored delimited output
- **WHEN** CSV or TSV output is combined with `-C`
- **THEN** headers, scalar fields, delimiters, and quotes use the default theme without changing undecorated field or row bytes

#### Scenario: Colored JSON sequence
- **WHEN** JSON sequence output is combined with `-C`
- **THEN** tq accepts forced color and stripping tq-generated SGR yields valid RFC 7464 output identical to monochrome output
