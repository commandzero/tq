# Manual behavior audit

The primary reviewer read all fourteen pinned source documents, including the
introduction. The original example ledger is necessary but not sufficient:
published tables do not exercise every documented arity, empty stream, error,
flag, or process effect. The following macOS count blocks are retained as
historical checkpoints. The earlier verified checkpoint passed the completion
gate with 602 exact matches and five identity-bound reviewed disparities. The
subsequent runtime-zero witness raises the corpus to 608 cases with 603 exact
matches and unchanged observations for those five disparities. The [later
executable checkpoint in the implementation review](implementation-review.md#current-acceptance-checkpoint-2026-09-10)
records the later results, and the final build must renew its identity-bound
approvals.
Presence of a witness alone is not a passing verdict.

Source paths below are relative to the pinned companion checkout. Execution uses
only committed fixtures. Source hashes are in
`tests/compatibility/reviews/jq-manual/reference-pin.toon`.

## Scope and evidence

Each section retains its original `manual-<section>.toon` review ledger.
Supplemental cases are linked one-to-one in
`tests/compatibility/reviews/jq-manual/prose-boundaries.toon`. That ledger records
the exact source line, query, input, and case ID without nested ID arrays.

| Source | Behavior beyond table examples | Acceptance tasks |
| --- | --- | --- |
| `jq-manual/index.md` | Identity and filter input/output model | 1.6, 10.3 |
| `jq-manual/invoking-jq.md` | Every long/short option; option ordering; no-read failures; raw/slurp/sequence/stream input; arguments; exact process effects; actual shell parsing | 2.1–2.4, 7.1–7.6, 9.3–9.6, 10.2 |
| `jq-manual/basic-filters.md` | Decimal identity and mutation; exact comparison; field/index/slice forms; absent and invalid values; optional iterator; comma/pipe scope | 3.1, 5.2, 10.3 |
| `jq-manual/types-and-values.md` | Literal preservation versus binary64 arithmetic; empty and multiple constructor results; variable keys; recursive descent | 3.2, 4.1, 5.2; supplemental `types.*` witnesses |
| `jq-manual/builtin-operators-and-functions.md` | Type-dependent operators; every overload and selector; empty/error/cardinality contracts; paths, recursion, formats, dates, environment, SQL operators, and builtin enumeration | 4.1–4.3, 5.1–5.8, 6.1–6.7, 7.5, 8.5 |
| `jq-manual/conditionals-and-comparisons.md` | Strict equality and ordering; optional else; truthiness; empty/multiple conditions; Boolean short circuit; alternatives; typed errors; lexical break; optional scope | 3.7–3.8; supplemental `condcomp.*` witnesses |
| `jq-manual/advanced-features.md` | Lexical scope; destructuring alternatives and retry; callback/value parameters; generator consumers; reduce/foreach cardinality; tail recursion | 3.2–3.8, 6.6 |
| `jq-manual/assignment.md` | Immutable roots; path identity; plain RHS branches; update first-result and deletion; all arithmetic updates; selected nested paths | 4.1–4.3 |
| `jq-manual/regular-expressions.md` | All arities and array forms; every flag; Unicode offsets; unmatched captures; empty/no-match streams; literal split overload; replacement generators | 8.1–8.4; supplemental `regex.*` witnesses |
| `jq-manual/math.md` | All listed functions and actual jq arities; domain boundaries; non-finite/signed-zero outcomes; matched-platform availability and bounded work | 5.1–5.8 |
| `jq-manual/io.md` | Shared input cursor; raw debug/stderr payloads; metadata at the current read position; null-input and EOF | 7.1, 7.5–7.6, 8.5 |
| `jq-manual/streaming.md` | Scalar/empty-container leaves; close events; truncate/fromstream/tostream; multiple roots; incremental processing | 7.4; supplemental `streaming.*` witnesses |
| `jq-manual/modules.md` | Search substitutions and termination; startup file; explicit/default paths; namespaces; JSON imports; constant metadata and dependency enumeration | 9.1–9.2 |
| `jq-manual/colors.md` | Seven ordered palette slots plus jq's key-style reuse; default/custom palette; terminal detection and environment overrides | 9.3 |

## Review findings

- Removed the sole query-rewriting adapter from
  `manual.types.fence-variable-key`. Both tools must execute the source query
  `"f o o" as $foo | "b a r" as $bar | {$foo, $bar:$foo}`. The strict gate now
  rejects adapters that change a manual query.
- Added seventeen executable prose-boundary cases. They cover constructor and
  condition cardinality, literal mutation, typed errors, short circuit, regex
  empty/Unicode/argument behavior, and streaming leaves/reconstruction.
- Added `arity-inventory.toon`: 220 source-linked callable signatures and
  one executable case asserting that none is missing from `builtins`. All 220
  were checked against pinned jq. Syntax operators are excluded; the two Math
  arity corrections remain explicit. This checks advertised availability, not
  the semantics of every overload.
- Added two Math boundary cases for `scalb` non-finite behavior and `abs`
  literal preservation in `math-boundaries.toon`. The expanded catalog
  reached 538 cases; platform-specific evidence remains required.
- Added eight path boundary cases for filtering/cardinality, dynamic and
  negative indices, duplicate deletion, early termination, and prior outputs
  before errors in `path-boundaries.toon`. All reference outcomes were
  verified with pinned jq. This brought the required catalog to 546 cases.
- Added a tiny-decimal `abs` witness, four measured math rounding witnesses,
  and a single-capture `scan` witness. The current strict selection contains
  608 cases, including the subsequent runtime-zero min/max witness and the
  original `date.strptime` case outside
  manual-named files; the original 303 cases remain protected.
- Added `completeness.toon` with 220 source-line rows that point each
  documented name/arity at a concrete catalog behavior case. The separate
  `clause_evidence` relationship rows normalize every empty/error/cardinality,
  regex flag/overload, math boundary, input-cursor, streaming, module, and
  CLI/color witness; `manual_completeness` validates that catalog references
  resolve. The arity matrix remains a separate availability guard, and neither
  ledger promotes a runtime error or a bounded call into a parity verdict.
- The conditionals prose describing piped `//` defaults reverses false/null
  versus non-false/non-null. The adjoining published examples and pinned jq
  agree: false/null inputs receive defaults. Preserve the imported source;
  use those executable examples as the behavioral authority.
- The builtin numeric introduction broadly describes operations on doubles,
  while Basic filters explicitly documents untruncated decimal comparisons.
  Preserve decimal comparison and test arithmetic conversion separately.
- Math's `frexp` and `modf` signatures and two published whitespace omissions
  remain recorded source corrections, not implementation disparities.
- JSON-sequence review found newline loss, recovery contamination, and numeric
  truncation errors. These are implementation failures requiring fixes, not
  acceptable library disparities.

## Acceptance and remaining release work

Task 1.6 passes primary review: the expanded campaign, source-linked semantic
inventory, and focused tests cover the audited value, empty-stream, typed-error,
flag, and process contracts. Four numerical witnesses and longest regex mode
remain separately counted, narrowly reviewed disparities. The arity availability
matrix is not used as a substitute for behavior evidence.

Regenerate reports and executable-bound approvals after the remaining code
changes. Verify actual native platform and shell contracts before release;
the macOS checkpoint does not establish Linux or Windows acceptance or
equivalence for every possible input outside the manual corpus.
