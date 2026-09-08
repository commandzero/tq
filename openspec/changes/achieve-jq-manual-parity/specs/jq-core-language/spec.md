## MODIFIED Requirements

### Requirement: jq decimal-literal hybrid numbers
The runtime SHALL preserve decimal literals and derive IEEE-754 binary64 values according to the pinned decimal-enabled jq reference. Literal identity, precision, lexical representation observable through JSON output and `tojson`, comparison, signed zero, rounding, and arithmetic SHALL match the reference. Arithmetic SHALL discard literal provenance when jq does. Large exponents MUST NOT require expansion into an unbounded decimal string. Configured resource limits remain enforceable, but an obsolete numeric envelope MUST NOT reject an admitted manual example. Runtime NaN and infinities SHALL be supported where jq produces them, with jq-compatible JSON projection and numeric predicates. Non-finite spellings in native TOON and YAML input remain invalid; strict JSON and `fromjson` acceptance SHALL follow the pinned reference. TOON output MUST remain valid TOON and MUST NOT invent non-finite syntax.

#### Scenario: Large exact integer identity
- **WHEN** an admitted exact integer passes through identity without arithmetic
- **THEN** its numeric value is preserved through TOON and its JSON representation matches jq

#### Scenario: Arithmetic leaves the literal domain
- **WHEN** arithmetic consumes a decimal literal not exactly representable as binary64
- **THEN** the result matches jq's binary64-derived result without a blanket comparison tolerance

#### Scenario: Large exponent identity
- **WHEN** the manual's large-exponent identity example runs with normal limits
- **THEN** it matches jq rather than reporting the former numeric-envelope difference

#### Scenario: Numeric envelope exceeded
- **WHEN** numeric input exceeds a configured digit or work limit
- **THEN** processing fails with a bounded resource diagnostic and the parity runner does not count that failure as a match to successful jq execution

#### Scenario: Non-finite runtime result
- **WHEN** a manual program produces `nan` or `infinite`
- **THEN** predicates, arithmetic, and JSON projection match jq while TOON serialization uses the documented JSON-compatible projection without invalid TOON syntax

#### Scenario: Non-finite input
- **WHEN** input contains a non-finite spelling
- **THEN** native TOON and YAML reject it, while strict JSON acceptance and projection match the pinned jq parser rather than the former blanket runtime prohibition

### Requirement: Variable binding
The language SHALL support scalar, array, and object destructuring bindings, nested patterns, and destructuring alternatives with jq lexical scope and generator behavior. CLI variables SHALL enter the same resolved environment. Pattern null filling, errors, shadowing, and backtracking SHALL match the pinned reference. Variable object shorthand and quoted field identifiers SHALL resolve through the same language rules.

#### Scenario: Bind multiple values
- **WHEN** `.items[] as $item | $item.name` runs
- **THEN** the body executes once per bound item in order

#### Scenario: Unknown variable
- **WHEN** a query references an unbound variable
- **THEN** resolution fails before execution with a source-spanned compile error

#### Scenario: Destructuring alternatives
- **WHEN** the manual's `?//` examples select different input shapes
- **THEN** variable availability, null bindings, branch errors, and output order match jq

#### Scenario: Nested lexical capture
- **WHEN** a function's filter argument references an outer binding that is shadowed at the call site
- **THEN** values resolve in the jq lexical environment rather than the caller's shadowing environment

### Requirement: Deferred syntax is explicit
All syntax and built-in arities documented by the pinned jq manual SHALL be supported, including non-finite numeric functions. Unknown syntax and unsupported arities SHALL fail at compile time with source context. A documented capability MUST NOT remain behind a deferred-capability error.

#### Scenario: Manual built-in resolves
- **WHEN** a documented built-in is called at an admitted arity
- **THEN** compilation succeeds, including functions whose platform implementation may subsequently report jq's documented runtime availability error

#### Scenario: Invalid reference arity
- **WHEN** the source manual's invalid two-argument `frexp` or `modf` example is compiled
- **THEN** it receives the pinned reference's compile-error contract, while the corrected zero-argument witness executes independently

#### Scenario: Deferred non-finite result built-in
- **WHEN** a query calls a documented non-finite result built-in
- **THEN** it compiles and executes under the numeric contract instead of returning the former deferred-capability error

