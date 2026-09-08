## Context

See [proposal.md](proposal.md) for motivation and scope. The checked-in report at `dcabecc` contains 518 referenced cases. [gap-inventory.toon](gap-inventory.toon) freezes all 215 non-matching rows with their original verdict and an owning workstream. Ownership groups work; it does not assert that each failure's root cause is already diagnosed.

The current evaluator shares parsing, resolution, VM execution, and values across native formats. CLI argument handling currently treats compact output as formatting only and rejects it with default TOON. The regex implementation uses Rust's regex engine; the numeric policy excludes results and exponents required by the manual. Workspace policy forbids first-party unsafe code. These constraints make a cross-cutting design necessary.

## Goals / Non-Goals

The design uses one evaluator for native formats and jq parity. Fixes belong in language, value, I/O, and standard-library behavior, not in special cases for manual case IDs. Every recorded gap gets an execution result and every existing match remains a regression test.

This work does not replace TOON as the default, change TOON syntax, run jq as a production subprocess, embed libjq, or claim that finite tests prove equivalence for every possible jq program. The release claim is complete behavioral coverage of the pinned manual on verified release targets, with the explicit native-output and product-identity contracts.

## Decisions

### One strict campaign, separate source provenance

Extend the existing compatibility runner rather than building a second test framework. Pin the imported source fingerprint and jq 1.8.1 executable/build configuration. Resolve source entries through the normalized TOON review model to executable cases. The original 518 cases are a minimum; enumerate missing documented overloads, flags, shell forms, and process behavior before calling the inventory complete.

Keep corrected manual text separate from execution verdicts. Invalid `frexp/2` and `modf/2` probes still execute as negative contracts, while valid `/0` witnesses execute as positive contracts. The two whitespace corrections retain their original source evidence. Removing a case, accepting an expected difference, or marking a probe unavailable cannot improve the passing percentage.

Use semantic ordered JSON comparison for ordinary programs, and exact observations for compact output, raw output, framing, colors, explicit stderr payloads, and `tojson`. Ordinary compiler diagnostic wording may differ when its error class, status, source context, and stream placement satisfy the contract. Do not normalize program-emitted stderr, error values, or numeric values. Identity commands get real tq assertions, never automatic success or jq impersonation. Test strict JSON parser behavior with `-i json`, and test automatic detection independently with valid JSON streams.

Keep fixture execution self-contained. A companion-repository source audit may require the pinned manual checkout, but running committed cases must not depend on a developer-specific sibling path. Reports record reference identity, target, environment, denominator, and every failure, and the strict campaign exits nonzero on missing evidence.

### Normalize output selection once

Resolve output options before opening input, with default TOON distinct from explicitly selected TOON. `-c` is a format selector plus compact formatting, not merely a pretty-printer toggle. Resolve short bundles and long options through the same state. Use this matrix in argument and process tests:

| Invocation | Structured output |
| --- | --- |
| `tq '.'` | TOON Text Sequence |
| `tq -o json '.'` | Pretty JSON |
| `tq -c '.'` | Compact JSON |
| `tq -c -o json '.'` | Compact JSON |
| `tq -o toon -c '.'` | Usage error before input |
| `tq --seq -c '.'` | Compact JSON sequence |

Explicit TOON with `-c` is an order-independent conflict. This avoids silently overriding an explicit native-format request. Explicit JSON Lines remains compatible with compact output under its existing constraints. Raw/join flags retain jq semantics. Other flags do not implicitly select JSON. Input selection remains independent; `--seq` without JSON output selection does not change TOON's default framing.

### Repair shared language semantics before adding wrappers

Add failing behavior tests at parser/resolver, evaluator, and CLI boundaries as appropriate. Correct precedence, quoted identifiers, comma indexing, destructuring and `?//`, then lexical filter closures, generator continuations, fold state, and path provenance. Plain assignment and update assignment must use distinct RHS evaluation/cardinality rules. Do not implement generator functions by collecting all values when jq can terminate early.

Use the same corrected semantics in DOM, decoder-event, and optimized execution. Unsupported optimization must fall back before consuming input; it must not become unsupported language. Compare execution modes for affected cases and preserve existing resource accounting and immutable branch behavior.

### Separate numeric provenance from computed values

