# tq

`tq` is a jq-shaped query tool for TOON, YAML, and JSON, written in Rust. It
uses jq 1.8.x as the language target, emits TOON Text Sequences by default, and
can stream JSON or TOON inputs without retaining the complete document.

The MVP implements identity and literals; field, computed, index, slice, and
iteration navigation; pipes and generators; array and ordered-object
construction; conditionals and operators; lexical and CLI variables; the core
type, collection, selection, conversion, range, ordering, and aggregation
built-ins; `empty`, `error`, optional access, and `try/catch`; and jq-style path
updates.

## Build and use

Rust 1.85 or newer is required.

```console
cargo build --release
target/release/tq '.features[] | {id, magnitude: .properties.mag}' feed.json
```

Input is read from stdin when no file is supplied. Multiple files and `-` are
processed in argument order. Best-effort detection tries TOON, then YAML, then
JSON; use `--input-format toon|yaml|json` when the syntax is ambiguous or an
exact parser is required.

```console
printf 'name: Ada\nactive: true\n' | tq '.name'
tq --input-format json --output-format json -c '.features | length' feed.json
tq --input-format yaml -r '.people[].name' people.yaml
```

Structured output defaults to an RFC 7464-style sequence whose records are an
ASCII RS byte, one canonical TOON document, and LF. This keeps zero, one, and
many results unambiguous and preserves complete earlier records if evaluation
later fails. Use `--output-format json` for jq-style JSON texts, `-r` for raw
strings, `-j` to join raw output, or `--unframed` when exactly one TOON result is
required.

## Streaming and memory

`--stream` forms jq-compatible `[path,value]` records and container-end
`[path]` records directly from JSON or TOON decoder events. YAML remains a
document-at-a-time input. Explicit streaming is the bounded-memory mode for
large sources:

```console
tq --stream --input-format json \
  'select(length == 2 and (.[0] | length) == 1)' buildings.geojson
```

Use `--explain` or `--explain-json` to see whether a query uses an event,
subtree, document, whole-input, or blocking plan, what it retains, and its
resource envelope. Document plans retain one decoded input document. Slurp
retains all input documents. Sorting, uniqueness, and aggregate operators also
retain blocking state. Automatic conversion of ordinary document queries into
stream plans is deliberately deferred.

Resource controls include `--max-input-bytes`, `--max-depth`,
`--max-token-bytes`, `--max-line-bytes`, `--max-lookahead-bytes`,
`--max-vm-steps`, `--max-results`, `--max-output-bytes`,
`--prepare-memory-bytes`, and `--max-spool-bytes`. SIGINT is handled
cooperatively and a closed downstream pipe exits successfully.

## Compatibility and benchmarks

The compatibility suite executes the same cases through jq, yq, and tq and
compares ordered JSON-model results, cardinality, failures, and raw framing.

```console
make compatibility-smoke
make compatibility-full
target/release/tq compatibility
```

Benchmark campaigns cover natural small, medium, and large datasets in JSON,
YAML, and TOON. Input representations are generated before timing and checked
for ordered semantic equivalence. Reports preserve wall time, time to first
result, CPU, peak RSS, throughput, output bytes, host/tool/corpus identity, and
incorrect or failed outcomes. Natural inputs are never resized.

```console
make benchmark-smoke
make benchmark-standard
make benchmark-large       # opt-in; uses the natural ~1 GB-class corpus
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the case-first workflow and
`benchmarks/README.md` for campaign details. The accepted local numbers and
regression policy are reviewed in `docs/performance-baseline.md`; detailed
syntax, framing, limits, and known-difference guidance lives in
`docs/compatibility.md`.

## Intentional MVP boundaries

User-defined functions and modules, `reduce`/`foreach`, labels and breaks,
recursive descent and interpolation, regex and date functions, environment or
platform I/O, automatic stream planning, and the long tail of jq CLI switches
are deferred with stable unsupported-capability diagnostics.

The numeric model is a hybrid: accepted input literals preserve arbitrary
precision spelling until arithmetic requires jq-compatible binary64 behavior.
Explicit digit, exponent-expansion, and index envelopes produce resource or
range diagnostics rather than silently losing data. These envelope failures,
and TOON versus JSON sequence framing bytes, are intentional jq divergences.

Licensed under MIT or Apache-2.0.
