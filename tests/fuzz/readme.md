# Bounded fuzz checks

`native_input` exercises JSON and TOON RS framing plus CSV and TSV logical rows
through committed input. It varies read chunk sizes and limits source bytes,
frame bytes, row bytes, field counts, token bytes, and nesting depth. JSON
sequences also exercise structural-event delivery and consumer termination.

Run with a nightly compiler and cargo-fuzz. On hosts where Homebrew's stable
`cargo` and `rustc` precede rustup in `PATH`, `RUSTUP_TOOLCHAIN` alone does not
select nightly for cargo-fuzz's child build. Put the chosen toolchain's `bin`
directory first and set `RUSTC` explicitly. Find those paths with
`rustup which --toolchain nightly-2026-08-30 rustc` and the corresponding
`cargo` command.

```console
PATH=/path/to/nightly/bin:$PATH RUSTC=/path/to/nightly/bin/rustc \
  CARGO_NET_OFFLINE=true cargo-fuzz run --fuzz-dir tests/fuzz native_input -- \
  -max_total_time=20 -timeout=5 -max_len=65536
```

Offline mode works only after dependencies are cached. A sandbox DNS failure
does not establish that crates.io is unavailable. Use cached dependencies or
request network access for dependency retrieval. Generated corpus, crash
artifacts, and instrumented binaries remain ignored; minimized regressions
belong in versioned tests.

These sanitizer runs are correctness checks, not performance or RSS benchmarks.
Authoritative benchmarks follow the separate [benchmark policy](../../benchmarks/README.md).
