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
The MVP parser SHALL support identity `.`, parentheses, pipes, comma generators, scalar literals, array/object literals, field access, computed access, array indexing, slices, iteration, optional suffix `?`, variable references, `as` bindings, conditionals, arithmetic/comparison/boolean/alternative operators, and update operators. Source spans MUST be retained for every syntax node.

#### Scenario: Parse nested composition
- **WHEN** `.features[] | select(.properties.mag >= 4) | {id, title: .properties.title}` is parsed
- **THEN** the parser produces a complete source-spanned syntax tree without interpreting the input data

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
The MVP SHALL provide compatibility-tested implementations of `empty`, `error`, `type`, `length`, `utf8bytelength`, `keys`, `keys_unsorted`, `has`, `in`, `select`, `map`, `map_values`, `values`, `scalars`, `arrays`, `objects`, `iterables`, `booleans`, `numbers`, `strings`, `nulls`, `tostring`, `tonumber`, `add`, `min`, `max`, `sort`, `sort_by`, `unique`, `unique_by`, `reverse`, `flatten`, and `range`.

#### Scenario: Map
- **WHEN** `map(. + 1)` is evaluated against `[1,2,3]`
- **THEN** it emits `[2,3,4]`

#### Scenario: Type selector
- **WHEN** `numbers` is evaluated across mixed inputs
- **THEN** it emits only numeric inputs and emits no result for other types

#### Scenario: Blocking built-in
- **WHEN** `sort_by(.score)` is compiled
- **THEN** analysis marks the operation as blocking before evaluation

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

### Requirement: Deferred syntax is explicit
User-defined `def`, modules/import/include, `reduce`, `foreach`, labels/break, recursive descent, string interpolation, format strings, advanced regex, date, environment, and platform I/O built-ins SHALL remain outside this MVP unless separately promoted by an accepted spec. Unsupported syntax MUST fail at compile time with a stable capability identifier.

#### Scenario: Deferred reduce
- **WHEN** an MVP binary receives a `reduce` expression
- **THEN** it reports that `reduce` belongs to a deferred compatibility capability and does not partially execute the query
