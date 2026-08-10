# jq reduce/foreach evidence — 2026-08-10

This artifact records compatibility and natural-data evidence for the
`jq-reduce-foreach` change.

## Compatibility

`make compatibility-full` completed all 163 cases with status
`ObservedDifferences`. The campaign included seven fold cases covering empty
and multi-value generators, update and extraction multiplicity, lexical scope,
and partial output before a later error. None of those cases reported a jq/tq
difference; the campaign's 190 observations remain in the established broad
jq/yq and numeric/framing difference classes.

## Natural-data reduction

The standard-tier `benchmark.numeric-reduction` workload now executes:

```jq
reduce (.features[].properties.mag // empty) as $mag (0; . + $mag)
```

It used the frozen USGS all-month GeoJSON source with 10,656 logical records
and 7,567,881 JSON bytes. One measured sample per adapter passed semantic
comparison:

| Adapter | Input | Wall time |
| --- | --- | ---: |
| jq | JSON | 94.009 ms |
| yq | JSON | 216.281 ms |
| yq | YAML | 383.526 ms |
| tq | JSON | 119.374 ms |
| tq | YAML | 243.315 ms |
| tq | TOON | 147.325 ms |

The machine identity was
`9b03ec81f92f9aa96e8ad589f791698bf5a9531a127d8b46ffad9928452b4e9f`.
The detailed local report is
`benchmarks/.work/jq-reduce-foreach-standard.json`.

The macOS benchmark sampler did not publish RSS, so matching one-shot
`/usr/bin/time -l` runs supplied exact peak values: tq used 132,726,784 bytes
(126.58 MiB), while jq used 61,210,624 bytes (58.37 MiB). tq reported a
`blocking-document` plan, 42,632 VM steps, one result, value-stack high water
1, call-stack high water 7, and no pending fork or path frames.

## Memory classes

- `reduce` is a blocking-document operation: it retains the decoded document,
  one immutable accumulator, and bounded managed fold/call state until the
  final value is available.
- `foreach` uses the same bounded accumulator/frame model but releases each
  extraction result after its update completes. Earlier complete output frames
  remain valid if a later update, extraction, or generator item fails.
- Accumulator replacement uses shared immutable `Value` nodes. Unchanged
  strings, arrays, and objects remain shared; updates path-copy only changed
  structure.
- Ordinary folds remain document/blocking plans. Explicit decoder-event mode
  admits a fold only when all blocking/subtree causes belong to fold state
  itself; a body-level blocking constructor, mutation, or document effect is
  rejected before input.
