## MODIFIED Requirements

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
