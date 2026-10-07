# jq Core Language Specification

## Purpose

Define the jq-compatible value model, syntax, evaluation semantics, built-ins,
updates, numeric policy, and explicitly deferred language surface for tq.

## Requirements

All reference-matching requirements below are subject to the reviewed safe-library disparity contract in `cross-tool-compatibility`. This exception does not excuse missing functions, wrong arities, uninvestigated errors, or regressions. Safe Rust math implementations need not reproduce platform C math bit for bit; measured differences remain visible and individually bounded when approved. For implementation/evidence closeout only, the six rounding, two non-rounding scale-conversion, and eleven Windows reference-availability target/case contracts explicitly mapped in `cross-tool-compatibility` remain unresolved follow-ups under #70/#69. This does not defer function implementation, arity coverage, native execution, or bounded-resource regressions and does not approve or normalize any difference.

### Requirement: JSON-shaped runtime values
The core language SHALL operate on null, boolean, number, string, array, and insertion-ordered object values. It MUST preserve array order and object encounter order and MUST distinguish zero emitted results from an emitted null.

#### Scenario: Identity value types
- **WHEN** identity is evaluated for each supported value type
- **THEN** exactly one value of the same type and content is emitted

#### Scenario: Empty versus null
- **WHEN** `empty` and `null` are evaluated separately
- **THEN** `empty` emits zero results and `null` emits one null result

### Requirement: jq-compatible truthiness
Only `false` and `null` SHALL be falsey. All other values, including zero, the empty string, empty array, and empty object, SHALL be truthy.

#### Scenario: Empty array condition
- **WHEN** `if [] then "yes" else "no" end` is evaluated
- **THEN** it emits `"yes"`

#### Scenario: Null condition
- **WHEN** `if null then "yes" else "no" end` is evaluated
- **THEN** it emits `"no"`

### Requirement: Core lexical and grammatical forms
The MVP parser SHALL support identity `.`, parentheses, pipes, comma generators, scalar literals, array/object literals, field access, computed access, array indexing, slices, iteration, optional suffix `?`, variable references, `as` bindings, conditionals, arithmetic/comparison/boolean/alternative operators, and update operators. Function calls MUST parse each semicolon-delimited filter argument as a complete comma expression, and comma expressions MUST NOT increase the resolved function arity. Source spans MUST be retained for every syntax node.

#### Scenario: Parse nested composition
- **WHEN** `.features[] | select(.properties.mag >= 4) | {id, title: .properties.title}` is parsed
- **THEN** the parser produces a complete source-spanned syntax tree without interpreting the input data

#### Scenario: Parse a comma generator as one function argument
- **WHEN** `sort_by(.a,.b)` is parsed and resolved
- **THEN** the call has one filter argument whose expression emits `.a` followed by `.b`

#### Scenario: Keep semicolon-delimited function arity
- **WHEN** `def pair(f; g): [f, g]; pair(1,2; 3,4)` is parsed and resolved
- **THEN** the call has two filter arguments and each argument is a comma generator

#### Scenario: Invalid syntax
- **WHEN** a required delimiter or `end` token is missing
- **THEN** parsing fails with a compile-class diagnostic at the unexpected token or end of query

### Requirement: Field and computed access
Field access, computed object access, array indexing, and slicing SHALL match jq behavior for supported value types, missing members, negative indices, and out-of-range positions.

#### Scenario: Missing object member
- **WHEN** `.missing` is evaluated against an object without that key
- **THEN** it emits null

#### Scenario: Negative array index
- **WHEN** `.[-1]` is evaluated against `[10,20,30]`
- **THEN** it emits `30`

#### Scenario: Slice
- **WHEN** `.[1:3]` is evaluated against `[0,1,2,3]`
- **THEN** it emits `[1,2]`

#### Scenario: Invalid index type
- **WHEN** an array is indexed with an object without optional suppression
- **THEN** evaluation produces a runtime type error

### Requirement: Iteration and generators
Array/object iteration, pipe composition, and comma composition SHALL preserve jq result cardinality and order. Every upstream result SHALL independently feed the downstream filter.

#### Scenario: Array iteration
- **WHEN** `.[]` is evaluated against `["a","b","c"]`
- **THEN** it emits `"a"`, `"b"`, and `"c"` in order

