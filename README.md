# tq

`tq` runs jq 1.8.x-style queries over TOON, YAML, JSON, and JSON Lines. It writes
TOON Text Sequences by default and can stream JSON, JSON Lines, and TOON without
loading the complete input.

Most everyday jq filters work, including navigation, pipes, generators,
conditionals, operators, variables, path updates, user filters, modules, and
the common built-ins. Arrays and ordered objects retain their jq semantics.
The language also includes `empty`, `error`, optional access, `try/catch`,
`reduce`, and `foreach`. See [jq compatibility](docs/compatibility.md) for the
exact boundary.

## Build and use

Rust 1.87 or newer is required.

```console
cargo build --release
target/release/tq '.features[] | {id, magnitude: .properties.mag}' feed.json
```

With no file argument, `tq` reads stdin. It processes files and `-` in argument
order. Recognized `.toon`, `.yaml`, `.yml`, `.json`, `.jsonl`, and `.ndjson`
extensions select the parser. Other sources use bounded content detection. For
ambiguous input, select a parser with
`--input-format toon|yaml|json|jsonl`. `ndjson` is an alias for `jsonl`.

```console
printf 'name: Ada\nactive: true\n' | tq '.name'
tq --input-format json --output-format json -c '.features | length' feed.json
tq --input-format yaml -r '.people[].name' people.yaml
tq -i jsonl -o jsonl '.event' events.ndjson
unpredictable-command | tq -x -i json
```

`-x` or `--proxy-on-error` handles sources whose format is uncertain. `tq`
retains the bounded source before parsing it. If parsing rejects the source,
`tq` writes the original bytes unchanged and treats that source as successful.
Resource, I/O, query, runtime, and output errors still fail.

For multiple sources, the fallback applies to each source separately. With
`--slurp`, one rejected source proxies the complete ordered source set because
slurp treats the set as one combined input. `--proxy-on-error` cannot be used
with `--stream-errors`.

By default, each structured result contains an ASCII RS byte, one canonical
TOON document, and LF. This framing distinguishes zero, one, and many results.
If a later result fails, earlier complete records remain valid. Use
`--output-format json` for jq-style JSON or `--output-format jsonl` for one
compact, LF-terminated JSON value per result. `-r` writes raw strings, `-j`
joins raw output, and `--unframed` is available when the query must return
exactly one TOON value.

## Streaming and memory

`--stream` creates jq-compatible `[path,value]` records and container-end
`[path]` records from JSON, JSON Lines, or TOON decoder events. JSON Lines resets
the root path for every physical record. For YAML, `tq` decodes one document at
a time. On large inputs, streaming avoids retaining the complete document:

```console
tq --stream --input-format json \
  'select(length == 2 and (.[0] | length) == 1)' buildings.geojson
```

`--explain` and `--explain-json` show the query plan and its memory limits.
Plans fall into five classes: event, subtree, document, whole-input, and
blocking. A document plan retains one decoded document, while slurp retains
every input document. Sorting, uniqueness, and final reductions need blocking
state. A fold keeps one immutable accumulator plus bounded evaluator state.

The resource controls are `--max-input-bytes`, `--max-depth`,
`--max-token-bytes`, `--max-line-bytes`, `--max-lookahead-bytes`,
`--max-vm-steps`, `--max-results`, `--max-output-bytes`,
`--prepare-memory-bytes`, and `--max-spool-bytes`. The evaluator checks for
SIGINT between units of work. A closed downstream pipe exits successfully.

## Compatibility and benchmarks

The compatibility suite sends the same cases through jq, yq, and tq, then
compares result order, result count, failures, and raw framing.

```console
make compatibility-smoke
make compatibility-full
target/release/tq compatibility
```

Benchmarks use the repository's natural small, medium, and large datasets in
JSON, YAML, and TOON. Before timing starts, the runner generates each
representation and checks that every format has the same ordered values.
Reports record time, CPU, peak RSS, throughput, output size, machine and tool
identity, and every incorrect or failed run. The runner does not resize inputs
to manufacture target sizes.

```console
make benchmark-smoke
make benchmark-standard
make benchmark-large       # opt-in; uses the natural ~1 GB-class corpus
```

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

Labels, breaks, and many less common jq CLI switches are not implemented. `tq`
reports them as unsupported capabilities.

The numeric model preserves accepted input literals as written until arithmetic
needs jq-compatible binary64 behavior. Digit, exponent-expansion, and index
limits return resource or range errors instead of silently losing data. These
errors and TOON sequence framing are known differences from jq.

Licensed under MIT.
