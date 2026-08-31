# Benchmark campaigns

The benchmark scripts, catalog, and Rust harness live in this repository.
Reviewed reports and generated results live in the separate
`commandzero/tq-benchmarks` repository. Do not commit downloaded corpora,
generated formats, full JSON sample data, or reviewed reports here. The
campaign runner discovers a sibling `tq-benchmarks` checkout automatically;
set `TQ_BENCHMARK_ARCHIVE_ROOT` when the archive lives elsewhere.

The catalog in `cases/workloads.jsonl` runs jq on JSON, yq on JSON and YAML, and
tq on JSON, YAML, and TOON. It reports native-format views separately. The
runner checks ordered values before it times a row.

Profiles keep their natural source sizes. Smoke uses checked-in examples.
Rapid uses the cached `usgs-all-month` USGS snapshot with the five high-signal
cases and one measured sample per row. Standard uses cached USGS feeds. Large
uses the roughly 1 GB Microsoft US building-footprint archive. The first run on
a machine downloads missing sources, then uses the release `tq` binary to
generate and validate a compact,
lossless YAML 1.2 JSON-subset representation and TOON. Later runs reuse the
admitted snapshot without network access, conversion, or full-file hashing.
Extra-large runs the selected-array JSON scaling benchmark against that same
Microsoft archive, comparing one, four, eight, and all available Rayon workers.

If a refresh stops after installing generated files but before admitting the
manifest, resume validation without regenerating the files:

```console
cargo run --release -p tq-test-support --bin tq-corpus -- \
  finalize CACHE_ROOT CACHE_ROOT/campaigns/ID/SOURCE/manifest.json
```

You can rerun `finalize` on a manifest that already passed cross-format
validation. It validates the existing representations, then records their byte
counts and SHA-256 identities in one atomic update.

Preparation is idempotent and normally happens through the Make target. To
prepare without running benchmarks, build `tq` and invoke the release corpus
helper directly:

```console
cargo build --release -p tq-cli
TQ_BIN="$PWD/target/release/tq" cargo run --release -p tq-test-support --bin tq-corpus -- \
  prepare tests/corpus/sources /path/to/tq-benchmarks/.work/corpus standard
```

Use `large` for the building-footprint snapshot. `prepare` reuses the newest
admitted source and resumes an interrupted snapshot. Use `refresh` instead of
`prepare` only when you intend to download current upstream data. Use `verify`
for an explicit full SHA-256 and cross-format audit. Normal benchmark runs use
the machine-local verification cache and check file metadata before replay.

The campaign runner prepares or replays the selected corpus and writes local
reports:

```console
./scripts/run-campaign.sh benchmark
./scripts/run-campaign.sh benchmark rapid
./scripts/run-campaign.sh benchmark smoke
./scripts/run-campaign.sh benchmark standard
./scripts/run-campaign.sh benchmark large
./scripts/run-campaign.sh benchmark extra-large
TQ_CORPUS_ORIGIN=refreshed ./scripts/run-campaign.sh benchmark extra-large
./scripts/run-campaign.sh benchmark stack-overflow
```

The standard and large profiles reuse the machine-local corpus. Set
`TQ_CORPUS_ORIGIN=refreshed` to acquire a new upstream snapshot before running
the rapid, standard, large, or extra-large profile.

The campaign runner defaults to the rapid profile when called as
`./scripts/run-campaign.sh benchmark`, and `tq-bench` uses the same default when
no profile is supplied.

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
export TQ_BENCHMARK_ARCHIVE_ROOT=/path/to/tq-benchmarks
TQ_BIN="$PWD/target/release/tq" cargo run --release -p tq-test-support --bin tq-bench -- \
  run --profile standard \
  --output "$TQ_BENCHMARK_ARCHIVE_ROOT/.work/standard.json" \
  --cache-root "$TQ_BENCHMARK_ARCHIVE_ROOT/.work/corpus" \
  --origin frozen --manifest PATH
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
lives in the archive checkout's `.work/` directory beside the concise
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

The latest full `tq`/`yq`/`jq` campaign is stored in the archive repository. It
includes the complete standard matrix and a bounded large-corpus diagnostic.

The extra-large parallel campaign is intentionally narrower than the full
large matrix. It correctness-checks `[.features[].properties.release] | sort`,
then records wall time, user/system CPU, peak RSS, and output digest for one,
four, eight, and all available workers. It reuses the validated
`microsoft-us-buildings-georgia` manifest and writes samples under
`.work/parallel-selected-json/YYYY-MM-DD/`.

```console
./scripts/run-campaign.sh benchmark extra-large
```

## Streaming transcode campaign

The identity-transcode campaign compares automatic structural transcode with
the internal forced-document benchmark override. It checks byte equality before
timing and records wall time, CPU, RSS, first-byte latency, output bytes,
first-payload latency, preparation high water, object-index spills, array
preparations, spool bytes written and replayed, and the final resource outcome.
It includes wide and
nested objects, root and nested arrays, scalar arrays, tabular candidates, and
the accepted natural `segments` and `recovery` documents.

```console
cargo build --release
export TQ_BENCHMARK_ARCHIVE_ROOT=/path/to/tq-benchmarks
benchmarks/cases/generate-streaming-transcode-fixtures.sh \
  "$TQ_BENCHMARK_ARCHIVE_ROOT/.work/streaming-transcode-inputs"
# Copy the accepted natural segments.json and recovery.json beside them.
RUNS=7 benchmarks/cases/streaming-transcode.sh \
  "$TQ_BENCHMARK_ARCHIVE_ROOT/.work/streaming-transcode-inputs" \
  target/release/tq \
  "$TQ_BENCHMARK_ARCHIVE_ROOT/.work/streaming-transcode-results"
```

The accepted-host release gate applies to direct TOON sequence output and
requires both natural cases to stay below 64 MiB peak RSS. Missing natural
inputs are reported and cannot be treated as a passing gate. The forced path is
enabled only through `TQ_BENCH_FORCE_DOCUMENT=1`; it is not a public CLI mode.

## `toon` faceoff

The three-way faceoff runs `tq`, default `toon`, and `toon` with its
`json_stream` feature through a shared correctness gate before timing. The
archive repository holds the reviewed report, charts, exact binary identities,
and replay data. Run it with:

```console
RUNS=7 benchmarks/cases/toon-vs-tq.sh \
  INPUT_DIR target/release/tq TOON_DEFAULT_BIN TOON_STREAM_BIN OUTPUT_DIR
```
