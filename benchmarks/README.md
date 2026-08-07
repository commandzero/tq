# Benchmark campaigns

`benchmarks/<timestamp>.md` is a reviewed final benchmark artifact. Do not
commit the downloaded corpus, generated formats, or full JSON sample
collection. The campaign runner writes those files to `benchmarks/.work/`,
which Git ignores. Create a date-named Markdown summary only after review.

The benchmark catalog in `cases/workloads.jsonl` compares jq over JSON, yq over
JSON/YAML, and tq over JSON/YAML/TOON. Native-format views are reported
separately. Every timed row first passes an ordered semantic correctness gate.

Profiles use natural source sizes: checked-in examples for smoke, refreshed
USGS feeds for standard small/medium coverage, and the Microsoft US building
footprint archive for the ~1 GB-class large profile. Representation generation,
archive extraction, and JSON/YAML/TOON equivalence checks happen before timing.

The Make targets refresh or replay the selected corpus and write local reports:

```console
make benchmark-smoke
make benchmark-standard
make benchmark-large
```

For an explicit reproducible run, first build tq in release mode and pass the
recorded corpus manifest:

```console
cargo build --release
TQ_BIN="$PWD/target/release/tq" cargo run -p tq-test-support --bin tq-bench -- \
  run --profile standard --output benchmarks/.work/standard.json \
  --cache-root benchmarks/.work/corpus --origin frozen --manifest PATH
```

Executable discovery prefers `../jq/jq` and `../yq/yq`, then
`target/reference-build/{jq,yq}/`, then `PATH`. Set `TQ_JQ`, `TQ_YQ`, or
`TQ_BIN` to make an exact local build authoritative. Every report hashes the
selected binaries, so a source checkout without a built executable is never
silently represented as that checkout.

Frozen investigations add `--origin frozen --manifest PATH`. `--max-samples 1`
is useful for validation, and `--case benchmark.event-stream` selects one
workload. The working JSON retains host, compiler, tool, corpus, command,
limit, and environment data. It is for local review only; the versioned final
artifact is the concise `YYYY-MM-DD.md` Markdown summary.

Pass `--baseline PATH` to evaluate a manifest-aware tq self-regression. The
accepted local defaults are 50% median wall time, 20% peak RSS, and at least
five samples; override them with `--wall-regression-percent`,
`--rss-regression-percent`, and `--minimum-regression-samples`. See
`docs/performance-baseline.md` for the baseline review and unfavorable results.

No aggregate winner score is calculated. Review wall time and dispersion,
startup/time-to-first-result, user and system CPU, logical records/s, physical
MiB/s, peak RSS, output bytes, spooling/blocking class, and all failure rows.
The large explicit-stream release gate targets peak RSS at or below 128 MiB on
the recorded local host.
