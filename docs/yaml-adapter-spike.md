# `yaml_serde` adapter spike

The MVP adapter uses `yaml_serde` 0.10.x as requested. Its streaming
`Deserializer` yields YAML documents independently, so tq can preserve document
order and release each document after evaluation. A custom serde visitor builds
`tq_core::Value` directly instead of detouring through `serde_json::Value`.

The accepted profile is deliberately narrower than YAML's full data model:

- mapping keys must be strings and duplicate keys are rejected before insert;
- custom tags and non-finite floats are rejected;
- aliases are resolved by the parser, while comments, style, directives, tag
  spelling, and anchor identity are not retained;
- signed and unsigned integers enter the exact decimal-literal side of the tq
  number model;
- finite YAML floats enter the explicit binary64/arithmetic side of the hybrid
  model. This preserves the exact value exposed by `yaml_serde` and never
  pretends that the original decimal spelling survived YAML scalar resolution.

Tests cover multiple documents, insertion order, duplicate and non-string keys,
tags, non-finite floats, and semantic equivalence with JSON and TOON. This is the
accepted MVP boundary; source-spelling fidelity would require a lower-level
scalar-event API from the YAML dependency.
