## ADDED Requirements

### Requirement: Issue 5 built-in compatibility coverage
The compatibility manifest SHALL contain jq-target cases for every built-in added by issue #5. The cases MUST cover successful values, output order and cardinality, supported arities, type or path errors, empty inputs, short-circuit behavior, and applicable resource limits. tq support MUST be enabled only after its normalized observations match the reviewed jq 1.8.x baseline, except for differences already permitted by the project compatibility policy.

#### Scenario: Every added built-in has a case
- **WHEN** the compatibility manifest coverage test runs
- **THEN** it finds at least one enabled tq case for each issue #5 built-in and its supported arity

#### Scenario: Generator cardinality differs
- **WHEN** an added generator emits too many, too few, or reordered results compared with jq
- **THEN** the compatibility campaign fails and identifies the first differing result

#### Scenario: Error classification differs
- **WHEN** tq accepts an invalid value that jq rejects or reports a different stable error class
- **THEN** the compatibility campaign records the mismatch rather than treating the filter as compatible

