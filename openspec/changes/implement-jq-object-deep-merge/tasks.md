## 1. Lock the jq contract

- [ ] 1.1 Add versioned operator and update cases under `tests/compatibility/cases/`, then run them against jq and verify recursive content, right-biased conflicts, key order, `*=`, and the unsupported-type error contract.
- [ ] 1.2 Add focused evaluator regression tests in `crates/tq-core/src/eval.rs` and verify they cover nested object pairs, scalar/object conflicts in both directions, right-only keys, numeric multiplication, mixed-type errors, and object `*=`.

## 2. Implement overloaded multiplication

- [ ] 2.1 Add the ordered recursive object merge and `binary_multiply` helpers in `crates/tq-core/src/eval.rs`, route both binary `*` and update `*=` through them, and verify the targeted `tq-core` evaluator tests pass.

## 3. Measure the soft performance target

- [ ] 3.1 Add a correctness-gated benchmark workload for `.[0] * .[1]` over a representative nested JSON pair, then verify release builds of jq and tq produce the same ordered result before timing begins.
- [ ] 3.2 Run the workload with the benchmark harness's normal warmup and sampling policy on one recorded host, then report tq-to-jq ratios for median wall time and peak RSS. Record whether they meet the soft goals of at most `2.0` and `1.5`, respectively, without making a miss fail preflight.

## 4. Verify compatibility and repository health

- [ ] 4.1 Update and review the compatibility baseline for the new cases, then run `./scripts/run-campaign.sh compatibility smoke` and verify tq matches the accepted jq results, order, error class, and exit status.
- [ ] 4.2 Run `./scripts/preflight.sh` and verify formatting, compilation, lint checks, workspace tests, and OpenSpec validation all pass.
