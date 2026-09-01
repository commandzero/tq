## Why

Issue [#10](https://github.com/commandzero/tq/issues/10) exposes a jq compatibility gap: tq already has the capability-gated `env` builtin, but `$ENV` fails during resolution, and `$__loc__` is not recognized at all. Queries that use these standard jq variables therefore cannot compile even when the caller explicitly permits environment access.

This is a focused extension of the existing variable, ambient-capability, and source-span machinery, allowing jq-oriented scripts to inspect their startup environment and report query source locations without weakening tq's capability policy.

## What Changes

- Recognize `$ENV` and `$__loc__` as jq built-in variable references during resolution rather than treating them as undeclared external variables.
- Make `$ENV` evaluate to the same startup environment snapshot as `env`, with string keys and values, only when environment access is admitted by `--allow-environment` and the library capability policy.
- Make `$__loc__` evaluate to an ordered object containing `file` and one-based numeric `line` for the source location where the reference occurs. Inline CLI filters use jq's `<top-level>` identity; named filters and modules use their source names.
- Preserve jq-compatible scope behavior: lexical bindings may shadow `$ENV`, while the special variables are not overridden by same-named CLI external arguments; `$__loc__` remains a reserved special reference.
- Exercise both variables through core, CLI, capability-policy, multi-line, filter-file, and compatibility tests, and document the supported ambient behavior and source-location contract.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `jq-core-language`: Add jq built-in variable resolution, values, scope, and source-location semantics.
- `query-runtime`: Carry special-variable values and source identity through compilation and every supported execution plan.
- `tq-cli`: Extend the existing environment capability and external-variable contract to cover `$ENV` without exposing ambient data by default.

## Impact

- Core resolver, source metadata, bytecode lowering, and VM evaluation in `crates/tq-core`.
- CLI filter loading, environment snapshot construction, reserved-variable handling, and capability-policy behavior in `crates/tq-cli`.
- Compatibility fixtures/reports, unit tests, and documentation for jq variables and ambient access.
- No new dependency or intentional breaking CLI option change.
