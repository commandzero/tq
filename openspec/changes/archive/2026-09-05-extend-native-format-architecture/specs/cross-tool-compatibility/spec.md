## ADDED Requirements

### Requirement: Native format compatibility coverage
The compatibility manifest SHALL include jq-target RFC 7464 cases and yq-peer CSV and TSV cases. Coverage MUST compare ordered document values, output bytes where framing or quoting matters, warnings, stable error classes, and exit status without coercing string and numeric values during normalization.

#### Scenario: jq JSON sequence recovery
- **WHEN** a JSON sequence fixture contains valid, malformed, and valid frames
- **THEN** the jq and tq observations compare emitted values, warning classification, continuation, and successful exit status

#### Scenario: JSON sequence output bytes
- **WHEN** jq and tq emit multiple results under sequence output
- **THEN** the compatibility report compares RS prefixes, JSON payloads, LF terminators, and result order

#### Scenario: JSON sequence publication before failure
- **WHEN** jq and tq process a complete value followed by invalid trailing bytes or partial structural events followed by malformed syntax
- **THEN** the suite compares observations published before failure, recovery, root reset, and both warning and `--stream-errors` behavior

#### Scenario: Multiple roots share a recovery segment
- **WHEN** jq and tq receive several valid roots between RS bytes, with a separate case containing malformed syntax before a later root in the same segment
- **THEN** the suite compares all input modes, per-Document root reset, and the pinned jq parser's same-segment reset behavior rather than assuming every failure skips to RS

#### Scenario: Input advancement policy differs by caller
- **WHEN** malformed sequence input is read by top-level advancement, `input`, `inputs`, or a query with `try/catch`
- **THEN** the suite compares warnings, query-visible errors, prior results, continuation, and exit status separately for each caller

#### Scenario: Stream projection shares the query cursor
- **WHEN** `--stream` or `--stream-errors` is combined with `-n`, slurp, `input`, `inputs`, or `try/catch`
- **THEN** jq and tq observations compare ordinary query evaluation over the projected records, shared cursor advancement, prior results, and failure handling

#### Scenario: CSV and TSV peer cases
- **WHEN** a delimited fixture exercises quoted strings, typed unquoted scalars, empty fields, and quoted newlines
- **THEN** the report preserves tq and yq observations and records any deliberate profile difference explicitly

#### Scenario: Deferred formats remain visible
- **WHEN** the format coverage report is generated
- **THEN** XML, TOML, Properties, and INI are marked as follow-on work while HCL and Lua remain deferred
