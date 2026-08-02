# MVP requirements traceability

This index is the release mapping for every `#### Scenario:` in the eight
`build-tq-mvp` capability specifications. A scenario maps to every evidence
route on its specification row. The checked-in traceability test enumerates the
scenario headings, requires the exact count below, verifies each evidence path,
and fails when a scenario is added without revising this review.

| Capability spec | Scenarios | Primary automated evidence | Release evidence or manual check |
| --- | ---: | --- | --- |
| `benchmark-corpus` | 16 | `crates/tq-test-support/tests/corpus_*.rs` | Frozen manifests under `baselines/2026-08-01/corpus/`; natural-file identity in performance reports |
| `cross-tool-compatibility` | 20 | `crates/tq-test-support/tests/compatibility_*.rs` | `compatibility/reviews/coverage-v1.json`; exact jq/tq divergence allowlist test |
| `jq-core-language` | 37 | `crates/tq-core/src/` unit/property tests and compatibility cases | Full compatibility report; unsupported/deferred capability matrix entries |
| `performance-benchmarks` | 26 | `crates/tq-test-support/tests/benchmark_*.rs` | Standard and natural-large reports under `baselines/2026-08-01/performance/`; performance review |
| `query-runtime` | 21 | `crates/tq-core/src/` bytecode, compiler, evaluator, plan, and VM tests | Parser/bytecode/VM fuzz targets; `--explain-json` CLI tests |
| `resource-governance` | 17 | `crates/tq-core/src/`, `crates/tq-cli/src/`, and `crates/tq-toon/src/` limit/cancellation tests | Large event-stream RSS report and fuzz release summary |
| `toon-stream-io` | 21 | `crates/tq-toon/tests/` plus decoder/writer unit and property tests | TOON rows in standard/large reports and framing compatibility cases |
| `tq-cli` | 38 | `crates/tq-cli/src/` argument, source, execution, and output tests | Full compatibility report and README command examples |

The mapping is intentionally specification-wide rather than a manually copied
list of 196 headings: the integration test reads the authoritative OpenSpec
files directly. Within each row, scenario behavior is exercised by named test
functions and case IDs derived from the scenario's requirement area. Campaign
facts that cannot be made hermetic—live recollection, local tool identity,
natural-large timing/RSS, signals, and release fuzz duration—are preserved in
the cited machine-readable release artifacts and reviewed manually.

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
