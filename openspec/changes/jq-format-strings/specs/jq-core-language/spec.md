## ADDED Requirements

### Requirement: jq format strings and escaping
The language SHALL support the jq 1.7 format filters `@text`, `@json`, `@html`, `@uri`, `@csv`, `@tsv`, `@sh`, `@base64`, and `@base64d`. Each filter MUST emit one string for a valid input, preserve jq's compact value representation where a format converts non-string values to text, and enforce the configured output-byte limit during conversion.

#### Scenario: Text, JSON, HTML, and URI formatting
- **WHEN** `@text`, `@json`, `@html`, or `@uri` receives any supported JSON value
- **THEN** `@text` applies jq `tostring` behavior, `@json` emits compact JSON text, `@html` escapes `<`, `>`, `&`, `'`, and `"` after jq text conversion, and `@uri` percent-encodes UTF-8 bytes outside the RFC 3986 unreserved set after jq text conversion

#### Scenario: CSV and TSV rows
- **WHEN** `@csv` or `@tsv` receives an array containing strings, numbers, booleans, and null
- **THEN** it emits one jq-compatible row without a trailing record separator, using jq's quoting and control-character escaping rules for the selected format

#### Scenario: Invalid tabular value
- **WHEN** `@csv` or `@tsv` receives a non-array or an array containing an array or object
- **THEN** evaluation fails with a runtime type diagnostic

#### Scenario: POSIX shell escaping
- **WHEN** `@sh` receives a scalar or an array of scalar values
- **THEN** strings use jq-compatible POSIX single-quote escaping, other scalars use jq text conversion, and array fields are joined by one space

#### Scenario: Invalid shell value
- **WHEN** `@sh` receives an object or an array containing an array or object
- **THEN** evaluation fails with a runtime type diagnostic

#### Scenario: Base64 round trip
- **WHEN** a UTF-8 string is passed through `@base64 | @base64d`
- **THEN** the result equals the original string using RFC 4648 base64

#### Scenario: Invalid base64 input
- **WHEN** `@base64d` receives malformed base64 or bytes that do not decode to valid UTF-8
- **THEN** evaluation fails with a stable runtime diagnostic instead of producing an invalid runtime string

#### Scenario: Formatted interpolation
- **WHEN** `@uri "https://example.test?q=\(.query)"` evaluates an interpolation expression
- **THEN** literal template text is copied unchanged and each interpolation result is URI-formatted before jq interpolation joins it into the output string

#### Scenario: Formatted interpolation multiplicity
- **WHEN** a formatted template contains interpolation expressions that emit zero, one, or multiple results
- **THEN** it preserves jq interpolation's result multiplicity and ordering while formatting each emitted value

#### Scenario: Unknown format
- **WHEN** a query contains an unrecognized `@name` format token
- **THEN** compilation fails with a stable, source-spanned format diagnostic

#### Scenario: Format output limit
- **WHEN** a format operation would emit more bytes than the configured output-byte limit
- **THEN** evaluation stops with the `output-bytes` resource error before retaining an oversized result

## MODIFIED Requirements

### Requirement: Deferred syntax is explicit
Labels and `break`, recursive built-ins such as `recurse` and `walk`, and non-finite result built-ins SHALL remain deferred unless separately promoted by an accepted spec. Unsupported syntax or built-ins MUST fail at compile time with a stable capability identifier. User-defined functions, modules, `reduce`, `foreach`, recursive descent, string interpolation, format strings, regex, date, environment, and admitted platform built-ins are supported by their accepted specifications and MUST NOT be described as deferred.

#### Scenario: Deferred labels and break
- **WHEN** a query contains `label $out` or `break $out`
- **THEN** compilation fails with the stable labels or break capability identifier and does not partially execute the query

#### Scenario: Deferred recursive built-in
- **WHEN** a query calls `recurse` or `walk`
- **THEN** compilation fails with the stable recursive-builtins capability identifier
