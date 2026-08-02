# Verification report: build-tq-mvp

## Summary

| Dimension | Status |
| --- | --- |
| Completeness | 198/198 tasks complete |
| Correctness | 8 capability specs and 196 scenarios mapped to automated evidence, reports, or release checks |
| Coherence | Rust typestate, TOON/JSON/YAML adapters, bounded plans, resource controls, and benchmark contracts agree with the design |

## Release evidence

- Rust 1.85 and stable `cargo test --workspace --offline`: 163 tests passed on each toolchain.
- `cargo fmt --all --check`, strict Clippy (`-D warnings`), `cargo doc --workspace --no-deps --offline`, and strict OpenSpec validation pass.
- `make fuzz` completed the parser, TOON decoder, bytecode validator, VM, and CLI argument targets at ten seconds each without crash artifacts; the release summary is `baselines/2026-08-01/fuzz/release-v1.json`.
- `compatibility/reviews/coverage-v1.json` records jq 1.8.2, yq 4.53.2, and tq. Its executable allowlist test permits only TOON sequence framing and the three numeric-envelope jq/tq differences; capability counts are 146 supported, 5 partial, 7 divergent, 2 unsupported, 14 deferred, and 0 untested.
- `tq-standard-mvp-v1.json` records 276 timed and 12 explicitly unsupported JSON/YAML/TOON standard rows. `tq-large-parse-discard-mvp-v1.json` records all six applicable jq/yq/tq rows over the natural 1 GB-class corpus; `large-event-stream-v1.json` and `large-event-stream-rss.json` preserve the bounded explicit-stream latency/RSS evidence.
- `tq-standard-parse-discard-stable-v1.json` and `tq-standard-parse-discard-regression-v1.json` prove the manifest-aware 50% wall / 20% RSS / five-sample tq-only regression gate.
- `docs/requirements-traceability.md` plus `requirements_traceability.rs` enforce the eight-spec scenario mapping. User and contributor guidance is in `README.md`, `docs/compatibility.md`, `docs/performance-baseline.md`, `benchmarks/README.md`, and `CONTRIBUTING.md`.
- Six apply-ready follow-up changes cover user functions/modules, reduce/foreach, recursive descent/interpolation, regex/date/platform built-ins, automatic stream planning, and extended jq CLI parity.

## Issues

No critical or warning issues remain for the MVP archive gate. The deliberately
deferred jq families and reviewed framing/numeric differences are published in
the capability matrix rather than treated as untracked gaps.

## Assessment

All checks passed. The change is ready for archive.