#### Scenario: Deferred labels and break
- **WHEN** a query contains `label $out` or `break $out`
- **THEN** it compiles under the lexical-label contract without the former deferred-capability error

#### Scenario: Deferred recursive built-in
- **WHEN** a query calls a supported arity of `recurse` or `walk`
- **THEN** it compiles under the recursive traversal contract without the former deferred-capability error

## ADDED Requirements

### Requirement: Complete manual grammar and generator semantics
Every manual expression SHALL preserve jq precedence, associativity, lexical scope, result order, and cardinality. This includes comma indexing, quoted identifiers, binding pipelines, optional access, recursive traversal, generator arguments, and optional foreach extraction. Empty generators MUST remain distinct from null and errors. Pull-driven consumers MUST stop once their result is determined.

#### Scenario: Boolean pipe precedence
- **WHEN** `[true, false | not]` runs
- **THEN** it emits the same array as jq

#### Scenario: Index generator
- **WHEN** `.[4,2]` runs on the manual input
- **THEN** it emits the two selected elements in jq order

#### Scenario: Bounded consumer
- **WHEN** `first`, `last`, `nth`, `skip`, `isempty`, or a short-circuit predicate consumes a generator
- **THEN** values, empty-input behavior, evaluated errors, and termination match jq without evaluating unnecessary branches

### Requirement: Complete manual built-in families
Every documented built-in and overload SHALL match jq values, ordering, multiplicity, type errors, and arity. The implementation SHALL cover collection membership and containment, `indices`/`index`/`rindex`, `pick`/`del`/`delpaths`, entry conversions, all `any`/`all` and `add` forms, `IN`/`INDEX`/`JOIN`, combinations, transpose, binary search, trim and ASCII utilities, `utf8bytelength`, `toboolean`, string division, `@urid`, and recursive/generator utilities. Registration or successful compilation alone SHALL NOT establish coverage.

#### Scenario: Overload inventory
- **WHEN** a manual family has multiple arities or argument forms
- **THEN** executable witnesses cover each form and invalid-type behavior, including empty and multi-result arguments where meaningful

#### Scenario: String and collection variants
- **WHEN** membership, containment, indexing, or joining is applied to each documented input type
- **THEN** results match the reference without conflating scalar equality, substring search, and array subsequences

### Requirement: Assignment path provenance and cardinality
Path-producing filters and assignment SHALL preserve the original root and selected path identity through traversal and predicates. Plain assignment SHALL evaluate its right side against the original input and produce jq's result branches. Update assignment SHALL use the selected old value, keep only the first right-side result where jq does, and delete paths for empty results. Branches MUST NOT mutate one another.

#### Scenario: Plain versus update generator
- **WHEN** the manual's `= range(...)` and `|= range(...)` examples run
- **THEN** their different result cardinalities and resulting structures match jq

#### Scenario: Nested selected assignment
- **WHEN** a predicate selects nested posts or comments for update
- **THEN** only those paths change and each emitted root preserves unaffected values and encounter order

### Requirement: Full manual math and numeric behavior
All functions listed in the pinned Math section SHALL exist at the reference arity, including the corrected `frexp/0` and `modf/0`. Available functions SHALL match the matched-platform jq result, including domain errors, overflow, underflow, signed zero, and non-finite projection. Platform-unavailable functions SHALL implement the reference runtime-error contract rather than being absent or silently skipped. `have_decnum` SHALL describe actual numeric behavior truthfully. A comparison tolerance MUST NOT hide a different JSON value.

#### Scenario: Math inventory
- **WHEN** the complete Math-section function inventory is compared with registered functions and executable witnesses
- **THEN** every entry has correct arity and matched-platform execution evidence

#### Scenario: Numeric boundaries
- **WHEN** decimal precision boundaries, exponent extremes, signed zero, domain boundaries, or fused operations are evaluated
- **THEN** their observable results match jq without replacing an operation with an approximate expression

### Requirement: Error values retain jq types
Errors passed to `catch` SHALL preserve jq's error value, including non-string values and empty-result cases. Optional suppression SHALL affect only its lexical expression. Runtime control termination SHALL remain distinct from catchable errors.

#### Scenario: Structured error value
- **WHEN** the manual's `try error catch .` examples run
- **THEN** the caught values match jq in both JSON type and content rather than becoming diagnostic strings
