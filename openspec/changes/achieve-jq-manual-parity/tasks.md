## 1. Freeze coverage and make failures enforceable

- [ ] 1.1 Pin the manual fingerprint, section inventory, and jq 1.8.1 build identity; verify a changed source or executable fingerprint fails reference validation.
- [ ] 1.2 Connect `gap-inventory.toon` to the executable catalog and review model; verify exactly 198 failures, 15 former differences, and 2 arity probes are accounted for, with no duplicate or missing IDs.
- [ ] 1.3 Separate source corrections from execution verdicts; verify both invalid arity probes, corrected `/0` witnesses, and both whitespace corrections retain original provenance and execute.
- [ ] 1.4 Add the strict manual campaign exit policy; test injected mismatch, skip, missing reference, timeout, normalization error, and deleted source mapping all cause nonzero exit.
- [ ] 1.5 Add semantic JSON, exact compact-byte, and independent TOON-output campaigns; verify numeric changes, changed result order, and changed explicit stderr payloads cannot normalize into a pass.
- [ ] 1.6 Audit every section's documented arities, flags, error/empty behavior, and runnable snippets beyond table examples; deliver a source-linked completeness inventory and executable cases for missing behaviors.
- [ ] 1.7 Remove developer-specific paths from case execution and record reproducible environment setup; verify the committed corpus runs without the companion repository while source auditing verifies its pinned checkout separately.

## 2. Output selection and CLI option resolution

- [ ] 2.1 Add failing process tests for default TOON, pretty JSON, `-c`, long compact output, bundled `-cr`, and both orders of explicit JSON/TOON combinations; verify they express the design matrix.
- [ ] 2.2 Normalize output selection before input consumption; make those tests pass without changing automatic input-format selection.
- [ ] 2.3 Verify compact/raw/join/sort/ASCII combinations and retained native-format options against jq or their native contract; include JSON Lines restrictions and invalid-option no-read tests.
- [ ] 2.4 Implement the remaining named/positional argument and argument-file contracts; verify `manual.invoking.args-named` and every documented option form with valid and invalid arguments.

## 3. Grammar, binding, and control flow

- [ ] 3.1 Repair comma indexing, quoted identifiers, namespace errors, and boolean/pipe precedence; verify the nonnumeric `basic` inventory entries and `manual.condcomp.not-array` against jq.
- [ ] 3.2 Implement nested array/object destructuring and variable object shorthand with lexical scope; verify manual pattern cases plus missing-field, shadowing, and invalid-type cases.
- [ ] 3.3 Implement destructuring alternatives and their backtracking bindings; verify all manual `?//` examples and errors from later alternative bodies against jq.
- [ ] 3.4 Repair captured filter environments and filter/value parameter behavior; verify nested `addvalue`, post-author lookup, and callback tests with multiple branches.
- [ ] 3.5 Repair reduce/foreach destructured bindings, optional extraction, and accumulator isolation; verify expanded manual examples and zero/multiple initializer/update results.
- [ ] 3.6 Repair pull-driven first/last/nth/skip/isempty and short-circuit consumers; verify empty generators and early termination do not evaluate later erroring branches.
- [ ] 3.7 Preserve typed error values and optional suppression scope; verify manual error examples, nested try/catch, and previously passing label/break cases.
- [ ] 3.8 Run all `advanced` inventory entries and parser/control-flow regressions across DOM and eligible optimized modes; require identical ordered results and process contracts.

## 4. Path selection and assignment

- [ ] 4.1 Preserve path identity through recursive traversal and selection; verify `path`, `paths`, `pick`, `del`, and `delpaths` against manual and nested-path cases.
- [ ] 4.2 Separate plain assignment RHS branching from update assignment first-result and empty-deletion behavior; verify range assignments and independent branches against jq.
- [ ] 4.3 Verify all `assignment` inventory entries, nested posts/comments, immutable roots, and unaffected key order; require the complete assignment campaign to pass.

## 5. Numeric model and math

- [ ] 5.1 Establish matched-platform math dependency feasibility using safe APIs under `unsafe_code = forbid`; deliver passing probes for arities, fused operations, signed zero, rounding, availability, supported targets, and license/resource review before integration.
- [ ] 5.2 Preserve decimal identity and observable literal representation without expanding exponents; verify numeric `basic` entries, `have_decnum`, and `tojson` precision/representation cases against pinned jq.
- [ ] 5.3 Add runtime non-finite values and jq-compatible numeric predicates, arithmetic, and JSON projection; verify valid native TOON projection and unchanged native parser rejection rules.
- [ ] 5.4 Implement trigonometric and hyperbolic math families, including inverse forms; verify every corresponding Math-section witness and domain boundary against matched-platform jq.
- [ ] 5.5 Implement exponent, logarithm, roots, powers, and scaling functions; verify all corresponding witnesses plus overflow, underflow, and large-exponent cases.
- [ ] 5.6 Implement rounding, sign, remainder, neighboring-value, min/max, and fused math functions; verify corresponding witnesses and signed-zero/rounding boundaries without numeric tolerance.
- [ ] 5.7 Implement error, gamma, Bessel, decomposition, and remaining platform math functions; verify the full Math inventory has no missing arity and both `frexp`/`modf` probes and valid witnesses execute.
- [ ] 5.8 Run every `math` inventory entry and existing numeric/resource tests; verify explicit tight limits fail boundedly and normal manual cases no longer use numeric-envelope exceptions.