#### Scenario: Pipe multiplicity
- **WHEN** `(.a, .b) | (.x, .y)` is evaluated against compatible objects
- **THEN** downstream evaluation runs for each upstream result in jq-compatible order

### Requirement: Literal construction
Array and object constructors SHALL collect or combine generator results with jq-compatible semantics. Object constructors MUST support explicit key/value entries, string keys, computed keys, and identifier shorthand for supported paths.

#### Scenario: Array constructor
- **WHEN** `[.items[] | .name]` is evaluated
- **THEN** all emitted names are collected into one ordered array

#### Scenario: Object shorthand
- **WHEN** `{id, name: .profile.display}` is evaluated
- **THEN** it emits one object whose keys appear in constructor order

#### Scenario: Computed object key
- **WHEN** `{(.key): .value}` is evaluated with a string key
- **THEN** it emits an object containing that dynamic key

### Requirement: Conditionals and boolean control
The language SHALL support `if/then/elif/else/end`, `and`, `or`, unary `not`, and alternative `//` with jq-compatible truthiness, short-circuiting, and result multiplicity.

#### Scenario: Alternative
- **WHEN** `.nickname // .name // "unknown"` is evaluated
- **THEN** it emits the first non-false/non-null results according to jq semantics

#### Scenario: Boolean short circuit
- **WHEN** the left operand determines the result of `and` or `or`
- **THEN** evaluation does not execute an unnecessary erroring right operand

### Requirement: Comparison and ordering
Equality, inequality, relational comparison, sort, min/max, and uniqueness SHALL use jq-compatible deep equality and total type ordering for the supported value model. Object comparison MUST be deterministic and covered by baseline cases.

#### Scenario: Deep equality
- **WHEN** two nested arrays/objects contain equal ordered values
- **THEN** `==` emits true

#### Scenario: Cross-type ordering
- **WHEN** values of different JSON-model types are sorted
- **THEN** their order matches the accepted jq baseline for the MVP reference version

### Requirement: Arithmetic and overloaded addition
The MVP SHALL support `+`, `-`, `*`, `/`, and `%` for jq-compatible operand combinations. Addition MUST include numeric addition, string concatenation, array concatenation, and object merge behavior covered by the baseline suite. Multiplication MUST include numeric multiplication and recursive object merge. At a key present in both object operands, multiplication MUST recursively merge the values when both are objects and MUST otherwise use the right-hand value. The result MUST retain the left operand's key positions and append right-only keys in right operand order, including within recursively merged objects.

#### Scenario: Numeric arithmetic
- **WHEN** `(6 * 7) + 1` is evaluated
- **THEN** it emits numeric `43`

#### Scenario: Object addition
- **WHEN** `{"a":1} + {"b":2,"a":3}` is evaluated
- **THEN** it emits the jq-compatible merged object with deterministic key order

#### Scenario: Recursive object multiplication
- **WHEN** `{"a":{"b":1}} * {"a":{"c":2}}` is evaluated
- **THEN** it emits `{"a":{"b":1,"c":2}}`

#### Scenario: Object multiplication conflict and order
- **WHEN** `{"a":{"x":1},"keep":0} * {"a":2,"new":3}` is evaluated
- **THEN** it emits `{"a":2,"keep":0,"new":3}` in that key order

#### Scenario: Invalid arithmetic types
- **WHEN** unsupported operand types are combined
- **THEN** evaluation emits a runtime type error rather than coercing them silently

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
- **WHEN** a query references an unbound variable other than a jq special variable
- **THEN** resolution fails before execution with a source-spanned compile error

#### Scenario: Built-in variable needs no declaration
- **WHEN** a query references `$ENV` or `$__loc__` without `--arg`, `--argjson`, or `--argtoon`
- **THEN** resolution succeeds and the reference is evaluated according to its built-in contract

#### Scenario: Lexical environment shadowing
- **WHEN** `1 as $ENV | $ENV` is evaluated
- **THEN** the lexical binding supplies `1` within its scope instead of the ambient environment object

#### Scenario: External special-variable arguments do not override built-ins
- **WHEN** `--arg ENV replacement` or `--arg __loc__ replacement` is supplied and the query references the corresponding special variable
- **THEN** the built-in variable semantics remain in effect

