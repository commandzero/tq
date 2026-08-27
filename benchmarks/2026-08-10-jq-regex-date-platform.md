# jq regex, date, and platform evidence, 2026-08-10

## Compatibility campaign

The full jq/yq/tq campaign executed 194 catalog cases and 911 observations.
All portable regex, UTC date, allowed environment-shape, input-filename,
local-year, and clock-type cases matched jq semantically. Two reviewed
differences remain:

- `regex.unsupported-lookaround`: jq's Oniguruma engine accepts look-around;
  tq's linear-time Rust regex engine returns a stable unsupported diagnostic.
- `date.range-error`: the reference host formats year 10000; tq's portable Jiff
  contract stops at year 9999 with a stable range message.

The published report is `tests/compatibility/reviews/coverage-v1.json`. It
records the exact jq 1.8.x reference executable selected by the runner. The
interactive edge-case baseline also used Apple jq 1.7.1 on arm64 macOS 26.5.

## Performance campaign

The standard-profile `benchmark.regex-test` workload ran once against the
frozen `usgs-all-day` manifest (197,727 JSON bytes). It applies a compiled
Unicode regex to every admitted feature-place string and returns one semantic
count.

| Adapter | Median wall time |
| --- | ---: |
| jq / JSON | 29,204 µs |
| tq / JSON | 30,903 µs |
| tq / YAML | 32,184 µs |
| tq / TOON | 32,275 µs |

The tq/JSON sample was 5.8% slower than jq/JSON. This one-sample run is a
correctness and regression smoke, not a statistically stable release claim.
The complete machine report is retained at
`benchmarks/.work/regex-date-platform.json`.

The ordinary four-case smoke campaign also passed after the new workload was
added to the validated 16-workload manifest.

## Resource and platform policy

Rust `regex` 1.13.1 bounds pattern, input, compiled program, VM work, results,
and output. Jiff 0.2.35 owns UTC conversions and the explicit portable range
`0000-01-01T00:00:00Z..=9999-12-30T22:00:00.999999999Z`. Environment
snapshots, the clock, local timezone, and
input metadata are denied unless the command and the library capability policy
both admit them. Compatibility evidence observes only environment type and
never serializes ambient values.

The release-host checks run in
`.github/workflows/regex-date-platform.yml`. Its Linux, macOS, and Windows
matrix runs the portable UTC boundary test and the ambient-policy and redaction
tests named in `tests/platform/regex-date-platform-v1.json`. The manifest does
not count a declared host as evidence unless those tests ran there.
