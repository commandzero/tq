# Changelog

All notable changes to this project are documented in this file.

The format follows [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

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
