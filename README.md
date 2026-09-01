# tq

`tq` runs jq 1.8.x-style queries over TOON, YAML, JSON, JSON5, and JSON Lines. It
writes TOON Text Sequences by default and can stream JSON, JSON Lines, and TOON
without loading the complete input.

`tq` supports common jq filters, including navigation, pipes, generators,
conditionals, operators, variables, path updates, user filters, modules, and
the common built-ins. Arrays and ordered objects retain their jq semantics.
The language also includes `empty`, `error`, optional access, `try/catch`,
`reduce`, and `foreach`. See [jq compatibility](docs/compatibility.md) for
supported syntax and known differences.

## Install and use

Rust 1.87 or newer is required.

```console
cargo install tq-cli
tq '.features[] | {id, magnitude: .properties.mag}' feed.json
```

To build from a checkout instead, run `cargo build --release` and use
`target/release/tq`.

With no file argument, `tq` reads stdin. It processes files and `-` in argument
order. Recognized `.toon`, `.yaml`, `.yml`, `.json`, `.json5`, `.jsonl`, and
`.ndjson` extensions select the parser. Other sources use bounded content
detection. For ambiguous input, select a parser with
`--input-format toon|yaml|json|json5|jsonl`. `ndjson` is an alias for `jsonl`.
JSON5 is document-at-a-time input and accepts the literal triple-double-quoted
multiline strings used by kibana-sync. Files ending in `.json` remain strict
JSON, so select `-i json5` when a JSON5 producer uses that extension.

```console
printf 'name: Ada\nactive: true\n' | tq '.name'
tq --input-format json --output-format json -c '.features | length' feed.json
tq --input-format yaml -r '.people[].name' people.yaml
tq --input-format json5 -r '.attributes.title' saved-object.json
tq -i jsonl -o jsonl '.event' events.ndjson
unpredictable-command | tq -x -i json
```

`-x` or `--proxy-on-error` handles sources whose format is uncertain. `tq`
keeps the bounded source before parsing it. If parsing rejects the source,
`tq` writes the original bytes unchanged and treats that source as successful.
Resource, I/O, query, runtime, and output errors still fail.

For multiple sources, the fallback applies to each source separately. With
`--slurp`, a rejected source proxies the complete ordered source set because
slurp evaluates the set as one input. `--proxy-on-error` cannot be used with
`--stream-errors`.

By default, each structured result contains an ASCII RS byte, one canonical
TOON document, and LF. This framing distinguishes zero, one, and many results.
If a later result fails, earlier complete records remain valid. Use
`--output-format json` for jq-style JSON or `--output-format jsonl` for one
compact, LF-terminated JSON value per result. `-r` writes raw strings, `-j`
joins raw output, and `--unframed` is available when the query must return
exactly one TOON value.

## Streaming and memory

`--stream` emits jq-compatible `[path,value]` records and container-end
`[path]` records from JSON, JSON Lines, or TOON decoder events. JSON Lines resets
the root path for every physical record. YAML and JSON5 decode one document at a
time. On large inputs, streaming avoids retaining the whole document:

```console
tq --stream --input-format json \
  'select(length == 2 and (.[0] | length) == 1)' buildings.geojson
```

`--explain` and `--explain-json` show the query plan and its memory limits.
Plans fall into six classes: transcode, event, subtree, document, whole-input,
and blocking.

For the identity query `.` with JSON or strict TOON input and canonical TOON
output, the planner selects `transcode`. This path bypasses jq bytecode and uses
one bounded preparation arena. The JSON decoder stages object members as they
arrive. If a later member repeats a name, transcode rejects and discards the
current record instead of applying jq's last-value normalization. Unframed
output stays empty as well.

Safe key folding, sorted keys, raw or joined output, slurp, explicit jq stream
mode, non-TOON output, and non-identity queries use the existing plans.

