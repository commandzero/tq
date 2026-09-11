## 1. Freeze coverage and make failures enforceable

Matching tasks in this plan target jq behavior subject to the reviewed safe-library disparity contract. Completion requires implemented and tested behavior or a specifically reviewed, reproducible library/platform disparity; unresolved implementation gaps stay unchecked. Exact-parity reports remain distinct from completion with documented disparities.

- [x] 1.8 Add reviewed disparity validation and separate exact/completion gate modes; reject stale or unknown approvals, preserve exact observations and all prior matches, and keep skips/timeouts/crashes as failures.
- [x] 1.1 Pin the manual fingerprint, section inventory, and jq 1.8.1 build identity; verify a changed source or executable fingerprint fails reference validation.
- [x] 1.2 Connect `gap-inventory.toon` to the executable catalog and review model; verify exactly 198 failures, 15 former differences, and 2 arity probes are accounted for, with no duplicate or missing IDs.
- [x] 1.3 Separate source corrections from execution verdicts; verify both invalid arity probes, corrected `/0` witnesses, and both whitespace corrections retain original provenance and execute.
- [x] 1.4 Add the strict manual campaign exit policy; test injected mismatch, skip, missing reference, timeout, normalization error, and deleted source mapping all cause nonzero exit.
- [x] 1.5 Add semantic JSON, exact compact-byte, and independent TOON-output campaigns; verify numeric changes, changed result order, and changed explicit stderr payloads cannot normalize into a pass.
- [x] 1.6 Audit every section's documented arities, flags, error/empty behavior, and runnable snippets beyond table examples; deliver a source-linked completeness inventory and executable cases for missing behaviors.
- [x] 1.7 Remove developer-specific paths from case execution and record reproducible environment setup; verify the committed corpus runs without the companion repository while source auditing verifies its pinned checkout separately.

## 2. Output selection and CLI option resolution

- [x] 2.1 Add failing process tests for default TOON, pretty JSON, `-c`, long compact output, bundled `-cr`, and both orders of explicit JSON/TOON combinations; verify they express the design matrix.
- [x] 2.2 Normalize output selection before input consumption; make those tests pass without changing automatic input-format selection.
- [x] 2.3 Verify compact/raw/join/sort/ASCII combinations and retained native-format options against jq or their native contract; include JSON Lines restrictions and invalid-option no-read tests.
- [x] 2.4 Implement the remaining named/positional argument and argument-file contracts; verify `manual.invoking.args-named` and every documented option form with valid and invalid arguments.
- [x] 2.5 Make LF-terminated TOON values the default for zero or more results; reserve RS framing for explicit `--seq` and strict single-document cardinality for explicit `--unframed`. Update output-byte, documentation, and regression contracts.

## 3. Grammar, binding, and control flow

- [x] 3.1 Repair comma indexing, quoted identifiers, namespace errors, and boolean/pipe precedence; verify the nonnumeric `basic` inventory entries and `manual.condcomp.not-array` against jq.
- [x] 3.2 Implement nested array/object destructuring and variable object shorthand with lexical scope; verify manual pattern cases plus missing-field, shadowing, and invalid-type cases.
- [x] 3.3 Implement destructuring alternatives and their backtracking bindings; verify all manual `?//` examples and errors from later alternative bodies against jq.
- [x] 3.4 Repair captured filter environments and filter/value parameter behavior; verify nested `addvalue`, post-author lookup, and callback tests with multiple branches.
- [x] 3.5 Repair reduce/foreach destructured bindings, optional extraction, and accumulator isolation; verify expanded manual examples and zero/multiple initializer/update results.
- [x] 3.6 Repair pull-driven first/last/nth/skip/isempty and short-circuit consumers; verify empty generators and early termination do not evaluate later erroring branches.
- [x] 3.7 Preserve typed error values and optional suppression scope; verify manual error examples, nested try/catch, and previously passing label/break cases.
- [x] 3.8 Run all `advanced` inventory entries and parser/control-flow regressions across DOM and eligible optimized modes; require identical ordered results and process contracts.
- [x] 3.9 Audit composition admission for every operation and documented built-in arity; add source-linked failing witnesses for operations inside definitions, around calls, and through filter/value parameters without altering the pinned original inventory.
- [x] 3.10 Implement bounded object-constructor and slice composition, including computed keys, field generators, lexical captures, empty branches, partial errors, and early termination.
- [x] 3.11 Implement bounded fold and path/update composition with user calls in generators, initializers, updates, extraction, selected paths, and RHS filters; preserve branch state, ordering, and cancellation.
- [x] 3.12 Integrate documented scalar and generator built-ins into user-filter execution without a restrictive arity whitelist or an unbounded collector fallback; verify math, string, collection, and generator family witnesses.
- [x] 3.13 Integrate callback-driven regex, recursive utilities, input, and process effects with composed user filters; verify lexical replacement scope, shared input consumption, debug/stderr ordering, non-catchable halt, and live early termination.
- [ ] 3.14 Run the full composition inventory through CLI and embedded interfaces, including recursion and tight resource limits; require no deferred execution gaps, then refresh native campaigns, approvals, and performance evidence before final acceptance.

