# TOON Stream I/O Specification

## Purpose

Define incremental TOON decoding, canonical output, event contracts, framing,
bounded preparation, and conformance requirements for structured stream I/O.

## Requirements

### Requirement: Incremental byte-oriented TOON decoding
The TOON decoder SHALL consume `Read`/`BufRead` input incrementally without first loading the full document or collecting all Unicode scalar values. It MUST validate UTF-8 and maintain byte offset, line, and column positions.

#### Scenario: Decode large input
- **WHEN** a valid large TOON document is read from a non-seekable stream
- **THEN** decoding begins before EOF and memory does not scale with total document bytes unless a downstream consumer requests materialization

#### Scenario: Invalid UTF-8
- **WHEN** an invalid UTF-8 sequence is encountered
- **THEN** decoding stops with a source-positioned input error and does not replace the bytes silently

### Requirement: Unified structural event contract
The decoder SHALL emit a stable ordered semantic event stream representing document boundaries, object boundaries, keys, array boundaries and typed scalar values. Physical TOON 4.1 table schemas, declared counts and delimiters SHALL be interpreted and validated during decoding; nested field groups SHALL reconstruct nested object events and keyed tables SHALL reconstruct object entries rather than arrays. Both document construction and explicit stream execution MUST consume this same event contract.

#### Scenario: Tabular array
- **WHEN** a tabular TOON array with recursive fields is decoded
- **THEN** array and nested row-object events reproduce its header-ordered semantic values and decoding enforces declared length, delimiter and recursive leaf width

#### Scenario: Build document
- **WHEN** the DOM builder consumes a valid event stream
- **THEN** it produces the same JSON-model value, with specified header-order normalization, as direct conformance decoding

#### Scenario: Keyed table events
- **WHEN** a keyed tabular object is decoded through events
- **THEN** consumers receive object entries in encounter order and nested values in recursive header order, not a synthetic array

### Requirement: Strict validation during consumption
Strict decoding SHALL validate root shape, indentation, whitespace rules, string escapes, delimiter scope, array/entry count, recursive tabular leaf width, field-list uniqueness at each level, duplicate sibling keys, and configured nesting/token limits while events are consumed, according to TOON 4.1. Validation MUST apply to event consumers as well as materialized documents. Declared counts MUST NOT cause proportional preallocation.

#### Scenario: Declared count mismatch
- **WHEN** an array ends after a different number of elements than its declared length
- **THEN** strict decoding returns a count-mismatch error at the array's source location

#### Scenario: Excessive nesting
- **WHEN** input exceeds the configured maximum nesting depth, including nested header groups
- **THEN** decoding fails before growing parser state beyond the configured bound

#### Scenario: Recursive header defect
- **WHEN** a table header repeats a sibling field name, contains an empty nested field group, or has malformed keyed syntax
- **THEN** strict decoding rejects the header independently of following rows and its declared count

#### Scenario: Discarding event consumer
- **WHEN** an event consumer discards values from input containing a duplicate sibling key or wrong-width keyed row
- **THEN** strict decoding still reports the source-positioned defect

### Requirement: Ordered loss-aware document model
The DOM builder SHALL preserve array order, object insertion order, string contents, boolean/null identity, and supported numeric representations. TOON 4.1 tabular rows and keyed entry values SHALL reconstruct object keys in header order recursively; equality SHALL allow this specified reordering only within those values. Dotted keys SHALL remain literal. Strict duplicate sibling keys SHALL error; non-strict duplicates SHALL use deterministic last-write-wins in document order. Out-of-envelope number behavior MUST be documented and compatibility-tested.

#### Scenario: Ordered object
- **WHEN** an ordinary object contains keys in non-lexicographic order
- **THEN** document construction and canonical re-encoding preserve encounter order

#### Scenario: Header order normalization
- **WHEN** uniform table values have the same recursive key sets in different encounter orders
- **THEN** decoded table values use the first value's key order at each header level while outer entries, arrays, and results remain ordered

#### Scenario: Literal dotted key
- **WHEN** quoted or unquoted `a.b` appears as a key
- **THEN** it remains a single key and never creates nested objects

#### Scenario: Non-strict duplicate
- **WHEN** non-strict decoding encounters a repeated ordinary or keyed-entry sibling key
- **THEN** its later value replaces the earlier value deterministically without a duplicate diagnostic

