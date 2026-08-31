## MODIFIED Requirements

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
