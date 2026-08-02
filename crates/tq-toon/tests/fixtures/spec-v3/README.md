# Vendored TOON decoder fixtures

The files under `decode/` and `encode/` come from the TOON specification test
suite at `toon-format/spec`. Each fixture has a `version` field. The adjacent
MIT license applies to these files.

The integration test runs each applicable case against `tq-toon`. It also
compares every successful case with the published `toon-format` Rust decoder.
Safe dotted-path expansion is a DOM-consumer policy. The event decoder keeps
the exact key text and quoted-key provenance.
