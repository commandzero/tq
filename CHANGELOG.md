# Changelog

All notable changes to this project are documented in this file.

The format follows [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- Added crates.io packaging for `tq-cli`, which installs the `tq` command.
- Initial release
- Added automatic format detection, block-style YAML output, stdin identity mode, and short format flags (#1).
- Added bounded JSON Lines and NDJSON input with record-aware execution and line-framed output (#1).
- Added the `-x/--proxy-on-error` fallback for sources rejected by structured parsing (#2).
- Added streaming TOON transcode and strict JSON Lines I/O with resource-limit enforcement (#3).
- Added jq collection, path, generator, string, and math utility builtins (#5).
- Added bounded `fromjson` processing under managed JSON limits (#5).
- Added execution of user-defined filters inside supported callback builtins (#8).
- Added jq format strings and bounded formatters such as `@base64` (#18).
- Added JSON5 input, including kibana-sync triple-quoted multiline strings (#13).
- Added a pinned jq manual conformance campaign with separate JSON, compact-byte, and TOON checks.
- Added safe-Rust implementations of the jq math function families.
- Added safe-Rust jq manual behavior coverage for filter composition and user-defined filter binding.
- Added reviewed jq-library disparity evidence for macOS and Linux ([disparity review](docs/jq-compatibility-disparities.md)); the reviewed exceptions are not a claim of 100% exact parity, and Windows runtime evidence remains unverified.

### Changed

- Changed macOS and Linux benchmark collection to direct spawn-to-exit timing and native child peak RSS, with separately labeled optional sampling and independent platform-time validation (#30).
- Changed benchmark result tables to show one decimal and units in measurement cells, use `-` for unavailable comparisons, and keep collector provenance outside tables (#30).
- Changed benchmark correctness checks to support default TOON result streams and retain failure diagnostics.
- Changed benchmark campaigns to return a failing exit status after saving failed observations.
- Changed benchmark campaigns to abort immediately when authoritative RSS collection is unavailable.
- Changed the benchmark catalog to enable verified YAML, TOON, and yq adapters and explain remaining exclusions.
- Changed optimized JSON execution to validate each input value before running its filter, preserving duplicate-key replacement and continuing after recoverable errors at the next input value.
- Changed `acos` and `exp` to use safe standard-library operations, matching the pinned macOS jq rounding witnesses.
- Raised the minimum Rust version to 1.95 to support native child accounting with `wait4 0.2.0` (#30).
- Corrected Linux CPU-time and process-group RSS measurements in benchmark reports.
- Changed streamed `inputs` processing to use bounded buffering and reduce per-document scheduling overhead (#5).
- Changed format conversion to preserve oversized JSON numbers instead of silently falling back to YAML strings (#18).
- Changed `-c` and `--compact-output` to select compact JSON while retaining TOON as the default output.
- Changed default TOON output to emit zero or more LF-terminated values, preserving earlier results on later errors. Use `--seq` for RS framing or `--unframed` to require exactly one document.
- Changed the process CLI to permit jq environment and platform access without extra allow flags; embedded capability controls remain available.

### Fixed

- Fixed comma generators in function arguments, including multi-key `sort_by` and `unique_by` filters (#7).
- Fixed document JSON decoding to reject numeric literals outside the supported envelope (#4).
- Fixed object multiplication to recursively merge objects for `*` and `*=`, preserving right-biased conflicts and key order (#9).
- Fixed malformed structured input leaking an incomplete TOON sequence record to stdout (#12).
