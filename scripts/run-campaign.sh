#!/bin/sh
set -eu

campaign="${1:-}"
profile="${2:-}"

case "$campaign:$profile" in
    compatibility:smoke|compatibility:full)
        mkdir -p target/compatibility
        cargo build --quiet --release -p tq-cli
        TQ_BIN="${TQ_BIN:-$PWD/target/release/tq}"
        export TQ_BIN
        exec cargo run --quiet -p tq-test-support --bin tq-compat -- run \
            --profile "$profile" --json "target/compatibility/$profile.json"
        ;;
    benchmark:smoke)
        mkdir -p target/benchmarks
        cargo build --quiet --release -p tq-cli
        TQ_BIN="${TQ_BIN:-$PWD/target/release/tq}"
        export TQ_BIN
        exec cargo run --quiet -p tq-test-support --bin tq-bench -- run \
            --profile smoke --output target/benchmarks/smoke.json --max-samples 1 \
            --case benchmark.startup --case benchmark.parse-discard \
            --case benchmark.scalar-extraction --case benchmark.event-stream
        ;;
    benchmark:standard|benchmark:large)
        mkdir -p target/benchmarks
        cache_root="${TQ_CORPUS_CACHE:-target/corpus}"
        corpus_origin="${TQ_CORPUS_ORIGIN:-refreshed}"
        if [ "$corpus_origin" = refreshed ]; then
            refresh_json="$(mktemp "${TMPDIR:-/tmp}/tq-corpus.XXXXXX")"
            trap 'rm -f "$refresh_json"' EXIT HUP INT TERM
            cargo run --quiet -p tq-test-support --bin tq-corpus -- \
                refresh corpus/sources "$cache_root" "$profile" >"$refresh_json"
            TQ_BENCH_MANIFESTS="$(jq -r '.manifests | join(":")' "$refresh_json")"
            export TQ_BENCH_MANIFESTS
            rm -f "$refresh_json"
            trap - EXIT HUP INT TERM
        fi
        cargo build --quiet --release -p tq-cli
        TQ_BIN="${TQ_BIN:-$PWD/target/release/tq}"
        export TQ_BIN
        exec cargo run --quiet -p tq-test-support --bin tq-bench -- run \
            --profile "$profile" --output "target/benchmarks/$profile.json" \
            --cache-root "$cache_root" \
            --origin "$corpus_origin"
        ;;
    fuzz:default)
        seconds="${TQ_FUZZ_SECONDS:-10}"
        if command -v cargo-fuzz >/dev/null 2>&1; then
            cargo_fuzz="cargo-fuzz"
        elif [ -x target/cargo-fuzz/bin/cargo-fuzz ]; then
            cargo_fuzz="target/cargo-fuzz/bin/cargo-fuzz"
        else
            echo "cargo-fuzz is required (install with: cargo install cargo-fuzz)" >&2
            exit 69
        fi
        for target in query_parser toon_decoder bytecode_decode vm_program cli_args; do
            RUSTUP_TOOLCHAIN="${TQ_FUZZ_TOOLCHAIN:-nightly}" \
                "$cargo_fuzz" run "$target" -- \
                -max_total_time="$seconds" \
                -timeout=5 \
                -max_len=65536
        done
        exit 0
        ;;
    *)
        echo "campaign runner: unsupported campaign '$campaign' profile '$profile'" >&2
        exit 64
        ;;
esac