#### Scenario: Destructuring alternatives
- **WHEN** the manual's `?//` examples select different input shapes
- **THEN** variable availability, null bindings, branch errors, and output order match jq

#### Scenario: Nested lexical capture
- **WHEN** a function's filter argument references an outer binding that is shadowed at the call site
- **THEN** values resolve in the jq lexical environment rather than the caller's shadowing environment

### Requirement: jq built-in variables
The language SHALL provide jq-compatible `$ENV` and `$__loc__` variable references without requiring callers to declare them as external variables. `$ENV` SHALL represent the process-start environment snapshot admitted by the host, and `$__loc__` SHALL represent the source location of the reference.

#### Scenario: Environment variable reference
- **WHEN** a query evaluates `$ENV` with environment access admitted
- **THEN** it emits one object whose keys and values are strings and whose contents match the invocation's startup environment snapshot

#### Scenario: Environment variable reference is capability-gated
- **WHEN** a query references `$ENV` without admitted environment access
- **THEN** resolution succeeds as a known jq variable and evaluation returns the same policy-class failure used by the `env` builtin

#### Scenario: Top-level source location
- **WHEN** the query `$__loc__` is evaluated from a one-line top-level source
- **THEN** it emits one ordered object with `file` set to the source identity and `line` set to the one-based number `1`

#### Scenario: Multi-line source location
- **WHEN** `$__loc__` occurs on a later line of a named query source
- **THEN** its `line` value identifies the one-based source line containing that reference, independent of the input document

#### Scenario: Source location inside a definition
- **WHEN** a filter definition evaluates `$__loc__` from its body
- **THEN** the returned line identifies the reference's definition source location rather than the call site

### Requirement: Core built-in functions
The MVP SHALL provide compatibility-tested implementations of `empty`, `error`, `type`, `length`, `utf8bytelength`, `keys`, `keys_unsorted`, `has`, `in`, `select`, `map`, `map_values`, `values`, `scalars`, `arrays`, `objects`, `iterables`, `booleans`, `numbers`, `strings`, `nulls`, `tostring`, `tonumber`, `add`, `min`, `max`, `sort`, `sort_by`, `unique`, `unique_by`, `reverse`, `flatten`, and `range`. `sort_by` and `unique_by` MUST use the complete ordered result sequence from their filter argument as the comparison key.

#### Scenario: Map
- **WHEN** `map(. + 1)` is evaluated against `[1,2,3]`
- **THEN** it emits `[2,3,4]`

#### Scenario: Type selector
- **WHEN** `numbers` is evaluated across mixed inputs
- **THEN** it emits only numeric inputs and emits no result for other types

#### Scenario: Blocking built-in
- **WHEN** `sort_by(.score)` is compiled
- **THEN** analysis marks the operation as blocking before evaluation

#### Scenario: Sort by a generated composite key
- **WHEN** `sort_by(.a,.b)` is evaluated against `[{"a":1,"b":2},{"a":1,"b":1},{"a":0,"b":9}]`
- **THEN** it emits `[{"a":0,"b":9},{"a":1,"b":1},{"a":1,"b":2}]`

#### Scenario: Deduplicate by a generated composite key
- **WHEN** `unique_by(.a,.b)` is evaluated against values with repeated and distinct `.a`, `.b` pairs
- **THEN** it retains one value for each distinct ordered pair using jq-compatible ordering

#### Scenario: Empty generated key
- **WHEN** `sort_by(empty)` is evaluated against an array
- **THEN** all elements compare with the same empty composite key and retain their stable order

#### Scenario: Issue 5 filters resolve
- **WHEN** any built-in added by issue #5 is called with its supported arity
- **THEN** resolution succeeds before tq consumes input

### Requirement: Collection transforms and bounded generators
Collection filters SHALL match jq 1.8.x result values, ordering, generator cardinality, and type errors. `to_entries` SHALL retain object encounter order and array index order. `with_entries` SHALL apply its filter with jq generator semantics. `group_by`, `min_by`, and `max_by` SHALL compare every key result using jq total ordering. `limit(n; expression)` SHALL emit at most `n` results and MUST stop evaluating the expression once it has emitted that count.

#### Scenario: Object entries preserve encounter order
- **WHEN** `to_entries` is evaluated against `{"z":1,"a":2}`
- **THEN** it emits `[{"key":"z","value":1},{"key":"a","value":2}]`

