---
okf_version: "0.2"
---

# tq documentation

The complete `docs/` directory is the OKF bundle. Markdown files are concepts;
`requirements-traceability.tsv` is supporting data. Keep source code, tooling,
benchmark outputs, and working reports outside this directory.

Validate the complete bundle with `./scripts/check-docs.sh` from the repository root.
The same entry point runs in PR checks.

1. [compatibility](compatibility.md) - jq semantics, supported behavior, and deliberate compatibility differences.
2. [formats](formats.md) - Supported native formats, framing, and conversion behavior.
3. [jq-1.8-cli-options](jq-1.8-cli-options.md) - Classification of jq command-line options and tq behavior.
4. [jq-regex-date-platform](jq-regex-date-platform.md) - Regex and date behavior, platform capabilities, and resource limits.
5. [performance-baseline](performance-baseline.md) - Benchmark correctness gates, baseline comparisons, and measurement limits.
6. [requirements-traceability](requirements-traceability.md) - Routes from specification scenarios to implementation and test evidence.
7. [toon-event-boundary](toon-event-boundary.md) - Event decoding and the boundary between TOON and query execution.
8. [yaml-adapter-spike](yaml-adapter-spike.md) - YAML adapter findings and implementation constraints.
9. [adr/0001-compose-native-formats](adr/0001-compose-native-formats.md) - Compose native formats from framing, document codecs, and directional profiles.
10. [Changelog policy](changelog-policy.md) - How tq records notable changes and preserves release history.
11. [Contributor checks](contributor-checks.md) - Local validation, OpenSpec completion, and required PR checks.
12. [Contributing to tq](contributing.md) - Contributor setup and development requirements.
13. [Releasing tq](releasing.md) - Prepare, validate, package, and publish a coordinated tq release.
