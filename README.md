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

### Benchmark snapshot, 2026-09-05

Ironhide ran release-built tq 0.2.0 at commit `f5c008b`, jq 1.8.1, and
yq 4.53.2 on Linux x86_64 with an AMD Ryzen 7 7700 and 16 logical CPUs.
The tables below use the largest standard snapshot, `usgs-all-month`, with
11,274 records. Each timed row passed an ordered semantic correctness check,
then ran two warmups and ten measured samples.

Column formats identify **input**, not output. These workloads use jq JSON,
yq compact JSON, and tq TOON output, so serialization costs and byte counts
differ. The JSON, YAML, and TOON inputs contain 8,013,185, 8,001,913, and
9,490,992 bytes respectively. YAML uses compact JSON-subset syntax here,
not block YAML. `U` means excluded by the benchmark catalog; it does not by
itself establish a missing tool capability.

Median wall time is in milliseconds. Lower is better.

| Scenario | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| blocking-sort | 125.9 | 361.7 | 1021.5 | 86.0 | 203.5 | 158.4 |
| event-stream | 303.6 | U | U | 781.4 | U | 815.8 |
| format-json | 272.4 | U | U | 200.0 | 237.9 | 236.4 |
| identity-reencode | 237.3 | 779.3 | 1382.7 | 323.0 | 239.1 | 342.1 |
| object-construction | 161.7 | 24654.2 | 23844.0 | 937.2 | 976.3 | 981.3 |
| parse-discard | 125.7 | 282.2 | 908.2 | 124.6 | 201.6 | 198.0 |
| path-update | 237.8 | 767.8 | 1359.9 | 163.2 | 240.0 | 238.3 |
| recursive-scalars | 382.4 | U | U | 220.0 | 276.9 | 275.4 |
| scalar-extraction | 123.5 | 282.4 | 944.1 | 126.7 | 201.9 | 200.1 |
| string-reduction | 123.8 | 359.6 | 911.8 | 344.7 | 236.7 | 237.2 |

Reported peak RSS is in MiB, taking the maximum across measured samples.
Lower is better.

| Scenario | jq JSON | yq JSON | yq YAML | tq JSON | tq YAML | tq TOON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| blocking-sort | 61.0 | 331.4 | 1303.3 | 19.4 | 88.4 | 7.8 |
| event-stream | 4.0 | U | U | 7.0 | U | 6.9 |
| format-json | 63.7 | U | U | 66.5 | 88.4 | 82.6 |
| identity-reencode | 64.8 | 479.6 | 1363.1 | 14.8 | 88.3 | 14.5 |
| object-construction | 65.8 | 490.9 | 1475.8 | 71.2 | 88.4 | 82.6 |
| parse-discard | 60.3 | 293.3 | 1298.0 | 66.4 | 88.3 | 82.8 |
| path-update | 64.9 | 486.0 | 1362.6 | 66.6 | 88.3 | 82.7 |
| recursive-scalars | 64.9 | U | U | 66.5 | 88.5 | 82.8 |
| scalar-extraction | 60.3 | 293.4 | 1301.5 | 66.5 | 88.3 | 82.6 |
| string-reduction | 60.9 | 333.6 | 1229.6 | 68.2 | 88.4 | 82.5 |

On this snapshot, tq beats yq in time and memory on all 13 shared JSON
workloads and all 13 shared YAML workloads. Against jq, tq JSON is faster
on sorting, path updates, and recursive scalar traversal. Identity reencoding
takes 1.36x jq time but uses 0.23x its reported RSS. Event streaming,
object construction, and string reduction miss the 2x time target at
2.57x, 5.79x, and 2.78x respectively. This is not a universal performance pass.

The formatter follow-up filled 56 previously excluded YAML/TOON rows, with
112 timed rows and no incorrect results across seven queries and four USGS
snapshots. A separate startup case also passed for all three tq input formats.
All eight `format-*` cases now cover tq JSON, YAML, and TOON. They exercise
query operators such as `@json`, `@csv`, and `@uri`, not the CLI output
serializer. The `format-json` row above uses the fresh follow-up measurements;
other rows retain the original run.

