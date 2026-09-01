# JSON5 input Specification

## Purpose

Define JSON5 document input for tq, including the multiline-string dialect used by kibana-sync saved objects and conversion into tq's ordered JSON-shaped value model.

## Requirements

### Requirement: JSON5 document decoding
tq SHALL decode one JSON5 value per source when JSON5 input is selected. The accepted grammar MUST include standard JSON5 comments, trailing commas, identifier keys, single-quoted strings, JSON5 escapes, line continuations, hexadecimal integers, leading or trailing decimal points, and explicit numeric signs. The decoder MUST consume the complete source except for JSON5 whitespace and comments.

#### Scenario: Standard JSON5 syntax
- **WHEN** a selected JSON5 source contains comments, unquoted keys, single-quoted strings, hexadecimal integers, and trailing commas
- **THEN** tq converts it to the equivalent ordered runtime value and evaluates the requested filter

#### Scenario: Trailing content
- **WHEN** a selected JSON5 source contains another value or non-comment token after its root value
- **THEN** tq rejects the source with a JSON5 input diagnostic

### Requirement: Kibana triple-quoted strings
JSON5 input SHALL accept a string delimited by three double quotes for compatibility with kibana-sync saved-object files. The decoder MUST preserve all characters between the delimiters, including physical newlines, carriage returns, tabs, double quotes that do not complete the closing delimiter, and backslashes. It MUST NOT interpret JSON or JSON5 escape sequences inside a triple-quoted string.

#### Scenario: Saved-object markdown
- **WHEN** a JSON5 object contains a kibana-sync `"""..."""` markdown value spanning physical lines
- **THEN** tq produces one string containing the exact text between the delimiters, including its line breaks

#### Scenario: Literal backslash
- **WHEN** a triple-quoted string contains the two characters `\` and `n`
- **THEN** tq preserves those two characters rather than converting them to a newline

#### Scenario: Unterminated multiline string
- **WHEN** a triple-quoted string reaches end of input without a closing three-quote delimiter
- **THEN** tq rejects the source with a JSON5 diagnostic that identifies an unterminated multiline string

### Requirement: Ordered JSON-shaped conversion
The JSON5 decoder SHALL preserve object insertion order and convert nulls, booleans, strings, arrays, objects, integers, and finite decimal numbers into tq's shared value model. Decimal and exponent forms MUST enter the binary64 side of tq's hybrid number model. Integer and hexadecimal forms MUST remain exact when tq's numeric envelope admits them. `NaN`, positive or negative `Infinity`, numeric overflow, and any value outside the runtime model MUST fail instead of being coerced.

#### Scenario: Object order
- **WHEN** a JSON5 object declares keys in a specific order
- **THEN** tq exposes those keys in the same insertion order unless the query or output options reorder them

#### Scenario: Exact hexadecimal integer
- **WHEN** a JSON5 hexadecimal integer fits tq's exact integer envelope
- **THEN** tq stores its exact mathematical value

#### Scenario: Non-finite number
- **WHEN** a JSON5 source contains `NaN`, `Infinity`, or `-Infinity`
- **THEN** tq rejects the source with a JSON5 numeric diagnostic

### Requirement: Bounded document parsing
JSON5 SHALL be a document-at-a-time input format subject to the configured source-byte, token-byte, and nesting-depth limits. Limit failures and malformed input MUST be classified as JSON5 input errors, MUST include bounded source context, and MUST NOT panic or dump unbounded input text.

#### Scenario: Source exceeds byte limit
- **WHEN** a JSON5 source exceeds the configured maximum input bytes
- **THEN** tq stops reading it and reports the input-byte resource limit

#### Scenario: Oversized multiline string
- **WHEN** a triple-quoted string exceeds the configured maximum token bytes
- **THEN** tq stops accumulating the token and reports the token-byte resource limit

#### Scenario: Excessive nesting
- **WHEN** JSON5 nesting exceeds the configured maximum depth
- **THEN** tq reports the depth limit without overflowing the native stack
