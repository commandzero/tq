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

The compatibility, benchmark, and fuzz targets are named now so their interface
does not drift while their harnesses are built:

```console
make compatibility-smoke
make compatibility-full
make benchmark-smoke
make benchmark-standard
make benchmark-large
make fuzz
```

Until the corresponding harness is implemented, these targets fail with an
explicit staged-implementation message. They must never report a placeholder
campaign as a successful test or measurement.

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
