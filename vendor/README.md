# Vendored reference dependency

`toon-format/` is the unmodified crates.io source for `toon-format` 0.5.0,
except for one Rust-version compatibility expression in
`src/decode/parser.rs`:

```rust
!indent_amount.is_multiple_of(indent_size)
```

is written as the arithmetic equivalent:

```rust
indent_amount % indent_size != 0
```

The published crate does not declare a Rust version and otherwise supports
tq's Rust 1.85 MSRV. tq uses it as an independent reference encoder/decoder for
corpus and conformance validation; production parsing uses `tq-toon`.

Upstream: https://github.com/toon-format/toon-rust
Crate: https://crates.io/crates/toon-format/0.5.0
License: MIT (retained in `toon-format/LICENSE`)
