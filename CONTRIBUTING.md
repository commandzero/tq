# Contributing to tq

The repository uses `make` as its stable local task interface. Run:

```console
make help
make preflight
```

`make preflight` is deliberately usable before the `tq` engine exists. It checks
formatting, compilation, lint policy, workspace tests, and the active OpenSpec
change. Future automation should call this same target; hosted CI is not an MVP
development dependency.

The compatibility, benchmark, and fuzz targets all execute real harnesses:

```console
make compatibility-smoke
make compatibility-full
make benchmark-smoke
make benchmark-standard
make benchmark-large
make fuzz
```

## Compatibility-case-first development

Behavior changes begin as a versioned case under `compatibility/cases/`. Run
the case against jq and, where applicable, yq before changing tq. Promote tq
support only when its ordered normalized result, cardinality, error class, and
exit behavior satisfy the reviewed jq-target contract. Do not sort objects or
result streams to hide differences.

Baseline updates are reviewed diffs, never a bulk bless. A changed observation
must be classified as common agreement, jq-target divergence, CLI adaptation,
unsupported, or deferred. Preserve raw and normalized evidence for unexpected
crashes, timeouts, signals, and malformed output.

## Benchmark correctness gates

A benchmark row is timed only after its output passes the same semantic
normalization contract. JSON, YAML, and TOON representations are generated
outside timing and must match the ordered source model. Natural source files
are not sliced, repeated, padded, sampled, or truncated. Reports from different
host or corpus manifests are informative but not directly comparable.

Add a self-regression threshold only after a stable local baseline exists.
jq/yq ratios are comparisons, not universal tq pass/fail gates. Preserve
incorrect, unsupported, timeout, signal/OOM, and resource-limit rows.

## Capability promotion

When adding syntax or a built-in, update the resolver registry and capability
analysis together with parser, evaluator, compatibility, and benchmark cases.
If a feature retains a complete document, all inputs, or blocking operator
state, make that visible in `--explain`. Event-plan APIs must remain separated
from document/whole-input plans by Rust typestate.

## Baseline-first gate

The benchmark corpus and jq/yq baselines define the behavior that `tq` must
follow. Therefore no task in section 6 or later of
`openspec/changes/build-tq-mvp/tasks.md` may begin until every task in sections
2 through 5 is complete.

Run `make engine-gate` before starting engine work. It exits unsuccessfully and
prints the remaining baseline tasks until that hard gate is satisfied. Local
development owns benchmark results during the MVP; reports must record the
machine and tool identities because results from different systems are
informative but not directly comparable.
