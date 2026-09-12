## Context

See [proposal.md](proposal.md) for motivation and scope. The checked-in report at `dcabecc` contains 518 referenced cases. [gap-inventory.toon](gap-inventory.toon) freezes all 215 non-matching rows with their original verdict and an owning workstream. Ownership groups work; it does not assert that each failure's root cause is already diagnosed.

The current evaluator shares parsing, resolution, VM execution, and values across native formats. CLI argument handling currently treats compact output as formatting only and rejects it with default TOON. The regex implementation uses Rust's regex engine; the numeric policy excludes results and exponents required by the manual. Workspace policy forbids first-party unsafe code. These constraints make a cross-cutting design necessary.

## Goals / Non-Goals

The design uses one evaluator for native formats and jq parity. Fixes belong in language, value, I/O, and standard-library behavior, not in special cases for manual case IDs. Every recorded gap gets an execution result and every existing match remains a regression test.

This work does not replace TOON as the default, change TOON syntax, run jq as a production subprocess, embed libjq, or claim that finite tests prove equivalence for every possible jq program. The release claim is complete tested coverage of the pinned manual on verified targets, with explicit native-output, product-identity, and reviewed safe-library disparities. It is not an exact-parity claim.

## Decisions

### Safe Rust compatibility takes precedence over exact parity

Use safe Rust and existing Rust libraries. Keep `unsafe_code = forbid`; do not introduce direct FFI, a native engine dependency, or a maintained unsafe bridge to chase exact jq behavior. Existing platform implementation details behind the Rust standard library are not a request to replace the standard library.

The matching requirements below describe the target behavior, subject to narrowly reviewed safe-library disparities. Implement missing functions, argument forms, and language behavior before considering an exception. A dependency limitation must be demonstrated, not inferred from missing API names or untested concerns.

Maintain `docs/jq-compatibility-disparities.md` with reproducible query/input, jq and tq observations, reference and target identity, cause, practical impact, safety or implementation tradeoff, regression evidence, and a future reconsideration condition. Reconsider these disparities after the spec is fully implemented. Exact matches, documented disparities, and unresolved failures remain separate counts. Never turn a disparity into an exact match or hide it with blanket normalization. Numeric differences require a function-specific justified error bound and boundary tests; changed types, result cardinality, domain handling, or unknown functions are not rounding differences. Resource tests verify the chosen engine's real limits without claiming an unmeasured cancellation deadline.

### One strict campaign, separate source provenance

Extend the existing compatibility runner rather than building a second test framework. Pin the imported source fingerprint and jq 1.8.1 executable/build configuration. Resolve source entries through the normalized TOON review model to executable cases. The original 518 cases are a minimum; enumerate missing documented overloads, flags, shell forms, and process behavior before calling the inventory complete.

Keep corrected manual text separate from execution verdicts. Invalid `frexp/2` and `modf/2` probes still execute as negative contracts, while valid `/0` witnesses execute as positive contracts. The two whitespace corrections retain their original source evidence. Removing a case, accepting an expected difference, or marking a probe unavailable cannot improve the passing percentage.

Use semantic ordered JSON comparison for ordinary programs, and exact observations for compact output, raw output, framing, colors, explicit stderr payloads, and `tojson`. Ordinary compiler diagnostic wording may differ when its error class, status, source context, and stream placement satisfy the contract. Do not normalize program-emitted stderr, error values, or numeric values. Identity commands get real tq assertions, never automatic success or jq impersonation. Test strict JSON parser behavior with `-i json`, and test automatic detection independently with valid JSON streams.

Keep fixture execution self-contained. A companion-repository source audit may require the pinned manual checkout, but running committed cases must not depend on a developer-specific sibling path. Reports record reference identity, target, environment, denominator, and every failure, and the strict campaign exits nonzero on missing evidence.

### Normalize output selection once

Resolve output options before opening input, with default TOON distinct from explicitly selected TOON. `-c` is a format selector plus compact formatting, not merely a pretty-printer toggle. Resolve short bundles and long options through the same state. Use this matrix in argument and process tests:

| Invocation | Structured output |
| --- | --- |
| `tq '.'` | Standalone TOON document |
| `tq -o json '.'` | Pretty JSON |
| `tq -c '.'` | Compact JSON |
| `tq -c -o json '.'` | Compact JSON |
| `tq -o toon -c '.'` | Usage error before input |
| `tq --seq -c '.'` | Compact JSON sequence |

