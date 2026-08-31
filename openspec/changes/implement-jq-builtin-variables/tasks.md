## 1. Establish regression coverage

- [ ] 1.1 Add core regression tests proving `$ENV` and `$__loc__` resolve without external declarations, while an unrelated undeclared variable still returns `TQ-RESOLVE-VARIABLE-001`; verify the resolver tests fail for the current implementation and pass after the feature is enabled.
- [ ] 1.2 Add core evaluation cases for an admitted environment snapshot, denied environment access, exact `$__loc__` object shape, multi-line locations, definition locations, lexical `$ENV` shadowing, and same-named external arguments; verify values, ordering, and error classification.
- [ ] 1.3 Add CLI regression cases for the issue reproduction with `--allow-environment`, inline `<top-level>` location output, filter-file locations, default-denied environment access, and an explicitly denying `CapabilityPolicy`; verify no input is consumed before policy rejection.

## 2. Resolve and materialize jq special variables

- [ ] 2.1 Extend name resolution with an explicit reserved built-in variable mapping for `ENV` and preserve outer built-in precedence over same-named external arguments while allowing lexical `$ENV` bindings to shadow it; verify HIR resolution and scope tests.
- [ ] 2.2 Record compact source-name and line-start metadata for the top-level query and every module source by `SourceId`; verify `$__loc__` resolves to the reference's one-based line in top-level and module definitions without retaining unbounded module source text.
- [ ] 2.3 Lower `$__loc__` to a source-derived ordered `{file, line}` value during resolution and preserve its original source span; verify references in interpolation, definitions, imported modules, and multi-line filters return definition locations rather than call-site locations.
- [ ] 2.4 Ensure reads of the reserved ambient-environment slot use the existing `env` policy/error helper in every evaluator route; verify `$ENV` returns the admitted snapshot and produces no generic unknown-variable or secret-bearing diagnostic when access is denied.

## 3. Harden the CLI/runtime boundary

- [ ] 3.1 Capture one environment snapshot at the existing variable-preparation boundary and share it between `$ENV` and `env` without reading the process environment when access is denied; verify repeated and multi-input evaluations observe identical values.
- [ ] 3.2 Reserve the internal `__tq_` namespace before merging external arguments with runtime markers, and keep `ENV`/`__loc__` external arguments from replacing special references; verify malicious internal arguments cannot manufacture ambient access and ordinary variables remain unchanged.
- [ ] 3.3 Use jq-compatible `<top-level>` as the inline CLI query source identity while retaining named filter-file and module identities; verify location output and existing compile-diagnostic classes through CLI tests.

## 4. Compatibility and documentation

- [ ] 4.1 Add secret-safe jq compatibility cases for `$ENV | type`, `$ENV | has("known-sentinel")`, and `$__loc__` under JSON input/output, and update generated baseline/traceability artifacts; verify the cases compare stable shapes or sentinel values without recording the host environment.
- [ ] 4.2 Document `$ENV` alongside the existing `env` capability and document `$__loc__` source identity/line semantics, default denial, policy behavior, and known Unicode environment handling; verify the documentation matches the accepted compatibility cases.

## 5. Final verification

- [ ] 5.1 Run `cargo fmt --check`, `cargo test -p tq-core`, and the relevant `tq-cli` unit/compatibility tests; verify all special-variable, policy, scope, module, and regression cases pass.
- [ ] 5.2 Run the repository preflight and the jq compatibility smoke campaign with telemetry disabled; verify no baseline regressions, input is not consumed on early policy failures, and diagnostics remain secret-safe.
- [ ] 5.3 Run `OPENSPEC_TELEMETRY=0 openspec validate --change "implement-jq-builtin-variables" --strict`; verify all proposal, delta-spec, design, and task artifacts are structurally valid and ready for implementation.
