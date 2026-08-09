# MVP requirements traceability

This index summarizes the eight `build-tq-mvp` capability specifications. The
authoritative [scenario manifest](requirements-traceability.tsv) contains one
row for every `#### Scenario:` heading, including its containing requirement
and a typed evidence locator. Locators use `test:path#symbol`,
`report:path#symbol`, or `manual:path#heading`. The checked-in traceability test
compares the manifest with the OpenSpec files in both directions, rejects
duplicate or stale IDs and titles, verifies every evidence path, and requires
the named test/report symbol or manual-review heading to exist.

| Capability spec | Scenarios | Primary automated evidence | Release evidence or manual check |
| --- | ---: | --- | --- |
| `benchmark-corpus` | 16 | `crates/tq-test-support/tests/corpus_*.rs` | Corpus source descriptors under `tests/corpus/`; generated corpus data stays ignored |
| `cross-tool-compatibility` | 20 | `crates/tq-test-support/tests/compatibility_*.rs` | `tests/compatibility/reviews/coverage-v1.json`; exact jq/tq divergence allowlist test |
| `jq-core-language` | 37 | `crates/tq-core/src/` unit/property tests and compatibility cases | Full compatibility report; unsupported/deferred capability matrix entries |
| `performance-benchmarks` | 26 | `crates/tq-test-support/tests/benchmark_*.rs` | Reviewed date-named artifact under `benchmarks/`; local collection data stays ignored |
| `query-runtime` | 21 | `crates/tq-core/src/` bytecode, compiler, evaluator, plan, and VM tests | Parser/bytecode/VM fuzz targets; `--explain-json` CLI tests |
| `resource-governance` | 17 | `crates/tq-core/src/`, `crates/tq-cli/src/`, and `crates/tq-toon/src/` limit/cancellation tests | Reviewed benchmark artifact and bounded fuzz targets |
| `toon-stream-io` | 21 | `crates/tq-toon/tests/` plus decoder/writer unit and property tests | TOON rows in standard/large reports and framing compatibility cases |
| `tq-cli` | 38 | `crates/tq-cli/src/` argument, source, execution, and output tests | Full compatibility report and README command examples |

The TSV is intentionally explicit: all 196 scenarios have independent rows, so
adding, renaming, removing, or moving one scenario requires a reviewed evidence
update. A source file by itself is not accepted as evidence. Manual locators
remain visible when verification finds a requirement gap; they are not counted
as automated coverage. The linked release artifacts record facts that cannot
be hermetic: live source data, local tool identity, natural-large timing and
RSS, signals, and release fuzz time. Reviewers check the final date-named
artifact manually.

Release review checks the following without suppressing failures:

1. The full compatibility campaign contains no jq/tq mismatch outside the
   four reviewed framing/numeric-envelope cases.
2. Every capability has one of the six published dispositions and the
   `untested` count is zero.
3. Correctness gates run before benchmark timing, and JSON, YAML, and TOON
   corpus identities remain in each report. Structured correctness output is
   digested incrementally rather than accumulated as a complete result vector.
4. The natural-large explicit stream stays within its 128 MiB RSS envelope;
   blocking/document cases retain their observed outcome even when unfavorable.
5. Stable and Rust 1.87 workspace tests, strict OpenSpec validation, Clippy,
   rustdoc, and all six bounded fuzz targets pass.
