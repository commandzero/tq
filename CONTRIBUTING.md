# Contributing to tq

Run the repository preflight before submitting a change:

```console
./scripts/preflight.sh
```

The script checks formatting, compilation, lint rules, workspace tests, and all
OpenSpec specifications. New automation should call this script.

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

Run every benchmark command outside restricted sandboxes with elevated
permission to inspect child processes. On macOS, use `/usr/bin/time -l` for
every authoritative peak RSS sample and record `maximum resident set size`.
Discard and rerun campaigns whose permissions leave RSS unavailable.

## Capability promotion

When you add syntax or a built-in, update the resolver registry and capability
analysis along with the parser, evaluator, compatibility cases, and benchmarks.
If the change keeps a complete document, every input, or blocking operator
state, expose that fact in `--explain`. Keep event-plan APIs separate from
document and whole-input plans with Rust typestate.
