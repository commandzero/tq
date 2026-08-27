# Benchmark campaigns

A reviewed benchmark ends as `benchmarks/<timestamp>.md`. Do not commit the
downloaded corpus, generated formats, or full JSON sample data. The runner
writes those files to `benchmarks/.work/`, which Git ignores. Create the
date-named summary only after review.

The catalog in `cases/workloads.jsonl` runs jq on JSON, yq on JSON and YAML, and
tq on JSON, YAML, and TOON. It reports native-format views separately. The
runner checks ordered values before it times a row.

Profiles keep their natural source sizes. Smoke uses checked-in examples.
Standard uses refreshed USGS feeds. Large uses the roughly 1 GB Microsoft US
building-footprint archive. The runner extracts archives, generates formats,
and checks JSON, YAML, and TOON equivalence before timing.

If a refresh stops after installing generated files but before admitting the
manifest, resume validation without regenerating the files:

```console
cargo run -p tq-test-support --bin tq-corpus -- \
  finalize CACHE_ROOT CACHE_ROOT/campaigns/ID/SOURCE/manifest.json
```

You can rerun `finalize` on a manifest that already passed cross-format
validation. It validates the existing representations, then records their byte
counts and SHA-256 identities in one atomic update.

The Make targets refresh or replay the selected corpus and write local reports:

```console
make benchmark-smoke
make benchmark-standard
make benchmark-large
make benchmark-stack-overflow
```

That target runs the checked-in Stack Overflow scenarios through the shared
Rust correctness and measurement code.

To regenerate the scenarios from Stack Exchange API snapshots, use the Rust
fixture generator:

```sh
cargo run --quiet -p tq-test-support --bin tq-stack-overflow-scenarios -- \
  --questions /path/to/questions.json \
  --answers /path/to/answers \
  --patch /tmp/stack-overflow.patch
```

The generator reads `tests/stack-overflow-benchmarks.json` and emits a patch. It
does not modify the scenario directory.

For an explicit reproducible run, first build tq in release mode and pass the
recorded corpus manifest:

```console
cargo build --release
TQ_BIN="$PWD/target/release/tq" cargo run -p tq-test-support --bin tq-bench -- \
  run --profile standard --output benchmarks/.work/standard.json \
  --cache-root benchmarks/.work/corpus --origin frozen --manifest PATH
```

Binary discovery prefers `../jq/jq` and `../yq/yq`, then
`target/reference-build/{jq,yq}/`, then `PATH`. Set `TQ_JQ`, `TQ_YQ`, or
`TQ_BIN` to select an exact local build. Every report hashes the selected
binaries. A source checkout without a built binary cannot be mistaken for the
binary under test.

Frozen investigations add `--origin frozen --manifest PATH`. `--max-samples 1`
is useful for validation, and `--case benchmark.event-stream` selects one
workload. A reviewed long-running campaign may also use `--timeout-seconds N`
and `--rss-limit-bytes N`; these overrides are copied into every report row,
and an existing stricter per-case RSS limit still wins. The working JSON
retains host, compiler, tool, corpus, command, limit, and environment data. It
is for local review only. The versioned result is the concise
`YYYY-MM-DD.md` Markdown summary.

Correctness normalization uses a file and has a 32 MiB limit. If the reference
result exceeds that limit, the campaign runs one bounded probe for each adapter
and records `resource-limit`, timeout, or signal outcomes. It does not time
unverified output or load a multi-gigabyte result into the runner.

On macOS, RSS enforcement samples the complete child process group with
`ps -axo pgid=,rss=`. Run resource-gated campaigns in an environment that
permits that command. If a sandbox blocks process inspection, the report marks
RSS unavailable and cannot claim that it enforced the limit.

Pass `--baseline PATH` to evaluate a manifest-aware tq self-regression. The
accepted local defaults are 50% median wall time, 20% peak RSS, and at least
five samples; override them with `--wall-regression-percent`,
`--rss-regression-percent`, and `--minimum-regression-samples`. See
`docs/performance-baseline.md` for the baseline review and unfavorable results.

The report does not calculate an aggregate winner. Review wall time and
dispersion, time to first result, CPU, records per second, MiB per second, peak
RSS, output bytes, plan class, and every failure row. On the recorded local
host, the large explicit-stream release gate requires peak RSS at or below 128
MiB.
