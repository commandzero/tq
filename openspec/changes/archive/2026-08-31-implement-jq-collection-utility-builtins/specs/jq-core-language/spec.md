## MODIFIED Requirements

### Requirement: Core built-in functions
The MVP SHALL provide compatibility-tested implementations of `empty`, `error`, `type`, `length`, `utf8bytelength`, `keys`, `keys_unsorted`, `has`, `in`, `select`, `map`, `map_values`, `values`, `scalars`, `arrays`, `objects`, `iterables`, `booleans`, `numbers`, `strings`, `nulls`, `tostring`, `tonumber`, `add`, `min`, `max`, `sort`, `sort_by`, `unique`, `unique_by`, `reverse`, `flatten`, `range`, `to_entries`, `with_entries`, `group_by`, `min_by`, `max_by`, `limit`, `paths`, `path`, `getpath`, `setpath`, `tostream`, `tojson`, `fromjson`, `inputs`, `any`, `all`, `ltrimstr`, `ascii_downcase`, `explode`, `implode`, `floor`, `ceil`, and `fabs`.

#### Scenario: Map
- **WHEN** `map(. + 1)` is evaluated against `[1,2,3]`
- **THEN** it emits `[2,3,4]`

#### Scenario: Type selector
- **WHEN** `numbers` is evaluated across mixed inputs
- **THEN** it emits only numeric inputs and emits no result for other types

#### Scenario: Blocking built-in
- **WHEN** `sort_by(.score)` is compiled
- **THEN** analysis marks the operation as blocking before evaluation

#### Scenario: Issue 5 filters resolve
- **WHEN** any built-in added by issue #5 is called with its supported arity
- **THEN** resolution succeeds before tq consumes input

## ADDED Requirements

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

