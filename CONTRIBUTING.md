# Contributing to tq

Use `make` for the repository's standard development tasks:

```console
make help
make preflight
```

`make preflight` works even before the `tq` engine exists. It checks formatting,
compilation, lint rules, workspace tests, and the active OpenSpec change. New
automation should call this target. The MVP does not depend on hosted CI.

These targets run the compatibility, benchmark, and fuzz programs:

```console
make compatibility-smoke
make compatibility-full
make benchmark-smoke
make benchmark-standard
make benchmark-large
make fuzz
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

## Capability promotion

When you add syntax or a built-in, update the resolver registry and capability
analysis along with the parser, evaluator, compatibility cases, and benchmarks.
If the change keeps a complete document, every input, or blocking operator
state, expose that fact in `--explain`. Keep event-plan APIs separate from
document and whole-input plans with Rust typestate.

## Baseline-first gate

The benchmark corpus and jq/yq baselines define the behavior that `tq` must
follow. Do not start section 6 or later in
`openspec/changes/build-tq-mvp/tasks.md` until every task in sections 2 through
5 is complete.

Run `make engine-gate` before engine work. Until the baseline is complete, the
command fails and prints the remaining tasks. During the MVP, benchmark results
come from local runs. Every report must identify the machine and tools because
results from different systems are not directly comparable.