A document plan keeps one decoded document, while slurp keeps every input
document. Sorting, uniqueness, and final reductions need blocking state. A fold
keeps one immutable accumulator plus bounded evaluator state. Transcode may
write array preparation, staged sequence records, or atomic unframed output to
private temporary files. Sequence output preserves completed records before a
later error. Unframed output publishes nothing until exactly one successful
result is known.

The resource controls are `--max-input-bytes`, `--max-depth`,
`--max-token-bytes`, `--max-line-bytes`, `--max-lookahead-bytes`,
`--max-vm-steps`, `--max-results`, `--max-output-bytes`,
`--prepare-memory-bytes`, and `--max-spool-bytes`. The evaluator checks for
SIGINT between units of work. A closed downstream pipe exits successfully.
`--report-file` records transcode preparation high-water bytes, object-index
spills, array preparations, spool bytes written and replayed, and the final
resource outcome. `--explain-json` includes the identity proof, decoder
duplicate policy, commitment mode, retained state, configured limits, and any
deterministic transcode fallback reason. JSON explanations also state the
duplicate-key limitation.

Each transcode report records the selected input format, whether selection came
from an override or detection, each bounded probe's inspected and commitment
bytes, and rejected probe candidates. Input staging counters are zero for the
single-pass transcode plan.

## Compatibility and benchmarks

The compatibility suite sends the same cases through jq, yq, and tq, then
compares result order, result count, failures, and raw framing.
See the [format compatibility matrix](docs/formats.md) for native input and
output support in each tool.

```console
./scripts/run-campaign.sh compatibility smoke
./scripts/run-campaign.sh compatibility full
target/release/tq compatibility
```

Benchmarks use the repository's small, medium, and large datasets in
JSON, YAML, and TOON. Before timing starts, the runner generates each
representation and checks that every format has the same ordered values.
Reports record time, CPU, peak RSS, throughput, output size, machine and tool
identity, and every incorrect or failed run. The runner does not resize inputs
to manufacture target sizes.

Benchmark commands require elevated child-process inspection permissions and
must run outside restricted sandboxes. On macOS, every authoritative peak RSS
sample comes from `/usr/bin/time -l`; a run with unavailable RSS must be rerun
with the required permissions.

```console
./scripts/run-campaign.sh benchmark smoke
./scripts/run-campaign.sh benchmark standard
./scripts/run-campaign.sh benchmark large  # opt-in; uses the natural ~1 GB-class corpus
```

### Benchmark snapshot, 2026-08-30

The runner collected these results from release builds on an Apple M4 Pro with
14 logical CPUs. The rapid campaign used the cached `usgs-all-month`
corpus with 11,081 records and one timed sample per row. The parallel campaign
used the cached 1.12 GB Microsoft US building-footprint GeoJSON, two warmups,
and three measured samples for each worker count. The tables report medians.
Peak RSS came from macOS `/usr/bin/time -l`. `U` means that the adapter does not
support the case.

Rapid wall time is in milliseconds. Lower is better.

| Scenario | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| event-stream | 330.7 | U | U | 580.5 | U | 610.1 |
| identity-reencode | 324.3 | 539.8 | 718.9 | 274.6 | 326.3 | 336.9 |
| parse-discard | 113.5 | 177.5 | 345.4 | 127.3 | 287.0 | 171.5 |
| path-update | 343.1 | 557.6 | 705.9 | 160.5 | 336.3 | 238.8 |
| scalar-extraction | 114.5 | 234.1 | 381.9 | 111.5 | 274.3 | 174.8 |

Rapid CPU time is user plus system time in milliseconds. Lower is better.

| Scenario | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| event-stream | 240 | U | U | 490 | U | 540 |
| identity-reencode | 290 | 660 | 930 | 230 | 260 | 270 |
| parse-discard | 60 | 200 | 480 | 60 | 220 | 110 |
| path-update | 280 | 670 | 940 | 110 | 270 | 160 |
| scalar-extraction | 60 | 220 | 500 | 60 | 220 | 110 |

