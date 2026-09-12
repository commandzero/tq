---
type: Report
title: jq regex, date, and platform compatibility
description: Safe-library behavior, resource limits, and ambient capability controls.
generated: { by: codex/gpt-6-astra, at: 2026-09-09T01:24:47Z }
---

# jq regex, date, and platform compatibility

## Reviewed baseline

The cases cover syntax, Unicode scalar offsets, optional captures, match order,
flags, splitting and substitution, UTC arrays, epoch ranges, environment shape,
and input metadata. The exploratory run used jq 1.7.1. The full campaign uses
the repository's pinned jq 1.8.x binary and records its exact identity in
the generated campaign report.

## Selected dependencies and limits

`tq` uses `fancy-regex` 0.19.1 for the jq-compatible regex surface and Jiff
0.2.35 for UTC-first date/time conversion. `fancy-regex` is a safe Rust
backtracking engine, so every compiled pattern receives an explicit
backtracking limit plus compiled/delegated-size limits. The VM also checks
input/pattern bytes, match and replacement counts, retained output, and
cancellation between bounded engine calls. The CLI maps
`--max-token-bytes` to the pattern limit and `--max-input-bytes` to the
searched-input limit; these are resource envelopes, not wall-clock
guarantees.

Supported regex built-ins are `test`, `match`, `capture`, `scan`, `split`,
`splits`, `sub`, and `gsub`. Supported flags are `g`, `i`, `m`, `s`, `p`, `x`,
and `n`; array pattern/flag forms are accepted only by the jq operations that
document them. The `l` longest-match flag returns a stable unsupported
diagnostic. Other patterns are parsed by the selected Rust engine; rejected
syntax produces a pattern error. `m` matches jq's dot-newline behavior; `s` retains single-line
anchors; `p` combines those modes. Offsets and lengths count Unicode scalar
values, matching jq's reviewed UTF-8 behavior. The `l` mode remains an
explicit safe-engine limitation and is not silently approximated.

Compatibility reports classify the rejected longest-match flag as
`unsupported-capability`, configured regex envelopes as `resource`, portable
date bounds as `runtime-range`, and denied ambient effects as `runtime-policy`.

## Date and time policy

UTC behavior does not depend on the host. `fromdate`, `fromdateiso8601`, `todate`,
`todateiso8601`, `gmtime`, `mktime`, `strptime`, and `strftime` use a reviewed
range from `0000-01-01T00:00:00Z` through
`9999-12-30T22:00:00.999999999Z` and return stable range/type errors.
Broken-down arrays use jq's zero-based month and year-day plus Sunday-based
weekday fields.

`localtime`, `strflocaltime`, and `now` read platform state. The process CLI
permits this access by default. Embedded callers can deny platform access with
an explicit `CapabilityPolicy`;
denied evaluation does not consult the clock or timezone. Local results use the
release host's configured timezone and are
classified as platform-dependent in the compatibility catalog. UTC/date
conversion remains safe Rust through Jiff; no C/FFI or `unsafe` exception is
used.
Run the UTC boundary and ambient-policy checks locally during PR preflight:

```console
cargo test -p tq-core regex_date_platform_release_host_contract
cargo test -p tq-cli ambient
```

The same checks run on Linux and macOS when a GitHub release is published by
`.github/workflows/regex-date-platform.yml`. Native Windows execution is deferred
until a runner is available. Windows test definitions remain in the repository;
their presence does not establish Windows compatibility.

## Environment and input metadata

The process CLI permits `env`, `$ENV`, `input_filename`, and
`input_line_number` without extra flags. Embedded callers must use
`parse_args_with_policy` with an explicit `CapabilityPolicy` to deny environment
or platform access. Supplying custom stdio to `run_with_io` does not itself
disable those capabilities. Policy denial wins even when a command requests
`--allow-environment` or `--allow-platform`; `input_line_number` needs no ambient
capability.

Policy failures name only the requested operation and policy class. Reports do
not serialize environment values unless the query returns them. Compatibility
cases inspect only object shape and the presence of a fixed campaign sentinel.

`$__loc__` does not require an ambient capability. It returns an ordered object
with `file` and one-based `line` fields for the reference in the query source.
Inline CLI filters use `<top-level>`. Filter files and imported modules use
their path identity, and references inside definitions report the definition
line rather than the call site.

For multi-document decoded input, `input_line_number` reports the one-based
line at the end of each decoded document. The shared input cursor updates source
identity and physical line metadata as it consumes records, including reads
through `input` and `inputs`. Native target and non-Unicode environment
behavior still require platform-specific evidence; an unverified target is not
an accepted disparity.
