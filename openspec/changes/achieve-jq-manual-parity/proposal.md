## Why

The manual audit has complete source coverage but not compatible execution: its 518 referenced cases contain 198 failures, 15 policy differences, and 2 reference discrepancies. tq should execute jq manual programs correctly rather than treating missing features or conflicting historical policies as acceptable outcomes.

## What Changes

- Prioritize close compatibility using safe Rust and existing Rust libraries over exact parity requiring native FFI or unsafe bridge code. Keep every manual behavior in scope. Record measured library or platform disparities for reconsideration after implementation, separately from exact matches and unresolved failures. Do not add a C-backed regex engine solely to reproduce jq internals.
- Make the pinned jq 1.8 manual corpus with jq 1.8.2 reference executables an executable release gate, including every table example, runnable source snippet, and concrete witness for a documented capability. Add missing platform and shell coverage rather than equating an audit entry with a passing test.
- Close the parser, variable scope, generator, assignment, built-in, math, regex, module, and input/stream gaps. Resolve all 198 recorded failures and the functional policies behind the 15 differences.
- Remove the remaining execution restrictions on user-defined filter composition. Documented operations must work inside definitions, around user calls, and as filter arguments, including constructors, slices, folds, math, regex, paths, and process effects. Add composition witnesses beyond isolated manual examples; preserve bounded execution and lexical scope.
- **BREAKING**: `-c` and `--compact-output` select compact JSON without requiring `-o json`. No output-format option still means TOON. JSON interoperability is selected with `-o json` or `-c`; raw output retains jq behavior.
- **BREAKING**: Replace incompatible numeric, regex, sequence, ambient CLI access, module search, color, and test-runner restrictions with jq behavior. Retain explicit library capability restrictions and configured resource limits, without counting their failures as parity successes.
- Separate reference corrections from execution verdicts. Preserve imported manual text and verify the corrected `frexp` and `modf` arities against the reference. Help/version/build output identifies tq honestly while implementing the documented command contracts.
- Preserve native TOON/YAML input extensions, LF-terminated TOON result output, explicit TOON sequence framing, and measured token-saving reports. Output representation is not a reason to weaken JSON-semantic correctness.
- Require fresh native acceptance on local macOS, ironhide Linux x86_64, and smokescreen Windows 11 Pro `x86_64-pc-windows-msvc`, using pinned jq 1.8.2 references. Windows invocation must run in native PowerShell; SSH into WSL, cross-compilation, and non-Windows PowerShell do not count. This requirement supersedes the earlier Windows deferral, including historical checkpoints and the not-yet-reconciled task wording. Native Windows compatibility capture is implemented with overlapped named pipes and JobObjects. Native CPU/RSS accounting is implemented through a retained exact-child handle using `GetProcessTimes` and `PeakWorkingSetSize`, with a surviving isolated worker. Validation remains incomplete: separate control executions vary in 15.625 ms CPU quanta, reaching 62.5 ms differences beyond the unchanged 20 ms check. No performance acceptance or issue #31 closure follows. The user disabled Smart App Control on development-only smokescreen; native launch was verified with state `0`, and no further security changes are needed. Strict-report differences remain differences, not exact matches.

## Capabilities

### New Capabilities

None. Extend the existing language, CLI, and compatibility contracts.

### Modified Capabilities

- `toon-stream-io`: LF-terminated default TOON result output, explicit TOON sequence framing, and completed-result preservation.
- `cross-tool-compatibility`: Frozen manual inventory, exact failure ownership, independent source-reference corrections, strict parity gate, and platform evidence.
- `tq-cli`: Compact JSON selection, ordered JSON input consumption, stream recovery, runtime I/O, and help/identity contracts.
- `extended-jq-cli-parity`: jq-compatible color controls subject to the archived `output-colors` main spec's tq presentation contract, sequence and test-runner options, and CLI-versus-library capability behavior.
- `jq-core-language`: Grammar/binding/cardinality repairs, full manual built-in surface, numeric fidelity, non-finite math, catch values, and assignment semantics.
- `jq-reduce-foreach`: Destructuring folds, optional foreach extraction, and generator state isolation.
- `jq-user-functions-modules`: Lexical filter capture and jq module/data import, lookup, startup, and metadata rules.
- `jq-regex-date-platform`: Complete manual regex flags and argument forms, replacement generators, date/platform and line metadata parity.

## Impact

Affected crates are `tq-core`, `tq-cli`, `tq-formats`, `tq-toon`, and `tq-test-support`; existing native-format, optimizer, resource, and compatibility tests remain required. Regex and math dependency choices need portability, license, resource, and performance review. jq remains a test reference, never a runtime subprocess or evaluator dependency.

The baseline is commit `dcabecc` and its checked-in manual source inventory and comparison report. Completion means every manual behavior is implemented and tested or has a reviewed, reproducible safe-library disparity. It does not mean exact jq parity. Missing functionality, uninvestigated failures, skipped cases, regressions, missing native acceptance evidence, and incomplete native Windows CPU/RSS validation remain incomplete work; the accounting backend is implemented, but issue #31 and performance acceptance remain open. The archived `output-colors` main spec governs tq's default palette, structural quote styling, palette overrides, and all-format coloring (including JSON Lines); these explicit presentation differences are not exact jq color matches or new safe-library disparity approvals. This planning revision completes no evidence and approves no new math differences.
