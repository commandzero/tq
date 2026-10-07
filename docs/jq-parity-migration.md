---
type: Report
title: jq parity migration and security notes
description: Unreleased output, numeric, and ambient-access changes for jq manual compatibility.
generated: { by: codex/gpt-6-astra, at: 2026-09-15T05:04:02Z }
---

# jq parity migration and security notes

These notes describe the unreleased manual-compatibility work. They are not a
claim that the full manual or every release platform has passed acceptance.
See the [compatibility guide](compatibility.md) for the evidence policy.

## Required native acceptance and reference version

Current acceptance targets pinned jq 1.8.2 on local macOS, ironhide Linux
x86_64, and smokescreen Windows 11 Pro `x86_64-pc-windows-msvc`. The earlier
Windows deferral is superseded: native Windows jq/tq execution in native
PowerShell is required before completion. SSH into WSL, cross-compilation,
and PowerShell on macOS/Linux do not provide native Windows evidence.
Historical jq 1.8.1 reports and approvals do not establish acceptance for a
new source, executable, reference, or target identity. Exact matches, reviewed
safe-library disparities, and unresolved failures remain separate; this update
approves no new math differences and completes no acceptance evidence.

Native Windows compatibility capture is implemented with overlapped named
pipes and JobObjects. Native CPU/RSS accounting is implemented through a
retained exact-child handle using `GetProcessTimes` and `PeakWorkingSetSize`,
with a surviving isolated worker. Validation remains incomplete: separate
control executions vary in 15.625 ms CPU quanta, with differences up to
62.5 ms exceeding the unchanged 20 ms check. No check relaxation or performance
acceptance is authorized; issue #31 remains open. Strict-report differences
remain differences, not exact matches.

The user disabled Smart App Control on development-only smokescreen and
reported native launch verified with state `0`. No further security changes
are needed; launch success does not establish compatibility or performance
acceptance.

## Rust library migration to 0.4.0

The incompatible value-model boundary remains 0.4.0; the current upgrade target
is 0.5.0. Upgrade all workspace crates together from 0.3.0 to 0.5.0 to avoid mixing
incompatible value types. This pre-1.0 API change does not change the jq language
target or establish a completed compatibility claim.

`tq_formats::Document` adds the required public `line_number: u64` field.
Code constructing a struct literal must supply the one-based physical source
line reached while decoding that document. For a synthetic single-line source:

```rust
use tq_core::Value;
use tq_formats::{Document, InputFormat};

let document = Document {
    value: Value::Null,
    identity: "synthetic".to_owned(),
    format: InputFormat::Json,
    index: 0,
    line_number: 1,
};
```

This is a Rust construction-contract change; CLI users do not construct these
values. Decoders populate the field themselves. For multiline or multi-document
sources, preserve the physical line position rather than deriving it from the
document index. The unreleased version bump does not publish crates or tags.

## Native TOON 4.1 migration to 0.5.0

Upgrade `tq-core`, `tq-toon`, `tq-formats`, `tq-cli`, and comparison helpers
together. `WriterConfig` no longer has key-folding/flattening fields;
`DecoderConfig` no longer has path-expansion configuration. Remove those fields
and the `KeyFolding` / `PathExpansion` imports rather than substituting aliases.
The CLI rejects `--fold-keys` and `--flatten-depth` before reading input.

Quoted and unquoted dotted names are literal keys. Use `.["profile.city"]` to
query that literal name; recursive headers such as `profile{city}` explicitly
construct a nested `profile` object. Uniform array rows and keyed object values
use the first row's recursive header order. Later matching rows may have a
different insertion order; ordinary expanded objects and keyed entry order do
not gain a general order-insensitive comparison.

Root/object-field empty arrays use `[]`; anonymous nested arrays retain counted
list headers, including `[0]:`. Full-line comments, BOM/CRLF, and quoted control
escapes are accepted. Encoding and strict input acceptance follow 4.1 rather
than retaining v3 folding behavior. JSON and TOON sequence directions and
default LF result terminators are unchanged.

Identity transcode prepares complete object/array shapes in a shared bounded
arena and replay store before selecting canonical layouts. Replay bodies may
spill, but active key indexes and recursive schemas remain memory-bounded.
Non-strict duplicate semantics use a staged/document path where consumers
cannot retract values.

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

LF-terminated TOON values are the default, including multiple results and
results published before a later error. Use `-o toon-seq` when an adapter needs
unambiguous RS-framed TOON records without changing the input parser. `-c` and
`--compact-output` now select compact JSON directly, including inside short-option
clusters such as `-cr`.
Scripts that previously supplied `-o json -c` can keep doing so. Scripts using
`-c` while expecting TOON must remove `-c`; explicit `-o toon` combined with
`-c` is rejected regardless of order.

`-o json` selects pretty JSON unless compact output is requested. Selecting an
output format does not select an input parser. For strict JSON conformance,
use `-i json`; automatic input detection continues to recognize native formats.

`--seq` selects JSON sequence input and TOON sequence output by default.
`--seq -c` and `--seq -o json` opt into JSON sequence output, corresponding to
jq's compact and pretty sequence modes. Use `-i toon-seq` when reading
TOON sequence records; JSON and TOON record payloads are different formats even
though both use an ASCII record separator.

## Color presentation

The archived `output-colors` main spec supersedes the earlier requirement for
exact jq default color bytes. Every supported output format, including JSON
Lines/NDJSON, accepts the shared tq color policy. `TQ_COLORS` takes priority
over `JQ_COLORS`, even when empty or invalid; an invalid selected palette falls
back to the complete tq default. Seven/eight-slot overrides remain supported.
Enclosing quotes use structural styles rather than jq's string/key styles.
These explicit presentation differences are not exact jq ANSI matches or new
safe-library disparity approvals.

JSON Lines remains compact and rejects pretty/indent/tab and raw/join modes,
but no longer rejects forced color. Forced-color output contains ANSI SGR;
use redirected automatic output or `-M` for directly parseable data. Removing
only tq-generated SGR must reproduce the exact corresponding plain bytes,
including framing. Raw strings and proxy bytes remain undecorated, while
structured non-string raw fallback retains compact JSON and its separators.
Data, numeric, stderr, and process comparisons are not weakened by this
presentation contract.

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
differences are recorded in the [jq manual coverage](tests/jq-manual/coverage.md#differences-by-test),
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

For ordinary JSON input, the VM-step allowance starts afresh for each input root. Prefix access, item
evaluation, and a blocking suffix share that root's allowance. An ordinary
runtime error stops the current root's evaluation and permits later roots to
run. Syntax errors, exhausted resource limits, cancellation, and explicit
halts stop processing. A later successful root can determine the final exit
status even when stderr contains an earlier runtime diagnostic.

The automatic hybrid JSON Lines route retains a shared VM-step allowance across
document suffixes; earlier completed results remain published if it is exhausted.

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