### Requirement: Canonical structured output
The TOON writer SHALL emit valid canonical TOON 4.1 to a sink with LF line endings, no trailing spaces, no document-internal trailing newline, correct quoting/escapes, canonical number formatting, declared array/entry lengths, deterministic object order, and configured delimiter and indent. Form selection SHALL follow whole-value shape and position, not a formatting preference. Uniform non-empty object arrays SHALL use recursively nested field groups where eligible. Root/object-field objects with at least two uniform non-empty object values SHALL use keyed tabular form. Root/object-field empty arrays SHALL use `[]`; empty list-item arrays SHALL use a zero-length header. Anonymous nested array list items SHALL NOT use fields-bearing keyless headers. The writer SHALL write completed output where shape and framing permit and MUST NOT require a result-sized string intermediate or perform key folding.

#### Scenario: Canonical round trip
- **WHEN** a supported runtime value is encoded and decoded in strict mode
- **THEN** the decoded JSON-model value equals the original under the specified recursive header-order equality

#### Scenario: Identity formatting
- **WHEN** tq applies identity to a noncanonical but valid TOON input
- **THEN** structured output is canonical and is not required to preserve original whitespace or delimiter choices

#### Scenario: Wide object sink output
- **WHEN** the writer receives a wide object
- **THEN** output waits for any required whole-shape decision and writes the selected form without collecting the complete output text

#### Scenario: Nested uniform columns
- **WHEN** `orders` contains objects with uniform nested `customer` fields `name` and `country`
- **THEN** its header declares `customer{name,country}` and each row supplies primitive leaves in depth-first header order

#### Scenario: Keyed uniform object
- **WHEN** `users` maps `alice` and `bob` to uniform objects with fields `age` and `city`
- **THEN** output uses `users[2:]{age,city}:` with entry-key-prefixed rows in encounter order

#### Scenario: Empty arrays by position
- **WHEN** an empty array appears at the root, as object field `items`, and as an inner list-item array
- **THEN** the respective forms are `[]`, `items: []`, and `- [0]:` with any configured inner-header delimiter suffix

#### Scenario: Table first field in list object
- **WHEN** a list-item object's first field is a tabular array or keyed tabular object
- **THEN** its header stays on the hyphen line, rows are at hyphen depth plus two, and later sibling fields are at hyphen depth plus one

### Requirement: Bounded array preparation and spooling
When an output array's or object's final count or recursive table schema is unknown, the writer SHALL retain only one replayable representation of pending values. All active replay preparations, schema state and transient composites SHALL use one configurable aggregate in-memory threshold and then spool securely where supported. State that cannot spill MUST fail with a resource diagnostic rather than allocate past that threshold. The writer MUST choose the canonical TOON 4.1 form after sufficient whole-shape information is known, write its final header before its body, replay values once in order, and expose whether spooling occurred.

#### Scenario: Unknown large array
- **WHEN** a generated array exceeds the aggregate in-memory preparation threshold
- **THEN** the writer spools excess content, emits the correct final array header, replays the body once, and cleans up temporary storage

#### Scenario: Spooling forbidden
- **WHEN** spooling is required but disabled
- **THEN** the writer fails with a resource-class diagnostic before claiming successful output

#### Scenario: Tabular eligibility changes
- **WHEN** later array elements invalidate a recursive schema inferred from earlier elements
- **THEN** prepared output uses the canonical non-tabular representation without losing or reordering prior elements

#### Scenario: Nested preparation budget
- **WHEN** nested objects and arrays are prepared concurrently
- **THEN** replay, schema, and transient retained bytes do not exceed the configured aggregate threshold apart from bounded bookkeeping and current tokens

#### Scenario: Wide keyed candidate
- **WHEN** a root or object-field keyed-table candidate exceeds the memory threshold before its shape is known
- **THEN** it uses secure bounded spooling or returns a resource error without root-sized memory materialization or a speculative header

### Requirement: TOON Text Sequence framing
Explicit TOON sequence output requested with `-o toon-seq` (or
`--output-format toon-seq`), or with `--seq` when TOON output is selected (the
default or `-o toon`/`--output-format toon`), SHALL encode every result
as ASCII RS (`0x1e`), followed by one canonical TOON document, followed by LF.
Sequence framing SHALL be distinct from the bytes of a standalone TOON document.

#### Scenario: Emit multiple results
- **WHEN** a filter emits two multiline objects with `--seq` using default TOON output or `-o toon`/`--output-format toon`
- **THEN** the output contains two independently parseable RS-framed records in emission order
- **AND** the same result-cardinality and RS-framing contract applies to explicit `-o toon-seq`/`--output-format toon-seq` output without changing input selection

#### Scenario: Emit zero results
- **WHEN** a filter emits no values with `--seq` using default TOON output or `-o toon`/`--output-format toon`
- **THEN** the structured sequence output is empty
- **AND** the same result-cardinality and RS-framing contract applies to explicit `-o toon-seq`/`--output-format toon-seq` output without changing input selection

