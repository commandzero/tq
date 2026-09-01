## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: Deferred syntax is explicit
Non-finite result built-ins SHALL remain deferred unless separately promoted by an accepted spec. Unsupported syntax or built-ins MUST fail at compile time with a stable capability identifier. Labels, `break`, `recurse`, `walk`, user-defined functions, modules, `reduce`, `foreach`, recursive descent, string interpolation, format strings, regex, date, environment, and admitted platform built-ins are supported by their accepted specifications and MUST NOT be described as deferred.

#### Scenario: Deferred non-finite result built-in
- **WHEN** a query calls a non-finite result built-in that has not been promoted by an accepted spec
- **THEN** compilation fails with the stable non-finite-results capability identifier and does not partially execute the query

#### Scenario: Deferred labels and break
- **WHEN** a query contains `label $out` or `break $out`
- **THEN** compilation MUST NOT return the former labels or break deferred-capability error and the query is compiled according to the lexical-label requirement

#### Scenario: Deferred recursive built-in
- **WHEN** a query calls a supported arity of `recurse` or `walk`
- **THEN** compilation MUST NOT return the former recursive-builtins deferred-capability error and the query is compiled according to the recursive traversal requirements
