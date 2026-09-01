## Context

See `proposal.md` for the policy mismatch. The evaluator currently routes both
`input_filename` and `input_line_number` through one helper that first checks the
ambient platform flag. Input cursors already carry a one-based line number, and
the CLI supplies a fallback value for modes without a cursor. No operating-system
call is needed to produce either value.

The current line number is the decoder document index plus one. For ordinary
single-value input this is `1`; for multi-document input it identifies the
admitted logical document. It is not a physical source-line counter.

## Goals / Non-Goals

**Goals:**

- Separate decoder-owned line context from the ambient platform gate.
- Preserve library errors when no line context exists.
- Keep compatibility evidence and user-facing policy descriptions derived from the same behavior.

**Non-Goals:**

- Rework document indexing into exact physical-line tracking.
- Change the policy for `input_filename`, `now`, local timezone conversion, or `env`.
- Add a capability flag or alter the public `CapabilityPolicy` fields.

## Decisions

### Read line context without checking ambient platform admission

`input_line_number` will continue to prefer the current `InputCursor` context.
When no cursor is active, it will read the supplied line-number context directly.
The lookup will report that metadata is unavailable if neither source exists, but
it will not consult the ambient platform boolean.

This keeps the evaluator's current precedence rules and limits the code change to
policy classification. The alternative was to make all input metadata available
by default. That would expose `input_filename`, including user paths, and would
broaden issue #11 beyond line numbers.

### Retain the existing line-number model

The CLI will keep populating line context from each decoded document's index and
will keep using `1` for modes that provide only fallback context. The change will
not infer byte positions or scan source text a second time.

Exact jq physical-line behavior varies with parser and input mode and needs its
own design. Folding it into this policy fix would mix a compatibility decision
with a decoder data-model change.

### Reclassify tests and documentation around input context

The compatibility manifest will run `input_line_number` without
`--allow-platform`. A CLI regression test will cover the command from issue #11,
and a core test will deny platform access while supplying line context. Existing
tests will continue to prove that clock, timezone, and filename access remains
gated.

Help and compatibility prose will describe `--allow-platform` in terms of clock,
timezone, and source identity. The documentation will state separately that line
numbers are available by default and retain their logical-document limitation.

## Risks / Trade-offs

- [Some callers treated policy denial as the contract for every input metadata built-in] -> Document the deliberate exception and keep filename disclosure gated.
- [A shared helper change accidentally ungates `input_filename`] -> Split the line lookup from the platform-gated helper and test both paths with platform access denied.
- [Generated compatibility evidence no longer matches the manifest] -> Regenerate the report and run its consistency tests in the same change.

## Migration Plan

Ship the relaxed behavior in place. Callers may remove `--allow-platform` when
their query only uses `input_line_number`; the flag remains valid and unchanged
for mixed queries. Rollback restores the prior line-number policy check without a
data migration or file-format change.
