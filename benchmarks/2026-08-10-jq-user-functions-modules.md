# jq user functions and modules evidence — 2026-08-10

This artifact records compatibility and natural-data evidence for
`jq-user-functions-modules`.

## Compatibility

`make compatibility-full` completed all 178 cases with status
`ObservedDifferences`: 837 observations and the same 190 established broad
jq/yq, numeric, and framing differences. The 12 new cases reported no jq/tq
difference. They cover definitions, filter and value parameters, Cartesian
generator order, lexical capture and shadowing, recursion, imports, includes,
metadata, cycles, confinement, unknown calls, and wrong arity.

## Natural-data user-filter call

The standard-tier `benchmark.user-filter-call` workload executes:

```jq
def magnitude(f): f; .features[] | magnitude(.properties.mag)
```

It used the frozen USGS all-month GeoJSON source with 10,656 logical records
and 7,567,881 JSON bytes. One measured sample per applicable adapter passed
ordered semantic comparison:

| Adapter | Input | Wall time |
| --- | --- | ---: |
| jq | JSON | 116.486 ms |
| tq | JSON | 148.163 ms |
| tq | YAML | 320.692 ms |
| tq | TOON | 174.893 ms |

yq is not applicable because its user-filter syntax and semantics differ. The
machine identity was
`9b03ec81f92f9aa96e8ad589f791698bf5a9531a127d8b46ffad9928452b4e9f`;
jq was `jq-1.8.2-8-g603db3f` and tq was a release build of `0.1.0`. The command
was:

```console
TQ_BIN="$PWD/target/release/tq" cargo run -p tq-test-support --bin tq-bench --release -- \
  run --profile standard --origin frozen \
  --manifest benchmarks/.work/corpus/campaigns/2026-08-07T17-29-44.995859Z/usgs-all-month/manifest.json \
  --output benchmarks/.work/user-functions-modules-standard.json \
  --max-samples 1 --case benchmark.user-filter-call
```

The macOS campaign sampler did not publish RSS. Matching one-shot
`/usr/bin/time -l` runs measured peak resident sets of 135,954,432 bytes
(129.66 MiB) for tq and 61,423,616 bytes (58.58 MiB) for jq. One sample is
correctness and resource-class evidence, not a stable regression baseline.

## Managed resource behavior

- User calls, returns, value-argument combinations, lazy filter closures, and
  recursive references execute through explicit VM frames. `--max-depth`
  bounds both module count and the corresponding VM call/path envelopes;
  `--max-vm-steps` bounds total call and generator work.
- Large generator iteration advances through one managed iterator task rather
  than queuing every result. The standard run exposed and then verified this
  property with all 10,656 feature results under the default fork limit.
- Module roots are explicit and canonicalized. Absolute or parent traversal,
  symlink escape, cycles, module-count exhaustion, per-file byte exhaustion,
  invalid UTF-8, and non-constant metadata fail during compilation before any
  input is consumed.
- The module cache keys canonical paths and records SHA-256 content identity.
  Dropping a failed or completed compilation releases cached ASTs and module
  bytes; module loading creates no temporary files.
