# MVP requirements traceability

This index maps every `#### Scenario:` in the eight `build-tq-mvp` capability
specifications to release evidence. Each specification row lists all evidence
routes for its scenarios. The checked-in traceability test counts the scenario
headings, checks the count below, checks each evidence path, and fails when a
new scenario has no revised review.

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

The mapping is specification-wide. It is not a copied list of 196 headings. The
integration test reads the OpenSpec files directly. Each row uses named test
functions and case IDs from its requirement area. The linked machine-readable
release artifacts record facts that cannot be hermetic: live source data, local
tool identity, natural-large timing and RSS, signals, and release fuzz time.
Reviewers check the final date-named artifact manually.

Release review checks the following without suppressing failures:

1. The full compatibility campaign contains no jq/tq mismatch outside the
   four reviewed framing/numeric-envelope cases.
2. Every capability has one of the six published dispositions and the
   `untested` count is zero.
3. Correctness gates run before benchmark timing, and JSON, YAML, and TOON
   corpus identities remain in each report.
4. The natural-large explicit stream stays within its 128 MiB RSS envelope;
   blocking/document cases retain their observed outcome even when unfavorable.
5. Stable and Rust 1.85 workspace tests, strict OpenSpec validation, Clippy,
   rustdoc, and all five bounded fuzz targets pass.
