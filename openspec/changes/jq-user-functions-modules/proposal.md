## Why

Real jq programs rely on reusable user-defined filters and module composition,
but the MVP deliberately rejects these forms. Adding them as one coherent
change prevents ad hoc scope or import semantics from becoming permanent.

## What Changes

- Add jq-compatible `def` declarations, parameters, recursion, and lexical capture.
- Add module search paths, `include`, `import`, module metadata, and deterministic loading.
- Add cycle detection, source-aware diagnostics, and module cache/resource limits.
- Extend compatibility fixtures for scope, arity, recursion, imports, and failures.

## Capabilities

### New Capabilities

- `jq-user-functions-modules`: User filter definitions, calls, recursion, and jq-compatible module loading.

### Modified Capabilities

None.

## Impact

The lexer/parser, resolver, HIR effects, bytecode call frames, diagnostics, CLI
module-path options, compatibility catalog, and resource governance are affected.
