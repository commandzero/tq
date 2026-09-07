# Changelog

All notable changes to this project are documented in this file.

New entries follow the [repository changelog policy](docs/changelog-policy.md).
Historical release entries retain their original ordering.

## [Unreleased]

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