Explicit TOON with `-c` is an order-independent conflict. This avoids silently overriding an explicit native-format request. Explicit JSON Lines remains compatible with compact output under its existing constraints. Raw/join flags retain jq semantics. Other flags do not implicitly select JSON. Input selection remains independent. Default TOON emits zero or more canonical values, each followed by LF without RS. Filter keywords and result counts never select framing. `--seq` explicitly selects JSON Text Sequence input and RS-framed TOON sequence output by default; `--seq -o json` matches jq JSON sequence output, and `-o toon-seq` selects only TOON output framing; `--unframed` alone requires exactly one standalone document. Default output preserves completed results when evaluation later fails. The comparison campaign uses independently captured JSON results to resolve TOON value boundaries, verifies each decoded value, and requires every stdout byte to be consumed.

### Repair shared language semantics before adding wrappers

Add failing behavior tests at parser/resolver, evaluator, and CLI boundaries as appropriate. Correct precedence, quoted identifiers, comma indexing, destructuring and `?//`, then lexical filter closures, generator continuations, fold state, and path provenance. Plain assignment and update assignment must use distinct RHS evaluation/cardinality rules. Do not implement generator functions by collecting all values when jq can terminate early.

Use the same corrected semantics in DOM, decoder-event, and optimized execution. Unsupported optimization must fall back before consuming input; it must not become unsupported language. Compare execution modes for affected cases and preserve existing resource accounting and immutable branch behavior.

### Complete bounded user-filter composition

The integrated evaluator still routes user calls through a restricted managed
operation set. As a result, operations accepted at top level can fail with
`TQ-CAP-USER-FUNCTIONS` when placed inside a definition or around a user call.
This contradicts the shared-language design; it is not a library disparity.

Extend the existing bounded, resumable execution machinery so that user calls
and built-in filter arguments share lexical environments, input cursors,
effects, and resource accounting with the surrounding program. Use explicit
continuations for operations that suspend or branch. Reuse scalar operation
implementations where they cannot invoke user filters, while routing callback
evaluation back through the same managed machinery. Keep the public evaluator
interface unchanged and avoid a parallel language implementation.

Audit every operation and documented built-in arity for composition admission.
Implement in dependency order: constructors and slices; folds and path updates;
scalar/generator built-ins; callback-driven and effectful built-ins. Object
fields, computed keys, fold updates, replacement filters, and assignment RHS
must retain their reference evaluation order, empty branches, errors, and
independent state. Consumers such as `first` must abandon pending work once
their result is determined. Recursion must not move onto the native stack.

Do not simply remove the admission guard or send unsupported calls through an
eager collector. Those alternatives can accept compilation while losing
closures, early termination, or bounded work. An optimization may fall back
before input consumption only to an execution path with the same language and
resource contracts. Preserve existing optimized paths where they remain valid.

Add source-linked composition witnesses without modifying the original manual
snapshot or its protected case IDs. Test each affected operation both inside a
definition and around a call, plus filter/value parameters, nested captures,
recursion, empty/multiple results, errors, cancellation, and tight limits.
Regenerate both native campaigns and executable-bound approvals after the
implementation stabilizes. Earlier matching reports remain checkpoint evidence.

### Separate numeric provenance from computed values

Retain the decimal literal representation needed by identity and `tojson`, together with lazily derived binary64 semantics. Avoid expanding large exponents. Computation discards literal provenance when jq does. Represent runtime non-finite results explicitly, then apply jq's JSON projection at serialization. Native TOON serializes the resulting JSON-compatible projection, not invented NaN/infinity spellings. This does not broaden native YAML/TOON parser acceptance.

Implement the complete math inventory using safe standard-library APIs and pure-Rust math libraries. Test arities, signed zero, non-finite behavior, fused operations, resource accounting, and licenses. Compare against jq on verified targets and document measured rounding or platform disparities with specific bounds. Prefer a library's fused operation over multiply-plus-add. Lack of bit-for-bit platform equality is not an integration blocker and does not authorize unsafe code or FFI.

### Use a bounded jq-compatible regex engine

Use an existing Rust regex library through safe APIs, with bounded backtracking and input/pattern limits. Test scoped flags, Unicode offsets, empty matches, replacement generators, and actual limit exhaustion. Investigate longest-match support, but document a demonstrated library limitation rather than adding Oniguruma FFI or unsafe callbacks. Check cancellation around bounded engine operations and document that it is not an immediate mid-match callback. A timeout around an uncancellable worker is not resource enforcement. Unverified targets remain unverified, not a reason to block unrelated host implementation.

Keep argument normalization and replacement filter evaluation in the standard library. This separates array/null flag forms and jq generator semantics from engine matching. Avoid partial rewrites of patterns into the existing engine when those rewrites change captures, backtracking, or offsets.

