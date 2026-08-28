# tq

`tq` is a Rust query tool for TOON, YAML, JSON, and JSON Lines. Its command line
and query language follow jq 1.8.x, but structured output defaults to TOON Text
Sequence. It can stream JSON, JSON Lines, and TOON without keeping the whole
input in memory.

The current release covers the jq language most filters need: navigation,
pipes and generators, arrays and ordered objects, conditionals, operators,
variables, path updates, user filters, modules, and the common built-ins. It
also supports `empty`, `error`, optional access, `try/catch`, `reduce`, and
`foreach`. See [jq compatibility](docs/compatibility.md) for the exact boundary.

## Build and use

Rust 1.87 or newer is required.

```console
cargo build --release
target/release/tq '.features[] | {id, magnitude: .properties.mag}' feed.json
```

With no file argument, `tq` reads stdin. It processes multiple files and `-` in
the order given. Recognized `.toon`, `.yaml`, `.yml`, `.json`, `.jsonl`, and
`.ndjson` extensions select their parser. Other sources use bounded content
detection. Use `--input-format toon|yaml|json|jsonl` for ambiguous input or to
force one parser. `ndjson` is an alias for `jsonl`.

```console
printf 'name: Ada\nactive: true\n' | tq '.name'
tq --input-format json --output-format json -c '.features | length' feed.json
tq --input-format yaml -r '.people[].name' people.yaml
tq -i jsonl -o jsonl '.event' events.ndjson
```

Each structured result is framed as an ASCII RS byte, one canonical TOON
document, and LF. The framing makes zero, one, and many results distinct. If a
later result fails, earlier complete records remain valid. Use
`--output-format json` for jq-style JSON, or `--output-format jsonl` for one
compact LF-terminated JSON value per result. Use `-r` for raw strings, `-j` to
join raw output, or `--unframed` when the query must return exactly one TOON
value.

## Streaming and memory

`--stream` creates jq-compatible `[path,value]` records and container-end
`[path]` records from JSON, JSON Lines, or TOON decoder events. JSON Lines resets
the root path for every physical record. For YAML, `tq` decodes one document at
a time. Use streaming to keep memory bounded on large inputs:

```console
tq --stream --input-format json \
  'select(length == 2 and (.[0] | length) == 1)' buildings.geojson
```

Use `--explain` or `--explain-json` to inspect the query plan and its memory
limits. Plans fall into five classes: event, subtree, document, whole-input,
and blocking. A document plan keeps one decoded document. Slurp keeps every
input document. Sorting, uniqueness, and final reductions keep blocking state.
A fold keeps one immutable accumulator plus bounded evaluator state.

Resource controls include `--max-input-bytes`, `--max-depth`,
`--max-token-bytes`, `--max-line-bytes`, `--max-lookahead-bytes`,
`--max-vm-steps`, `--max-results`, `--max-output-bytes`,
`--prepare-memory-bytes`, and `--max-spool-bytes`. The evaluator checks for
SIGINT between units of work. A closed downstream pipe exits successfully.

## Compatibility and benchmarks

The compatibility suite runs the same cases through jq, yq, and tq. It compares
result order, result count, failures, and raw framing.

```console
make compatibility-smoke
make compatibility-full
target/release/tq compatibility
```

Benchmark campaigns use natural small, medium, and large datasets in JSON,
YAML, and TOON. Before timing starts, the runner generates each representation
and checks that all formats have the same ordered values. Reports record time,
CPU, peak RSS, throughput, output size, machine and tool identity, and every
incorrect or failed run. The runner never resizes natural inputs.

```console
make benchmark-smoke
make benchmark-standard
make benchmark-large       # opt-in; uses the natural ~1 GB-class corpus
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the case-first workflow and
[the benchmark guide](benchmarks/README.md) for campaign details. The
[performance policy](docs/performance-baseline.md) defines the accepted local
baseline. The [compatibility guide](docs/compatibility.md) documents syntax,
framing, limits, and known differences.

## jq user filters and modules

Parameterized `def` filters support lexical capture, filter and value
parameters, generator cardinality, shadowing, and recursion on tq's bounded
managed call stack. Modules are loaded only from explicit roots:

```console
tq -L ./jq-libs 'import "metrics" as m; m::normalize' input.json
tq -L ./jq-libs 'include "shared"; shared_filter' input.json
```

Repeat `-L` to search multiple roots in order. The loader rejects absolute
paths and `..` escapes after canonicalization. `--explain` and
`--explain-json` include each loaded path and SHA-256 digest. A module may
declare constant metadata with `module {...};`; jq's `modulemeta` filter reads
it.

## Regex, date, and governed platform built-ins

Unicode-aware `test`, `match`, `capture`, `scan`, `split`, `splits`, `sub`, and
`gsub` use a bounded linear-time regex engine. UTC parsing, formatting, broken
down time, and epoch conversion support jq's date arrays across the documented
year 0000 through 9999 range.

Ambient data is opt-in. Use `--allow-environment` for `env`. Use
`--allow-platform` for `now`, local timezone conversion, and input metadata.
See [the compatibility policy](docs/jq-regex-date-platform.md) for engine
differences, limits, redaction, and release-host classifications.

## MVP boundaries

Labels, breaks, and many less common jq CLI switches are deferred. `tq` reports
them as unsupported capabilities.

The numeric model preserves the spelling of accepted input literals until
arithmetic needs jq-compatible binary64 behavior. Digit, exponent-expansion,
and index limits return resource or range errors instead of silently losing
data. Those errors and TOON sequence framing are known differences from jq.

Licensed under MIT or Apache-2.0.
