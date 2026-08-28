## Context

The current JSON document adapter can iterate whitespace-separated JSON texts in retained document mode, while the event decoder requires one root value followed by EOF. JSON output writes LF after each result, but pretty output may span several lines. This makes JSON Lines behavior depend on the selected query plan and flags.

The implementation must preserve exact decimal literals, ordered objects, bounded input handling, complete prior output records after late errors, and pre-input CLI validation. See `specs/json-lines-io/spec.md` for the observable record contract.

## Goals / Non-Goals

**Goals:**

- Give JSON Lines a distinct format identity in parsing, reports, diagnostics, and output validation.
- Share the existing JSON value conversion and compact writer instead of maintaining a second JSON data model.
- Process one bounded line at a time for normal, event, and subtree plans.
- Keep option behavior independent of argument order.

**Non-Goals:**

- Detect JSON Lines from stdin or unrecognized extensions by reading through a second record.
- Accept pretty, multiline JSON as one JSON Lines record.
- Add RFC 7464 JSON Text Sequences or change plain JSON's existing multi-text behavior.
- Add comments, trailing commas, or other non-JSON syntax.

## Decisions

### Use distinct input and output format variants

Add `InputFormat::JsonLines` and `OutputFormat::JsonLines`. Map both `jsonl` and `ndjson` CLI spellings to those variants and report the canonical name as `jsonl`.

A distinct identity keeps diagnostics, explanation reports, extension selection, plan eligibility, and option validation honest. Treating `jsonl` as a parser alias for `json` would preserve the current mismatch between retained and event execution. Treating output as `json` plus an implicit `-c` would make behavior depend on option order and hide the framing choice from library callers.

### Split records with a bounded buffered reader

Add a JSON Lines source adapter that reads through LF using bounded buffering. Count physical lines from one for each source, strip LF while allowing CR as trailing JSON whitespace, ignore only zero-length lines, and parse each remaining line as exactly one JSON value with no trailing non-whitespace bytes. Enforce the physical-line limit while filling the buffer so an oversized line cannot allocate without bound.

The adapter will reuse the existing arbitrary-precision JSON conversion after establishing the record boundary. A general serde JSON stream iterator was rejected because it accepts multiline values and multiple values on one line, neither of which matches the specified format.

### Expose record-at-a-time document and event entry points

The format layer will expose a pull or callback API that yields one value and its physical line number at a time. Document execution will evaluate and release each record before requesting the next unless slurp or a blocking plan requires retention. Event and subtree execution will pass each bounded record to the existing JSON event decoder, finish the record, reset decoder and capture state, and then continue.

This reuses the proven single-root decoder and makes record boundaries explicit. Extending the current event decoder to accept several roots without a reset was rejected because it would blur root-end events and make state leakage harder to test.

### Select JSON Lines by explicit format or file extension

Extend per-source path selection with case-insensitive `.jsonl` and `.ndjson` mappings. Explicit `--input-format` remains global and wins over every extension. Stdin and files with unknown extensions keep bounded content probing, which does not attempt to infer JSON Lines.

Content detection cannot reliably distinguish a JSON document from JSON Lines until it reaches a later record. Waiting that long would violate the current bounded commitment model, so explicit selection is the right stdin contract.

### Make JSON Lines output compact by construction

The JSON Lines writer will call the exact-number-preserving compact JSON encoder for each result and append LF after the complete encoded value. It will not expose a pretty branch or terminal color policy.

CLI parsing will track whether pretty, indentation, tab, forced color, raw, or joined output was explicitly requested. It will validate those choices against the final output format after parsing all arguments. `-c`, ASCII escaping, key sorting, monochrome mode, and unbuffered flushing remain compatible. This avoids last-option-wins behavior.

### Keep failure boundaries observable

Input errors will identify the source and physical line. A malformed later record will not retract complete results from earlier records. Resource errors will use the existing source and resource categories and will not include the full rejected line.

## Risks / Trade-offs

- [A single JSON Lines record may still be large] -> Enforce source, line, token, and depth limits before or during decode.
- [Per-record event decoder setup adds fixed overhead] -> Reuse configuration and buffers where practical, then add focused multi-record benchmarks before considering a stateful multi-root decoder.
- [Ignoring empty lines differs from strict JSON Lines descriptions] -> Document and test this deliberate interoperability rule; whitespace-only lines remain invalid.
- [Plain JSON already accepts multiple texts in some paths] -> Keep that behavior unchanged and add cross-plan tests proving that only JSON Lines promises physical record boundaries.

## Migration Plan

This is additive. Release the new variants, aliases, extension mappings, help text, and tests together. Existing `-i json` and `-o json -c` commands keep their current behavior. Users may migrate to `-i jsonl` for strict record boundaries and `-o jsonl` for guaranteed compact framing.

Rollback consists of removing the new variants and mappings. No stored data or configuration migration is required.
