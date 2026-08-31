## MODIFIED Requirements

### Requirement: Recursive descent
The system SHALL implement jq recursive descent over arrays and insertion-ordered objects using jq's depth-first result order. It SHALL support `..`, `recurse/0`, `recurse/1`, and `recurse/2`; `recurse/0` SHALL behave as `recurse(.[]?)`, `recurse(f)` SHALL behave as `recurse(f; true)`, and `recurse(f; condition)` SHALL emit its input before recursively visiting each output of `f` for which `condition` is truthy. Traversal MUST remain pull-driven and MUST stop work that is no longer demanded by a downstream consumer.

#### Scenario: Deep traversal
- **WHEN** `..` or a recursive built-in traverses a nested value within configured limits
- **THEN** each visited value is emitted in jq order without native-stack recursion

#### Scenario: Default recursive traversal
- **WHEN** `[recurse(.[]?)]` runs on `{"a":[1]}`
- **THEN** it produces `[{"a":[1]},[1],1]`

#### Scenario: Recursive filter emits multiple children
- **WHEN** the filter passed to `recurse/1` emits multiple child values
- **THEN** every child and its descendants are emitted in jq depth-first order with jq-compatible multiplicity

#### Scenario: Conditional recursive traversal
- **WHEN** `2 | recurse(. + 1; . < 4)` is evaluated
- **THEN** it emits `2`, `3`, and no other values, because the root is unconditional and the generated value `4` fails the descent condition

#### Scenario: Downstream consumer stops early
- **WHEN** a downstream operator stops after consuming a prefix of a recursive built-in's results
- **THEN** the system does not evaluate pending descendants that cannot contribute another demanded result

#### Scenario: Traversal limit
- **WHEN** traversal exceeds depth, work, result, or cancellation limits
- **THEN** execution stops with the stable limit class and releases traversal frames

## ADDED Requirements

### Requirement: Post-order recursive walk
The system SHALL implement `walk/1` with jq 1.7-compatible post-order traversal. It MUST transform array elements and object values before applying the callback to their containing value, preserve array and object encounter order, and preserve jq's callback output multiplicity, empty-output behavior, and errors.

#### Scenario: Children are transformed before their container
- **WHEN** `walk(f)` processes a nested array or object
- **THEN** `f` observes each rebuilt child before it observes the rebuilt containing value

#### Scenario: Callback changes cardinality
- **WHEN** the callback passed to `walk/1` emits zero or multiple results at any visited value
- **THEN** `walk/1` emits the same ordered results and rebuilt structures as jq 1.7

#### Scenario: Object encounter order is stable
- **WHEN** `walk/1` transforms an object without changing its keys
- **THEN** the rebuilt object's keys retain their input encounter order

#### Scenario: Walk fails or is interrupted
- **WHEN** a callback errors, a configured resource limit is reached, or cancellation is requested during `walk/1`
- **THEN** execution stops with the jq-compatible error or stable limit class and releases pending walk state
