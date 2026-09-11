---
type: Report
title: jq parity migration and security notes
description: Unreleased output, numeric, and ambient-access changes for jq manual compatibility.
generated: { by: codex/gpt-6-astra, at: 2026-09-10T00:05:28Z }
---

# jq parity migration and security notes

These notes describe the unreleased manual-compatibility work. They are not a
claim that the full manual or every release platform has passed acceptance.
See the [compatibility guide](compatibility.md) for the evidence policy.

## Minimum Rust version

Building from source now requires Rust 1.95 or newer for `wait4 0.2.0`
native benchmark accounting. The earlier parity checkpoint required Rust 1.88:
the previous 1.87 declaration was incompatible with existing let chains and
the pinned JSON5 dependency.
The parser checkpoint passed workspace checks for all targets and features
with Rust 1.88 selected explicitly; the compatibility test campaigns used
Rust 1.98.0. These are separate checks, not a claim that the full test campaign
ran under Rust 1.88. Final acceptance requires rerunning checks after the
remaining performance work.

## Output selection

Standalone TOON documents are the default. Use `--seq` when a query emits
multiple TOON results, when a late error must preserve earlier results, or when
an adapter explicitly requests TOON Text Sequence framing. `-c` and
`--compact-output` now select compact JSON directly, including inside short-option
clusters such as `-cr`.
Scripts that previously supplied `-o json -c` can keep doing so. Scripts using
`-c` while expecting TOON must remove `-c`; explicit `-o toon` combined with
`-c` is rejected regardless of order.

`-o json` selects pretty JSON unless compact output is requested. Selecting an
output format does not select an input parser. For strict JSON conformance,
use `-i json`; automatic input detection continues to recognize native formats.

`--seq -c` and `--seq -o json` select JSON sequence output. `--seq` without an
output selector selects TOON sequence output. Use `-i toon-seq` when reading
TOON sequence records; JSON and TOON record payloads are different formats even
though both use an ASCII record separator.

## Numbers

Admitted decimal literals retain their identity and observable scale until an
operation requires binary64 arithmetic. For example, `1.000 | tojson` returns
the string `"1.000"`; canonical native TOON still represents that numeric value
as `1`. Arithmetic and mathematical functions are not arbitrary-precision
decimal arithmetic. They may round values that a literal or comparison can
retain exactly.

Runtime `nan` and `infinite` are numeric values, not strings. JSON and native
TOON output project NaN to `null` and infinities to signed binary64 finite
maximum. This output projection does not erase their runtime predicate or
arithmetic behavior. JSON input and `fromjson` now accept jq's non-finite
numeric tokens, including `NaN`, `Infinity`, and `-Infinity`, through a shared
safe-Rust incremental parser. Their numeric type is preserved during
evaluation: `"NaN" | fromjson | type` produces `"number"` even though rendering the value
itself produces `null`. Native TOON and YAML continue to reject non-finite
input; accepting these tokens in JSON does not broaden native-format syntax.

The default decimal exponent envelope increases from 1,000,000 to
2,000,000,000 so the manual's large-exponent examples do not require decimal
expansion. The significant-coefficient limit remains 4,096 digits, plain
expansion remains bounded at 4,096 digits, and rendered numeric tokens remain
bounded at 8,192 bytes. Input token limits can impose a tighter bound. Embedded
numeric callers can supply explicit `NumberLimits`.

Mathematical functions use safe Rust libraries. Measured last-bit and target
differences are recorded in the [disparity register](jq-compatibility-disparities.md),
not silently rounded into exact matches. The compatibility harness compares
decimal results losslessly, separately from runtime JSON projection.

## JSON roots and resource limits

Ordinary JSON input accepts whitespace-separated values. Automatic JSON
execution validates each complete root before publishing its selected results
or running effects such as `debug`. A malformed later root does not erase
results from earlier valid roots. Duplicate object keys use their final value
while retaining the first insertion position for object iteration.

Selected values are staged in memory or private temporary storage. The
dynamic staging store, retained subtrees, replayed values, and object-key index
share `--prepare-memory-bytes`. Fixed spool I/O buffers are reported separately
from this dynamic budget. `--max-spool-bytes` bounds cumulative spool writes,
including overwritten records. Reports expose root-staging memory, index,
encoded-data, and spool-write observations separately from output preparation.
JSON root staging currently uses serial decoding; reports do not count it as
parallel selected decoding.

The VM-step allowance starts afresh for each input root. Prefix access, item
evaluation, and a blocking suffix share that root's allowance. An ordinary
runtime error stops the current root's evaluation and permits later roots to
run. Syntax errors, exhausted resource limits, cancellation, and explicit
halts stop processing. A later successful root can determine the final exit
status even when stderr contains an earlier runtime diagnostic.

## Ambient access and untrusted filters

The process CLI now permits jq-style environment and platform access without
`--allow-environment` or `--allow-platform`. A filter can read `env`, `$ENV`, the
clock, local timezone, and input source metadata. Default module lookup and a
`HOME/.jq` startup file can also affect execution. Use a controlled environment
and HOME for reproducible jobs; do not run untrusted filters with credentials
in their environment.

The CLI is not a security sandbox. Embedded applications should call
`parse_args_with_policy` with an explicit `CapabilityPolicy`, disabling
environment, filesystem, terminal, and platform access as appropriate, and
retain their input/output and VM resource limits. Do not assume that injecting
stdio alone disables every ambient integration.

Module and argument-file reads remain bounded, and module resolution retains
canonical-path confinement and cycle checks. A denied capability or exhausted
resource budget is an error, not a successful compatibility result. Error
diagnostics should omit argument values and file contents; explicit user
filters such as `env`, `debug`, and `stderr` can intentionally emit data.

Regex operations have input, pattern, compiled-size, backtracking, match-count,
and replacement-count limits. Cancellation is checked between bounded
operations; this is not an immediate mid-match interruption guarantee. Math
recurrences also have bounded work. No first-party unsafe code, native regex
engine, or FFI bridge is introduced to chase exact platform behavior.