#### Scenario: Group by evaluated key
- **WHEN** `group_by(.kind)` is evaluated against an unsorted array of objects
- **THEN** it sorts by `.kind` using jq ordering and emits one array per distinct key

#### Scenario: Empty keyed extrema
- **WHEN** `min_by(.)` or `max_by(.)` is evaluated against an empty array
- **THEN** it emits `null`

#### Scenario: Limit stops its generator
- **WHEN** `limit(2; range(0; 1000000))` is evaluated
- **THEN** it emits `0` and `1` without evaluating the remaining range results

### Requirement: Path inspection and mutation filters
`paths`, `path`, `getpath`, `setpath`, and `tostream` SHALL use jq path arrays whose components are object-key strings or non-negative array indices. Traversal SHALL preserve array order and object encounter order. `setpath` SHALL create missing object and array ancestors according to jq behavior, while malformed paths and incompatible traversals MUST produce runtime path or type errors.

#### Scenario: Enumerate nested paths
- **WHEN** `[paths]` is evaluated against `{"a":[10]}`
- **THEN** it emits `[["a"],["a",0]]` and does not include the empty root path

#### Scenario: Capture an expression path
- **WHEN** `path(.a[0])` is evaluated against `{"a":[10]}`
- **THEN** it emits `["a",0]`

#### Scenario: Read and create a path
- **WHEN** `[getpath(["a",0]), setpath(["b",1]; 7)]` is evaluated against `{"a":[10]}`
- **THEN** it reads `10` and creates `{"a":[10],"b":[null,7]}` without changing the original value

#### Scenario: Stream representation order
- **WHEN** `tostream` is evaluated against a nested array or object
- **THEN** it emits jq-compatible leaf and container-close records in depth-first encounter order

### Requirement: JSON text conversion
`tojson` SHALL encode its input as one compact jq-compatible JSON string. `fromjson` SHALL decode exactly one JSON value using tq's JSON value and exact-number policy. Both filters MUST enforce existing depth, numeric, and output resource limits and MUST return a runtime or input-class error for invalid JSON rather than reading external input.

#### Scenario: JSON round trip
- **WHEN** `tojson | fromjson` is evaluated against any supported JSON-shaped value
- **THEN** it emits an equal value with object encounter order preserved

#### Scenario: Invalid JSON string
- **WHEN** `fromjson` is evaluated against `"not json"`
- **THEN** it produces a runtime error and emits no value

### Requirement: Predicate, text, character, and math utilities
`any(generator; condition)` and `all(generator; condition)` SHALL preserve jq truthiness and short-circuit evaluation. `ltrimstr`, `ascii_downcase`, `explode`, `implode`, `floor`, `ceil`, and `fabs` SHALL match jq 1.8.x for supported values, Unicode scalar conversion, numeric results, and type errors. `ascii_downcase` MUST change only ASCII uppercase letters.

#### Scenario: Predicates short circuit
- **WHEN** `any(range(0; 10); . == 1)` or `all(range(0; 10); . < 1)` is evaluated
- **THEN** evaluation stops as soon as the result is known

#### Scenario: ASCII-only lowercase
- **WHEN** `ascii_downcase` is evaluated against `"ABCÉ"`
- **THEN** it emits `"abcÉ"`

#### Scenario: Unicode character round trip
- **WHEN** `explode | implode` is evaluated against a valid Unicode string
- **THEN** it emits the original string

#### Scenario: Numeric utilities
- **WHEN** `[floor, ceil, fabs]` is evaluated against `-1.5`
- **THEN** it emits `[-2,-1,1.5]` under tq's jq number policy

### Requirement: Built-in resource accounting
Every added filter SHALL charge work to existing VM limits. Generator filters MUST remain pull-driven where jq permits early termination. Materializing filters MUST expose their blocking classification during analysis, and recursive path or stream traversal MUST enforce the configured path and call-stack limits.

#### Scenario: Limited path depth
- **WHEN** `paths` or `tostream` traverses a value deeper than the configured path limit
- **THEN** evaluation stops with the stable path-stack resource diagnostic

#### Scenario: Limited generated output
- **WHEN** an added generator reaches the configured VM step or result limit
- **THEN** evaluation stops with the corresponding resource diagnostic without emitting unbounded additional results

