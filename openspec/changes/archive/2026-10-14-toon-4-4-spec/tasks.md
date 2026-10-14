## 1. Dependency and conformance cutover

- [x] 1.1 Upgrade both exact oracle pins and Cargo.lock to toon-format 0.7.0 with default features disabled.
- [x] 1.2 Replace the official fixture suite with complete, unmodified v4.4.0 decode/encode fixtures and LICENSE; pin tag/commit, hashes, Git blobs, byte lengths, and case membership.
- [x] 1.3 Preserve independent native/oracle, ordered-value, fragmented-event, discarded-event, color, and memory/spool assertions without exclusions.

## 2. Native behavior

- [x] 2.1 Restrict non-strict decoding to the five recoveries and reject malformed headers, width mismatches, trailing root content, and out-of-scope lines in both modes.
- [x] 2.2 Implement v4.4 header classification, quoted spans, SP/HTAB preparation, empty keys, Unicode escape validation, oversized length declarations, and initial nested depth-jump recovery.
- [x] 2.3 Preserve semantic resource limits, duplicate ordering, exact supported numbers, and event/document equivalence, including discarded and fragmented consumers.
- [x] 2.4 Update document and replay/transcode writers to escape only U+0000–U+001F and preserve literal DEL/C1 UTF-8.
- [x] 2.5 Update CLI version declaration, user migration documentation, changelog, and complete active requirement/scenario deltas.

## 3. Verification

- [x] 3.1 Run all official fixtures and workspace tests; exercise the CLI's corrected acceptance, depth recovery, output bytes, and errors.
- [x] 3.2 Run the full release compatibility campaign and preserve observed cross-tool differences separately from TOON parity.
- [x] 3.3 Pass the repository's full ordinary preflight, including Clippy, library tests/doctests, benchmark correctness smoke, documentation, and native OpenSpec validation.