## 4. Path selection and assignment

- [x] 4.1 Preserve path identity through recursive traversal and selection; verify `path`, `paths`, `pick`, `del`, and `delpaths` against manual and nested-path cases.
- [x] 4.2 Separate plain assignment RHS branching from update assignment first-result and empty-deletion behavior; verify range assignments and independent branches against jq.
- [x] 4.3 Verify all `assignment` inventory entries, nested posts/comments, immutable roots, and unaffected key order; require the complete assignment campaign to pass.

## 5. Numeric model and math

- [x] 5.1 Establish pure-Rust math dependency feasibility using safe APIs under `unsafe_code = forbid`; test arities, fused operations, signed zero, rounding, availability, and license/resource behavior. Record verified targets and measured disparities; exact C-library equality is not an integration prerequisite.
- [x] 5.2 Preserve decimal identity and observable literal representation without expanding exponents; verify numeric `basic` entries, `have_decnum`, and `tojson` precision/representation cases against pinned jq.
- [x] 5.3 Add runtime non-finite values and jq-compatible numeric predicates, arithmetic, and JSON projection; verify valid native TOON projection and unchanged native parser rejection rules.
- [x] 5.4 Implement trigonometric and hyperbolic math families, including inverse forms; verify every corresponding Math-section witness and domain boundary against matched-platform jq.
- [x] 5.5 Implement exponent, logarithm, roots, powers, and scaling functions; verify all corresponding witnesses plus overflow, underflow, and large-exponent cases.
- [x] 5.6 Implement rounding, sign, remainder, neighboring-value, min/max, and fused math functions; verify witnesses and signed-zero/rounding boundaries, recording measured disparities with function-specific bounds separately from exact matches.
- [x] 5.7 Implement error, gamma, Bessel, decomposition, and remaining platform math functions; verify the full Math inventory has no missing arity and both `frexp`/`modf` probes and valid witnesses execute.
- [x] 5.8 Run every `math` inventory entry and existing numeric/resource tests; verify explicit tight limits fail boundedly and normal manual cases no longer use numeric-envelope exceptions.

## 6. Remaining built-in families

- [x] 6.1 Repair collection containment and string/array index variants; verify all manual `contains`, `inside`, `indices`, `index`, and `rindex` cases plus type and empty cases.
- [x] 6.2 Repair entry conversion, add/filter overloads, and all any/all forms; verify generator cardinality, key aliases, empty collections, and short-circuit behavior against jq.
- [x] 6.3 Implement all documented `IN`, `INDEX`, and `JOIN` forms; verify every overload with duplicate keys, empty streams, and generated results.
- [x] 6.4 Repair string division, trim variants, starts/ends predicates, joining, ASCII case, UTF-8 byte length, boolean conversion, and `@urid`; verify manual witnesses plus Unicode and invalid-input cases.
- [x] 6.5 Repair combinations, transpose, and binary search; verify all manual cases and empty/ragged/insertion-position behavior against jq.
- [x] 6.6 Repair while/repeat/until/walk and remaining recursive utilities using shared generator semantics; verify manual witnesses and bounded early termination.
- [x] 6.7 Run all `builtins` inventory entries, including cross-workstream math/path/regex/I/O cases; verify no unknown documented arity or unresolved behavioral failure remains.

## 7. Input streams and process effects

