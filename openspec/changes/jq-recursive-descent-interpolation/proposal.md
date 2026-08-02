## Why

Recursive traversal and string interpolation are common jq authoring tools, but
both require careful generator, escaping, error, and resource semantics beyond
the MVP grammar.

## What Changes

- Add jq-compatible recursive descent (`..`) in deterministic traversal order.
- Add string interpolation with nested filters, escaping, and multiplicity.
- Execute deep traversal on managed stacks with depth/work limits and cancellation.
- Add differential, hostile-depth, and source-diagnostic cases.

## Capabilities

### New Capabilities

- `jq-recursive-descent-interpolation`: Recursive value traversal and filter-driven string construction.

### Modified Capabilities

None.

## Impact

The lexer, parser, HIR, VM continuation model, string writer, resource controls,
fuzz targets, compatibility catalog, and traversal benchmarks are affected.
