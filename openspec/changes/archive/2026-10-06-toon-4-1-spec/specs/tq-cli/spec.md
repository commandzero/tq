## ADDED Requirements

### Requirement: TOON 4.1 option surface
The CLI SHALL expose canonical TOON 4.1 indentation, delimiter and strictness controls while retaining existing framing, raw and color modes. It SHALL NOT accept `--fold-keys` or `--flatten-depth`, alias them to another behavior, or silently ignore them. Help and migration documentation SHALL identify literal dotted keys and the removed options. Public TOON configuration SHALL NOT expose key folding or path expansion.

#### Scenario: Removed folding flag
- **WHEN** an invocation supplies `--fold-keys`
- **THEN** argument parsing fails with the ordinary unknown-option contract before input consumption

#### Scenario: Removed flatten-depth flag
- **WHEN** an invocation supplies `--flatten-depth 2`
- **THEN** argument parsing fails before input consumption instead of changing object structure or ignoring the option

#### Scenario: Remaining format controls
- **WHEN** output uses a supported delimiter and indentation with default, sequence or exactly-one unframed framing
- **THEN** canonical 4.1 formatting applies independently of the unchanged selected framing

### Requirement: TOON 4.1 bounded input discovery
Automatic input discovery SHALL recognize valid TOON 4.1 keyed root headers and leading BOM/full-line comments within its configured lookahead without misclassifying keyed headers as JSON arrays or falling down after commitment. Explicit TOON input and `.toon` files SHALL accept root `[]`. A bare `[]` without format selection SHALL retain the existing JSON-first shared-array tie-break, since its value is identical in both formats. Explicit format selection SHALL continue to disable detection/faildown.

#### Scenario: Keyed root header
- **WHEN** automatic input begins with `[2:]{age}:` followed by valid entry rows
- **THEN** bounded discovery selects TOON and evaluation receives an object, not an array

#### Scenario: Comment-prefixed document
- **WHEN** a valid TOON object or header follows BOM and full-line comments within the lookahead bound
- **THEN** discovery and decoding agree without interpreting comment text as scalar data

#### Scenario: Empty root file
- **WHEN** a `.toon` file or explicit TOON stdin contains `[]`
- **THEN** evaluation receives an empty array and canonical default output is `[]` followed by LF

#### Scenario: Shared empty-array syntax
- **WHEN** syntax probing receives bare `[]` without an explicit format or recognized extension
- **THEN** existing JSON-first tie-breaking remains valid and does not change the resulting empty-array value

#### Scenario: Bounded ambiguous prefix
- **WHEN** comments or incomplete headers exhaust the configured probing lookahead
- **THEN** discovery follows its bounded diagnostic/commitment contract without collecting the complete document

## MODIFIED Requirements

### Requirement: Output formatting controls
For implementation/evidence closeout only, the 32 native Windows newline target/case contracts and the Linux/Windows presentation and test-runner contracts in the `cross-tool-compatibility` follow-up map remain unresolved under #69/#70. This qualification applies only to those enumerated observations, including compact/raw/sequence/stream bytes where mapped; no other option, process-effect, framing, input, or default-output behavior is deferred. Captured bytes and both manual gate policies SHALL remain unchanged, without blanket CRLF/ANSI normalization or deferred-as-pass counting.

