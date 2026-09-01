## Context

The existing resolver admits only CLI-declared variables. Runtime environment access already has a capability-gated path: the CLI captures a Unicode environment snapshot under an internal key, and the `env` builtin returns that value or a policy error when the key is absent. Query source spans already retain source IDs and byte offsets, while module expansion assigns distinct source IDs but currently discards the module source metadata after parsing.

See `proposal.md` and the three delta specs for the externally visible contract. The design must make both variables available before input consumption without turning environment access into an implicit capability or losing source identity during module expansion.

## Goals / Non-Goals

**Goals:**

- Resolve `$ENV` as a reserved jq variable while preserving lexical `$ENV` shadowing and preventing CLI arguments from replacing the built-in binding.
- Lower `$__loc__` to a stable value using the source name and one-based line of the reference, including references inside definitions and expanded modules.
- Reuse the existing environment snapshot and policy failure path for both `$ENV` and `env`.
- Keep the values valid in every existing VM and execution-plan route, with no input-dependent state for `$__loc__` and no per-document environment recapture.
- Add bounded, secret-safe core, CLI, and differential coverage.

**Non-Goals:**

- Adding environment mutation, live environment refresh, or a new CLI option.
- Changing `env`, `input_filename`, or `input_line_number` semantics beyond sharing the established environment policy and snapshot behavior.
- Implementing additional jq special variables or changing jq's module language.
- Preserving source text for runtime diagnostics after compilation; only the metadata needed to materialize `$__loc__` is required.

## Decisions

### Resolve `$ENV` through the existing ambient value slot

Seed the resolver's outer variable scope with a built-in mapping from `ENV` to the existing internal ambient-environment slot. External names are added without replacing reserved built-in names, and lexical scopes remain searched first, so `1 as $ENV | $ENV` continues to use the lexical value.

At the VM boundary, a read of the ambient-environment slot uses the same helper as `env`: an admitted object is returned, while an absent object produces the environment-policy runtime error. This keeps resolution independent of the host capability policy and makes `$ENV` fail at evaluation rather than as the unknown-variable error in issue #10.

Using a normal resolved variable slot avoids a second environment representation and lets the existing document, event, and tree evaluators handle `$ENV` without separate dispatch logic. A distinct public API for ambient context was rejected because the current private slot already crosses the CLI/core boundary and the issue does not require exposing host state through a new library type.

The CLI will keep the `__tq_` namespace reserved at the external-variable boundary before inserting internal values. This prevents a user-supplied argument from manufacturing the ambient slot or other policy markers.

### Lower `$__loc__` during resolution

When resolution encounters the reserved `__loc__` reference, replace that expression with an object value whose keys are inserted as `file` then `line`. The file value comes from the source metadata for the expression's source ID, and the line value comes from the source position at the expression span's start. The resulting literal retains the original span for diagnostics and HIR/source mapping.

Resolution receives a compact source metadata registry containing the top-level source and every module source loaded during expansion. Module loading records each source's display name and line-start index before releasing the full source text. This preserves module locations without adding all module text to the compiled program. An unresolved source ID is an internal compilation error rather than a guessed filename or line.

The CLI names inline filters `<top-level>` to match jq's observable location value. Filter files use their displayed path, and module sources use their canonical module path. A compile-time literal was chosen over a runtime location opcode because the jq value describes the reference location, not the dynamic call site, and literal lowering automatically works in all existing evaluators and execution plans.

### Keep special-variable precedence explicit

`ENV` is installed as a built-in outer binding, so a same-named external argument cannot replace it but a lexical binding can shadow it. `__loc__` is handled as a reserved reference and is never sourced from an external argument or lexical environment. The resolver continues to return the existing unknown-variable diagnostic for every other undeclared name.

This precedence mirrors jq behavior observed for `--arg ENV` and `--arg __loc__`, while keeping ordinary external variables unchanged. Rejecting all user variables named `ENV` or `__loc__` was rejected because jq permits the ordinary `$ENV` lexical shadowing form and accepts the CLI shape even though the special reference retains precedence.

### Capture environment once and keep diagnostics secret-safe

The CLI captures the process environment at the existing variable-preparation boundary only when both `--allow-environment` and the capability policy permit it. `$ENV` and `env` receive the same immutable object for the invocation. If access is denied, the object is omitted; no OS environment lookup is performed as a fallback during VM evaluation.

Policy errors identify only the operation and policy class. Tests use a non-sensitive sentinel environment key or inspect only the resulting type, and compatibility reports must not serialize the full host environment.

### Test behavior at the language and CLI boundaries

Add core tests for resolution without declarations, exact object shape, multi-line and definition locations, lexical shadowing, denied ambient access, and stable values across repeated inputs. Add CLI tests for inline and filter-file source identities, `--allow-environment`, default and explicit policy denial, and same-named external arguments. Add compatibility cases that assert `$ENV | type` under allowed access and `$__loc__` under JSON output, with secret-safe normalization.

The test suite will also assert that the internal namespace cannot be supplied through external arguments and that no input is consumed before a policy rejection that is already checked at the CLI boundary.

## Risks / Trade-offs

- [A user-supplied internal variable could bypass ambient policy] → Reserve the `__tq_` namespace before merging CLI variables with internal runtime values, and cover both `env` and `$ENV` denial cases.
- [Environment values could leak through diagnostics or compatibility artifacts] → Reuse the existing policy error, avoid serializing the environment in reports, and use type-only or sentinel-key assertions.
- [Module source metadata could increase compilation memory] → Retain only source name and line-start offsets after parsing, bounded by the existing module count and byte limits.
- [Inlining `$__loc__` too early could lose a module's source identity] → Register source metadata by `SourceId` during module loading and test locations in top-level and module definitions.
- [Changing inline source identity could alter compile-diagnostic text] → Treat `<top-level>` as the jq-compatible canonical identity and add regression coverage for both location values and existing diagnostic classification.
- [Special-variable precedence could accidentally break lexical scope] → Resolve ordinary lexical scopes before the reserved outer `$ENV` binding and keep `__loc__` reserved only for its special reference form.

## Migration Plan

No data or configuration migration is required. After the change, queries that previously failed with an unknown-variable compile diagnostic will either produce the jq-compatible special value or receive the established runtime/policy failure for denied environment access. Existing `env` behavior remains the compatibility reference.

If the feature must be rolled back, remove the built-in resolver entries and compatibility cases together; callers that use `$ENV` or `$__loc__` will return to the prior unknown-variable behavior, while the existing `env` capability path remains unchanged.
