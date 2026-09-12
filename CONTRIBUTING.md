# Contributing to tq

The workspace requires Rust 1.95 or newer. Use the repository's pinned
toolchain for preflight checks.

Run the repository preflight before submitting a change:

```console
./scripts/preflight.sh
```

The script checks formatting, compilation, lint rules, workspace tests, and all
OpenSpec specifications, plus the complete `docs/` bundle with OKF 0.2.7.
Install the pinned documentation validator with
`cargo install okf --version 0.2.7 --locked` when it is not already available.
Install OpenSpec 1.11.0 with
`npm install --global --save-exact @fission-ai/openspec@1.11.0` as well.
New automation should call this script.

The campaign runner handles compatibility, benchmark, and fuzz programs:

```console
./scripts/run-campaign.sh compatibility smoke
./scripts/run-campaign.sh compatibility full
./scripts/run-campaign.sh benchmark smoke
./scripts/run-campaign.sh benchmark standard
./scripts/run-campaign.sh benchmark large
./scripts/run-campaign.sh fuzz default
```

## Compatibility-case-first development

Start each behavior change with a versioned case under
`tests/compatibility/cases/`. Run it against jq and, when relevant, yq before
changing tq. Mark the behavior as supported only after tq matches the reviewed
jq contract for result order, result count, error class, and exit status. Never
sort objects or result streams to hide a difference.

Review each baseline update as a diff. Do not accept all changes in one step.
Classify each changed observation as common agreement, jq-target divergence,
CLI adaptation, unsupported, or deferred. Keep raw and normalized evidence for
crashes, timeouts, signals, and malformed output.

## Benchmark correctness gates

The runner times a benchmark row only after its output passes the semantic
correctness check. It generates JSON, YAML, and TOON before timing, and each
representation must match the ordered source model. Do not slice, repeat, pad,
sample, or truncate a natural source file. Results from different machines or
corpus manifests are not directly comparable.

Add a self-regression threshold only after a stable local baseline exists.
jq/yq ratios are comparisons, not universal tq pass/fail gates. Preserve
incorrect, unsupported, timeout, signal/OOM, and resource-limit rows.

Run every authoritative benchmark campaign outside restricted sandboxes with
the elevated permissions needed for native child accounting. The required
production path is the Rust harness launching each executable directly and,
once its native backend passes lifecycle and audited-counter validation,
collecting the exact child's exit status, CPU usage, and OS-recorded peak RSS
through its resource-aware waiter. It runs the native allocation preflight
before corpus preparation; if RSS, units, or collector provenance are
unavailable, abort before corpus work and repair the host permissions before
retrying. A later invalid or missing authoritative RSS sample invalidates the
campaign and must not be published. A draft or unverified native backend is
not accepted benchmark evidence.

Use `/usr/bin/time -l` on macOS and GNU `/usr/bin/time -v` on Linux only for
independent validation of the native counters. These commands do not wrap
production benchmark invocations or replace the harness accounting. `ps` is
optional and may be selected only for explicitly requested process-group RSS
limits or diagnostics; record its scope and interval because sampling can miss
short peaks. Measurements without sampled limits do not require `ps`.
Catalog cases that select an RSS limit, such as event streaming, still require
it for separate enforcement repetitions. Production measurements never require
`/usr/bin/time` or Python probes.

Reports record the direct spawn-to-exit timing boundary, input-delivery method,
RSS scope, and observed timing controls. One-decimal display precision is a
presentation choice, not evidence of sub-millisecond or nanosecond accuracy.
No-op and known-duration controls include startup and scheduling effects, not
just timer error. Compare tools on the same host and OS with equivalent
measurement settings, using repetitions and dispersion. Native
Windows accounting remains deferred to issue #31 and cannot be claimed as
verified by cross-compilation or emulation.

State whether the platform waiter accounts for only the waited-for child or
also includes waited-for descendants. Limit process-only comparisons to
verified non-forking jq, yq, and tq workloads; forked workloads require an
explicitly comparable RSS scope.

For issue #30 self-regression review, disclose each comparable wall-time and
peak-RSS increase above 20% independently with baseline, candidate, samples,
dispersion, and an explanation. An increase greater than 20% and at most 50%
is acceptable only when documented and all other gates pass; an increase above
50% blocks acceptance until it is mitigated and remeasured. Exactly 20% does
not require disclosure and exactly 50% does not block acceptance.

## Capability promotion

When you add syntax or a built-in, update the resolver registry and capability
analysis along with the parser, evaluator, compatibility cases, and benchmarks.
If the change keeps a complete document, every input, or blocking operator
state, expose that fact in `--explain`. Keep event-plan APIs separate from
document and whole-input plans with Rust typestate.
