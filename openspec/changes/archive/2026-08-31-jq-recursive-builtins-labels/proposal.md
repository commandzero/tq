## Why

Issue #6 remains incomplete after jq format strings landed. Queries that use jq's recursive built-ins or lexical early-exit control flow still fail with explicit deferred-feature errors, blocking common tree traversal and bounded-search programs.

## What Changes

- Add jq 1.7-compatible `recurse/0`, `recurse/1`, and `recurse/2` with depth-first ordering, generator cardinality, conditional descent, and pull-driven early termination.
- Add jq 1.7-compatible `walk/1` with post-order array and object traversal, stable encounter order, and callback generator semantics.
- Parse, resolve, and execute lexical `label` and `break`, including nearest-label shadowing, source-spanned unbound-label errors, and jq-compatible interaction with `try`/`catch`, functions, and reducers.
- Remove recursive built-ins and labels from the deferred compatibility surface while leaving non-finite result built-ins deferred.
- Add jq differential, deep-input, resource-limit, cancellation, fuzz, and benchmark coverage for the promoted features.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `jq-recursive-descent-interpolation`: Extend recursive traversal from `..` to the jq recursive built-in family and post-order `walk/1`.
- `jq-core-language`: Add lexical label and break control flow and narrow the explicitly deferred syntax list.

## Impact

The change affects the lexer, parser, AST, resolver, compiler, explicit-stack evaluator, built-in registry, capability analysis, compatibility fixtures, fuzz targets, and benchmark suite. It adds no external dependency and preserves the existing resource-governance model.
