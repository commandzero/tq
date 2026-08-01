# Vendored TOON decoder fixtures

The files under `decode/` and `encode/` are copied from the TOON specification test suite
at `toon-format/spec`, versioned by each fixture's `version` field. They are
distributed under the adjacent MIT license.

The integration test runs the applicable cases against `tq-toon` and also
compares every successful case to the published `toon-format` Rust decoder.
Safe dotted-path expansion is implemented as a DOM-consumer policy; the event
decoder continues to preserve exact key text and quoted-key provenance.
