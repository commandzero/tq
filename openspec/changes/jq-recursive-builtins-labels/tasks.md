## 1. Label Syntax and Resolution

- [ ] 1.1 Add failing lexer and parser tests for `label $name | expression` and `break $name`, then add dedicated tokens and AST forms; verify the focused parser tests preserve spans and reject malformed forms.
- [ ] 1.2 Add failing resolver tests for nearest-label shadowing, a same-named value variable, and an unbound break, then resolve labels to a separate symbol space; verify unbound breaks fail before execution at the break span.
- [ ] 1.3 Add label and break bytecode operations, structural validation, display, and executable-closure coverage; verify bytecode round-trip and malformed-bytecode tests pass before enabling runtime admission.

## 2. Label Runtime Control Flow

- [ ] 2.1 Add failing evaluator tests for basic exit, values emitted before exit, sibling pruning, and nested shadowing, then implement label ownership and matching-break cleanup on the explicit work stack; verify the focused evaluator tests match jq output order.
- [ ] 2.2 Add failing differential tests with `try` inside and outside a label, then route breaks through ordered catch and label boundaries; verify both specified catch-order queries match jq 1.7 exactly.
- [ ] 2.3 Add failing cases that break through user calls, filter parameters, `reduce`, and `foreach`, then propagate and clean up the structured exit across those continuations; verify no pending inner alternative runs after the break.
- [ ] 2.4 Add work-limit, stack-limit, cancellation, and output-prefix tests for label exits; verify each interruption returns the stable class and releases label-owned work.

## 3. Recursive Built-ins

- [ ] 3.1 Add failing registry, arity, planner, and executable-closure tests for `recurse/0`, `recurse/1`, `recurse/2`, and `walk/1`, then register their filter signatures and route metadata; verify unsafe streaming routes are rejected before input.
- [ ] 3.2 Add failing jq differential tests for root-first traversal, default child selection, multi-result child filters, conditional descent, and error order, then implement managed `recurse` continuations; verify all three arities produce jq's depth-first sequence.
- [ ] 3.3 Add deep-input, downstream `first` or `limit`, resource-limit, and cancellation tests for `recurse`; verify traversal does not use the native stack or evaluate descendants after demand stops.
- [ ] 3.4 Add failing jq differential tests for scalar, array, and insertion-ordered object walks with zero-, one-, and multi-result callbacks, then implement post-order walk and branching rebuild continuations; verify ordered outputs and rebuilt structures match jq 1.7.
- [ ] 3.5 Add error, deep-input, resource-limit, cancellation, and early-stop tests for `walk`; verify pending partial rebuilds are released and every branch is charged to existing limits.

## 4. Compatibility and Robustness

- [ ] 4.1 Promote the issue #6 label, recurse, and walk cases from the deferred catalog into jq 1.7 differential fixtures while retaining non-finite results as deferred; verify the compatibility runner reports no unexpected defer or mismatch.
- [ ] 4.2 Add the issue examples plus nested labels, catches, callback cardinality, and deep recursive inputs to CLI integration tests; verify batch and streaming input modes either match jq or reject an unsafe plan before reading input.
- [ ] 4.3 Extend parser and evaluator fuzz generation with balanced label scopes, breaks, recursive filters, and walk callbacks; verify the bounded fuzz corpus completes without panics, hangs, or uncontrolled allocation.
- [ ] 4.4 Run formatting, type checking, focused crate tests throughout the work, then run the full workspace test and preflight suites once; verify every required check passes on the final tree.

## 5. Performance Evidence

- [ ] 5.1 Add correctness-gated bounded `recurse`, structural `walk`, and early-break cases to the benchmark campaign; verify release tq and the manifest-recorded jq run identical inputs, queries, sample counts, and output sinks.
- [ ] 5.2 Run every benchmark outside the restricted sandbox with elevated child-process inspection permissions and wrap every measured jq and tq invocation with `/usr/bin/time -l`; verify each accepted sample includes `maximum resident set size` and reject any campaign missing the required permission or RSS data.
- [ ] 5.3 Record raw medians and tq-to-jq ratios in `benchmark-evidence.md`, checking the soft goals of at most 2.0 wall time and 1.5 peak RSS; verify every miss remains visible with profiling notes and follow-up work without failing correctness by itself.
