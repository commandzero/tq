# jq Core Language Specification

## Purpose

Define the jq-compatible value model, syntax, evaluation semantics, built-ins,
updates, numeric policy, and explicitly deferred language surface for tq.

## Requirements

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
The MVP SHALL support `+`, `-`, `*`, `/`, and `%` for jq-compatible operand combinations. Addition MUST include numeric addition, string concatenation, array concatenation, and object merge behavior covered by the baseline suite.

#### Scenario: Numeric arithmetic
- **WHEN** `(6 * 7) + 1` is evaluated
- **THEN** it emits numeric `43`

#### Scenario: Object addition
- **WHEN** `{"a":1} + {"b":2,"a":3}` is evaluated
- **THEN** it emits the jq-compatible merged object with deterministic key order

#### Scenario: Invalid arithmetic types
- **WHEN** unsupported operand types are combined
- **THEN** evaluation emits a runtime type error rather than coercing them silently

### Requirement: jq decimal-literal hybrid numbers
The runtime SHALL represent a finite number with a lazily derived IEEE-754 binary64 value and, when available, a normalized arbitrary-precision decimal literal. For numeric inputs inside the documented MVP resource envelope, literal parsing, identity, construction, equality, ordering, arithmetic, and canonical output SHALL match accepted decimal-enabled jq 1.8.x baseline cases. Identity conversion MUST avoid silent precision loss. jq-compatible arithmetic SHALL use binary64 behavior and SHALL NOT retain the operands' exact source literals in its result. Inputs outside the supported envelope MUST produce a specific numeric-range or resource diagnostic, and NaN or infinity MUST NOT be accepted as TOON, JSON, or YAML input.

#### Scenario: Large exact integer identity
- **WHEN** a supported exact integer passes through identity without arithmetic
- **THEN** its numeric value is preserved through TOON and JSON output

#### Scenario: Arithmetic leaves the literal domain
- **WHEN** arithmetic is applied to a decimal-literal value that binary64 cannot represent exactly
- **THEN** the result matches the accepted jq binary64-derived result and does not promise exact arbitrary-precision arithmetic

#### Scenario: Numeric envelope exceeded
- **WHEN** an input number cannot be represented under the accepted MVP policy without silent loss or excessive canonical expansion
- **THEN** evaluation or decoding fails with a numeric-range/resource diagnostic

#### Scenario: Non-finite input
- **WHEN** input attempts to encode NaN or positive or negative infinity
- **THEN** decoding fails instead of adding non-JSON numeric values to the runtime model

### Requirement: Variable binding
The language SHALL support `$name` references and `EXP as $name | EXP` lexical binding with jq-compatible scope and generator behavior. CLI-provided variables MUST enter the same resolved environment.

#### Scenario: Bind multiple values
- **WHEN** `.items[] as $item | $item.name` is evaluated
- **THEN** the body executes once per bound item in order

#### Scenario: Unknown variable
- **WHEN** a query references an unbound variable
- **THEN** resolution fails before execution with a source-spanned compile error

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
The MVP SHALL support `=`, `|=`, `+=`, `-=`, `*=`, `/=`, and `//=` for jq-compatible assignable paths. Updates MUST preserve unaffected structure/order and MUST handle multi-path left sides according to accepted jq cases.

#### Scenario: Relative update
- **WHEN** `.count |= . + 1` is evaluated against `{"count":4,"name":"x"}`
- **THEN** it emits `{"count":5,"name":"x"}` with unaffected member order preserved

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

### Requirement: Deferred syntax is explicit
Labels and `break`, recursive built-ins such as `recurse` and `walk`, and non-finite result built-ins SHALL remain deferred unless separately promoted by an accepted spec. Unsupported syntax or built-ins MUST fail at compile time with a stable capability identifier. User-defined functions, modules, `reduce`, `foreach`, recursive descent, string interpolation, format strings, regex, date, environment, and admitted platform built-ins are supported by their accepted specifications and MUST NOT be described as deferred.

#### Scenario: Deferred labels and break
- **WHEN** a query contains `label $out` or `break $out`
- **THEN** compilation fails with the stable labels or break capability identifier and does not partially execute the query

#### Scenario: Deferred recursive built-in
- **WHEN** a query calls `recurse` or `walk`
- **THEN** compilation fails with the stable recursive-builtins capability identifier