Rapid peak RSS is in MiB. Lower is better.

| Scenario | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| event-stream | 4.0 | U | U | 5.2 | U | 5.2 |
| identity-reencode | 65.3 | 503.5 | 661.4 | 5.1 | 157.1 | 5.2 |
| parse-discard | 61.3 | 290.9 | 399.6 | 71.0 | 157.5 | 72.8 |
| path-update | 66.4 | 500.4 | 671.0 | 72.1 | 157.5 | 73.0 |
| scalar-extraction | 61.3 | 290.1 | 399.4 | 70.4 | 156.1 | 72.7 |

In this run, `tq` beats `jq` on identity re-encoding, path updates, and scalar
extraction. `tq` uses far less memory than `yq` in every document case. Event
streaming is still slower: `tq` JSON takes 1.8x as long as `jq` on this corpus,
while keeping RSS near 5 MiB.

The parallel run compared a single-process `jq` baseline with `tq` using
Rayon. Every run produced the same correctness digest.

| Tool and workers | Wall time (s) | CPU time (s) | Peak RSS (MiB) | Speedup vs jq |
| --- | ---: | ---: | ---: | ---: |
| jq, 1 process | 28.26 | 25.79 | 14,015.3 | 1.00x |
| tq, 1 worker | 12.18 | 15.39 | 1,383.5 | 2.32x |
| tq, 4 workers | 4.86 | 19.52 | 1,472.3 | 5.81x |
| tq, 8 workers | 4.05 | 21.14 | 1,447.2 | 6.98x |
| tq, 14 workers | 3.93 | 22.32 | 1,452.8 | 7.19x |

Parallel decoding cuts wall time by 86% against `jq` at 14 workers, but this
workload still uses about 1.4 GiB. Moving from eight to 14 workers saves only
0.12 seconds while increasing CPU time.

Raw reports and replay data are in the separate `tq-benchmarks` checkout.

The case-first workflow is in [CONTRIBUTING.md](CONTRIBUTING.md). See the
[benchmark guide](benchmarks/README.md) for campaign details, the
[performance policy](docs/performance-baseline.md) for the accepted local
baseline, and the [compatibility guide](docs/compatibility.md) for syntax,
framing, limits, and known differences.

## jq user filters and modules

Parameterized `def` filters support lexical capture, filter and value
parameters, generator cardinality, shadowing, and recursion on tq's bounded
managed call stack. Modules load only from explicit roots:

```console
tq -L ./jq-libs 'import "metrics" as m; m::normalize' input.json
tq -L ./jq-libs 'include "shared"; shared_filter' input.json
```

Repeat `-L` to search multiple roots in order. The loader rejects absolute
paths and `..` escapes after canonicalization. `--explain` and
`--explain-json` include each loaded path and SHA-256 digest. Modules can
declare constant metadata with `module {...};`, which jq's `modulemeta` filter
reads.

## Regex, dates, and platform data

The Unicode-aware `test`, `match`, `capture`, `scan`, `split`, `splits`, `sub`,
and `gsub` built-ins use a bounded linear-time regex engine. UTC parsing,
formatting, broken-down time, and epoch conversion support jq's date arrays for
the documented range from year 0000 through 9999.

Environment and platform data are opt-in. `--allow-environment` enables `env`.
`--allow-platform` enables `now`, local timezone conversion, and input metadata.
See [the compatibility policy](docs/jq-regex-date-platform.md) for engine
differences, limits, redaction, and release-host classifications.

## Current boundaries

`tq` does not implement labels, breaks, or many less common jq CLI switches. It
reports them as unsupported capabilities.

The numeric model preserves accepted input literals as written. Arithmetic uses
jq-compatible binary64 behavior when needed. Digit, exponent-expansion, and
index limits return resource or range errors instead of silently losing data.
These errors and TOON sequence framing are known differences from jq.

Licensed under MIT.
