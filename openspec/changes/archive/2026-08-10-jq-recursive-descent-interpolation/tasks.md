## 1. Front End and Baselines

- [x] 1.1 Add jq byte/result cases for recursive order, scalars, deep values, interpolation escapes, generators, and errors
- [x] 1.2 Implement recursive-descent and interpolation tokens, AST nodes, spans, and resolution
- [x] 1.3 Extend effect/cardinality analysis and stable unsupported diagnostics for adjacent grammar

## 2. Managed Execution

- [x] 2.1 Implement explicit depth-first traversal cursor frames with limits and cancellation
- [x] 2.2 Compile interpolation segments to ordered generator continuations
- [x] 2.3 Implement jq-compatible interpolation conversion, escaping, errors, and partial output

## 3. Release Evidence

- [x] 3.1 Add hostile-depth/result-explosion tests and parser/VM fuzz coverage
- [x] 3.2 Run full compatibility and traversal benchmarks and publish resource guidance
