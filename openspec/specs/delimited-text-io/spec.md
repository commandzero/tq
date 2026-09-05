# Delimited Text I/O Specification

## Purpose

Define CSV and TSV as header-shaped document sequences with typed scalar fields, deterministic quoting, row validation, and bounded incremental I/O.

## Requirements

### Requirement: Delimited sequence header and row documents
CSV and TSV input SHALL treat the first logical row as a sequence header that establishes an ordered set of unique object keys. Each later logical row SHALL be one document whose root value is an object shaped by that header. Quoted newlines MUST remain inside one logical row rather than creating another frame.

#### Scenario: Header shapes documents
- **WHEN** a header contains `name,age` and two data rows follow
- **THEN** tq emits two ordered object-root documents with `name` and `age` keys

#### Scenario: Duplicate header
- **WHEN** a sequence header repeats a key
- **THEN** tq reports a profile rejection before emitting a row document

#### Scenario: Quoted newline
- **WHEN** a quoted field contains a newline
- **THEN** frame detection keeps the complete quoted field in one row document

### Requirement: Delimited row width
A row with fewer fields than its row shape SHALL decode missing trailing fields as null. A row with more fields than its row shape MUST be rejected because the excess fields have no declared keys.

#### Scenario: Missing trailing field
- **WHEN** a two-key row shape receives a row with one field
- **THEN** the second key is present with a null value

#### Scenario: Excess field
- **WHEN** a two-key row shape receives a row with three fields
- **THEN** tq reports a profile rejection for that row

### Requirement: Delimited scalar input profile
CSV and TSV SHALL decode quoted fields as strings. They SHALL decode unquoted JSON booleans and numbers by type, an unquoted empty field as null, and every other unquoted field as a string. A quoted empty field MUST decode as an empty string.

#### Scenario: Typed scalar row
- **WHEN** a row contains an unquoted number, an unquoted boolean, an unquoted empty field, and a quoted numeric-looking field
- **THEN** tq decodes them as number, boolean, null, and string respectively

#### Scenario: Quoted empty string
- **WHEN** a field is represented by an explicitly quoted empty value
- **THEN** tq decodes it as an empty string rather than null

### Requirement: Delimited output row shape
CSV and TSV output SHALL require object-root results. The first result SHALL establish and emit the sequence header only after complete profile and sequence-context validation. One row shape SHALL span all input sources and intervening proxy bytes within command output. Later results MAY omit declared keys or assign them null but MUST NOT add keys outside the row shape. Zero results SHALL produce no native output bytes.

#### Scenario: First result establishes header
- **WHEN** the first result object has keys in non-lexicographic order
- **THEN** output emits those keys as the header in encounter order and writes every row in that order

#### Scenario: Later missing key
- **WHEN** a later result omits one key from the established row shape
- **THEN** output writes an unquoted empty field for that key

#### Scenario: Later extra key
- **WHEN** a later result contains a key outside the established row shape
- **THEN** output rejects that result before committing its row while preserving earlier complete rows

#### Scenario: Empty result sequence
- **WHEN** the query emits no results under CSV or TSV output
- **THEN** stdout is empty and no header is emitted

#### Scenario: Row shape spans input sources
- **WHEN** Results from several input sources are written as CSV or TSV
- **THEN** the first Result establishes one header and every later Result conforms to its row shape

#### Scenario: First row rejection emits no header
- **WHEN** the first Result fails output-profile validation
- **THEN** neither its sequence header nor its row commits any bytes

#### Scenario: Proxy interruption retains row shape
- **WHEN** proxy bytes occur between accepted row Results
- **THEN** the bytes pass through unchanged and later rows use the original row shape without another header

### Requirement: Delimited scalar output profile
CSV and TSV output SHALL accept string, finite number, boolean, and null fields and SHALL reject arrays or objects in field positions. Strings whose unquoted text would decode as another scalar type MUST be quoted; numbers and booleans SHALL remain unquoted, empty strings SHALL be quoted, and null SHALL be an unquoted empty field. Normal delimiter, quote, newline, and control-character escaping rules MUST still apply.

#### Scenario: Numeric string remains a string
- **WHEN** a row contains string `"42"` and number `42`
- **THEN** output quotes the string field, leaves the number unquoted, and input decodes both with their original types

#### Scenario: Empty string and null differ
- **WHEN** a row contains an empty string and null
- **THEN** output writes a quoted empty field and an unquoted empty field respectively

#### Scenario: Container field is rejected
- **WHEN** a row field contains an array or object
- **THEN** tq reports a profile rejection before committing that row

### Requirement: Delimited dialects
CSV SHALL use comma delimiters and TSV SHALL use tab delimiters. Both dialects SHALL use double-quote field quoting with doubled internal quotes and SHALL emit LF row endings. Header fields MUST always decode as literal strings without scalar inference.

#### Scenario: CSV escaping
- **WHEN** a CSV string field contains a comma, quote, or newline
- **THEN** output quotes the field and doubles each internal quote

#### Scenario: TSV escaping
- **WHEN** a TSV string field contains a tab, quote, or newline
- **THEN** output quotes the field and doubles each internal quote

### Requirement: Delimited resource limits
CSV and TSV processing SHALL enforce configured source-byte, logical-row-byte, field-byte, field-count, result, output-byte, and query execution limits while reading and writing incrementally. Completed row documents MUST be releasable so retained decoder state does not grow with the number of rows.

#### Scenario: Oversized quoted row
- **WHEN** a quoted multiline row exceeds the configured logical-row limit
- **THEN** tq reports a resource-class failure without buffering the complete source

#### Scenario: Large row sequence
- **WHEN** a large CSV or TSV source contains many individually bounded rows
- **THEN** tq processes rows in order without retaining all prior row documents
