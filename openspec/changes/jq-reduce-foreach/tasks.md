## 1. Semantics and Analysis

- [ ] 1.1 Add jq differential cases for zero/many generators, update multiplicity, extraction, scope, and errors
- [ ] 1.2 Implement `reduce` and `foreach` grammar, spans, and accumulator resolution
- [ ] 1.3 Extend cardinality, failure, retention, and event-plan analysis

## 2. Managed Fold Execution

- [ ] 2.1 Add validated fold continuation instructions and bounded accumulator frames
- [ ] 2.2 Execute reduce update generators with jq-compatible ordering and sharing
- [ ] 2.3 Execute foreach extraction generators with valid partial-output unwinding

## 3. Release Evidence

- [ ] 3.1 Add limit, cancellation, hostile-depth, fuzz, and structural-sharing regressions
- [ ] 3.2 Run compatibility and natural-data reduction benchmarks and document memory classes
