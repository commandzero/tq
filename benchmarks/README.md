# Benchmark campaigns

The benchmark scripts, catalog, and Rust harness live in this repository.
Reviewed findings and generated comparison pages live here under
`docs/tests/comparison/`. Raw reports, logs, provenance, downloaded corpora,
generated formats, and full JSON sample data live in the separate
`commandzero/tq-benchmarks` archive repository; do not commit those artifacts
here. The campaign runner discovers a sibling `tq-benchmarks` checkout
automatically; set `TQ_BENCHMARK_ARCHIVE_ROOT` when the archive lives elsewhere.

The saved-report renderer updates the `## Results` blocks in the stable
user-facing pages under `docs/tests/comparison/`. Each workload has an authored
explanation that regeneration preserves. These pages show a last-updated date,
not dated filenames or historical run records. Do not embed archive paths,
local executable paths, or exhaustive provenance in them. Raw samples, logs,
and corpus artifacts remain in the benchmark archive. Render smoke before
standard; rendering standard last leaves the index on the full standard review.

Every benchmark report must include the measured host's CPU model, logical
CPU count, RAM capacity, OS, architecture, kernel, and build profile. Use
recorded run metadata, not specifications from a different machine or a later
session. Mark unavailable specifications as not recorded. Never include the
hostname in published benchmark artifacts. Keep worker-validation controls
separate from jq/yq/tq workload measurements.

To update only these result blocks from saved measurements without rerunning
the tools, use:

```console
cargo run -p tq-test-support --bin tq-bench -- \
  --render-only REPORT_PATH --markdown-dir docs/tests/comparison
```

Rapid and large runs do not replace the standard reference pages. Smoke fills
the three additional workload pages included in the current review.

## Required execution permissions and accounting

The audited `wait4 0.2.0` dependency assumes valid, nonnegative OS counters
within its numeric range. Its internal conversions expose no overflow error;
on supported 64-bit hosts, RSS overflow would require roughly 8 EiB and CPU
overflow roughly 292,000 CPU-years. First-party arithmetic remains checked,
and the harness rejects detectable invalid returned values. This does not
guarantee detection of every malformed upstream value. Native allocation and
independent `time` controls remain required.

The legacy shell experiments under `benchmarks/cases/` retain their original
`time` wrappers for reproducing earlier runs. Their output is not
native-accounting campaign evidence and cannot serve as an equivalent-method
baseline for this change.

Run every authoritative benchmark campaign outside restricted sandboxes with
the elevated permissions needed for native child accounting. In Codex, this
means approving elevated execution for the complete campaign command. The
required production path is `tq-bench` using an isolated Rust worker to invoke
each executable directly. One resource-aware waiter collects the exact child's
exit status, CPU usage, and OS-recorded peak RSS. The worker keeps coordinator
memory out of the child's inherited RSS floor. Its allocation preflight runs through that same
native interface before corpus preparation. If preflight cannot collect positive
RSS, valid units, or explicit collector provenance, abandon the campaign before
corpus work and repair the host permissions before retrying. A later missing or
invalid authoritative sample invalidates the campaign; do not write or review a
partial report. A draft or unverified native backend is not accepted benchmark
evidence.

Use `/usr/bin/time -l` on macOS and GNU `/usr/bin/time -v` on Linux only as
independent validation of the native counters. They are not production
wrappers and their observations must remain separately labeled. `ps` is not a
default dependency: select it only for an explicitly requested process-group
RSS limit or diagnostic, and record its process scope and sampling interval
because a sampler can miss short-lived peaks. Measurements without sampled
limits work without `ps`; catalog cases selecting an RSS limit still require
it for enforcement repetitions. Production runs need neither `/usr/bin/time`
nor Python allocation probes.

For RSS-limited rows, the sampling loop runs a separate instrumented repetition
before each timing repetition. A failed instrumented repetition stops that
row. The timing repetition has no recurring RSS sampler; it still checks the
native peak against the limit at exit and enforces timeout and output limits
while running. This is not an in-flight RSS ceiling for the timing repetition.
Reports retain `instrumented_samples` separately and never mix them into
primary timing or peak-RSS summaries. Primary RSS still comes from the native
waiter for that exact timing child.

Requested inspection fails if it returns no positive process-group RSS while
the exact child is still alive. A child that exits before inspection can have
no sampled observation; its native exit-time peak still enforces the limit.
Reports keep that sampled field unavailable rather than fabricating zero.

Reports retain the direct spawn-to-exit timing boundary, input-delivery method,
RSS scope, collector provenance, and observed host timing controls. Displayed
one-decimal values are presentation only; stored precision and clock
resolution do not establish equivalent accuracy, and nanosecond storage does
not imply nanosecond accuracy. Repeat no-op and known-duration controls when
validating supported precision. Native Windows accounting is deferred to issue
#31; cross-compilation or emulation is not native verification.

Run the retained native controls with the release build and elevated permissions:

```console
TQ_NATIVE_VALIDATION_OUT=/path/to/tq-benchmarks/.work/native-validation \
  cargo test --release -p tq-test-support --test benchmark_native_validation -- \
  --ignored --exact native_accounting_validation_writes_retained_evidence --nocapture
```

For publication, pass the successful control summary to the campaign with
`--timing-calibration SUMMARY_PATH`. The driver checks the host configuration,
compiled collector-source identity, release profile, control sample counts, and
measurement protocol. Repeat the option when separate instrumentation needs
separate controls. An uninstrumented control cannot validate a sampled run.
The report retains the summary's SHA-256 and maximum observed control excess
duration. This is full process runtime minus the requested control interval,
including startup, sleep scheduling and exit observation. It is not isolated
timer error or a guaranteed bound, and is never subtracted from samples.
Compare tools only within the same host and OS, with equivalent harness
settings, timing boundaries, instrumentation and disclosed concessions.
Repeated samples and dispersion remain necessary; a shared harness does not
make noise cancel exactly. Runs without linked controls are diagnostic
evidence, not a completed native publication review.