- [x] 7.1 Share the ordered cursor between top-level input, `input`, and `inputs`; verify null-input mode, EOF, multiple files/stdin, and changing source location against jq.
- [x] 7.2 Accept ordinary JSON whitespace streams and match strict JSON error behavior using one shared safe-Rust incremental parser, including jq non-finite input and `fromjson`; preserve numeric identity, event streaming, source positions, cancellation, and resource limits across JSON routes without broadening native-format admission. Verify automatic detection separately so `-i json` cannot hide a valid-input regression.
- [x] 7.3 Implement JSON sequence recovery and JSON/TOON output-mode separation; verify malformed records, RS/LF framing, diagnostics, and `--seq` with default output, `-o json`, and `-c`.
- [x] 7.4 Repair stream/error events and reconstruction functions; verify all `streaming` entries plus stream reduce, empty containers, malformed input, and partial output.
- [x] 7.5 Implement debug/stderr payloads and non-catchable halt termination; verify exact payload bytes, statuses, preserved earlier output, and abandoned pending branches.
- [x] 7.6 Verify all `io` entries and invoking stream/halt cases; add a live producer test proving `--unbuffered` output arrives before input EOF.

## 8. Regex and date/platform behavior

- [x] 8.1 Select an existing pure-Rust regex dependency through safe APIs; test scoped flags, Unicode, empty matches, actual bounded-work failure, and cancellation between bounded operations. Investigate longest matching and document demonstrated limitations and verified targets without unsafe bridges or native FFI engines.
- [x] 8.2 Implement all documented array/null pattern/flag forms and flag combinations; verify argument interpretation and inline, extended, longest, multiline, and empty-match behavior against jq.
- [x] 8.3 Preserve capture scope and replacement generator results in sub/gsub; verify zero/multiple replacement choices, unmatched captures, Unicode offsets, and errors.
- [x] 8.4 Run every `regex` inventory entry and existing regex resource tests; replace historical policy labels with exact results or specifically reviewed safe-library disparities and verify bounded hostile-pattern failures.
- [x] 8.5 Verify date, environment, clock, filename, and line behavior under controlled locale/timezone/files; test CLI default access and embedded denial independently on matched macOS and Linux platforms. Retain native Windows verification as explicitly deferred and unverified until a runner is available.

## 9. Modules, colors, and remaining CLI contracts

- [x] 9.1 Implement jq-compatible module search, substitutions, startup file, repeated-component rejection, and search termination; verify controlled-home/default/explicit-path cases and embedded confinement/cycle tests.
- [x] 9.2 Implement JSON data imports and dependency metadata; verify every `modules` inventory entry with namespace, ordering, parsing, and bounded file-read cases.
- [x] 9.3 Implement JSON color palette, JQ_COLORS, NO_COLOR, terminal detection, and ordered force/disable flags; verify every `colors` entry and invoking color difference with byte and PTY tests.
- [x] 9.4 Implement `--run-tests` through tq evaluation; verify passing/failing test files, compile-error expectations, comments, stdin, option interactions, diagnostics, and status against jq.
- [x] 9.5 Verify help/version/build commands against explicit truthful tq contracts and complete option documentation; replace unconditional identity differences with executable assertions.
- [x] 9.6 Run every remaining `invoking` inventory entry; require jq-compatible JSON/process behavior without tq-specific allow flags.

## 10. Cross-platform acceptance and release evidence

- [ ] 10.1 Wire the strict campaign to advertised release OS/architectures with pinned matched jq builds; verify macOS and Linux execution and that missing target/reference evidence blocks a compatibility claim. Record native Windows verification as explicitly deferred and unverified without accepting or skipping its cases into passing totals.
- [x] 10.2 Verify actual POSIX invocation on macOS and Linux, including quoted filters, variables, paths, and outputs. Retain PowerShell/cmd and Windows binary/newline tests, document native Windows execution as deferred until a runner is available, and do not count non-Windows shell execution as Windows evidence.
- [ ] 10.3 Reconcile the complete source inventory against executable evidence; require all original 518 cases and new witnesses to be exact matches or reviewed safe-library disparities, with zero skips, timeouts, unresolved failures, regressions, or missing mappings. Publish exact and disparity counts separately without a 100% exact-parity claim.
- [ ] 10.4 Run workspace formatting, clippy, tests, native-format/resource regressions, and affected benchmark comparisons; preserve all 303 baseline matches and document any performance change before acceptance.
- [ ] 10.5 Regenerate compatibility reports and update help, formats documentation, numeric/security migration notes, and changelog; verify TOON remains the documented default, `-c` selects compact JSON, and token savings remain separate from verdicts.
- [ ] 10.6 Validate OpenSpec and affected documentation, verify implementation against all eight delta specs, then synchronize and archive only after completion; retain campaign results and the resolved gap inventory as review evidence.
- [x] 10.7 Complete `docs/jq-compatibility-disparities.md` with observed jq/tq witnesses, target/dependency identity, practical impact, accepted bounds or restrictions, and regression evidence; retain explicit reconsideration criteria for after the spec is fully implemented.