### Requirement: Errors, optional access, and try/catch
The MVP SHALL support `error`, optional suffix `?`, and `try EXP catch EXP` with jq-compatible value/error flow for covered cases. Optional suppression MUST suppress only errors within its defined scope and MUST NOT convert every error into null.

#### Scenario: Optional iteration
- **WHEN** `.items[]?` is evaluated against a value that cannot be iterated
- **THEN** it emits no result instead of a runtime error

#### Scenario: Catch error
- **WHEN** `try error("bad") catch .` is evaluated
- **THEN** it emits the jq-compatible error value supplied to the catch filter

### Requirement: Path update operators
The MVP SHALL support `=`, `|=`, `+=`, `-=`, `*=`, `/=`, and `//=` for jq-compatible assignable paths. Arithmetic updates MUST use the same operand semantics as their corresponding binary operators. Updates MUST preserve unaffected structure/order and MUST handle multi-path left sides according to accepted jq cases.

#### Scenario: Relative update
- **WHEN** `.count |= . + 1` is evaluated against `{"count":4,"name":"x"}`
- **THEN** it emits `{"count":5,"name":"x"}` with unaffected member order preserved

#### Scenario: Recursive object multiplication update
- **WHEN** `.settings *= {"display":{"theme":"dark"}}` is evaluated against `{"settings":{"display":{"density":"compact"},"cache":true}}`
- **THEN** it emits `{"settings":{"display":{"density":"compact","theme":"dark"},"cache":true}}`

#### Scenario: Selected path update
- **WHEN** `(.items[] | select(.id == 2) | .active) = true` is evaluated
- **THEN** only the selected root path is updated and the complete updated root is emitted

#### Scenario: Invalid update path
- **WHEN** the left side emits a non-path value
- **THEN** evaluation fails with a path-assignment runtime error

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

### Requirement: Lexical labels and break
The system SHALL parse and execute jq lexical `label $name | expression` and `break $name` control flow. Labels MUST use a lexical namespace distinct from value variables, a break MUST target the nearest visible label with the same name, and a matched break MUST make that label behave as though its protected expression produced `empty` from that point forward. Pending alternatives inside the matched label MUST be abandoned while values already emitted before the break remain emitted.

#### Scenario: Break exits a generating expression
- **WHEN** `label $out | foreach .[] as $item (null; $item; if . == false then break $out else . end)` runs on `[1,2,false,3,null]`
- **THEN** it emits `1` and `2` and does not evaluate the remaining items

#### Scenario: Nearest shadowing label wins
- **WHEN** nested labels use the same name and the inner expression breaks that name
- **THEN** the inner label exits while the outer label remains available to subsequent outer expressions

#### Scenario: Label and value namespaces are distinct
- **WHEN** a value variable and a label have the same source name
- **THEN** references to the value variable and `break` resolve independently

#### Scenario: Break has no visible label
- **WHEN** a query contains a `break $name` with no lexically visible matching label
- **THEN** compilation fails before execution with a source-spanned unbound-label error

#### Scenario: Try inside a label catches a break
- **WHEN** `label $out | try (1, break $out, 2) catch "caught", 3` is evaluated
- **THEN** it emits `1`, `"caught"`, and `3`, matching jq 1.7 control-flow boundary ordering

#### Scenario: Label inside try consumes a break first
- **WHEN** `try (label $out | 1, break $out, 2) catch "caught"` is evaluated
- **THEN** it emits only `1` and the outer catch does not run

#### Scenario: Break crosses function and reducer frames
- **WHEN** a visible label is broken from a called function, `reduce`, or `foreach` expression
- **THEN** the break reaches that label without leaking pending call or reducer alternatives

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
All functions listed in the pinned Math section SHALL exist at the reference arity, including the corrected `frexp/0` and `modf/0`. Safe standard-library and pure-Rust math implementations SHALL target jq domain errors, overflow, underflow, signed zero, and non-finite projection. Measured platform or rounding disparities SHALL follow the reviewed disparity contract and retain both observed values. Platform-unavailable functions SHALL have a tested runtime availability contract rather than being absent or silently skipped. `have_decnum` SHALL describe actual numeric behavior truthfully. A comparison tolerance MUST NOT hide a different JSON value or label it an exact match.

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
