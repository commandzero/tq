# `yaml_serde` adapter spike

The MVP adapter uses `yaml_serde` 0.10.x. Its streaming `Deserializer` yields
one YAML document at a time, so tq preserves document order and releases each
document after evaluation. A custom serde visitor builds `tq_core::Value`
directly. It does not convert through `serde_json::Value`.

The accepted profile is smaller than the full YAML data model:

- Mapping keys must be strings. The visitor rejects duplicate keys before
  insertion.
- The parser rejects custom tags and non-finite floats.
- The parser resolves aliases. tq does not retain comments, style, directives,
  tag spelling, or anchor identity.
- Signed and unsigned integers enter tq's exact decimal-literal number model.
- Finite YAML floats enter the binary64 side of the number model. This keeps
  the value returned by `yaml_serde`; YAML scalar resolution has already lost
  the original decimal spelling.

Tests cover multiple documents, insertion order, invalid keys, tags,
non-finite floats, and equivalence with JSON and TOON. Retaining source spelling
would require a lower-level scalar-event API from the YAML dependency.