## 6. Remaining built-in families

- [ ] 6.1 Repair collection containment and string/array index variants; verify all manual `contains`, `inside`, `indices`, `index`, and `rindex` cases plus type and empty cases.
- [ ] 6.2 Repair entry conversion, add/filter overloads, and all any/all forms; verify generator cardinality, key aliases, empty collections, and short-circuit behavior against jq.
- [ ] 6.3 Implement all documented `IN`, `INDEX`, and `JOIN` forms; verify every overload with duplicate keys, empty streams, and generated results.
- [ ] 6.4 Repair string division, trim variants, starts/ends predicates, joining, ASCII case, UTF-8 byte length, boolean conversion, and `@urid`; verify manual witnesses plus Unicode and invalid-input cases.
- [ ] 6.5 Repair combinations, transpose, and binary search; verify all manual cases and empty/ragged/insertion-position behavior against jq.
- [ ] 6.6 Repair while/repeat/until/walk and remaining recursive utilities using shared generator semantics; verify manual witnesses and bounded early termination.
- [ ] 6.7 Run all `builtins` inventory entries, including cross-workstream math/path/regex/I/O cases; verify no unknown documented arity or unresolved behavioral failure remains.

## 7. Input streams and process effects

- [ ] 7.1 Share the ordered cursor between top-level input, `input`, and `inputs`; verify null-input mode, EOF, multiple files/stdin, and changing source location against jq.
- [ ] 7.2 Accept ordinary JSON whitespace streams and match strict JSON error behavior; verify automatic detection separately so `-i json` cannot hide a valid-input regression.
- [ ] 7.3 Implement JSON sequence recovery and JSON/TOON output-mode separation; verify malformed records, RS/LF framing, diagnostics, and `--seq` with default output, `-o json`, and `-c`.
- [ ] 7.4 Repair stream/error events and reconstruction functions; verify all `streaming` entries plus stream reduce, empty containers, malformed input, and partial output.
- [ ] 7.5 Implement debug/stderr payloads and non-catchable halt termination; verify exact payload bytes, statuses, preserved earlier output, and abandoned pending branches.
- [ ] 7.6 Verify all `io` entries and invoking stream/halt cases; add a live producer test proving `--unbuffered` output arrives before input EOF.

## 8. Regex and date/platform behavior

- [ ] 8.1 Prove a safe jq-compatible regex dependency meets longest/scoped-flag/Unicode/empty-match semantics and enforceable work/stack/cancellation limits on release targets; deliver executable feasibility tests and dependency/license review before replacing the engine.
- [ ] 8.2 Implement all documented array/null pattern/flag forms and flag combinations; verify argument interpretation and inline, extended, longest, multiline, and empty-match behavior against jq.
- [ ] 8.3 Preserve capture scope and replacement generator results in sub/gsub; verify zero/multiple replacement choices, unmatched captures, Unicode offsets, and errors.
- [ ] 8.4 Run every `regex` inventory entry and existing regex resource tests; require zero former engine-policy differences and bounded hostile-pattern failures.
- [ ] 8.5 Verify date, environment, clock, filename, and line behavior under controlled locale/timezone/files; test CLI default access and embedded denial independently on matched platforms.

## 9. Modules, colors, and remaining CLI contracts

- [ ] 9.1 Implement jq-compatible module search, substitutions, startup file, repeated-component rejection, and search termination; verify controlled-home/default/explicit-path cases and embedded confinement/cycle tests.
- [ ] 9.2 Implement JSON data imports and dependency metadata; verify every `modules` inventory entry with namespace, ordering, parsing, and bounded file-read cases.
- [ ] 9.3 Implement JSON color palette, JQ_COLORS, NO_COLOR, terminal detection, and ordered force/disable flags; verify every `colors` entry and invoking color difference with byte and PTY tests.
- [ ] 9.4 Implement `--run-tests` through tq evaluation; verify passing/failing test files, compile-error expectations, comments, stdin, option interactions, diagnostics, and status against jq.
- [ ] 9.5 Verify help/version/build commands against explicit truthful tq contracts and complete option documentation; replace unconditional identity differences with executable assertions.
- [ ] 9.6 Run every remaining `invoking` inventory entry; require jq-compatible JSON/process behavior without tq-specific allow flags.

## 10. Cross-platform acceptance and release evidence

- [ ] 10.1 Wire the strict campaign to every advertised release OS/architecture with pinned matched jq builds; verify missing target/reference evidence blocks the compatibility claim.
- [ ] 10.2 Add actual POSIX, PowerShell, and cmd invocation tests where applicable, plus Windows binary/newline behavior; verify quoted filters, variables, paths, and outputs match the documented shell contracts.
- [ ] 10.3 Reconcile the complete source inventory against executable evidence; require all original 518 cases and newly required witnesses to pass with zero skips, timeouts, unexpected differences, or missing mappings.
- [ ] 10.4 Run workspace formatting, clippy, tests, native-format/resource regressions, and affected benchmark comparisons; preserve all 303 baseline matches and document any performance change before acceptance.
- [ ] 10.5 Regenerate compatibility reports and update help, formats documentation, numeric/security migration notes, and changelog; verify TOON remains the documented default, `-c` selects compact JSON, and token savings remain separate from verdicts.
- [ ] 10.6 Validate OpenSpec and affected documentation, verify implementation against all seven delta specs, then synchronize and archive only after completion; retain campaign results and the resolved gap inventory as review evidence.