### Share input state and model process termination separately

Use one shared safe-Rust incremental JSON parser in `tq-core` for jq-compatible
input and `fromjson`. The user approved this parser expansion after final review
found that `serde_json` rejects non-finite input before custom visitors can see
it. A value reader and an event reader must share the same grammar and lexer;
streaming adapters must not materialize whole documents to obtain events.
Document, line, sequence, structural-event, selected/parallel, argument-file,
and module-data routes must agree on JSON admission.

Preserve runtime numeric identity for NaN and infinities until output projection.
Preserve exact finite numeric provenance, key ordering, duplicate-key event
delivery, source positions, partial output, and early cancellation. Verify token
boundaries and accepted numeric spellings against the pinned jq executable,
including rejection of invalid suffixes. Enforce depth, token, and input/work
limits while parsing, before unbounded allocation. Do not rewrite input with
sentinel strings, use an eager fallback after consuming input, or introduce FFI
or first-party unsafe code. Native TOON, YAML, and JSON5 admission remains
unchanged; JSON compatibility does not authorize broader native-format syntax.

Give top-level evaluation, `input`, and `inputs` one ordered source cursor. It owns filename/line state, decoding, EOF, stream events, and sequence recovery. Null-input mode suppresses the initial pull only. Preserve already emitted results when a later input fails. Test sequence recovery and stream errors at the byte boundary, not only as pre-decoded values.

Represent `halt`/`halt_error` as process termination distinct from a catchable language error. Route debug/stderr payloads independently from diagnostic rendering. Verify unbuffered output with a live producer/consumer test and colors with PTY tests. Complete CLI variable modes, `--run-tests`, and platform flags using this process contract.

### Match CLI ambient behavior while retaining library policy

The process CLI admits the manual's environment, clock, file metadata, module lookup, data import, and startup-file behavior by default. The embedded API retains explicit capability restrictions and configured resource limits. Module resolution tracks canonical source identity, metadata, dependencies, cycles, and bounded reads; jq search order and startup behavior apply to the CLI, not only explicit roots.

This is a deliberate change to earlier CLI confinement. Document the security consequence of executing startup/module code from jq-compatible locations. Tests use controlled home directories, environment variables, clocks, paths, and terminal state rather than a developer's ambient configuration. Never add tq-specific allow flags merely to make a reference case pass.

## Native Windows verification deferral

Native Windows execution is deferred because no runner is available. Retain the
PowerShell/cmd and binary/newline tests, but do not count non-Windows PowerShell
execution or compilation as Windows evidence. macOS and Linux campaigns remain
required for this change. Record Windows as unverified in release evidence and
documentation; do not approve Windows disparities or advertise verified Windows
manual compatibility without a native pinned-jq campaign. Reopen verification
when a native runner is available. This explicit deferral does not waive missing
cases, failed tests on verified hosts, or any other release target.

## Risks / Trade-offs

- Numeric and regex dependencies may have portability or semantic limitations. Keep resource failures bounded, record verified targets, and distinguish measured safe-library disparities from unresolved implementation gaps. Do not block unrelated implementation on hypothetical exactness concerns.
- CLI startup/module parity increases ambient access. Keep library restrictions, OS permissions, bounded reads, and explicit migration guidance; do not present ordinary CLI execution as a sandbox.
- Parser and generator repairs can regress the 303 matching cases or optimized execution. Run the full regression set after each workstream, not only its newly fixed cases.
- Platform math and shell behavior differ. Run matched jq builds on every advertised OS/architecture, with POSIX, PowerShell/cmd, binary mode, PTY, and unbuffered tests where applicable. Missing target evidence blocks an unqualified claim.
- Native detection and TOON output intentionally prevent byte-for-byte equivalence of every plain `jq` invocation. Document JSON selection and strict-parser adapters alongside the compatibility claim.

## Migration Plan

Land the strict campaign and frozen inventory first, with the known failing state visible. Close workstreams in dependency order, then remove obsolete expected-difference policies only as executable contracts pass. Preserve negative arity probes as provenance-backed tests.

Update help, format compatibility documentation, capability reports, and breaking-change notes with implementation. Keep token-saving reports about representation separate from correctness verdicts. Run workspace tests, formatting, linting, native-format/resource regressions, and the complete target campaign before claiming completion.

No production data migration is needed. Rollback restores the previous implementation and its truthful partial-compatibility report; it must not retain a 100% claim while restoring old behavior. Synchronize and archive this change only after implementation verification, following the repository's PR policy.
