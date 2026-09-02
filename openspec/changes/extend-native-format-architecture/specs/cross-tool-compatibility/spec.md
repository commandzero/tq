## ADDED Requirements

### Requirement: Native format compatibility coverage
The compatibility manifest SHALL include jq-target RFC 7464 cases and yq-peer CSV and TSV cases. Coverage MUST compare ordered document values, output bytes where framing or quoting matters, warnings, stable error classes, and exit status without coercing string and numeric values during normalization.

#### Scenario: jq JSON sequence recovery
- **WHEN** a JSON sequence fixture contains valid, malformed, and valid frames
- **THEN** the jq and tq observations compare emitted values, warning classification, continuation, and successful exit status

#### Scenario: JSON sequence output bytes
- **WHEN** jq and tq emit multiple results under sequence output
- **THEN** the compatibility report compares RS prefixes, JSON payloads, LF terminators, and result order

#### Scenario: CSV and TSV peer cases
- **WHEN** a delimited fixture exercises quoted strings, typed unquoted scalars, empty fields, and quoted newlines
- **THEN** the report preserves tq and yq observations and records any deliberate profile difference explicitly

#### Scenario: Deferred formats remain visible
- **WHEN** the format coverage report is generated
- **THEN** XML, TOML, Properties, and INI are marked as follow-on work while HCL and Lua remain deferred