#### Early-break correctness follow-up

The original standard campaign had four incorrect tq JSON rows for
`label $out | (.features[] | .id, break $out)`, one per USGS snapshot.
A pipe/comma precedence fix makes the query stop after its first ID.
A separately rebuilt candidate passed all four gates on the same frozen
inputs, with fresh jq references and 120 measured samples in total.

| Snapshot | tq correctness | jq ms | tq ms | tq/jq RSS |
| --- | --- | ---: | ---: | ---: |
| all-hour | Pass | 47.41 | 47.51 | 1.693x |
| all-day | Pass | 49.79 | 50.74 | 1.849x |
| all-week | Pass | 48.03 | 48.56 | 1.285x |
| all-month | Pass | 121.91 | 124.09 | 1.105x |

All four meet the strict 2x time target. Week and month meet the strict
1.5x RSS target; hour and day do not. This follow-up is not a full
remeasurement of the patched binary. The original campaign's four incorrect
rows and six empty-result resource-limit classifications remain in its raw
report.

#### Native formats and measurement limits

On 131,072 synthetic records, native CSV and TSV extraction take 121.3 and
121.8 ms with tq versus 2,412.3 and 2,431.4 ms with yq, about 20x faster.
Reported tq RSS is about 6.5 MiB versus yq's 598–599 MiB. JSON sequence
extraction takes 230.0 ms versus jq's 121.3 ms, meeting the 2x time target
but missing the 1.5x RSS target at 1.63x.

Wall time includes startup, output capture, and RSS-sampler shutdown.
Short runs cluster near 48 ms, so small differences there are not reliable
execution-speed differences. Reported RSS combines GNU time and sampled
process-group RSS; it is not necessarily executable-only memory.
CPU fields were unavailable because the runner did not parse GNU time's
verbose CPU labels, so this snapshot has no CPU comparison.

These results are not directly comparable with the earlier Apple M4 Pro
run: host, tool versions, corpus, and YAML representation differ. The
roughly 1 GB parallel-decoding benchmark was not rerun on Ironhide.

Raw reports and the detailed `2026-09-05-ironhide-tq-yq-jq.md` analysis remain
in the separate `tq-benchmarks` checkout:

- `.work/ironhide-f5c008b-standard.json`
- `.work/ironhide-f5c008b-format-coverage.json`
- `.work/ironhide-f5c008b-format-startup.json`
- `.work/ironhide-label-early-break-fixed.json`

The original tq executable SHA-256 is
`c645e35a7640f6af8f605975aa593cdf3930745d9b5afb1885cf034651eeef63`.
The early-break candidate is
`1bda5710e878a5950481b7a250ef18c6244d889db5375dfeedf3868ba4d4d3aa`.

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

Environment and ambient platform data are opt-in. `--allow-environment` enables
both `env` and jq's `$ENV` startup snapshot. `$__loc__` is always available and
returns `{file, line}` for its location in the query source. Inline filters use
`<top-level>`; filter files and modules retain their path identities.
`--allow-platform` enables `now`, local timezone conversion, and `input_filename`.
Decoder-owned `input_line_number` context is available without a capability flag.
See [the compatibility policy](docs/jq-regex-date-platform.md) for engine
differences, limits, redaction, and release-host classifications.

## Current boundaries

`tq` supports lexical `label` and `break`, including early exit from a
generator. Some less common jq capabilities remain unsupported; see the
[compatibility guide](docs/compatibility.md) for the supported CLI switches
and known differences.

The numeric model preserves accepted input literals as written. Arithmetic uses
jq-compatible binary64 behavior when needed. Digit, exponent-expansion, and
index limits return resource or range errors instead of silently losing data.
These errors and TOON sequence framing are known differences from jq.

Licensed under MIT.
