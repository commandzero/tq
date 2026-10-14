## MODIFIED Requirements

### Requirement: Proven transcode eligibility
The system SHALL select the transcode plan only when analysis proves that the resolved query is semantic identity, the selected decoder exposes ordered structural events with the requested duplicate-key semantics, and the requested output is canonical TOON 4.4 with supported options. The proof MUST be complete before semantic input consumption. If any condition fails, the system SHALL select another sound plan without speculative transcode output. Removed folding options SHALL NOT remain proof conditions.

#### Scenario: JSON identity conversion
- **WHEN** a JSON input uses the identity query and default TOON writer options
- **THEN** the system selects the transcode plan before decoding the root value

#### Scenario: Implicit identity conversion
- **WHEN** the CLI supplies its implicit identity query for eligible JSON-to-TOON conversion
- **THEN** the system applies the same transcode proof as it does for an explicit `.` query

#### Scenario: Unsupported writer option
- **WHEN** a supported CLI invocation requests a writer option not supported by structural transcode
- **THEN** the system selects the document plan before consuming semantic input and explains the rejected transcode condition

#### Scenario: Non-identity query
- **WHEN** the query performs selection, projection, mutation, or any other non-identity operation
- **THEN** the system does not select the transcode plan

#### Scenario: Non-strict duplicate resolution
- **WHEN** a decoder's direct events cannot preserve required non-strict last-write-wins semantics
- **THEN** analysis chooses a path that can preserve those values before consuming input and does not claim direct event equivalence

### Requirement: Document-equivalent output
For every accepted input without duplicate object names, the transcode plan SHALL emit the same canonical TOON 4.4 bytes and result framing as the document plan under the same input and output options. It MUST preserve outer object encounter order, array order, supported exact numbers, string contents, booleans, nulls, and empty containers. Table-value object key order SHALL follow the recursive header order required by 4.4, identically in both paths.

#### Scenario: Differential identity conversion
- **WHEN** a correctness fixture is encoded through both transcode and forced-document plans
- **THEN** their output bytes and exit classifications are identical

#### Scenario: Exact number
- **WHEN** JSON contains an accepted number outside the lossless IEEE-754 integer range
- **THEN** transcode output preserves the same exact numeric value as document-mode output

#### Scenario: Ordered object
- **WHEN** an ordinary object uses non-lexicographic key order
- **THEN** transcode output preserves that encounter order

#### Scenario: Recursive keyed table
- **WHEN** a root object has two or more uniform object values with nested-uniform columns
- **THEN** both paths emit identical keyed headers, recursive field groups and entry rows

#### Scenario: Anonymous nested array
- **WHEN** an array of uniform objects is itself an anonymous array list item
- **THEN** both paths emit nested list form rather than a keyless fields-bearing header

### Requirement: Single-source array preparation
When TOON 4.4 array or keyed-object syntax requires a final count or recursive layout decision, the transcode plan SHALL retain one replayable representation of each pending element or entry. All active replay preparations, schema state and transient composites MUST share the configured in-memory budget. Once exhausted, eligible root/object-field preparations SHALL move to secure bounded spooling; unspillable state SHALL fail with a resource diagnostic rather than allocate past the aggregate budget. After the candidate closes, the writer MUST select the canonical position-appropriate form and replay stored values once in order.

#### Scenario: Unknown root array
- **WHEN** an unknown-length root array exceeds the shared in-memory preparation budget
- **THEN** preparation moves to disk, emits the final count before the body, replays every element once, and cleans up the spool

#### Scenario: Nested active arrays
- **WHEN** several object and array preparations are open at once
- **THEN** replay storage, schemas and transient composite state are charged to one shared budget, and exhaustion either spools an eligible preparation or returns a resource error

#### Scenario: Tabular candidate becomes ineligible
- **WHEN** a later element invalidates the recursive schema inferred from earlier elements
- **THEN** the writer selects the canonical non-tabular layout without losing, duplicating, or reordering earlier elements

#### Scenario: Keyed candidate becomes ineligible
- **WHEN** a late object entry contains an array, an empty object, or an incompatible nested field
- **THEN** the entire object uses canonical expanded form and no speculative keyed header escapes
