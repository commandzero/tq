## 1. Format model and adapters

- [x] 1.1 Add distinct JSON Lines input and output variants, canonical format names, and diagnostic/report serialization.
- [x] 1.2 Implement a bounded JSON Lines reader that yields one exact-number-preserving value per non-empty physical line with source and line metadata.
- [x] 1.3 Enforce source, physical-line, token, and nesting limits while reading JSON Lines and add focused malformed-input and limit tests.
- [x] 1.4 Add compact LF-terminated JSON Lines output by reusing the exact-number-preserving JSON encoder, including empty and late-error framing tests.

## 2. CLI selection and validation

- [x] 2.1 Accept `jsonl` and `ndjson` for `-i/--input-format` and `-o/--output-format`, using `jsonl` as the canonical help and report name.
- [x] 2.2 Infer JSON Lines from case-insensitive `.jsonl` and `.ndjson` extensions while preserving explicit-override precedence and per-file mixed-format selection.
- [x] 2.3 Track explicit output formatting requests and reject pretty, indentation, tab, forced-color, raw, and joined modes with JSON Lines independent of argument order.
- [x] 2.4 Permit compatible compact, ASCII, key-sort, monochrome, and unbuffered controls and add parser tests for separate and attached `-i/-o` forms.

## 3. Record execution

- [x] 3.1 Run normal document plans one JSON Lines record at a time and release completed records when the selected plan does not require retention.
- [x] 3.2 Make slurp collect ordered JSON Lines records across stdin and files into one array.
- [x] 3.3 Run explicit stream mode independently for each record and reset root event state at every line boundary.
- [x] 3.4 Enable automatic event and subtree plans for explicit or extension-selected JSON Lines input, resetting decoder and capture state between records.
- [x] 3.5 Preserve complete prior results when a later JSON Lines record fails and report the failing source and physical line.

## 4. Verification and documentation

- [x] 4.1 Add executable CLI tests for aliases, extension inference, override precedence, scalars and composites, blank lines, EOF without LF, invalid lines, mixed files, and output bytes.
- [x] 4.2 Add cross-plan equivalence tests covering document, event, subtree, slurp, and blocking filters over the same multi-record fixture.
- [x] 4.3 Update generated help, README and compatibility documentation to distinguish JSON documents, JSON Lines, and TOON Text Sequences.
- [x] 4.4 Run formatting, workspace lint, focused crate tests, and the workspace test suite; record any unrelated environment-dependent failure in the handoff.