Retain the decimal literal representation needed by identity and `tojson`, together with lazily derived binary64 semantics. Avoid expanding large exponents. Computation discards literal provenance when jq does. Represent runtime non-finite results explicitly, then apply jq's JSON projection at serialization. Native TOON serializes the resulting JSON-compatible projection, not invented NaN/infinity spellings. This does not broaden native YAML/TOON parser acceptance.

Implement the complete math inventory against the matched platform's math behavior through audited dependencies exposing safe APIs. Keep first-party `unsafe_code = forbid`. Before choosing dependencies, prove target availability, correct arities, rounding, signed zero, non-finite behavior, fused operations, resource accounting, and license compatibility. Pure arithmetic substitutions such as multiply-plus-add for fused operations are unacceptable when they change observable results. A failed dependency feasibility check blocks that workstream; it does not permit relaxing parity or the unsafe-code policy.

### Use a bounded jq-compatible regex engine

The current engine's syntax and semantics cannot define the compatibility contract. Use the engine family required to reproduce the pinned jq behavior through an audited safe dependency. Before integration, require passing probes for longest-match behavior, scoped flags, Unicode offsets, empty matches, and replacement generators, plus enforceable engine work/stack limits and cancellation on all release targets. A timeout around an uncancellable worker is not sufficient resource enforcement.

Keep argument normalization and replacement filter evaluation in the standard library. This separates array/null flag forms and jq generator semantics from engine matching. Avoid partial rewrites of patterns into the existing engine when those rewrites change captures, backtracking, or offsets.

### Share input state and model process termination separately

Give top-level evaluation, `input`, and `inputs` one ordered source cursor. It owns filename/line state, decoding, EOF, stream events, and sequence recovery. Null-input mode suppresses the initial pull only. Preserve already emitted results when a later input fails. Test sequence recovery and stream errors at the byte boundary, not only as pre-decoded values.

Represent `halt`/`halt_error` as process termination distinct from a catchable language error. Route debug/stderr payloads independently from diagnostic rendering. Verify unbuffered output with a live producer/consumer test and colors with PTY tests. Complete CLI variable modes, `--run-tests`, and platform flags using this process contract.

### Match CLI ambient behavior while retaining library policy

The process CLI admits the manual's environment, clock, file metadata, module lookup, data import, and startup-file behavior by default. The embedded API retains explicit capability restrictions and configured resource limits. Module resolution tracks canonical source identity, metadata, dependencies, cycles, and bounded reads; jq search order and startup behavior apply to the CLI, not only explicit roots.

This is a deliberate change to earlier CLI confinement. Document the security consequence of executing startup/module code from jq-compatible locations. Tests use controlled home directories, environment variables, clocks, paths, and terminal state rather than a developer's ambient configuration. Never add tq-specific allow flags merely to make a reference case pass.

## Risks / Trade-offs

- Numeric and regex dependencies may fail portability or bounded-execution requirements. Run feasibility tests early and keep the release gate red until both constraints hold.
- CLI startup/module parity increases ambient access. Keep library restrictions, OS permissions, bounded reads, and explicit migration guidance; do not present ordinary CLI execution as a sandbox.
- Parser and generator repairs can regress the 303 matching cases or optimized execution. Run the full regression set after each workstream, not only its newly fixed cases.
- Platform math and shell behavior differ. Run matched jq builds on every advertised OS/architecture, with POSIX, PowerShell/cmd, binary mode, PTY, and unbuffered tests where applicable. Missing target evidence blocks an unqualified claim.
- Native detection and TOON output intentionally prevent byte-for-byte equivalence of every plain `jq` invocation. Document JSON selection and strict-parser adapters alongside the compatibility claim.

## Migration Plan

Land the strict campaign and frozen inventory first, with the known failing state visible. Close workstreams in dependency order, then remove obsolete expected-difference policies only as executable contracts pass. Preserve negative arity probes as provenance-backed tests.

Update help, format compatibility documentation, capability reports, and breaking-change notes with implementation. Keep token-saving reports about representation separate from correctness verdicts. Run workspace tests, formatting, linting, native-format/resource regressions, and the complete target campaign before claiming completion.

No production data migration is needed. Rollback restores the previous implementation and its truthful partial-compatibility report; it must not retain a 100% claim while restoring old behavior. Synchronize and archive this change only after implementation verification, following the repository's PR policy.
