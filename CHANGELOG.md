# Changelog

All notable changes to this project are documented in this file.

New entries follow the [repository changelog policy](docs/changelog-policy.md).
Historical release entries retain their original ordering.

## [Unreleased]

### Security

- Enforced configured TOON sequence frame limits in byte-slice decoding before document materialization (#35).
- Prevented runtime-root staging from spilling to disk when an embedded caller denies filesystem access (#42).
- Kept signed jq artifact URLs out of downloader arguments and sanitized download failure diagnostics (#47).

### Removed

- **Breaking:** Removed TOON key folding/path expansion configuration and the `--fold-keys` / `--flatten-depth` switches. Dotted keys are literal keys; removed switches fail before input consumption.

### Changed

- `TQ_COLORS` now takes priority over `JQ_COLORS` for terminal palette customization.
- **Breaking:** Upgrade the tq Rust crates together to 0.5.0; see the [native TOON 4.1 migration notes](docs/jq-parity-migration.md#native-toon-41-migration-to-050).
- **Breaking:** Native TOON now follows 4.1, with recursive field groups, keyed tables, position-specific empty arrays/list indentation, comments, and control escapes. Canonical bytes and strict input acceptance change; default LF and explicit sequence/unframed output contracts remain unchanged.
- **Breaking:** `-c` and `--compact-output` now select compact JSON. Remove `-c` when expecting TOON; combining it with explicit TOON output is rejected (#42).
- **Breaking:** Default TOON output now emits zero or more LF-terminated values, preserving earlier results on later errors. Use `-o toon-seq` for RS framing or `--unframed` to require exactly one document (#41, #42).
- **Breaking:** `--seq` now reads JSON sequences and emits TOON sequences by default. Use `--seq -o json` or `--seq -c` to retain JSON sequence output (#42).
- The process CLI now permits jq environment and platform access without allow flags and loads startup definitions from `HOME/.jq`. Use a controlled environment and `HOME` for reproducible jobs; embedded capability controls remain available (#39, #42).
- With `input` and `inputs`, `-x` now passes through only rejected sources while evaluating valid sources in order. `--slurp` retains whole-source-set passthrough (#46).
- The CLI now uses mimalloc by default. Allocation-heavy queries may run faster but use more memory; build with `--no-default-features` to use the system allocator (#62).
- CLI memory-budget defaults now scale with available RAM and readable Linux cgroup limits. Explicit byte-limit flags override these defaults; embedded library defaults remain deterministic (#59).
- Terminal colors now use a shared palette across all output formats, with light-blue booleans, delimiter-colored quotes, and seven/eight-slot `JQ_COLORS` customization (#38).
- Source builds now require Rust 1.95 or newer (#34).
- macOS and Linux benchmarks now measure direct spawn-to-exit time and OS-recorded child peak RSS, keeping optional diagnostic sampling separate (#47).
- Benchmark suites now support independent quick, standard, and extended sampling. Quick runs are time-bounded diagnostics, not evidence for formal regression comparisons or replacements for extended archives (#60).

### Fixed

- Non-strict TOON-sequence `--stream` input now reaches the existing staged last-write-wins projection, emitting jq-compatible leaf and container-close records.
- jq-manual token savings now exclude otherwise equal values when a required jq/JSON or JSON/TOON process contract differs, retaining the exact mismatch and strict failure.
- Large comma-expression queries, including right-nested expressions, no longer overflow the Windows CLI's default stack (#68).
- Deeply nested malformed queries now return parser diagnostics without overflowing during error cleanup (#68).
- Enforced configured nesting-depth and token-size limits when decoding JSON byte slices, whether explicitly selected or auto-detected (#41, #46).
- `-R -s` now concatenates multiple sources into one bounded string, including remaining-input access with `-n` (#46).
- Input filename and line metadata now follow the shared input cursor and physical source position (#41, #42, #46).
- Automatic JSON subtree queries now validate each complete input value before running its filter, honor final duplicate-key values, and continue at the next input value after recoverable runtime errors (#41, #42).
- Decimal literals now retain their scale in JSON output and `tojson` until arithmetic requires binary64 conversion (#39).
- `sub` and `gsub` now preserve empty and multiple replacement-filter results (#39, #40).
- Previously rejected user-defined filter compositions now support object construction, slicing, folds, assignments, and additional callbacks (#39, #44, #45).

### Added

- Focused, correctness-gated TOON migration wall-time comparisons with repeated paired runs, bounded confirmation, a user-approved greater-than-20% hard regression limit, and nonblocking review notes above 10%. This local guard does not replace calibrated benchmark publication gates.
- Bounded longest-match regex support with the `l` flag; whole-pattern recursion remains unsupported.
- Expanded jq mathematical function support, including trigonometric, logarithmic, gamma, and Bessel functions; platform differences remain documented in the [jq manual coverage](docs/tests/jq-manual/coverage.md#differences-by-test) (#39, #40).
- Runtime NaN and infinity values, plus jq-style non-finite JSON input and `fromjson`. Native TOON and YAML input still reject non-finite values (#39, #40, #41).
- Nested array/object destructuring and alternative patterns with `?//` (#39, #44).
- SQL-style indexing and joins with `IN`, `INDEX`, and `JOIN` (#39, #40, #44).
- JSON data imports in jq modules (#39).
- URI decoding with `@urid` (#39, #40).
- Filter-driven diagnostics and termination with `debug`, `stderr`, `halt`, and `halt_error` (#39, #40, #42).
- Execution of jq-format test files with `--run-tests` (#42).

## [0.3.0] - 2026-09-06

### Added

- Added RFC 7464 JSON Text Sequence input and output with jq-compatible recovery.
- Added CSV and TSV input and output with header-defined row objects and type-preserving quoting.
- Added `--strict-conversion` to reject output normalizations that change the decoded value.

### Changed

- Changed comma and pipe precedence to match jq, so `label $out | (.[] | ., break $out)` stops after its first result.
- Changed `try` expressions to retain their pipeline boundaries, matching jq.
- Reduced per-document evaluation overhead for native input sequences and stopped retaining report observations when no report is requested.
- Changed `--seq` to select JSON sequence input and output; use explicit `toon-seq` selectors for TOON Text Sequences.
- Changed `--stream` records to use ordinary query evaluation and the shared `input`/`inputs` cursor.
- Upgraded Cargo dependencies, including major dependency updates, in the workspace and fuzz harness.
- Raised the Rust minimum for unpublished test and benchmark tooling to 1.88 for the latest ZIP dependency.
- Simplified the benchmark overview and removed private environment references from public documentation and fixtures.

## [0.2.0] - 2026-09-01

### Added

- Added automatic format detection, block-style YAML output, stdin identity mode, and short format flags (#1).
- Added bounded JSON Lines and NDJSON input with record-aware execution and line-framed output (#1).
- Added the `-x/--proxy-on-error` fallback for sources rejected by structured parsing (#2).
- Added streaming TOON transcode and strict JSON Lines I/O with resource-limit enforcement (#3).
- Added jq collection, path, generator, string, and math utility builtins (#5).
- Added bounded `fromjson` processing under managed JSON limits (#5).
- Added execution of user-defined filters inside supported callback builtins (#8).
- Added jq format strings and bounded formatters such as `@base64` (#18).
- Added JSON5 input, including kibana-sync triple-quoted multiline strings (#13).

### Changed

- Updated Rust dependencies to the latest releases compatible with Rust 1.87.
- Changed streamed `inputs` processing to use bounded buffering and reduce per-document scheduling overhead (#5).
- Changed format conversion to preserve oversized JSON numbers instead of silently falling back to YAML strings (#18).
- Changed `input_line_number` to work without `--allow-platform`, matching jq's default capability behavior (#11).
- Changed runtime objects with up to three members to use compact inline storage, reducing document-query peak memory (#16).

### Fixed

- Fixed comma generators in function arguments, including multi-key `sort_by` and `unique_by` filters (#7).
- Fixed document JSON decoding to reject numeric literals outside the supported envelope (#4).
- Fixed object multiplication to recursively merge objects for `*` and `*=`, preserving right-biased conflicts and key order (#9).
- Fixed malformed structured input leaking an incomplete TOON sequence record to stdout (#12).

## [0.1.0] - 2026-08-31

### Added

- Added crates.io packaging for `tq-cli`, which installs the `tq` command.
- Initial release

[Unreleased]: https://github.com/commandzero/tq/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/commandzero/tq/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/commandzero/tq/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/commandzero/tq/releases/tag/v0.1.0
