---
type: Guide
title: "Contributing to tq"
description: "Contributor setup, validation, and development requirements."
generated: { by: codex/gpt-6-astra, at: 2026-09-15T18:15:13Z }
---

# Contributing to tq

All workspace crates require Rust 1.95 or newer. Use the repository's pinned
toolchain for preflight checks. Run all commands below from the repository root.

Run the repository preflight before submitting a change:

```console
./scripts/preflight.sh
```

The script checks documentation, shell scripts, repository governance tests,
Rust formatting, compilation, lint rules, workspace tests, and main OpenSpec
specifications. New automation should call this script.

For tool installation, documentation-only checks, and PR OpenSpec completion,
see [contributor checks](contributor-checks.md).
Use the [release playbook](releasing.md) for coordinated publication and the
[changelog policy](changelog-policy.md) for new history entries.

Shared standards come from the repo-man bundle selected by the nearest workspace
AGENTS.md. Resolve its pointer relative to that file. If no pointer is configured,
use the local repo-man entry point at `~/.agents/memory/repo-man/index.md`.

The campaign runner handles compatibility, benchmark, and fuzz programs:

```console
./scripts/campaign-run.sh compatibility smoke
./scripts/campaign-run.sh compatibility full
./scripts/campaign-run.sh benchmark smoke
./scripts/campaign-run.sh benchmark standard
./scripts/campaign-run.sh benchmark large
./scripts/campaign-run.sh fuzz default
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

For in-process Criterion attribution, see
[event-stream microbenchmarks](tests/event-stream-microbenchmarks.md).
The full preflight compiles these optimized targets and runs their correctness smoke path.

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