For `scripts/run-campaign.sh benchmark standard`, set
`TQ_TIMING_CALIBRATION` to that summary path. The script requires it before
preparing the corpus because standard runs publish comparison tables. Rapid,
smoke, and large runs also forward this variable when supplied. Use the CLI
directly for multiple calibration files or to record a standard campaign
without publishing tables.

For issue #30, review wall time and peak RSS independently for every comparable
workload. Disclose each increase above 20% with baseline, candidate, sample
count, dispersion, and an explanation. Documented increases greater than 20%
and at most 50% are acceptable when every other gate passes; an increase above
50% blocks acceptance until mitigated and remeasured. Exactly 20% is not a
disclosure and exactly 50% is not a blocking increase. Cross-tool jq/yq ratios
remain comparative evidence, never tq self-regression evidence.

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

The generator reads `tests/stack-overflow-benchmarks.toon` and emits a patch. It
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

Before a full review, verify that jq, yq, and tq are all discoverable and that
their recorded paths, versions, and build identities match that campaign's
pinned inputs. For the current Linux review, the pinned yq identity is
4.53.2. A missing or mismatched required executable is an environment blocker:
stop and repair discovery or installation instead of recording its adapters as
unsupported. Claim a final comparison only after all three tools have completed
the requested suite.

Frozen investigations add `--origin frozen --manifest PATH`. `--max-samples 1`
is useful for validation, and `--case benchmark.event-stream` selects one
workload. A reviewed long-running campaign may also use `--timeout-seconds N`
and `--rss-limit-bytes N`; these overrides are copied into every report row,
and an existing stricter per-case RSS limit still wins. The working JSON
retains host, compiler, tool, corpus, command, limit, environment, and
measurement-protocol data, including timing boundary, input delivery, RSS
scope, and validated precision. It lives in the archive checkout's `.work/`
directory. The reviewed findings are
rendered into the committed pages under `docs/tests/comparison/`; raw archive
files do not have a second dated Markdown summary.

Correctness normalization uses a file and has a 32 MiB limit. If the reference
result exceeds that limit, the campaign runs one bounded probe for each adapter
and records `resource-limit`, timeout, or signal outcomes. It does not time
unverified output or load a multi-gigabyte result into the runner.

The production contract launches the selected executable directly and freezes
wall time at native child exit observation, after inputs and captures are
prepared and before cleanup or report generation. Worker startup and request
delivery are outside this boundary. First-result latency is the first captured output when available; its
observation method and validated precision are recorded in the report. Native
RSS covers the waited-for child's lifetime, including its pre-exec footprint,
threads, and waited-for descendants included by the platform waiter. It is not
a heap, physical-footprint, or summed process-tree metric. The small worker
footprint remains a measured RSS floor; it is not subtracted from results.

Reports must state whether the platform waiter accounts for only the waited-for
child or also includes waited-for descendants. Limit process-only comparisons
to verified non-forking jq, yq, and tq workloads; forked workloads require an
explicitly comparable RSS scope.

If an explicit RSS limit or diagnostic selects process-group sampling, macOS
may use `ps -axo pgid=,rss=` and Linux the equivalent host query. Record the
sampled scope and interval and treat it as an enforcement/diagnostic signal
that can miss peaks; it never replaces native child accounting. For independent
validation, use `/usr/bin/time -l` on macOS or GNU `/usr/bin/time -v` on Linux
with the same executable, input, and command contract, and retain that evidence
as a separate method. Do not accept a report that marks authoritative RSS
unavailable: stop, repair the execution environment, and rerun the campaign.

The harness does not use Python allocation probes. Native Windows accounting is
deferred to issue #31 and unsupported-platform preflight failures must remain
visible rather than being presented as verification. Existing wrapper or
sampled reports retain their original method and are not pooled with native
measurements.

Pass `--baseline PATH` to evaluate a manifest-aware tq self-regression. The
command's conservative local defaults are 50% median wall time, 50% peak RSS,
and at least five samples; override them with `--wall-regression-percent`,
`--rss-regression-percent`, and `--minimum-regression-samples`. For issue #30
publication review, disclose every comparable wall-time and peak-RSS increase
above 20% independently, document increases through 50% as acceptable only
with an explanation, and block any increase above 50% until remeasured. The
comparison must use equivalent native-method reports; wrapper or sampled
historical reports are retained but are not comparable baselines. See
`docs/performance-baseline.md` for the baseline review and unfavorable results.

The report does not calculate an aggregate winner. Review wall time and
dispersion, time to first result, CPU, records per second, MiB per second, peak
RSS, output bytes, plan class, and every failure row. On the recorded local
host, the large explicit-stream release gate requires peak RSS at or below 128
MiB.

The current reviewed `tq`/`yq`/`jq` comparison is the accepted Linux rapid and
standard campaigns plus the three smoke-only workloads. Its findings are
documented in `docs/tests/comparison/`. The archive stores the corresponding
raw reports, logs, and provenance. This review does not include a new large
corpus campaign; the large and extra-large procedures below remain separate
diagnostic campaigns.

The extra-large parallel campaign is intentionally narrower than the full
large matrix. It correctness-checks `[.features[].properties.release] | sort`
against a single-process `jq` baseline, then records wall time, user/system
CPU, peak RSS, and output digest for `jq` and `tq` with one, four, eight, and all
available workers. It reuses the validated
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
