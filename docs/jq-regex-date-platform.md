# jq regex, date, and platform compatibility

## Reviewed baseline

The cases cover syntax, Unicode scalar offsets, optional captures, match order,
flags, splitting and substitution, UTC arrays, epoch ranges, environment shape,
and input metadata. The 2026-08-10 exploratory run used Apple jq 1.7.1 on arm64
macOS 26.5. The full campaign uses the repository's pinned jq 1.8.x binary and
records its exact identity in `tests/compatibility/reviews/coverage-v1.json`.

## Selected dependencies and limits

`tq` uses Rust `regex` 1.13.1 and Jiff 0.2.35. The regex engine provides
linear-time Unicode matching without backtracking. Each VM invocation bounds
the UTF-8 pattern, searched input, compiled program, result count, instruction
work, and output bytes. The CLI maps `--max-token-bytes` to the pattern limit
and `--max-input-bytes` to the searched-input limit.

Supported regex built-ins are `test`, `match`, `capture`, `scan`, `split`,
`splits`, `sub`, and `gsub`. Supported flags are `g`, `i`, `m`, `s`, `p`, `x`,
and `n`. The `l` longest-match flag, look-around, backreferences, atomic groups,
and other Oniguruma-only syntax return a stable unsupported diagnostic. `m`
matches jq's dot-newline behavior; `s` retains single-line anchors; `p`
combines those modes. Offsets and lengths count Unicode scalar values, matching
jq's reviewed UTF-8 behavior.

Compatibility reports classify rejected engine syntax as
`unsupported-capability`, configured regex envelopes as `resource`, portable
date bounds as `runtime-range`, and denied ambient effects as `runtime-policy`.

## Date and time policy

UTC behavior does not depend on the host. `fromdate`, `fromdateiso8601`, `todate`,
`todateiso8601`, `gmtime`, `mktime`, `strptime`, and `strftime` use a reviewed
range from `0000-01-01T00:00:00Z` through
`9999-12-30T22:00:00.999999999Z` and return stable range/type errors.
Broken-down arrays use jq's zero-based month and year-day plus Sunday-based
weekday fields.

`localtime`, `strflocaltime`, and `now` read platform state. They require
`--allow-platform`; otherwise evaluation fails without consulting the clock or
timezone. Local results use the release host's configured timezone and are
classified as platform-dependent in
`tests/platform/regex-date-platform-v1.json`.
The UTC boundary and ambient-policy tests run on Linux, macOS, and Windows in
`.github/workflows/regex-date-platform.yml`.

## Environment and input metadata

`env`, `input_filename`, and `input_line_number` are denied by default.
`--allow-environment` captures Unicode process environment pairs into a
query-visible object. `--allow-platform` admits input identity/line metadata.
Library callers can independently deny either capability with
`CapabilityPolicy`, even if the command requests it.

Policy failures name only the requested operation and policy class. Reports do
not serialize environment values unless the query returns them. The executable
compatibility case checks only `env | type`.

For multi-document decoded input, `input_line_number` currently identifies the
first admitted logical input. Per-record physical line tracking and jq's exact
non-Unicode environment behavior remain documented platform divergences.
