## 1. Semantics and Analysis

- [x] 1.1 Add jq differential cases for zero/many generators, update multiplicity, extraction, scope, and errors
- [x] 1.2 Implement `reduce` and `foreach` grammar, spans, and accumulator resolution
- [x] 1.3 Extend cardinality, failure, retention, and event-plan analysis

## 2. Managed Fold Execution

- [x] 2.1 Add validated fold bytecode operands and bounded accumulator evaluation state
- [x] 2.2 Execute reduce update generators with jq-compatible ordering and sharing
- [x] 2.3 Execute foreach extraction generators with valid partial-output unwinding

## 3. Release Evidence

- [x] 3.1 Add limit, cancellation, hostile-depth, fuzz, and structural-sharing regressions
- [x] 3.2 Run compatibility and natural-data reduction benchmarks and document memory classes
