# jq recursive descent and interpolation evidence, 2026-08-10

This report records compatibility, traversal timing, and resource limits for
`jq-recursive-descent-interpolation`.

## Compatibility

`make compatibility-full` completed all 169 cases with status
`ObservedDifferences`: 793 observations and the same 190 established broad
jq/yq, numeric, and framing differences. The eight new recursive-descent and
interpolation cases reported no jq/tq difference. They cover depth-first
insertion order, scalar and deep inputs, conversion and escaping, nested
filters, Cartesian generator ordering, and partial output before a later
error.

## Traversal smoke benchmark

The focused command was:

```console
TQ_BIN="$PWD/target/release/tq" cargo run -p tq-test-support --bin tq-bench -- \
  run --profile smoke \
  --output benchmarks/.work/recursive-interpolation-smoke.json \
  --max-samples 1 --case benchmark.recursive-scalars
```

It ran `.. | scalars` over the generated 99-byte GeoJSON smoke document (one
logical record) and passed ordered semantic comparison. jq was
`jq-1.8.2-8-g603db3f`; tq was `0.1.0` built in release mode. The local machine
identity was
`9b03ec81f92f9aa96e8ad589f791698bf5a9531a127d8b46ffad9928452b4e9f`.

| Adapter | Input | Wall time | First result | Output bytes |
| --- | --- | ---: | ---: | ---: |
| jq | JSON | 27.134 ms | 10.700 ms | 42 |
| tq | JSON | 36.950 ms | 36.879 ms | 40 |
| tq | YAML | 37.014 ms | 36.942 ms | 40 |
| tq | TOON | 37.112 ms | 8.058 ms | 40 |

The two-byte jq/tq output-size difference is framing only; the semantic
sequence matched. One sample is correctness and smoke evidence, not a stable
performance claim. The macOS sampler did not publish peak RSS.

## Managed resource guidance

A direct report over the checked-in 857-byte USGS all-hour fixture emitted 17
scalars and 465 output bytes using 51 VM steps. High-water observations were
one value slot, two call frames, four traversal path frames, and one pending
fork.

- Recursive descent is a document plan: the decoded input remains retained,
  while the traversal cursor holds one shallow shared value handle per active
  depth level. It does not recurse on the native Rust stack.
- `--max-depth` bounds traversal cursor depth, `--max-vm-steps` bounds visits
  and continuation work, and cancellation is checked before every managed
  task. `--max-results` and `--max-output-bytes` stop expansion while
  preserving already completed output frames.
- Interpolation evaluates embedded generators from the last segment toward
  the first so earlier generators vary fastest, matching jq. Active
  combinations use bounded VM continuations/forks; one materialized string is
  checked against the output-byte limit before its final allocation.
- A depth, work, fork, result, output, or cancellation failure releases all
  pending cursor and interpolation continuation state when the VM stops or is
  dropped.
