# Verification checkpoint

## Assessment

The change is not ready for acceptance or archive. Twelve of 24 tasks are
complete. The explicit isolated-worker prototype is not the default collector.
Passing local tests do not replace native calibration or the paired matrices.

## Completeness: critical outstanding tasks

Each item below remains an acceptance blocker in `tasks.md`.

- 1.3: Finish native API unit/scope proof on both hosts for the final collector.
- 1.4: Prove worker independence with repeated parent-memory, prepared-input,
  request-sequence, and independent platform-time controls on both hosts.
- 2.7: Integrate the worker only after that proof. Complete sampled limits,
  cancellation, worker/coordinator failure, bounded communication, and cleanup
  tests before making it the default.
- 3.5: Finish identity and evidence integration and operational guidance for
  the adopted worker. Report and loader gates alone do not complete this task.
- 4.1: Run final elevated macOS BSD-time controls.
- 4.2: Run final native Linux GNU-time controls.
- 4.3: Quantify final timing error and dispersion on both hosts.
- 4.4: Run the full correctness-gated jq/yq/tq matrices on both hosts.
- 4.5: Compare baseline and candidate tq under the same final collector and
  resolve the issue-specific 20% disclosure and 50% blocking thresholds.
- 4.6: Publish only verified results, preserving authored text outside markers.
- 5.1: Complete final repository preflight after worker integration.
- 5.2: Repeat this verification against the completed implementation and
  retained native evidence.

The earlier source-transfer restriction was resolved by the user's explicit
standing Ironhide testing permission on 2026-09-11. The source refresh and
native release build succeeded. Final native calibration and campaign
acceptance remain incomplete; permission is no longer the blocker.

The resumed [native worker proof](worker-proof.md) produced 480 paired
observations across two runs per host. All RSS comparisons passed; 11 CPU
comparisons failed. The unchanged repeat reproduced the discrepancy. Task 1.4
remains critical and open, and the worker has not been adopted.

## Correctness checks

The local worker tests cover coordinator memory, large prepared stdin,
literal arguments, binary input and EOF, nonzero exits, and target timeout.
They also verify the requested working directory. Five integration tests pass.
These checks exercise the explicit prototype, not full campaign adoption.
The initial reply protocol failed deserializing `u128`; changing to externally
tagged replies resolved the observed failure.

Report tests cover historical decoding, worker identity mismatches across
rows, missing launch evidence, and publication rejection. Calibration review
found that comparing evidence-bearing metadata to a raw sample would reject
valid launches. The comparison now preserves the worker/launch contract while
attaching independently loaded calibration evidence afterward.

The loader computes the evidence digest from retained summary bytes, avoiding
a self-referential digest requirement. Review warnings for zero page size,
insufficient declared repetitions, unconstrained tolerances, and inconsistent
residual-floor summaries were fixed with rejection tests. All 16 loader tests
pass. Worker identity remains part of the exact launch-contract comparison.

## Coherence

The implementation keeps prepared input outside worker launch memory, owns
target reaping in the worker, and leaves worker startup outside the target
interval. It neither subtracts an RSS floor nor relabels old measurements.
The default collector and publication acceptance must not be enabled for this
worker until the open proof and lifecycle tasks pass.

No Windows verification or new native Results publication is claimed.
Full local prototype preflight passed, including strict Clippy, all workspace
tests, documentation validation, and all 20 OpenSpec items. Rust 1.95.0
all-target checking also passed. These checks do not close the post-integration
verification or native evidence tasks.
See [implementation evidence](implementation-notes.md) for checkpoint commands,
historical controls, and their collector identities.
