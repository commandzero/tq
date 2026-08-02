## Context

The MVP resolver only knows built-ins and lexical value bindings; bytecode has
bounded catch frames but no user-call ABI. Module loading also crosses a trust
boundary through filesystem search paths.

## Goals / Non-Goals

**Goals:** Match jq definition/call/scope behavior, use bounded managed call
frames, and make module resolution deterministic and diagnosable.

**Non-Goals:** Remote modules, package registries, native extensions, or implicit
network access.

## Decisions

- Resolve definitions into stable symbols before lowering; this preserves
  lexical shadowing and avoids runtime name lookup. Dynamic lookup was rejected
  because it weakens diagnostics and makes recursion limits harder to audit.
- Compile calls to explicit VM frames holding return PC, arguments, captures,
  and fork heights. Native Rust recursion is not used.
- Canonicalize modules within configured roots, cache by canonical path and
  digest, and reject cycles with the complete import chain.

## Risks / Trade-offs

- [Recursive generators retain continuations] → Bound call/fork frames and expose high-water marks.
- [Module paths can escape a root] → Canonicalize before admission and reject traversal outside configured roots.
- [jq module edge cases vary by invocation] → Capture the exact jq reference command in every baseline.
