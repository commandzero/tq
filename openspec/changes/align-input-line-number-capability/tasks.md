## 1. Runtime policy

- [ ] 1.1 Add evaluator tests for supplied, cursor-owned, and unavailable line context under denied platform policy; verify the targeted `tq-core` tests fail against the old gate and encode the delta-spec scenarios.
- [ ] 1.2 Split line-context lookup from ambient platform lookup so `input_line_number` returns available context without admission while `input_filename` remains gated; verify `cargo test -p tq-core` passes.
- [ ] 1.3 Add a CLI regression test for the issue #11 stdin command without `--allow-platform`, plus denial checks for filename and clock access; verify `cargo test -p tq-cli ambient` passes.

## 2. Compatibility contract and documentation

- [ ] 2.1 Add a jq-target compatibility case for default `input_line_number` access, remove its platform-policy adapter argument, and refresh `tests/compatibility/reviews/coverage-v1.json`; verify the compatibility reporting and manifest tests pass.
- [ ] 2.2 Update `--allow-platform` help, the README, and regex/date/platform compatibility docs to distinguish default line context from gated filename, clock, and timezone access; verify documentation searches contain no claim that all input metadata requires `--allow-platform`.
- [ ] 2.3 Add an Unreleased changelog entry linked to issue #11 and verify it describes the observable compatibility change rather than implementation details.

## 3. Final verification

- [ ] 3.1 Run formatting and the workspace test suite, then verify the issue reproduction prints `1` without `--allow-platform` and still prints `1` when the flag is present.
- [ ] 3.2 Run strict OpenSpec validation for `align-input-line-number-capability` and verify every artifact and delta requirement passes.
