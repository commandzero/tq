## 1. Compatibility fixtures and registry

- [x] 1.1 Add jq baseline cases for every issue #5 builtin and supported arity, including ordering, multiplicity, empty input, invalid types, malformed paths, and short-circuit sentinels; verify the compatibility manifest coverage test finds every new case.
- [x] 1.2 Record and review jq 1.8.x observations for the new cases; verify the baseline report has no unexplained reference failures.
- [x] 1.3 Add the 23 builtin signatures and blocking classifications to `BuiltinRegistry`; verify resolver tests accept supported arities, reject unsupported arities, and still reject unknown names before input consumption.
- [x] 1.4 Mark new builtins ineligible for automatic event, subtree, hybrid, and transcode plans until proved safe; verify explain tests select the document plan for representative calls.

## 2. Collection and scalar utilities

- [x] 2.1 Implement `to_entries` and `with_entries` with object encounter order, array index order, filter-generator cardinality, and jq type errors; verify focused evaluator tests and their compatibility cases pass.
- [x] 2.2 Implement `group_by`, `min_by`, and `max_by` using the existing jq total-order comparison and filter-key evaluation; verify empty arrays, multiple key results, stable grouping, and invalid input cases match the baseline.
- [x] 2.3 Implement pull-driven `limit` so reaching the requested count stops its child generator; verify a sentinel error after the limit is not evaluated and VM step limits still apply.
- [x] 2.4 Implement short-circuiting `any(generator; condition)` and `all(generator; condition)`; verify truthiness, empty generators, multiple condition results, early termination, and pre-decision errors against jq.
- [x] 2.5 Implement `ltrimstr`, `ascii_downcase`, `explode`, `implode`, `floor`, `ceil`, and `fabs` in focused value helpers; verify Unicode, invalid code points, exact-number boundaries, and wrong-type cases match jq.

## 3. Path and stream filters

- [x] 3.1 Add checked conversion between jq path arrays and `PathComponent` values; verify string keys, exact non-negative indices, empty paths, and malformed components have jq-compatible results or errors.
- [x] 3.2 Factor a bounded depth-first path walker shared by `paths` and `tostream`; verify object encounter order, array order, leaf and container-close records, empty containers, path-stack limits, and cancellation.
- [x] 3.3 Expose the existing assignment path collector to `path(expression)`; verify field, computed-index, iteration, optional, missing, and multi-result expressions match jq cardinality and errors.
- [x] 3.4 Implement `getpath` and `setpath` using the shared path conversion and persistent update helpers; verify reads, root replacement, missing ancestor creation, sparse arrays, structural sharing, and incompatible traversal errors.

## 4. JSON conversion

- [x] 4.1 Extract or expose bounded JSON value encode and decode helpers for evaluator use without CLI I/O; verify they retain object order and enforce tq's numeric, duplicate-key, depth, and byte policies.
- [x] 4.2 Implement `tojson` and `fromjson` through the shared helpers; verify compact strings, exact-number round trips, Unicode escaping, invalid JSON, wrong input types, and resource failures against the approved compatibility policy.

## 5. Remaining input provider

- [x] 5.1 Add an optional pull-based remaining-input provider to document evaluation with an in-memory test provider; verify exhaustion emits no result and provider errors retain their classified failure.
- [x] 5.2 Refactor the CLI document input driver so the outer loop and `inputs` share one ordered source cursor while preserving file identity, decoding limits, proxy behavior, cancellation, slurp, raw-input, and null-input rules; verify existing CLI input tests remain green.
- [x] 5.3 Implement `inputs` against the provider; verify stdin sequences, multiple files, partial consumption through `limit`, nested calls, exhaustion, malformed later input, and absence of duplicate top-level evaluation.

## 6. Integration and documentation

- [x] 6.1 Enable tq for the new compatibility cases and run the focused jq/tq campaign; verify normalized values, order, cardinality, errors, and exit status match every reviewed baseline.
- [x] 6.2 Add resource regression tests for large generators, deep paths, JSON limits, interruption, and output limits; verify each stops with the expected stable diagnostic.
- [x] 6.3 Add representative correctness-gated JSON benchmarks for each issue #5 builtin family and jq comparison fields for median wall time and maximum observed peak RSS; verify the report marks ratios at or below 2.0 time and 1.5 memory as met, higher ratios as soft misses, and invalid comparisons as not comparable.
- [x] 6.4 Run the representative jq/tq benchmark campaign on a manifest-recorded host and retain the measured ratios and dispersion; verify a soft target miss remains visible without failing implementation acceptance.
- [x] 6.5 Update `docs/compatibility.md`, builtin coverage assertions, performance documentation, and requirements traceability for the implemented set; verify documentation checks and traceability generation pass.
- [x] 6.6 Run formatting, clippy, the full Cargo test suite, and strict OpenSpec validation; verify all commands pass before marking issue #5 complete.