TOON output SHALL support indentation and comma/tab/pipe delimiter selection. When no output-format selector is supplied and none of `-c`/`--compact-output`, `--seq`, or `--unframed` is supplied, structured output SHALL emit zero or more canonical TOON values, each followed by LF without RS. `--seq` SHALL select JSON Text Sequence input. With default TOON output or explicit TOON output (`-o toon`/`--output-format toon`), it SHALL select RS-framed TOON Text Sequence output; with JSON output (`-o json`/`--output-format json` or `-c`), it SHALL select RS-framed JSON Text Sequence output. Other explicit native structured outputs SHALL retain their native framing. `-o toon-seq`/`--output-format toon-seq` SHALL select TOON sequence output without changing input selection. `-o json` SHALL select JSON, pretty by default. `-c` and `--compact-output`, including bundled short options, SHALL select compact JSON without requiring `-o json`. An explicit TOON output selection combined with compact JSON SHALL fail before input consumption regardless of argument order. Explicit JSON selection with `-c` SHALL be valid in either order. JSON Lines output SHALL remain compact and SHALL reject `--pretty-output`, `--indent`, `--tab`, and raw or joined output modes. JSON Lines and its NDJSON alias SHALL accept color under the archived `output-colors` main spec, including automatic terminal color and forced `-C`; removing tq-generated SGR SHALL preserve the plain compact LF-terminated bytes. Consumers requiring directly parseable JSON Lines SHALL use redirected automatic output or `-M`. Compact output, ASCII escaping, and recursive key sorting MAY be combined with JSON Lines output. Incompatible options MUST fail before input is consumed. An output-format selector MUST NOT silently select an input parser; `--seq` explicitly selects JSON sequence input and framing only for TOON or JSON output.

#### Scenario: Default TOON
- **WHEN** `tq '.'` runs without output options
- **THEN** it emits each canonical TOON value followed by LF without RS; zero results produce empty stdout and multiple results succeed

#### Scenario: Explicit TOON sequence output
- **WHEN** `tq --seq '.'` runs with default TOON output
- **THEN** it emits one RS-framed canonical TOON record for each result and preserves completed records before a later error

#### Scenario: Compact JSON selector
- **WHEN** `tq -c '.'` or `tq --compact-output '.'` processes structured input
- **THEN** it emits jq-compatible compact JSON with LF-separated records and no TOON framing

#### Scenario: Compact option combinations
- **WHEN** compact output is combined with `-o json`, `-r`, `-j`, `-S`, or a bundled option such as `-cr`
- **THEN** JSON and raw output behavior matches the corresponding jq flags

#### Scenario: JSON-only compact option
- **WHEN** `-c -o toon` or `-o toon -c` is supplied
- **THEN** the CLI reports an incompatible-option usage error before reading input

#### Scenario: Pipe delimiter
- **WHEN** TOON output selects the pipe delimiter
- **THEN** eligible arrays use valid TOON pipe-delimited syntax with correct quoting

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
- **AND** with terminal capability permitted, JSON Lines and its NDJSON alias use the shared tq palette, and removing generated SGR yields exactly the corresponding plain compact records and LF framing

### Requirement: Native format option compatibility
The CLI SHALL validate format-specific controls before consuming semantic input. All supported native output formats SHALL accept color and monochrome controls as presentation options under the output-colors policy. JSON sequence color SHALL decorate only the document payload and preserve unstyled RS/LF boundaries. JSON sequence output SHALL accept the JSON formatting controls that preserve valid RFC 7464 framing. CSV and TSV output MUST reject JSON-only, TOON-only, raw-output, and joined-output controls unless a control has an explicitly documented delimited-text meaning.

#### Scenario: Pretty JSON sequence
- **WHEN** JSON sequence output uses a compatible JSON indentation control
- **THEN** each frame contains the configured valid JSON document between RS and LF

#### Scenario: Raw CSV conflict
- **WHEN** CSV output is combined with raw or joined output
- **THEN** tq reports an incompatible-option usage error before reading input

#### Scenario: TOON option on TSV
- **WHEN** TSV output is combined with a TOON delimiter option
- **THEN** tq reports an incompatible-option usage error before reading input

#### Scenario: Colored delimited output
- **WHEN** CSV or TSV output is combined with `-C`
- **THEN** headers, scalar fields, delimiters, and quotes use the default theme without changing undecorated field or row bytes

#### Scenario: Colored JSON sequence
- **WHEN** JSON sequence output is combined with `-C`
- **THEN** tq accepts forced color and stripping tq-generated SGR yields valid RFC 7464 output identical to monochrome output
