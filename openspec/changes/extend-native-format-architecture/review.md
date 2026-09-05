# Implementation review and verification

Reviewed the implementation against documentation baseline
`ea81b21e1ea8d8e711dd53cb4ce79aff1ea037f2`, including new files.
The standards and spec reviews ran independently in parallel, without running
builds or tests during benchmark measurements. Both reviewers rechecked repairs.

## Standards

No outstanding concrete finding remains. Repairs preserve failed benchmark
warmups, record probe-script identity, validate fixture object-key order, and
align JSON-sequence benchmark output framing. The direct codec-consumer adapter
retains allocation-avoiding callbacks and classifies typed event errors
separately from the existing trait's text-hook failures.

The reviewers noted possible duplication in callback stop/error adapters.
This is a maintainability judgment, not a documented-standard violation.
The adapters currently serve distinct observation, selected-record, and direct
codec consumer contracts; no speculative unification was added.

## Spec

No outstanding concrete implementation finding remains. Regression tests cover
the repaired empty auto-detected proxy sequence and stale paths after consecutive
recoverable parse failures. Transcode decoding now enters through committed
native input rather than selecting codecs in the runner.

The performance goals are not fully met. Paired complete-Document JSON timing
increases are 15.74% to 21.91%, above the less-than-10% goal. No paired refactor
increase remains above 25%. New-format soft reference misses are explicit:
large JSON sequence input is 23.79x jq time and 3.44x jq RSS; CSV and TSV are
about 2.5x yq time with about 0.023x yq RSS. These are soft-objective misses,
not correctness failures or passing performance claims.

## Verification

- `cargo fmt --all --check`: passed.
- `cargo check --workspace --all-targets`: passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: exited
  successfully. The existing `clippy::assert_is_empty` allowance produces an
  unknown-lint warning on Homebrew rustc 1.98.0; no new code warning remains.
- `cargo test --workspace`: 446 passed across 51 suites, run once after repairs.
- `openspec validate --all --strict`: 19 passed.
- Final native compatibility campaign: 33 cases; all 23 jq/tq cases agree.
  Seven deliberate yq differences are covered by the versioned review.
- Native-input sanitizer fuzzing: 444,173 executions, no crash or timeout.
- Refactor matrix: all 30 correctness gates and seven RSS samples per row passed;
  every target miss received 21 alternating baseline/candidate pairs.
- Native reference matrix: all 12 rows passed correctness, with 30 small or
  seven large measured samples and two warmups per row.

All benchmark commands ran elevated outside the sandbox with `/usr/bin/time -l`.
The measured candidate SHA-256 is
`9ab2c14a6a458d39fde97f60cd2f72862fbada453be7a8b981fa57b6ad1e0227`.
Full commands, hashes, host metadata, individual samples, preserved failed
attempts, and the reviewed performance report are in the separate `tq-benchmarks`
checkout, `2026-09-05-native-format-architecture.md` and its `.work/` references.

Outstanding concrete findings: Standards 0; Spec 0. Performance target misses
remain recorded separately above.