#### Scenario: Emit one result
- **WHEN** a filter emits one structured value with `--seq` using default TOON output or `-o toon`/`--output-format toon`
- **THEN** the output contains exactly one RS-framed record
- **AND** the same result-cardinality and RS-framing contract applies to explicit `-o toon-seq`/`--output-format toon-seq` output without changing input selection

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

### Requirement: Explicit unframed single output
The writer SHALL support an unframed mode that produces exactly one standalone
canonical TOON document. It MUST report an error if evaluation yields zero or
multiple values. Output in this mode MUST remain unpublished until successful
exactly-one-result cardinality is known, using bounded memory and secure spooling
when the result exceeds the preparation threshold.

#### Scenario: Exactly one result
- **WHEN** unframed mode receives one valid value
- **THEN** stdout contains only that canonical TOON document

#### Scenario: Multiple results
- **WHEN** unframed mode receives a second value
- **THEN** execution fails with a cardinality diagnostic and writes no result bytes

#### Scenario: Late preparation failure
- **WHEN** encoding or input validation fails before an unframed result is committed
- **THEN** the writer publishes no partial document and cleans up temporary storage

### Requirement: Sequence input
The decoder SHALL support explicit TOON Text Sequence input and MUST treat each framed record as a separate input document. Ordinary mode SHALL treat each file as one TOON document.

#### Scenario: Read framed stream
- **WHEN** sequence input contains three valid records
- **THEN** the evaluator receives three input documents in order

#### Scenario: Unframed multiple roots
- **WHEN** ordinary strict input contains multiple root documents
- **THEN** decoding fails rather than guessing record boundaries

### Requirement: Event-stream memory bound
With a consumer that does not materialize subtrees, event decoding SHALL retain memory proportional to configured nesting depth, the current source line/token, active delimiter/schema stacks, and current consumer result—not total input size.

#### Scenario: Stream a large feature collection
- **WHEN** a consumer discards completed path/value events from a large document
- **THEN** completed feature values are released and peak parser memory does not grow with the feature count

### Requirement: I/O conformance and differential tests
The I/O implementation SHALL pass the official pinned TOON 4.1 conformance fixtures, round-trip property tests, malformed-input tests, and event-versus-DOM equivalence tests before it is used by tq compatibility benchmarks. Reference differentials SHALL use a declared conforming reference version and document numeric/depth-domain exclusions; reference behavior MUST NOT override official fixture expectations. Fixture provenance and complete applicable coverage SHALL be auditable without name-based silent skips.

#### Scenario: Event and DOM agreement
- **WHEN** a valid fixture is decoded through events and built into a document
- **THEN** its value equals the established conforming decoder result within the documented shared domain

#### Scenario: Encoder fixture
- **WHEN** an official 4.1 encoder fixture is exercised through document and transcode output
- **THEN** each applicable path matches its canonical bytes and declared framing separately

### Requirement: TOON 4.1 lexical and non-strict interpretation
Decoding SHALL apply TOON 4.1 BOM, CRLF, full-line comment, quoted-token, Unicode escape, U+0020-only trimming, scope, empty-array, and strict/non-strict rules. Comments SHALL be excluded before structural interpretation without changing source locations or surrounding scopes. Encoders SHALL quote hash-leading strings and non-ASCII-pattern keys and escape control characters. Surrogate escapes, malformed quoted tokens and misplaced scalar lines SHALL error in all modes. Non-strict counts SHALL NOT truncate scopes; missing row leaves SHALL be absent and surplus cells SHALL contribute nothing. Legacy empty-array headers and empty list-item `- []` SHALL remain valid input.

#### Scenario: Comment between rows
- **WHEN** a full-line comment with arbitrary leading spaces occurs between table rows
- **THEN** it does not count as a row or blank line, terminate the table, or trigger indentation rejection

#### Scenario: Control and Unicode preservation
- **WHEN** quoted input contains a valid BMP `\uXXXX` escape or literal supplementary Unicode
- **THEN** the decoded string preserves its scalar values, while a surrogate escape is rejected

#### Scenario: Empty-array compatibility
- **WHEN** strict input uses root `[]`, object-field `items: []`, legacy zero-count headers, or list item `- []`
- **THEN** each produces an empty array; a `[]` token inside a row remains a string

#### Scenario: Non-strict overfull scope
- **WHEN** non-strict input declares one row but contains two rows in its scope
- **THEN** both rows are decoded instead of truncating at the declaration

#### Scenario: Misplaced scalar
- **WHEN** a bare scalar line occurs inside an object or array scope without a valid row/item role
- **THEN** decoding fails in both strict and non-strict modes

